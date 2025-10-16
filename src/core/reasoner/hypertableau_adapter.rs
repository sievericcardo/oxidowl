//! Adapter for integrating the hypertableau algorithm with the reasoning system
//!
//! This module provides a bridge between the hypertableau expansion algorithm
//! and the existing tableau-based reasoning infrastructure. It translates
//! ontology structures into hypertableau format and provides a unified
//! interface through the `TableauRunner` trait.

use crate::{
    Result,
    core::{
        hypergraph::expansion::{ExpansionState, ExpansionStatistics, HypertableauExpansion},
        tableau::TableauState,
    },
    ontology::{Axiom, ClassExpression, Ontology},
};
use log::{debug, info, trace};

/// Wrapper for hypertableau algorithm that implements TableauRunner
pub struct HypertableauRunner {
    /// The hypertableau expansion engine
    expansion: HypertableauExpansion,

    /// Current state after expansion
    state: ExpansionState,

    /// Statistics about the expansion
    stats: ExpansionStatistics,

    /// Ontology being reasoned over (for reference)
    ontology_name: String,
}

impl HypertableauRunner {
    /// Create a new hypertableau runner from an ontology
    pub fn new(ontology: &Ontology) -> Result<Self> {
        info!("Initializing hypertableau runner for ontology");

        let mut expansion = HypertableauExpansion::new();

        // Extract root concepts from ontology
        let root_concepts = Self::extract_root_concepts(ontology)?;
        debug!("Extracted {} root concepts", root_concepts.len());

        // Add disjointness constraints
        Self::add_disjointness_constraints(&mut expansion, ontology)?;

        // Initialize expansion with root concepts
        expansion.initialize(root_concepts)?;

        Ok(Self {
            expansion,
            state: ExpansionState::Running,
            stats: ExpansionStatistics::default(),
            ontology_name: ontology
                .get_iri()
                .map_or_else(|| "anonymous".to_string(), |iri| iri.as_str().to_string()),
        })
    }

    /// Create a hypertableau runner for consistency checking
    pub fn for_consistency(ontology: &Ontology) -> Result<Self> {
        Self::new(ontology)
    }

    /// Create a hypertableau runner for satisfiability checking
    pub fn for_satisfiability(ontology: &Ontology, class_expr: &ClassExpression) -> Result<Self> {
        info!("Creating hypertableau runner for satisfiability check");

        let mut expansion = HypertableauExpansion::new();

        // Extract concepts from the class expression
        let mut concepts = Self::extract_concepts_from_expression(class_expr);

        // Add ontology axioms as root concepts
        let mut root_concepts = Self::extract_root_concepts(ontology)?;
        concepts.append(&mut root_concepts);

        // Add disjointness constraints
        Self::add_disjointness_constraints(&mut expansion, ontology)?;

        // Initialize expansion
        expansion.initialize(concepts)?;

        Ok(Self {
            expansion,
            state: ExpansionState::Running,
            stats: ExpansionStatistics::default(),
            ontology_name: ontology
                .get_iri()
                .map_or_else(|| "anonymous".to_string(), |iri| iri.as_str().to_string()),
        })
    }

    /// Create a hypertableau runner for subsumption checking
    pub fn for_subsumption(
        ontology: &Ontology,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<Self> {
        info!("Creating hypertableau runner for subsumption check");

        // To check if A ⊑ B, we check if A ⊓ ¬B is unsatisfiable
        let mut expansion = HypertableauExpansion::new();

        // Extract concepts
        let mut concepts = Self::extract_concepts_from_expression(subclass);

        // Add negated superclass (simplified - actual implementation would need proper negation)
        let superclass_name = Self::extract_class_name(superclass);
        if let Some(name) = superclass_name {
            expansion.add_contradiction("Thing".to_string(), name.clone());
            concepts.push(name);
        }

        // Add ontology axioms
        let mut root_concepts = Self::extract_root_concepts(ontology)?;
        concepts.append(&mut root_concepts);

        // Add disjointness constraints
        Self::add_disjointness_constraints(&mut expansion, ontology)?;

        // Initialize expansion
        expansion.initialize(concepts)?;

        Ok(Self {
            expansion,
            state: ExpansionState::Running,
            stats: ExpansionStatistics::default(),
            ontology_name: ontology
                .get_iri()
                .map_or_else(|| "anonymous".to_string(), |iri| iri.as_str().to_string()),
        })
    }

    /// Extract root concepts from ontology axioms
    fn extract_root_concepts(ontology: &Ontology) -> Result<Vec<String>> {
        let mut concepts = Vec::new();

        // Extract concepts from SubClassOf axioms
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(axiom) => {
                    // Add subclass concepts
                    if let Some(name) = Self::extract_class_name(&axiom.subclass) {
                        concepts.push(name);
                    }
                    // Add superclass concepts
                    if let Some(name) = Self::extract_class_name(&axiom.superclass) {
                        concepts.push(name);
                    }
                }
                Axiom::EquivalentClasses(axiom) => {
                    for class in &axiom.classes {
                        if let Some(name) = Self::extract_class_name(class) {
                            concepts.push(name);
                        }
                    }
                }
                Axiom::ClassAssertion(axiom) => {
                    if let Some(name) = Self::extract_class_name(&axiom.class) {
                        concepts.push(name);
                    }
                }
                _ => {}
            }
        }

