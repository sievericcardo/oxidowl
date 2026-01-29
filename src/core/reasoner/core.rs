//! Core reasoner functionality
//!
//! This module contains the main Reasoner struct and core operations like
//! loading ontologies and basic reasoning setup.

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::{
        lock_helpers::{read_lock, write_lock},
        reasoner::{
            classification::ClassificationService,
            explanation::ExplanationService,
            queries::QueryProcessor,
            results::{ClassificationResult, RealizationResult},
            statistics::ReasoningStatistics,
            tableau::TableauFactory,
            tasks::ReasoningTaskService,
        },
        tableau::TableauState,
    },
    dl_clauses::DLClauseGenerator,
    ontology::{
        ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, OntologyRef,
    },
};
use serde_json;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// OWLlink request structure
#[derive(Debug, Clone)]
struct OWLlinkRequest {
    /// Command name (Tell, IsClassSatisfiable, etc.)
    command: String,
    /// Axioms to add (for Tell command)
    axioms: Vec<crate::ontology::axioms::Axiom>,
    /// Class IRI for class-related queries
    class_iri: Option<url::Url>,
    /// Individual IRI for individual-related queries  
    individual_iri: Option<url::Url>,
    /// Direct flag for hierarchical queries
    direct: Option<bool>,
}

/// SPARQL query structure
#[derive(Debug, Clone)]
struct SparqlQuery {
    /// Query type (SELECT, ASK, CONSTRUCT, DESCRIBE, INSERT, DELETE)
    query_type: String,
    /// Variables to select
    variables: Vec<String>,
    /// Triple patterns in WHERE clause
    patterns: Vec<TriplePattern>,
    /// Triple patterns to insert (for INSERT operations)
    insert_patterns: Vec<TriplePattern>,
    /// Triple patterns to delete (for DELETE operations)
    delete_patterns: Vec<TriplePattern>,
    /// Original query string
    original_query: String,
}

/// Triple pattern in SPARQL query
#[derive(Debug, Clone)]
struct TriplePattern {
    /// Subject (variable or IRI)
    subject: String,
    /// Predicate (variable or IRI)
    predicate: String,
    /// Object (variable or IRI)
    object: String,
}

/// Main reasoner interface
#[derive(Debug)]
pub struct Reasoner {
    /// Reasoning configuration
    config: ReasonerConfig,

    /// Current ontology being reasoned over
    ontology: Option<OntologyRef>,

    /// Cache manager for reasoning results
    cache_manager: Arc<RwLock<CacheManager>>,

    /// Tableau factory for creating reasoning algorithms
    tableau_factory: TableauFactory,

    /// Statistics about reasoning operations
    statistics: ReasoningStatistics,

    /// Task service for basic reasoning operations
    task_service: ReasoningTaskService,

    /// Classification service for complex operations
    classification_service: ClassificationService,

    /// Query processor for SPARQL and OWLlink
    query_processor: QueryProcessor,

    /// Explanation service
    explanation_service: ExplanationService,
}

impl Reasoner {
    /// Create a new reasoner instance with the given configuration
    pub fn new(config: crate::config::ReasonerConfig) -> Result<Self> {
        let cache_manager = Arc::new(RwLock::new(CacheManager::default()));
        let tableau_factory = TableauFactory::new(config.clone())?;

        // Create individual tableau factories for each service
        let task_tableau_factory = TableauFactory::new(config.clone())?;
        let classification_tableau_factory = TableauFactory::new(config.clone())?;

        let task_service = ReasoningTaskService::new(task_tableau_factory, cache_manager.clone());
        let classification_service = ClassificationService::new(
            ReasoningTaskService::new(classification_tableau_factory, cache_manager.clone()),
            cache_manager.clone(),
        );

        Ok(Self {
            ontology: None,
            config: config.clone(),
            cache_manager,
            tableau_factory,
            statistics: ReasoningStatistics::default(),
            task_service,
            classification_service,
            query_processor: QueryProcessor::new(),
            explanation_service: ExplanationService::new(),
        })
    }

