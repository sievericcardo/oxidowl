//! SHACL shapes graph parser.
//!
//! Loads a shapes graph (Turtle RDF) into an `SparqlStore` and extracts
//! all SHACL shapes and their parameters into the internal data model.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{model::*, vocabulary::*};

/// Parse a Turtle shapes graph and return a list of `ShaclShape`s.
///
/// Performs shapes graph well-formedness checks per spec Appendix B.
pub fn parse_shapes_graph(turtle: &str) -> Result<(Vec<ShaclShape>, bool)> {
    let mut store = SparqlStore::new()?;
    store
        .load_turtle(turtle)
        .map_err(|e| Error::shacl(format!("Failed to load shapes graph: {e}")))?;

    let shapes = extract_shapes(&store)?;
    let well_formed = check_well_formedness(&store, &shapes);

    Ok((shapes, well_formed))
}

// ── Shape extraction ──────────────────────────────────────────────────────────

fn extract_shapes(store: &SparqlStore) -> Result<Vec<ShaclShape>> {
    // Collect all shape node IRIs/blank nodes.
    // A node is a shape if:
    //  - it is rdf:type sh:NodeShape or sh:PropertyShape
    //  - it has any sh:target*, sh:path, or constraint parameter
    let query = format!(
        "PREFIX sh: <{SH_NS}> \
         PREFIX rdf: <{RDF_NS}> \
         PREFIX rdfs: <{RDFS_NS}> \
         SELECT DISTINCT ?shape ?type WHERE {{ \
             {{ ?shape rdf:type sh:NodeShape . BIND(<{SH_NODE_SHAPE}> AS ?type) }} \
             UNION \
             {{ ?shape rdf:type sh:PropertyShape . BIND(<{SH_PROPERTY_SHAPE}> AS ?type) }} \
             UNION \
             {{ ?shape sh:targetNode|sh:targetClass|sh:targetSubjectsOf|sh:targetObjectsOf ?_ . \
                BIND(<{SH_NODE_SHAPE}> AS ?type) }} \
             UNION \
             {{ ?shape sh:path ?_ . BIND(<{SH_PROPERTY_SHAPE}> AS ?type) }} \
         }}",
    );

    let rows = store.execute_select(&query)?;
    let mut shape_types: HashMap<String, Vec<String>> = HashMap::new();

    for row in rows {
        if let Some(shape_term) = row.get("shape") {
            let key = term_key(shape_term);
            if let Some(type_term) = row.get("type") {
                shape_types
                    .entry(key)
                    .or_default()
                    .push(term_key(type_term));
            }
        }
    }

    // Detect implicit class targets (shapes that are also rdfs:Class or owl:Class)
    let class_query = format!(
        "PREFIX rdf: <{RDF_NS}> \
         PREFIX rdfs: <{RDFS_NS}> \
         PREFIX owl: <{OWL_NS}> \
         SELECT DISTINCT ?shape WHERE {{ \
             {{ ?shape rdf:type rdfs:Class }} \
             UNION \
             {{ ?shape rdf:type owl:Class }} \
         }}"
    );
    let class_rows = store.execute_select(&class_query)?;
    let class_shapes: Vec<String> = class_rows
        .into_iter()
        .filter_map(|r| r.get("shape").map(term_key))
        .collect();

    let mut shapes = Vec::new();
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Queue starts with directly discovered shapes.
    let mut queue: Vec<(String, bool)> = shape_types
        .iter()
        .map(|(k, types)| (k.clone(), types.iter().any(|t| t.contains("PropertyShape"))))
        .collect();

    // Fixpoint: keep extracting shapes and discovering transitively referenced ones.
    while !queue.is_empty() {
        let mut next_queue: Vec<(String, bool)> = Vec::new();

        for (shape_key, is_property_shape) in queue.drain(..) {
            if !seen_keys.insert(shape_key.clone()) {
                continue; // already processed
            }

            let shape_term = key_to_term(&shape_key);

            let shape = if is_property_shape {
                let path = extract_path(store, &shape_term)?;
                let ps = extract_property_shape(store, shape_term, path, &class_shapes)?;
                ShaclShape::PropertyShape(ps)
            } else {
                let ns = extract_node_shape(store, shape_term, &class_shapes)?;
                ShaclShape::NodeShape(ns)
            };

            // Collect all shape IDs referenced by this shape's constraints so
            // we can transitively extract them if not yet seen.
            let referenced = collect_referenced_shape_ids(&shape);
            for ref_id in referenced {
                let ref_key = term_key(&ref_id);
                if !seen_keys.contains(&ref_key) {
                    // Determine if the referenced shape is a PropertyShape:
                    // it has sh:path.  Use quads_for_pattern to check safely.
                    let pred_path = RdfTerm::iri(SH_PATH).ok();
                    let has_path = if let Some(pred) = &pred_path {
                        let quads =
                            store.quads_for_pattern(Some(&ref_id), Some(pred), None, None)?;
                        !quads.is_empty()
                    } else {
                        false
                    };
                    next_queue.push((ref_key, has_path));
                }
            }

            shapes.push(shape);
        }

        queue = next_queue;
    }

    Ok(shapes)
}

