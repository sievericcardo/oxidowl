//! SHACL logical constraint evaluators.
//!
//! Implements `sh:not`, `sh:and`, `sh:or`, and `sh:xone`.
//!
//! These constraints require recursive conformance checking.  The callers
//! provide a `conforms_fn` closure that delegates back to the engine.

use crate::error::Result;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity, ShapeId},
    report::ShaclValidationResult,
    vocabulary::*,
};

fn node_result(
    focus_node: &RdfTerm,
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
        value: Some(focus_node.clone()),
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

// ── sh:not ────────────────────────────────────────────────────────────────────

/// Evaluate `sh:not <inner_shape>` for a single `focus_node`.
///
/// The focus node conforms iff it does NOT conform to `inner_shape`.
/// `conforms_fn` returns `(conforms: bool, sub_results: Vec<ShaclValidationResult>)`.
pub fn evaluate_not<F>(
    focus_node: &RdfTerm,
    inner_shape: &ShapeId,
    conforms_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let (inner_conforms, _sub) = conforms_fn(focus_node, inner_shape)?;
    if inner_conforms {
        Ok(vec![node_result(
            focus_node,
            SH_NOT_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            "sh:not constraint: node should not conform to the inner shape",
            Vec::new(),
        )])
    } else {
        Ok(Vec::new())
    }
}

// ── sh:and ────────────────────────────────────────────────────────────────────

/// Evaluate `sh:and <shapes>` — focus node must conform to ALL shapes.
pub fn evaluate_and<F>(
    focus_node: &RdfTerm,
    shapes: &[ShapeId],
    conforms_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let mut all_details = Vec::new();
    let mut failed = false;

    for shape_id in shapes {
        let (conforms, sub) = conforms_fn(focus_node, shape_id)?;
        if !conforms {
            failed = true;
            all_details.extend(sub);
        }
    }

    if failed {
        Ok(vec![node_result(
            focus_node,
            SH_AND_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            "sh:and constraint: node fails one or more member shapes",
            all_details,
        )])
    } else {
        Ok(Vec::new())
    }
}

// ── sh:or ─────────────────────────────────────────────────────────────────────

/// Evaluate `sh:or <shapes>` — focus node must conform to AT LEAST ONE shape.
pub fn evaluate_or<F>(
    focus_node: &RdfTerm,
    shapes: &[ShapeId],
    conforms_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let mut all_details = Vec::new();

    for shape_id in shapes {
        let (conforms, sub) = conforms_fn(focus_node, shape_id)?;
        if conforms {
            return Ok(Vec::new()); // at least one conforms → ok
        }
        all_details.extend(sub);
    }

    Ok(vec![node_result(
        focus_node,
        SH_OR_CONSTRAINT_COMPONENT,
        severity,
        source_shape,
        messages,
        "sh:or constraint: node fails all member shapes",
        all_details,
    )])
}

// ── sh:xone ───────────────────────────────────────────────────────────────────

/// Evaluate `sh:xone <shapes>` — focus node must conform to EXACTLY ONE shape.
pub fn evaluate_xone<F>(
    focus_node: &RdfTerm,
    shapes: &[ShapeId],
    conforms_fn: &mut F,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>>
where
    F: FnMut(&RdfTerm, &ShapeId) -> Result<(bool, Vec<ShaclValidationResult>)>,
{
    let mut conforming_count = 0usize;
    let mut all_details = Vec::new();

    for shape_id in shapes {
        let (conforms, sub) = conforms_fn(focus_node, shape_id)?;
        if conforms {
            conforming_count += 1;
        } else {
            all_details.extend(sub);
        }
    }

    if conforming_count == 1 {
        Ok(Vec::new())
    } else {
        Ok(vec![node_result(
            focus_node,
            SH_XONE_CONSTRAINT_COMPONENT,
            severity,
            source_shape,
            messages,
            &format!(
                "sh:xone constraint: node conforms to {conforming_count} shapes, expected exactly 1"
            ),
            all_details,
        )])
    }
}
