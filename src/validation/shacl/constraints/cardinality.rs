//! SHACL cardinality constraint evaluators: `sh:minCount` and `sh:maxCount`.

use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity},
    report::ShaclValidationResult,
    vocabulary::*,
};

/// Evaluate `sh:minCount <n>`.
///
/// Produces a violation on the focus node (no specific value node) if the
/// count of value nodes is less than `min_count`.
pub fn evaluate_min_count(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    min_count: u64,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let count = values.len() as u64;
    if count < min_count {
        vec![ShaclValidationResult {
            focus_node: focus_node.clone(),
            result_path: None,
            value: None,
            source_shape: source_shape.cloned(),
            source_constraint_component: SH_MIN_COUNT_CONSTRAINT_COMPONENT.to_string(),
            severity: severity.clone(),
            messages: augment_messages(
                messages,
                &format!("Expected at least {min_count} value(s), found {count}"),
            ),
            details: Vec::new(),
        }]
    } else {
        Vec::new()
    }
}

/// Evaluate `sh:maxCount <n>`.
pub fn evaluate_max_count(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    max_count: u64,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let count = values.len() as u64;
    if count > max_count {
        vec![ShaclValidationResult {
            focus_node: focus_node.clone(),
            result_path: None,
            value: None,
            source_shape: source_shape.cloned(),
            source_constraint_component: SH_MAX_COUNT_CONSTRAINT_COMPONENT.to_string(),
            severity: severity.clone(),
            messages: augment_messages(
                messages,
                &format!("Expected at most {max_count} value(s), found {count}"),
            ),
            details: Vec::new(),
        }]
    } else {
        Vec::new()
    }
}

/// If `extra` messages exist, prepend the default one; otherwise return just
/// the provided message as a fallback.
fn augment_messages(messages: &[ShaclMessage], default_text: &str) -> Vec<ShaclMessage> {
    if messages.is_empty() {
        vec![ShaclMessage::plain(default_text)]
    } else {
        messages.to_vec()
    }
}
