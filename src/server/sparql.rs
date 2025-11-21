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
    model::{Dataset, GraphName, NamedNode, Quad, Subject, Term, Triple},
    sparql::{Query, QueryResults, QuerySolution},
    store::Store,
};
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
            store: Arc::new(RwLock::new(Store::new().expect("Failed to create new SPARQL store"))),
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
        let ontology = reasoner.read().expect("Failed to acquire read lock on reasoner").get_ontology().expect("Failed to get ontology from reasoner");
        let ontology_guard = ontology.read().expect("Failed to acquire read lock on ontology");

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
                    .map_err(|e| Error::SparqlError(e.to_string()))?;
            }
        }

        // Add inferred triples from reasoning
        let classification = self.reasoning_service.get_classification().await?;
        for (subclass, superclasses) in &classification.hierarchy {
            for superclass in superclasses {
                let sub_iri =
                    NamedNode::new(subclass).map_err(|e| Error::SparqlError(e.to_string()))?;
                let sup_iri =
                    NamedNode::new(superclass).map_err(|e| Error::SparqlError(e.to_string()))?;
                let rdfs_subclass =
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")
                        .map_err(|e| Error::SparqlError(e.to_string()))?;

                let triple = Triple::new(sub_iri, rdfs_subclass, sup_iri);
                let quad = Quad::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphName::DefaultGraph,
                );
                store
                    .insert(&quad)
                    .map_err(|e| Error::SparqlError(e.to_string()))?;
            }
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
                            .map_err(|e| Error::SparqlError(e.to_string()))?;
                    triples.push(Triple::new(sub_iri, rdfs_subclass, sup_iri));
                }
            }
            Axiom::ClassAssertion(class, individual) => {
                if let Some(class_iri) = self.class_expr_to_iri(class) {
                    let individual_iri = NamedNode::new(&individual.iri)
                        .map_err(|e| Error::SparqlError(e.to_string()))?;
                    let rdf_type =
                        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                            .map_err(|e| Error::SparqlError(e.to_string()))?;
                    triples.push(Triple::new(individual_iri, rdf_type, class_iri));
                }
            }
            Axiom::ObjectPropertyAssertion(prop, subj, obj) => {
                let prop_iri = NamedNode::new(&prop.to_string())
                    .map_err(|e| Error::SparqlError(e.to_string()))?;
                let subj_iri =
                    NamedNode::new(&subj.iri).map_err(|e| Error::SparqlError(e.to_string()))?;
                let obj_iri =
                    NamedNode::new(&obj.iri).map_err(|e| Error::SparqlError(e.to_string()))?;
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
    /// Value type (uri, literal, bnode)
    #[serde(rename = "type")]
    pub value_type: String,
    /// Value content
    pub value: String,
    /// Language tag (for literals)
    pub lang: Option<String>,
    /// Datatype (for literals)
    pub datatype: Option<String>,
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
) -> Result<impl Reply, warp::Rejection> {
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
) -> Result<impl Reply, warp::Rejection> {
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
        },
        Term::BlankNode(node) => SparqlValue {
            value_type: "bnode".to_string(),
            value: node.as_str().to_string(),
            lang: None,
            datatype: None,
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
            }
        }
        _ => SparqlValue {
            value_type: "unknown".to_string(),
            value: term.to_string(),
            lang: None,
            datatype: None,
        },
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
