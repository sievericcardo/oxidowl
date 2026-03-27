//! RDF Graph Skolemization and Deskolemization
//!
//! **Skolemization** replaces every blank node in an RDF graph with a fresh IRI
//! in the well-known `/.well-known/genid/` namespace (per RFC 9247 / RDF 1.2).
//! Skolemized IRIs can be published and shared safely without ambiguity.
//!
//! **Deskolemization** is the inverse operation: Skolem IRIs that were created
//! by this library (identifiable by the prefix) are replaced back with blank
//! nodes.
//!
//! # Usage
//!
//! ```rust
//! use oxidowl::semantics::{RdfGraph, RdfTerm, Triple};
//! use oxidowl::semantics::skolemization::{skolemize, deskolemize};
//!
//! // Create an RdfGraph and skolemize it
//! let mut graph = RdfGraph::new();
//! // ... populate graph ...
//! let (skolemized, mapping) = skolemize(&graph);
//! ```

use crate::semantics::{RdfGraph, RdfTerm, Triple};
use std::collections::HashMap;
use url::Url;

/// The default Skolem IRI prefix used by this library.
///
/// Per RFC 9247, Skolem IRIs should use the `/.well-known/genid/` path.
pub const SKOLEM_PREFIX: &str = "http://oxidowl.example/.well-known/genid/";

/// Skolemize an RDF graph: replace each blank node with a globally-unique IRI.
///
/// Returns the new graph together with a mapping
/// `{ blank_node_id → skolem_iri_string }`.
///
/// The Skolem IRI for a blank node with label `label` (stripped of the
/// leading `_:` if present) is:
/// ```text
/// http://oxidowl.example/.well-known/genid/<label>
/// ```
pub fn skolemize(graph: &RdfGraph) -> (RdfGraph, HashMap<String, String>) {
    let mut mapping: HashMap<String, String> = HashMap::new();
    let mut skolemized = RdfGraph::new();

    // Pre-build mapping for all blank nodes
    for triple in graph.triples() {
        collect_blank_nodes(&triple.subject, &mut mapping);
        collect_blank_nodes(&triple.predicate, &mut mapping);
        collect_blank_nodes(&triple.object, &mut mapping);
    }

    // Rewrite every triple
    for triple in graph.triples() {
        let new_triple = Triple::new(
            apply_skolem(&triple.subject, &mapping),
            apply_skolem(&triple.predicate, &mapping),
            apply_skolem(&triple.object, &mapping),
        );
        skolemized.add_triple(new_triple);
    }

    (skolemized, mapping)
}

/// Deskolemize an RDF graph: replace Skolem IRIs created by [`skolemize`]
/// back into blank nodes.
///
/// Only IRIs that start with [`SKOLEM_PREFIX`] are deskolemized; all other
/// terms are kept unchanged.
pub fn deskolemize(graph: &RdfGraph) -> RdfGraph {
    let mut result = RdfGraph::new();

    for triple in graph.triples() {
        result.add_triple(Triple::new(
            apply_deskolem(&triple.subject),
            apply_deskolem(&triple.predicate),
            apply_deskolem(&triple.object),
        ));
    }

    result
}

/// Collect blank node labels from a term into the mapping.
fn collect_blank_nodes(term: &RdfTerm, mapping: &mut HashMap<String, String>) {
    match term {
        RdfTerm::BlankNode(id) => {
            mapping.entry(id.clone()).or_insert_with(|| {
                let label = id.strip_prefix("_:").unwrap_or(id.as_str());
                format!("{SKOLEM_PREFIX}{label}")
            });
        }
        RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => {
            collect_blank_nodes(&t.subject, mapping);
            collect_blank_nodes(&t.predicate, mapping);
            collect_blank_nodes(&t.object, mapping);
        }
        _ => {}
    }
}

