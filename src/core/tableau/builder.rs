//! Tableau builder and construction logic
//!
//! This module handles the construction and initialization of tableaux,
//! including root node creation and initial concept loading.

use super::{
    edge::{PropertyInclusion, TableauEdge},
    node::{ConceptLabel, NodeId, NodeType, TableauNode},
    state::{ClashDetector, TableauState, TableauStatistics},
};
use crate::{
    Error, Result,
    config::ReasoningConfig,
    core::{
        blocking::BlockingChecker,
        completion::CompletionRuleSet,
        dependency::DependencyTracker,
        expansion::{DefaultExpansionStrategy, ExpansionStrategy},
    },
    ontology::{ClassExpression, Ontology},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Tableau builder for constructing configured tableau instances
#[derive(Debug)]
pub struct TableauBuilder {
    /// Configuration for reasoning
    config: ReasoningConfig,

    /// Initial concepts to add to root node
    initial_concepts: Vec<ConceptLabel>,

    /// Property inclusions to enforce
    property_inclusions: Vec<PropertyInclusion>,

    /// Inverse property mappings
    inverse_properties: HashMap<String, String>,

    /// Functional properties
    functional_properties: HashSet<String>,

    /// Transitive properties
    transitive_properties: HashSet<String>,
}

impl TableauBuilder {
    /// Create a new tableau builder
    pub fn new(config: ReasoningConfig) -> Self {
        Self {
            config,
            initial_concepts: Vec::new(),
            property_inclusions: Vec::new(),
            inverse_properties: HashMap::new(),
            functional_properties: HashSet::new(),
            transitive_properties: HashSet::new(),
        }
    }

    /// Add an initial concept to the root node
    pub fn add_initial_concept(mut self, concept: ConceptLabel) -> Self {
        self.initial_concepts.push(concept);
        self
    }

    /// Add multiple initial concepts
    pub fn add_initial_concepts(mut self, concepts: Vec<ConceptLabel>) -> Self {
        self.initial_concepts.extend(concepts);
        self
    }

    /// Add a property inclusion constraint
    pub fn add_property_inclusion(mut self, inclusion: PropertyInclusion) -> Self {
        self.property_inclusions.push(inclusion);
        self
    }

    /// Add an inverse property relationship
    pub fn add_inverse_property(mut self, property: String, inverse: String) -> Self {
        self.inverse_properties
            .insert(property.clone(), inverse.clone());
        self.inverse_properties.insert(inverse, property);
        self
    }

    /// Add a functional property
    pub fn add_functional_property(mut self, property: String) -> Self {
        self.functional_properties.insert(property);
        self
    }

    /// Add a transitive property  
    pub fn add_transitive_property(mut self, property: String) -> Self {
        self.transitive_properties.insert(property);
        self
    }

    /// Configure builder from ontology axioms
    pub fn from_ontology(mut self, ontology: &Ontology) -> Result<Self> {
        // Extract property axioms from ontology
        // This would analyze the ontology and extract:
        // - Functional properties
        // - Transitive properties
        // - Inverse properties
        // - Property inclusions (SubObjectPropertyOf axioms)

        // For now, just return self - full implementation would
        // parse all relevant axioms from the ontology

        // Example of what this might do:
        // for axiom in ontology.axioms() {
        //     match axiom {
        //         ObjectPropertyAxiom::FunctionalObjectProperty(prop) => {
        //             self.functional_properties.insert(prop.to_string());
        //         }
        //         ObjectPropertyAxiom::TransitiveObjectProperty(prop) => {
        //             self.transitive_properties.insert(prop.to_string());
        //         }
        //         ObjectPropertyAxiom::InverseObjectProperties(prop1, prop2) => {
        //             self.inverse_properties.insert(prop1.to_string(), prop2.to_string());
        //             self.inverse_properties.insert(prop2.to_string(), prop1.to_string());
        //         }
        //         ObjectPropertyAxiom::SubObjectPropertyOf(sub, super_) => {
        //             let inclusion = PropertyInclusion::new(sub.to_string(), super_.to_string());
        //             self.property_inclusions.push(inclusion);
        //         }
        //         _ => {}
        //     }
        // }

        Ok(self)
    }

    /// Build the tableau
    pub fn build(self) -> Result<super::Tableau> {
        let mut tableau = super::Tableau::new(self.config);

        // Add root node
        let root_id = tableau.add_node(NodeType::Root)?;

        // Add initial concepts to root node
        for concept in self.initial_concepts {
            tableau.add_concept_to_node(root_id, concept)?;
        }

        // TODO: Add property inclusions, functional properties, etc.

        Ok(tableau)
    }

    /// Build a tableau for consistency checking
    pub fn build_for_consistency(&self, ontology: &Ontology) -> Result<super::Tableau> {
        // Build tableau configured for consistency checking
        super::Tableau::from_ontology(ontology, self.config.clone())
    }

    /// Build a tableau for subsumption checking  
    pub fn build_for_subsumption(
        &self,
        ontology: &Ontology,
        sub_str: &str,
        super_str: &str,
    ) -> Result<super::Tableau> {
        // For subsumption checking A ⊑ B, we create a tableau with A ⊓ ¬B and check for unsatisfiability
        let negated_super = format!("not({})", super_str);
        let conjunction = format!("and({}, {})", sub_str, negated_super);
        let concept_label = ConceptLabel::parse(&conjunction);
        let mut tableau = super::Tableau::from_ontology(ontology, self.config.clone())?;

        // Add the conjunction A ⊓ ¬B to check subsumption
        let root_node = tableau.add_node(crate::core::tableau::NodeType::Root)?;
        tableau.add_concept_to_node(root_node, concept_label)?;

        Ok(tableau)
    }

    /// Build a tableau for satisfiability checking of a class expression
    pub fn build_for_satisfiability(
        &self,
        ontology: &Ontology,
        class_str: &str,
    ) -> Result<super::Tableau> {
        // Parse the class expression and build tableau
        let concept_label = ConceptLabel::parse(class_str);
        let mut tableau = super::Tableau::from_ontology(ontology, self.config.clone())?;

        // Add the class expression as an initial concept to check satisfiability
        let root_node = tableau.add_node(crate::core::tableau::NodeType::Root)?;
        tableau.add_concept_to_node(root_node, concept_label)?;

        Ok(tableau)
    }

    /// Build a tableau for instance checking (individual membership in class)
    pub fn build_for_instance_check(
        &self,
        ontology: &Ontology,
        individual: &str,
        class_str: &str,
    ) -> Result<super::Tableau> {
        // For instance checking, we create a tableau with individual ∈ ¬C and check for satisfiability
        // If unsatisfiable, then individual ∈ C
        let negated_class = format!("not({})", class_str);
        let concept_label = ConceptLabel::parse(&negated_class);
        let mut tableau = super::Tableau::from_ontology(ontology, self.config.clone())?;

        // Add the negated class assertion to check instance membership
        let root_node = tableau.add_node(crate::core::tableau::NodeType::Root)?;
        tableau.add_concept_to_node(root_node, concept_label)?;

        Ok(tableau)
    }

    fn create_expansion_strategy(&self) -> Result<DefaultExpansionStrategy> {
        // Create expansion strategy based on configuration
        // For now, we'll just use the default expansion strategy
        Ok(DefaultExpansionStrategy::default())
    }

    /// Static constructor for consistency checking
    pub fn for_consistency_check(
        ontology: &Ontology,
        config: ReasoningConfig,
    ) -> Result<super::Tableau> {
        TableauBuilder::new(config)
            .from_ontology(ontology)?
            .build_for_consistency(ontology)
    }

    /// Static constructor for subsumption checking
    pub fn for_subsumption_check(
        ontology: &Ontology,
        config: ReasoningConfig,
        sub: &ClassExpression,
        super_: &ClassExpression,
    ) -> Result<super::Tableau> {
        let sub_str = &format!("{sub}");
        let super_str = &format!("{super_}");
        TableauBuilder::new(config)
            .from_ontology(ontology)?
            .build_for_subsumption(ontology, sub_str, super_str)
    }

    /// Static constructor for classification  
    pub fn for_classification(
        ontology: &Ontology,
        config: ReasoningConfig,
    ) -> Result<super::Tableau> {
        // Classification would build a tableau suitable for computing class hierarchy
        TableauBuilder::new(config)
            .from_ontology(ontology)?
            .build_for_consistency(ontology)
    }
}