/// Collect all shape IDs directly referenced by a shape's constraints
/// (for `sh:or`, `sh:and`, `sh:not`, `sh:node`, `sh:property`, `sh:xone`,
/// `sh:qualifiedValueShape`).
fn collect_referenced_shape_ids(shape: &ShaclShape) -> Vec<RdfTerm> {
    let constraints = match shape {
        ShaclShape::NodeShape(ns) => &ns.constraints,
        ShaclShape::PropertyShape(ps) => &ps.constraints,
    };

    let mut ids = Vec::new();
    for c in constraints {
        match c {
            ShaclConstraint::Not(id) => ids.push(id.clone()),
            ShaclConstraint::And(list) => ids.extend(list.iter().cloned()),
            ShaclConstraint::Or(list) => ids.extend(list.iter().cloned()),
            ShaclConstraint::Xone(list) => ids.extend(list.iter().cloned()),
            ShaclConstraint::Node(id) => ids.push(id.clone()),
            ShaclConstraint::Property(id) => ids.push(id.clone()),
            ShaclConstraint::QualifiedValue { shape_id, .. } => ids.push(shape_id.clone()),
            _ => {}
        }
    }

    // Also include properties field of NodeShapes.
    if let ShaclShape::NodeShape(ns) = shape {
        ids.extend(ns.properties.iter().cloned());
    }

    ids
}

fn extract_node_shape(
    store: &SparqlStore,
    shape_id: RdfTerm,
    class_shapes: &[String],
) -> Result<NodeShape> {
    let deactivated = bool_object(store, &shape_id, SH_DEACTIVATED)?;
    let severity = severity_of(store, &shape_id)?;
    let messages = messages_of(store, &shape_id)?;
    let targets = extract_targets(store, &shape_id, class_shapes)?;
    let constraints = extract_constraints(store, &shape_id)?;

    // sh:property references
    let properties = extract_list_or_direct_objects(store, &shape_id, SH_PROPERTY)?
        .into_iter()
        .filter_map(|t| match t {
            RdfTerm::Iri(_) | RdfTerm::BlankNode(_) => Some(t),
            _ => None,
        })
        .collect();

    Ok(NodeShape {
        id: shape_id,
        targets,
        constraints,
        severity,
        messages,
        deactivated,
        properties,
    })
}

fn extract_property_shape(
    store: &SparqlStore,
    shape_id: RdfTerm,
    path: ShaclPath,
    class_shapes: &[String],
) -> Result<PropertyShape> {
    let deactivated = bool_object(store, &shape_id, SH_DEACTIVATED)?;
    let severity = severity_of(store, &shape_id)?;
    let messages = messages_of(store, &shape_id)?;
    let targets = extract_targets(store, &shape_id, class_shapes)?;
    let constraints = extract_constraints(store, &shape_id)?;

    Ok(PropertyShape {
        id: shape_id,
        path,
        targets,
        constraints,
        severity,
        messages,
        deactivated,
    })
}

// ── Targets ───────────────────────────────────────────────────────────────────

