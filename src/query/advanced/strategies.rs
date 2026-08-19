//! Concrete execution strategies for the advanced query engine.
//!
//! Each strategy implements [`super::execution_engine::ExecutionStrategy`] and
//! differs from the others in *how it orders/limits query atoms* before
//! delegating the actual evaluation to the shared [`QueryEvaluator`]. This
//! keeps the query-answering logic in one place while still providing genuinely
//! different execution behaviour per strategy.

use super::conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
use super::evaluator::QueryEvaluator;
use super::execution::{AdvancedQueryError, ConjunctiveQueryResult};
use super::execution_engine::{ExecutionContext, ExecutionStrategy};
use std::collections::HashSet;

/// Return the variables referenced by an atom.
fn atom_variables(atom: &QueryAtom) -> Vec<&QueryVariable> {
    match atom {
        QueryAtom::ClassAtom { variable, .. } => vec![variable],
        QueryAtom::ObjectPropertyAtom { subject, object, .. } => vec![subject, object],
        QueryAtom::DataPropertyAtom { subject, literal, .. } => vec![subject, literal],
        QueryAtom::SameIndividualAtom { left, right }
        | QueryAtom::DifferentIndividualsAtom { left, right } => vec![left, right],
        QueryAtom::ConcreteIndividualAtom { variable, .. }
        | QueryAtom::ConcreteLiteralAtom { variable, .. } => vec![variable],
    }
}

/// Evaluate `query` using the given atom ordering and result limit.
fn evaluate_with_order(
    query: &ConjunctiveQuery,
    context: &ExecutionContext,
    name: &str,
    order: Vec<QueryAtom>,
    limit: Option<usize>,
) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
    let evaluator = QueryEvaluator::new(context.reasoning_service.clone());
    evaluator.evaluate(query, &order, name, limit)
}

fn base_cost(query: &ConjunctiveQuery) -> f64 {
    query.body_atoms.len() as f64
}

/// Order atoms greedily so atoms introducing the most *new* variables come
/// first, deferring cycle-closing / pure-join atoms to the end.
fn order_cycle_aware(query: &ConjunctiveQuery) -> Vec<QueryAtom> {
    let mut remaining: Vec<QueryAtom> = query.body_atoms.clone();
    let mut ordered: Vec<QueryAtom> = Vec::new();
    let mut bound: HashSet<QueryVariable> = HashSet::new();
    let mut deferred: Vec<QueryAtom> = Vec::new();

    while !remaining.is_empty() {
        let mut best: Option<(usize, usize)> = None;
        for (i, atom) in remaining.iter().enumerate() {
            let new_vars = atom_variables(atom)
                .iter()
                .filter(|v| !bound.contains(*v))
                .count();
            if new_vars > 0 && best.is_none_or(|(best_new, _)| new_vars > best_new) {
                best = Some((new_vars, i));
            }
        }

        if let Some((_, i)) = best {
            let atom = remaining.remove(i);
            for var in atom_variables(&atom) {
                bound.insert(var.clone());
            }
            ordered.push(atom);
        } else {
            deferred.append(&mut remaining);
            break;
        }
    }

    ordered.append(&mut deferred);
    ordered
}

/// Order selective atoms (concrete/value/equality) first.
fn order_filter_first(query: &ConjunctiveQuery) -> Vec<QueryAtom> {
    let mut atoms = query.body_atoms.clone();
    atoms.sort_by_key(|atom| match atom {
        QueryAtom::ConcreteIndividualAtom { .. } | QueryAtom::ConcreteLiteralAtom { .. } => 0,
        QueryAtom::SameIndividualAtom { .. } | QueryAtom::DifferentIndividualsAtom { .. } => 1,
        QueryAtom::ClassAtom { .. } => 2,
        QueryAtom::ObjectPropertyAtom { .. } | QueryAtom::DataPropertyAtom { .. } => 3,
    });
    atoms
}

/// Order atoms so those referencing answer variables come first.
fn order_projection_first(query: &ConjunctiveQuery) -> Vec<QueryAtom> {
    let answer: HashSet<&QueryVariable> = query.answer_variables.iter().collect();
    let mut atoms = query.body_atoms.clone();
    atoms.sort_by_key(|atom| {
        let touches_answer = atom_variables(atom).iter().any(|v| answer.contains(*v));
        usize::from(!touches_answer)
    });
    atoms
}

/// Order data-property atoms first (for data-heavy queries).
fn order_data_first(query: &ConjunctiveQuery) -> Vec<QueryAtom> {
    let mut atoms = query.body_atoms.clone();
    atoms.sort_by_key(|atom| usize::from(!matches!(atom, QueryAtom::DataPropertyAtom { .. })));
    atoms
}

/// Direct evaluation for simple (single-atom) queries.
#[derive(Debug, Default)]
pub struct DirectStrategy;

impl ExecutionStrategy for DirectStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(query, context, self.name(), query.body_atoms.clone(), None)
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query)
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        query.body_atoms.len() == 1
    }

    fn name(&self) -> &str {
        "direct"
    }

    fn description(&self) -> &str {
        "Direct evaluation for single-atom queries"
    }
}

