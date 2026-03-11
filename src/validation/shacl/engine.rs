//! SHACL validation engine — the main orchestrator.
//!
//! `ShaclValidator` takes a shapes graph (Turtle) and a data graph (Turtle),
//! parses both, resolves targets, evaluates all constraints, and produces a
//! spec-compliant `ShaclValidationReport`.

use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::query::sparql_store::SparqlStore;
use crate::semantics::RdfTerm;
use crate::validation::shacl::{
    constraints::{
        cardinality::{evaluate_max_count, evaluate_min_count},
        logical::{evaluate_and, evaluate_not, evaluate_or, evaluate_xone},
        other::{evaluate_closed, evaluate_has_value, evaluate_in},
        property_pair::{
            evaluate_disjoint, evaluate_equals, evaluate_less_than, evaluate_less_than_or_equals,
        },
        shape_based::{evaluate_node_constraint, evaluate_qualified_value_shape},
        string_based::{
            evaluate_language_in, evaluate_max_length, evaluate_min_length, evaluate_pattern,
            evaluate_unique_lang,
        },
        value_range::{
            evaluate_max_exclusive, evaluate_max_inclusive, evaluate_min_exclusive,
            evaluate_min_inclusive,
        },
        value_type::{evaluate_class, evaluate_datatype, evaluate_node_kind},
    },
    model::{ShaclConstraint, ShaclShape, ShapeId},
    parser::parse_shapes_graph,
    paths::resolve_values,
    report::{ShaclValidationReport, ShaclValidationResult},
    sparql_constraints::evaluate_sparql_constraint,
    targets::resolve_targets,
};

/// Configuration for the SHACL validator.
#[derive(Debug, Clone)]
pub struct ShaclConfig {
    /// Maximum recursion depth for logical/shape-based constraints.
    pub max_recursion_depth: usize,
    /// Whether to apply entailment triples from a `ReasoningService`.
    pub use_entailment: bool,
    /// Whether to populate `sh:detail` for nested results.
    pub report_details: bool,
    /// Optional cap on the total number of validation results.
    pub max_results: Option<usize>,
}

impl Default for ShaclConfig {
    fn default() -> Self {
        ShaclConfig {
            max_recursion_depth: 50,
            use_entailment: true,
            report_details: true,
            max_results: None,
        }
    }
}

/// The main SHACL validator.
pub struct ShaclValidator {
    /// Parsed shapes.
    shapes: Vec<ShaclShape>,
    /// Oxigraph-backed data graph store.
    data_store: SparqlStore,
    /// Shapes graph well-formedness flag.
    shapes_graph_well_formed: bool,
    /// Recursion guard: `(focus_node, shape_id)` pairs currently being evaluated.
    recursion_stack: HashSet<RecursionKey>,
    /// Configuration.
    config: ShaclConfig,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct RecursionKey {
    focus: String,
    shape: String,
}

impl RecursionKey {
    fn new(focus: &RdfTerm, shape: &RdfTerm) -> Self {
        RecursionKey {
            focus: format!("{focus:?}"),
            shape: format!("{shape:?}"),
        }
    }
}

impl ShaclValidator {
    /// Create a new validator from Turtle strings for the shapes graph and data
    /// graph.
    pub fn new(shapes_turtle: &str, data_turtle: &str) -> Result<Self> {
        Self::with_config(shapes_turtle, data_turtle, ShaclConfig::default())
    }

    /// Create a new validator with a custom `ShaclConfig`.
    pub fn with_config(
        shapes_turtle: &str,
        data_turtle: &str,
        config: ShaclConfig,
    ) -> Result<Self> {
        let (shapes, shapes_graph_well_formed) = parse_shapes_graph(shapes_turtle)?;

        let mut data_store = SparqlStore::new()?;
        if !data_turtle.is_empty() {
            data_store
                .load_turtle(data_turtle)
                .map_err(|e| Error::shacl(format!("Failed to load data graph: {e}")))?;
        }

        Ok(ShaclValidator {
            shapes,
            data_store,
            shapes_graph_well_formed,
            recursion_stack: HashSet::new(),
            config,
        })
    }

    /// Run validation and return the full `ShaclValidationReport`.
    pub fn validate(&mut self) -> Result<ShaclValidationReport> {
        if !self.shapes_graph_well_formed {
            let mut report = ShaclValidationReport::conforming();
            report.shapes_graph_well_formed = Some(false);
            return Ok(report);
        }

        let mut all_results: Vec<ShaclValidationResult> = Vec::new();

        // Clone shapes to avoid borrow issues during recursive calls.
        let shapes: Vec<ShaclShape> = self.shapes.clone();

        for shape in &shapes {
            if shape.is_deactivated() {
                continue;
            }

            let focus_nodes = resolve_targets(&self.data_store, shape.targets())?;

            for focus_node in focus_nodes {
                let results = self.validate_focus_node(&focus_node, shape, &shapes)?;
                all_results.extend(results);

                if let Some(max) = self.config.max_results
                    && all_results.len() >= max {
                        let conforms = all_results.is_empty();
                        return Ok(ShaclValidationReport {
                            conforms,
                            results: all_results,
                            shapes_graph_well_formed: Some(self.shapes_graph_well_formed),
                        });
                    }
            }
        }

        let conforms = all_results.is_empty();
        Ok(ShaclValidationReport {
            conforms,
            results: all_results,
            shapes_graph_well_formed: Some(self.shapes_graph_well_formed),
        })
    }

