//! SHACL property-pair constraint evaluators.
//!
//! Implements `sh:equals`, `sh:disjoint`, `sh:lessThan`, and
//! `sh:lessThanOrEquals`.

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::error::Result;
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    constraints::literal_compare::compare_terms,
    model::{ShaclMessage, ShaclPath, ShaclSeverity},
    paths::resolve_values,
    report::ShaclValidationResult,
    vocabulary::*,
};

fn make_result(
    focus_node: &RdfTerm,
    value_opt: Option<&RdfTerm>,
    component: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
    default_msg: &str,
) -> ShaclValidationResult {
    ShaclValidationResult {
        focus_node: focus_node.clone(),
        result_path: None,
        value: value_opt.cloned(),
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

// ── sh:equals ─────────────────────────────────────────────────────────────────

/// Evaluate `sh:equals <other_prop>`.
///
/// The set of value nodes via the shape path must equal the set of values for
/// `other_prop` on the focus node.
pub fn evaluate_equals(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    other_prop: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let other_path = ShaclPath::Predicate(iri_str(other_prop));
    let other_values = resolve_values(store, focus_node, &other_path)?;

    let set_a: HashSet<_> = values.iter().collect();
    let set_b: HashSet<_> = other_values.iter().collect();

    let mut out = Vec::new();
    for v in set_a.difference(&set_b) {
        out.push(make_result(
            focus_node,
            Some(v),
            SH_EQUALS_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            "sh:equals constraint: value present in shape path but not in other property",
        ));
    }
    for v in set_b.difference(&set_a) {
        out.push(make_result(
            focus_node,
            Some(v),
            SH_EQUALS_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            "sh:equals constraint: value present in other property but not in shape path",
        ));
    }
    Ok(out)
}

// ── sh:disjoint ───────────────────────────────────────────────────────────────

pub fn evaluate_disjoint(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    other_prop: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let other_path = ShaclPath::Predicate(iri_str(other_prop));
    let other_values = resolve_values(store, focus_node, &other_path)?;

    let set_b: HashSet<_> = other_values.iter().collect();
    let mut out = Vec::new();

    for v in values {
        if set_b.contains(v) {
            out.push(make_result(
                focus_node,
                Some(v),
                SH_DISJOINT_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "sh:disjoint constraint: value appears in both property sets",
            ));
        }
    }
    Ok(out)
}

// ── sh:lessThan / sh:lessThanOrEquals ─────────────────────────────────────────

pub fn evaluate_less_than(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    other_prop: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    compare_pairs(
        store,
        focus_node,
        values,
        other_prop,
        |ord| ord == Ordering::Less,
        SH_LESS_THAN_CONSTRAINT_COMPONENT,
        "sh:lessThan constraint violated",
        severity,
        source_shape,
        messages,
    )
}

pub fn evaluate_less_than_or_equals(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    other_prop: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    compare_pairs(
        store,
        focus_node,
        values,
        other_prop,
        |ord| ord != Ordering::Greater,
        SH_LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT,
        "sh:lessThanOrEquals constraint violated",
        severity,
        source_shape,
        messages,
    )
}

fn compare_pairs(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    other_prop: &RdfTerm,
    check: impl Fn(Ordering) -> bool,
    component: &str,
    default_msg: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let other_path = ShaclPath::Predicate(iri_str(other_prop));
    let other_values = resolve_values(store, focus_node, &other_path)?;

    let mut out = Vec::new();
    for a in values {
        for b in &other_values {
            let ord = compare_terms(a, b);
            let passes = ord.map(|o| check(o)).unwrap_or(false);
            if !passes {
                out.push(make_result(
                    focus_node,
                    Some(a),
                    component,
                    severity,
                    source_shape,
                    messages,
                    default_msg,
                ));
            }
        }
    }
    Ok(out)
}

fn iri_str(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => iri.as_str().to_string(),
        other => format!("{other:?}"),
    }
}