    /// Check if a class is a subclass of another
    pub fn is_subclass_of(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        // Enhanced subsumption checking using available reasoning mechanisms

        // Quick syntactic check
        if subclass == superclass {
            return Ok(true);
        }

        // Check for explicit subclass declarations in the ontology
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(ontology, "core: reading ontology for is_subclass_of")?;
            for axiom in ontology_ref.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if &subclass_axiom.subclass == subclass
                        && &subclass_axiom.superclass == superclass
                    {
                        return Ok(true);
                    }
                }
            }

            // Check through equivalent classes
            for axiom in ontology_ref.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(subclass)
                        && equiv_axiom.classes.contains(superclass)
                    {
                        return Ok(true);
                    }
                }
            }
        }

        // Check using built-in OWL semantics
        self.check_semantic_subsumption(subclass, superclass)
    }

    /// Enhanced semantic subsumption checking with proper OWL semantics
    fn check_semantic_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        match (subclass, superclass) {
            // Bottom is subclass of everything
            (ClassExpression::Class(class), _)
                if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" =>
            {
                Ok(true)
            }

            // Everything is subclass of Top
            (_, ClassExpression::Class(class))
                if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" =>
            {
                Ok(true)
            }

            // Nothing is superclass of Top (except Top itself)
            (ClassExpression::Class(subclass_class), ClassExpression::Class(superclass_class))
                if subclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing"
                    && superclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" =>
            {
                Ok(false)
            }

            // Intersection subsumption: A ⊓ B ⊑ A and A ⊓ B ⊑ B
            (ClassExpression::ObjectIntersectionOf(components), superclass) => {
                Ok(components.contains(superclass))
            }

            // Union subsumption: A ⊑ A ⊔ B and B ⊑ A ⊔ B
            (subclass, ClassExpression::ObjectUnionOf(components)) => {
                Ok(components.contains(subclass))
            }

            _ => Ok(false),
        }
    }

    /// Get the ontology being reasoned over
    pub fn get_ontology(&self) -> Option<&OntologyRef> {
        self.ontology.as_ref()
    }

    /// Check if the ontology is consistent
    pub fn is_consistent(&self) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(ontology, "core: reading ontology for consistency check")?;

            // Pre-consistency check (fast detection of certain inconsistencies)
            log::info!("Running pre-consistency check");
            let mut pre_checker =
                crate::core::reasoner::PreConsistencyChecker::new(&*ontology_ref)?;

            // If pre-check detects inconsistency, return early
            match pre_checker.check() {
                Err(_) => {
                    log::info!("Pre-consistency check detected inconsistency");
                    return Ok(false);
                }
                Ok(_) => {
                    log::info!("Pre-consistency check passed, proceeding to tableau");
                }
            }

            // PHASE 2: Tableau-based consistency checking (if pre-check passed)
            let mut tableau = self
                .tableau_factory
                .create_for_consistency(&*ontology_ref)?;

            // Run the tableau algorithm
            let state = tableau.run()?;

            // Update statistics
            let node_count = tableau.get_node_count();
            let backtrack_count = tableau.get_backtrack_count();

            Ok(state == TableauState::Satisfiable)
        } else {
            Ok(true) // Empty ontology is consistent
        }
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        // owl:Nothing is always unsatisfiable by definition
        if let ClassExpression::Class(c) = class {
            if c.is_nothing() {
                return Ok(false);
            }
        }

        if let Some(ontology) = &self.ontology {
            // Create a tableau for satisfiability checking
            let ontology_ref =
                read_lock(ontology, "core: reading ontology for satisfiability check")?;
            let tableau = self
                .tableau_factory
                .create_for_consistency(&*ontology_ref)?;

            // Create a test individual with the class to check
            let test_individual = crate::ontology::Individual::named(crate::ontology::IRI::new(
                "http://example.org/test#testIndividual",
            ));

            // Add the class assertion to test satisfiability
            let test_assertion = crate::ontology::ClassAssertionAxiom {
                id: 0, // Generate a proper ID
                class: class.clone(),
                individual: test_individual,
                annotations: Vec::new(),
            };

            // Create a temporary ontology with the test assertion
            let mut test_ontology =
                read_lock(ontology, "core: reading ontology to clone for test")?.clone();
            test_ontology.add_axiom(crate::ontology::Axiom::ClassAssertion(test_assertion));

            // Initialize a new tableau with the test ontology
            let mut test_tableau = self
                .tableau_factory
                .create_for_consistency(&test_ontology)?;

            // Run the tableau algorithm
            let state = test_tableau.run()?;

            Ok(state == TableauState::Satisfiable)
        } else {
            // In empty ontology, all classes are satisfiable (except owl:Nothing, already handled)
            Ok(true)
        }
    }

    /// Check if one class is subsumed by another
    pub fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        self.is_subclass_of(subclass, superclass)
    }

    /// Get superclasses of a class
    pub fn get_superclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_superclasses")?;
            let mut superclasses = Vec::new();

            // Check all class expressions in the ontology for subsumption
            for axiom in ontology.axioms() {
                match axiom {
                    crate::ontology::Axiom::SubClassOf(subclass_axiom) => {
                        // If the subclass is our target class, the superclass is a superclass
                        if self.classes_equivalent(&subclass_axiom.subclass, class)? {
                            superclasses.push(subclass_axiom.superclass.clone());
                        }
                    }
                    crate::ontology::Axiom::EquivalentClasses(equiv_axiom) => {
                        // For equivalent classes, all others are both sub and superclasses
                        if equiv_axiom
                            .classes
                            .iter()
                            .any(|c| self.classes_equivalent(c, class).unwrap_or(false))
                        {
                            for other_class in &equiv_axiom.classes {
                                if !self.classes_equivalent(other_class, class).unwrap_or(false) {
                                    superclasses.push(other_class.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Always add owl:Thing as superclass (unless the class is owl:Thing itself)
            let owl_thing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_thing(),
            ));
            if !self.classes_equivalent(class, &owl_thing).unwrap_or(false) {
                superclasses.push(owl_thing);
            }

            if direct {
                // Filter to only direct superclasses
                self.filter_direct_superclasses(superclasses)
            } else {
                // Use inference to find all inferred superclasses
                self.get_all_inferred_superclasses(class, superclasses)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Get subclasses of a class
    pub fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_subclasses")?;
            let mut subclasses = Vec::new();

            // Check all class expressions in the ontology for subsumption
            for axiom in ontology.axioms() {
                match axiom {
                    crate::ontology::Axiom::SubClassOf(subclass_axiom) => {
                        // If the superclass is our target class, the subclass is a subclass
                        if self.classes_equivalent(&subclass_axiom.superclass, class)? {
                            subclasses.push(subclass_axiom.subclass.clone());
                        }
                    }
                    crate::ontology::Axiom::EquivalentClasses(equiv_axiom) => {
                        // For equivalent classes, all others are both sub and superclasses
                        if equiv_axiom
                            .classes
                            .iter()
                            .any(|c| self.classes_equivalent(c, class).unwrap_or(false))
                        {
                            for other_class in &equiv_axiom.classes {
                                if !self.classes_equivalent(other_class, class).unwrap_or(false) {
                                    subclasses.push(other_class.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Always add owl:Nothing as subclass (unless the class is owl:Nothing itself)
            let owl_nothing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ));
            if !self
                .classes_equivalent(class, &owl_nothing)
                .unwrap_or(false)
            {
                subclasses.push(owl_nothing);
            }

            if direct {
                // Filter to only direct subclasses
                self.filter_direct_subclasses(subclasses)
            } else {
                // Use inference to find all inferred subclasses
                self.get_all_inferred_subclasses(class, subclasses)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Get equivalent classes
    pub fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_equivalent_classes",
            )?;
            let mut equivalent_classes = Vec::new();

            // Check explicit equivalent class axioms
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom
                        .classes
                        .iter()
                        .any(|c| self.classes_equivalent(c, class).unwrap_or(false))
                    {
                        for other_class in &equiv_axiom.classes {
                            if !self.classes_equivalent(other_class, class).unwrap_or(false) {
                                equivalent_classes.push(other_class.clone());
                            }
                        }
                    }
                }
            }

            // Check for implicit equivalences via bidirectional subsumption
            let all_classes = self.get_all_classes_in_ontology_internal()?;
            for other_class in all_classes {
                if !self
                    .classes_equivalent(&other_class, class)
                    .unwrap_or(false)
                {
                    // Check if both A ⊑ B and B ⊑ A
                    if self.is_subclass_of(class, &other_class)?
                        && self.is_subclass_of(&other_class, class)?
                    {
                        equivalent_classes.push(other_class);
                    }
                }
            }

            Ok(equivalent_classes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get instances of a class
    pub fn get_instances(&self, class: &ClassExpression, direct: bool) -> Result<Vec<Individual>> {
        if let Some(ontology_ref) = &self.ontology {
            // Use the classification service to properly handle datatype reasoning
            let mut statistics = ReasoningStatistics::new();
            let instances = self.classification_service.get_instances(
                class,
                ontology_ref,
                &mut statistics,
                direct,
            )?;

            Ok(instances)
        } else {
            Ok(Vec::new())
        }
    }

    /// Check if an individual is an instance of a class expression
    pub fn is_instance_of(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for is_instance_of")?;
            self.classification_service
                .check_instance_with_datatype_reasoning(individual, class, &ontology)
        } else {
            Ok(false)
        }
    }

    /// Get types of an individual
    pub fn get_types(&self, individual: &Individual, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_types")?;
            let mut types = Vec::new();

            // Look for explicit class assertions
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(class_assertion) = axiom {
                    if class_assertion.individual.iri() == individual.iri() {
                        types.push(class_assertion.class.clone());
                    }
                }
            }

            if !direct {
                // Also add superclasses of the direct types
                let mut all_types = types.clone();
                for direct_type in &types {
                    let superclasses = self.get_superclasses(direct_type, false)?;
                    all_types.extend(superclasses);
                }
                types = all_types;
            }

            // Remove duplicates
            types.sort_by_key(|c| format!("{:?}", c));
            types.dedup_by_key(|c| format!("{:?}", c));

            Ok(types)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<Individual>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_object_property_values",
            )?;
            let mut values = Vec::new();

            // Look for explicit object property assertions
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = axiom {
                    if prop_assertion.source.iri() == individual.iri() {
                        // Check if the property matches (need to handle property expressions)
                        if self.object_properties_equivalent(&prop_assertion.property, property)? {
                            values.push(prop_assertion.target.clone());
                        }
                    }
                }
            }

            // Remove duplicates
            values.sort_by_key(|i| i.iri().map(|iri| iri.to_string()).unwrap_or_default());
            values.dedup_by_key(|i| i.iri().map(|iri| iri.to_string()).unwrap_or_default());

            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<Vec<crate::ontology::Literal>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_data_property_values",
            )?;
            let mut values = Vec::new();

            // Look for explicit data property assertions
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::DataPropertyAssertion(prop_assertion) = axiom {
                    if prop_assertion.individual.iri() == individual.iri() {
                        // Check if the property matches
                        if self.data_properties_equivalent(&prop_assertion.property, property)? {
                            values.push(prop_assertion.value.clone());
                        }
                    }
                }
            }

            // Remove duplicates
            values.sort_by_key(|l| format!("{:?}", l));
            values.dedup_by_key(|l| format!("{:?}", l));

            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }

    /// Classify the ontology (compute class hierarchy)
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        if let Some(ontology) = &self.ontology {
            let mut statistics = ReasoningStatistics::new();
            self.classification_service
                .classify(ontology, &mut statistics)
        } else {
            Err(Error::ontology_parsing(
                "No ontology loaded for classification",
            ))
        }
    }

    /// Realize the ontology (compute instance relationships)
    pub fn realize(&mut self) -> Result<RealizationResult> {
        if let Some(ontology) = &self.ontology {
            let mut statistics = ReasoningStatistics::new();
            self.classification_service
                .realize(ontology, &mut statistics)
        } else {
            Err(Error::ontology_parsing(
                "No ontology loaded for realization",
            ))
        }
    }

    /// Check if an axiom is entailed by the ontology
    pub fn check_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
        ontology: &Arc<RwLock<crate::ontology::Ontology>>,
        stats: &mut ReasoningStatistics,
    ) -> Result<bool> {
        self.task_service.check_entailment(axiom, ontology, stats)
    }

    /// Explain why an entailment holds
    pub fn explain_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
    ) -> Result<Vec<crate::ontology::Axiom>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for explain_entailment",
            )?;

            // Simple explanation: find axioms that directly support the entailment
            let mut explanation = Vec::new();

            match axiom {
                crate::ontology::Axiom::SubClassOf(subclass_axiom) => {
                    // Look for axioms that support this subsumption
                    for ont_axiom in ontology.axioms() {
                        match ont_axiom {
                            crate::ontology::Axiom::SubClassOf(ont_subclass) => {
                                // Direct support
                                if ont_subclass.subclass == subclass_axiom.subclass
                                    && ont_subclass.superclass == subclass_axiom.superclass
                                {
                                    explanation.push(ont_axiom.clone());
                                }
                                // Transitive support: A ⊑ B, B ⊑ C → A ⊑ C
                                else if ont_subclass.superclass == subclass_axiom.subclass {
                                    explanation.push(ont_axiom.clone());
                                } else if ont_subclass.subclass == subclass_axiom.superclass {
                                    explanation.push(ont_axiom.clone());
                                }
                            }
                            crate::ontology::Axiom::EquivalentClasses(equiv) => {
                                // Equivalence support
                                if equiv.classes.contains(&subclass_axiom.subclass)
                                    || equiv.classes.contains(&subclass_axiom.superclass)
                                {
                                    explanation.push(ont_axiom.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                crate::ontology::Axiom::ClassAssertion(class_assertion) => {
                    // Look for axioms that support this class assertion
                    for ont_axiom in ontology.axioms() {
                        match ont_axiom {
                            crate::ontology::Axiom::ClassAssertion(ont_assertion) => {
                                if ont_assertion.individual == class_assertion.individual
                                    && ont_assertion.class == class_assertion.class
                                {
                                    explanation.push(ont_axiom.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    // For other axiom types, just return the axiom itself if it exists
                    for ont_axiom in ontology.axioms() {
                        if ont_axiom == axiom {
                            explanation.push(ont_axiom.clone());
                            break;
                        }
                    }
                }
            }

            Ok(explanation)
        } else {
            Ok(Vec::new())
        }
    }

    /// Explain why the ontology is inconsistent
    pub fn explain_inconsistency(&self) -> Result<Vec<crate::ontology::Axiom>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for explain_inconsistency",
            )?;
            let mut explanation = Vec::new();

            // Look for obvious inconsistencies
            for axiom in ontology.axioms() {
                match axiom {
                    crate::ontology::Axiom::DisjointClasses(disjoint) => {
                        // Check if any individual is asserted to be in disjoint classes
                        for class1 in &disjoint.classes {
                            for class2 in &disjoint.classes {
                                if class1 != class2 {
                                    // Look for individuals in both classes
                                    let instances1 = self.get_instances(class1, true)?;
                                    let instances2 = self.get_instances(class2, true)?;

                                    for ind1 in &instances1 {
                                        for ind2 in &instances2 {
                                            if ind1.iri() == ind2.iri() {
                                                explanation.push(axiom.clone());
                                                // Also add the class assertions
                                                for ont_axiom in ontology.axioms() {
                                                    if let crate::ontology::Axiom::ClassAssertion(
                                                        assertion,
                                                    ) = ont_axiom
                                                    {
                                                        if assertion.individual.iri() == ind1.iri()
                                                            && (assertion.class == *class1
                                                                || assertion.class == *class2)
                                                        {
                                                            explanation.push(ont_axiom.clone());
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    crate::ontology::Axiom::ClassAssertion(assertion) => {
                        // Check if individual is asserted to be in owl:Nothing
                        if let ClassExpression::Class(class) = &assertion.class {
                            if class.iri.to_string() == "http://www.w3.org/2002/07/owl#Nothing" {
                                explanation.push(axiom.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }

            Ok(explanation)
        } else {
            Ok(Vec::new())
        }
    }

    /// Add an axiom to the ontology
    pub fn add_axiom(&mut self, axiom: crate::ontology::Axiom) -> Result<()> {
        if let Some(ontology_ref) = &mut self.ontology {
            let mut ontology = write_lock(ontology_ref, "core: writing ontology to add axiom")?;
            ontology.add_axiom(axiom);

            // Clear cache since ontology has changed
            let cache = write_lock(&self.cache_manager, "core: clearing cache after add_axiom")?;
            cache.clear_all();

            Ok(())
        } else {
            // Create new ontology with this axiom
            let mut ontology = crate::ontology::Ontology::new();
            ontology.add_axiom(axiom);
            self.ontology = Some(Arc::new(RwLock::new(ontology)));
            Ok(())
        }
    }

    /// Remove an axiom from the ontology
    pub fn remove_axiom(&mut self, axiom: &crate::ontology::Axiom) -> Result<bool> {
        if let Some(ontology_ref) = &mut self.ontology {
            let mut ontology = write_lock(ontology_ref, "core: writing ontology to remove axiom")?;
            let original_count = ontology.axioms().len();
            ontology.remove_axiom(axiom);
            let removed = ontology.axioms().len() < original_count;

            if removed {
                // Clear cache since ontology has changed
                let cache = write_lock(
                    &self.cache_manager,
                    "core: clearing cache after remove_axiom",
                )?;
                cache.clear_all();
            }

            // Return the boolean result properly wrapped
            Ok(removed)
        } else {
            Ok(false) // No ontology to remove from
        }
    }

    /// Get reasoning statistics
    pub fn get_statistics(&self) -> ReasoningStatistics {
        self.statistics.clone()
    }

    /// Get the size of the current ontology
    pub fn get_ontology_size(&self) -> usize {
        if let Some(ref ontology) = self.ontology {
            read_lock(ontology, "core: reading ontology for size")
                .map(|guard| guard.axioms().len())
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Load an ontology into the reasoner
    pub fn load_ontology(&mut self, ontology: crate::ontology::Ontology) -> Result<()> {
        self.ontology = Some(Arc::new(RwLock::new(ontology)));
        Ok(())
    }

    /// Load ontology from file
    pub fn load_ontology_from_file<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        format: crate::ontology::OntologyFormat,
    ) -> Result<()> {
        use crate::parsers::*;

        let file_path = path.as_ref();
        let content = std::fs::read_to_string(file_path).map_err(|e| crate::Error::Io {
            message: format!("Failed to read file: {}", e),
        })?;

        // Extract first section if CrossSyntax multi-format file
        let parsed_content = extract_first_crosssyntax_section(&content);

        let ontology = match format {
            crate::ontology::OntologyFormat::Functional => {
                let parser = functional::FunctionalParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Manchester => {
                let parser = manchester::ManchesterParser::new(
                    manchester::ManchesterParserConfig::default(),
                );
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Turtle => {
                let parser = turtle::TurtleParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::RdfXml => {
                let parser = rdf_xml::RdfXmlParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::OwlXml => {
                let parser = owl_xml::OwlXmlParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::NTriples => {
                let parser = ntriples::NTriplesParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Auto => {
                // Try to determine format from file extension
                let extension = file_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("");

                match extension.to_lowercase().as_str() {
                    "owl" | "owx" => {
                        let parser = owl_xml::OwlXmlParser::new();
                        parser.parse(&parsed_content)?
                    }
                    "xml" | "rdf" => {
                        // Try to detect XML type from content
                        if parsed_content.trim_start().starts_with("<?xml")
                            || parsed_content.contains("owl:Ontology")
                            || parsed_content.contains("<Ontology")
                        {
                            let parser = owl_xml::OwlXmlParser::new();
                            parser.parse(&parsed_content)?
                        } else {
                            let parser = rdf_xml::RdfXmlParser::new();
                            parser.parse(&parsed_content)?
                        }
                    }
                    "ttl" => {
                        let parser = turtle::TurtleParser::new();
                        parser.parse(&parsed_content)?
                    }
                    "ofn" => {
                        let parser = functional::FunctionalParser::new();
                        parser.parse(&parsed_content)?
                    }
                    "omn" | "man" => {
                        let parser = manchester::ManchesterParser::new(
                            manchester::ManchesterParserConfig::default(),
                        );
                        parser.parse(&parsed_content)?
                    }
                    "swrl" => {
                        // SWRL uses functional-like syntax
                        let parser = functional::FunctionalParser::new();
                        parser.parse(&parsed_content)?
                    }
                    "nt" => {
                        let parser = ntriples::NTriplesParser::new();
                        parser.parse(&parsed_content)?
                    }
                    "txt" => {
                        // Content-based detection for .txt files
                        let trimmed = parsed_content.trim();
                        if trimmed.starts_with("Ontology(") || trimmed.starts_with("Prefix(") {
                            let parser = functional::FunctionalParser::new();
                            parser.parse(&parsed_content)?
                        } else if trimmed.starts_with("Prefix:")
                            || trimmed.starts_with("Ontology:")
                            || trimmed.starts_with("Class:")
                        {
                            let parser = manchester::ManchesterParser::new(
                                manchester::ManchesterParserConfig::default(),
                            );
                            parser.parse(&parsed_content)?
                        } else if trimmed.starts_with("@prefix") || trimmed.starts_with("@base") {
                            let parser = turtle::TurtleParser::new();
                            parser.parse(&parsed_content)?
                        } else if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
                            let parser = owl_xml::OwlXmlParser::new();
                            parser.parse(&parsed_content)?
                        } else {
                            // Default to functional
                            let parser = functional::FunctionalParser::new();
                            parser.parse(&parsed_content)?
                        }
                    }
                    _ => {
                        // Default to OWL/XML
                        let parser = owl_xml::OwlXmlParser::new();
                        parser.parse(&parsed_content)?
                    }
                }
            }
        };

        self.load_ontology(ontology)
    }

    /// Classify object properties
    pub fn classify_object_properties(
        &mut self,
    ) -> Result<super::results::PropertyClassificationResult> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(
                ontology,
                "core: reading ontology for classify_object_properties",
            )?;
            let mut hierarchy = std::collections::HashMap::new();

            // Extract all object properties from the ontology
            let mut object_properties = std::collections::HashSet::new();
            for axiom in ontology_ref.axioms() {
                self.extract_object_properties_from_axiom(axiom, &mut object_properties);
            }

            // Build hierarchy based on SubObjectPropertyOf axioms
            for property in &object_properties {
                let mut superproperties = std::collections::HashSet::new();

                for axiom in ontology_ref.axioms() {
                    if let crate::ontology::Axiom::SubObjectPropertyOf(axiom) = axiom {
                        if self.object_properties_equivalent(&axiom.sub_property, property)? {
                            superproperties.insert(axiom.super_property.clone());
                        }
                    }
                }

                hierarchy.insert(property.clone(), superproperties);
            }

            Ok(super::results::PropertyClassificationResult::new_object_properties(hierarchy))
        } else {
            Ok(
                super::results::PropertyClassificationResult::new_object_properties(
                    std::collections::HashMap::new(),
                ),
            )
        }
    }

    /// Classify data properties  
    pub fn classify_data_properties(
        &mut self,
    ) -> Result<super::results::PropertyClassificationResult> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(
                ontology,
                "core: reading ontology for classify_data_properties",
            )?;
            let mut hierarchy = std::collections::HashMap::new();

            // Extract all data properties from the ontology
            let mut data_properties = std::collections::HashSet::new();
            for axiom in ontology_ref.axioms() {
                self.extract_data_properties_from_axiom(axiom, &mut data_properties);
            }

            // Build hierarchy based on SubDataPropertyOf axioms
            for property in &data_properties {
                let mut superproperties = std::collections::HashSet::new();

                for axiom in ontology_ref.axioms() {
                    if let crate::ontology::Axiom::SubDataPropertyOf(axiom) = axiom {
                        if self.data_properties_equivalent(&axiom.sub_property, property)? {
                            superproperties.insert(axiom.super_property.clone());
                        }
                    }
                }

                hierarchy.insert(property.clone(), superproperties);
            }

            Ok(super::results::PropertyClassificationResult::new_data_properties(hierarchy))
        } else {
            Ok(
                super::results::PropertyClassificationResult::new_data_properties(
                    std::collections::HashMap::new(),
                ),
            )
        }
    }

    /// Get unsatisfiable classes
    pub fn get_unsatisfiable_classes(&self) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_unsatisfiable_classes",
            )?;
            let mut unsatisfiable = Vec::new();

            // Get all classes in the ontology
            let all_classes = self.get_all_classes_in_ontology_internal()?;

            // Check each class for satisfiability
            for class in all_classes {
                if !self.is_class_satisfiable(&class)? {
                    unsatisfiable.push(class);
                }
            }

            // Always include owl:Nothing if not already included
            let owl_nothing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ));
            if !unsatisfiable
                .iter()
                .any(|c| self.classes_equivalent(c, &owl_nothing).unwrap_or(false))
            {
                unsatisfiable.push(owl_nothing);
            }

            Ok(unsatisfiable)
        } else {
            // In empty ontology, only owl:Nothing is unsatisfiable
            Ok(vec![ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ))])
        }
    }

    /// Get ontology prefixes
    pub fn get_prefixes(&self) -> Result<std::collections::HashMap<String, String>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_prefixes")?;
            let mut prefixes = std::collections::HashMap::new();

            // Extract prefixes from ontology entities
            for axiom in ontology.axioms() {
                self.extract_prefixes_from_axiom(axiom, &mut prefixes);
            }

            // Add standard prefixes if not present
            if !prefixes.contains_key("rdf") {
                prefixes.insert(
                    "rdf".to_string(),
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
                );
            }
            if !prefixes.contains_key("rdfs") {
                prefixes.insert(
                    "rdfs".to_string(),
                    "http://www.w3.org/2000/01/rdf-schema#".to_string(),
                );
            }
            if !prefixes.contains_key("owl") {
                prefixes.insert(
                    "owl".to_string(),
                    "http://www.w3.org/2002/07/owl#".to_string(),
                );
            }
            if !prefixes.contains_key("xsd") {
                prefixes.insert(
                    "xsd".to_string(),
                    "http://www.w3.org/2001/XMLSchema#".to_string(),
                );
            }

            Ok(prefixes)
        } else {
            // Return standard prefixes for empty ontology
            let mut prefixes = std::collections::HashMap::new();
            prefixes.insert(
                "rdf".to_string(),
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
            );
            prefixes.insert(
                "rdfs".to_string(),
                "http://www.w3.org/2000/01/rdf-schema#".to_string(),
            );
            prefixes.insert(
                "owl".to_string(),
                "http://www.w3.org/2002/07/owl#".to_string(),
            );
            prefixes.insert(
                "xsd".to_string(),
                "http://www.w3.org/2001/XMLSchema#".to_string(),
            );
            Ok(prefixes)
        }
    }

    /// Dump DL clauses
    pub fn dump_dl_clauses(&self) -> Result<crate::dl_clauses::DLClauseSet> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(ontology, "core: reading ontology for dump_dl_clauses")?;
            let mut generator = DLClauseGenerator::new();
            generator.generate_clauses(&*ontology_ref)
        } else {
            Ok(crate::dl_clauses::DLClauseSet::new())
        }
    }

    /// Process OWLlink request
    pub fn process_owllink_request(&self, request: &str) -> Result<String> {
        // Parse OWLlink XML request
        let parsed_request = self.parse_owllink_xml(request)?;

        match parsed_request.command.as_str() {
            "Tell" => {
                // Add axioms to ontology - would need mutable access
                Ok("<Response><OK/></Response>".to_string())
            }
            "IsClassSatisfiable" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let result = self.is_class_satisfiable(&class_expr)?;
                    Ok(format!(
                        "<Response><BooleanResponse result=\"{}\"/></Response>",
                        result
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in IsClassSatisfiable request",
                    ))
                }
            }
            "IsKBConsistent" => {
                let result = self.is_consistent()?;
                Ok(format!(
                    "<Response><BooleanResponse result=\"{}\"/></Response>",
                    result
                ))
            }
            "GetSubClasses" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let subclasses =
                        self.get_subclasses(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let subclass_elements: Vec<String> = subclasses
                        .iter()
                        .map(|expr| match expr {
                            crate::ontology::concepts::ClassExpression::Class(c) => {
                                format!("<Class IRI=\"{}\"/>", c.iri)
                            }
                            _ => {
                                "<Class IRI=\"http://www.w3.org/2002/07/owl#Nothing\"/>".to_string()
                            }
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfClasses>{}</SetOfClasses></Response>",
                        subclass_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetSubClasses request",
                    ))
                }
            }
            "GetSuperClasses" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let superclasses =
                        self.get_superclasses(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let superclass_elements: Vec<String> = superclasses
                        .iter()
                        .map(|expr| match expr {
                            crate::ontology::concepts::ClassExpression::Class(c) => {
                                format!("<Class IRI=\"{}\"/>", c.iri)
                            }
                            _ => "<Class IRI=\"http://www.w3.org/2002/07/owl#Thing\"/>".to_string(),
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfClasses>{}</SetOfClasses></Response>",
                        superclass_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetSuperClasses request",
                    ))
                }
            }
            "GetInstances" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let instances =
                        self.get_instances(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let instance_elements: Vec<String> = instances
                        .iter()
                        .map(|ind| {
                            format!(
                                "<NamedIndividual IRI=\"{}\"/>",
                                ind.iri()
                                    .map(|iri| iri.to_string())
                                    .unwrap_or_else(|| "unknown".to_string())
                            )
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfIndividuals>{}</SetOfIndividuals></Response>",
                        instance_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetInstances request",
                    ))
                }
            }
            _ => Err(Error::reasoning(&format!(
                "Unsupported OWLlink command: {}",
                parsed_request.command
            ))),
        }
    }

    /// Parse OWLlink XML request into structured data
    fn parse_owllink_xml(&self, xml: &str) -> Result<OWLlinkRequest> {
        // Simple XML parsing - would use proper XML parser in production
        let mut request = OWLlinkRequest {
            command: String::new(),
            axioms: Vec::new(),
            class_iri: None,
            individual_iri: None,
            direct: None,
        };

        // Extract command from XML
        if xml.contains("<Tell") {
            request.command = "Tell".to_string();
        } else if xml.contains("<IsClassSatisfiable") {
            request.command = "IsClassSatisfiable".to_string();
        } else if xml.contains("<IsKBConsistent") {
            request.command = "IsKBConsistent".to_string();
        } else if xml.contains("<GetSubClasses") {
            request.command = "GetSubClasses".to_string();
        } else if xml.contains("<GetSuperClasses") {
            request.command = "GetSuperClasses".to_string();
        } else if xml.contains("<GetInstances") {
            request.command = "GetInstances".to_string();
        }

        // Extract class IRI if present
        if let Some(start) = xml.find("IRI=\"") {
            if let Some(end) = xml[start + 5..].find("\"") {
                let iri_str = &xml[start + 5..start + 5 + end];
                request.class_iri = Some(
                    url::Url::parse(iri_str)
                        .map_err(|e| Error::reasoning(&format!("Invalid IRI: {}", e)))?,
                );
            }
        }

        // Extract direct attribute if present
        if xml.contains("direct=\"true\"") {
            request.direct = Some(true);
        } else if xml.contains("direct=\"false\"") {
            request.direct = Some(false);
        }

        Ok(request)
    }

    /// Execute SPARQL query
    pub fn execute_sparql_query(&self, query: &str) -> Result<String> {
        // Parse SPARQL query
        let parsed_query = self.parse_sparql_query(query)?;

        match parsed_query.query_type.as_str() {
            "SELECT" => self.execute_sparql_select(&parsed_query),
            "ASK" => self.execute_sparql_ask(&parsed_query),
            "CONSTRUCT" => self.execute_sparql_construct(&parsed_query),
            "DESCRIBE" => self.execute_sparql_describe(&parsed_query),
            "INSERT" => self.execute_sparql_insert(&parsed_query),
            "DELETE" => self.execute_sparql_delete(&parsed_query),
            _ => Err(Error::reasoning(&format!(
                "Unsupported SPARQL query type: {}",
                parsed_query.query_type
            ))),
        }
    }

    /// Parse SPARQL query into structured form
    fn parse_sparql_query(&self, query: &str) -> Result<SparqlQuery> {
        let query_upper = query.to_uppercase();

        let query_type = if query_upper.contains("INSERT") {
            "INSERT"
        } else if query_upper.contains("DELETE") {
            "DELETE"
        } else if query_upper.contains("SELECT") {
            "SELECT"
        } else if query_upper.contains("ASK") {
            "ASK"
        } else if query_upper.contains("CONSTRUCT") {
            "CONSTRUCT"
        } else if query_upper.contains("DESCRIBE") {
            "DESCRIBE"
        } else {
            return Err(Error::reasoning("Unknown SPARQL query type"));
        };

        // Extract variables for SELECT queries
        let mut variables = Vec::new();
        if query_type == "SELECT" {
            if let Some(start) = query_upper.find("SELECT") {
                if let Some(end) = query_upper.find("WHERE") {
                    let select_clause = &query[start + 6..end].trim();
                    if select_clause.starts_with('*') {
                        variables.push("*".to_string());
                    } else {
                        // Extract ?variable names
                        for word in select_clause.split_whitespace() {
                            if word.starts_with('?') {
                                variables.push(word.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Extract WHERE clause patterns
        let mut patterns = Vec::new();
        if let Some(start) = query_upper.find("WHERE") {
            if let Some(brace_start) = query[start..].find('{') {
                if let Some(brace_end) = query[start + brace_start..].find('}') {
                    let where_content =
                        &query[start + brace_start + 1..start + brace_start + brace_end].trim();

                    // Simple triple pattern extraction
                    for line in where_content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                patterns.push(TriplePattern {
                                    subject: parts[0].to_string(),
                                    predicate: parts[1].to_string(),
                                    object: parts[2].to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Extract INSERT DATA patterns
        let mut insert_patterns = Vec::new();
        if query_type == "INSERT" {
            insert_patterns = self.extract_insert_delete_patterns(query, "INSERT")?;
        }

        // Extract DELETE DATA patterns
        let mut delete_patterns = Vec::new();
        if query_type == "DELETE" {
            delete_patterns = self.extract_insert_delete_patterns(query, "DELETE")?;
        }

        Ok(SparqlQuery {
            query_type: query_type.to_string(),
            variables,
            patterns,
            insert_patterns,
            delete_patterns,
            original_query: query.to_string(),
        })
    }

    /// Execute SPARQL SELECT query
    fn execute_sparql_select(&self, query: &SparqlQuery) -> Result<String> {
        let mut results = Vec::new();

        if let Some(ontology) = &self.ontology {
            let ontology_guard =
                read_lock(ontology, "core: reading ontology for execute_sparql_select")?;

            // For each triple pattern, find matching axioms
            for pattern in &query.patterns {
                if pattern.predicate.contains("type") || pattern.predicate.contains("rdf:type") {
                    // Class assertion pattern: ?x rdf:type ClassName
                    for axiom in ontology_guard.axioms() {
                        if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                            let class_iri = match &assertion.class {
                                crate::ontology::concepts::ClassExpression::Class(c) => {
                                    c.iri.to_string()
                                }
                                _ => continue,
                            };
                            let individual_iri = assertion.individual.to_string();

                            if pattern.object.contains(&class_iri) || pattern.object == "?class" {
                                let mut binding = HashMap::new();
                                if pattern.subject.starts_with('?') {
                                    binding.insert(pattern.subject.clone(), individual_iri);
                                }
                                if pattern.object.starts_with('?') {
                                    binding.insert(pattern.object.clone(), class_iri);
                                }
                                results.push(binding);
                            }
                        }
                    }
                } else {
                    // Property assertion pattern: ?x propertyName ?y
                    for axiom in ontology_guard.axioms() {
                        match axiom {
                            crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) => {
                                let property_iri = assertion.property.to_string();
                                let subject_iri = assertion.source.to_string();
                                let object_iri = assertion.target.to_string();
                                if pattern.predicate.contains(&property_iri)
                                    || pattern.predicate.starts_with('?')
                                {
                                    let mut binding = HashMap::new();
                                    if pattern.subject.starts_with('?') {
                                        binding.insert(pattern.subject.clone(), subject_iri);
                                    }
                                    if pattern.predicate.starts_with('?') {
                                        binding.insert(pattern.predicate.clone(), property_iri);
                                    }
                                    if pattern.object.starts_with('?') {
                                        binding.insert(pattern.object.clone(), object_iri);
                                    }
                                    results.push(binding);
                                }
                            }
                            crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) => {
                                let property_iri = assertion.property.to_string();
                                let subject_iri = assertion.individual.to_string();
                                let object_value = assertion.value.to_string();
                                if pattern.predicate.contains(&property_iri)
                                    || pattern.predicate.starts_with('?')
                                {
                                    let mut binding = HashMap::new();
                                    if pattern.subject.starts_with('?') {
                                        binding.insert(pattern.subject.clone(), subject_iri);
                                    }
                                    if pattern.predicate.starts_with('?') {
                                        binding.insert(pattern.predicate.clone(), property_iri);
                                    }
                                    if pattern.object.starts_with('?') {
                                        binding.insert(pattern.object.clone(), object_value);
                                    }
                                    results.push(binding);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Format results as JSON
        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|binding| {
                serde_json::Value::Object(
                    binding
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect(),
                )
            })
            .collect();

        Ok(serde_json::json!({
            "head": {
                "vars": query.variables
            },
            "results": {
                "bindings": json_results
            }
        })
        .to_string())
    }

    /// Execute SPARQL ASK query
    fn execute_sparql_ask(&self, query: &SparqlQuery) -> Result<String> {
        // ASK queries return true/false
        let select_result = self.execute_sparql_select(query)?;
        let parsed: serde_json::Value = serde_json::from_str(&select_result)
            .map_err(|e| Error::reasoning(&format!("Failed to parse SELECT result: {}", e)))?;

        let has_results = if let Some(bindings) = parsed["results"]["bindings"].as_array() {
            !bindings.is_empty()
        } else {
            false
        };

        Ok(serde_json::json!({
            "head": {},
            "boolean": has_results
        })
        .to_string())
    }

    /// Execute SPARQL CONSTRUCT query
    fn execute_sparql_construct(&self, _query: &SparqlQuery) -> Result<String> {
        // CONSTRUCT queries build new RDF graphs
        // For now, return empty graph
        Ok(serde_json::json!({
            "head": {},
            "results": {
                "bindings": []
            }
        })
        .to_string())
    }

    /// Execute SPARQL DESCRIBE query
    fn execute_sparql_describe(&self, _query: &SparqlQuery) -> Result<String> {
        // DESCRIBE queries return information about resources
        // For now, return empty result
        Ok(serde_json::json!({
            "head": {},
            "results": {
                "bindings": []
            }
        })
        .to_string())
    }

    /// Execute SPARQL INSERT DATA query
    fn execute_sparql_insert(&self, query: &SparqlQuery) -> Result<String> {
        let ontology = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))?;

        let mut ontology_guard =
            write_lock(ontology, "core: writing ontology for execute_sparql_insert")?;

        let mut inserted_count = 0;

        // Convert triple patterns to axioms and add to ontology
        for pattern in &query.insert_patterns {
            let axiom = self.triple_pattern_to_axiom(pattern)?;
            ontology_guard.add_axiom(axiom);
            inserted_count += 1;
        }

        Ok(serde_json::json!({
            "status": "success",
            "message": format!("Inserted {} triples", inserted_count),
            "inserted": inserted_count
        })
        .to_string())
    }

    /// Execute SPARQL DELETE DATA query
    fn execute_sparql_delete(&self, query: &SparqlQuery) -> Result<String> {
        let ontology = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))?;

        let mut ontology_guard =
            write_lock(ontology, "core: writing ontology for execute_sparql_delete")?;

        let mut deleted_count = 0;
        let initial_count = ontology_guard.axioms().len();

        // Convert triple patterns to axioms and remove from ontology
        for pattern in &query.delete_patterns {
            let axiom = self.triple_pattern_to_axiom(pattern)?;
            // Remove the axiom
            ontology_guard.remove_axiom(&axiom);
        }

        // Calculate how many were actually deleted
        deleted_count = initial_count - ontology_guard.axioms().len();

        Ok(serde_json::json!({
            "status": "success",
            "message": format!("Deleted {} triples", deleted_count),
            "deleted": deleted_count
        })
        .to_string())
    }

    /// Extract INSERT or DELETE DATA patterns from query
    fn extract_insert_delete_patterns(
        &self,
        query: &str,
        operation: &str,
    ) -> Result<Vec<TriplePattern>> {
        let mut patterns = Vec::new();
        let query_upper = query.to_uppercase();

        // Look for "INSERT DATA" or "DELETE DATA" followed by braces
        let keyword = format!("{} DATA", operation);
        if let Some(start) = query_upper.find(&keyword) {
            // Find the opening brace after the keyword
            let search_from = start + keyword.len();
            let remaining_query = &query[search_from..];

            if let Some(brace_start) = remaining_query.find('{') {
                let content_start = search_from + brace_start + 1;
                let remaining_after_brace = &query[content_start..];

                if let Some(brace_end) = remaining_after_brace.find('}') {
                    let data_content = &query[content_start..content_start + brace_end];

                    // Parse triple patterns line by line
                    // Each line can contain one triple pattern
                    // Lines can end with . or ; or just a newline
                    let lines: Vec<&str> = data_content.lines().collect();

                    for line in lines {
                        let line = line.trim();

                        // Skip empty lines
                        if line.is_empty() {
                            continue;
                        }

                        // Remove trailing . or ; if present
                        let line_clean = line.trim_end_matches('.').trim_end_matches(';').trim();

                        if line_clean.is_empty() {
                            continue;
                        }

                        // Parse statement into triple pattern
                        // Split by whitespace to get subject, predicate, object
                        let parts: Vec<&str> = line_clean.split_whitespace().collect();

                        if parts.len() >= 3 {
                            let subject = parts[0];
                            let predicate = parts[1];
                            // Object might have spaces (e.g., literals)
                            let object = parts[2..].join(" ");

                            patterns.push(TriplePattern {
                                subject: subject.to_string(),
                                predicate: predicate.to_string(),
                                object,
                            });
                        }
                    }
                }
            }
        }

        if patterns.is_empty() {
            return Err(Error::reasoning(&format!(
                "No patterns found in {} query. Data content might be malformed.",
                operation
            )));
        }

        Ok(patterns)
    }

    /// Convert a triple pattern to an OWL axiom
    fn triple_pattern_to_axiom(&self, pattern: &TriplePattern) -> Result<crate::ontology::Axiom> {
        use crate::ontology::{
            Class, ClassExpression, DataProperty, DataPropertyExpression, IRI, Individual, Literal,
            ObjectProperty, ObjectPropertyExpression,
            axioms::{
                Axiom, ClassAssertionAxiom, DataPropertyAssertionAxiom,
                ObjectPropertyAssertionAxiom,
            },
        };

        // Remove angle brackets from IRIs
        let subject = pattern.subject.trim_matches(|c| c == '<' || c == '>');
        let predicate = pattern.predicate.trim_matches(|c| c == '<' || c == '>');
        let object_raw = pattern.object.trim();

        // Generate axiom ID (using a simple hash for now)
        let axiom_id = self.generate_axiom_id(pattern);

        // Check if predicate is rdf:type (class assertion)
        if predicate.contains("type") || predicate.ends_with("#type") {
            let object = object_raw.trim_matches(|c| c == '<' || c == '>');
            let individual = Individual::named(IRI::new(subject));
            let class = ClassExpression::Class(Class::new(IRI::new(object)));
            return Ok(Axiom::ClassAssertion(ClassAssertionAxiom {
                id: axiom_id,
                individual,
                class,
                annotations: Vec::new(),
            }));
        }

        // Check if object is a literal (data property assertion)
        if object_raw.starts_with('"') {
            let individual = Individual::named(IRI::new(subject));
            let property = DataPropertyExpression::DataProperty(DataProperty {
                iri: IRI::new(predicate),
            });

            // Parse literal value
            let value = if let Some(quote_end) = object_raw[1..].find('"') {
                object_raw[1..quote_end + 1].to_string()
            } else {
                object_raw.trim_matches('"').to_string()
            };

            let literal = Literal::new(value);

            return Ok(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                id: axiom_id,
                individual,
                property,
                value: literal,
                annotations: Vec::new(),
            }));
        }

        // Otherwise, it's an object property assertion
        let object = object_raw.trim_matches(|c| c == '<' || c == '>');
        let source = Individual::named(IRI::new(subject));
        let target = Individual::named(IRI::new(object));
        let property = ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(predicate),
        });

        Ok(Axiom::ObjectPropertyAssertion(
            ObjectPropertyAssertionAxiom {
                id: axiom_id,
                source,
                target,
                property,
                annotations: Vec::new(),
            },
        ))
    }

    /// Generate a unique axiom ID from a triple pattern
    fn generate_axiom_id(&self, pattern: &TriplePattern) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pattern.subject.hash(&mut hasher);
        pattern.predicate.hash(&mut hasher);
        pattern.object.hash(&mut hasher);
        hasher.finish()
    }

    /// Get DL clauses as string
    pub fn get_dl_clauses_string(&self) -> Result<String> {
        let clause_set = self.dump_dl_clauses()?;
        Ok(clause_set.to_string())
    }

    // Helper methods for reasoning

    /// Check if two classes are equivalent
    fn classes_equivalent(
        &self,
        class1: &ClassExpression,
        class2: &ClassExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(class1 == class2)
    }

    /// Check if two object properties are equivalent
    fn object_properties_equivalent(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }

    /// Check if two data properties are equivalent
    fn data_properties_equivalent(
        &self,
        prop1: &DataPropertyExpression,
        prop2: &DataPropertyExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }

    /// Filter superclasses to only direct ones (remove transitively implied ones)
    fn filter_direct_superclasses(
        &self,
        superclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut direct = Vec::new();

        for superclass in &superclasses {
            let mut is_direct = true;

            // Check if this superclass is implied by any other superclass
            for other_superclass in &superclasses {
                if superclass != other_superclass {
                    if self.is_subclass_of(other_superclass, superclass)? {
                        is_direct = false;
                        break;
                    }
                }
            }

            if is_direct {
                direct.push(superclass.clone());
            }
        }

        Ok(direct)
    }

    /// Filter subclasses to only direct ones (remove transitively implied ones)
    fn filter_direct_subclasses(
        &self,
        subclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut direct = Vec::new();

        for subclass in &subclasses {
            let mut is_direct = true;

            // Check if this subclass is implied by any other subclass
            for other_subclass in &subclasses {
                if subclass != other_subclass {
                    if self.is_subclass_of(subclass, other_subclass)? {
                        is_direct = false;
                        break;
                    }
                }
            }

            if is_direct {
                direct.push(subclass.clone());
            }
        }

        Ok(direct)
    }

    /// Get all inferred superclasses through transitive closure
    fn get_all_inferred_superclasses(
        &self,
        class: &ClassExpression,
        direct_superclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut all_superclasses = direct_superclasses;
        let mut to_process = all_superclasses.clone();
        let mut processed = std::collections::HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&format!("{:?}", current)) {
                continue;
            }
            processed.insert(format!("{:?}", current));

            // Get superclasses of current class (careful to avoid infinite recursion)
            if let Some(ontology_ref) = &self.ontology {
                let ontology = read_lock(
                    ontology_ref,
                    "core: reading ontology for get_all_inferred_superclasses",
                )?;
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                        if self.classes_equivalent(&subclass_axiom.subclass, &current)? {
                            let superclass = &subclass_axiom.superclass;
                            if !all_superclasses
                                .iter()
                                .any(|c| self.classes_equivalent(c, superclass).unwrap_or(false))
                            {
                                all_superclasses.push(superclass.clone());
                                to_process.push(superclass.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(all_superclasses)
    }

    /// Get all inferred subclasses through transitive closure
    fn get_all_inferred_subclasses(
        &self,
        class: &ClassExpression,
        direct_subclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut all_subclasses = direct_subclasses;
        let mut to_process = all_subclasses.clone();
        let mut processed = std::collections::HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&format!("{:?}", current)) {
                continue;
            }
            processed.insert(format!("{:?}", current));

            // Get subclasses of current class (careful to avoid infinite recursion)
            if let Some(ontology_ref) = &self.ontology {
                let ontology = read_lock(
                    ontology_ref,
                    "core: reading ontology for get_all_inferred_subclasses",
                )?;
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                        if self.classes_equivalent(&subclass_axiom.superclass, &current)? {
                            let subclass = &subclass_axiom.subclass;
                            if !all_subclasses
                                .iter()
                                .any(|c| self.classes_equivalent(c, subclass).unwrap_or(false))
                            {
                                all_subclasses.push(subclass.clone());
                                to_process.push(subclass.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(all_subclasses)
    }

    /// Get all classes mentioned in the ontology
    fn get_all_classes_in_ontology(
        &self,
        _ontology: &crate::ontology::Ontology,
    ) -> Result<Vec<ClassExpression>> {
        self.get_all_classes_in_ontology_internal()
    }

    /// Get all classes mentioned in the ontology (internal implementation)
    fn get_all_classes_in_ontology_internal(&self) -> Result<Vec<ClassExpression>> {
        let mut classes = Vec::new();

        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_all_classes_in_ontology_internal",
            )?;
            for axiom in ontology.axioms() {
                match axiom {
                    crate::ontology::Axiom::SubClassOf(axiom) => {
                        classes.push(axiom.subclass.clone());
                        classes.push(axiom.superclass.clone());
                    }
                    crate::ontology::Axiom::EquivalentClasses(axiom) => {
                        classes.extend(axiom.classes.clone());
                    }
                    crate::ontology::Axiom::DisjointClasses(axiom) => {
                        classes.extend(axiom.classes.clone());
                    }
                    crate::ontology::Axiom::ClassAssertion(axiom) => {
                        classes.push(axiom.class.clone());
                    }
                    _ => {}
                }
            }
        }

        // Remove duplicates
        classes.sort_by_key(|c| format!("{:?}", c));
        classes.dedup_by_key(|c| format!("{:?}", c));

        Ok(classes)
    }

    /// Extract prefixes from a single axiom
    fn extract_prefixes_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        prefixes: &mut std::collections::HashMap<String, String>,
    ) {
        match axiom {
            crate::ontology::Axiom::ClassAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&assertion.class) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
                if let Some(iri) = assertion.individual.iri() {
                    self.add_prefix_from_iri(&iri.to_string(), prefixes);
                }
            }
            crate::ontology::Axiom::SubClassOf(axiom) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.subclass) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.superclass) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
            }
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_object_property(&assertion.property) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
            }
            _ => {} // Handle other axiom types as needed
        }
    }

    /// Extract IRI from class expression
    fn extract_iri_from_class_expression(&self, expr: &ClassExpression) -> Option<String> {
        match expr {
            ClassExpression::Class(class) => Some(class.iri.to_string()),
            _ => None,
        }
    }

    /// Extract IRI from object property expression
    fn extract_iri_from_object_property(&self, expr: &ObjectPropertyExpression) -> Option<String> {
        match expr {
            ObjectPropertyExpression::ObjectProperty(prop) => Some(prop.iri.to_string()),
            _ => None,
        }
    }

    /// Add prefix from IRI
    fn add_prefix_from_iri(
        &self,
        iri: &str,
        prefixes: &mut std::collections::HashMap<String, String>,
    ) {
        if let Some(hash_pos) = iri.rfind('#') {
            let base = &iri[..hash_pos + 1];
            if !prefixes.values().any(|v| v == base) {
                // Try to detect common namespaces
                let prefix_name = match base {
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#" => "rdf".to_string(),
                    "http://www.w3.org/2000/01/rdf-schema#" => "rdfs".to_string(),
                    "http://www.w3.org/2002/07/owl#" => "owl".to_string(),
                    "http://www.w3.org/2001/XMLSchema#" => "xsd".to_string(),
                    _ => format!("ns{}", prefixes.len()),
                };
                prefixes.insert(prefix_name, base.to_string());
            }
        }
    }

    /// Extract object properties from axiom
    fn extract_object_properties_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        properties: &mut std::collections::HashSet<ObjectPropertyExpression>,
    ) {
        match axiom {
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                properties.insert(assertion.property.clone());
            }
            crate::ontology::Axiom::SubObjectPropertyOf(axiom) => {
                properties.insert(axiom.sub_property.clone());
                properties.insert(axiom.super_property.clone());
            }
            crate::ontology::Axiom::ObjectPropertyDomain(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::ObjectPropertyRange(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::FunctionalObjectProperty(axiom) => {
                properties.insert(axiom.property.clone());
            }
            _ => {} // Handle other axiom types that mention object properties
        }
    }

    /// Extract data properties from axiom
    fn extract_data_properties_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        properties: &mut std::collections::HashSet<DataPropertyExpression>,
    ) {
        match axiom {
            crate::ontology::Axiom::DataPropertyAssertion(assertion) => {
                properties.insert(assertion.property.clone());
            }
            crate::ontology::Axiom::SubDataPropertyOf(axiom) => {
                properties.insert(axiom.sub_property.clone());
                properties.insert(axiom.super_property.clone());
            }
            crate::ontology::Axiom::DataPropertyDomain(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::DataPropertyRange(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::FunctionalDataProperty(axiom) => {
                properties.insert(axiom.property.clone());
            }
            _ => {} // Handle other axiom types that mention data properties
        }
    }

    /// Save DL clauses to file
    pub fn save_dl_clauses<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let clause_set = self.dump_dl_clauses()?;
        let content = clause_set.to_string();
        std::fs::write(path, content).map_err(|e| Error::Io {
            message: format!("Failed to write DL clauses: {}", e),
        })
    }

    /// Create a server manager for this reasoner
    /// This allows starting web servers (OWLlink, SPARQL, REST API) for remote access
    #[cfg(feature = "server")]
    pub fn create_server_manager(&self) -> Result<crate::server::ServerManager> {
        use std::sync::Arc;

        let ontology = self.ontology.as_ref().ok_or_else(|| Error::Reasoning {
            message: "No ontology loaded. Load an ontology before starting the server.".to_string(),
        })?;

        let ontology_clone = ontology
            .read()
            .map_err(|_| Error::Reasoning {
                message: "Failed to acquire ontology read lock".to_string(),
            })?
            .clone();

        let reasoning_service = Arc::new(crate::reasoning::ReasoningService::new(
            ontology_clone,
            self.config.clone(),
        ));

        Ok(crate::server::ServerManager::new(
            self.config.server.clone(),
            reasoning_service,
        ))
    }

    /// Start a server with the specified configuration
    /// Returns a ServerManager that can be used to stop the server
    #[cfg(feature = "server")]
    pub async fn start_server(&self) -> Result<crate::server::ServerManager> {
        let mut server_manager = self.create_server_manager()?;
        server_manager.start_all().await?;
        Ok(server_manager)
    }

    /// Start a server on a specific port
    /// Returns a ServerManager that can be used to stop the server
    #[cfg(feature = "server")]
    pub async fn start_server_on_port(&self, port: u16) -> Result<crate::server::ServerManager> {
        use std::sync::Arc;

        let ontology = self.ontology.as_ref().ok_or_else(|| Error::Reasoning {
            message: "No ontology loaded. Load an ontology before starting the server.".to_string(),
        })?;

        let ontology_clone = ontology
            .read()
            .map_err(|_| Error::Reasoning {
                message: "Failed to acquire ontology read lock".to_string(),
            })?
            .clone();

        let reasoning_service = Arc::new(crate::reasoning::ReasoningService::new(
            ontology_clone,
            self.config.clone(),
        ));

        let mut server_manager = crate::server::ServerManager::with_port(reasoning_service, port);
        server_manager.start_all().await?;
        Ok(server_manager)
    }
}
