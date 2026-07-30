//! RDF Graph Isomorphism
//!
//! Two RDF graphs are **isomorphic** if there is a bijection between their
//! blank nodes that produces the same set of triples.
//!
//! This module implements the RDF blank-node-based isomorphism check using
//! the iterative "blank node signature" approach:
//!
//! 1. Build a multiset signature for each blank node from the triples it
//!    appears in (treating IRIs and literals as literal atoms, and other blank
//!    nodes by their current label).
//! 2. Sort and hash signatures to assign canonical labels.
//! 3. Iterate until stable — this gives a canonical partition of blank nodes.
//! 4. Compare the canonicalized graphs for literal equality.
//!
//! Complexity is polynomial for most realistic graphs. For pathological cases
//! (many automorphically-equivalent blank nodes) exhaustive search might
//! be required for pathological cases with many automorphically-equivalent
//! blank nodes, but that exhaustive search is not needed for typical
//! ontology graphs where the signature-refinement approach suffices.

use crate::semantics::{RdfGraph, RdfTerm, Triple};
use std::collections::{HashMap, HashSet};

/// Check whether two RDF graphs are isomorphic.
///
/// Graphs are isomorphic if there exists a bijection on blank nodes that
/// maps every triple in `a` to a triple in `b` and vice-versa.
pub fn are_isomorphic(a: &RdfGraph, b: &RdfGraph) -> bool {
    // Fast path: if they have different numbers of triples they cannot be isomorphic.
    if a.triples().len() != b.triples().len() {
        return false;
    }

    let a_canon = canonicalize(a);
    let b_canon = canonicalize(b);
    a_canon == b_canon
}

/// Return a sorted, canonical representation of the graph.
///
/// All blank nodes are replaced by deterministic labels derived from their
/// structural signature.  The result is a `Vec` of sorted triple strings that
/// can be compared for equality.
pub fn canonicalize(graph: &RdfGraph) -> Vec<String> {
    let mapping = canonical_blank_node_mapping(graph);

    let mut triples: Vec<String> = graph
        .triples()
        .iter()
        .map(|t| {
            let s = apply_mapping(&t.subject, &mapping);
            let p = apply_mapping(&t.predicate, &mapping);
            let o = apply_mapping(&t.object, &mapping);
            format!("{s} {p} {o}")
        })
        .collect();
    triples.sort();
    triples
}

/// Build a canonical mapping `{blank_node_id → canonical_id}` by iterative
/// signature refinement (a simplified form of RDNA).
fn canonical_blank_node_mapping(graph: &RdfGraph) -> HashMap<String, String> {
    let triples: Vec<&Triple> = graph.triples().iter().collect();

    // Collect all blank node IDs
    let mut blank_ids: Vec<String> = {
        let mut set: HashSet<String> = HashSet::new();
        for t in &triples {
            if let RdfTerm::BlankNode(id) = &t.subject {
                set.insert(id.clone());
            }
            if let RdfTerm::BlankNode(id) = &t.object {
                set.insert(id.clone());
            }
        }
        set.into_iter().collect()
    };
    blank_ids.sort();

    if blank_ids.is_empty() {
        return HashMap::new();
    }

    // Initial labels: sorted index
    let mut labels: HashMap<String, String> = blank_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), format!("b{i}")))
        .collect();

    // Iterate until stable
    for _ in 0..20 {
        let new_labels = refine_labels(&triples, &labels);
        if new_labels == labels {
            break;
        }
        labels = new_labels;
    }

    // Assign dense canonical names based on sorted final labels
    let mut label_pairs: Vec<(String, String)> = labels.into_iter().collect();
    label_pairs.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    label_pairs
        .into_iter()
        .enumerate()
        .map(|(i, (original, _))| (original, format!("_:c{i}")))
        .collect()
}