/// Apply the Skolem mapping to a single term, returning the rewritten term.
fn apply_skolem(term: &RdfTerm, mapping: &HashMap<String, String>) -> RdfTerm {
    match term {
        RdfTerm::BlankNode(id) => {
            if let Some(iri_str) = mapping.get(id) {
                Url::parse(iri_str)
                    .map(RdfTerm::Iri)
                    .unwrap_or_else(|_| term.clone())
            } else {
                term.clone()
            }
        }
        RdfTerm::QuotedTriple(inner) => RdfTerm::QuotedTriple(Box::new(Triple::new(
            apply_skolem(&inner.subject, mapping),
            apply_skolem(&inner.predicate, mapping),
            apply_skolem(&inner.object, mapping),
        ))),
        RdfTerm::TripleTerm(inner) => RdfTerm::TripleTerm(Box::new(Triple::new(
            apply_skolem(&inner.subject, mapping),
            apply_skolem(&inner.predicate, mapping),
            apply_skolem(&inner.object, mapping),
        ))),
        other => other.clone(),
    }
}

/// Replace a Skolem IRI back to a blank node, or return the term unchanged.
fn apply_deskolem(term: &RdfTerm) -> RdfTerm {
    match term {
        RdfTerm::Iri(url) => {
            let iri_str = url.as_str();
            if let Some(label) = iri_str.strip_prefix(SKOLEM_PREFIX) {
                RdfTerm::BlankNode(format!("_:{label}"))
            } else {
                term.clone()
            }
        }
        RdfTerm::QuotedTriple(inner) => RdfTerm::QuotedTriple(Box::new(Triple::new(
            apply_deskolem(&inner.subject),
            apply_deskolem(&inner.predicate),
            apply_deskolem(&inner.object),
        ))),
        RdfTerm::TripleTerm(inner) => RdfTerm::TripleTerm(Box::new(Triple::new(
            apply_deskolem(&inner.subject),
            apply_deskolem(&inner.predicate),
            apply_deskolem(&inner.object),
        ))),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantics::{RdfGraph, RdfTerm, Triple};
    use url::Url;

    fn iri(s: &str) -> RdfTerm {
        RdfTerm::Iri(Url::parse(s).unwrap())
    }

    fn bn(id: &str) -> RdfTerm {
        RdfTerm::BlankNode(format!("_:{id}"))
    }

    #[test]
    fn test_skolemize_blank_nodes() {
        let mut g = RdfGraph::new();
        let p = iri("http://example.org/p");
        g.add_triple(Triple::new(bn("a"), p.clone(), bn("b")));

        let (skolemized, mapping) = skolemize(&g);

        // Check mapping entries exist
        assert!(mapping.contains_key("_:a"));
        assert!(mapping.contains_key("_:b"));
        let a_iri = mapping.get("_:a").unwrap();
        assert!(a_iri.starts_with(SKOLEM_PREFIX));

        // Check no blank nodes remain
        for triple in skolemized.triples() {
            assert!(!matches!(triple.subject, RdfTerm::BlankNode(_)));
            assert!(!matches!(triple.object, RdfTerm::BlankNode(_)));
        }
    }

    #[test]
    fn test_deskolemize_round_trip() {
        let mut g = RdfGraph::new();
        let p = iri("http://example.org/p");
        g.add_triple(Triple::new(bn("foo"), p.clone(), bn("bar")));

        let (skolemized, _) = skolemize(&g);
        let restored = deskolemize(&skolemized);

        // The restored graph should have the same triple count
        assert_eq!(g.triples().len(), restored.triples().len());

        // All blank nodes should be restored
        for triple in restored.triples() {
            // Subject and object started as blank nodes
            assert!(matches!(triple.subject, RdfTerm::BlankNode(_)));
            assert!(matches!(triple.object, RdfTerm::BlankNode(_)));
        }
    }

    #[test]
    fn test_skolemize_ground_triples_unchanged() {
        let mut g = RdfGraph::new();
        let s = iri("http://example.org/s");
        let p = iri("http://example.org/p");
        let o = iri("http://example.org/o");
        g.add_triple(Triple::new(s, p, o));

        let (skolemized, mapping) = skolemize(&g);
        assert!(mapping.is_empty());
        assert_eq!(g.triples().len(), skolemized.triples().len());
    }
}
