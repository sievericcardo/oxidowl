//! SPARQL Server Implementation using Oxigraph
//!
//! This module provides a SPARQL endpoint for ontology querying
//! using the Oxigraph RDF store and SPARQL engine.

use crate::{
    Error, Result,
    ontology::{Axiom, Ontology},
    reasoning::ReasoningService,
};
use oxigraph::{
    io::{RdfFormat, RdfParser},
    model::{Dataset, GraphName, NamedNode, NamedOrBlankNode, Quad, Subject, Term, Triple},
    sparql::{Query, QueryResults, QuerySolution},
    store::Store,
};
use crate::semantics::{RdfGraph, RdfTerm, Triple as OxidowlTriple};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use warp::{Filter, Reply};

/// SPARQL server for ontology querying
#[derive(Debug)]
pub struct SparqlServer {
    /// Server port
    port: u16,
    /// Bind address
    bind_address: String,
    /// Reasoning service
    reasoning_service: Arc<ReasoningService>,
    /// RDF store
    store: Arc<RwLock<Store>>,
}

impl SparqlServer {
    /// Create a new SPARQL server
    pub fn new(port: u16, bind_address: String, reasoning_service: Arc<ReasoningService>) -> Self {
        Self {
            port,
            bind_address,
            reasoning_service,
            store: Arc::new(RwLock::new(
                Store::new().expect("Failed to create new SPARQL store"),
            )),
        }
    }

    /// Start the SPARQL server
    pub async fn start(self) -> Result<SparqlServerHandle> {
        // Initialize RDF store with ontology data
        self.initialize_store().await?;

        let store = self.store.clone();
        let reasoning_service = self.reasoning_service.clone();

        // SPARQL query endpoint
        let sparql_query = warp::path("sparql")
            .and(warp::post())
            .and(warp::body::form())
            .and(warp::any().map(move || store.clone()))
            .and(warp::any().map(move || reasoning_service.clone()))
            .and_then(handle_sparql_query);

        // SPARQL update endpoint
        let sparql_update = warp::path("sparql-update")
            .and(warp::post())
            .and(warp::body::form())
            .and(warp::any().map(move || self.store.clone()))
            .and_then(handle_sparql_update);

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::json(&serde_json::json!({"status": "ok"})));

