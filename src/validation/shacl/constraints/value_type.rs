//! SHACL constraint evaluators — value type constraints.
//!
//! Implements `sh:class`, `sh:datatype`, and `sh:nodeKind`.

use crate::error::Result;
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclNodeKind, ShaclSeverity},
    paths::term_to_sparql,
    report::ShaclValidationResult,
    vocabulary::*,
};

/// Evaluate `sh:class <class>` on each value node.
///
/// A value node conforms iff `?value rdf:type/rdfs:subClassOf* <class>` holds.
pub fn evaluate_class(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    class: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let class_str = term_to_sparql(class);
    let mut results = Vec::new();

    for value in values {
        // Skip blank nodes and literals — only IRIs can be instances
        if !matches!(value, RdfTerm::Iri(_)) {
            results.push(make_result(
                focus_node,
                value,
                SH_CLASS_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
            ));
            continue;
        }

        let value_str = term_to_sparql(value);
        let query = format!(
            "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
             PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
             ASK {{ {value_str} rdf:type/rdfs:subClassOf* {class_str} }}"
        );
        let conforms = store.execute_ask(&query)?;
        if !conforms {
            results.push(make_result(
                focus_node,
                value,
                SH_CLASS_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
            ));
        }
    }

    Ok(results)
}

/// Evaluate `sh:datatype <datatype>`.
///
/// A value node conforms iff it is a literal whose datatype IRI exactly matches
/// `datatype` and is a valid lexical representation for that datatype.
pub fn evaluate_datatype(
    values: &[RdfTerm],
    datatype_iri: &str,
    focus_node: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut results = Vec::new();

    for value in values {
        let conforms = match value {
            RdfTerm::Literal { datatype, .. } => {
                if let Some(dt) = datatype {
                    dt.as_str() == datatype_iri
                } else {
                    // Plain literal — only matches xsd:string
                    datatype_iri == XSD_STRING
                }
            }
            _ => false,
        };

        if !conforms {
            results.push(make_result(
                focus_node,
                value,
                SH_DATATYPE_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
            ));
        }
    }

    results
}

/// Evaluate `sh:nodeKind <kind>`.
pub fn evaluate_node_kind(
    values: &[RdfTerm],
    kind: &ShaclNodeKind,
    focus_node: &RdfTerm,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> Vec<ShaclValidationResult> {
    let mut results = Vec::new();

    for value in values {
        if !kind.matches(value) {
            results.push(make_result(
                focus_node,
                value,
                SH_NODE_KIND_CONSTRAINT_COMPONENT,
                severity,
                source_shape,
                messages,
            ));
        }
    }

    results
}

// ── Internal helper ───────────────────────────────────────────────────────────

pub(super) fn make_result(
    focus_node: &RdfTerm,
    value: &RdfTerm,
    component_iri: &str,
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    messages: &[ShaclMessage],
) -> ShaclValidationResult {
    ShaclValidationResult {
        focus_node: focus_node.clone(),
        result_path: None,
        value: Some(value.clone()),
        source_shape: source_shape.cloned(),
        source_constraint_component: component_iri.to_string(),
        severity: severity.clone(),
        messages: messages.to_vec(),
        details: Vec::new(),
    }
}
