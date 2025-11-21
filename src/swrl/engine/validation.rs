//! SWRL Rule Validation and Goal Satisfaction
//!
//! This module implements validation logic for SWRL rules and goal satisfaction checking.

use crate::ontology::{Axiom, ClassExpression, Individual, ObjectPropertyExpression};
use crate::swrl::{SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule};
use crate::{Error, Result};
use crate::core::lock_helpers::{read_lock, write_lock};
use log::{debug, warn};
use std::collections::{HashMap, HashSet};

use super::core::SWRLRuleEngine;
use super::matching::{PatternMatcher, UnificationEngine};

/// Rule validation engine
#[derive(Debug)]
pub struct RuleValidator {
    pattern_matcher: PatternMatcher,
}

impl RuleValidator {
    /// Create a new rule validator
    pub fn new() -> Self {
        Self {
            pattern_matcher: PatternMatcher::new(),
        }
    }

    /// Validate a SWRL rule
    pub fn validate_rule(&mut self, rule: &SWRLRule) -> Result<()> {
        // Check if rule has both body and head
        if rule.body.is_empty() {
            return Err(Error::reasoning("SWRL rule must have a non-empty body"));
        }

        if rule.head.is_empty() {
            return Err(Error::reasoning("SWRL rule must have a non-empty head"));
        }

        // Validate that all variables in head appear in body (safety condition)
        let body_variables = self.extract_variables_from_atoms(&rule.body);
        let head_variables = self.extract_variables_from_atoms(&rule.head);

        for var in &head_variables {
            if !body_variables.contains(var) {
                return Err(Error::reasoning(format!(
                    "Variable {} in rule head does not appear in rule body (safety violation)",
                    var
                )));
            }
        }

        // Additional validation rules can be added here
        self.validate_atom_consistency(&rule.body)?;
        self.validate_atom_consistency(&rule.head)?;

        debug!("Rule validation successful: {:?}", rule);
        Ok(())
    }