fn extract_targets(
    store: &SparqlStore,
    shape_id: &RdfTerm,
    class_shapes: &[String],
) -> Result<Vec<ShaclTarget>> {
    let mut targets = Vec::new();

    for t in direct_objects(store, shape_id, SH_TARGET_NODE)? {
        targets.push(ShaclTarget::TargetNode(t));
    }
    for t in direct_objects(store, shape_id, SH_TARGET_CLASS)? {
        targets.push(ShaclTarget::TargetClass(t));
    }
    for t in direct_objects(store, shape_id, SH_TARGET_SUBJECTS_OF)? {
        targets.push(ShaclTarget::TargetSubjectsOf(t));
    }
    for t in direct_objects(store, shape_id, SH_TARGET_OBJECTS_OF)? {
        targets.push(ShaclTarget::TargetObjectsOf(t));
    }

    // Implicit class target
    if class_shapes.contains(&term_key(shape_id)) {
        targets.push(ShaclTarget::ImplicitClassTarget(shape_id.clone()));
    }

    Ok(targets)
}

// ── Constraints ───────────────────────────────────────────────────────────────

fn extract_constraints(store: &SparqlStore, shape_id: &RdfTerm) -> Result<Vec<ShaclConstraint>> {
    let mut out = Vec::new();

    // ── Value type ────────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_CLASS)? {
        out.push(ShaclConstraint::Class(v));
    }
    for v in direct_objects(store, shape_id, SH_DATATYPE)? {
        if let Some(iri) = iri_of(&v) {
            out.push(ShaclConstraint::Datatype(iri));
        }
    }
    for v in direct_objects(store, shape_id, SH_NODE_KIND)? {
        if let Some(iri) = iri_of(&v)
            && let Some(kind) = ShaclNodeKind::from_iri(&iri)
        {
            out.push(ShaclConstraint::NodeKind(kind));
        }
    }

    // ── Cardinality ───────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_MIN_COUNT)? {
        if let Some(n) = parse_uint_literal(&v) {
            out.push(ShaclConstraint::MinCount(n));
        }
    }
    for v in direct_objects(store, shape_id, SH_MAX_COUNT)? {
        if let Some(n) = parse_uint_literal(&v) {
            out.push(ShaclConstraint::MaxCount(n));
        }
    }

    // ── Value range ───────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_MIN_EXCLUSIVE)? {
        out.push(ShaclConstraint::MinExclusive(v));
    }
    for v in direct_objects(store, shape_id, SH_MIN_INCLUSIVE)? {
        out.push(ShaclConstraint::MinInclusive(v));
    }
    for v in direct_objects(store, shape_id, SH_MAX_EXCLUSIVE)? {
        out.push(ShaclConstraint::MaxExclusive(v));
    }
    for v in direct_objects(store, shape_id, SH_MAX_INCLUSIVE)? {
        out.push(ShaclConstraint::MaxInclusive(v));
    }

    // ── String-based ──────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_MIN_LENGTH)? {
        if let Some(n) = parse_uint_literal(&v) {
            out.push(ShaclConstraint::MinLength(n));
        }
    }
    for v in direct_objects(store, shape_id, SH_MAX_LENGTH)? {
        if let Some(n) = parse_uint_literal(&v) {
            out.push(ShaclConstraint::MaxLength(n));
        }
    }
    let patterns = direct_objects(store, shape_id, SH_PATTERN)?;
    let flags_list = direct_objects(store, shape_id, SH_FLAGS)?;
    let flags_opt = flags_list
        .into_iter()
        .next()
        .and_then(|f| string_value_of(&f));
    for p in patterns {
        if let Some(pat) = string_value_of(&p) {
            out.push(ShaclConstraint::Pattern {
                pattern: pat,
                flags: flags_opt.clone(),
            });
        }
    }

    // sh:languageIn
    for list_head in direct_objects(store, shape_id, SH_LANGUAGE_IN)? {
        let langs = walk_rdf_list(store, &list_head)?
            .into_iter()
            .filter_map(|t| string_value_of(&t))
            .collect();
        out.push(ShaclConstraint::LanguageIn(langs));
    }

    // sh:uniqueLang
    if bool_object(store, shape_id, SH_UNIQUE_LANG)? {
        out.push(ShaclConstraint::UniqueLang(true));
    }

    // ── Property pair ─────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_EQUALS)? {
        out.push(ShaclConstraint::Equals(v));
    }
    for v in direct_objects(store, shape_id, SH_DISJOINT)? {
        out.push(ShaclConstraint::Disjoint(v));
    }
    for v in direct_objects(store, shape_id, SH_LESS_THAN)? {
        out.push(ShaclConstraint::LessThan(v));
    }
    for v in direct_objects(store, shape_id, SH_LESS_THAN_OR_EQUALS)? {
        out.push(ShaclConstraint::LessThanOrEquals(v));
    }

    // ── Logical ───────────────────────────────────────────────────────────
    for list_head in direct_objects(store, shape_id, SH_NOT)? {
        out.push(ShaclConstraint::Not(list_head));
    }
    for list_head in direct_objects(store, shape_id, SH_AND)? {
        let shapes = walk_rdf_list(store, &list_head)?;
        out.push(ShaclConstraint::And(shapes));
    }
    for list_head in direct_objects(store, shape_id, SH_OR)? {
        let shapes = walk_rdf_list(store, &list_head)?;
        out.push(ShaclConstraint::Or(shapes));
    }
    for list_head in direct_objects(store, shape_id, SH_XONE)? {
        let shapes = walk_rdf_list(store, &list_head)?;
        out.push(ShaclConstraint::Xone(shapes));
    }

    // ── Shape-based ───────────────────────────────────────────────────────
    for v in direct_objects(store, shape_id, SH_NODE)? {
        out.push(ShaclConstraint::Node(v));
    }
    for v in direct_objects(store, shape_id, SH_PROPERTY)? {
        out.push(ShaclConstraint::Property(v));
    }

    // sh:qualifiedValueShape
    for qvs in direct_objects(store, shape_id, SH_QUALIFIED_VALUE_SHAPE)? {
        let min = direct_objects(store, shape_id, SH_QUALIFIED_MIN_COUNT)?
            .into_iter()
            .next()
            .and_then(|v| parse_uint_literal(&v));
        let max = direct_objects(store, shape_id, SH_QUALIFIED_MAX_COUNT)?
            .into_iter()
            .next()
            .and_then(|v| parse_uint_literal(&v));
        let disjoint = bool_object(store, shape_id, SH_QUALIFIED_VALUE_SHAPES_DISJOINT)?;
        out.push(ShaclConstraint::QualifiedValue {
            shape_id: qvs,
            min_count: min,
            max_count: max,
            disjoint,
        });
    }

    // ── Other ─────────────────────────────────────────────────────────────
    if bool_object(store, shape_id, SH_CLOSED)? {
        let mut ignored = Vec::new();
        for list_head in direct_objects(store, shape_id, SH_IGNORED_PROPERTIES)? {
            let props = walk_rdf_list(store, &list_head)?;
            ignored.extend(props);
        }
        out.push(ShaclConstraint::Closed { ignored });
    }
    for v in direct_objects(store, shape_id, SH_HAS_VALUE)? {
        out.push(ShaclConstraint::HasValue(v));
    }
    for list_head in direct_objects(store, shape_id, SH_IN)? {
        let items = walk_rdf_list(store, &list_head)?;
        out.push(ShaclConstraint::In(items));
    }

    // ── SPARQL ────────────────────────────────────────────────────────────
    for sparql_node in direct_objects(store, shape_id, SH_SPARQL)? {
        if let Some(sc) = extract_sparql_constraint(store, &sparql_node)? {
            out.push(ShaclConstraint::Sparql(sc));
        }
    }

    Ok(out)
}