        // Deduplicate
        concepts.sort();
        concepts.dedup();

        Ok(concepts)
    }

    /// Extract concepts from a class expression
    fn extract_concepts_from_expression(expr: &ClassExpression) -> Vec<String> {
        let mut concepts = Vec::new();

        match expr {
            ClassExpression::Class(class) => {
                concepts.push(class.iri.as_str().to_string());
            }
            ClassExpression::ObjectIntersectionOf(exprs) => {
                for e in exprs {
                    concepts.extend(Self::extract_concepts_from_expression(e));
                }
            }
            ClassExpression::ObjectUnionOf(exprs) => {
                for e in exprs {
                    concepts.extend(Self::extract_concepts_from_expression(e));
                }
            }
            ClassExpression::ObjectComplementOf(e) => {
                concepts.extend(Self::extract_concepts_from_expression(e));
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => {
                concepts.extend(Self::extract_concepts_from_expression(filler));
            }
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                concepts.extend(Self::extract_concepts_from_expression(filler));
            }
            _ => {
                // Handle other cases as needed
            }
        }

        concepts
    }

    /// Extract class name from class expression
    fn extract_class_name(expr: &ClassExpression) -> Option<String> {
        match expr {
            ClassExpression::Class(class) => Some(class.iri.as_str().to_string()),
            _ => None,
        }
    }

    /// Add disjointness constraints from ontology
    fn add_disjointness_constraints(
        expansion: &mut HypertableauExpansion,
        ontology: &Ontology,
    ) -> Result<()> {
        for axiom in ontology.axioms() {
            if let Axiom::DisjointClasses(axiom) = axiom {
                // Add pairwise disjointness
                let class_names: Vec<String> = axiom
                    .classes
                    .iter()
                    .filter_map(Self::extract_class_name)
                    .collect();

                for i in 0..class_names.len() {
                    for j in (i + 1)..class_names.len() {
                        expansion.add_contradiction(class_names[i].clone(), class_names[j].clone());
                        trace!(
                            "Added disjointness: {} ⊓ {} ≡ ⊥",
                            class_names[i], class_names[j]
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the expansion statistics
    pub fn statistics(&self) -> &ExpansionStatistics {
        &self.stats
    }

    /// Get the current expansion state
    pub fn expansion_state(&self) -> &ExpansionState {
        &self.state
    }
}

impl super::tableau::TableauRunner for HypertableauRunner {
    /// Run the hypertableau expansion algorithm
    fn run(&mut self) -> Result<TableauState> {
        info!("Running hypertableau expansion for {}", self.ontology_name);

        // Run the expansion
        self.state = self.expansion.expand()?;

        // Copy statistics
        self.stats = self.expansion.statistics().clone();

        // Log statistics
        info!(
            "Hypertableau expansion completed: state={:?}, nodes={}, edges={}, reused={}, merges={}",
            self.state,
            self.stats.nodes_created,
            self.stats.edges_created,
            self.stats.nodes_reused,
            self.stats.merges_performed
        );

        // Convert expansion state to tableau state
        let tableau_state = match self.state {
            ExpansionState::Satisfiable => TableauState::Satisfiable,
            ExpansionState::Unsatisfiable => TableauState::Unsatisfiable,
            ExpansionState::Running | ExpansionState::Unknown => TableauState::Unknown,
        };

        Ok(tableau_state)
    }

    /// Get node count (nodes created)
    fn get_node_count(&self) -> usize {
        self.stats.nodes_created
    }

    /// Get backtrack count
    fn get_backtrack_count(&self) -> usize {
        self.stats.backtracks
    }

    /// Get maximum depth (use merge count as proxy)
    fn get_max_depth(&self) -> usize {
        self.stats.merges_performed
    }

    /// Check if consistent
    fn is_consistent(&self) -> bool {
        self.state == ExpansionState::Satisfiable
    }

    /// Check if completed
    fn is_completed(&self) -> bool {
        matches!(
            self.state,
            ExpansionState::Satisfiable | ExpansionState::Unsatisfiable
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_extract_concepts() {
        let class_a = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/A"),
        });
        let class_b = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/B"),
        });

        let intersection = ClassExpression::ObjectIntersectionOf(vec![class_a, class_b]);
        let concepts = HypertableauRunner::extract_concepts_from_expression(&intersection);

        assert_eq!(concepts.len(), 2);
        assert!(concepts.contains(&"http://example.org/A".to_string()));
        assert!(concepts.contains(&"http://example.org/B".to_string()));
    }

    #[test]
    fn test_extract_class_name() {
        let class = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Person"),
        });

        let name = HypertableauRunner::extract_class_name(&class);
        assert_eq!(name, Some("http://example.org/Person".to_string()));
    }

    #[test]
    fn test_empty_ontology() {
        let ontology = Ontology::new();
        let runner = HypertableauRunner::new(&ontology);

        assert!(runner.is_ok());
    }
}
