//! Pre-extracted TBox-driven ABox classification SPARQL rule generation.
//!
//! This module discovers supported OWL class-axiom patterns from the loaded
//! TBox once, then emits simple ABox-only INSERT rules for fast repeated
//! classification rounds.

#[cfg(feature = "sparql-store")]
use crate::query::SparqlStore;
#[cfg(feature = "sparql-store")]
use crate::semantics::RdfTerm;

/// Pre-extract TBox class-axiom patterns as simple ABox-only SPARQL INSERT rules.
///
/// Runs SELECT queries against the already-loaded TBox once at ontology load time
/// to discover all class-defining axioms. Generates simple INSERT rules that only
/// touch ABox triples at classification time.
///
/// Supported pattern families:
/// 1. `C ≡ (P hasValue V)`
/// 2. `C ≡ (P someValuesFrom D)` with named filler
/// 3. `C ≡ D` named-class bi-directional equivalence
/// 4. `C ≡ intersect(double-range(dp,facets), hasValue(hvp,hv))`
/// 5. `C ≡ intersect(integer-range(ip), double-range(dp,facets), hasValue(hvp,hv))`
/// 6. `C ≡ intersect(NamedClass1, NamedClass2, ...)` pure named-class intersection
/// 7. `sub rdfs:subClassOf sup` subclass propagation
/// 8. `P rdfs:domain C` subject typing
/// 9. `P rdfs:range C` object typing
#[cfg(feature = "sparql-store")]
#[allow(clippy::too_many_lines)]
pub fn extract_owl_rules_from_tbox(store: &SparqlStore) -> Vec<String> {
    use std::collections::HashMap;

    #[inline]
    fn is_iri(t: &RdfTerm) -> bool {
        matches!(t, RdfTerm::Iri(_))
    }

    let mut rules: Vec<String> = Vec::new();

    let owl = "http://www.w3.org/2002/07/owl#";
    let rdf = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    let rdfs = "http://www.w3.org/2000/01/rdf-schema#";
    let xsd = "http://www.w3.org/2001/XMLSchema#";

    // Rule 1: C equiv (P hasValue V)
    let q = format!(
        "SELECT DISTINCT ?C ?P ?V WHERE {{ \
          ?C <{owl}equivalentClass> ?R . \
          ?R <{rdf}type> <{owl}Restriction> . \
          ?R <{owl}onProperty> ?P . \
          ?R <{owl}hasValue> ?V . \
          FILTER(isIRI(?C)) FILTER(isIRI(?P)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(c), Some(p), Some(v)) = (row.get("C"), row.get("P"), row.get("V")) {
                if !is_iri(c) || !is_iri(p) {
                    continue;
                }
                let (c_s, p_s, v_s) = (c.to_string(), p.to_string(), v.to_string());
                rules.push(format!(
                    "INSERT {{ ?x a {c_s} }} WHERE {{ ?x {p_s} {v_s} . \
                     FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
                ));
            }
        }
    }

    // Rule 2: C equiv (P someValuesFrom D), named D
    let q = format!(
        "SELECT DISTINCT ?C ?P ?D WHERE {{ \
          ?C <{owl}equivalentClass> ?R . \
          ?R <{rdf}type> <{owl}Restriction> . \
          ?R <{owl}onProperty> ?P . \
          ?R <{owl}someValuesFrom> ?D . \
          FILTER(isIRI(?C)) FILTER(isIRI(?P)) FILTER(isIRI(?D)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(c), Some(p), Some(d)) = (row.get("C"), row.get("P"), row.get("D")) {
                if !is_iri(c) || !is_iri(p) || !is_iri(d) {
                    continue;
                }
                let (c_s, p_s, d_s) = (c.to_string(), p.to_string(), d.to_string());
                rules.push(format!(
                    "INSERT {{ ?x a {c_s} }} WHERE {{ ?x {p_s} ?y . ?y a {d_s} . \
                     FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
                ));
            }
        }
    }

    // Rules 3a/3b: C equiv D named class equivalence (both directions)
    let q = format!(
        "SELECT DISTINCT ?C ?D WHERE {{ \
          ?C <{owl}equivalentClass> ?D . \
          FILTER(isIRI(?C)) FILTER(isIRI(?D)) FILTER(?C != ?D) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for row in rows {
            if let (Some(c), Some(d)) = (row.get("C"), row.get("D")) {
                if !is_iri(c) || !is_iri(d) {
                    continue;
                }
                let (c_s, d_s) = (c.to_string(), d.to_string());
                let key = if c_s < d_s {
                    (c_s.clone(), d_s.clone())
                } else {
                    (d_s.clone(), c_s.clone())
                };
                if seen.insert(key) {
                    rules.push(format!(
                        "INSERT {{ ?x a {c_s} }} WHERE {{ ?x a {d_s} . \
                         FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
                    ));
                    rules.push(format!(
                        "INSERT {{ ?x a {d_s} }} WHERE {{ ?x a {c_s} . \
                         FILTER NOT EXISTS {{ ?x a {d_s} }} }}"
                    ));
                }
            }
        }
    }

    // Rules 4+4e: C equiv intersect(double-range(dp, facets), hasValue(hvp, hv))
    #[allow(clippy::items_after_statements)]
    #[derive(Default)]
    struct FacetState {
        min_incl: Option<String>,
        min_excl: Option<String>,
        max_incl: Option<String>,
        max_excl: Option<String>,
    }
    let mut double_intersections: HashMap<(String, String, String, String), FacetState> =
        HashMap::new();

    let q = format!(
        "SELECT ?C ?dp ?hvp ?hv ?facetPred ?facetVal WHERE {{ \
          ?C <{owl}equivalentClass> ?bn . \
          ?bn <{owl}intersectionOf> ?ilist . \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_range . \
          ?r_range <{owl}onProperty> ?dp . \
          ?r_range <{owl}someValuesFrom> ?dr . \
          ?dr <{owl}onDatatype> <{xsd}double> . \
          ?dr <{owl}withRestrictions> ?flist . \
          ?flist (<{rdf}rest>*/<{rdf}first>) ?facetNode . \
          ?facetNode ?facetPred ?facetVal . \
          FILTER(?facetPred IN (<{xsd}minInclusive>, <{xsd}minExclusive>, \
                                <{xsd}maxInclusive>, <{xsd}maxExclusive>)) \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_hv . \
          ?r_hv <{owl}onProperty> ?hvp . \
          ?r_hv <{owl}hasValue> ?hv . \
          FILTER(?r_range != ?r_hv) \
          FILTER(isIRI(?C)) \
          FILTER NOT EXISTS {{ \
            ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_int . \
            ?r_int <{owl}someValuesFrom> ?di . \
            ?di <{owl}onDatatype> <{xsd}integer> . \
          }} \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(c), Some(dp), Some(hvp), Some(hv), Some(fp), Some(fv)) = (
                row.get("C"),
                row.get("dp"),
                row.get("hvp"),
                row.get("hv"),
                row.get("facetPred"),
                row.get("facetVal"),
            ) {
                if !is_iri(c) || !is_iri(dp) || !is_iri(hvp) || !is_iri(fp) {
                    continue;
                }
                let key = (c.to_string(), dp.to_string(), hvp.to_string(), hv.to_string());
                let facets = double_intersections.entry(key).or_default();
                let fv_s = fv.to_string();
                if let RdfTerm::Iri(fp_url) = fp {
                    match fp_url.as_str() {
                        s if s.ends_with("minInclusive") => facets.min_incl = Some(fv_s),
                        s if s.ends_with("minExclusive") => facets.min_excl = Some(fv_s),
                        s if s.ends_with("maxInclusive") => facets.max_incl = Some(fv_s),
                        s if s.ends_with("maxExclusive") => facets.max_excl = Some(fv_s),
                        _ => {}
                    }
                }
            }
        }
    }
    for ((c_s, dp_s, hvp_s, hv_s), facets) in &double_intersections {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref v) = facets.min_incl {
            parts.push(format!("?numval >= {v}"));
        }
        if let Some(ref v) = facets.min_excl {
            parts.push(format!("?numval > {v}"));
        }
        if let Some(ref v) = facets.max_incl {
            parts.push(format!("?numval <= {v}"));
        }
        if let Some(ref v) = facets.max_excl {
            parts.push(format!("?numval < {v}"));
        }
        let filter_str = if parts.is_empty() {
            String::new()
        } else {
            format!("FILTER({}) ", parts.join(" && "))
        };
        rules.push(format!(
            "INSERT {{ ?x a {c_s} }} WHERE {{ \
             ?x {dp_s} ?numval . ?x {hvp_s} {hv_s} . \
             {filter_str}FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
        ));
    }

    // Rules 4b+4d: C equiv intersect(integer-range, double-range, hasValue)
    #[derive(Default)]
    struct BiRangeFacets {
        int_min_incl: Option<String>,
        int_min_excl: Option<String>,
        int_max_incl: Option<String>,
        int_max_excl: Option<String>,
        dbl_min_incl: Option<String>,
        dbl_min_excl: Option<String>,
        dbl_max_incl: Option<String>,
        dbl_max_excl: Option<String>,
    }
    let mut int_dbl: HashMap<(String, String, String, String, String), BiRangeFacets> =
        HashMap::new();

    let q = format!(
        "SELECT ?C ?ip ?dp ?hvp ?hv ?intFP ?intFV ?dblFP ?dblFV WHERE {{ \
          ?C <{owl}equivalentClass> ?bn . \
          ?bn <{owl}intersectionOf> ?ilist . \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_int . \
          ?r_int <{owl}onProperty> ?ip . \
          ?r_int <{owl}someValuesFrom> ?di . \
          ?di <{owl}onDatatype> <{xsd}integer> . \
          ?di <{owl}withRestrictions> ?fi . \
          ?fi (<{rdf}rest>*/<{rdf}first>) ?intFN . \
          ?intFN ?intFP ?intFV . \
          FILTER(?intFP IN (<{xsd}minInclusive>,<{xsd}minExclusive>,\
                            <{xsd}maxInclusive>,<{xsd}maxExclusive>)) \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_dbl . \
          ?r_dbl <{owl}onProperty> ?dp . \
          ?r_dbl <{owl}someValuesFrom> ?dd . \
          ?dd <{owl}onDatatype> <{xsd}double> . \
          ?dd <{owl}withRestrictions> ?fd . \
          ?fd (<{rdf}rest>*/<{rdf}first>) ?dblFN . \
          ?dblFN ?dblFP ?dblFV . \
          FILTER(?dblFP IN (<{xsd}minInclusive>,<{xsd}minExclusive>,\
                            <{xsd}maxInclusive>,<{xsd}maxExclusive>)) \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_hv . \
          ?r_hv <{owl}onProperty> ?hvp . \
          ?r_hv <{owl}hasValue> ?hv . \
          FILTER(?r_int != ?r_dbl && ?r_int != ?r_hv && ?r_dbl != ?r_hv) \
          FILTER(isIRI(?C)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (
                Some(c),
                Some(ip),
                Some(dp),
                Some(hvp),
                Some(hv),
                Some(ifp),
                Some(ifv),
                Some(dfp),
                Some(dfv),
            ) = (
                row.get("C"),
                row.get("ip"),
                row.get("dp"),
                row.get("hvp"),
                row.get("hv"),
                row.get("intFP"),
                row.get("intFV"),
                row.get("dblFP"),
                row.get("dblFV"),
            ) {
                if !is_iri(c)
                    || !is_iri(ip)
                    || !is_iri(dp)
                    || !is_iri(hvp)
                    || !is_iri(ifp)
                    || !is_iri(dfp)
                {
                    continue;
                }
                let key = (
                    c.to_string(),
                    ip.to_string(),
                    dp.to_string(),
                    hvp.to_string(),
                    hv.to_string(),
                );
                let f = int_dbl.entry(key).or_default();
                let (ifv_s, dfv_s) = (ifv.to_string(), dfv.to_string());
                if let RdfTerm::Iri(iu) = ifp {
                    match iu.as_str() {
                        s if s.ends_with("minInclusive") => {
                            f.int_min_incl = Some(ifv_s.clone());
                        }
                        s if s.ends_with("minExclusive") => {
                            f.int_min_excl = Some(ifv_s.clone());
                        }
                        s if s.ends_with("maxInclusive") => {
                            f.int_max_incl = Some(ifv_s.clone());
                        }
                        s if s.ends_with("maxExclusive") => {
                            f.int_max_excl = Some(ifv_s.clone());
                        }
                        _ => {}
                    }
                }
                if let RdfTerm::Iri(du) = dfp {
                    match du.as_str() {
                        s if s.ends_with("minInclusive") => {
                            f.dbl_min_incl = Some(dfv_s.clone());
                        }
                        s if s.ends_with("minExclusive") => {
                            f.dbl_min_excl = Some(dfv_s.clone());
                        }
                        s if s.ends_with("maxInclusive") => {
                            f.dbl_max_incl = Some(dfv_s.clone());
                        }
                        s if s.ends_with("maxExclusive") => {
                            f.dbl_max_excl = Some(dfv_s.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    for ((c_s, ip_s, dp_s, hvp_s, hv_s), f) in &int_dbl {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref v) = f.int_min_incl {
            parts.push(format!("?intval >= {v}"));
        }
        if let Some(ref v) = f.int_min_excl {
            parts.push(format!("?intval > {v}"));
        }
        if let Some(ref v) = f.int_max_incl {
            parts.push(format!("?intval <= {v}"));
        }
        if let Some(ref v) = f.int_max_excl {
            parts.push(format!("?intval < {v}"));
        }
        if let Some(ref v) = f.dbl_min_incl {
            parts.push(format!("?dblval >= {v}"));
        }
        if let Some(ref v) = f.dbl_min_excl {
            parts.push(format!("?dblval > {v}"));
        }
        if let Some(ref v) = f.dbl_max_incl {
            parts.push(format!("?dblval <= {v}"));
        }
        if let Some(ref v) = f.dbl_max_excl {
            parts.push(format!("?dblval < {v}"));
        }
        let filter_str = if parts.is_empty() {
            String::new()
        } else {
            format!("FILTER({}) ", parts.join(" && "))
        };
        rules.push(format!(
            "INSERT {{ ?x a {c_s} }} WHERE {{ \
             ?x {ip_s} ?intval . ?x {dp_s} ?dblval . ?x {hvp_s} {hv_s} . \
             {filter_str}FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
        ));
    }

    // Rule 5b: C equiv intersect(NamedClass1, NamedClass2, ...)
    let q = format!(
        "SELECT DISTINCT ?C ?member WHERE {{ \
          ?C <{owl}equivalentClass> ?bn . \
          ?bn <{owl}intersectionOf> ?ilist . \
          ?ilist (<{rdf}rest>*/<{rdf}first>) ?member . \
          FILTER(isIRI(?member)) FILTER(isIRI(?C)) \
          FILTER NOT EXISTS {{ \
            ?ilist (<{rdf}rest>*/<{rdf}first>) ?r_restr . \
            ?r_restr <{rdf}type> <{owl}Restriction> . \
          }} \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        let mut named_intersections: HashMap<String, Vec<String>> = HashMap::new();
        for row in rows {
            if let (Some(c), Some(m)) = (row.get("C"), row.get("member")) {
                if !is_iri(c) || !is_iri(m) {
                    continue;
                }
                named_intersections
                    .entry(c.to_string())
                    .or_default()
                    .push(m.to_string());
            }
        }
        for (c_s, members) in named_intersections {
            if members.len() < 2 {
                continue;
            }
            let mut seen_m: std::collections::HashSet<&str> = std::collections::HashSet::new();
            let checks: Vec<String> = members
                .iter()
                .filter(|m| seen_m.insert(m.as_str()))
                .map(|m| format!("?x a {m}"))
                .collect();
            rules.push(format!(
                "INSERT {{ ?x a {c_s} }} WHERE {{ {} . \
                 FILTER NOT EXISTS {{ ?x a {c_s} }} }}",
                checks.join(" . ")
            ));
        }
    }

    // Rule 6: rdfs:subClassOf propagation
    let q = format!(
        "SELECT DISTINCT ?sub ?sup WHERE {{ \
          ?sub <{rdfs}subClassOf> ?sup . \
          FILTER(isIRI(?sub)) FILTER(isIRI(?sup)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(sub), Some(sup)) = (row.get("sub"), row.get("sup")) {
                if !is_iri(sub) || !is_iri(sup) {
                    continue;
                }
                let (sub_s, super_s) = (sub.to_string(), sup.to_string());
                rules.push(format!(
                    "INSERT {{ ?x a {super_s} }} WHERE {{ ?x a {sub_s} . \
                     FILTER NOT EXISTS {{ ?x a {super_s} }} }}"
                ));
            }
        }
    }

    // Rule 7: rdfs:domain
    let q = format!(
        "SELECT DISTINCT ?P ?C WHERE {{ \
          ?P <{rdfs}domain> ?C . \
          FILTER(isIRI(?P)) FILTER(isIRI(?C)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(p), Some(c)) = (row.get("P"), row.get("C")) {
                if !is_iri(p) || !is_iri(c) {
                    continue;
                }
                let (p_s, c_s) = (p.to_string(), c.to_string());
                rules.push(format!(
                    "INSERT {{ ?x a {c_s} }} WHERE {{ ?x {p_s} ?o . \
                     FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
                ));
            }
        }
    }

    // Rule 8: rdfs:range
    let q = format!(
        "SELECT DISTINCT ?P ?C WHERE {{ \
          ?P <{rdfs}range> ?C . \
          FILTER(isIRI(?P)) FILTER(isIRI(?C)) \
        }}"
    );
    if let Ok(rows) = store.execute_select(&q) {
        for row in rows {
            if let (Some(p), Some(c)) = (row.get("P"), row.get("C")) {
                if !is_iri(p) || !is_iri(c) {
                    continue;
                }
                let (p_s, c_s) = (p.to_string(), c.to_string());
                rules.push(format!(
                    "INSERT {{ ?x a {c_s} }} WHERE {{ ?s {p_s} ?x . FILTER(isIRI(?x)) . \
                     FILTER NOT EXISTS {{ ?x a {c_s} }} }}"
                ));
            }
        }
    }

    tracing::debug!(
        "TBox rule extraction: {} simple ABox rules generated",
        rules.len()
    );
    rules
}