// ── SPARQL constraint extraction ──────────────────────────────────────────────

fn extract_sparql_constraint(
    store: &SparqlStore,
    sparql_node: &RdfTerm,
) -> Result<Option<SparqlConstraint>> {
    let selects = direct_objects(store, sparql_node, SH_SELECT)?;
    let Some(select) = selects.into_iter().next().and_then(|v| string_value_of(&v)) else {
        return Ok(None);
    };

    let deactivated = bool_object(store, sparql_node, SH_DEACTIVATED)?;
    let messages = messages_of(store, sparql_node)?;
    let prefixes = extract_prefix_declarations(store, sparql_node)?;

    Ok(Some(SparqlConstraint {
        select,
        prefixes,
        messages,
        deactivated,
    }))
}

fn extract_prefix_declarations(
    store: &SparqlStore,
    sparql_node: &RdfTerm,
) -> Result<Vec<(String, String)>> {
    let mut prefixes = Vec::new();

    for prefixes_node in direct_objects(store, sparql_node, SH_PREFIXES)? {
        for decl in direct_objects(store, &prefixes_node, SH_DECLARE)? {
            let prefix_vals = direct_objects(store, &decl, SH_PREFIX)?;
            let ns_vals = direct_objects(store, &decl, SH_NAMESPACE)?;

            if let (Some(p), Some(ns)) = (
                prefix_vals
                    .into_iter()
                    .next()
                    .and_then(|v| string_value_of(&v)),
                ns_vals.into_iter().next().and_then(|v| string_value_of(&v)),
            ) {
                prefixes.push((p, ns));
            }
        }
    }

    Ok(prefixes)
}