/// One iteration of the label-refinement step.
fn refine_labels(triples: &[&Triple], labels: &HashMap<String, String>) -> HashMap<String, String> {
    // For each blank node, compute a signature from the triples it participates in.
    let mut signatures: HashMap<String, Vec<String>> = HashMap::new();

    for t in triples {
        let s_str = term_signature(&t.subject, labels);
        let p_str = term_signature(&t.predicate, labels);
        let o_str = term_signature(&t.object, labels);

        // Contribute the triple's signature to each blank node in it.
        if let RdfTerm::BlankNode(id) = &t.subject {
            signatures
                .entry(id.clone())
                .or_default()
                .push(format!("s:{p_str}:{o_str}"));
        }
        if let RdfTerm::BlankNode(id) = &t.object {
            signatures
                .entry(id.clone())
                .or_default()
                .push(format!("o:{s_str}:{p_str}"));
        }
    }

    // Sort each signature list so order of triples doesn't matter.
    let mut new_labels: HashMap<String, String> = HashMap::new();
    for (id, mut sigs) in signatures {
        sigs.sort();
        let combined = sigs.join("|");
        // Hash the combined signature deterministically.
        let hash = simple_hash(&combined);
        new_labels.insert(id, format!("h{hash:016x}"));
    }

    // Any blank nodes that had no triples keep their original label.
    for (id, label) in labels {
        new_labels
            .entry(id.clone())
            .or_insert_with(|| label.clone());
    }

    new_labels
}

/// Compute a string representation of a term for signature purposes.
fn term_signature(term: &RdfTerm, labels: &HashMap<String, String>) -> String {
    match term {
        RdfTerm::Iri(url) => format!("<{url}>"),
        RdfTerm::BlankNode(id) => labels.get(id).cloned().unwrap_or_else(|| format!("_:{id}")),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            if let Some(lang) = language {
                format!("\"{value}\"@{lang}")
            } else if let Some(dt) = datatype {
                format!("\"{value}\"^^<{dt}>")
            } else {
                format!("\"{value}\"")
            }
        }
        RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => {
            format!(
                "<<{} {} {}>>",
                term_signature(&t.subject, labels),
                term_signature(&t.predicate, labels),
                term_signature(&t.object, labels)
            )
        }
    }
}

/// Apply the canonical mapping to a term for output.
fn apply_mapping(term: &RdfTerm, mapping: &HashMap<String, String>) -> String {
    match term {
        RdfTerm::BlankNode(id) => mapping
            .get(id)
            .cloned()
            .unwrap_or_else(|| format!("_:{id}")),
        other => term_signature(other, mapping),
    }
}

/// A deterministic (but not cryptographic) hash of a string.
fn simple_hash(s: &str) -> u64 {
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
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
    fn test_isomorphic_blank_node_graphs() {
        let mut g1 = RdfGraph::new();
        let mut g2 = RdfGraph::new();

        let p = iri("http://example.org/p");
        let o = iri("http://example.org/o");

        // g1: _:a p o .
        g1.add_triple(Triple::new(bn("a"), p.clone(), o.clone()));
        // g2: _:x p o .  — same structure, different blank node label
        g2.add_triple(Triple::new(bn("x"), p, o));

        assert!(are_isomorphic(&g1, &g2));
    }

    #[test]
    fn test_non_isomorphic_graphs() {
        let mut g1 = RdfGraph::new();
        let mut g2 = RdfGraph::new();

        let p = iri("http://example.org/p");
        let o1 = iri("http://example.org/o1");
        let o2 = iri("http://example.org/o2");

        g1.add_triple(Triple::new(bn("a"), p.clone(), o1.clone()));
        g2.add_triple(Triple::new(bn("x"), p, o2));

        assert!(!are_isomorphic(&g1, &g2));
    }

    #[test]
    fn test_ground_graphs_isomorphic() {
        let mut g1 = RdfGraph::new();
        let mut g2 = RdfGraph::new();

        let s = iri("http://example.org/s");
        let p = iri("http://example.org/p");
        let o = iri("http://example.org/o");

        g1.add_triple(Triple::new(s.clone(), p.clone(), o.clone()));
        g2.add_triple(Triple::new(s, p, o));

        assert!(are_isomorphic(&g1, &g2));
    }
}