        let routes = sparql_query.or(sparql_update).or(health).with(
            warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["content-type"])
                .allow_methods(vec!["GET", "POST"]),
        );

        let addr: SocketAddr = format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|e| Error::config(format!("Invalid server address: {}", e)))?;

        let server = warp::serve(routes).bind(addr);
        let server_task = tokio::spawn(server);

        tracing::info!(
            "SPARQL server started on {}:{}",
            self.bind_address,
            self.port
        );

        Ok(SparqlServerHandle { task: server_task })
    }

    /// Initialize the RDF store with ontology data
    async fn initialize_store(&self) -> Result<()> {
        let reasoner = self.reasoning_service.get_reasoner().await?;
        let ontology = reasoner
            .read()
            .map_err(|e| {
                Error::lock_poisoned(format!("Failed to acquire read lock on reasoner: {}", e))
            })?
            .get_ontology()
            .ok_or_else(|| Error::reasoning("Failed to get ontology from reasoner"))?;
        let ontology_guard = ontology.read().map_err(|e| {
            Error::lock_poisoned(format!("Failed to acquire read lock on ontology: {}", e))
        })?;

        let mut store = self.store.write().await;

        // Convert ontology axioms to RDF triples and add to store
        for axiom in &ontology_guard.axioms {
            let triples = self.axiom_to_triples(axiom)?;
            for triple in triples {
                let quad = Quad::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphName::DefaultGraph,
                );
                store
                    .insert(&quad)
                    .map_err(|e| Error::Sparql { message: e.to_string() })?;
            }
        }

        // Add inferred triples from reasoning
        let classification = self.reasoning_service.get_classification().await?;
        for (subclass, superclasses) in &classification.hierarchy {
            for superclass in superclasses {
                let sub_iri =
                    NamedNode::new(subclass).map_err(|e| Error::Sparql { message: e.to_string() })?;
                let sup_iri =
                    NamedNode::new(superclass).map_err(|e| Error::Sparql { message: e.to_string() })?;
                let rdfs_subclass =
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;

                let triple = Triple::new(sub_iri, rdfs_subclass, sup_iri);
                let quad = Quad::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphName::DefaultGraph,
                );
                store
                    .insert(&quad)
                    .map_err(|e| Error::Sparql { message: e.to_string() })?;
            }
        }

        // Add RDF-star triples from ontology's RDF graph
        if let Some(rdf_graph) = ontology_guard.get_rdf_graph() {
            for rdf_triple in rdf_graph.triples() {
                if let Ok(oxigraph_quad) = self.convert_rdfstar_triple_to_quad(rdf_triple) {
                    store
                        .insert(&oxigraph_quad)
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;
                }
            }
            tracing::info!("Added {} RDF-star triples to SPARQL store", rdf_graph.triples().len());
        }

        tracing::info!("Initialized SPARQL store with ontology data");
        Ok(())
    }

    /// Convert an ontology axiom to RDF triples
    fn axiom_to_triples(&self, axiom: &Axiom) -> Result<Vec<Triple>> {
        let mut triples = Vec::new();

        match axiom {
            Axiom::SubClassOf(sub, sup) => {
                // Convert class expressions to IRIs (simplified)
                if let (Some(sub_iri), Some(sup_iri)) =
                    (self.class_expr_to_iri(sub), self.class_expr_to_iri(sup))
                {
                    let rdfs_subclass =
                        NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")
                            .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    triples.push(Triple::new(sub_iri, rdfs_subclass, sup_iri));
                }
            }
            Axiom::ClassAssertion(class, individual) => {
                if let Some(class_iri) = self.class_expr_to_iri(class) {
                    let individual_iri = NamedNode::new(&individual.iri)
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    let rdf_type =
                        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                            .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    triples.push(Triple::new(individual_iri, rdf_type, class_iri));
                }
            }
            Axiom::ObjectPropertyAssertion(prop, subj, obj) => {
                let prop_iri = NamedNode::new(&prop.to_string())
                    .map_err(|e| Error::Sparql { message: e.to_string() })?;
                let subj_iri =
                    NamedNode::new(&subj.iri).map_err(|e| Error::Sparql { message: e.to_string() })?;
                let obj_iri =
                    NamedNode::new(&obj.iri).map_err(|e| Error::Sparql { message: e.to_string() })?;
                triples.push(Triple::new(subj_iri, prop_iri, obj_iri));
            }
            _ => {
                // Handle other axiom types as needed
            }
        }

        Ok(triples)
    }

    /// Convert class expression to IRI (simplified for atomic classes)
    fn class_expr_to_iri(
        &self,
        class_expr: &crate::ontology::ClassExpression,
    ) -> Option<NamedNode> {
        match class_expr {
            crate::ontology::ClassExpression::Class(class) => NamedNode::new(&class.iri).ok(),
            _ => None, // Complex expressions would need more sophisticated handling
        }
    }

    /// Convert an oxidowl RDF-star triple to an Oxigraph quad
    /// This handles quoted triples (RDF-star) by converting them to Oxigraph's Triple term
    fn convert_rdfstar_triple_to_quad(
        &self,
        rdf_triple: &OxidowlTriple,
    ) -> Result<Quad> {
        let subject = self.convert_rdf_term_to_oxigraph_subject(&rdf_triple.subject)?;
        let predicate = self.convert_rdf_term_to_oxigraph_predicate(&rdf_triple.predicate)?;
        let object = self.convert_rdf_term_to_oxigraph_term(&rdf_triple.object)?;

        Ok(Quad::new(
            subject,
            predicate,
            object,
            GraphName::DefaultGraph,
        ))
    }

    /// Convert oxidowl RdfTerm to Oxigraph Subject (IRI, BlankNode, or Triple)
    fn convert_rdf_term_to_oxigraph_subject(&self, term: &RdfTerm) -> Result<Subject> {
        match term {
            RdfTerm::IRI(iri) => {
                let node = NamedNode::new(iri).map_err(|e| Error::Sparql { message: e.to_string() })?;
                Ok(Subject::NamedNode(node))
            }
            RdfTerm::BlankNode(id) => {
                let node = oxigraph::model::BlankNode::new(id)
                    .map_err(|e| Error::Sparql { message: e.to_string() })?;
                Ok(Subject::BlankNode(node))
            }
            RdfTerm::QuotedTriple(boxed_triple) => {
                // RDF-star: Convert nested triple to Oxigraph Triple
                let s = self.convert_rdf_term_to_oxigraph_subject(&boxed_triple.subject)?;
                let p = self.convert_rdf_term_to_oxigraph_predicate(&boxed_triple.predicate)?;
                let o = self.convert_rdf_term_to_oxigraph_term(&boxed_triple.object)?;
                Ok(Subject::Triple(Box::new(Triple::new(s, p, o))))
            }
            _ => Err(Error::Sparql { message: "Invalid RDF term for subject position".to_string() }),
        }
    }

    /// Convert oxidowl RdfTerm to Oxigraph NamedNode (predicate must be IRI)
    fn convert_rdf_term_to_oxigraph_predicate(&self, term: &RdfTerm) -> Result<NamedNode> {
        match term {
            RdfTerm::IRI(iri) => {
                NamedNode::new(iri).map_err(|e| Error::Sparql { message: e.to_string() })
            }
            _ => Err(Error::Sparql { message: "Predicate must be an IRI".to_string() }),
        }
    }

    /// Convert oxidowl RdfTerm to Oxigraph Term (IRI, BlankNode, Literal, or Triple)
    fn convert_rdf_term_to_oxigraph_term(&self, term: &RdfTerm) -> Result<Term> {
        match term {
            RdfTerm::IRI(iri) => {
                let node = NamedNode::new(iri).map_err(|e| Error::Sparql { message: e.to_string() })?;
                Ok(Term::NamedNode(node))
            }
            RdfTerm::BlankNode(id) => {
                let node = oxigraph::model::BlankNode::new(id)
                    .map_err(|e| Error::Sparql { message: e.to_string() })?;
                Ok(Term::BlankNode(node))
            }
            RdfTerm::Literal { value, datatype, language } => {
                let lit = if let Some(lang) = language {
                    oxigraph::model::Literal::new_language_tagged_literal(value, lang)
                        .map_err(|e| Error::Sparql { message: e.to_string() })?
                } else if let Some(dt) = datatype {
                    let dt_node = NamedNode::new(dt)
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    oxigraph::model::Literal::new_typed_literal(value, dt_node)
                } else {
                    oxigraph::model::Literal::new_simple_literal(value)
                };
                Ok(Term::Literal(lit))
            }
            RdfTerm::QuotedTriple(boxed_triple) => {
                // RDF-star: Convert nested triple to Oxigraph Triple
                let s = self.convert_rdf_term_to_oxigraph_subject(&boxed_triple.subject)?;
                let p = self.convert_rdf_term_to_oxigraph_predicate(&boxed_triple.predicate)?;
                let o = self.convert_rdf_term_to_oxigraph_term(&boxed_triple.object)?;
                Ok(Term::Triple(Box::new(Triple::new(s, p, o))))
            }
        }
    }
}