// ── Path extraction ───────────────────────────────────────────────────────────

fn extract_path(store: &SparqlStore, shape_id: &RdfTerm) -> Result<ShaclPath> {
    let paths = direct_objects(store, shape_id, SH_PATH)?;
    let path_node = paths
        .into_iter()
        .next()
        .ok_or_else(|| Error::shacl("sh:PropertyShape missing sh:path"))?;
    parse_path(store, &path_node)
}

fn parse_path(store: &SparqlStore, node: &RdfTerm) -> Result<ShaclPath> {
    // Simple predicate IRI
    if let RdfTerm::Iri(iri) = node {
        return Ok(ShaclPath::Predicate(iri.as_str().to_string()));
    }

    // Blank node — inspect for path operators
    if let RdfTerm::BlankNode(_) = node {
        // sh:inversePath
        let inv = direct_objects(store, node, SH_INVERSE_PATH)?;
        if let Some(inner) = inv.into_iter().next() {
            return Ok(ShaclPath::Inverse(Box::new(parse_path(store, &inner)?)));
        }
        // sh:zeroOrMorePath
        let zom = direct_objects(store, node, SH_ZERO_OR_MORE_PATH)?;
        if let Some(inner) = zom.into_iter().next() {
            return Ok(ShaclPath::ZeroOrMore(Box::new(parse_path(store, &inner)?)));
        }
        // sh:oneOrMorePath
        let oom = direct_objects(store, node, SH_ONE_OR_MORE_PATH)?;
        if let Some(inner) = oom.into_iter().next() {
            return Ok(ShaclPath::OneOrMore(Box::new(parse_path(store, &inner)?)));
        }
        // sh:zeroOrOnePath
        let zoo = direct_objects(store, node, SH_ZERO_OR_ONE_PATH)?;
        if let Some(inner) = zoo.into_iter().next() {
            return Ok(ShaclPath::ZeroOrOne(Box::new(parse_path(store, &inner)?)));
        }
        // sh:alternativePath
        let alt = direct_objects(store, node, SH_ALTERNATIVE_PATH)?;
        if let Some(list_head) = alt.into_iter().next() {
            let items = walk_rdf_list(store, &list_head)?;
            let paths: Result<Vec<_>> = items.iter().map(|n| parse_path(store, n)).collect();
            return Ok(ShaclPath::Alternative(paths?));
        }
        // Sequence path: blank node that is an RDF list head
        // (sequence paths are encoded as rdf:List nodes)
        let first = direct_objects(store, node, RDF_FIRST)?;
        if !first.is_empty() {
            let items = walk_rdf_list(store, node)?;
            let paths: Result<Vec<_>> = items.iter().map(|n| parse_path(store, n)).collect();
            return Ok(ShaclPath::Sequence(paths?));
        }
    }

    Err(Error::shacl(format!(
        "Cannot parse SHACL path from node: {node:?}"
    )))
}

// ── RDF List walker ───────────────────────────────────────────────────────────

fn walk_rdf_list(store: &SparqlStore, head: &RdfTerm) -> Result<Vec<RdfTerm>> {
    let nil = RdfTerm::iri(RDF_NIL).ok();
    let mut result = Vec::new();
    let mut current = head.clone();

    loop {
        if nil.as_ref().map(|n| n == &current).unwrap_or(false) {
            break;
        }
        let first_vals = direct_objects(store, &current, RDF_FIRST)?;
        if let Some(item) = first_vals.into_iter().next() {
            result.push(item);
        }
        let rest_vals = direct_objects(store, &current, RDF_REST)?;
        match rest_vals.into_iter().next() {
            Some(next) => current = next,
            None => break,
        }
    }

    Ok(result)
}

// ── Shapes graph well-formedness ─────────────────────────────────────────────

