//! SHACL "other" constraint evaluators.
//!
//! Implements `sh:closed`, `sh:hasValue`, and `sh:in`.

use crate::error::Result;
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity},
    paths::term_to_sparql,
    report::ShaclValidationResult,
    vocabulary::*,
};

fn simple_result(
    focus_node: &RdfTerm,
    value: Option<&RdfTerm>,
    component: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
    default_msg: &str,
) -> ShaclValidationResult {
    ShaclValidationResult {
        focus_node: focus_node.clone(),
        result_path: None,
        value: value.cloned(),
        source_shape: source_shape.cloned(),
        source_constraint_component: component.to_string(),
        severity: severity.clone(),
        messages: if messages.is_empty() {
            vec![ShaclMessage::plain(default_msg)]
        } else {
            messages.to_vec()
        },
        details: Vec::new(),
    }
}

// ── sh:closed ─────────────────────────────────────────────────────────────────

/// Evaluate `sh:closed true` with `sh:ignoredProperties`.
///
/// Collects all predicates used in triples where the focus node is the subject.
/// Any predicate not in `allowed_predicates ∪ ignored` produces a violation.
pub fn evaluate_closed(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    allowed_predicates: &[RdfTerm],
    ignored_properties: &[RdfTerm],
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let focus_str = term_to_sparql(focus_node);
    let query = format!(
        "SELECT DISTINCT ?pred WHERE {{ {focus_str} ?pred ?any }}"
    );
    let rows = store.execute_select(&query)?;

    let is_allowed = |pred_term: &RdfTerm| -> bool {
        allowed_predicates.contains(pred_term) || ignored_properties.contains(pred_term)
    };

    // Always allow rdf:type
    let rdf_type = RdfTerm::iri(RDF_TYPE).ok();
    let is_rdf_type = |t: &RdfTerm| rdf_type.as_ref().map(|rt| rt == t).unwrap_or(false);

    let mut out = Vec::new();
    for row in rows {
        if let Some(pred) = row.get("pred") {
            if !is_rdf_type(pred) && !is_allowed(pred) {
                out.push(simple_result(
                    focus_node,
                    Some(pred),
                    SH_CLOSED_CONSTRAINT_COMPONENT,
                    severity,
                    source_shape,
                    messages,
                    &format!("sh:closed violated: predicate not in allowed set"),
                ));
            }
        }
    }
    Ok(out)
}

// ── sh:hasValue ───────────────────────────────────────────────────────────────

/// Evaluate `sh:hasValue <required_value>`.
///
/// The required RDF term must be among the value nodes.
pub fn evaluate_has_value(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    required_value: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    if values.contains(required_value) {
        Vec::new()
    } else {
        vec![simple_result(
            focus_node,
            None,
            SH_HAS_VALUE_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            "sh:hasValue constraint: required value not present",
        )]
    }
}

// ── sh:in ─────────────────────────────────────────────────────────────────────

/// Evaluate `sh:in <list>`.
///
/// Every value node must be a member of `allowed`.
pub fn evaluate_in(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    allowed: &[RdfTerm],
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut out = Vec::new();
    for value in values {
        if !allowed.contains(value) {
            out.push(simple_result(
                focus_node,
                Some(value),
                SH_IN_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "sh:in constraint: value not in allowed list",
            ));
        }
    }
    out
}
