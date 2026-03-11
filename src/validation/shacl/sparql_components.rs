//! SHACL SPARQL-based constraint component evaluator (§5.4).
//!
//! Handles custom `sh:ConstraintComponent` declarations with `sh:validator`
//! (ASK), `sh:nodeValidator` / `sh:propertyValidator` (SELECT).

use crate::error::{Error, Result};
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    model::{ShaclMessage, ShaclSeverity, SparqlComponentConstraint},
    paths::term_to_sparql,
    report::ShaclValidationResult,
    // vocabulary constants are used via vocabulary::
};

/// Evaluate a custom SPARQL-based constraint component against the focus node.
///
/// `values` is the list of values reached via the property path (or `[focus_node]`
/// for node shapes).
pub fn evaluate_sparql_component(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    values: &[RdfTerm],
    constraint: &SparqlComponentConstraint,
    ask_query: Option<&str>,
    select_query: Option<&str>,
    prefixes: &[(String, String)],
    severity: &ShaclSeverity,
    source_shape: Option<&RdfTerm>,
    param_messages: &[ShaclMessage],
) -> Result<Vec<ShaclValidationResult>> {
    let prefix_block: String = prefixes
        .iter()
        .map(|(p, ns)| format!("PREFIX {p}: <{ns}>\n"))
        .collect();

    // Bind parameter values
    let param_bindings = build_param_bindings(&constraint.parameters);

    let mut out = Vec::new();

    if let Some(ask) = ask_query {
        // ASK-based validator: evaluated per value node
        for value in values {
            let value_str = term_to_sparql(value);
            let this_str = term_to_sparql(focus_node);
            let mut query = format!("{prefix_block}{ask}");
            query = substitute_value(&query, &value_str);
            query = substitute_this(&query, &this_str);
            query = apply_param_bindings(&query, &param_bindings);

            let conforms = store
                .execute_ask(&query)
                .map_err(|e| Error::shacl(format!("SPARQL ASK validator failed: {e}")))?;

            if !conforms {
                let msgs = build_messages(param_messages, None, value, &constraint.parameters);
                out.push(ShaclValidationResult {
                    focus_node: focus_node.clone(),
                    result_path: None,
                    value: Some(value.clone()),
                    source_shape: source_shape.cloned(),
                    source_constraint_component: constraint
                        .source_component
                        .clone()
                        .to_string_repr(),
                    severity: severity.clone(),
                    messages: msgs,
                    details: Vec::new(),
                });
            }
        }
    } else if let Some(select) = select_query {
        // SELECT-based validator: evaluated per focus node
        let this_str = term_to_sparql(focus_node);
        let mut query = format!("{prefix_block}{select}");
        query = substitute_this(&query, &this_str);
        query = apply_param_bindings(&query, &param_bindings);

        let rows = store
            .execute_select(&query)
            .map_err(|e| Error::shacl(format!("SPARQL SELECT validator failed: {e}")))?;

        for row in rows {
            if let Some(fail) = row.get("failure")
                && matches!(fail, RdfTerm::Literal { value, .. } if value == "true")
            {
                return Err(Error::shacl("SPARQL component reports processing failure"));
            }

            let value = row.get("value").cloned();
            let row_msg = row.get("message").cloned();

            let mut msgs = param_messages.to_vec();
            if let Some(RdfTerm::Literal {
                value: m,
                language: lang,
                ..
            }) = row_msg
            {
                msgs.push(ShaclMessage {
                    value: m,
                    language: lang,
                });
            }

            out.push(ShaclValidationResult {
                focus_node: focus_node.clone(),
                result_path: None,
                value,
                source_shape: source_shape.cloned(),
                source_constraint_component: constraint.source_component.clone().to_string_repr(),
                severity: severity.clone(),
                messages: msgs,
                details: Vec::new(),
            });
        }
    }

    Ok(out)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_param_bindings(parameters: &[(String, RdfTerm)]) -> Vec<(String, String)> {
    parameters
        .iter()
        .map(|(name, value)| (name.clone(), term_to_sparql(value)))
        .collect()
}

fn apply_param_bindings(query: &str, bindings: &[(String, String)]) -> String {
    let mut q = query.to_string();
    for (name, value_str) in bindings {
        q = q.replace(&format!("${name}"), value_str);
    }
    q
}

fn substitute_value(query: &str, value_str: &str) -> String {
    query.replace("$value", value_str)
}

fn substitute_this(query: &str, this_str: &str) -> String {
    query.replace("$this", this_str)
}

fn build_messages(
    base: &[ShaclMessage],
    label_template: Option<&str>,
    value: &RdfTerm,
    params: &[(String, RdfTerm)],
) -> Vec<ShaclMessage> {
    if !base.is_empty() {
        return base.to_vec();
    }
    if let Some(template) = label_template {
        let mut msg = template.to_string();
        msg = msg.replace("{$value}", &format!("{value:?}"));
        for (name, v) in params {
            msg = msg.replace(&format!("{{${name}}}"), &format!("{v:?}"));
        }
        return vec![ShaclMessage::plain(msg)];
    }
    vec![ShaclMessage::plain(
        "Custom SPARQL-based constraint violated",
    )]
}

// ── RdfTerm → String repr helper (for source_constraint_component) ─────────

trait ToStringRepr {
    fn to_string_repr(&self) -> String;
}

impl ToStringRepr for RdfTerm {
    fn to_string_repr(&self) -> String {
        match self {
            RdfTerm::Iri(iri) => iri.as_str().to_string(),
            RdfTerm::BlankNode(id) => format!("_:{id}"),
            other => format!("{other:?}"),
        }
    }
}
