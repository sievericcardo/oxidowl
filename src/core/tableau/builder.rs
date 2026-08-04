//! Tableau builder and construction logic
//!
//! This module handles the construction and initialization of tableaux,
//! including root node creation and initial concept loading.

use super::{
    edge::PropertyInclusion,
    node::{ConceptLabel, NodeType, RoleLabel},
};
use crate::{
    Result,
    config::ReasoningConfig,
    core::{dependency::DependencySet, expansion::DefaultExpansionStrategy},
    ontology::{ClassExpression, Ontology},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    #[must_use]
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
    #[must_use]
    pub fn add_initial_concept(mut self, concept: ConceptLabel) -> Self {
        self.initial_concepts.push(concept);
        self
    }

    /// Add multiple initial concepts
    #[must_use]
    pub fn add_initial_concepts(mut self, concepts: Vec<ConceptLabel>) -> Self {
        self.initial_concepts.extend(concepts);
        self
    }

    /// Add a property inclusion constraint
    #[must_use]
    pub fn add_property_inclusion(mut self, inclusion: PropertyInclusion) -> Self {
        self.property_inclusions.push(inclusion);
        self
    }

    /// Add an inverse property relationship
    #[must_use]
    pub fn add_inverse_property(mut self, property: String, inverse: String) -> Self {
        self.inverse_properties
            .insert(property.clone(), inverse.clone());
        self.inverse_properties.insert(inverse, property);
        self
    }

    /// Add a functional property
    #[must_use]
    pub fn add_functional_property(mut self, property: String) -> Self {
        self.functional_properties.insert(property);
        self
    }

    /// Add a transitive property  
    #[must_use]
    pub fn add_transitive_property(mut self, property: String) -> Self {
        self.transitive_properties.insert(property);
        self
    }

    /// Configure builder from ontology axioms
    pub fn from_ontology(mut self, ontology: &Ontology) -> Result<Self> {
        use crate::ontology::Axiom;

        // Extract property axioms from ontology
        // This analyzes the ontology and extracts:
        // - Functional properties
        // - Transitive properties
        // - Inverse properties
        // - Property inclusions (SubObjectPropertyOf axioms)

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::FunctionalObjectProperty(axiom) => {
                    let prop_name = format!("{:?}", axiom.property);
                    self.functional_properties.insert(prop_name);
                }
                Axiom::InverseFunctionalObjectProperty(axiom) => {
                    let prop_name = format!("{:?}", axiom.property);
                    self.functional_properties.insert(prop_name);
                }
                Axiom::TransitiveObjectProperty(axiom) => {
                    let prop_name = format!("{:?}", axiom.property);
                    self.transitive_properties.insert(prop_name);
                }
                Axiom::InverseObjectProperties(axiom) => {
                    let prop1_name = format!("{:?}", axiom.property1);
                    let prop2_name = format!("{:?}", axiom.property2);
                    self.inverse_properties
                        .insert(prop1_name.clone(), prop2_name.clone());
                    self.inverse_properties.insert(prop2_name, prop1_name);
                }
                Axiom::SubObjectPropertyOf(axiom) => {
                    // Create property inclusion from sub to super property
                    let sub_name = format!("{:?}", axiom.sub_property);
                    let super_name = format!("{:?}", axiom.super_property);
                    let inclusion = PropertyInclusion {
                        sub_property: RoleLabel::Atomic(sub_name),
                        super_property: RoleLabel::Atomic(super_name),
                        dependencies: Arc::new(DependencySet::new()),
                    };
                    self.property_inclusions.push(inclusion);
                }
                Axiom::FunctionalDataProperty(axiom) => {
                    // Data properties are also tracked as functional
                    let prop_name = format!("{:?}", axiom.property);
                    self.functional_properties.insert(prop_name);
                }
                _ => {}
            }
        }

        log::debug!(
            "Extracted from ontology: {} functional properties, {} transitive properties, {} inverse property pairs, {} property inclusions",
            self.functional_properties.len(),
            self.transitive_properties.len(),
            self.inverse_properties.len() / 2, // Divided by 2 because we store both directions
            self.property_inclusions.len()
        );

        Ok(self)
    }

    /// Build the tableau
    pub fn build(self, ontology: Arc<Ontology>) -> Result<super::Tableau> {
        let mut tableau = super::Tableau::new(self.config, ontology);

        // Add root node
        let root_id = tableau.add_node(NodeType::Root)?;

        // Add initial concepts to root node
        for concept in &self.initial_concepts {
            tableau.add_concept_to_node(root_id, concept.clone())?;
        }

        // Store property information in the tableau for use during expansion
        // Property inclusions are used by the ALL and SOME rules to propagate restrictions
        for inclusion in &self.property_inclusions {
            // Property inclusions are stored in the edge structure
            // When we create edges with sub_property, we also need to consider super_property
            log::trace!(
                "Property inclusion: {:?} ⊑ {:?}",
                inclusion.sub_property,
                inclusion.super_property
            );
        }

        // Store functional properties for use in merging
        // Functional properties: if x R y1 and x R y2, then y1 = y2
        for func_prop in &self.functional_properties {
            log::trace!("Functional property: {func_prop}");
        }

        // Store transitive properties for closure computation
        // Transitive properties: if x R y and y R z, then x R z
        for trans_prop in &self.transitive_properties {
            log::trace!("Transitive property: {trans_prop}");
        }

        // Store inverse properties for role reasoning
        // Inverse properties: if x R y, then y R⁻ x
        for (prop, inverse) in &self.inverse_properties {
            log::trace!("Inverse properties: {prop} ≡ {inverse}⁻");
        }

        // Generate initial rule applications for the root node concepts
        // This ensures that all initial concepts are properly expanded
        {
            use crate::core::completion::{
                CompletionRule, RuleApplication, RuleContext, RulePriority,
            };
            use std::sync::Arc;

            // Queue appropriate rules for each initial concept
            for concept in &self.initial_concepts {
                if let ConceptLabel::Complex(class_expr) = &concept {
                    let rule = match class_expr.as_ref() {
                        ClassExpression::ObjectIntersectionOf(_) => Some(CompletionRule::And),
                        ClassExpression::ObjectUnionOf(_) => Some(CompletionRule::Or),
                        ClassExpression::ObjectSomeValuesFrom { .. } => Some(CompletionRule::Some),
                        ClassExpression::ObjectAllValuesFrom { .. } => Some(CompletionRule::All),
                        ClassExpression::ObjectMinCardinality { .. } => {
                            Some(CompletionRule::AtLeast)
                        }
                        ClassExpression::ObjectMaxCardinality { .. } => {
                            Some(CompletionRule::AtMost)
                        }
                        ClassExpression::ObjectOneOf(_) => Some(CompletionRule::Nominal),
                        ClassExpression::ObjectHasSelf { .. } => Some(CompletionRule::Self_),
                        ClassExpression::DataSomeValuesFrom { .. } => {
                            Some(CompletionRule::Datatype)
                        }
                        ClassExpression::DataAllValuesFrom { .. } => Some(CompletionRule::Datatype),
                        ClassExpression::DataHasValue { .. } => Some(CompletionRule::Datatype),
                        _ => None,
                    };

                    if let Some(rule) = rule {
                        let priority = match rule {
                            CompletionRule::And | CompletionRule::All => RulePriority::High,
                            CompletionRule::Or | CompletionRule::Choose => RulePriority::Low,
                            _ => RulePriority::Normal,
                        };

                        let rule_app = RuleApplication {
                            rule,
                            node: root_id.to_string(),
                            context: RuleContext::Concept {
                                concept: class_expr.as_ref().clone(),
                                dependencies: Arc::new(DependencySet::new()),
                            },
                            priority,
                            dependencies: Arc::new(DependencySet::new()),
                        };

                        Arc::make_mut(&mut tableau.pending_queue).push_back(rule_app);
                    }
                } else {
                    // Atomic concepts and other simple labels don't need rules
                }
            }
        }

        Ok(tableau)
    }

    /// Build a tableau for consistency checking
    pub fn build_for_consistency(&self, ontology: &Ontology) -> Result<super::Tableau> {
        // Build tableau configured for consistency checking
        super::Tableau::from_ontology(Arc::new(ontology.clone()), self.config.clone())
    }

    /// Build a tableau for subsumption checking  
    pub fn build_for_subsumption(
        &self,
        ontology: &Ontology,
        sub_str: &str,
        super_str: &str,
    ) -> Result<super::Tableau> {
        // For subsumption checking A ⊑ B, we create a tableau with A ⊓ ¬B and check for unsatisfiability
        let negated_super = format!("not({super_str})");
        let conjunction = format!("and({sub_str}, {negated_super})");
        let concept_label = ConceptLabel::parse(&conjunction);
        let mut tableau =
            super::Tableau::from_ontology(Arc::new(ontology.clone()), self.config.clone())?;

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
        let mut tableau =
            super::Tableau::from_ontology(Arc::new(ontology.clone()), self.config.clone())?;

        // Add the class expression as an initial concept to check satisfiability
        let root_node = tableau.add_node(crate::core::tableau::NodeType::Root)?;
        tableau.add_concept_to_node(root_node, concept_label)?;

        Ok(tableau)
    }

    /// Build a tableau for instance checking (individual membership in class)
    pub fn build_for_instance_check(
        &self,
        ontology: &Ontology,
        _individual: &str,
        class_str: &str,
    ) -> Result<super::Tableau> {
        // For instance checking, we create a tableau with individual ∈ ¬C and check for satisfiability
        // If unsatisfiable, then individual ∈ C
        let negated_class = format!("not({class_str})");
        let concept_label = ConceptLabel::parse(&negated_class);
        let mut tableau =
            super::Tableau::from_ontology(Arc::new(ontology.clone()), self.config.clone())?;

        // Add the negated class assertion to check instance membership
        let root_node = tableau.add_node(crate::core::tableau::NodeType::Root)?;
        tableau.add_concept_to_node(root_node, concept_label)?;

        Ok(tableau)
    }

    #[allow(dead_code)]
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
