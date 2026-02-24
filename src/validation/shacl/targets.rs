//! SHACL target resolution.
//!
//! Resolves the focus nodes for a shape by evaluating all declared
//! `ShaclTarget` expressions against the data graph.

use std::collections::HashSet;

use crate::error::Result;
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::model::ShaclTarget;
use crate::validation::shacl::paths::term_to_sparql;

/// Resolve all focus nodes produced by `targets` against `store`.
///
/// Returns a deduplicated `HashSet<RdfTerm>` containing every node that
/// must be validated.
pub fn resolve_targets(
    store: &SparqlStore,
    targets: &[ShaclTarget],
) -> Result<HashSet<RdfTerm>> {
    let mut result: HashSet<RdfTerm> = HashSet::new();

    for target in targets {
        let nodes = resolve_single_target(store, target)?;
        result.extend(nodes);
    }

    Ok(result)
}

fn resolve_single_target(
    store: &SparqlStore,
    target: &ShaclTarget,
) -> Result<Vec<RdfTerm>> {
    match target {
        // §2.1.3.1  sh:targetNode — the term itself is the focus node.
        ShaclTarget::TargetNode(term) => Ok(vec![term.clone()]),

        // §2.1.3.2  sh:targetClass — all instances of class (transitive)
        ShaclTarget::TargetClass(class) | ShaclTarget::ImplicitClassTarget(class) => {
            let class_str = term_to_sparql(class);
            let query = format!(
                "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
                 PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
                 SELECT DISTINCT ?this WHERE {{ \
                     ?this rdf:type/rdfs:subClassOf* {class_str} \
                 }}"
            );
            collect_var(store, &query, "this")
        }

        // §2.1.3.3  sh:targetSubjectsOf — all subjects of a predicate.
        ShaclTarget::TargetSubjectsOf(pred) => {
            let pred_str = term_to_sparql(pred);
            let query = format!(
                "SELECT DISTINCT ?this WHERE {{ ?this {pred_str} ?_any }}"
            );
            collect_var(store, &query, "this")
        }

        // §2.1.3.4  sh:targetObjectsOf — all objects of a predicate.
        ShaclTarget::TargetObjectsOf(pred) => {
            let pred_str = term_to_sparql(pred);
            let query = format!(
                "SELECT DISTINCT ?this WHERE {{ ?_any {pred_str} ?this }}"
            );
            collect_var(store, &query, "this")
        }
    }
}

/// Execute a SELECT query and collect a single variable into a `Vec<RdfTerm>`.
fn collect_var(store: &SparqlStore, query: &str, var: &str) -> Result<Vec<RdfTerm>> {
    let rows = store.execute_select(query)?;
    let mut out = Vec::new();
    for row in rows {
        if let Some(term) = row.get(var) {
            out.push(term.clone());
        }
    }
    Ok(out)
}
