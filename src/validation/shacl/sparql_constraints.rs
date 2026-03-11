//! SHACL-SPARQL constraint evaluator.
//!
//! Implements `sh:sparql` SELECT-based constraints.

use crate::error::{Error, Result};
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity, SparqlConstraint},
    paths::term_to_sparql,
    report::ShaclValidationResult,
    vocabulary::*,
};

/// Evaluate a `sh:sparql` constraint against `focus_node`.
///
/// Per §5.3.2 the SPARQL SELECT query is expected to bind:
/// - `?this`    → the focus node
/// - `?value`   → the offending value (optional)
/// - `?path`    → the result path (optional)
/// - `?message` → an additional message (optional)
/// - `?failure` → if `true`, the constraint generates a processing failure
pub fn evaluate_sparql_constraint(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    constraint: &SparqlConstraint,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
) -> Result<Vec<ShaclValidationResult>> {
    if constraint.deactivated {
        return Ok(Vec::new());
    }

    // Build prefix block
    let prefix_block = build_prefix_block(&constraint.prefixes);

    // Pre-bind $this to focus_node using a VALUES clause.
    let focus_str = term_to_sparql(focus_node);
    let this_binding = format!("VALUES (?this) {{ ({focus_str}) }}");

    // Inject the VALUES clause at the start of the WHERE clause.
    let full_query = inject_this_binding(
        &format!("{prefix_block}{}", constraint.select),
        &this_binding,
    );

    let rows = match store.execute_select(&full_query) {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::shacl(format!(
                "sh:sparql query execution failed: {e}"
            )));
        }
    };

    let mut results = Vec::new();

    for row in rows {
        // Check for ?failure binding
        if let Some(failure) = row.get("failure")
            && term_is_true_literal(failure)
        {
            return Err(Error::shacl(format!(
                "sh:sparql constraint reports processing failure for focus node {focus_node:?}"
            )));
        }

        // Build result from bindings
        let value = row.get("value").cloned();
        let path_term = row.get("path").cloned();
        let row_message = row.get("message").cloned();

        let mut msgs = constraint.messages.clone();
        if let Some(RdfTerm::Literal {
            value: msg_text,
            language: lang,
            ..
        }) = row_message
        {
            msgs.push(ShaclMessage {
                value: msg_text,
                language: lang,
            });
        }

        let result_path = path_term.and_then(|t| {
            if let RdfTerm::Iri(iri) = t {
                Some(crate::validation::shacl::model::ShaclPath::Predicate(
                    iri.as_str().to_string(),
                ))
            } else {
                None
            }
        });

        results.push(ShaclValidationResult {
            focus_node: focus_node.clone(),
            result_path,
            value,
            source_shape: source_shape.cloned(),
            source_constraint_component: SH_SPARQL_CONSTRAINT_COMPONENT.to_string(),
            severity: severity.clone(),
            messages: msgs,
            details: Vec::new(),
        });
    }

    Ok(results)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_prefix_block(prefixes: &[(String, String)]) -> String {
    let mut out = String::new();
    for (prefix, ns) in prefixes {
        out.push_str(&format!("PREFIX {prefix}: <{ns}>\n"));
    }
    out
}

/// Inject a VALUES clause immediately after the `WHERE {` in a SPARQL SELECT.
fn inject_this_binding(query: &str, values_clause: &str) -> String {
    // Find `WHERE {` (case-insensitive) and inject after the opening brace.
    let lower = query.to_ascii_lowercase();
    if let Some(pos) = lower.find("where")
        && let Some(brace_pos) = query[pos..].find('{')
    {
        let insert_at = pos + brace_pos + 1;
        let (before, after) = query.split_at(insert_at);
        return format!("{before} {values_clause} {after}");
    }
    // Fallback: prepend
    format!("{values_clause} {query}")
}

fn term_is_true_literal(term: &RdfTerm) -> bool {
    matches!(
        term,
        RdfTerm::Literal { value, .. } if value == "true" || value == "1"
    )
}