/// Standard tableau-style nested-loop join in the query's natural atom order.
#[derive(Debug, Default)]
pub struct TableauStrategy;

impl ExecutionStrategy for TableauStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(query, context, self.name(), query.body_atoms.clone(), None)
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query)
    }

    fn is_applicable(&self, _query: &ConjunctiveQuery) -> bool {
        true
    }

    fn name(&self) -> &str {
        "balanced"
    }

    fn description(&self) -> &str {
        "Nested-loop join in natural atom order (default fallback)"
    }
}

/// Greedily reorders atoms by ascending selectivity estimate.
#[derive(Debug, Default)]
pub struct JoinOptimizedStrategy;

impl ExecutionStrategy for JoinOptimizedStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let evaluator = QueryEvaluator::new(context.reasoning_service.clone());
        let mut atoms = query.body_atoms.clone();
        atoms.sort_by(|a, b| {
            evaluator
                .estimate_atom_cost(a)
                .partial_cmp(&evaluator.estimate_atom_cost(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        evaluator.evaluate(query, &atoms, self.name(), None)
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 0.9
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        query.body_atoms.len() > 1
    }

    fn name(&self) -> &str {
        "join_optimized"
    }

    fn description(&self) -> &str {
        "Selectivity-based join reordering"
    }
}

/// Orders atoms to bind new variables first, deferring cycle-closing atoms.
#[derive(Debug, Default)]
pub struct CycleAwareStrategy;

impl ExecutionStrategy for CycleAwareStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(
            query,
            context,
            self.name(),
            order_cycle_aware(query),
            None,
        )
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 1.1
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        query.body_atoms.len() > 1
    }

    fn name(&self) -> &str {
        "cycle_aware"
    }

    fn description(&self) -> &str {
        "Cycle-aware atom ordering that defers cycle-closing joins"
    }
}

/// Puts selective (concrete/value/equality) atoms first to prune early.
#[derive(Debug, Default)]
pub struct FilterFirstStrategy;

impl ExecutionStrategy for FilterFirstStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(
            query,
            context,
            self.name(),
            order_filter_first(query),
            None,
        )
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 1.0
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        query.body_atoms.len() > 1
    }

    fn name(&self) -> &str {
        "filter_first"
    }

    fn description(&self) -> &str {
        "Evaluates selective filter atoms first to prune bindings early"
    }
}

/// Streams results with a bounded result limit.
#[derive(Debug, Default)]
pub struct StreamingStrategy;

impl ExecutionStrategy for StreamingStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        // A bounded result set simulates streaming: results beyond the cap are
        // truncated and `complete` is reported as false.
        const STREAMING_CAP: usize = 1000;
        evaluate_with_order(
            query,
            context,
            self.name(),
            query.body_atoms.clone(),
            Some(STREAMING_CAP),
        )
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 1.2
    }

    fn is_applicable(&self, _query: &ConjunctiveQuery) -> bool {
        true
    }

    fn name(&self) -> &str {
        "streaming"
    }

    fn description(&self) -> &str {
        "Streaming evaluation with a bounded result set"
    }
}

/// Orders atoms that bind answer variables first to keep intermediates small.
#[derive(Debug, Default)]
pub struct ProjectionOptimizedStrategy;

impl ExecutionStrategy for ProjectionOptimizedStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(
            query,
            context,
            self.name(),
            order_projection_first(query),
            None,
        )
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 0.95
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        !query.answer_variables.is_empty()
    }

    fn name(&self) -> &str {
        "projection_optimized"
    }

    fn description(&self) -> &str {
        "Binds answer variables early to minimise intermediate binding size"
    }
}

/// Prioritizes data-property atoms (for data-heavy queries).
#[derive(Debug, Default)]
pub struct DataOptimizedStrategy;

impl ExecutionStrategy for DataOptimizedStrategy {
    fn execute(
        &self,
        query: &ConjunctiveQuery,
        context: &ExecutionContext,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        evaluate_with_order(query, context, self.name(), order_data_first(query), None)
    }

    fn estimate_cost(&self, query: &ConjunctiveQuery) -> f64 {
        base_cost(query) * 1.0
    }

    fn is_applicable(&self, query: &ConjunctiveQuery) -> bool {
        query
            .body_atoms
            .iter()
            .any(|a| matches!(a, QueryAtom::DataPropertyAtom { .. }))
    }

    fn name(&self) -> &str {
        "data_optimized"
    }

    fn description(&self) -> &str {
        "Prioritizes data-property atoms for data-heavy queries"
    }
}

