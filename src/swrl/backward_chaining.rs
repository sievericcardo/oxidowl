//! Backward Chaining Implementation for SWRL

#![allow(dead_code)]

use crate::ontology::{
    ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
};
use crate::swrl::{SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule, SWRLValue, SWRLVariable};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::fmt;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Backward chaining engine for goal-driven reasoning
pub struct BackwardChainingEngine {
    /// Available SWRL rules
    rules: Vec<SWRLRule>,
    /// Ground facts (from forward chaining or asserted)
    fact_base: FactBase,
    /// Query processing stack
    query_stack: QueryStack,
    /// Maximum recursion depth to prevent infinite loops
    max_depth: usize,
    /// Cache for resolved queries
    query_cache: HashMap<SWRLAtom, Vec<VariableBindings>>,
}

/// Represents a query goal to be resolved
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The goal atom to prove
    goal: SWRLAtom,
    /// Current variable bindings
    bindings: VariableBindings,
    /// Depth in the resolution tree
    depth: usize,
}

/// Stack for managing query resolution with cycle detection
#[derive(Debug)]
pub struct QueryStack {
    /// Stack of active queries
    stack: Vec<Query>,
    /// Set of visited queries for cycle detection
    visited: HashSet<SWRLAtom>,
    /// Current depth
    current_depth: usize,
}

impl Default for QueryStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable bindings for unification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableBindings {
    /// Map from variables to their bound values
    bindings: HashMap<SWRLVariable, SWRLTerm>,
}

/// SWRL term for unification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SWRLTerm {
    Variable(SWRLVariable),
    Individual(Individual),
    Literal(SWRLValue),
    Class(ClassExpression),
    ObjectProperty(ObjectPropertyExpression),
    DataProperty(DataPropertyExpression),
}

/// Fact base storing ground facts
#[derive(Debug, Clone)]
pub struct FactBase {
    /// Ground class assertions: Class(individual)
    class_assertions: HashSet<(ClassExpression, Individual)>,
    /// Ground object property assertions: Property(subject, object)
    object_property_assertions: HashSet<(ObjectPropertyExpression, Individual, Individual)>,
    /// Ground data property assertions: Property(subject, value)
    data_property_assertions: HashSet<(DataPropertyExpression, Individual, SWRLValue)>,
    /// Ground same individual assertions
    same_individual_assertions: HashSet<(Individual, Individual)>,
    /// Ground different individual assertions
    different_individual_assertions: HashSet<(Individual, Individual)>,
}

/// Result of backward chaining query
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Whether the query succeeded
    pub success: bool,
    /// All variable bindings that satisfy the query
    pub solutions: Vec<VariableBindings>,
    /// Proof tree (optional)
    pub proof: Option<ProofTree>,
}

/// Proof tree for explanation
#[derive(Debug, Clone)]
pub struct ProofTree {
    /// The proven goal
    pub goal: SWRLAtom,
    /// The rule used (if any)
    pub rule: Option<SWRLRule>,
    /// Sub-proofs for rule body
    pub sub_proofs: Vec<ProofTree>,
    /// Whether this was proven from facts
    pub from_facts: bool,
}

// =============================================================================
// IMPLEMENTATION
// =============================================================================

impl BackwardChainingEngine {
    /// Create a new backward chaining engine
    #[must_use]
    pub fn new(rules: Vec<SWRLRule>, max_depth: usize) -> Self {
        Self {
            rules,
            fact_base: FactBase::new(),
            query_stack: QueryStack::new(),
            max_depth,
            query_cache: HashMap::new(),
        }
    }

    /// Add ground facts to the fact base
    pub fn add_facts(&mut self, facts: FactBase) {
        self.fact_base.merge(facts);
    }

    /// Query for a goal with optional variable bindings
    pub fn query(&mut self, goal: SWRLAtom) -> Result<QueryResult> {
        self.query_with_bindings(goal, VariableBindings::new())
    }

