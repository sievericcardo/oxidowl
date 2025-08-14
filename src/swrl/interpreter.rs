//! SWRL Rule Interpreter
//!
//! This module implements the core logic for interpreting and executing
//! individual SWRL rules, including atom evaluation and variable binding.

use crate::{Result};
use crate::ontology::{axioms::*, *};
use crate::swrl::{
    SWRLExecutionContext, SWRLExecutionResult,
    builtins::{SWRLBuiltInRegistry, SWRLValue},
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use log::trace;

/// SWRL Rule Interpreter
///
/// Responsible for executing individual SWRL rules by evaluating atoms,
/// managing variable bindings, and generating inferences.
pub struct SWRLInterpreter {
    /// Registry of built-in predicates
    builtin_registry: Arc<SWRLBuiltInRegistry>,
}

impl SWRLInterpreter {
    /// Create a new rule interpreter
    #[must_use]
    pub fn new(builtin_registry: Arc<SWRLBuiltInRegistry>) -> Self {
        Self {
            builtin_registry,
        }
    }

    /// Execute a SWRL rule against the given ontology
    pub fn execute_rule(
        &mut self,
        rule: &SWRLRule,
        context: &mut SWRLExecutionContext,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<SWRLExecutionResult> {
        trace!("Interpreting SWRL rule: {:?}", rule);

        // Find all possible variable bindings that satisfy the rule body
        let bindings = self.find_satisfying_bindings(rule, context, ontology)?;
        
        if bindings.is_empty() {
            return Ok(SWRLExecutionResult::empty());
        }

        let mut inferences = Vec::new();
        let mut applications = 0;

        // For each satisfying binding, generate head inferences
        for binding in bindings {
            context.bindings = binding;
            applications += 1;

            // Generate inferences from the rule head
            let head_inferences = self.generate_head_inferences(rule, context)?;
            inferences.extend(head_inferences);
        }

        let fired = !inferences.is_empty();
        Ok(SWRLExecutionResult::new(fired, inferences, applications))
    }

    /// Find all variable bindings that satisfy the rule body
    fn find_satisfying_bindings(
        &self,
        rule: &SWRLRule,
        context: &mut SWRLExecutionContext,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut satisfying_bindings = Vec::new();
        
        // Start with empty binding and try to satisfy all body atoms
        let initial_binding = HashMap::new();
        let mut candidate_bindings = vec![initial_binding];

        for atom in &rule.body {
            let mut new_candidates = Vec::new();
            
            for binding in candidate_bindings {
                let atom_bindings = self.find_atom_bindings(atom, &binding, ontology)?;
                new_candidates.extend(atom_bindings);
            }
            
            candidate_bindings = new_candidates;
            
            // Early termination if no bindings satisfy current atom
            if candidate_bindings.is_empty() {
                break;
            }
        }

        satisfying_bindings.extend(candidate_bindings);
        
        trace!("Found {} satisfying bindings", satisfying_bindings.len());
        Ok(satisfying_bindings)
    }

    /// Find bindings that satisfy a single atom
    fn find_atom_bindings(
        &self,
        atom: &SWRLAtom,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        trace!("Finding bindings for atom: {:?}", atom);

        match atom {
            SWRLAtom::ClassAtom { predicate, argument } => {
                self.find_class_atom_bindings(predicate, argument, current_binding, ontology)
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => self.find_object_property_atom_bindings(
                predicate,
                first_argument,
                second_argument,
                current_binding,
                ontology,
            ),
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => self.find_data_property_atom_bindings(
                predicate,
                first_argument,
                second_argument,
                current_binding,
                ontology,
            ),
            SWRLAtom::DataRangeAtom { predicate, argument } => {
                self.find_data_range_atom_bindings(predicate, argument, current_binding, ontology)
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => self.find_same_individual_atom_bindings(
                first_argument,
                second_argument,
                current_binding,
                ontology,
            ),
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => self.find_different_individuals_atom_bindings(
                first_argument,
                second_argument,
                current_binding,
                ontology,
            ),
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => self.find_builtin_atom_bindings(
                predicate,
                arguments,
                current_binding,
                ontology,
            ),
        }
    }

    /// Find bindings for a class atom
    fn find_class_atom_bindings(
        &self,
        predicate: &ClassExpression,
        argument: &SWRLIArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut bindings = Vec::new();
        let ontology_guard = ontology.read().unwrap();

        match argument {
            SWRLIArgument::Individual(individual) => {
                // Check if the individual is an instance of the class
                if self.is_individual_instance_of_class(individual, predicate, &ontology_guard)? {
                    bindings.push(current_binding.clone());
                }
            }
            SWRLIArgument::Variable(variable) => {
                // Check if variable is already bound
                if let Some(value) = current_binding.get(variable) {
                    if let Some(individual) = value.as_individual() {
                        if self.is_individual_instance_of_class(individual, predicate, &ontology_guard)? {
                            bindings.push(current_binding.clone());
                        }
                    }
                } else {
                    // Find all individuals that are instances of the class
                    let instances = self.find_class_instances(predicate, &ontology_guard)?;
                    for instance in instances {
                        let mut new_binding = current_binding.clone();
                        new_binding.insert(variable.clone(), SWRLValue::Individual(instance));
                        bindings.push(new_binding);
                    }
                }
            }
        }

        Ok(bindings)
    }

    /// Find bindings for an object property atom
    fn find_object_property_atom_bindings(
        &self,
        predicate: &ObjectPropertyExpression,
        first_argument: &SWRLIArgument,
        second_argument: &SWRLIArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut bindings = Vec::new();
        let ontology_guard = ontology.read().unwrap();

        // Get all object property assertions for this property
        let property_assertions = self.find_object_property_assertions(predicate, &ontology_guard)?;

        for (subject, object) in property_assertions {
            let mut new_binding = current_binding.clone();
            let mut valid = true;

            // Try to bind first argument
            match first_argument {
                SWRLIArgument::Individual(individual) => {
                    if *individual != subject {
                        valid = false;
                    }
                }
                SWRLIArgument::Variable(variable) => {
                    if let Some(existing_value) = current_binding.get(variable) {
                        if let Some(existing_individual) = existing_value.as_individual() {
                            if *existing_individual != subject {
                                valid = false;
                            }
                        }
                    } else {
                        new_binding.insert(variable.clone(), SWRLValue::Individual(subject.clone()));
                    }
                }
            }

            // Try to bind second argument
            if valid {
                match second_argument {
                    SWRLIArgument::Individual(individual) => {
                        if *individual != object {
                            valid = false;
                        }
                    }
                    SWRLIArgument::Variable(variable) => {
                        if let Some(existing_value) = new_binding.get(variable) {
                            if let Some(existing_individual) = existing_value.as_individual() {
                                if *existing_individual != object {
                                    valid = false;
                                }
                            }
                        } else {
                            new_binding.insert(variable.clone(), SWRLValue::Individual(object));
                        }
                    }
                }
            }

            if valid {
                bindings.push(new_binding);
            }
        }

        Ok(bindings)
    }

    /// Find bindings for a data property atom
    fn find_data_property_atom_bindings(
        &self,
        predicate: &DataPropertyExpression,
        first_argument: &SWRLIArgument,
        second_argument: &SWRLDArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut bindings = Vec::new();
        let ontology_guard = ontology.read().unwrap();

        // Get all data property assertions for this property
        let property_assertions = self.find_data_property_assertions(predicate, &ontology_guard)?;

        for (subject, literal) in property_assertions {
            let mut new_binding = current_binding.clone();
            let mut valid = true;

            // Try to bind first argument (individual)
            match first_argument {
                SWRLIArgument::Individual(individual) => {
                    if *individual != subject {
                        valid = false;
                    }
                }
                SWRLIArgument::Variable(variable) => {
                    if let Some(existing_value) = current_binding.get(variable) {
                        if let Some(existing_individual) = existing_value.as_individual() {
                            if *existing_individual != subject {
                                valid = false;
                            }
                        }
                    } else {
                        new_binding.insert(variable.clone(), SWRLValue::Individual(subject.clone()));
                    }
                }
            }

            // Try to bind second argument (data value)
            if valid {
                match second_argument {
                    SWRLDArgument::Literal(expected_literal) => {
                        if *expected_literal != literal {
                            valid = false;
                        }
                    }
                    SWRLDArgument::Variable(variable) => {
                        if let Some(existing_value) = new_binding.get(variable) {
                            if let Some(existing_literal) = existing_value.as_literal() {
                                if *existing_literal != literal {
                                    valid = false;
                                }
                            }
                        } else {
                            new_binding.insert(variable.clone(), SWRLValue::Literal(literal));
                        }
                    }
                }
            }

            if valid {
                bindings.push(new_binding);
            }
        }

        Ok(bindings)
    }

    /// Find bindings for a data range atom
    fn find_data_range_atom_bindings(
        &self,
        _predicate: &DataRange,
        _argument: &SWRLDArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        _ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        // For now, just return the current binding
        // A full implementation would check data range constraints
        Ok(vec![current_binding.clone()])
    }

    /// Find bindings for a same individual atom
    fn find_same_individual_atom_bindings(
        &self,
        first_argument: &SWRLIArgument,
        second_argument: &SWRLIArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut bindings = Vec::new();
        let ontology_guard = ontology.read().unwrap();

        // Get the individuals from arguments or variables
        let first_individuals = self.get_individuals_from_argument(first_argument, current_binding, &ontology_guard)?;
        let second_individuals = self.get_individuals_from_argument(second_argument, current_binding, &ontology_guard)?;

        for first_ind in &first_individuals {
            for second_ind in &second_individuals {
                if self.are_same_individuals(first_ind, second_ind, &ontology_guard)? {
                    let mut new_binding = current_binding.clone();
                    
                    // Bind variables if needed
                    if let SWRLIArgument::Variable(var) = first_argument {
                        new_binding.insert(var.clone(), SWRLValue::Individual(first_ind.clone()));
                    }
                    if let SWRLIArgument::Variable(var) = second_argument {
                        new_binding.insert(var.clone(), SWRLValue::Individual(second_ind.clone()));
                    }
                    
                    bindings.push(new_binding);
                }
            }
        }

        Ok(bindings)
    }

    /// Find bindings for a different individuals atom
    fn find_different_individuals_atom_bindings(
        &self,
        first_argument: &SWRLIArgument,
        second_argument: &SWRLIArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        let mut bindings = Vec::new();
        let ontology_guard = ontology.read().unwrap();

        // Get the individuals from arguments or variables
        let first_individuals = self.get_individuals_from_argument(first_argument, current_binding, &ontology_guard)?;
        let second_individuals = self.get_individuals_from_argument(second_argument, current_binding, &ontology_guard)?;

        for first_ind in &first_individuals {
            for second_ind in &second_individuals {
                if self.are_different_individuals(first_ind, second_ind, &ontology_guard)? {
                    let mut new_binding = current_binding.clone();
                    
                    // Bind variables if needed
                    if let SWRLIArgument::Variable(var) = first_argument {
                        new_binding.insert(var.clone(), SWRLValue::Individual(first_ind.clone()));
                    }
                    if let SWRLIArgument::Variable(var) = second_argument {
                        new_binding.insert(var.clone(), SWRLValue::Individual(second_ind.clone()));
                    }
                    
                    bindings.push(new_binding);
                }
            }
        }

        Ok(bindings)
    }

    /// Find bindings for a built-in atom
    fn find_builtin_atom_bindings(
        &self,
        predicate: &IRI,
        arguments: &[SWRLDArgument],
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        _ontology: &Arc<RwLock<Ontology>>,
    ) -> Result<Vec<HashMap<SWRLVariable, SWRLValue>>> {
        
        if self.builtin_registry.is_registered(predicate) {
            // Convert arguments to values using current bindings
            let mut arg_values = Vec::new();
            let mut all_bound = true;
            
            for arg in arguments {
                match arg {
                    SWRLDArgument::Literal(lit) => {
                        arg_values.push(SWRLValue::Literal(lit.clone()));
                    }
                    SWRLDArgument::Variable(var) => {
                        if let Some(value) = current_binding.get(var) {
                            arg_values.push(value.clone());
                        } else {
                            all_bound = false;
                            break;
                        }
                    }
                }
            }
            
            if all_bound {
                // Execute the built-in
                match self.builtin_registry.execute(predicate, &arg_values) {
                    Ok(SWRLValue::Boolean(true)) => {
                        return Ok(vec![current_binding.clone()]);
                    }
                    Ok(SWRLValue::Boolean(false)) => {
                        return Ok(vec![]);
                    }
                    Ok(_result_value) => {
                        // Built-in returned a value - this might be used for variable binding
                        // For now, just consider it successful
                        return Ok(vec![current_binding.clone()]);
                    }
                    Err(_) => {
                        // Built-in execution failed
                        return Ok(vec![]);
                    }
                }
            }
        }
        
        Ok(Vec::new())
    }

    /// Generate inferences from the rule head
    fn generate_head_inferences(
        &self,
        rule: &SWRLRule,
        context: &SWRLExecutionContext,
    ) -> Result<Vec<Axiom>> {
        let mut inferences = Vec::new();
        
        for atom in &rule.head {
            let inference = self.generate_atom_inference(atom, context)?;
            if let Some(axiom) = inference {
                inferences.push(axiom);
            }
        }
        
        Ok(inferences)
    }

    /// Generate an inference axiom from a head atom
    fn generate_atom_inference(
        &self,
        atom: &SWRLAtom,
        context: &SWRLExecutionContext,
    ) -> Result<Option<Axiom>> {
        match atom {
            SWRLAtom::ClassAtom { predicate, argument } => {
                if let Some(individual) = self.resolve_individual_argument(argument, context) {
                    let axiom = ClassAssertionAxiom {
                        id: 0, // Will be assigned by the axiom store
                        individual,
                        class: predicate.clone(),
                        annotations: Vec::new(),
                    };
                    Ok(Some(Axiom::ClassAssertion(axiom)))
                } else {
                    Ok(None)
                }
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                if let (Some(source), Some(target)) = (
                    self.resolve_individual_argument(first_argument, context),
                    self.resolve_individual_argument(second_argument, context),
                ) {
                    let axiom = ObjectPropertyAssertionAxiom {
                        id: 0,
                        source,
                        target,
                        property: predicate.clone(),
                        annotations: Vec::new(),
                    };
                    Ok(Some(Axiom::ObjectPropertyAssertion(axiom)))
                } else {
                    Ok(None)
                }
            }
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                if let (Some(individual), Some(literal)) = (
                    self.resolve_individual_argument(first_argument, context),
                    self.resolve_data_argument(second_argument, context),
                ) {
                    let axiom = DataPropertyAssertionAxiom {
                        id: 0,
                        individual,
                        property: predicate.clone(),
                        value: literal,
                        annotations: Vec::new(),
                    };
                    Ok(Some(Axiom::DataPropertyAssertion(axiom)))
                } else {
                    Ok(None)
                }
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => {
                if let (Some(first), Some(second)) = (
                    self.resolve_individual_argument(first_argument, context),
                    self.resolve_individual_argument(second_argument, context),
                ) {
                    let axiom = SameIndividualAxiom {
                        id: 0,
                        individuals: vec![first, second],
                        annotations: Vec::new(),
                    };
                    Ok(Some(Axiom::SameIndividual(axiom)))
                } else {
                    Ok(None)
                }
            }
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                if let (Some(first), Some(second)) = (
                    self.resolve_individual_argument(first_argument, context),
                    self.resolve_individual_argument(second_argument, context),
                ) {
                    let axiom = DifferentIndividualsAxiom {
                        id: 0,
                        individuals: vec![first, second],
                        annotations: Vec::new(),
                    };
                    Ok(Some(Axiom::DifferentIndividuals(axiom)))
                } else {
                    Ok(None)
                }
            }
            _ => {
                // Built-in atoms and data range atoms don't generate inferences directly
                Ok(None)
            }
        }
    }

    /// Resolve an individual argument to a concrete individual
    fn resolve_individual_argument(
        &self,
        argument: &SWRLIArgument,
        context: &SWRLExecutionContext,
    ) -> Option<Individual> {
        match argument {
            SWRLIArgument::Individual(individual) => Some(individual.clone()),
            SWRLIArgument::Variable(variable) => {
                context.get_binding(variable)?.as_individual().cloned()
            }
        }
    }

    /// Resolve a data argument to a concrete literal
    fn resolve_data_argument(
        &self,
        argument: &SWRLDArgument,
        context: &SWRLExecutionContext,
    ) -> Option<Literal> {
        match argument {
            SWRLDArgument::Literal(literal) => Some(literal.clone()),
            SWRLDArgument::Variable(variable) => {
                context.get_binding(variable)?.as_literal().cloned()
            }
        }
    }

    // Helper methods for ontology queries

    /// Check if an individual is an instance of a class
    fn is_individual_instance_of_class(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        for axiom in ontology.axioms() {
            if let Axiom::ClassAssertion(assertion) = axiom {
                if assertion.individual == *individual && assertion.class == *class_expr {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Find all instances of a class
    fn find_class_instances(
        &self,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<Vec<Individual>> {
        let mut instances = Vec::new();
        
        for axiom in ontology.axioms() {
            if let Axiom::ClassAssertion(assertion) = axiom {
                if assertion.class == *class_expr {
                    instances.push(assertion.individual.clone());
                }
            }
        }
        
        Ok(instances)
    }

    /// Find all object property assertions for a property
    fn find_object_property_assertions(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &Ontology,
    ) -> Result<Vec<(Individual, Individual)>> {
        let mut assertions = Vec::new();
        
        for axiom in ontology.axioms() {
            if let Axiom::ObjectPropertyAssertion(assertion) = axiom {
                if assertion.property == *property {
                    assertions.push((assertion.source.clone(), assertion.target.clone()));
                }
            }
        }
        
        Ok(assertions)
    }

    /// Find all data property assertions for a property
    fn find_data_property_assertions(
        &self,
        property: &DataPropertyExpression,
        ontology: &Ontology,
    ) -> Result<Vec<(Individual, Literal)>> {
        let mut assertions = Vec::new();
        
        for axiom in ontology.axioms() {
            if let Axiom::DataPropertyAssertion(assertion) = axiom {
                if assertion.property == *property {
                    assertions.push((assertion.individual.clone(), assertion.value.clone()));
                }
            }
        }
        
        Ok(assertions)
    }

    /// Get individuals from an argument (handling variables)
    fn get_individuals_from_argument(
        &self,
        argument: &SWRLIArgument,
        current_binding: &HashMap<SWRLVariable, SWRLValue>,
        ontology: &Ontology,
    ) -> Result<Vec<Individual>> {
        match argument {
            SWRLIArgument::Individual(individual) => Ok(vec![individual.clone()]),
            SWRLIArgument::Variable(variable) => {
                if let Some(value) = current_binding.get(variable) {
                    if let Some(individual) = value.as_individual() {
                        Ok(vec![individual.clone()])
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    // Return all individuals from the ontology
                    let individuals = ontology.individuals();
                    Ok(individuals.into_iter().map(|(_, ind)| ind).collect())
                }
            }
        }
    }

    /// Check if two individuals are the same
    fn are_same_individuals(
        &self,
        first: &Individual,
        second: &Individual,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for explicit same individual assertions
        for axiom in ontology.axioms() {
            if let Axiom::SameIndividual(assertion) = axiom {
                if assertion.individuals.contains(first) && assertion.individuals.contains(second) {
                    return Ok(true);
                }
            }
        }
        
        // Also check structural equality
        Ok(first == second)
    }

    /// Check if two individuals are different
    fn are_different_individuals(
        &self,
        first: &Individual,
        second: &Individual,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for explicit different individuals assertions
        for axiom in ontology.axioms() {
            if let Axiom::DifferentIndividuals(assertion) = axiom {
                if assertion.individuals.contains(first) && assertion.individuals.contains(second) {
                    return Ok(true);
                }
            }
        }
        
        // If no explicit assertion, assume different if not the same
        Ok(first != second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, ClassExpression, Individual};

    fn create_test_interpreter() -> SWRLInterpreter {
        let builtin_registry = Arc::new(SWRLBuiltInRegistry::new());
        SWRLInterpreter::new(builtin_registry)
    }

    fn create_test_ontology_with_data() -> Arc<RwLock<Ontology>> {
        let mut ontology = Ontology::new();
        
        // Add individuals
        let john = Individual::named(IRI::new("http://example.org/john"));
        let mary = Individual::named(IRI::new("http://example.org/mary"));
        
        // Add class assertions
        let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        let john_assertion = ClassAssertionAxiom {
            id: 1,
            individual: john.clone(),
            class: person_class.clone(),
            annotations: Vec::new(),
        };
        
        let mary_assertion = ClassAssertionAxiom {
            id: 2,
            individual: mary,
            class: person_class,
            annotations: Vec::new(),
        };
        
        ontology.add_axiom(Axiom::ClassAssertion(john_assertion));
        ontology.add_axiom(Axiom::ClassAssertion(mary_assertion));
        
        Arc::new(RwLock::new(ontology))
    }

    #[test]
    fn test_interpreter_creation() {
        let interpreter = create_test_interpreter();
        let iri = IRI::new("http://www.w3.org/2003/11/swrlb#equal");
        assert!(interpreter.builtin_registry.is_registered(&iri));
    }

    #[test]
    fn test_class_atom_bindings() {
        let interpreter = create_test_interpreter();
        let ontology = create_test_ontology_with_data();
        
        let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let argument = SWRLIArgument::Variable(var_x);
        
        let current_binding = HashMap::new();
        
        let bindings = interpreter.find_class_atom_bindings(
            &person_class,
            &argument,
            &current_binding,
            &ontology,
        ).unwrap();
        
        // Should find bindings for both john and mary
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_resolve_individual_argument() {
        let interpreter = create_test_interpreter();
        let john = Individual::named(IRI::new("http://example.org/john"));
        
        // Test with concrete individual
        let concrete_arg = SWRLIArgument::Individual(john.clone());
        let context = SWRLExecutionContext::new();
        
        let resolved = interpreter.resolve_individual_argument(&concrete_arg, &context);
        assert_eq!(resolved, Some(john.clone()));
        
        // Test with variable
        let var = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_arg = SWRLIArgument::Variable(var.clone());
        
        let mut context_with_binding = SWRLExecutionContext::new();
        context_with_binding.bind(var, SWRLValue::Individual(john.clone())).unwrap();
        
        let resolved = interpreter.resolve_individual_argument(&var_arg, &context_with_binding);
        assert_eq!(resolved, Some(john));
    }
}