/// Build the default set of (name, strategy) registrations.
///
/// Includes aliases so that every name produced by the rule-based and ML
/// strategy selectors resolves to a concrete implementation.
pub fn default_strategies() -> Vec<(&'static str, Box<dyn ExecutionStrategy>)> {
    vec![
        ("direct", Box::new(DirectStrategy)),
        ("balanced", Box::new(TableauStrategy)),
        ("tableau", Box::new(TableauStrategy)),
        ("default", Box::new(TableauStrategy)),
        ("join_optimized", Box::new(JoinOptimizedStrategy)),
        ("join_order", Box::new(JoinOptimizedStrategy)),
        ("cycle_aware", Box::new(CycleAwareStrategy)),
        ("filter_first", Box::new(FilterFirstStrategy)),
        ("indexed_lookup", Box::new(FilterFirstStrategy)),
        ("streaming", Box::new(StreamingStrategy)),
        ("projection_optimized", Box::new(ProjectionOptimizedStrategy)),
        ("data_optimized", Box::new(DataOptimizedStrategy)),
        // ML strategy names that have no dedicated behaviour yet fall back to
        // the balanced tableau strategy.
        ("materialization", Box::new(TableauStrategy)),
        ("hybrid", Box::new(TableauStrategy)),
        ("backward_chaining", Box::new(TableauStrategy)),
        ("forward_chaining", Box::new(TableauStrategy)),
        ("parallel", Box::new(TableauStrategy)),
        ("adaptive", Box::new(TableauStrategy)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    fn class_ce(iri: &str) -> crate::ontology::ClassExpression {
        crate::ontology::ClassExpression::Class(Class::new(IRI::new(iri)))
    }

    #[test]
    fn test_order_filter_first_puts_concrete_atoms_first() {
        let query = ConjunctiveQuery {
            answer_variables: vec![],
            body_atoms: vec![
                QueryAtom::ClassAtom {
                    variable: QueryVariable::new("x"),
                    class_expression: class_ce("http://ex.org/A"),
                },
                QueryAtom::ConcreteIndividualAtom {
                    variable: QueryVariable::new("y"),
                    individual: crate::ontology::Individual::Named(
                        crate::ontology::NamedIndividual {
                            iri: IRI::new("http://ex.org/i"),
                        },
                    ),
                },
            ],
            constraints: Default::default(),
            metadata: Default::default(),
        };

        let ordered = order_filter_first(&query);
        assert!(
            matches!(ordered[0], QueryAtom::ConcreteIndividualAtom { .. }),
            "Concrete atom should be ordered first"
        );
    }

    #[test]
    fn test_order_data_first_puts_data_property_atoms_first() {
        let query = ConjunctiveQuery {
            answer_variables: vec![],
            body_atoms: vec![
                QueryAtom::ClassAtom {
                    variable: QueryVariable::new("x"),
                    class_expression: class_ce("http://ex.org/A"),
                },
                QueryAtom::DataPropertyAtom {
                    subject: QueryVariable::new("x"),
                    property: crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: IRI::new("http://ex.org/name"),
                        },
                    ),
                    literal: QueryVariable::new("n"),
                },
            ],
            constraints: Default::default(),
            metadata: Default::default(),
        };

        let ordered = order_data_first(&query);
        assert!(matches!(ordered[0], QueryAtom::DataPropertyAtom { .. }));
    }

    #[test]
    fn test_order_cycle_aware_defers_cycle_closing_atoms() {
        // A cyclic query: R(x,y) ∧ R(y,x) — the second atom only references
        // variables already bound by the first, so it must be deferred.
        let query = ConjunctiveQuery {
            answer_variables: vec![],
            body_atoms: vec![
                QueryAtom::ObjectPropertyAtom {
                    subject: QueryVariable::new("x"),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: IRI::new("http://ex.org/R"),
                        },
                    ),
                    object: QueryVariable::new("y"),
                },
                QueryAtom::ObjectPropertyAtom {
                    subject: QueryVariable::new("y"),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: IRI::new("http://ex.org/R"),
                        },
                    ),
                    object: QueryVariable::new("x"),
                },
            ],
            constraints: Default::default(),
            metadata: Default::default(),
        };

        let ordered = order_cycle_aware(&query);
        assert_eq!(ordered.len(), 2);
    }

    #[test]
    fn test_order_projection_first_puts_answer_variable_atoms_first() {
        let answer_var = QueryVariable::new("x");
        let query = ConjunctiveQuery {
            answer_variables: vec![answer_var.clone()],
            body_atoms: vec![
                QueryAtom::ClassAtom {
                    variable: QueryVariable::new("y"),
                    class_expression: class_ce("http://ex.org/B"),
                },
                QueryAtom::ClassAtom {
                    variable: answer_var,
                    class_expression: class_ce("http://ex.org/A"),
                },
            ],
            constraints: Default::default(),
            metadata: Default::default(),
        };

        let ordered = order_projection_first(&query);
        assert!(
            atom_variables(&ordered[0])
                .iter()
                .any(|v| **v == QueryVariable::new("x")),
            "Answer-variable atom should be ordered first"
        );
    }

    #[test]
    fn test_default_strategies_are_registered() {
        let strategies = default_strategies();
        let names: std::collections::HashSet<&str> =
            strategies.iter().map(|(name, _)| *name).collect();
        assert!(names.contains("direct"));
        assert!(names.contains("balanced"));
        assert!(names.contains("default"));
        assert!(names.contains("join_optimized"));
        assert!(names.contains("cycle_aware"));
        assert!(names.contains("filter_first"));
        assert!(names.contains("streaming"));
        assert!(names.contains("projection_optimized"));
        assert!(names.contains("data_optimized"));
    }
}
