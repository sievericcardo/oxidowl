//! Shared conjunctive query evaluator.
//!
//! This module is the single source of truth for conjunctive query answering:
//! single-atom evaluation, nested-loop joins, constraint application,
//! projection, deduplication and result limits. It is used both by the legacy
//! [`super::execution::QueryEngine`] and by the concrete
//! [`super::execution_engine::ExecutionStrategy`] implementations, so the
//! evaluation logic is never duplicated.

use super::conjunctive::{
    ConjunctiveQuery, QueryAtom, QueryConstraints, ValueConstraint,
};
use super::execution::{
    AdvancedQueryError, BoundValue, ConjunctiveQueryResult, ExecutionMetadata, MemoryUsage,
    QueryBinding,
};
use crate::reasoning::ReasoningService;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// Evaluates conjunctive queries against the reasoning service.
pub struct QueryEvaluator {
    reasoning_service: Arc<ReasoningService>,
}

impl QueryEvaluator {
    /// Create an evaluator backed by `reasoning_service`.
    #[must_use]
    pub fn new(reasoning_service: Arc<ReasoningService>) -> Self {
        Self { reasoning_service }
    }

    /// Evaluate a single atom in the context of an existing partial binding.
    ///
    /// Returns the set of (possibly empty) extended bindings that satisfy the
    /// atom together with `binding`.
    pub fn evaluate_atom(
        &self,
        atom: &QueryAtom,
        binding: &QueryBinding,
    ) -> Result<Vec<QueryBinding>, AdvancedQueryError> {
        match atom {
            QueryAtom::ClassAtom {
                variable,
                class_expression,
            } => {
                if let Some(bound_value) = binding.get_binding(variable) {
                    if let BoundValue::Individual(individual) = bound_value {
                        if self
                            .reasoning_service
                            .is_instance_of_sync(individual, class_expression)
                            .unwrap_or(false)
                        {
                            Ok(vec![QueryBinding::new()])
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Ok(vec![])
                    }
                } else {
                    let instances = self
                        .reasoning_service
                        .get_instances_sync(class_expression)
                        .unwrap_or_default();
                    Ok(instances
                        .into_iter()
                        .map(|instance| {
                            let mut new_binding = QueryBinding::new();
                            new_binding.bind_variable(
                                variable.clone(),
                                BoundValue::Individual(instance),
                            );
                            new_binding
                        })
                        .collect())
                }
            }
            QueryAtom::ObjectPropertyAtom {
                subject,
                property,
                object,
            } => {
                let assertions = self
                    .reasoning_service
                    .get_object_property_assertions_sync(property)
                    .unwrap_or_default();
                let mut results = Vec::new();

                for (subj, obj) in assertions {
                    let mut compatible = true;

                    if let Some(bound_subj) = binding.get_binding(subject)
                        && let BoundValue::Individual(bound_individual) = bound_subj
                        && bound_individual != &subj
                    {
                        compatible = false;
                    }

                    if let Some(bound_obj) = binding.get_binding(object)
                        && let BoundValue::Individual(bound_individual) = bound_obj
                        && bound_individual != &obj
                    {
                        compatible = false;
                    }

                    if compatible {
                        let mut new_binding = QueryBinding::new();
                        new_binding.bind_variable(subject.clone(), BoundValue::Individual(subj));
                        new_binding.bind_variable(object.clone(), BoundValue::Individual(obj));
                        results.push(new_binding);
                    }
                }

                Ok(results)
            }
            QueryAtom::DataPropertyAtom {
                subject,
                property,
                literal,
            } => {
                let assertions = self
                    .reasoning_service
                    .get_data_property_assertions_sync(property)
                    .unwrap_or_default();
                let mut results = Vec::new();

                for (subj, value) in assertions {
                    let mut compatible = true;

                    if let Some(bound_subj) = binding.get_binding(subject)
                        && let BoundValue::Individual(bound_individual) = bound_subj
                        && bound_individual != &subj
                    {
                        compatible = false;
                    }

                    if let Some(bound_lit) = binding.get_binding(literal)
                        && let BoundValue::Literal(bound_literal) = bound_lit
                        && bound_literal != &value
                    {
                        compatible = false;
                    }

                    if compatible {
                        let mut new_binding = QueryBinding::new();
                        new_binding.bind_variable(subject.clone(), BoundValue::Individual(subj));
                        new_binding.bind_variable(literal.clone(), BoundValue::Literal(value));
                        results.push(new_binding);
                    }
                }

                Ok(results)
            }
            QueryAtom::SameIndividualAtom { left, right } => {
                let left_value = binding.get_binding(left);
                let right_value = binding.get_binding(right);

                match (left_value, right_value) {
                    (Some(BoundValue::Individual(l)), Some(BoundValue::Individual(r))) => {
                        if l == r {
                            Ok(vec![QueryBinding::new()])
                        } else {
                            Ok(vec![])
                        }
                    }
                    (Some(BoundValue::Individual(ind)), None)
                    | (None, Some(BoundValue::Individual(ind))) => {
                        let mut new_binding = QueryBinding::new();
                        let var = if left_value.is_some() { right } else { left };
                        new_binding.bind_variable(var.clone(), BoundValue::Individual(ind.clone()));
                        Ok(vec![new_binding])
                    }
                    (None, None) => Ok(vec![QueryBinding::new()]),
                    _ => Ok(vec![]),
                }
            }
            QueryAtom::DifferentIndividualsAtom { left, right } => {
                let left_value = binding.get_binding(left);
                let right_value = binding.get_binding(right);

                match (left_value, right_value) {
                    (Some(BoundValue::Individual(l)), Some(BoundValue::Individual(r))) => {
                        if l != r {
                            Ok(vec![QueryBinding::new()])
                        } else {
                            Ok(vec![])
                        }
                    }
                    _ => Ok(vec![QueryBinding::new()]),
                }
            }
            QueryAtom::ConcreteIndividualAtom {
                variable,
                individual,
            } => {
                if let Some(bound_value) = binding.get_binding(variable) {
                    if let BoundValue::Individual(bound_ind) = bound_value
                        && bound_ind == individual
                    {
                        Ok(vec![QueryBinding::new()])
                    } else {
                        Ok(vec![])
                    }
                } else {
                    let mut new_binding = QueryBinding::new();
                    new_binding.bind_variable(
                        variable.clone(),
                        BoundValue::Individual(individual.clone()),
                    );
                    Ok(vec![new_binding])
                }
            }
            QueryAtom::ConcreteLiteralAtom { variable, literal } => {
                if let Some(bound_value) = binding.get_binding(variable) {
                    if let BoundValue::Literal(bound_lit) = bound_value
                        && bound_lit == literal
                    {
                        Ok(vec![QueryBinding::new()])
                    } else {
                        Ok(vec![])
                    }
                } else {
                    let mut new_binding = QueryBinding::new();
                    new_binding.bind_variable(variable.clone(), BoundValue::Literal(literal.clone()));
                    Ok(vec![new_binding])
                }
            }
        }
    }

    /// Evaluate a conjunctive query via nested-loop join over `atom_order`.
    pub fn evaluate(
        &self,
        query: &ConjunctiveQuery,
        atom_order: &[QueryAtom],
        strategy_name: &str,
        limit: Option<usize>,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let mut reasoning_calls = 0usize;
        let mut current_bindings = vec![QueryBinding::new()];

        for atom in atom_order {
            let mut next_bindings = Vec::new();

            for current_binding in &current_bindings {
                let atom_bindings = self.evaluate_atom(atom, current_binding)?;
                reasoning_calls += atom_bindings.len();

                for atom_binding in atom_bindings {
                    if let Some(combined_binding) = current_binding.combine(&atom_binding) {
                        next_bindings.push(combined_binding);
                    }
                }
            }

            current_bindings = next_bindings;
            if current_bindings.is_empty() {
                break;
            }
        }

        // Project to answer variables only.
        let mut bindings: Vec<QueryBinding> = current_bindings
            .into_iter()
            .map(|binding| binding.project(&query.answer_variables))
            .collect();

        bindings = self.apply_constraints(&bindings, &query.constraints);
        bindings.dedup();

        let complete = match limit {
            Some(limit) if bindings.len() > limit => {
                bindings.truncate(limit);
                false
            }
            _ => true,
        };

        Ok(ConjunctiveQueryResult {
            bindings,
            metadata: ExecutionMetadata {
                execution_time: Duration::from_millis(0),
                optimization_time: Duration::from_millis(0),
                strategy_used: strategy_name.to_string(),
                intermediate_results: reasoning_calls,
                cache_hit: false,
                reasoning_calls,
                memory_usage: MemoryUsage::default(),
            },
            complete,
        })
    }

    /// Estimate the relative cost/selectivity of an atom (lower = more selective).
    ///
    /// This is a heuristic proxy: concrete/value and equality atoms are
    /// considered most selective, followed by class atoms, then property atoms.
    #[must_use]
    pub fn estimate_atom_cost(&self, atom: &QueryAtom) -> f64 {
        match atom {
            QueryAtom::ConcreteIndividualAtom { .. } | QueryAtom::ConcreteLiteralAtom { .. } => 0.0,
            QueryAtom::SameIndividualAtom { .. } | QueryAtom::DifferentIndividualsAtom { .. } => 0.5,
            QueryAtom::ClassAtom { .. } => 1.0,
            QueryAtom::ObjectPropertyAtom { .. } | QueryAtom::DataPropertyAtom { .. } => 2.0,
        }
    }

    fn apply_constraints(
        &self,
        bindings: &[QueryBinding],
        constraints: &QueryConstraints,
    ) -> Vec<QueryBinding> {
        let mut filtered = bindings.to_vec();

        // DISTINCT constraints — remove duplicate bindings over distinct sets.
        for distinct_set in &constraints.distinct_variables {
            let mut seen = HashSet::new();
            filtered.retain(|binding| {
                let sig: Vec<String> = distinct_set
                    .iter()
                    .map(|var| format!("{:?}", binding.get_binding(var)))
                    .collect();
                seen.insert(sig)
            });
        }

        // Type constraints.
        for (variable, required_types) in &constraints.type_constraints {
            filtered.retain(|binding| {
                if let Some(bound_value) = binding.get_binding(variable) {
                    match bound_value {
                        BoundValue::Individual(_) => !required_types.is_empty(),
                        BoundValue::Literal(_) => false,
                        BoundValue::Class(_) => true,
                        BoundValue::Property(_) => false,
                    }
                } else {
                    true
                }
            });
        }

        // Value constraints.
        for (variable, required_value) in &constraints.value_constraints {
            filtered.retain(|binding| {
                if let Some(bound_value) = binding.get_binding(variable) {
                    match (bound_value, required_value) {
                        (BoundValue::Literal(lit), ValueConstraint::ExactValue(required_lit)) => {
                            lit == required_lit
                        }
                        (BoundValue::Literal(lit), ValueConstraint::ValueSet(allowed)) => {
                            allowed.contains(lit)
                        }
                        (BoundValue::Literal(_), ValueConstraint::StringPattern(_)) => true,
                        _ => true,
                    }
                } else {
                    true
                }
            });
        }

        filtered
    }
}
