//! SHACL value-range constraint evaluators.
//!
//! Implements `sh:minExclusive`, `sh:minInclusive`, `sh:maxExclusive`, and
//! `sh:maxInclusive` using the `literal_compare` module.

use std::cmp::Ordering;

use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    constraints::literal_compare::compare_terms,
    model::{ShaclMessage, ShaclSeverity},
    report::ShaclValidationResult,
    vocabulary::*,
};

macro_rules! range_check {
    ($fn_name:ident, $comp:expr, $component:expr, $desc:literal) => {
        pub fn $fn_name(
            focus_node: &RdfTerm,
            values: &[RdfTerm],
            bound: &RdfTerm,
            severity: &ShaclSeverity,
            source_shape: Option<&RdfTerm>,
            messages: &[ShaclMessage],
        ) -> Vec<ShaclValidationResult> {
            let mut results = Vec::new();
            for value in values {
                let passes = compare_terms(value, bound)
                    .map(|ord| $comp(ord))
                    .unwrap_or(false); // None → uncomparable → violation
                if !passes {
                    results.push(ShaclValidationResult {
                        focus_node: focus_node.clone(),
                        result_path: None,
                        value: Some(value.clone()),
                        source_shape: source_shape.cloned(),
                        source_constraint_component: $component.to_string(),
                        severity: severity.clone(),
                        messages: if messages.is_empty() {
                            vec![ShaclMessage::plain(format!(
                                "Value does not satisfy {} constraint",
                                $desc
                            ))]
                        } else {
                            messages.to_vec()
                        },
                        details: Vec::new(),
                    });
                }
            }
            results
        }
    };
}

// value > bound  (value.cmp(bound) == Greater)
range_check!(
    evaluate_min_exclusive,
    |ord: Ordering| ord == Ordering::Greater,
    SH_MIN_EXCLUSIVE_CONSTRAINT_COMPONENT,
    "sh:minExclusive"
);

// value >= bound (value.cmp(bound) != Less)
range_check!(
    evaluate_min_inclusive,
    |ord: Ordering| ord != Ordering::Less,
    SH_MIN_INCLUSIVE_CONSTRAINT_COMPONENT,
    "sh:minInclusive"
);

// value < bound  (value.cmp(bound) == Less)
range_check!(
    evaluate_max_exclusive,
    |ord: Ordering| ord == Ordering::Less,
    SH_MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
    "sh:maxExclusive"
);

// value <= bound (value.cmp(bound) != Greater)
range_check!(
    evaluate_max_inclusive,
    |ord: Ordering| ord != Ordering::Greater,
    SH_MAX_INCLUSIVE_CONSTRAINT_COMPONENT,
    "sh:maxInclusive"
);
