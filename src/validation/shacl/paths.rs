//! SHACL property path translation and traversal.
//!
//! Translates `ShaclPath` values to SPARQL 1.1 property path expressions and
//! provides a helper that executes those paths against an `SparqlStore`.

use crate::error::Result;
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::model::ShaclPath;

// ── Path → SPARQL string ─────────────────────────────────────────────────────

impl ShaclPath {
    /// Convert this path to a SPARQL 1.1 property path expression string.
    pub fn to_sparql_path(&self) -> String {
        match self {
            ShaclPath::Predicate(iri) => format!("<{iri}>"),
            ShaclPath::Sequence(steps) => {
                let parts: Vec<String> = steps.iter().map(|s| s.to_sparql_path()).collect();
                parts.join("/")
            }
            ShaclPath::Alternative(alts) => {
                let parts: Vec<String> = alts.iter().map(|a| a.to_sparql_path()).collect();
                format!("({})", parts.join("|"))
            }
            ShaclPath::Inverse(inner) => format!("^({})", inner.to_sparql_path()),
            ShaclPath::ZeroOrMore(inner) => format!("({})*", inner.to_sparql_path()),
            ShaclPath::OneOrMore(inner) => format!("({})+", inner.to_sparql_path()),
            ShaclPath::ZeroOrOne(inner) => format!("({})?", inner.to_sparql_path()),
        }
    }
}

// ── Path traversal ───────────────────────────────────────────────────────────

/// Return all RDF terms reachable from `focus_node` via `path` in `store`.
///
/// Executes `SELECT ?value WHERE { <focus> <path_expr> ?value }` and collects
/// the `?value` bindings.
pub fn resolve_values(
    store: &SparqlStore,
    focus_node: &RdfTerm,
    path: &ShaclPath,
) -> Result<Vec<RdfTerm>> {
    let focus_str = term_to_sparql_subject(focus_node);
    let path_expr = path.to_sparql_path();
    let query = format!(
        "SELECT ?value WHERE {{ {focus_str} {path_expr} ?value }}",
    );

    let rows = store.execute_select(&query)?;
    let mut values = Vec::new();
    for row in rows {
        if let Some(term) = row.get("value") {
            values.push(term.clone());
        }
    }
    Ok(values)
}

/// Format an `RdfTerm` as a SPARQL subject (IRI or blank node).
pub fn term_to_sparql_subject(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", iri.as_str()),
        RdfTerm::BlankNode(id) => {
            // In SPARQL you cannot directly address blank nodes by name in a
            // query text; we use a VALUES clause workaround by binding it as
            // a variable.  For blank node subjects, we return a placeholder
            // that callers must substitute via a VALUES block.
            format!("_:{id}")
        }
        RdfTerm::Literal { .. } => {
            // Literals cannot be subjects in standard SPARQL — return empty
            // string; callers should skip path resolution for literals.
            String::new()
        }
        RdfTerm::QuotedTriple(_) => String::new(),
    }
}

/// Format an `RdfTerm` for use inside a SPARQL query (subject, object, or
/// predicate position).
pub fn term_to_sparql(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", iri.as_str()),
        RdfTerm::BlankNode(id) => format!("_:{id}"),
        RdfTerm::Literal { value, datatype, language, .. } => {
            let escaped = sparql_escape_string(value);
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

/// Escape a string for use inside a SPARQL string literal.
pub fn sparql_escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_path() {
        let p = ShaclPath::Predicate("http://example.org/p".to_string());
        assert_eq!(p.to_sparql_path(), "<http://example.org/p>");
    }

    #[test]
    fn sequence_path() {
        let p = ShaclPath::Sequence(vec![
            ShaclPath::Predicate("http://example.org/a".to_string()),
            ShaclPath::Predicate("http://example.org/b".to_string()),
        ]);
        assert_eq!(p.to_sparql_path(), "<http://example.org/a>/<http://example.org/b>");
    }

    #[test]
    fn alternative_path() {
        let p = ShaclPath::Alternative(vec![
            ShaclPath::Predicate("http://example.org/a".to_string()),
            ShaclPath::Predicate("http://example.org/b".to_string()),
        ]);
        assert_eq!(p.to_sparql_path(), "(<http://example.org/a>|<http://example.org/b>)");
    }

    #[test]
    fn inverse_path() {
        let p = ShaclPath::Inverse(Box::new(ShaclPath::Predicate(
            "http://example.org/parent".to_string(),
        )));
        assert_eq!(p.to_sparql_path(), "^(<http://example.org/parent>)");
    }

    #[test]
    fn zero_or_more_path() {
        let p = ShaclPath::ZeroOrMore(Box::new(ShaclPath::Predicate(
            "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
        )));
        assert_eq!(
            p.to_sparql_path(),
            "(<http://www.w3.org/2000/01/rdf-schema#subClassOf>)*"
        );
    }

    #[test]
    fn one_or_more_path() {
        let p = ShaclPath::OneOrMore(Box::new(ShaclPath::Predicate(
            "http://example.org/child".to_string(),
        )));
        assert_eq!(p.to_sparql_path(), "(<http://example.org/child>)+");
    }

    #[test]
    fn zero_or_one_path() {
        let p = ShaclPath::ZeroOrOne(Box::new(ShaclPath::Predicate(
            "http://example.org/optional".to_string(),
        )));
        assert_eq!(p.to_sparql_path(), "(<http://example.org/optional>)?");
    }
}
