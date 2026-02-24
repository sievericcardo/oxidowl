//! Literal value-space comparison for SHACL value-range and property-pair
//! constraints.
//!
//! Implements SPARQL/XSD-compatible ordering for numeric, date/time, and string
//! literal types.

use std::cmp::Ordering;

use crate::semantics::RdfTerm;

/// Compare two RDF terms using the SPARQL/XSD value-space order.
///
/// Returns `None` if the two terms are uncomparable (different types, or a
/// non-literal), which per the SHACL spec means any range constraint on those
/// values is violated.
pub fn compare_terms(a: &RdfTerm, b: &RdfTerm) -> Option<Ordering> {
    match (a, b) {
        (
            RdfTerm::Literal {
                value: va,
                datatype: dta,
                language: la,
                ..
            },
            RdfTerm::Literal {
                value: vb,
                datatype: dtb,
                language: lb,
                ..
            },
        ) => {
            let dta_str = dta
                .as_ref()
                .map(|u| u.as_str())
                .unwrap_or(XSD_STRING_FALLBACK);
            let dtb_str = dtb
                .as_ref()
                .map(|u| u.as_str())
                .unwrap_or(XSD_STRING_FALLBACK);

            // Both plain string literals without types → compare as strings
            if dta_str == dtb_str {
                compare_same_type(va, dta_str, vb, la, lb)
            } else {
                // Try numeric promotion (integer/decimal/float/double can be
                // cross-compared by promoting to f64)
                if is_numeric(dta_str) && is_numeric(dtb_str) {
                    let na = parse_numeric(va)?;
                    let nb = parse_numeric(vb)?;
                    na.partial_cmp(&nb)
                } else {
                    None // incomparable types
                }
            }
        }
        // IRIs compared as strings per SPARQL spec (for sh:lessThan etc.)
        (RdfTerm::Iri(a_iri), RdfTerm::Iri(b_iri)) => Some(a_iri.as_str().cmp(b_iri.as_str())),
        _ => None,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const XSD_STRING_FALLBACK: &str = "http://www.w3.org/2001/XMLSchema#string";

fn is_numeric(dt: &str) -> bool {
    matches!(
        dt,
        "http://www.w3.org/2001/XMLSchema#integer"
            | "http://www.w3.org/2001/XMLSchema#decimal"
            | "http://www.w3.org/2001/XMLSchema#float"
            | "http://www.w3.org/2001/XMLSchema#double"
            | "http://www.w3.org/2001/XMLSchema#int"
            | "http://www.w3.org/2001/XMLSchema#long"
            | "http://www.w3.org/2001/XMLSchema#short"
            | "http://www.w3.org/2001/XMLSchema#byte"
            | "http://www.w3.org/2001/XMLSchema#negativeInteger"
            | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
            | "http://www.w3.org/2001/XMLSchema#positiveInteger"
            | "http://www.w3.org/2001/XMLSchema#unsignedLong"
            | "http://www.w3.org/2001/XMLSchema#unsignedInt"
            | "http://www.w3.org/2001/XMLSchema#unsignedShort"
            | "http://www.w3.org/2001/XMLSchema#unsignedByte"
    )
}

fn parse_numeric(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn is_date(dt: &str) -> bool {
    dt == "http://www.w3.org/2001/XMLSchema#date"
}

fn is_datetime(dt: &str) -> bool {
    dt == "http://www.w3.org/2001/XMLSchema#dateTime"
}

fn compare_same_type(
    va: &str,
    dt: &str,
    vb: &str,
    la: &Option<String>,
    lb: &Option<String>,
) -> Option<Ordering> {
    if is_numeric(dt) {
        let a = parse_numeric(va)?;
        let b = parse_numeric(vb)?;
        a.partial_cmp(&b)
    } else if is_datetime(dt) {
        compare_datetimes(va, vb)
    } else if is_date(dt) {
        compare_dates(va, vb)
    } else if dt == "http://www.w3.org/2001/XMLSchema#boolean" {
        let a = parse_bool(va)?;
        let b = parse_bool(vb)?;
        Some(a.cmp(&b))
    } else if dt == "http://www.w3.org/2001/XMLSchema#string"
        || dt == "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
    {
        // String/lang-string: language tags must match for comparison
        if la == lb { Some(va.cmp(vb)) } else { None }
    } else {
        // For other types, compare lexicographically (best effort)
        Some(va.cmp(vb))
    }
}

fn compare_datetimes(a: &str, b: &str) -> Option<Ordering> {
    use chrono::{DateTime, FixedOffset};
    let pa = DateTime::parse_from_rfc3339(a)
        .or_else(|_| DateTime::parse_from_str(a, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    let pb = DateTime::parse_from_rfc3339(b)
        .or_else(|_| DateTime::parse_from_str(b, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    let _: FixedOffset = *pa.offset(); // type inference helper
    Some(pa.cmp(&pb))
}

fn compare_dates(a: &str, b: &str) -> Option<Ordering> {
    use chrono::NaiveDate;
    let pa = NaiveDate::parse_from_str(a, "%Y-%m-%d").ok()?;
    let pb = NaiveDate::parse_from_str(b, "%Y-%m-%d").ok()?;
    Some(pa.cmp(&pb))
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn int_lit(v: &str) -> RdfTerm {
        RdfTerm::Literal {
            value: v.to_string(),
            datatype: Some(Url::parse("http://www.w3.org/2001/XMLSchema#integer").unwrap()),
            language: None,
            direction: None,
        }
    }

    fn str_lit(v: &str) -> RdfTerm {
        RdfTerm::Literal {
            value: v.to_string(),
            datatype: Some(Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap()),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn numeric_comparison() {
        assert_eq!(
            compare_terms(&int_lit("3"), &int_lit("5")),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_terms(&int_lit("5"), &int_lit("5")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_terms(&int_lit("7"), &int_lit("5")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn string_comparison() {
        assert_eq!(
            compare_terms(&str_lit("abc"), &str_lit("abd")),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_terms(&str_lit("abc"), &str_lit("abc")),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn incomparable_types_return_none() {
        let a_dt = Url::parse("http://www.w3.org/2001/XMLSchema#integer").unwrap();
        let b_dt = Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap();
        let a = RdfTerm::Literal {
            value: "42".to_string(),
            datatype: Some(a_dt),
            language: None,
            direction: None,
        };
        let b = RdfTerm::Literal {
            value: "hello".to_string(),
            datatype: Some(b_dt),
            language: None,
            direction: None,
        };
        assert_eq!(compare_terms(&a, &b), None);
    }
}