    /// Validate a single focus node against a shape, using the given set of
    /// all shapes (needed for recursive lookups).
    fn validate_focus_node(
        &mut self,
        focus_node: &RdfTerm,
        shape: &ShaclShape,
        all_shapes: &[ShaclShape],
    ) -> Result<Vec<ShaclValidationResult>> {
        let key = RecursionKey::new(focus_node, shape.id());

        // Recursion guard: return empty (implementation-defined: assume conformance)
        if self.recursion_stack.len() >= self.config.max_recursion_depth {
            return Ok(Vec::new());
        }
        if self.recursion_stack.contains(&key) {
            return Ok(Vec::new()); // cycle detected
        }
        self.recursion_stack.insert(key.clone());

        let result = self.evaluate_shape_constraints(focus_node, shape, all_shapes);

        self.recursion_stack.remove(&key);
        result
    }

    fn evaluate_shape_constraints(
        &mut self,
        focus_node: &RdfTerm,
        shape: &ShaclShape,
        all_shapes: &[ShaclShape],
    ) -> Result<Vec<ShaclValidationResult>> {
        let mut out = Vec::new();
        let severity = shape.severity();
        let source_shape = Some(shape.id());
        let messages = shape.messages();

        for constraint in shape.constraints() {
            // For property shapes, resolve values via path; for node shapes via
            // the focus node itself.
            let values: Vec<RdfTerm> = match shape {
                ShaclShape::PropertyShape(ps) => {
                    resolve_values(&self.data_store, focus_node, &ps.path)?
                }
                ShaclShape::NodeShape(_) => vec![focus_node.clone()],
            };

            let results = self.evaluate_single_constraint(
                focus_node,
                &values,
                constraint,
                severity,
                source_shape,
                messages,
                all_shapes,
                shape,
            )?;
            out.extend(results);
        }

        Ok(out)
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate_single_constraint(
        &mut self,
        focus_node: &RdfTerm,
        values: &[RdfTerm],
        constraint: &ShaclConstraint,
        severity: &crate::validation::shacl::model::ShaclSeverity,
        source_shape: Option<&RdfTerm>,
        messages: &[crate::validation::shacl::model::ShaclMessage],
        all_shapes: &[ShaclShape],
        current_shape: &ShaclShape,
    ) -> Result<Vec<ShaclValidationResult>> {
        match constraint {
            // ── Value type ────────────────────────────────────────────────
            ShaclConstraint::Class(class) => evaluate_class(
                &self.data_store,
                focus_node,
                values,
                class,
                severity,
                source_shape,
                messages,
            ),
            ShaclConstraint::Datatype(dt_iri) => Ok(evaluate_datatype(
                values,
                dt_iri,
                focus_node,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::NodeKind(kind) => Ok(evaluate_node_kind(
                values,
                kind,
                focus_node,
                severity,
                source_shape,
                messages,
            )),

            // ── Cardinality ───────────────────────────────────────────────
            ShaclConstraint::MinCount(n) => Ok(evaluate_min_count(
                focus_node,
                values,
                *n,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::MaxCount(n) => Ok(evaluate_max_count(
                focus_node,
                values,
                *n,
                severity,
                source_shape,
                messages,
            )),

            // ── Value range ───────────────────────────────────────────────
            ShaclConstraint::MinExclusive(bound) => Ok(evaluate_min_exclusive(
                focus_node,
                values,
                bound,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::MinInclusive(bound) => Ok(evaluate_min_inclusive(
                focus_node,
                values,
                bound,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::MaxExclusive(bound) => Ok(evaluate_max_exclusive(
                focus_node,
                values,
                bound,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::MaxInclusive(bound) => Ok(evaluate_max_inclusive(
                focus_node,
                values,
                bound,
                severity,
                source_shape,
                messages,
            )),

            // ── String-based ──────────────────────────────────────────────
            ShaclConstraint::MinLength(n) => Ok(evaluate_min_length(
                focus_node,
                values,
                *n,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::MaxLength(n) => Ok(evaluate_max_length(
                focus_node,
                values,
                *n,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::Pattern { pattern, flags } => Ok(evaluate_pattern(
                focus_node,
                values,
                pattern,
                flags.as_deref(),
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::LanguageIn(langs) => Ok(evaluate_language_in(
                focus_node,
                values,
                langs,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::UniqueLang(_) => Ok(evaluate_unique_lang(
                focus_node,
                values,
                severity,
                source_shape,
                messages,
            )),

            // ── Property pair ─────────────────────────────────────────────
            ShaclConstraint::Equals(prop) => evaluate_equals(
                &self.data_store,
                focus_node,
                values,
                prop,
                severity,
                source_shape,
                messages,
            ),
            ShaclConstraint::Disjoint(prop) => evaluate_disjoint(
                &self.data_store,
                focus_node,
                values,
                prop,
                severity,
                source_shape,
                messages,
            ),
            ShaclConstraint::LessThan(prop) => evaluate_less_than(
                &self.data_store,
                focus_node,
                values,
                prop,
                severity,
                source_shape,
                messages,
            ),
            ShaclConstraint::LessThanOrEquals(prop) => evaluate_less_than_or_equals(
                &self.data_store,
                focus_node,
                values,
                prop,
                severity,
                source_shape,
                messages,
            ),

            // ── Logical ───────────────────────────────────────────────────
            ShaclConstraint::Not(inner_id) => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_not(
                    focus_node,
                    inner_id,
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }
            ShaclConstraint::And(shape_ids) => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_and(
                    focus_node,
                    shape_ids,
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }
            ShaclConstraint::Or(shape_ids) => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_or(
                    focus_node,
                    shape_ids,
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }
            ShaclConstraint::Xone(shape_ids) => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_xone(
                    focus_node,
                    shape_ids,
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }

            // ── Shape-based ───────────────────────────────────────────────
            ShaclConstraint::Node(node_id) => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_node_constraint(
                    focus_node,
                    values,
                    node_id,
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }
            ShaclConstraint::Property(prop_id) => {
                // Recursively validate each value node as focus against the property shape
                let all_shapes_clone = all_shapes.to_vec();
                let prop_shape = all_shapes_clone.iter().find(|s| s.id() == prop_id);
                match prop_shape {
                    Some(ps) => {
                        let mut sub_results = Vec::new();
                        let ps_clone = ps.clone();
                        for value in values {
                            let r =
                                self.validate_focus_node(value, &ps_clone, &all_shapes_clone)?;
                            sub_results.extend(r);
                        }
                        Ok(sub_results)
                    }
                    None => Ok(Vec::new()),
                }
            }
            ShaclConstraint::QualifiedValue {
                shape_id,
                min_count,
                max_count,
                ..
            } => {
                let all_shapes_clone = all_shapes.to_vec();
                evaluate_qualified_value_shape(
                    focus_node,
                    values,
                    shape_id,
                    min_count.as_ref().copied(),
                    max_count.as_ref().copied(),
                    &mut |node, sid| self.conforms_to_shape(node, sid, &all_shapes_clone),
                    severity,
                    source_shape,
                    messages,
                )
            }

            // ── Other ─────────────────────────────────────────────────────
            ShaclConstraint::Closed { ignored } => {
                // Allowed predicates = union of all sh:property/sh:path values
                let allowed = collect_allowed_predicates(all_shapes, current_shape);
                evaluate_closed(
                    &self.data_store,
                    focus_node,
                    &allowed,
                    ignored,
                    severity,
                    source_shape,
                    messages,
                )
            }
            ShaclConstraint::HasValue(required) => Ok(evaluate_has_value(
                focus_node,
                values,
                required,
                severity,
                source_shape,
                messages,
            )),
            ShaclConstraint::In(allowed) => Ok(evaluate_in(
                focus_node,
                values,
                allowed,
                severity,
                source_shape,
                messages,
            )),

            // ── SPARQL ────────────────────────────────────────────────────
            ShaclConstraint::Sparql(sc) => {
                evaluate_sparql_constraint(&self.data_store, focus_node, sc, severity, source_shape)
            }
            ShaclConstraint::SparqlComponent(_) => {
                // TODO: full custom component resolution requires a components
                // registry; returning Ok for now.
                Ok(Vec::new())
            }
        }
    }

    /// Returns `(conforms, sub_results)` for `node` against the shape
    /// identified by `shape_id`.
    fn conforms_to_shape(
        &mut self,
        node: &RdfTerm,
        shape_id: &ShapeId,
        all_shapes: &[ShaclShape],
    ) -> Result<(bool, Vec<ShaclValidationResult>)> {
        let shape = match all_shapes.iter().find(|s| s.id() == shape_id) {
            Some(s) => s.clone(),
            None => return Ok((true, Vec::new())), // unknown shape → assume conformance
        };

        let results = self.validate_focus_node(node, &shape, all_shapes)?;
        let conforms = results.is_empty();
        Ok((conforms, results))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect all predicates allowed by `sh:property`/`sh:path` on a shape.
fn collect_allowed_predicates(all_shapes: &[ShaclShape], shape: &ShaclShape) -> Vec<RdfTerm> {
    let mut allowed = Vec::new();

    // For node shapes, look at all sh:property references
    if let ShaclShape::NodeShape(ns) = shape {
        for prop_id in &ns.properties {
            if let Some(ShaclShape::PropertyShape(ps)) =
                all_shapes.iter().find(|s| s.id() == prop_id)
                && let crate::validation::shacl::model::ShaclPath::Predicate(iri) = &ps.path
                    && let Ok(t) = RdfTerm::iri(iri) {
                        allowed.push(t);
                    }
        }
    }

    // Always allow rdf:type
    if let Ok(rdf_type) = RdfTerm::iri(crate::validation::shacl::vocabulary::RDF_TYPE) {
        allowed.push(rdf_type);
    }

    allowed
}