    /// Extract all variables from a list of atoms
    fn extract_variables_from_atoms(&self, atoms: &[SWRLAtom]) -> HashSet<String> {
        let mut variables = HashSet::new();

        for atom in atoms {
            match atom {
                SWRLAtom::ClassAtom { argument, .. } => {
                    if let SWRLIArgument::Variable(var) = argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::DataRangeAtom { argument, .. } => {
                    if let SWRLDArgument::Variable(var) = argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::ObjectPropertyAtom {
                    first_argument,
                    second_argument,
                    ..
                } => {
                    if let SWRLIArgument::Variable(var) = first_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                    if let SWRLIArgument::Variable(var) = second_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::DataPropertyAtom {
                    first_argument,
                    second_argument,
                    ..
                } => {
                    if let SWRLIArgument::Variable(var) = first_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                    if let SWRLDArgument::Variable(var) = second_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::SameIndividualAtom {
                    first_argument,
                    second_argument,
                } => {
                    if let SWRLIArgument::Variable(var) = first_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                    if let SWRLIArgument::Variable(var) = second_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument,
                    second_argument,
                } => {
                    if let SWRLIArgument::Variable(var) = first_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                    if let SWRLIArgument::Variable(var) = second_argument {
                        variables.insert(var.iri.as_str().to_string());
                    }
                }
                SWRLAtom::BuiltInAtom { arguments, .. } => {
                    for arg in arguments {
                        if let SWRLDArgument::Variable(var) = arg {
                            variables.insert(var.iri.as_str().to_string());
                        }
                    }
                }
            }
        }

        variables
    }

    /// Validate consistency within atoms
    fn validate_atom_consistency(&self, atoms: &[SWRLAtom]) -> Result<()> {
        // Check for obvious contradictions like SameIndividual(x,y) and DifferentIndividuals(x,y)
        let mut same_pairs = HashSet::new();
        let mut different_pairs = HashSet::new();

        for atom in atoms {
            match atom {
                SWRLAtom::SameIndividualAtom {
                    first_argument,
                    second_argument,
                } => {
                    let pair = self.normalize_argument_pair(first_argument, second_argument);
                    same_pairs.insert(pair);
                }
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument,
                    second_argument,
                } => {
                    let pair = self.normalize_argument_pair(first_argument, second_argument);
                    different_pairs.insert(pair);
                }
                _ => {} // Other atoms don't directly contradict
            }
        }

        // Check for contradictions
        for pair in &same_pairs {
            if different_pairs.contains(pair) {
                return Err(Error::reasoning(format!(
                    "Rule contains contradiction: individuals are both same and different: {:?}",
                    pair
                )));
            }
        }

        Ok(())
    }

    /// Normalize argument pair for comparison
    fn normalize_argument_pair(
        &self,
        arg1: &SWRLIArgument,
        arg2: &SWRLIArgument,
    ) -> (String, String) {
        let str1 = format!("{:?}", arg1);
        let str2 = format!("{:?}", arg2);

        if str1 <= str2 {
            (str1, str2)
        } else {
            (str2, str1)
        }
    }
}

impl Default for RuleValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Goal satisfaction checker
#[derive(Debug)]
pub struct GoalChecker {
    pattern_matcher: PatternMatcher,
    unification_engine: UnificationEngine,
}

impl GoalChecker {
    /// Create a new goal checker
    pub fn new() -> Self {
        Self {
            pattern_matcher: PatternMatcher::new(),
            unification_engine: UnificationEngine::new(),
        }
    }

    /// Check if a goal is satisfied by current facts in the ontology
    pub fn is_goal_satisfied(&mut self, engine: &SWRLRuleEngine, goal: &SWRLAtom) -> Result<bool> {
        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL validation: reading ontology for goal satisfaction")?;

            match goal {
                SWRLAtom::ClassAtom {
                    predicate,
                    argument,
                } => self.check_class_membership(engine, argument, predicate),
                SWRLAtom::ObjectPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => self.check_object_property_assertion(
                    engine,
                    first_argument,
                    predicate,
                    second_argument,
                ),
                SWRLAtom::DataPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => self.check_data_property_assertion(
                    engine,
                    first_argument,
                    predicate,
                    second_argument,
                ),
                SWRLAtom::SameIndividualAtom {
                    first_argument,
                    second_argument,
                } => self.check_same_individual(engine, first_argument, second_argument),
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument,
                    second_argument,
                } => self.check_different_individuals(engine, first_argument, second_argument),
                SWRLAtom::DataRangeAtom {
                    predicate,
                    argument,
                } => self.check_data_range_membership(engine, argument, predicate),
                SWRLAtom::BuiltInAtom { .. } => {
                    // Built-in atoms need special evaluation
                    self.check_builtin_satisfaction(engine, goal)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Find rules that could potentially prove a goal
    pub fn find_rules_for_goal(
        &mut self,
        engine: &SWRLRuleEngine,
        goal: &SWRLAtom,
    ) -> Result<Vec<u64>> {
        let mut applicable_rules = Vec::new();

        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL validation: reading ontology for finding rules")?;

            for axiom in ontology_guard.axioms() {
                if let Axiom::Rule(rule_axiom) = axiom {
                    // Check if any atom in the rule head can unify with the goal
                    for head_atom in &rule_axiom.rule.head {
                        if self
                            .pattern_matcher
                            .goal_matches_rule_head(goal, head_atom)?
                        {
                            applicable_rules.push(rule_axiom.id);
                            break;
                        }
                    }
                }
            }
        }

        Ok(applicable_rules)
    }

    /// Extract subgoals from rule body
    pub fn extract_subgoals_from_rule_body(&self, body: &[SWRLAtom]) -> Result<Vec<SWRLAtom>> {
        // For now, return all body atoms as subgoals
        // More sophisticated goal decomposition could be implemented
        Ok(body.to_vec())
    }

    /// Check class membership
    fn check_class_membership(
        &self,
        engine: &SWRLRuleEngine,
        individual: &SWRLIArgument,
        class: &ClassExpression,
    ) -> Result<bool> {
        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL validation: reading ontology for class membership")?;

            // Check for direct class assertions
            for axiom in ontology_guard.axioms() {
                if let Axiom::ClassAssertion(assertion) = axiom {
                    if self.matches_class_assertion_axiom(individual, class, assertion)? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check object property assertion
    fn check_object_property_assertion(
        &self,
        engine: &SWRLRuleEngine,
        subject: &SWRLIArgument,
        property: &ObjectPropertyExpression,
        object: &SWRLIArgument,
    ) -> Result<bool> {
        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL validation: reading ontology for object property")?;

            for axiom in ontology_guard.axioms() {
                if let Axiom::ObjectPropertyAssertion(assertion) = axiom {
                    if self.matches_object_property_assertion_axiom(
                        subject, property, object, assertion,
                    )? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check data property assertion
    fn check_data_property_assertion(
        &self,
        engine: &SWRLRuleEngine,
        subject: &SWRLIArgument,
        property: &crate::ontology::DataPropertyExpression,
        object: &SWRLDArgument,
    ) -> Result<bool> {
        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL validation: reading ontology for data property")?;

            for axiom in ontology_guard.axioms() {
                if let Axiom::DataPropertyAssertion(assertion) = axiom {
                    if self.matches_data_property_assertion_axiom(
                        subject, property, object, assertion,
                    )? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check same individual assertion
    fn check_same_individual(
        &self,
        _engine: &SWRLRuleEngine,
        _arg1: &SWRLIArgument,
        _arg2: &SWRLIArgument,
    ) -> Result<bool> {
        // Implement same individual checking
        Ok(false)
    }

    /// Check different individuals assertion
    fn check_different_individuals(
        &self,
        _engine: &SWRLRuleEngine,
        _arg1: &SWRLIArgument,
        _arg2: &SWRLIArgument,
    ) -> Result<bool> {
        // Implement different individuals checking
        Ok(false)
    }

    /// Check data range membership
    fn check_data_range_membership(
        &self,
        _engine: &SWRLRuleEngine,
        _argument: &SWRLDArgument,
        _data_range: &crate::ontology::DataRange,
    ) -> Result<bool> {
        // Implement data range checking
        Ok(false)
    }

    /// Check built-in satisfaction
    fn check_builtin_satisfaction(
        &self,
        _engine: &SWRLRuleEngine,
        _atom: &SWRLAtom,
    ) -> Result<bool> {
        // Built-ins require evaluation with current variable bindings
        Ok(false)
    }

    /// Helper methods for matching assertions
    fn matches_class_assertion_axiom(
        &self,
        individual: &SWRLIArgument,
        class: &ClassExpression,
        assertion: &crate::ontology::axioms::ClassAssertionAxiom,
    ) -> Result<bool> {
        // Implement matching logic
        match individual {
            SWRLIArgument::Individual(ind) => Ok(ind.iri().map(|i| i.as_str())
                == assertion.individual.iri().map(|i| i.as_str())
                && class == &assertion.class),
            SWRLIArgument::Variable(_) => {
                // Variables can match any individual
                Ok(class == &assertion.class)
            }
        }
    }

    fn matches_object_property_assertion_axiom(
        &self,
        subject: &SWRLIArgument,
        property: &ObjectPropertyExpression,
        object: &SWRLIArgument,
        assertion: &crate::ontology::axioms::ObjectPropertyAssertionAxiom,
    ) -> Result<bool> {
        let subject_match = match subject {
            SWRLIArgument::Individual(ind) => {
                ind.iri().map(|i| i.as_str()) == assertion.source.iri().map(|i| i.as_str())
            }
            SWRLIArgument::Variable(_) => true, // Variables can match any individual
        };

        let object_match = match object {
            SWRLIArgument::Individual(ind) => {
                ind.iri().map(|i| i.as_str()) == assertion.target.iri().map(|i| i.as_str())
            }
            SWRLIArgument::Variable(_) => true, // Variables can match any individual
        };

        let property_match = self.object_properties_match(property, &assertion.property);

        Ok(subject_match && object_match && property_match)
    }

    fn matches_data_property_assertion_axiom(
        &self,
        subject: &SWRLIArgument,
        property: &crate::ontology::DataPropertyExpression,
        object: &SWRLDArgument,
        assertion: &crate::ontology::axioms::DataPropertyAssertionAxiom,
    ) -> Result<bool> {
        let subject_match = match subject {
            SWRLIArgument::Individual(ind) => ind.iri() == assertion.individual.iri(),
            SWRLIArgument::Variable(_) => true, // Variables can match any individual
        };

        let object_match = match object {
            SWRLDArgument::Literal(lit) => {
                lit.value == assertion.value.value && lit.datatype == assertion.value.datatype
            }
            SWRLDArgument::Variable(_) => true, // Variables can match any literal
        };

        let property_match = self.data_properties_match(property, &assertion.property);

        Ok(subject_match && object_match && property_match)
    }

    /// Helper methods for expression matching
    fn class_expressions_match(&self, expr1: &ClassExpression, expr2: &ClassExpression) -> bool {
        match (expr1, expr2) {
            (ClassExpression::Class(c1), ClassExpression::Class(c2)) => c1.iri == c2.iri,
            _ => false, // More complex matching could be implemented
        }
    }

    fn object_properties_match(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                ObjectPropertyExpression::ObjectProperty(p1),
                ObjectPropertyExpression::ObjectProperty(p2),
            ) => p1.iri == p2.iri,
            _ => false, // More complex matching could be implemented
        }
    }

    fn data_properties_match(
        &self,
        prop1: &crate::ontology::DataPropertyExpression,
        prop2: &crate::ontology::DataPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                crate::ontology::DataPropertyExpression::DataProperty(p1),
                crate::ontology::DataPropertyExpression::DataProperty(p2),
            ) => p1.iri == p2.iri,
            _ => false,
        }
    }
}

impl Default for GoalChecker {
    fn default() -> Self {
        Self::new()
    }
}
