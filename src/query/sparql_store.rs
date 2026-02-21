//! In-process SPARQL store for Oxidowl
//!
//! This module provides a [`SparqlStore`] struct that wraps an `oxigraph::Store`
//! and exposes methods for loading ontology data, performing incremental triple
//! mutations, and executing SPARQL SELECT / ASK / CONSTRUCT / UPDATE operations
//! entirely in-process — no HTTP server required.
//!
//! It is the building block consumed by the SMOL interpreter's `TripleManager`
//! so that every interpreter step can query and update the semantic model
//! without network round-trips.
//!
//! # Feature gate
//! This module is compiled only when the `sparql-store` feature is enabled.

use crate::{
    Error, Result,
    ontology::{Axiom, ClassExpression, Ontology},
    reasoning::ReasoningService,
    semantics::{RdfTerm, Triple as OxidowlTriple},
};
use oxigraph::{
    io::{RdfFormat, RdfParser},
    model::{
        BlankNode as OxBlankNode, GraphName, Literal as OxLiteral, NamedNode,
        NamedOrBlankNode, Quad, Term, Triple as OxTriple,
    },
    sparql::{QueryResults, SparqlEvaluator},
    store::Store,
};
use std::{collections::HashMap, sync::Arc};
use url::Url;

// ---------------------------------------------------------------------------
// XSD constants
// ---------------------------------------------------------------------------

mod xsd {
    pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    pub const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
    pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
}

// ---------------------------------------------------------------------------
// SparqlStore
// ---------------------------------------------------------------------------

/// An in-process SPARQL store backed by Oxigraph.
///
/// Use [`SparqlStore::new`] to create an empty store, or
/// [`SparqlStore::from_ontology`] to pre-populate it from an [`Ontology`] and
/// its inferred classification hierarchy.
pub struct SparqlStore {
    store: Store,
}

impl SparqlStore {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a new, empty in-process SPARQL store.
    ///
    /// # Errors
    /// Returns an error if the underlying Oxigraph store fails to initialise.
    pub fn new() -> Result<Self> {
        let store = Store::new().map_err(|e| Error::Sparql {
            message: format!("Failed to create SPARQL store: {e}"),
        })?;
        Ok(Self { store })
    }

    /// Create a store pre-populated from an [`Ontology`] plus inferred triples
    /// produced by `reasoning_service`.
    ///
    /// This is the primary constructor used by the SMOL interpreter at startup.
    ///
    /// # Errors
    /// Returns an error if the reasoning service cannot be accessed or if any
    /// triple conversion fails.
    pub async fn from_ontology(
        ontology: &Ontology,
        reasoning_service: &Arc<ReasoningService>,
    ) -> Result<Self> {
        let mut store_wrapper = Self::new()?;

        // Insert axiom-derived triples
        for axiom in &ontology.axioms {
            let triples = axiom_to_oxtriples(axiom)?;
            for triple in triples {
                store_wrapper.insert_oxtriple(triple)?;
            }
        }

        // Insert inferred subClassOf hierarchy
        let classification = reasoning_service.classify().await?;
        let rdfs_sub = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")
            .map_err(|e| Error::Sparql { message: e.to_string() })?;
        for (subclass, superclasses) in &classification.hierarchy {
            if let Some(sub_iri) = class_expr_to_named_node(subclass) {
                for superclass in superclasses {
                    if let Some(sup_iri) = class_expr_to_named_node(superclass) {
                        store_wrapper.insert_oxtriple(OxTriple::new(
                            sub_iri.clone(),
                            rdfs_sub.clone(),
                            sup_iri,
                        ))?;
                    }
                }
            }
        }

        // Insert RDF-star triples from the ontology graph (if any)
        if let Some(rdf_graph) = ontology.get_rdf_graph() {
            for oxidowl_triple in rdf_graph.triples() {
                if let Ok(quad) = oxidowl_triple_to_quad(oxidowl_triple) {
                    store_wrapper.store.insert(&quad).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                }
            }
        }

        Ok(store_wrapper)
    }

    // -----------------------------------------------------------------------
    // Incremental mutation
    // -----------------------------------------------------------------------

    /// Insert a batch of oxidowl triples into the default graph.
    ///
    /// Silently skips quads that cannot be converted (e.g. quoted triples in
    /// subject position that require the `rdf-12` oxigraph feature).
    ///
    /// # Errors
    /// Returns an error if the underlying store rejects an insert.
    pub fn update_from_triples(&mut self, triples: &[OxidowlTriple]) -> Result<()> {
        for t in triples {
            match oxidowl_triple_to_quad(t) {
                Ok(quad) => {
                    self.store.insert(&quad).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                }
                Err(e) => {
                    tracing::warn!("Skipping unconvertible triple: {e}");
                }
            }
        }
        Ok(())
    }

