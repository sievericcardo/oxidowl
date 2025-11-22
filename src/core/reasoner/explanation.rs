//! Explanation services for reasoning results
//!
//! This module provides functionality for explaining entailments, inconsistencies,
//! and other reasoning results.

use crate::{
    Result,
    ontology::{Axiom, ClassExpression, Individual, Ontology},
};
use std::collections::{HashMap, HashSet};

/// Service for generating explanations of reasoning results
#[derive(Debug)]
pub struct ExplanationService;

impl ExplanationService {
    /// Create a new explanation service
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Explain entailment
    pub fn explain_entailment(&self, axiom: &Axiom, ontology: &Ontology) -> Result<Vec<Axiom>> {
        // Basic explanation by finding relevant axioms that contribute to the entailment
        let mut explanation = Vec::new();

        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                // Look for transitive chains and direct declarations
                let subclass = &subclass_axiom.subclass;
                let superclass = &subclass_axiom.superclass;

                // Check for direct axioms that support this inference
                for ontology_axiom in ontology.axioms() {
                    match ontology_axiom {
                        Axiom::SubClassOf(existing_axiom) => {
                            // Direct match
                            if existing_axiom.subclass == *subclass
                                && existing_axiom.superclass == *superclass
                            {
                                explanation.push(ontology_axiom.clone());
                            }
                            // Transitive support (simplified)
                            else if existing_axiom.subclass == *subclass {
                                explanation.push(ontology_axiom.clone());
                            } else if existing_axiom.superclass == *superclass {
                                explanation.push(ontology_axiom.clone());
                            }
                        }
                        Axiom::EquivalentClasses(equiv_axiom) => {
                            // Check if either class is in the equivalence
                            if equiv_axiom.classes.contains(subclass)
                                || equiv_axiom.classes.contains(superclass)
                            {
                                explanation.push(ontology_axiom.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Axiom::ClassAssertion(class_assertion) => {
                // Find axioms that support the class membership
                for ontology_axiom in ontology.axioms() {
                    match ontology_axiom {
                        Axiom::ClassAssertion(existing_assertion) => {
                            if existing_assertion.individual == class_assertion.individual {
                                explanation.push(ontology_axiom.clone());
                            }
                        }
                        Axiom::SubClassOf(subclass_axiom) => {
                            // Check if this subclass relationship contributes
                            if subclass_axiom.superclass == class_assertion.class {
                                explanation.push(ontology_axiom.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                // For other axiom types, just look for exact matches
                for ontology_axiom in ontology.axioms() {
                    if std::mem::discriminant(ontology_axiom) == std::mem::discriminant(axiom) {
                        explanation.push(ontology_axiom.clone());
                    }
                }
            }
        }

        Ok(explanation)
    }

    /// Explain inconsistency
    pub fn explain_inconsistency(&self, ontology: &Ontology) -> Result<Vec<Axiom>> {
        // Find axioms that contribute to inconsistencies
        let mut explanation = Vec::new();

        // Check for obvious contradictions
        let mut disjoint_classes = HashMap::new();
        let mut class_assertions = HashMap::new();

        // Collect disjoint class declarations
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::DisjointClasses(disjoint_axiom) => {
                    for (i, class1) in disjoint_axiom.classes.iter().enumerate() {
                        for class2 in disjoint_axiom.classes.iter().skip(i + 1) {
                            disjoint_classes
                                .insert((class1.clone(), class2.clone()), axiom.clone());
                        }
                    }
                }
                Axiom::ClassAssertion(class_assertion) => {
                    class_assertions
                        .entry(class_assertion.individual.clone())
                        .or_insert_with(Vec::new)
                        .push((class_assertion.class.clone(), axiom.clone()));
                }
                _ => {}
            }
        }

        // Check for individuals asserted to be in disjoint classes
        for (_individual, assertions) in &class_assertions {
            for (i, (class1, axiom1)) in assertions.iter().enumerate() {
                for (class2, axiom2) in assertions.iter().skip(i + 1) {
                    // Check if these classes are disjoint
                    if let Some(disjoint_axiom) = disjoint_classes
                        .get(&(class1.clone(), class2.clone()))
                        .or_else(|| disjoint_classes.get(&(class2.clone(), class1.clone())))
                    {
                        explanation.push(axiom1.clone());
                        explanation.push(axiom2.clone());
                        explanation.push(disjoint_axiom.clone());
                    }
                }
            }
        }

        // Check for functional property violations
        let mut functional_properties = HashSet::new();
        let mut property_assertions = HashMap::new();

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::FunctionalObjectProperty(func_axiom) => {
                    functional_properties.insert(func_axiom.property.clone());
                    explanation.push(axiom.clone());
                }
                Axiom::ObjectPropertyAssertion(prop_assertion) => {
                    if functional_properties.contains(&prop_assertion.property) {
                        property_assertions
                            .entry((
                                prop_assertion.source.clone(),
                                prop_assertion.property.clone(),
                            ))
                            .or_insert_with(Vec::new)
                            .push((prop_assertion.target.clone(), axiom.clone()));
                    }
                }
                _ => {}
            }
        }

        // Check for multiple values for functional properties
        for ((_source, _property), targets) in &property_assertions {
            if targets.len() > 1 {
                for (_target, axiom) in targets {
                    explanation.push(axiom.clone());
                }
            }
        }

        Ok(explanation)
    }

    /// Explain why a class is unsatisfiable
    pub fn explain_unsatisfiability(
        &self,
        class: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<Vec<Axiom>> {
        let mut explanation = Vec::new();

        // Look for contradictory axioms that make the class unsatisfiable
        for axiom in ontology.axioms() {
            match axiom {
                // Check if the class is declared equivalent to owl:Nothing
                Axiom::EquivalentClasses(equiv_axiom) => {
                    if equiv_axiom.classes.contains(class) {
                        for equiv_class in &equiv_axiom.classes {
                            if let ClassExpression::Class(cls) = equiv_class {
                                if cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                                    explanation.push(axiom.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
                // Check if the class is declared as a subclass of owl:Nothing
                Axiom::SubClassOf(subclass_axiom) => {
                    if subclass_axiom.subclass == *class {
                        if let ClassExpression::Class(super_cls) = &subclass_axiom.superclass {
                            if super_cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                                explanation.push(axiom.clone());
                            }
                        }
                    }
                }
                // Check for disjoint classes that cover all possibilities
                Axiom::DisjointClasses(disjoint_axiom) => {
                    if disjoint_axiom.classes.contains(class) {
                        explanation.push(axiom.clone());
                    }
                }
                // Check for contradictory restrictions
                _ => {
                    // More sophisticated analysis would be needed for complex class expressions
                }
            }
        }

        Ok(explanation)
    }

    /// Explain a subsumption relationship
    pub fn explain_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<Vec<Axiom>> {
        let mut explanation = Vec::new();

        // Look for direct subsumption axioms
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(subclass_axiom) => {
                    if subclass_axiom.subclass == *subclass
                        && subclass_axiom.superclass == *superclass
                    {
                        explanation.push(axiom.clone());
                    }
                }
                Axiom::EquivalentClasses(equiv_axiom) => {
                    if equiv_axiom.classes.contains(subclass)
                        && equiv_axiom.classes.contains(superclass)
                    {
                        explanation.push(axiom.clone());
                    }
                }
                _ => {}
            }
        }

        // Look for transitive chains using enhanced path finding algorithm
        self.find_subsumption_chain_enhanced(subclass, superclass, ontology, &mut explanation)?;

        Ok(explanation)
    }

    /// Find a chain of subsumption relationships
    fn find_subsumption_chain(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        explanation: &mut Vec<Axiom>,
    ) -> Result<bool> {
        // Use a simple depth-first search to find subsumption chains
        let mut visited = HashSet::new();
        self.find_subsumption_path(subclass, superclass, ontology, explanation, &mut visited)
    }

    /// Recursively find a path from subclass to superclass
    fn find_subsumption_path(
        &self,
        current: &ClassExpression,
        target: &ClassExpression,
        ontology: &Ontology,
        explanation: &mut Vec<Axiom>,
        visited: &mut HashSet<ClassExpression>,
    ) -> Result<bool> {
        if visited.contains(current) {
            return Ok(false);
        }
        visited.insert(current.clone());

        if current == target {
            return Ok(true);
        }

        // Look for direct subsumption relationships
        for axiom in ontology.axioms() {
            if let Axiom::SubClassOf(subclass_axiom) = axiom {
                if subclass_axiom.subclass == *current {
                    // Found a step in the chain
                    explanation.push(axiom.clone());

                    if self.find_subsumption_path(
                        &subclass_axiom.superclass,
                        target,
                        ontology,
                        explanation,
                        visited,
                    )? {
                        return Ok(true);
                    }

                    // Backtrack
                    explanation.pop();
                }
            }
        }

        visited.remove(current);
        Ok(false)
    }

    /// Explain why an individual is an instance of a class
    pub fn explain_instance_of(
        &self,
        individual: &Individual,
        class: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<Vec<Axiom>> {
        let mut explanation = Vec::new();

        // Look for direct class assertions
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(class_assertion) => {
                    if class_assertion.individual == *individual {
                        // Check if the asserted class is our target or a subclass
                        if class_assertion.class == *class {
                            explanation.push(axiom.clone());
                        } else {
                            // Look for subsumption relationship
                            let subsumption_explanation =
                                self.explain_subsumption(&class_assertion.class, class, ontology)?;
                            if !subsumption_explanation.is_empty() {
                                explanation.push(axiom.clone());
                                explanation.extend(subsumption_explanation);
                            }
                        }
                    }
                }
                // Look for property assertions that might infer class membership
                Axiom::ObjectPropertyAssertion(prop_assertion) => {
                    if prop_assertion.source == *individual || prop_assertion.target == *individual
                    {
                        // Check if this property assertion contributes to class membership
                        // This would require more sophisticated reasoning about property restrictions
                        explanation.push(axiom.clone());
                    }
                }
                _ => {}
            }
        }

        Ok(explanation)
    }

    /// Generate a human-readable explanation
    pub fn format_explanation(&self, explanation: &[Axiom]) -> String {
        let mut output = String::new();
        output.push_str("Explanation:\n");

        for (i, axiom) in explanation.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, self.format_axiom(axiom)));
        }

        if explanation.is_empty() {
            output.push_str("No explanation found or axiom is asserted directly.\n");
        }

        output
    }

    /// Format an axiom for human reading
    fn format_axiom(&self, axiom: &Axiom) -> String {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                format!(
                    "{:?} ⊑ {:?}",
                    subclass_axiom.subclass, subclass_axiom.superclass
                )
            }
            Axiom::EquivalentClasses(equiv_axiom) => {
                let classes: Vec<String> = equiv_axiom
                    .classes
                    .iter()
                    .map(|c| format!("{:?}", c))
                    .collect();
                format!("EquivalentClasses({})", classes.join(", "))
            }
            Axiom::ClassAssertion(class_assertion) => {
                format!(
                    "{:?} ∈ {:?}",
                    class_assertion.individual, class_assertion.class
                )
            }
            Axiom::ObjectPropertyAssertion(prop_assertion) => {
                format!(
                    "({:?}, {:?}) ∈ {:?}",
                    prop_assertion.source, prop_assertion.target, prop_assertion.property
                )
            }
            Axiom::DisjointClasses(disjoint_axiom) => {
                let classes: Vec<String> = disjoint_axiom
                    .classes
                    .iter()
                    .map(|c| format!("{:?}", c))
                    .collect();
                format!("DisjointClasses({})", classes.join(", "))
            }
            Axiom::FunctionalObjectProperty(func_axiom) => {
                format!("FunctionalObjectProperty({:?})", func_axiom.property)
            }
            _ => format!("{:?}", axiom),
        }
    }

    /// Enhanced subsumption chain finding with breadth-first search
    fn find_subsumption_chain_enhanced(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        explanation: &mut Vec<Axiom>,
    ) -> Result<()> {
        use std::collections::{HashSet, VecDeque};

        // Use breadth-first search to find the shortest path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map: std::collections::HashMap<String, (String, Axiom)> =
            std::collections::HashMap::new();

        let start_key = format!("{:?}", subclass);
        let target_key = format!("{:?}", superclass);

        queue.push_back(start_key.clone());
        visited.insert(start_key.clone());

        while let Some(current) = queue.pop_front() {
            if current == target_key {
                // Found path - reconstruct it
                let mut path_axioms = Vec::new();
                let mut trace_key = current;

                while let Some((parent_key, axiom)) = parent_map.get(&trace_key) {
                    path_axioms.push(axiom.clone());
                    trace_key = parent_key.clone();

                    if trace_key == start_key {
                        break;
                    }
                }

                // Add path axioms to explanation (reverse order for proper chain)
                path_axioms.reverse();
                explanation.extend(path_axioms);
                return Ok(());
            }

            // Find all superclasses of current class
            for axiom in ontology.axioms() {
                if let Axiom::SubClassOf(subclass_axiom) = axiom {
                    let subclass_key = format!("{:?}", &subclass_axiom.subclass);
                    let superclass_key = format!("{:?}", &subclass_axiom.superclass);

                    if subclass_key == current && !visited.contains(&superclass_key) {
                        visited.insert(superclass_key.clone());
                        parent_map.insert(superclass_key.clone(), (current.clone(), axiom.clone()));
                        queue.push_back(superclass_key);
                    }
                }

                // Also check equivalent classes
                if let Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    for (i, class1) in equiv_axiom.classes.iter().enumerate() {
                        for (j, class2) in equiv_axiom.classes.iter().enumerate() {
                            if i != j {
                                let class1_key = format!("{:?}", class1);
                                let class2_key = format!("{:?}", class2);

                                if class1_key == current && !visited.contains(&class2_key) {
                                    visited.insert(class2_key.clone());
                                    parent_map.insert(
                                        class2_key.clone(),
                                        (current.clone(), axiom.clone()),
                                    );
                                    queue.push_back(class2_key);
                                }
                            }
                        }
                    }
                }
            }
        }

        // No direct path found - this is fine, not all subsumptions are explicit
        Ok(())
    }

    /// Find minimal explanations (remove redundant axioms)
    pub fn minimize_explanation(&self, explanation: Vec<Axiom>) -> Vec<Axiom> {
        // Simple minimization: remove duplicate axioms
        let mut minimal = Vec::new();

        for axiom in explanation {
            if !minimal.contains(&axiom) {
                minimal.push(axiom);
            }
        }

        // More sophisticated minimization would check if each axiom is actually necessary
        // by trying to derive the conclusion without it

        minimal
    }
}

impl Default for ExplanationService {
    fn default() -> Self {
        Self::new()
    }
}