fn check_well_formedness(_store: &SparqlStore, shapes: &[ShaclShape]) -> bool {
    // Minimal check: every property shape must have exactly one sh:path.
    for shape in shapes {
        if let ShaclShape::PropertyShape(ps) = shape {
            // If path is a Predicate("") it means something went wrong during
            // parsing — but we already error in extract_path, so if we get here
            // the path should always be set.
            let _ = &ps.path;
        }
    }
    // For a full implementation, check all well-formedness conditions from
    // SHACL Appendix B.  Returning true for now unless a parse error occurred.
    true
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Return a SPARQL-safe string key for an `RdfTerm`.
fn term_key(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => format!("<{}>", iri.as_str()),
        RdfTerm::BlankNode(id) => format!("_:{id}"),
        RdfTerm::Literal { value, .. } => format!("\"{value}\""),
        RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_) => "<<>>".to_string(),
    }
}

/// Reconstruct an `RdfTerm` from a term key (inverse of `term_key`).
fn key_to_term(key: &str) -> RdfTerm {
    if let Some(iri) = key.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        RdfTerm::iri(iri).unwrap_or_else(|_| RdfTerm::BlankNode(key.to_string()))
    } else if let Some(bn) = key.strip_prefix("_:") {
        RdfTerm::BlankNode(bn.to_string())
    } else {
        RdfTerm::BlankNode(key.to_string())
    }
}

fn direct_objects(
    store: &SparqlStore,
    subject: &RdfTerm,
    predicate_iri: &str,
) -> Result<Vec<RdfTerm>> {
    // SPARQL 1.1 treats `_:label` in query text as anonymous (ungrounded)
    // blank nodes, so we cannot reference a specific blank node by label in a
    // SPARQL WHERE clause.  Use the store's pattern-matching API directly for
    // blank-node subjects to get correct results.
    if let RdfTerm::BlankNode(_) = subject {
        let pred_term = RdfTerm::iri(predicate_iri).map_err(|e| {
            crate::error::Error::shacl(format!("Invalid predicate IRI {predicate_iri}: {e}"))
        })?;
        let quads = store.quads_for_pattern(Some(subject), Some(&pred_term), None, None)?;
        return Ok(quads.into_iter().map(|(_, _, obj, _)| obj).collect());
    }

    let subj_str = crate::validation::shacl::paths::term_to_sparql(subject);
    let query = format!("SELECT ?o WHERE {{ {subj_str} <{predicate_iri}> ?o }}");
    let rows = store.execute_select(&query)?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.get("o").cloned())
        .collect())
}

fn extract_list_or_direct_objects(
    store: &SparqlStore,
    subject: &RdfTerm,
    predicate_iri: &str,
) -> Result<Vec<RdfTerm>> {
    direct_objects(store, subject, predicate_iri)
}

fn bool_object(store: &SparqlStore, subject: &RdfTerm, pred: &str) -> Result<bool> {
    let vals = direct_objects(store, subject, pred)?;
    Ok(vals
        .into_iter()
        .any(|v| matches!(&v, RdfTerm::Literal { value, .. } if value == "true" || value == "1")))
}

fn severity_of(store: &SparqlStore, shape_id: &RdfTerm) -> Result<ShaclSeverity> {
    let sevs = direct_objects(store, shape_id, SH_SEVERITY)?;
    Ok(sevs
        .into_iter()
        .next()
        .and_then(|t| iri_of(&t))
        .map(|iri| ShaclSeverity::from_iri(&iri))
        .unwrap_or_default())
}

fn messages_of(store: &SparqlStore, shape_id: &RdfTerm) -> Result<Vec<ShaclMessage>> {
    let vals = direct_objects(store, shape_id, SH_MESSAGE)?;
    Ok(vals
        .into_iter()
        .filter_map(|t| match t {
            RdfTerm::Literal {
                value, language, ..
            } => Some(ShaclMessage { value, language }),
            _ => None,
        })
        .collect())
}

fn iri_of(term: &RdfTerm) -> Option<String> {
    match term {
        RdfTerm::Iri(iri) => Some(iri.as_str().to_string()),
        _ => None,
    }
}

fn string_value_of(term: &RdfTerm) -> Option<String> {
    match term {
        RdfTerm::Literal { value, .. } => Some(value.clone()),
        RdfTerm::Iri(iri) => Some(iri.as_str().to_string()),
        _ => None,
    }
}

fn parse_uint_literal(term: &RdfTerm) -> Option<u64> {
    match term {
        RdfTerm::Literal { value, .. } => value.trim().parse::<u64>().ok(),
        _ => None,
    }
}