    /// Query for a goal with initial variable bindings
    pub fn query_with_bindings(
        &mut self,
        goal: SWRLAtom,
        initial_bindings: VariableBindings,
    ) -> Result<QueryResult> {
        // Clear the query stack
        self.query_stack.clear();

        // Check cache first
        if let Some(cached_solutions) = self.query_cache.get(&goal) {
            return Ok(QueryResult {
                success: !cached_solutions.is_empty(),
                solutions: cached_solutions.clone(),
                proof: None, // Cached results don't include proof
            });
        }

        let mut solutions = Vec::new();
        let mut proof_trees = Vec::new();

        // Attempt to resolve the goal
        self.resolve_goal(
            &goal,
            &initial_bindings,
            &mut solutions,
            &mut proof_trees,
            0,
        )?;

        // Cache the result
        self.query_cache.insert(goal.clone(), solutions.clone());

        let result = QueryResult {
            success: !solutions.is_empty(),
            solutions,
            proof: proof_trees.into_iter().next(), // Return first proof tree
        };

        Ok(result)
    }

    /// Resolve a goal with given bindings
    fn resolve_goal(
        &mut self,
        goal: &SWRLAtom,
        bindings: &VariableBindings,
        solutions: &mut Vec<VariableBindings>,
        proof_trees: &mut Vec<ProofTree>,
        depth: usize,
    ) -> Result<()> {
        // Check depth limit
        if depth > self.max_depth {
            return Err(Error::reasoning("Maximum recursion depth exceeded"));
        }

        // Check for cycles
        if self.query_stack.contains_goal(goal) {
            return Ok(()); // Skip cyclic goals
        }

        // Push goal onto stack
        let query = Query {
            goal: goal.clone(),
            bindings: bindings.clone(),
            depth,
        };
        self.query_stack.push(query)?;

        // Apply current bindings to the goal
        let instantiated_goal = self.apply_bindings_to_atom(goal, bindings);

        // First, try to resolve from facts
        if self.resolve_from_facts(&instantiated_goal, bindings, solutions, proof_trees) {
            self.query_stack.pop();
            return Ok(());
        }

        // Then, try to resolve using rules.
        // We clone each rule individually to release the immutable borrow before
        // the mutable `prove_rule_body` call — cheaper than cloning the whole Vec.
        for i in 0..self.rules.len() {
            let rule = self.rules[i].clone();
            if let Some(head_atom) = rule.head.first()
                && let Some(unifier) = self.unify_atoms(&instantiated_goal, head_atom, bindings)
            {
                // Try to prove the rule body
                if self.prove_rule_body(&rule.body, &unifier, solutions, proof_trees, depth + 1)? {
                    // Create proof tree
                    let proof = ProofTree {
                        goal: goal.clone(),
                        rule: Some(rule),
                        sub_proofs: Vec::new(), // Would need to collect from body proof
                        from_facts: false,
                    };
                    proof_trees.push(proof);
                }
            }
        }

        self.query_stack.pop();
        Ok(())
    }