    /// Remove a batch of oxidowl triples from the default graph.
    ///
    /// # Errors
    /// Returns an error if the underlying store rejects a removal.
    pub fn remove_triples(&mut self, triples: &[OxidowlTriple]) -> Result<()> {
        for t in triples {
            match oxidowl_triple_to_quad(t) {
                Ok(quad) => {
                    self.store.remove(&quad).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                }
                Err(e) => {
                    tracing::warn!("Skipping unconvertible triple during remove: {e}");
                }
            }
        }
        Ok(())
    }

    /// Insert a single raw Oxigraph triple into the default graph.
    fn insert_oxtriple(&mut self, triple: OxTriple) -> Result<()> {
        let quad = Quad::new(
            triple.subject,
            triple.predicate,
            triple.object,
            GraphName::DefaultGraph,
        );
        self.store.insert(&quad).map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Bulk-load a Turtle document into the default graph.
    ///
    /// This is the primary way to load background ontologies; it is far more
    /// efficient than inserting triples one-by-one via SPARQL UPDATE.
    ///
    /// # Errors
    /// Returns an error if the Turtle cannot be parsed or the store rejects a
    /// triple.
    pub fn load_turtle(&mut self, turtle: &str) -> Result<()> {
        self.store
            .load_from_slice(
                RdfParser::from_format(RdfFormat::Turtle),
                turtle,
            )
            .map_err(|e| Error::Sparql {
                message: format!("Failed to bulk-load Turtle: {e}"),
            })
    }

    // -----------------------------------------------------------------------
    // SPARQL query execution
    // -----------------------------------------------------------------------

    /// Execute a SPARQL SELECT query.
    ///
    /// Returns each solution row as a `HashMap<variable_name, RdfTerm>`.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_select(&self, query: &str) -> Result<Vec<HashMap<String, RdfTerm>>> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Solutions(solutions) => {
                let mut rows = Vec::new();
                for solution in solutions {
                    let sol = solution.map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    let row: HashMap<String, RdfTerm> = sol
                        .iter()
                        .map(|(var, term)| {
                            (var.as_str().to_string(), oxterm_to_rdfterm(term))
                        })
                        .collect();
                    rows.push(row);
                }
                Ok(rows)
            }
            _ => Err(Error::Sparql {
                message: "Expected SELECT query results but got ASK/CONSTRUCT".to_string(),
            }),
        }
    }

    /// Execute a SPARQL ASK query.  Returns `true` if the pattern matches.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_ask(&self, query: &str) -> Result<bool> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Boolean(b) => Ok(b),
            _ => Err(Error::Sparql {
                message: "Expected ASK query results but got SELECT/CONSTRUCT".to_string(),
            }),
        }
    }

    /// Execute a SPARQL CONSTRUCT query.
    ///
    /// Returns the resulting graph as a `Vec<OxidowlTriple>`.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_construct(&self, query: &str) -> Result<Vec<OxidowlTriple>> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Graph(graph) => {
                let mut triples = Vec::new();
                for triple_result in graph {
                    let t = triple_result.map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    triples.push(quad_triple_to_oxidowl(&t)?);
                }
                Ok(triples)
            }
            _ => Err(Error::Sparql {
                message: "Expected CONSTRUCT/DESCRIBE query results but got SELECT/ASK"
                    .to_string(),
            }),
        }
    }

    /// Execute a SPARQL UPDATE operation (INSERT DATA / DELETE DATA / etc.).
    ///
    /// # Errors
    /// Returns an error if the update fails to parse or execute.
    pub fn execute_update(&mut self, update: &str) -> Result<()> {
        self.store.update(update).map_err(|e| Error::Sparql {
            message: format!("SPARQL update failed: {e}"),
        })
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Execute any SPARQL query via the Oxigraph `SparqlEvaluator` API.
    fn run_query(&self, query: &str) -> Result<QueryResults> {
        SparqlEvaluator::new()
            .parse_query(query)
            .map_err(|e| Error::Sparql {
                message: format!("SPARQL parse error: {e}"),
            })?
            .on_store(&self.store)
            .execute()
            .map_err(|e| Error::Sparql {
                message: format!("SPARQL execution error: {e}"),
            })
    }

    /// Return a reference to the raw Oxigraph store (for advanced use-cases).
    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Return a mutable reference to the raw Oxigraph store.
    #[must_use]
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers: oxidowl → oxigraph
// ---------------------------------------------------------------------------

/// Convert an oxidowl axiom to a (possibly empty) list of raw Oxigraph triples.
fn axiom_to_oxtriples(axiom: &Axiom) -> Result<Vec<OxTriple>> {
    let mut triples = Vec::new();

    match axiom {
        Axiom::SubClassOf(ax) => {
            if let (Some(sub_iri), Some(sup_iri)) =
                (class_expr_to_named_node(&ax.subclass), class_expr_to_named_node(&ax.superclass))
            {
                let rdfs_sub =
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;
                triples.push(OxTriple::new(sub_iri, rdfs_sub, sup_iri));
            }
        }
        Axiom::ClassAssertion(ax) => {
            if let Some(class_iri) = class_expr_to_named_node(&ax.class) {
                if let Some(ind_str) = individual_iri_str(&ax.individual) {
                    let ind_iri =
                        NamedNode::new(ind_str).map_err(|e| Error::Sparql {
                            message: e.to_string(),
                        })?;
                    let rdf_type =
                        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                            .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    triples.push(OxTriple::new(ind_iri, rdf_type, class_iri));
                }
            }
        }
        Axiom::ObjectPropertyAssertion(ax) => {
            if let Some(prop_str) = obj_prop_expr_iri_str(&ax.property) {
                if let (Some(subj_str), Some(obj_str)) =
                    (individual_iri_str(&ax.source), individual_iri_str(&ax.target))
                {
                    let prop_iri = NamedNode::new(prop_str).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    let subj_iri = NamedNode::new(subj_str).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    let obj_iri = NamedNode::new(obj_str).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    triples.push(OxTriple::new(subj_iri, prop_iri, obj_iri));
                }
            }
        }
        _ => {} // remaining axiom types handled by the reasoner classification
    }

    Ok(triples)
}

/// Convert an atomic `ClassExpression` to a `NamedNode`; returns `None` for
/// complex expressions that cannot be mapped to a single IRI.
fn class_expr_to_named_node(expr: &ClassExpression) -> Option<NamedNode> {
    match expr {
        ClassExpression::Class(cls) => NamedNode::new(cls.iri.as_str()).ok(),
        _ => None,
    }
}

/// Extract the IRI string from an `Individual`, returning `None` for anonymous individuals.
fn individual_iri_str(ind: &crate::ontology::Individual) -> Option<&str> {
    match ind {
        crate::ontology::Individual::Named(n) => Some(n.iri.as_str()),
        crate::ontology::Individual::Anonymous(_) => None,
    }
}

/// Extract the IRI string from an `ObjectPropertyExpression`, returning `None` for chains.
fn obj_prop_expr_iri_str(expr: &crate::ontology::ObjectPropertyExpression) -> Option<&str> {
    match expr {
        crate::ontology::ObjectPropertyExpression::ObjectProperty(p) => Some(p.iri.as_str()),
        crate::ontology::ObjectPropertyExpression::InverseObjectProperty(p) => Some(p.iri.as_str()),
        crate::ontology::ObjectPropertyExpression::PropertyChain(_) => None,
    }
}

/// Convert an oxidowl `Triple` to an Oxigraph `Quad` in the default graph.
pub fn oxidowl_triple_to_quad(t: &OxidowlTriple) -> Result<Quad> {
    let subject = rdfterm_to_oxsubject(&t.subject)?;
    let predicate = rdfterm_to_oxpredicate(&t.predicate)?;
    let object = rdfterm_to_oxterm(&t.object)?;
    Ok(Quad::new(subject, predicate, object, GraphName::DefaultGraph))
}

fn rdfterm_to_oxsubject(term: &RdfTerm) -> Result<NamedOrBlankNode> {
    match term {
        RdfTerm::Iri(url) => {
            let node = NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(NamedOrBlankNode::NamedNode(node))
        }
        RdfTerm::BlankNode(id) => {
            let node = OxBlankNode::new(id).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(NamedOrBlankNode::BlankNode(node))
        }
        RdfTerm::QuotedTriple(_) => Err(Error::Sparql {
            message:
                "Quoted triples in subject position require the rdf-12 oxigraph feature"
                    .to_string(),
        }),
        _ => Err(Error::Sparql {
            message: format!("Invalid RDF term in subject position: {term:?}"),
        }),
    }
}

fn rdfterm_to_oxpredicate(term: &RdfTerm) -> Result<NamedNode> {
    match term {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
            message: e.to_string(),
        }),
        _ => Err(Error::Sparql {
            message: format!("Predicate must be an IRI, got: {term:?}"),
        }),
    }
}

