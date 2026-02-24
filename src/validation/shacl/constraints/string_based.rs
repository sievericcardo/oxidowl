//! SHACL string-based constraint evaluators.
//!
//! Implements `sh:minLength`, `sh:maxLength`, `sh:pattern`, `sh:flags`,
//! `sh:languageIn`, and `sh:uniqueLang`.

use std::collections::HashMap;

use regex::RegexBuilder;

use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity},
    report::ShaclValidationResult,
    vocabulary::*,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Return the `str()` function value for an RDF term (SPARQL semantics).
/// Blank nodes have no string value per SHACL spec → returns `None`.
fn str_value(term: &RdfTerm) -> Option<String> {
    match term {
        RdfTerm::Iri(iri) => Some(iri.as_str().to_string()),
        RdfTerm::Literal { value, .. } => Some(value.clone()),
        RdfTerm::BlankNode(_) | RdfTerm::QuotedTriple(_) => None,
    }
}

fn make_result(
    focus_node: &RdfTerm,
    value: &RdfTerm,
    component: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
    default_msg: &str,
) -> ShaclValidationResult {
    ShaclValidationResult {
        focus_node: focus_node.clone(),
        result_path: None,
        value: Some(value.clone()),
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

// ── sh:minLength / sh:maxLength ───────────────────────────────────────────────

pub fn evaluate_min_length(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    min_len: u64,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut out = Vec::new();
    for value in values {
        match str_value(value) {
            None => out.push(make_result(
                focus_node,
                value,
                SH_MIN_LENGTH_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "Blank nodes have no string length",
            )),
            Some(s) => {
                let char_count = s.chars().count() as u64;
                if char_count < min_len {
                    out.push(make_result(
                        focus_node,
                        value,
                        SH_MIN_LENGTH_CONSTRAINT_COMPONENT,
                        severity,
                        source_shape,
                        messages,
                        &format!("String length {char_count} < minLength {min_len}"),
                    ));
                }
            }
        }
    }
    out
}

pub fn evaluate_max_length(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    max_len: u64,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut out = Vec::new();
    for value in values {
        match str_value(value) {
            None => out.push(make_result(
                focus_node,
                value,
                SH_MAX_LENGTH_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "Blank nodes have no string length",
            )),
            Some(s) => {
                let char_count = s.chars().count() as u64;
                if char_count > max_len {
                    out.push(make_result(
                        focus_node,
                        value,
                        SH_MAX_LENGTH_CONSTRAINT_COMPONENT,
                        severity,
                        source_shape,
                        messages,
                        &format!("String length {char_count} > maxLength {max_len}"),
                    ));
                }
            }
        }
    }
    out
}

// ── sh:pattern ───────────────────────────────────────────────────────────────

pub fn evaluate_pattern(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    pattern: &str,
    flags: Option<&str>,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    // Compile the regex once (with optional flags).
    // SHACL uses XSD F&O regex flags: 'i' (case-insensitive), 's' (dot-all),
    // 'm' (multi-line), 'x' (extended whitespace), 'q' (literal).
    // Rust's regex crate supports 'i', 's', 'm', 'x'.
    let flag_str = flags.unwrap_or("");
    let re = match RegexBuilder::new(pattern)
        .case_insensitive(flag_str.contains('i'))
        .dot_matches_new_line(flag_str.contains('s'))
        .multi_line(flag_str.contains('m'))
        .ignore_whitespace(flag_str.contains('x'))
        .build()
    {
        Ok(r) => r,
        Err(_) => {
            // Ill-formed pattern → all values violate
            return values
                .iter()
                .map(|v| {
                    make_result(
                        focus_node,
                        v,
                        SH_PATTERN_CONSTRAINT_COMPONENT,
                        severity,
                        source_shape,
                        messages,
                        "Invalid sh:pattern regex",
                    )
                })
                .collect();
        }
    };

    let mut out = Vec::new();
    for value in values {
        match str_value(value) {
            None => out.push(make_result(
                focus_node,
                value,
                SH_PATTERN_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
                "Blank nodes cannot be matched against sh:pattern",
            )),
            Some(s) => {
                if !re.is_match(&s) {
                    out.push(make_result(
                        focus_node,
                        value,
                        SH_PATTERN_CONSTRAINT_COMPONENT,
                        severity,
                        source_shape,
                        messages,
                        &format!("Value does not match pattern /{pattern}/"),
                    ));
                }
            }
        }
    }
    out
}

// ── sh:languageIn ─────────────────────────────────────────────────────────────

/// Compare two BCP47 language ranges using SPARQL `langMatches()` semantics:
/// the range `"*"` matches any non-empty language tag; otherwise the tag must
/// start with the range (case-insensitive) optionally followed by a `-`.
fn lang_matches(tag: &str, range: &str) -> bool {
    if range == "*" {
        return !tag.is_empty();
    }
    let tag_lower = tag.to_ascii_lowercase();
    let range_lower = range.to_ascii_lowercase();
    tag_lower == range_lower || tag_lower.starts_with(&format!("{range_lower}-"))
}

pub fn evaluate_language_in(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    language_in: &[String],
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut out = Vec::new();
    for value in values {
        match value {
            RdfTerm::Literal {
                language: Some(tag),
                ..
            } => {
                let conforms = language_in.iter().any(|r| lang_matches(tag, r));
                if !conforms {
                    out.push(make_result(
                        focus_node,
                        value,
                        SH_LANGUAGE_IN_CONSTRAINT_COMPONENT,
                        severity,
                        source_shape,
                        messages,
                        &format!("Language tag '{tag}' not in sh:languageIn list"),
                    ));
                }
            }
            _ => {
                // Non-language literals and non-literals always violate
                out.push(make_result(
                    focus_node,
                    value,
                    SH_LANGUAGE_IN_CONSTRAINT_COMPONENT,
                    severity,
                    source_shape,
                    messages,
                    "Value is not a language-tagged literal for sh:languageIn",
                ));
            }
        }
    }
    out
}

// ── sh:uniqueLang ─────────────────────────────────────────────────────────────

pub fn evaluate_unique_lang(
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut duplicate_tags: Vec<String> = Vec::new();

    for value in values {
        if let RdfTerm::Literal {
            language: Some(tag),
            ..
        } = value
        {
            let lower_tag = tag.to_ascii_lowercase();
            let count = seen.entry(lower_tag.clone()).or_insert(0);
            *count += 1;
            if *count == 2 {
                duplicate_tags.push(tag.clone());
            }
        }
    }

    if duplicate_tags.is_empty() {
        return Vec::new();
    }

    vec![ShaclValidationResult {
        focus_node: focus_node.clone(),
        result_path: None,
        value: None,
        source_shape: source_shape.cloned(),
        source_constraint_component: SH_UNIQUE_LANG_CONSTRAINT_COMPONENT.to_string(),
        severity: severity.clone(),
        messages: if messages.is_empty() {
            vec![ShaclMessage::plain(format!(
                "Duplicate language tags: {}",
                duplicate_tags.join(", ")
            ))]
        } else {
            messages.to_vec()
        },
        details: Vec::new(),
    }]
}