/// Handle for a running SPARQL server
#[derive(Debug)]
pub struct SparqlServerHandle {
    task: tokio::task::JoinHandle<()>,
}

impl SparqlServerHandle {
    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        self.task.abort();
        Ok(())
    }
}

/// SPARQL query request structure
#[derive(Debug, Deserialize)]
pub struct SparqlQueryRequest {
    /// SPARQL query string
    pub query: String,
    /// Result format (optional)
    pub format: Option<String>,
}

/// SPARQL query response structure
#[derive(Debug, Serialize)]
pub struct SparqlQueryResponse {
    /// Query results
    pub results: SparqlResults,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// SPARQL results structure
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SparqlResults {
    /// SELECT query results
    Bindings {
        bindings: Vec<HashMap<String, SparqlValue>>,
    },
    /// ASK query result
    Boolean(bool),
    /// CONSTRUCT/DESCRIBE query results
    Graph { triples: Vec<SparqlTriple> },
}

/// SPARQL value in results
#[derive(Debug, Serialize)]
pub struct SparqlValue {
    /// Value type (uri, literal, bnode, triple)
    #[serde(rename = "type")]
    pub value_type: String,
    /// Value content
    pub value: String,
    /// Language tag (for literals)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Datatype (for literals)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    /// RDF-star: Nested triple structure (for quoted triples)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triple: Option<Box<SparqlTriple>>,
}

/// SPARQL triple in graph results
#[derive(Debug, Serialize)]
pub struct SparqlTriple {
    /// Subject
    pub subject: SparqlValue,
    /// Predicate
    pub predicate: SparqlValue,
    /// Object
    pub object: SparqlValue,
}

/// Handle SPARQL query requests
async fn handle_sparql_query(
    form: HashMap<String, String>,
    store: Arc<RwLock<Store>>,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let start_time = std::time::Instant::now();

    let query_string = form
        .get("query")
        .ok_or_else(|| warp::reject::custom(SparqlError("Missing query parameter".to_string())))?;

    tracing::debug!("Executing SPARQL query: {}", query_string);

    // Parse SPARQL query
    let query = Query::parse(query_string, None)
        .map_err(|e| warp::reject::custom(SparqlError(format!("Query parse error: {}", e))))?;

    // Execute query
    let store_guard = store.read().await;
    let results = store_guard
        .query(query)
        .map_err(|e| warp::reject::custom(SparqlError(format!("Query execution error: {}", e))))?;

    let execution_time = start_time.elapsed().as_millis() as u64;

    // Format results
    let sparql_results = match results {
        QueryResults::Solutions(solutions) => {
            let bindings: Result<Vec<_>, _> = solutions
                .map(|solution| {
                    solution.map(|sol| {
                        sol.iter()
                            .map(|(var, term)| {
                                (var.as_str().to_string(), term_to_sparql_value(term))
                            })
                            .collect::<HashMap<_, _>>()
                    })
                })
                .collect();

            let bindings =
                bindings.map_err(|e| warp::reject::custom(SparqlError(e.to_string())))?;
            SparqlResults::Bindings { bindings }
        }
        QueryResults::Boolean(result) => SparqlResults::Boolean(result),
        QueryResults::Graph(graph) => {
            let triples: Vec<_> = graph
                .map(|triple| {
                    triple.map(|t| SparqlTriple {
                        subject: term_to_sparql_value(&t.subject.into()),
                        predicate: term_to_sparql_value(&t.predicate.into()),
                        object: term_to_sparql_value(&t.object),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| warp::reject::custom(SparqlError(e.to_string())))?;

            SparqlResults::Graph { triples }
        }
    };

    let response = SparqlQueryResponse {
        results: sparql_results,
        execution_time_ms: execution_time,
    };

    Ok(warp::reply::json(&response))
}

/// Handle SPARQL update requests
async fn handle_sparql_update(
    form: HashMap<String, String>,
    store: Arc<RwLock<Store>>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let update_string = form
        .get("update")
        .ok_or_else(|| warp::reject::custom(SparqlError("Missing update parameter".to_string())))?;

    tracing::debug!("Executing SPARQL update: {}", update_string);

    // Parse and execute update
    let mut store_guard = store.write().await;
    store_guard
        .update(update_string)
        .map_err(|e| warp::reject::custom(SparqlError(format!("Update execution error: {}", e))))?;

    Ok(warp::reply::json(&serde_json::json!({
        "status": "success",
        "message": "Update executed successfully"
    })))
}

/// Convert Oxigraph term to SPARQL value
fn term_to_sparql_value(term: &Term) -> SparqlValue {
    match term {
        Term::NamedNode(node) => SparqlValue {
            value_type: "uri".to_string(),
            value: node.as_str().to_string(),
            lang: None,
            datatype: None,
            triple: None,
        },
        Term::BlankNode(node) => SparqlValue {
            value_type: "bnode".to_string(),
            value: node.as_str().to_string(),
            lang: None,
            datatype: None,
            triple: None,
        },
        Term::Literal(literal) => {
            let lang = literal.language().map(|l| l.to_string());
            let datatype = if literal.datatype() == xsd::STRING {
                None
            } else {
                Some(literal.datatype().as_str().to_string())
            };

            SparqlValue {
                value_type: "literal".to_string(),
                value: literal.value().to_string(),
                lang,
                datatype,
                triple: None,
            }
        }
        Term::Triple(triple_box) => {
            // RDF-star: Serialize the quoted triple
            let nested_triple = SparqlTriple {
                subject: term_to_sparql_value(&triple_box.subject.into()),
                predicate: term_to_sparql_value(&triple_box.predicate.into()),
                object: term_to_sparql_value(&triple_box.object),
            };
            
            SparqlValue {
                value_type: "triple".to_string(),
                value: format!(
                    "<< {} {} >>",
                    triple_to_ntriples_string(&**triple_box),
                    // Simplified string representation
                ),
                lang: None,
                datatype: None,
                triple: Some(Box::new(nested_triple)),
            }
        }
        _ => SparqlValue {
            value_type: "unknown".to_string(),
            value: term.to_string(),
            lang: None,
            datatype: None,
            triple: None,
        },
    }
}

/// Convert a triple to N-Triples string representation
fn triple_to_ntriples_string(triple: &Triple) -> String {
    let subject_str = match &triple.subject {
        NamedOrBlankNode::NamedNode(node) => format!("<{}>", node.as_str()),
        NamedOrBlankNode::BlankNode(node) => format!("_:{}", node.as_str()),
    };
    let predicate_str = format!("<{}>", triple.predicate.as_str());
    let object_str = term_to_ntriples_string(&triple.object);
    
    format!("{} {} {}", subject_str, predicate_str, object_str)
}

/// Convert a term to N-Triples string representation
fn term_to_ntriples_string(term: &Term) -> String {
    match term {
        Term::NamedNode(node) => format!("<{}>", node.as_str()),
        Term::BlankNode(node) => format!("_:{}", node.as_str()),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                format!("\"{}\"@{}", lit.value(), lang)
            } else if lit.datatype() != xsd::STRING {
                format!("\"{}\"^^<{}>", lit.value(), lit.datatype().as_str())
            } else {
                format!("\"{}\"", lit.value())
            }
        }
        Term::Triple(t) => format!("<< {} >>", triple_to_ntriples_string(t)),
        _ => term.to_string(),
    }
}

/// SPARQL error for warp rejection
#[derive(Debug)]
struct SparqlError(String);

impl warp::reject::Reject for SparqlError {}

// XSD namespace constants
mod xsd {
    use oxigraph::model::NamedNode;

    pub const STRING: NamedNode =
        NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantics::{RdfGraph, RdfTerm, Triple as OxidowlTriple};
    use crate::ontology::Ontology;
    use crate::reasoning::ReasoningService;

    #[test]
    fn test_convert_simple_rdf_triple() {
        let server = create_test_server();
        
        let triple = OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/alice".to_string()),
            predicate: RdfTerm::IRI("http://example.org/knows".to_string()),
            object: RdfTerm::IRI("http://example.org/bob".to_string()),
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_convert_quoted_triple_as_subject() {
        let server = create_test_server();
        
        // Create inner quoted triple: << :alice :knows :bob >>
        let inner_triple = Box::new(OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/alice".to_string()),
            predicate: RdfTerm::IRI("http://example.org/knows".to_string()),
            object: RdfTerm::IRI("http://example.org/bob".to_string()),
        });
        
        // Create outer triple: << :alice :knows :bob >> :certainty "0.95"
        let outer_triple = OxidowlTriple {
            subject: RdfTerm::QuotedTriple(inner_triple),
            predicate: RdfTerm::IRI("http://example.org/certainty".to_string()),
            object: RdfTerm::Literal {
                value: "0.95".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#double".to_string()),
                language: None,
            },
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&outer_triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_convert_quoted_triple_as_object() {
        let server = create_test_server();
        
        // Create quoted triple: << :doc1 :author "Smith" >>
        let inner_triple = Box::new(OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/doc1".to_string()),
            predicate: RdfTerm::IRI("http://example.org/author".to_string()),
            object: RdfTerm::Literal {
                value: "Smith".to_string(),
                datatype: None,
                language: None,
            },
        });
        
        // Create triple: :archive23 :contains << :doc1 :author "Smith" >>
        let outer_triple = OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/archive23".to_string()),
            predicate: RdfTerm::IRI("http://example.org/contains".to_string()),
            object: RdfTerm::QuotedTriple(inner_triple),
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&outer_triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_convert_nested_quoted_triple() {
        let server = create_test_server();
        
        // Create innermost triple: << :a :b :c >>
        let innermost = Box::new(OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/a".to_string()),
            predicate: RdfTerm::IRI("http://example.org/b".to_string()),
            object: RdfTerm::IRI("http://example.org/c".to_string()),
        });
        
        // Create middle triple: << << :a :b :c >> :d :e >>
        let middle = Box::new(OxidowlTriple {
            subject: RdfTerm::QuotedTriple(innermost),
            predicate: RdfTerm::IRI("http://example.org/d".to_string()),
            object: RdfTerm::IRI("http://example.org/e".to_string()),
        });
        
        // Create outer triple: << << << :a :b :c >> :d :e >> :f :g >> :h :i
        let outer = OxidowlTriple {
            subject: RdfTerm::QuotedTriple(middle),
            predicate: RdfTerm::IRI("http://example.org/h".to_string()),
            object: RdfTerm::IRI("http://example.org/i".to_string()),
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&outer);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_convert_literal_with_language() {
        let server = create_test_server();
        
        let triple = OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/doc".to_string()),
            predicate: RdfTerm::IRI("http://example.org/title".to_string()),
            object: RdfTerm::Literal {
                value: "Example Document".to_string(),
                datatype: None,
                language: Some("en".to_string()),
            },
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_convert_literal_with_datatype() {
        let server = create_test_server();
        
        let triple = OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/measurement".to_string()),
            predicate: RdfTerm::IRI("http://example.org/value".to_string()),
            object: RdfTerm::Literal {
                value: "42".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
                language: None,
            },
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_term_to_sparql_value_quoted_triple() {
        // Create an Oxigraph triple for RDF-star
        let subject = oxigraph::model::NamedNode::new("http://example.org/alice").unwrap();
        let predicate = oxigraph::model::NamedNode::new("http://example.org/knows").unwrap();
        let object = oxigraph::model::NamedNode::new("http://example.org/bob").unwrap();
        
        let triple = Triple::new(subject, predicate, object);
        let term = Term::Triple(Box::new(triple));
        
        let sparql_value = term_to_sparql_value(&term);
        
        assert_eq!(sparql_value.value_type, "triple");
        assert!(sparql_value.triple.is_some());
        
        let nested = sparql_value.triple.unwrap();
        assert_eq!(nested.subject.value_type, "uri");
        assert_eq!(nested.subject.value, "http://example.org/alice");
        assert_eq!(nested.predicate.value_type, "uri");
        assert_eq!(nested.predicate.value, "http://example.org/knows");
        assert_eq!(nested.object.value_type, "uri");
        assert_eq!(nested.object.value, "http://example.org/bob");
    }

    #[test]
    fn test_term_to_ntriples_string_quoted_triple() {
        let subject = oxigraph::model::NamedNode::new("http://ex.org/s").unwrap();
        let predicate = oxigraph::model::NamedNode::new("http://ex.org/p").unwrap();
        let object = oxigraph::model::NamedNode::new("http://ex.org/o").unwrap();
        
        let triple = Triple::new(subject, predicate, object);
        let ntriples_str = triple_to_ntriples_string(&triple);
        
        assert!(ntriples_str.contains("<http://ex.org/s>"));
        assert!(ntriples_str.contains("<http://ex.org/p>"));
        assert!(ntriples_str.contains("<http://ex.org/o>"));
    }

    #[test]
    fn test_blank_node_conversion() {
        let server = create_test_server();
        
        let triple = OxidowlTriple {
            subject: RdfTerm::BlankNode("b0".to_string()),
            predicate: RdfTerm::IRI("http://example.org/property".to_string()),
            object: RdfTerm::BlankNode("b1".to_string()),
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&triple);
        assert!(quad.is_ok());
    }

    #[test]
    fn test_rdfstar_provenance_pattern() {
        let server = create_test_server();
        
        // Pattern: << :doc1 :author "Smith" >> :source :archive23
        let inner = Box::new(OxidowlTriple {
            subject: RdfTerm::IRI("http://example.org/doc1".to_string()),
            predicate: RdfTerm::IRI("http://example.org/author".to_string()),
            object: RdfTerm::Literal {
                value: "Smith".to_string(),
                datatype: None,
                language: None,
            },
        });
        
        let provenance = OxidowlTriple {
            subject: RdfTerm::QuotedTriple(inner),
            predicate: RdfTerm::IRI("http://example.org/source".to_string()),
            object: RdfTerm::IRI("http://example.org/archive23".to_string()),
        };
        
        let quad = server.convert_rdfstar_triple_to_quad(&provenance);
        assert!(quad.is_ok());
    }

    // Helper function to create a test server
    fn create_test_server() -> SparqlServer {
        let ontology = Arc::new(Ontology::new());
        let reasoning_service = Arc::new(ReasoningService::new(ontology));
        SparqlServer::new(8082, "127.0.0.1".to_string(), reasoning_service)
    }
}