fn rdfterm_to_oxterm(term: &RdfTerm) -> Result<Term> {
    match term {
        RdfTerm::Iri(url) => {
            let node = NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(Term::NamedNode(node))
        }
        RdfTerm::BlankNode(id) => {
            let node = OxBlankNode::new(id).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(Term::BlankNode(node))
        }
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                OxLiteral::new_language_tagged_literal(value, lang).map_err(|e| {
                    Error::Sparql {
                        message: e.to_string(),
                    }
                })?
            } else if let Some(dt_url) = datatype {
                let dt_node = NamedNode::new(dt_url.as_str()).map_err(|e| Error::Sparql {
                    message: e.to_string(),
                })?;
                OxLiteral::new_typed_literal(value, dt_node)
            } else {
                OxLiteral::new_simple_literal(value)
            };
            Ok(Term::Literal(lit))
        }
        RdfTerm::QuotedTriple(_) => Err(Error::Sparql {
            message:
                "Quoted triples in object position require the rdf-12 oxigraph feature"
                    .to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers: oxigraph → oxidowl
// ---------------------------------------------------------------------------

/// Convert an Oxigraph `Term` to an oxidowl `RdfTerm`.
pub fn oxterm_to_rdfterm(term: &Term) -> RdfTerm {
    match term {
        Term::NamedNode(n) => {
            // Best-effort URL parse; fall back to opaque IRI as BlankNode label
            match Url::parse(n.as_str()) {
                Ok(url) => RdfTerm::Iri(url),
                Err(_) => RdfTerm::BlankNode(n.as_str().to_string()),
            }
        }
        Term::BlankNode(b) => RdfTerm::BlankNode(b.as_str().to_string()),
        Term::Literal(lit) => {
            let datatype = {
                let dt_str = lit.datatype().as_str();
                if dt_str == xsd::STRING {
                    None
                } else {
                    Url::parse(dt_str).ok()
                }
            };
            let language = lit.language().map(|l| l.to_string());
            RdfTerm::Literal {
                value: lit.value().to_string(),
                datatype,
                language,
                direction: None,
            }
        }
        _ => RdfTerm::BlankNode(term.to_string()),
    }
}

fn ox_subject_to_rdfterm(subj: &NamedOrBlankNode) -> RdfTerm {
    match subj {
        NamedOrBlankNode::NamedNode(n) => {
            Url::parse(n.as_str())
                .map(RdfTerm::Iri)
                .unwrap_or_else(|_| RdfTerm::BlankNode(n.as_str().to_string()))
        }
        NamedOrBlankNode::BlankNode(b) => RdfTerm::BlankNode(b.as_str().to_string()),
    }
}

fn ox_named_node_to_rdfterm(n: &NamedNode) -> RdfTerm {
    Url::parse(n.as_str())
        .map(RdfTerm::Iri)
        .unwrap_or_else(|_| RdfTerm::BlankNode(n.as_str().to_string()))
}

/// Convert an Oxigraph `Triple` (from a CONSTRUCT result) to an oxidowl `Triple`.
fn quad_triple_to_oxidowl(t: &OxTriple) -> Result<OxidowlTriple> {
    Ok(OxidowlTriple::new(
        ox_subject_to_rdfterm(&t.subject),
        ox_named_node_to_rdfterm(&t.predicate),
        oxterm_to_rdfterm(&t.object),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_store_creation() {
        let store = SparqlStore::new();
        assert!(store.is_ok());
    }

    #[test]
    fn test_insert_and_select() {
        let mut s = SparqlStore::new().unwrap();

        // Insert a triple via SPARQL UPDATE
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let rows = s
            .execute_select(
                "PREFIX : <http://example.org/>  \
                 SELECT ?o WHERE { :alice :knows ?o }",
            )
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains_key("o"));
    }

    #[test]
    fn test_ask() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let result = s
            .execute_ask(
                "PREFIX : <http://example.org/>  \
                 ASK { :alice :knows :bob }",
            )
            .unwrap();
        assert!(result);

        let result2 = s
            .execute_ask(
                "PREFIX : <http://example.org/>  \
                 ASK { :alice :knows :charlie }",
            )
            .unwrap();
        assert!(!result2);
    }

    #[test]
    fn test_construct() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let triples = s
            .execute_construct(
                "PREFIX : <http://example.org/>  \
                 CONSTRUCT { ?s :knows ?o } WHERE { ?s :knows ?o }",
            )
            .unwrap();
        assert_eq!(triples.len(), 1);
    }

    #[test]
    fn test_incremental_updates() {
        let mut s = SparqlStore::new().unwrap();

        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/alice").unwrap(),
            RdfTerm::iri("http://example.org/knows").unwrap(),
            RdfTerm::iri("http://example.org/bob").unwrap(),
        );

        s.update_from_triples(&[triple.clone()]).unwrap();

        let count = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count, 1);

        s.remove_triples(&[triple]).unwrap();

        let count2 = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count2, 0);
    }
}
