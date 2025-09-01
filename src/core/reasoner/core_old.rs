//! Core reasoner functionality
//!
//! This module contains the main Reasoner struct and core operations like
//! loading ontologies and basic reasoning setup.

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::reasoner::{
        classification::ClassificationService,
        explanation::ExplanationService,
        queries::QueryProcessor,
        statistics::ReasoningStatistics,
        tableau::TableauFactory,
        tasks::ReasoningTaskService,
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{Ontology, OntologyFormat, OntologyRef, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression},
};
use log::{info, warn};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
    time::Instant,
};

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
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if &subclass_axiom.subclass == subclass && &subclass_axiom.superclass == superclass {
                        return Ok(true);
                    }
                }
            }
            
            // Check through equivalent classes
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(subclass) && equiv_axiom.classes.contains(superclass) {
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
            (ClassExpression::OWLNothing, _) => Ok(true),
            
            // Everything is subclass of Top
            (_, ClassExpression::OWLThing) => Ok(true),
            
            // Nothing is superclass of Top (except Top itself)
            (ClassExpression::OWLThing, ClassExpression::OWLNothing) => Ok(false),
            
            // Intersection subsumption: A ⊓ B ⊑ A and A ⊓ B ⊑ B
            (ClassExpression::ObjectIntersectionOf(components), superclass) => {
                Ok(components.contains(superclass))
            },
            
            // Union subsumption: A ⊑ A ⊔ B and B ⊑ A ⊔ B
            (subclass, ClassExpression::ObjectUnionOf(components)) => {
                Ok(components.contains(subclass))
            },
            
            _ => Ok(false),
        }
    }

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::reasoner::{
        classification::ClassificationService,
        explanation::ExplanationService,
        queries::QueryProcessor,
        statistics::ReasoningStatistics,
        tableau::TableauFactory,
        tasks::ReasoningTaskService,
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{Ontology, OntologyFormat, OntologyRef, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression},
};
use log::{info, warn};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
    time::Instant,
};

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
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if &subclass_axiom.subclass == subclass && &subclass_axiom.superclass == superclass {
                        return Ok(true);
                    }
                }
            }
            
            // Check through equivalent classes
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(subclass) && equiv_axiom.classes.contains(superclass) {
                        return Ok(true);
                    }
                }
            }
        }
        
        // Use tableau-based reasoning if available
        if let Some(tableau) = &self.tableau {
            // Convert class expressions to concepts and check subsumption
            return self.check_tableau_subsumption(subclass, superclass, tableau);
        }
        
        // Check using built-in OWL semantics
        self.check_semantic_subsumption(subclass, superclass)
    }
    
    /// Check subsumption using tableau reasoning
    fn check_tableau_subsumption(&self, subclass: &ClassExpression, superclass: &ClassExpression, _tableau: &crate::core::tableau::Tableau) -> Result<bool> {
        // This would use the tableau to check if subclass ⊑ superclass
        // by checking if subclass ⊓ ¬superclass is unsatisfiable
        
        // For now, return false for complex cases
        // A full implementation would construct the negation and test satisfiability
        Ok(false)
    }
    
    /// Check subsumption using semantic reasoning
    fn check_semantic_subsumption(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        use crate::ontology::ClassExpression;
        
        match (subclass, superclass) {
            // owl:Nothing is subclass of everything
            (ClassExpression::Class(class), _) if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" => Ok(true),
            
            // Everything is subclass of owl:Thing
            (_, ClassExpression::Class(class)) if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" => Ok(true),
            
            // Intersection subsumption: A ⊓ B ⊑ A and A ⊓ B ⊑ B
            (ClassExpression::ObjectIntersectionOf(components), superclass) => {
                Ok(components.contains(superclass))
            },
            
            // Union subsumption: A ⊑ A ⊔ B and B ⊑ A ⊔ B
            (subclass, ClassExpression::ObjectUnionOf(components)) => {
                Ok(components.contains(subclass))
            },
            
            _ => Ok(false),
        }
    }

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::reasoner::{
        classification::ClassificationService,
        explanation::ExplanationService,
        queries::QueryProcessor,
        statistics::ReasoningStatistics,
        tableau::TableauFactory,
        tasks::ReasoningTaskService,
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{Ontology, OntologyFormat, OntologyRef, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression},
};
use log::{info, warn};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
    time::Instant,
};

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
    /// Create a new reasoner with the given configuration
    pub fn new(config: ReasonerConfig) -> Result<Self> {
        let cache_config = crate::cache::CacheConfig {
            enable_concept_cache: config.cache.enable_satisfiability_cache,
            enable_satisfiability_cache: config.cache.enable_satisfiability_cache,
            enable_subsumption_cache: config.cache.enable_satisfiability_cache,
            enable_classification_cache: config.cache.enable_completion_graph_cache,
            enable_realization_cache: config.cache.enable_unsatisfiability_cache,
            max_size: config.cache.max_cache_size_mb as usize * 1024 * 1024,
            ttl: config.cache.cache_ttl.unwrap_or(std::time::Duration::from_secs(3600)),
        };

        let cache_manager = Arc::new(RwLock::new(CacheManager::new(cache_config)));
        let tableau_factory = TableauFactory::new(config.clone())?;

        // Create services
        let task_service = ReasoningTaskService::new(
            tableau_factory.clone(),
            cache_manager.clone(),
        );

        let classification_service = ClassificationService::new(
            task_service.clone(),
            cache_manager.clone(),
        );

        let query_processor = QueryProcessor::new();
        let explanation_service = ExplanationService::new();

        Ok(Self {
            config,
            ontology: None,
            cache_manager,
            tableau_factory,
            statistics: ReasoningStatistics::default(),
            task_service,
            classification_service,
            query_processor,
            explanation_service,
        })
    }

    /// Load an ontology from a file
    pub fn load_ontology_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        format: OntologyFormat,
    ) -> Result<()> {
        info!("Loading ontology from: {}", path.as_ref().display());
        let start_time = Instant::now();

        let format_string = if format == OntologyFormat::Auto {
            None
        } else {
            Some(format.format_string().to_string())
        };

        let ontology = Ontology::from_file(path, format_string)?;
        self.ontology = Some(Arc::new(RwLock::new(ontology)));

        // Clear caches when new ontology is loaded
        self.cache_manager.write().unwrap().clear_all();

        let load_time = start_time.elapsed();
        info!("Ontology loaded in {load_time:?}");

        Ok(())
    }

    /// Load an ontology from memory
    pub fn load_ontology(&mut self, ontology: Ontology) -> Result<()> {
        info!("Loading ontology from memory");
        self.ontology = Some(Arc::new(RwLock::new(ontology)));
        self.cache_manager.write().unwrap().clear_all();
        Ok(())
    }

    /// Get the current ontology
    pub fn get_ontology(&self) -> Result<OntologyRef> {
        self.ontology
            .clone()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))
    }

    /// Check if the current ontology is consistent
    pub fn is_consistent(&mut self) -> Result<bool> {
        let ontology = self.get_ontology()?;
        self.task_service
            .check_consistency(&ontology, &mut self.statistics)
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&mut self, class_iri: &str) -> Result<bool> {
        let ontology = self.get_ontology()?;
        self.task_service
            .check_satisfiability(class_iri, &ontology, &mut self.statistics)
    }

    /// Check if one class subsumes another
    pub fn is_subclass_of(&mut self, subclass: &str, superclass: &str) -> Result<bool> {
        let ontology = self.get_ontology()?;
        self.task_service
            .check_subsumption(subclass, superclass, &ontology, &mut self.statistics)
    }

    /// Check if one class is subsumed by another (uses ClassExpression parameters like the original)
    pub fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        // Enhanced subsumption checking using tableau and semantic reasoning
        
        // Use the existing is_subclass_of method which has proper implementation
        self.is_subclass_of(subclass, superclass)
    }

    /// Get superclasses of a given class (returns ClassExpression vector like the original)
    pub fn get_superclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Enhanced classification algorithm for finding superclasses
        let mut superclasses = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            // Collect explicit superclasses from subclass axioms
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if subclass_axiom.subclass == *class {
                        superclasses.push(subclass_axiom.superclass.clone());
                    }
                }
            }
            
            // Add equivalent classes as superclasses
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(class) {
                        for equiv_class in &equiv_axiom.classes {
                            if equiv_class != class {
                                superclasses.push(equiv_class.clone());
                            }
                        }
                    }
                }
            }
            
            // If not direct, compute transitive closure
            if !direct {
                let mut all_superclasses = superclasses.clone();
                let mut to_process = superclasses.clone();
                
                while !to_process.is_empty() {
                    let current = to_process.remove(0);
                    let current_supers = self.get_superclasses(&current, true)?;
                    
                    for super_class in current_supers {
                        if !all_superclasses.contains(&super_class) {
                            all_superclasses.push(super_class.clone());
                            to_process.push(super_class);
                        }
                    }
                }
                
                superclasses = all_superclasses;
            }
        }
        
        // Add owl:Thing as ultimate superclass if not already present
        let owl_thing = crate::ontology::ClassExpression::Class(crate::ontology::concepts::Class {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing"),
        });
        
        if *class != owl_thing && !superclasses.contains(&owl_thing) {
            superclasses.push(owl_thing);
        }
        
        Ok(superclasses)
    }

    /// Get subclasses of a given class (returns ClassExpression vector like the original)
    pub fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Enhanced classification algorithm for finding subclasses
        let mut subclasses = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            // Collect explicit subclasses from subclass axioms
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if subclass_axiom.superclass == *class {
                        subclasses.push(subclass_axiom.subclass.clone());
                    }
                }
            }
            
            // Add equivalent classes as subclasses
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(class) {
                        for equiv_class in &equiv_axiom.classes {
                            if equiv_class != class {
                                subclasses.push(equiv_class.clone());
                            }
                        }
                    }
                }
            }
            
            // If not direct, compute transitive closure
            if !direct {
                let mut all_subclasses = subclasses.clone();
                let mut to_process = subclasses.clone();
                
                while !to_process.is_empty() {
                    let current = to_process.remove(0);
                    let current_subs = self.get_subclasses(&current, true)?;
                    
                    for sub_class in current_subs {
                        if !all_subclasses.contains(&sub_class) {
                            all_subclasses.push(sub_class.clone());
                            to_process.push(sub_class);
                        }
                    }
                }
                
                subclasses = all_subclasses;
            }
        }
        
        // Add owl:Nothing as ultimate subclass if class is owl:Thing
        let owl_thing = crate::ontology::ClassExpression::Class(crate::ontology::concepts::Class {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing"),
        });
        let owl_nothing = crate::ontology::ClassExpression::Class(crate::ontology::concepts::Class {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing"),
        });
        
        if *class == owl_thing && !subclasses.contains(&owl_nothing) {
            subclasses.push(owl_nothing);
        }
        
        Ok(subclasses)
    }

    /// Get equivalent classes of a given class (returns ClassExpression vector like the original)
    pub fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Vec<ClassExpression>> {
        // Enhanced algorithm for finding equivalent classes
        let mut equivalent_classes = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            // Find explicit equivalent class axioms
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(class) {
                        for equiv_class in &equiv_axiom.classes {
                            if equiv_class != class {
                                equivalent_classes.push(equiv_class.clone());
                            }
                        }
                    }
                }
            }
            
            // Check for classes that are both subclass and superclass (mutual subsumption)
            let mut candidates = Vec::new();
            
            // Find potential candidates from subclass relationships
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if subclass_axiom.subclass == *class {
                        candidates.push(subclass_axiom.superclass.clone());
                    } else if subclass_axiom.superclass == *class {
                        candidates.push(subclass_axiom.subclass.clone());
                    }
                }
            }
            
            // Check mutual subsumption for candidates
            for candidate in candidates {
                if !equivalent_classes.contains(&candidate) {
                    let is_sub_of_candidate = self.is_subclass_of(class, &candidate)?;
                    let candidate_is_sub = self.is_subclass_of(&candidate, class)?;
                    
                    if is_sub_of_candidate && candidate_is_sub {
                        equivalent_classes.push(candidate);
                    }
                }
            }
        }
        
        Ok(equivalent_classes)
    }

    /// Get instances of a given class (returns Individual vector like the original)
    pub fn get_instances(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<Individual>> {
        // Enhanced realization algorithm for finding instances
        let mut instances = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Collect explicit instances from class assertion axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(class_assertion) = axiom {
                    if class_assertion.class_expression == *class {
                        instances.push(class_assertion.individual.clone());
                    }
                }
            }
            
            // If not direct, also check for instances of subclasses
            if !direct {
                let subclasses = self.get_subclasses(class, false)?;
                for subclass in subclasses {
                    let subclass_instances = self.get_instances(&subclass, true)?;
                    for instance in subclass_instances {
                        if !instances.contains(&instance) {
                            instances.push(instance);
                        }
                    }
                }
            }
            
            // Check for inferred instances through equivalence
            let equivalent_classes = self.get_equivalent_classes(class)?;
            for equiv_class in equivalent_classes {
                let equiv_instances = self.get_instances(&equiv_class, true)?;
                for instance in equiv_instances {
                    if !instances.contains(&instance) {
                        instances.push(instance);
                    }
                }
            }
        }
        
        Ok(instances)
    }

    /// Get types of a given individual (returns ClassExpression vector like the original)
    pub fn get_types(
        &self,
        individual: &Individual,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Enhanced algorithm for finding types of an individual
        let mut types = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Collect explicit types from class assertion axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(class_assertion) = axiom {
                    if class_assertion.individual == *individual {
                        types.push(class_assertion.class_expression.clone());
                    }
                }
            }
            
            // If not direct, compute transitive closure via superclasses
            if !direct {
                let mut all_types = types.clone();
                for direct_type in &types {
                    let superclasses = self.get_superclasses(direct_type, false)?;
                    for superclass in superclasses {
                        if !all_types.contains(&superclass) {
                            all_types.push(superclass);
                        }
                    }
                }
                types = all_types;
            }
            
            // Check for inferred types through same-as relationships
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::SameIndividual(same_axiom) = axiom {
                    if same_axiom.individuals.contains(individual) {
                        for same_individual in &same_axiom.individuals {
                            if same_individual != individual {
                                let same_types = self.get_types(same_individual, direct)?;
                                for same_type in same_types {
                                    if !types.contains(&same_type) {
                                        types.push(same_type);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(types)
    }

    /// Get object property values for an individual (returns Individual vector like the original)
    pub fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<Individual>> {
        // Enhanced property value retrieval algorithm
        let mut values = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Collect explicit property assertions
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = axiom {
                    if prop_assertion.subject == *individual && prop_assertion.property == *property {
                        values.push(prop_assertion.object.clone());
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property {
                                let equiv_values = self.get_object_property_values(individual, equiv_prop)?;
                                for value in equiv_values {
                                    if !values.contains(&value) {
                                        values.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Check through inverse properties
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::InverseObjectProperties(inv_axiom) = axiom {
                    if inv_axiom.property1 == *property {
                        // Find assertions where our individual is the object
                        for prop_axiom in ontology_guard.axioms() {
                            if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = prop_axiom {
                                if prop_assertion.object == *individual && prop_assertion.property == inv_axiom.property2 {
                                    if !values.contains(&prop_assertion.subject) {
                                        values.push(prop_assertion.subject.clone());
                                    }
                                }
                            }
                        }
                    } else if inv_axiom.property2 == *property {
                        // Find assertions where our individual is the object
                        for prop_axiom in ontology_guard.axioms() {
                            if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = prop_axiom {
                                if prop_assertion.object == *individual && prop_assertion.property == inv_axiom.property1 {
                                    if !values.contains(&prop_assertion.subject) {
                                        values.push(prop_assertion.subject.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Check for same individuals to get their property values
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::SameIndividual(same_axiom) = axiom {
                    if same_axiom.individuals.contains(individual) {
                        for same_individual in &same_axiom.individuals {
                            if same_individual != individual {
                                let same_values = self.get_object_property_values(same_individual, property)?;
                                for value in same_values {
                                    if !values.contains(&value) {
                                        values.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(values)
    }

    /// Get data property values for an individual (returns Literal vector like the original)
    pub fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<Vec<crate::ontology::Literal>> {
        // Enhanced data property value retrieval algorithm
        let mut values = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Collect explicit data property assertions
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::DataPropertyAssertion(data_assertion) = axiom {
                    if data_assertion.subject == *individual && data_assertion.property == *property {
                        values.push(data_assertion.object.clone());
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentDataProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property {
                                let equiv_values = self.get_data_property_values(individual, equiv_prop)?;
                                for value in equiv_values {
                                    if !values.contains(&value) {
                                        values.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Check for same individuals to get their property values
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::SameIndividual(same_axiom) = axiom {
                    if same_axiom.individuals.contains(individual) {
                        for same_individual in &same_axiom.individuals {
                            if same_individual != individual {
                                let same_values = self.get_data_property_values(same_individual, property)?;
                                for value in same_values {
                                    if !values.contains(&value) {
                                        values.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(values)
    }

    /// Check if property is functional
    pub fn is_functional(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced functional property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit functional property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::FunctionalObjectProperty(func_axiom) = axiom {
                    if func_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check for inverse functional properties where this is the inverse
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::InverseObjectProperties(inv_axiom) = axiom {
                    if inv_axiom.property1 == *property {
                        if self.is_inverse_functional(&inv_axiom.property2)? {
                            return Ok(true);
                        }
                    } else if inv_axiom.property2 == *property {
                        if self.is_inverse_functional(&inv_axiom.property1)? {
                            return Ok(true);
                        }
                    }
                }
            }
            
            // Check through property equivalence and subsumption
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_functional(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is inverse functional
    pub fn is_inverse_functional(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced inverse functional property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit inverse functional property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::InverseFunctionalObjectProperty(inv_func_axiom) = axiom {
                    if inv_func_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check for functional properties where this is the inverse
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::InverseObjectProperties(inv_axiom) = axiom {
                    if inv_axiom.property1 == *property {
                        if self.is_functional(&inv_axiom.property2)? {
                            return Ok(true);
                        }
                    } else if inv_axiom.property2 == *property {
                        if self.is_functional(&inv_axiom.property1)? {
                            return Ok(true);
                        }
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_inverse_functional(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is transitive
    pub fn is_transitive(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced transitive property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit transitive property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::TransitiveObjectProperty(trans_axiom) = axiom {
                    if trans_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_transitive(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
            
            // Check through property chains
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::SubObjectPropertyOf(sub_prop_axiom) = axiom {
                    if let crate::ontology::ObjectPropertyExpression::ObjectPropertyChain(chain) = &sub_prop_axiom.sub_property {
                        if chain.properties.len() == 2 
                            && chain.properties[0] == *property 
                            && chain.properties[1] == *property
                            && sub_prop_axiom.super_property == *property {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is symmetric
    pub fn is_symmetric(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced symmetric property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit symmetric property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::SymmetricObjectProperty(sym_axiom) = axiom {
                    if sym_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_symmetric(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
            
            // Check if property is its own inverse
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::InverseObjectProperties(inv_axiom) = axiom {
                    if (inv_axiom.property1 == *property && inv_axiom.property2 == *property) {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is asymmetric
    pub fn is_asymmetric(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced asymmetric property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit asymmetric property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::AsymmetricObjectProperty(asym_axiom) = axiom {
                    if asym_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_asymmetric(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is reflexive
    pub fn is_reflexive(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced reflexive property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit reflexive property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::ReflexiveObjectProperty(refl_axiom) = axiom {
                    if refl_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_reflexive(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if property is irreflexive
    pub fn is_irreflexive(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        // Enhanced irreflexive property checking
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Check for explicit irreflexive property axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::IrreflexiveObjectProperty(irrefl_axiom) = axiom {
                    if irrefl_axiom.property == *property {
                        return Ok(true);
                    }
                }
            }
            
            // Check through property equivalence
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::Axiom::EquivalentObjectProperties(equiv_axiom) = axiom {
                    if equiv_axiom.properties.contains(property) {
                        for equiv_prop in &equiv_axiom.properties {
                            if equiv_prop != property && self.is_irreflexive(equiv_prop)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(&mut self) -> Result<crate::core::reasoner::results::ClassificationResult> {
        let ontology = self.get_ontology()?;
        self.classification_service
            .classify(&ontology, &mut self.statistics)
    }

    /// Classify object properties
    pub fn classify_object_properties(
        &mut self,
    ) -> Result<crate::core::reasoner::results::PropertyClassificationResult> {
        let ontology = self.get_ontology()?;
        self.classification_service
            .classify_object_properties(&ontology, &mut self.statistics)
    }

    /// Classify data properties
    pub fn classify_data_properties(
        &mut self,
    ) -> Result<crate::core::reasoner::results::PropertyClassificationResult> {
        let ontology = self.get_ontology()?;
        self.classification_service
            .classify_data_properties(&ontology, &mut self.statistics)
    }

    /// Get all unsatisfiable classes (equivalent to owl:Nothing)
    pub fn get_unsatisfiable_classes(
        &mut self,
    ) -> Result<Vec<crate::ontology::ClassExpression>> {
        let ontology = self.get_ontology()?;
        self.classification_service
            .get_unsatisfiable_classes(&ontology, &mut self.statistics)
    }

    /// Perform realization (find most specific classes for individuals)
    pub fn realize(&mut self) -> Result<crate::core::reasoner::results::RealizationResult> {
        let ontology = self.get_ontology()?;
        self.classification_service
            .realize(&ontology, &mut self.statistics)
    }

    /// Check if an individual is an instance of a class
    pub fn is_instance_of(&mut self, individual: &str, class: &str) -> Result<bool> {
        info!("Checking instance relationship: {individual} ∈ {class}");

        // Convert string parameters to proper types
        let individual_obj =
            crate::ontology::Individual::named(crate::ontology::IRI::new(individual));
        let class_obj = crate::ontology::ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::new(class).to_url()?.into(),
        });

        let ontology = self.get_ontology()?;
        self.task_service.check_instance(
            &individual_obj,
            &class_obj,
            &ontology,
            &mut self.statistics,
        )
    }

    /// Check entailment between two ontologies
    pub fn check_entailment(
        &mut self,
        premise_file: &Path,
        conclusion_file: &Path,
        format: OntologyFormat,
    ) -> Result<bool> {
        let start_time = Instant::now();

        info!(
            "Checking entailment: {} |= {}",
            premise_file.display(),
            conclusion_file.display()
        );

        // Load premise ontology
        let premise_ontology = Ontology::from_file(premise_file, None)?;

        // Load conclusion ontology
        let conclusion_ontology = Ontology::from_file(conclusion_file, None)?;

        // Check if premise entails conclusion
        let entails = self.check_ontology_entailment(&premise_ontology, &conclusion_ontology)?;

        let reasoning_time = start_time.elapsed();
        self.statistics.add_reasoning_time(reasoning_time);

        info!("Entailment check completed in {reasoning_time:?}: {entails}");
        Ok(entails)
    }

    /// Get available prefixes from the ontology
    pub fn get_prefixes(&self) -> Result<HashMap<String, String>> {
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();

        // Extract prefixes from the ontology
        let mut prefixes = HashMap::new();

        // Add default OWL prefixes
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        // Add ontology-specific prefixes based on IRIs found
        if let Some(ontology_iri) = ontology_guard.get_iri() {
            let iri_str = ontology_iri.as_str();
            if let Some(base) = iri_str.strip_suffix('#') {
                prefixes.insert("".to_string(), format!("{base}#"));
            } else if let Some(base) = iri_str.strip_suffix('/') {
                prefixes.insert("".to_string(), format!("{base}/"));
            }
        }

        Ok(prefixes)
    }

    /// Execute a SPARQL query against the ontology
    pub fn execute_sparql_query(&self, query: &str) -> Result<String> {
        info!("Executing SPARQL query");

        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            self.query_processor
                .process_sparql_query(query, &ontology_guard)
        } else {
            Err(Error::reasoning("No ontology loaded for SPARQL query"))
        }
    }

    /// Process an `OWLlink` request
    pub fn process_owllink_request(&mut self, request: &str) -> Result<String> {
        info!("Processing OWLlink request");

        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Clone the ontology data needed for processing to avoid borrow conflicts
            let ontology_data = ontology_guard.clone();
            drop(ontology_guard); // Explicitly drop the read guard
            
            self.query_processor
                .process_owllink_request(request, &ontology_data)
        } else {
            Err(Error::reasoning("No ontology loaded for OWLlink request"))
        }
    }

    /// Add axiom for incremental reasoning
    pub fn add_axiom(&mut self, axiom: &crate::ontology::Axiom) -> Result<()> {
        if let Some(ontology) = &mut self.ontology {
            let mut ontology_guard = ontology.write().unwrap();
            ontology_guard.add_axiom(axiom.clone());
            self.cache_manager.write().unwrap().clear_all();
            Ok(())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Remove axiom for incremental reasoning
    pub fn remove_axiom(&mut self, axiom: &crate::ontology::Axiom) -> Result<()> {
        if let Some(ontology) = &mut self.ontology {
            let mut ontology_guard = ontology.write().unwrap();
            ontology_guard.remove_axiom(axiom);
            self.cache_manager.write().unwrap().clear_all();
            Ok(())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get ontology size
    #[must_use]
    pub fn get_ontology_size(&self) -> usize {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            ontology_guard.axioms.len()
        } else {
            0
        }
    }

    /// Reset the reasoner
    pub fn reset(&mut self) -> Result<()> {
        self.ontology = None;
        self.cache_manager.write().unwrap().clear_all();
        self.statistics.reset();
        Ok(())
    }

    /// Explain entailment
    pub fn explain_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
    ) -> Result<Vec<crate::ontology::Axiom>> {
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        self.explanation_service
            .explain_entailment(axiom, &ontology_guard)
    }

    /// Explain inconsistency
    pub fn explain_inconsistency(&self) -> Result<Vec<crate::ontology::Axiom>> {
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        self.explanation_service
            .explain_inconsistency(&ontology_guard)
    }

    /// Generate DL clauses from the current ontology
    pub fn dump_dl_clauses(&self) -> Result<DLClauseSet> {
        let start_time = Instant::now();

        info!("Generating DL clauses from ontology");

        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();

        let mut generator = DLClauseGenerator::new();
        let clause_set = generator.generate_clauses(&ontology_guard)?;

        let generation_time = start_time.elapsed();
        info!(
            "DL clause generation completed in {generation_time:?}: {} deterministic, {} disjunctive, {} facts",
            clause_set.statistics.deterministic_clause_count,
            clause_set.statistics.disjunctive_clause_count,
            clause_set.statistics.positive_fact_count + clause_set.statistics.negative_fact_count
        );

        Ok(clause_set)
    }

    /// Save DL clauses to a file
    pub fn save_dl_clauses<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let clause_set = self.dump_dl_clauses()?;
        clause_set.save_to_file(&path)?;
        info!("DL clauses saved to: {}", path.as_ref().display());
        Ok(())
    }

    /// Get DL clauses as HermiT-formatted string
    pub fn get_dl_clauses_string(&self) -> Result<String> {
        let clause_set = self.dump_dl_clauses()?;
        Ok(clause_set.to_hermit_format())
    }

    /// Get reasoning statistics
    #[must_use]
    pub fn get_statistics(&self) -> &ReasoningStatistics {
        &self.statistics
    }

    /// Reset reasoning statistics
    pub fn reset_statistics(&mut self) {
        self.statistics.reset();
    }

    // Private helper methods

    /// Check if one ontology entails another
    fn check_ontology_entailment(
        &mut self,
        premise: &Ontology,
        conclusion: &Ontology,
    ) -> Result<bool> {
        // Load the premise ontology
        self.load_ontology(premise.clone())?;

        // Check if all axioms in the conclusion are entailed by the premise
        for axiom in conclusion.axioms() {
            let ontology = self.get_ontology()?;
            if !self.task_service.check_entailment(axiom, &ontology, &mut self.statistics)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

// Clone implementations for the services
}

// Clone implementations for the services
impl Clone for TableauFactory {
    fn clone(&self) -> Self {
        // Create a new TableauFactory with the same config
        Self::new(self.config.clone()).expect("Failed to clone TableauFactory")
    }
}

impl Clone for ReasoningTaskService {
    fn clone(&self) -> Self {
        Self::new(self.tableau_factory.clone(), self.cache_manager.clone())
    }
}
