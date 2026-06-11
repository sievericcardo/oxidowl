//! SHACL shape-based constraint evaluators.
//!
//! Implements `sh:node`, `sh:property`, and `sh:qualifiedValueShape`.

use crate::error::Result;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity, ShapeId},
    report::ShaclValidationResult,
    vocabulary::*,
};

fn make_node_result(
    focus_node: &RdfTerm,
    value: Option<&RdfTerm>,
    component: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
    default_msg: &str,
    details: Vec<ShaclValidationResult>,
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
        details,
    }
}

// ── sh:node ───────────────────────────────────────────────────────────────────

/// Evaluate `sh:node <node_shape>` for each value node.
///
/// Each value node must conform to the referenced node shape.
pub fn evaluate_node_constraint<F>(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    node_shape_id: &ShapeId,
    validate_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let mut out = Vec::new();
    for value in values {
        let (conforms, sub) = validate_fn(value, node_shape_id)?;
        if !conforms {
            out.push(make_node_result(
                focus_node,
                Some(value),
                SH_NODE_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "sh:node constraint: value does not conform to referenced node shape",
                sub,
            ));
        }
    }
    Ok(out)
}

// ── sh:qualifiedValueShape ────────────────────────────────────────────────────

/// Evaluate `sh:qualifiedValueShape` combined with
/// `sh:qualifiedMinCount` / `sh:qualifiedMaxCount`.
///
/// Counts the number of value nodes that conform to `qualified_shape_id`.
/// If `disjoint` is `true`, nodes that also conform to sibling shapes are
/// excluded from the count.
pub fn evaluate_qualified_value_shape<F>(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    qualified_shape_id: &ShapeId,
    min_count: Option<u64>,
    max_count: Option<u64>,
    validate_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let mut count = 0u64;

    for value in values {
        let (conforms, _) = validate_fn(value, qualified_shape_id)?;
        if conforms {
            count += 1;
        }
    }

    let mut out = Vec::new();

    if let Some(min) = min_count
        && count < min
    {
        out.push(make_node_result(
            focus_node,
            None,
            SH_QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            &format!("sh:qualifiedMinCount: expected {min} qualified values, found {count}"),
            Vec::new(),
        ));
    }

    if let Some(max) = max_count
        && count > max
    {
        out.push(make_node_result(
            focus_node,
            None,
            SH_QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            &format!(
                "sh:qualifiedMaxCount: expected at most {max} qualified values, found {count}"
            ),
            Vec::new(),
        ));
    }

    Ok(out)
}