    /// Resolve goal from ground facts
    fn resolve_from_facts(
        &self,
        goal: &SWRLAtom,
        bindings: &VariableBindings,
        solutions: &mut Vec<VariableBindings>,
        proof_trees: &mut Vec<ProofTree>,
    ) -> bool {
        match goal {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => {
                if let SWRLTerm::Individual(individual) = self.swrl_argument_to_term(argument)
                    && self.fact_base.has_class_assertion(predicate, &individual)
                {
                    solutions.push(bindings.clone());
                    proof_trees.push(ProofTree {
                        goal: goal.clone(),
                        rule: None,
                        sub_proofs: Vec::new(),
                        from_facts: true,
                    });
                    return true;
                }
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                if let (SWRLTerm::Individual(subj), SWRLTerm::Individual(obj)) = (
                    self.swrl_argument_to_term(first_argument),
                    self.swrl_argument_to_term(second_argument),
                ) && self
                    .fact_base
                    .has_object_property_assertion(predicate, &subj, &obj)
                {
                    solutions.push(bindings.clone());
                    proof_trees.push(ProofTree {
                        goal: goal.clone(),
                        rule: None,
                        sub_proofs: Vec::new(),
                        from_facts: true,
                    });
                    return true;
                }
            }
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                if let SWRLTerm::Individual(subj) = self.swrl_argument_to_term(first_argument) {
                    // For data properties, second argument could be a literal
                    if let SWRLTerm::Literal(value) =
                        self.swrl_data_argument_to_term(second_argument)
                        && self
                            .fact_base
                            .has_data_property_assertion(predicate, &subj, &value)
                    {
                        solutions.push(bindings.clone());
                        proof_trees.push(ProofTree {
                            goal: goal.clone(),
                            rule: None,
                            sub_proofs: Vec::new(),
                            from_facts: true,
                        });
                        return true;
                    }
                }
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => {
                if let (SWRLTerm::Individual(ind1), SWRLTerm::Individual(ind2)) = (
                    self.swrl_argument_to_term(first_argument),
                    self.swrl_argument_to_term(second_argument),
                ) && self.fact_base.has_same_individual_assertion(&ind1, &ind2)
                {
                    solutions.push(bindings.clone());
                    proof_trees.push(ProofTree {
                        goal: goal.clone(),
                        rule: None,
                        sub_proofs: Vec::new(),
                        from_facts: true,
                    });
                    return true;
                }
            }
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                if let (SWRLTerm::Individual(ind1), SWRLTerm::Individual(ind2)) = (
                    self.swrl_argument_to_term(first_argument),
                    self.swrl_argument_to_term(second_argument),
                ) && self
                    .fact_base
                    .has_different_individual_assertion(&ind1, &ind2)
                {
                    solutions.push(bindings.clone());
                    proof_trees.push(ProofTree {
                        goal: goal.clone(),
                        rule: None,
                        sub_proofs: Vec::new(),
                        from_facts: true,
                    });
                    return true;
                }
            }
            SWRLAtom::BuiltInAtom {
                predicate: _,
                arguments: _,
            } => {
                // Built-ins are evaluated directly, not resolved from facts
                return false;
            }
            SWRLAtom::DataRangeAtom {
                predicate: _,
                argument,
            } => {
                if let SWRLDArgument::Literal(lit) = argument.clone() {
                    if lit.value.parse::<f64>().is_ok() {
                        return true;
                    }
                }
                return false;
            }
        }

