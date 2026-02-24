//! SHACL validation report types.
//!
//! Defines `ShaclValidationReport` and `ShaclValidationResult`, with `Serialize`
//! for JSON output and an `to_turtle()` helper that produces a spec-compliant
//! RDF validation report in Turtle format.

use crate::semantics::RdfTerm;
use crate::validation::shacl::model::{ShaclMessage, ShaclPath, ShaclSeverity};
use serde::{Deserialize, Serialize};

/// A `sh:ValidationReport` — the top-level result of a SHACL validation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaclValidationReport {
    /// `sh:conforms` — `true` iff there are no validation results.
    pub conforms: bool,
    /// The individual `sh:ValidationResult` instances.
    pub results: Vec<ShaclValidationResult>,
    /// Whether the shapes graph itself was well-formed (per spec Appendix B).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shapes_graph_well_formed: Option<bool>,
}

impl ShaclValidationReport {
    /// Create an empty, conforming report.
    pub fn conforming() -> Self {
        ShaclValidationReport {
            conforms: true,
            results: Vec::new(),
            shapes_graph_well_formed: None,
        }
    }

    /// Create a non-conforming report from a list of results.
    pub fn non_conforming(results: Vec<ShaclValidationResult>) -> Self {
        let conforms = results.is_empty();
        ShaclValidationReport {
            conforms,
            results,
            shapes_graph_well_formed: None,
        }
    }

    /// Produce a Turtle serialization of the validation report.
    pub fn to_turtle(&self) -> String {
        let mut out = String::new();

        // Prefix declarations
        out.push_str("@prefix sh: <http://www.w3.org/ns/shacl#> .\n");
        out.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
        out.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\n");

        // Report node
        out.push_str("[] a sh:ValidationReport ;\n");
        out.push_str(&format!(
            "   sh:conforms {} ;\n",
            if self.conforms { "true" } else { "false" }
        ));

        if let Some(wf) = self.shapes_graph_well_formed {
            out.push_str(&format!(
                "   sh:shapesGraphWellFormed {} ;\n",
                if wf { "true" } else { "false" }
            ));
        }

        if self.results.is_empty() {
            out.push_str("   .\n");
        } else {
            let last = self.results.len() - 1;
            for (i, result) in self.results.iter().enumerate() {
                let trailing = if i == last { "   .\n" } else { "   ;\n" };
                out.push_str(&format!(
                    "   sh:result {}{trailing}",
                    result.to_turtle_inline()
                ));
            }
        }

        out
    }
}

/// A single `sh:ValidationResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaclValidationResult {
    /// `sh:focusNode`
    pub focus_node: RdfTerm,
    /// `sh:resultPath` (absent for node shape violations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_path: Option<ShaclPath>,
    /// `sh:value` — the offending value node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<RdfTerm>,
    /// `sh:sourceShape`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_shape: Option<RdfTerm>,
    /// `sh:sourceConstraintComponent`
    pub source_constraint_component: String,
    /// `sh:resultSeverity`
    pub severity: ShaclSeverity,
    /// `sh:resultMessage`
    pub messages: Vec<ShaclMessage>,
    /// `sh:detail` — nested results from sub-shape violations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<ShaclValidationResult>,
}

impl ShaclValidationResult {
    /// Produce a Turtle blank-node block for embedding in a report, e.g.:
    /// `[ a sh:ValidationResult ; sh:focusNode <…> ; … ]`
    pub fn to_turtle_inline(&self) -> String {
        let mut out = String::from("[\n      a sh:ValidationResult ;\n");

        out.push_str(&format!(
            "      sh:focusNode {} ;\n",
            turtle_term(&self.focus_node)
        ));
        out.push_str(&format!(
            "      sh:resultSeverity <{}> ;\n",
            self.severity.as_iri()
        ));
        out.push_str(&format!(
            "      sh:sourceConstraintComponent <{}> ;\n",
            self.source_constraint_component
        ));

        if let Some(shape) = &self.source_shape {
            out.push_str(&format!("      sh:sourceShape {} ;\n", turtle_term(shape)));
        }

        if let Some(path) = &self.result_path {
            out.push_str(&format!("      sh:resultPath {} ;\n", path_to_turtle(path)));
        }

        if let Some(value) = &self.value {
            out.push_str(&format!("      sh:value {} ;\n", turtle_term(value)));
        }

        for msg in &self.messages {
            if let Some(lang) = &msg.language {
                out.push_str(&format!(
                    "      sh:resultMessage \"{}\"@{} ;\n",
                    escape_turtle_string(&msg.value),
                    lang
                ));
            } else {
                out.push_str(&format!(
                    "      sh:resultMessage \"{}\" ;\n",
                    escape_turtle_string(&msg.value)
                ));
            }
        }

        // Remove trailing " ;\n" and close blank node
        if out.ends_with(" ;\n") {
            out.truncate(out.len() - 3);
            out.push_str("\n   ]");
        } else {
            out.push_str("   ]");
        }

        out
    }
}

// ── Turtle helpers ───────────────────────────────────────────────────────────

/// Format an `RdfTerm` as a Turtle literal.
fn turtle_term(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", iri.as_str()),
        RdfTerm::BlankNode(id) => format!("_:{id}"),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let escaped = escape_turtle_string(value);
            if let Some(lang) = language {
                format!("\"{escaped}\"@{lang}")
            } else if let Some(dt) = datatype {
                format!("\"{escaped}\"^^<{dt}>")
            } else {
                format!("\"{escaped}\"")
            }
        }
        RdfTerm::QuotedTriple(_) => "\"_quoted_triple_\"".to_string(),
    }
}

/// Convert a `ShaclPath` to a Turtle inline path expression.
fn path_to_turtle(path: &ShaclPath) -> String {
    match path {
        ShaclPath::Predicate(iri) => format!("<{iri}>"),
        ShaclPath::Sequence(steps) => {
            let parts: Vec<String> = steps.iter().map(path_to_turtle).collect();
            format!("({})", parts.join("/"))
        }
        ShaclPath::Alternative(alts) => {
            let parts: Vec<String> = alts.iter().map(path_to_turtle).collect();
            format!("({})", parts.join("|"))
        }
        ShaclPath::Inverse(inner) => format!("[sh:inversePath {}]", path_to_turtle(inner)),
        ShaclPath::ZeroOrMore(inner) => format!("[sh:zeroOrMorePath {}]", path_to_turtle(inner)),
        ShaclPath::OneOrMore(inner) => format!("[sh:oneOrMorePath {}]", path_to_turtle(inner)),
        ShaclPath::ZeroOrOne(inner) => format!("[sh:zeroOrOnePath {}]", path_to_turtle(inner)),
    }
}

fn escape_turtle_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