        false
    }

    /// Prove all atoms in a rule body
    fn prove_rule_body(
        &mut self,
        body: &[SWRLAtom],
        bindings: &VariableBindings,
        solutions: &mut Vec<VariableBindings>,
        _proof_trees: &mut Vec<ProofTree>,
        depth: usize,
    ) -> Result<bool> {
        if body.is_empty() {
            solutions.push(bindings.clone());
            return Ok(true);
        }

        let mut current_solutions = vec![bindings.clone()];

        for atom in body {
            let mut next_solutions = Vec::new();

            for solution in current_solutions {
                let mut atom_solutions = Vec::new();
                let mut atom_proofs = Vec::new();

                self.resolve_goal(
                    atom,
                    &solution,
                    &mut atom_solutions,
                    &mut atom_proofs,
                    depth,
                )?;

                next_solutions.extend(atom_solutions);
            }

            current_solutions = next_solutions;

            if current_solutions.is_empty() {
                return Ok(false); // Body failed
            }
        }

        solutions.extend(current_solutions);
        Ok(true)
    }

    /// Unify two atoms
    fn unify_atoms(
        &self,
        goal: &SWRLAtom,
        head: &SWRLAtom,
        bindings: &VariableBindings,
    ) -> Option<VariableBindings> {
        // Proper unification implementation
        match (goal, head) {
            (
                SWRLAtom::ClassAtom {
                    predicate: p1,
                    argument: a1,
                },
                SWRLAtom::ClassAtom {
                    predicate: p2,
                    argument: a2,
                },
            ) => {
                // Unify predicates first
                if !self.unify_class_expressions(p1, p2)? {
                    return None;
                }

                // Then unify arguments
                self.unify_i_arguments(a1, a2, bindings)
            }
            (
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p1,
                    first_argument: fa1,
                    second_argument: sa1,
                },
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p2,
                    first_argument: fa2,
                    second_argument: sa2,
                },
            ) => {
                // Unify predicates first
                if !self.unify_object_property_expressions(p1, p2)? {
                    return None;
                }

                // Unify first arguments
                let mut result_bindings = self.unify_i_arguments(fa1, fa2, bindings)?;

                // Unify second arguments
                result_bindings = self.unify_i_arguments(sa1, sa2, &result_bindings)?;

                Some(result_bindings)
            }
            (
                SWRLAtom::DataPropertyAtom {
                    predicate: p1,
                    first_argument: fa1,
                    second_argument: sa1,
                },
                SWRLAtom::DataPropertyAtom {
                    predicate: p2,
                    first_argument: fa2,
                    second_argument: sa2,
                },
            ) => {
                // Unify predicates first
                if !self.unify_data_property_expressions(p1, p2)? {
                    return None;
                }

                // Unify first arguments (individuals)
                let mut result_bindings = self.unify_i_arguments(fa1, fa2, bindings)?;

                // Unify second arguments (data values)
                result_bindings = self.unify_d_arguments(sa1, sa2, &result_bindings)?;

                Some(result_bindings)
            }
            _ => None, // Different atom types don't unify
        }
    }

    /// Unify class expressions
    fn unify_class_expressions(
        &self,
        expr1: &ClassExpression,
        expr2: &ClassExpression,
    ) -> Option<bool> {
        match (expr1, expr2) {
            (ClassExpression::Class(c1), ClassExpression::Class(c2)) => Some(c1.iri == c2.iri),
            // For more complex expressions, would need recursive unification
            _ => Some(false), // Simplified for now
        }
    }

    /// Unify object property expressions
    fn unify_object_property_expressions(
        &self,
        expr1: &ObjectPropertyExpression,
        expr2: &ObjectPropertyExpression,
    ) -> Option<bool> {
        match (expr1, expr2) {
            (
                ObjectPropertyExpression::ObjectProperty(p1),
                ObjectPropertyExpression::ObjectProperty(p2),
            ) => Some(p1.iri == p2.iri),
            _ => Some(false), // Simplified for complex expressions
        }
    }

    /// Unify data property expressions
    fn unify_data_property_expressions(
        &self,
        expr1: &DataPropertyExpression,
        expr2: &DataPropertyExpression,
    ) -> Option<bool> {
        match (expr1, expr2) {
            (
                DataPropertyExpression::DataProperty(p1),
                DataPropertyExpression::DataProperty(p2),
            ) => Some(p1.iri == p2.iri),
        }
    }

    /// Unify individual arguments
    fn unify_i_arguments(
        &self,
        arg1: &SWRLIArgument,
        arg2: &SWRLIArgument,
        bindings: &VariableBindings,
    ) -> Option<VariableBindings> {
        match (arg1, arg2) {
            (SWRLIArgument::Variable(var1), SWRLIArgument::Variable(var2)) => {
                // Variable-variable unification
                let mut new_bindings = bindings.clone();

                // Check if either variable is already bound
                let val1 = bindings.bindings.get(var1);
                let val2 = bindings.bindings.get(var2);

                match (val1, val2) {
                    (Some(v1), Some(v2)) => {
                        if v1 == v2 {
                            Some(new_bindings)
                        } else {
                            None
                        }
                    }
                    (Some(v1), None) => {
                        new_bindings.bindings.insert(var2.clone(), v1.clone());
                        Some(new_bindings)
                    }
                    (None, Some(v2)) => {
                        new_bindings.bindings.insert(var1.clone(), v2.clone());
                        Some(new_bindings)
                    }
                    (None, None) => {
                        // Bind one variable to the other
                        new_bindings
                            .bindings
                            .insert(var1.clone(), SWRLTerm::Variable(var2.clone()));
                        Some(new_bindings)
                    }
                }
            }
            (SWRLIArgument::Variable(var), SWRLIArgument::Individual(ind)) => {
                // Variable-individual unification
                let mut new_bindings = bindings.clone();
                let ind_name = match ind {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => format!("_:{}", anon.id),
                };

                if let Some(existing_val) = bindings.bindings.get(var) {
                    if let SWRLTerm::Individual(existing_ind) = existing_val {
                        if existing_ind.iri().map(super::super::ontology::IRI::as_str)
                            == Some(&ind_name)
                        {
                            Some(new_bindings)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    let individual = Individual::Named(crate::ontology::NamedIndividual::new(
                        crate::ontology::IRI::new(&ind_name),
                    ));
                    new_bindings
                        .bindings
                        .insert(var.clone(), SWRLTerm::Individual(individual));
                    Some(new_bindings)
                }
            }
            (SWRLIArgument::Individual(ind), SWRLIArgument::Variable(var)) => {
                // Individual-variable unification (symmetric)
                self.unify_i_arguments(
                    &SWRLIArgument::Variable(var.clone()),
                    &SWRLIArgument::Individual(ind.clone()),
                    bindings,
                )
            }
            (SWRLIArgument::Individual(ind1), SWRLIArgument::Individual(ind2)) => {
                // Individual-individual unification
                let ind1_name = match ind1 {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => format!("_:{}", anon.id),
                };
                let ind2_name = match ind2 {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => format!("_:{}", anon.id),
                };

                if ind1_name == ind2_name {
                    Some(bindings.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Unify data arguments  
    fn unify_d_arguments(
        &self,
        arg1: &SWRLDArgument,
        arg2: &SWRLDArgument,
        bindings: &VariableBindings,
    ) -> Option<VariableBindings> {
        match (arg1, arg2) {
            (SWRLDArgument::Variable(var1), SWRLDArgument::Variable(var2)) => {
                // Variable-variable unification
                let mut new_bindings = bindings.clone();

                let val1 = bindings.bindings.get(var1);
                let val2 = bindings.bindings.get(var2);

                match (val1, val2) {
                    (Some(v1), Some(v2)) => {
                        if v1 == v2 {
                            Some(new_bindings)
                        } else {
                            None
                        }
                    }
                    (Some(v1), None) => {
                        new_bindings.bindings.insert(var2.clone(), v1.clone());
                        Some(new_bindings)
                    }
                    (None, Some(v2)) => {
                        new_bindings.bindings.insert(var1.clone(), v2.clone());
                        Some(new_bindings)
                    }
                    (None, None) => {
                        // Bind one variable to the other
                        new_bindings
                            .bindings
                            .insert(var1.clone(), SWRLTerm::Variable(var2.clone()));
                        Some(new_bindings)
                    }
                }
            }
            (SWRLDArgument::Variable(var), SWRLDArgument::Literal(lit)) => {
                // Variable-literal unification
                let mut new_bindings = bindings.clone();
                let lit_value = lit.to_string();

                if let Some(existing_val) = bindings.bindings.get(var) {
                    if let SWRLTerm::Literal(existing_lit) = existing_val {
                        if existing_lit.to_string() == lit_value {
                            Some(new_bindings)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    new_bindings
                        .bindings
                        .insert(var.clone(), SWRLTerm::Literal(SWRLValue::String(lit_value)));
                    Some(new_bindings)
                }
            }
            (SWRLDArgument::Literal(lit), SWRLDArgument::Variable(var)) => {
                // Literal-variable unification (symmetric)
                self.unify_d_arguments(
                    &SWRLDArgument::Variable(var.clone()),
                    &SWRLDArgument::Literal(lit.clone()),
                    bindings,
                )
            }
            (SWRLDArgument::Literal(lit1), SWRLDArgument::Literal(lit2)) => {
                // Literal-literal unification
                if lit1.to_string() == lit2.to_string() {
                    Some(bindings.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Unify two SWRL arguments
    fn unify_arguments(
        &self,
        arg1: &crate::swrl::SWRLIArgument,
        arg2: &crate::swrl::SWRLIArgument,
        bindings: &VariableBindings,
    ) -> Option<VariableBindings> {
        // Use the proper unification implementation
        self.unify_i_arguments(arg1, arg2, bindings)
    }

    /// Apply variable bindings to an atom
    fn apply_bindings_to_atom(&self, atom: &SWRLAtom, bindings: &VariableBindings) -> SWRLAtom {
        match atom {
            SWRLAtom::ClassAtom {
                predicate: class,
                argument,
            } => {
                let new_argument = self.apply_bindings_to_argument(argument, bindings);
                SWRLAtom::ClassAtom {
                    predicate: class.clone(),
                    argument: new_argument,
                }
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate: property,
                first_argument: subject,
                second_argument: object,
            } => {
                let new_subject = self.apply_bindings_to_argument(subject, bindings);
                let new_object = self.apply_bindings_to_argument(object, bindings);
                SWRLAtom::ObjectPropertyAtom {
                    predicate: property.clone(),
                    first_argument: new_subject,
                    second_argument: new_object,
                }
            }
            SWRLAtom::DataPropertyAtom {
                predicate: property,
                first_argument: subject,
                second_argument: object,
            } => {
                let new_subject = self.apply_bindings_to_argument(subject, bindings);
                let new_object = self.apply_bindings_to_dargument(object, bindings);
                SWRLAtom::DataPropertyAtom {
                    predicate: property.clone(),
                    first_argument: new_subject,
                    second_argument: new_object,
                }
            }
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => {
                let new_arguments = arguments
                    .iter()
                    .map(|arg| self.apply_bindings_to_dargument(arg, bindings))
                    .collect();
                SWRLAtom::BuiltInAtom {
                    predicate: predicate.clone(),
                    arguments: new_arguments,
                }
            }
            SWRLAtom::SameIndividualAtom {
                first_argument: left,
                second_argument: right,
            } => {
                let new_left = self.apply_bindings_to_argument(left, bindings);
                let new_right = self.apply_bindings_to_argument(right, bindings);
                SWRLAtom::SameIndividualAtom {
                    first_argument: new_left,
                    second_argument: new_right,
                }
            }
            SWRLAtom::DifferentIndividualsAtom {
                first_argument: left,
                second_argument: right,
            } => {
                let new_left = self.apply_bindings_to_argument(left, bindings);
                let new_right = self.apply_bindings_to_argument(right, bindings);
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: new_left,
                    second_argument: new_right,
                }
            }
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => {
                let new_argument = self.apply_bindings_to_dargument(argument, bindings);
                SWRLAtom::DataRangeAtom {
                    predicate: predicate.clone(),
                    argument: new_argument,
                }
            }
        }
    }

    /// Apply bindings to an individual argument
    fn apply_bindings_to_argument(
        &self,
        arg: &crate::swrl::SWRLIArgument,
        bindings: &VariableBindings,
    ) -> crate::swrl::SWRLIArgument {
        match arg {
            crate::swrl::SWRLIArgument::Variable(var) => {
                if let Some(binding) = bindings.bindings.get(var) {
                    match binding {
                        SWRLTerm::Individual(individual) => {
                            crate::swrl::SWRLIArgument::Individual(individual.clone())
                        }
                        _ => arg.clone(),
                    }
                } else {
                    arg.clone()
                }
            }
            _ => arg.clone(),
        }
    }

    /// Apply bindings to a data argument
    fn apply_bindings_to_dargument(
        &self,
        arg: &crate::swrl::SWRLDArgument,
        _bindings: &VariableBindings,
    ) -> crate::swrl::SWRLDArgument {
        match arg {
            crate::swrl::SWRLDArgument::Variable(_var) => {
                // For simplicity, assume data variables are not bound in this context
                arg.clone()
            }
            _ => arg.clone(),
        }
    }

    /// Convert SWRL argument to term
    fn swrl_argument_to_term(&self, arg: &crate::swrl::SWRLIArgument) -> SWRLTerm {
        match arg {
            crate::swrl::SWRLIArgument::Variable(var) => SWRLTerm::Variable(var.clone()),
            crate::swrl::SWRLIArgument::Individual(ind) => SWRLTerm::Individual(ind.clone()),
        }
    }

    fn swrl_data_argument_to_term(&self, arg: &crate::swrl::SWRLDArgument) -> SWRLTerm {
        match arg {
            crate::swrl::SWRLDArgument::Variable(var) => SWRLTerm::Variable(var.clone()),
            crate::swrl::SWRLDArgument::Literal(lit) => {
                SWRLTerm::Literal(SWRLValue::Literal(lit.clone()))
            }
        }
    }

    /// Clear query cache
    pub fn clear_cache(&mut self) {
        self.query_cache.clear();
    }

    /// Get statistics about the engine
    #[must_use]
    pub fn get_statistics(&self) -> BackwardChainingStatistics {
        BackwardChainingStatistics {
            rules_count: self.rules.len(),
            facts_count: self.fact_base.total_facts(),
            cache_size: self.query_cache.len(),
            max_depth: self.max_depth,
        }
    }
}

// =============================================================================
// SUPPORTING IMPLEMENTATIONS
// =============================================================================

impl VariableBindings {
    /// Create new empty bindings
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Bind a variable to a term
    pub fn bind(&mut self, var: SWRLVariable, term: SWRLTerm) -> Result<()> {
        // Check for occurs check
        if self.occurs_check(&var, &term) {
            return Err(Error::reasoning("Occurs check failure"));
        }

        self.bindings.insert(var, term);
        Ok(())
    }

    /// Lookup binding for a variable
    #[must_use]
    pub fn lookup(&self, var: &SWRLVariable) -> Option<&SWRLTerm> {
        self.bindings.get(var)
    }

    /// Check if a variable occurs in a term (prevents infinite structures)
    fn occurs_check(&self, var: &SWRLVariable, term: &SWRLTerm) -> bool {
        match term {
            SWRLTerm::Variable(v) => v == var,
            SWRLTerm::Individual(_) => false,
            SWRLTerm::Literal(_) => false,
            SWRLTerm::Class(_) => false,
            SWRLTerm::ObjectProperty(_) => false,
            SWRLTerm::DataProperty(_) => false,
        }
    }

    /// Merge with another binding set
    pub fn merge(&mut self, other: &VariableBindings) -> Result<()> {
        for (var, term) in &other.bindings {
            if let Some(existing) = self.bindings.get(var) {
                if existing != term {
                    return Err(Error::reasoning("Conflicting variable bindings"));
                }
            } else {
                self.bind(var.clone(), term.clone())?;
            }
        }
        Ok(())
    }
}

impl QueryStack {
    /// Create new query stack
    #[must_use]
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            visited: HashSet::new(),
            current_depth: 0,
        }
    }

    /// Push query onto stack
    pub fn push(&mut self, query: Query) -> Result<()> {
        if self.visited.contains(&query.goal) {
            return Err(Error::reasoning("Cycle detected in query resolution"));
        }

        self.visited.insert(query.goal.clone());
        self.stack.push(query);
        self.current_depth += 1;
        Ok(())
    }

    /// Pop query from stack
    pub fn pop(&mut self) {
        if let Some(query) = self.stack.pop() {
            self.visited.remove(&query.goal);
            self.current_depth -= 1;
        }
    }

    /// Check if goal is on the stack
    #[must_use]
    pub fn contains_goal(&self, goal: &SWRLAtom) -> bool {
        self.visited.contains(goal)
    }

    /// Clear the stack
    pub fn clear(&mut self) {
        self.stack.clear();
        self.visited.clear();
        self.current_depth = 0;
    }
}

impl FactBase {
    /// Create new empty fact base
    #[must_use]
    pub fn new() -> Self {
        Self {
            class_assertions: HashSet::new(),
            object_property_assertions: HashSet::new(),
            data_property_assertions: HashSet::new(),
            same_individual_assertions: HashSet::new(),
            different_individual_assertions: HashSet::new(),
        }
    }

    /// Add class assertion
    pub fn add_class_assertion(&mut self, class: ClassExpression, individual: Individual) {
        self.class_assertions.insert((class, individual));
    }

    /// Check if class assertion exists
    #[must_use]
    pub fn has_class_assertion(&self, class: &ClassExpression, individual: &Individual) -> bool {
        self.class_assertions
            .contains(&(class.clone(), individual.clone()))
    }

    /// Add object property assertion
    pub fn add_object_property_assertion(
        &mut self,
        property: ObjectPropertyExpression,
        subject: Individual,
        object: Individual,
    ) {
        self.object_property_assertions
            .insert((property, subject, object));
    }

    /// Check if object property assertion exists
    #[must_use]
    pub fn has_object_property_assertion(
        &self,
        property: &ObjectPropertyExpression,
        subject: &Individual,
        object: &Individual,
    ) -> bool {
        self.object_property_assertions.contains(&(
            property.clone(),
            subject.clone(),
            object.clone(),
        ))
    }

    /// Add data property assertion
    pub fn add_data_property_assertion(
        &mut self,
        property: DataPropertyExpression,
        subject: Individual,
        value: SWRLValue,
    ) {
        self.data_property_assertions
            .insert((property, subject, value));
    }

    /// Check if data property assertion exists
    #[must_use]
    pub fn has_data_property_assertion(
        &self,
        property: &DataPropertyExpression,
        subject: &Individual,
        value: &SWRLValue,
    ) -> bool {
        self.data_property_assertions
            .contains(&(property.clone(), subject.clone(), value.clone()))
    }

    /// Add same individual assertion
    pub fn add_same_individual_assertion(&mut self, ind1: Individual, ind2: Individual) {
        self.same_individual_assertions
            .insert((ind1.clone(), ind2.clone()));
        self.same_individual_assertions.insert((ind2, ind1)); // Symmetric
    }

    /// Check if same individual assertion exists
    #[must_use]
    pub fn has_same_individual_assertion(&self, ind1: &Individual, ind2: &Individual) -> bool {
        self.same_individual_assertions
            .contains(&(ind1.clone(), ind2.clone()))
    }

    /// Add different individual assertion
    pub fn add_different_individual_assertion(&mut self, ind1: Individual, ind2: Individual) {
        self.different_individual_assertions
            .insert((ind1.clone(), ind2.clone()));
        self.different_individual_assertions.insert((ind2, ind1)); // Symmetric
    }

    /// Check if different individual assertion exists
    #[must_use]
    pub fn has_different_individual_assertion(&self, ind1: &Individual, ind2: &Individual) -> bool {
        self.different_individual_assertions
            .contains(&(ind1.clone(), ind2.clone()))
    }

    /// Merge with another fact base
    pub fn merge(&mut self, other: FactBase) {
        self.class_assertions.extend(other.class_assertions);
        self.object_property_assertions
            .extend(other.object_property_assertions);
        self.data_property_assertions
            .extend(other.data_property_assertions);
        self.same_individual_assertions
            .extend(other.same_individual_assertions);
        self.different_individual_assertions
            .extend(other.different_individual_assertions);
    }

    /// Get total number of facts
    #[must_use]
    pub fn total_facts(&self) -> usize {
        self.class_assertions.len()
            + self.object_property_assertions.len()
            + self.data_property_assertions.len()
            + self.same_individual_assertions.len()
            + self.different_individual_assertions.len()
    }
}

/// Statistics about backward chaining engine
#[derive(Debug, Clone)]
pub struct BackwardChainingStatistics {
    pub rules_count: usize,
    pub facts_count: usize,
    pub cache_size: usize,
    pub max_depth: usize,
}

impl Default for VariableBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FactBase {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueryResult {{ success: {}, solutions: {} }}",
            self.success,
            self.solutions.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_backward_chaining_engine_creation() {
        let engine = BackwardChainingEngine::new(Vec::new(), 10);
        let stats = engine.get_statistics();

        assert_eq!(stats.rules_count, 0);
        assert_eq!(stats.facts_count, 0);
        assert_eq!(stats.max_depth, 10);
    }

    #[test]
    fn test_fact_base_operations() {
        let mut fact_base = FactBase::new();

        let person_class =
            ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let john = Individual::named(IRI::new("http://example.org/john"));

        // Add and check class assertion
        fact_base.add_class_assertion(person_class.clone(), john.clone());
        assert!(fact_base.has_class_assertion(&person_class, &john));

        // Check total facts
        assert_eq!(fact_base.total_facts(), 1);
    }

    #[test]
    fn test_variable_bindings() {
        let mut bindings = VariableBindings::new();
        let var = SWRLVariable::new(IRI::new("http://example.org/var1"));
        let term = SWRLTerm::Individual(Individual::named(IRI::new("http://example.org/john")));

        // Test binding
        assert!(bindings.bind(var.clone(), term.clone()).is_ok());
        assert_eq!(bindings.lookup(&var), Some(&term));
    }

    #[test]
    fn test_query_stack() {
        let mut stack = QueryStack::new();

        let goal = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: crate::swrl::SWRLIArgument::Individual(Individual::named(IRI::new(
                "http://example.org/john",
            ))),
        };

        let query = Query {
            goal: goal.clone(),
            bindings: VariableBindings::new(),
            depth: 0,
        };

        // Test push and contains
        assert!(stack.push(query).is_ok());
        assert!(stack.contains_goal(&goal));

        // Test pop
        stack.pop();
        assert!(!stack.contains_goal(&goal));
    }
}
