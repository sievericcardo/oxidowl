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
        results::{ClassificationResult, RealizationResult},
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
            cache_manager.clone()
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
            let ontology_ref = ontology.read().unwrap();
            for axiom in ontology_ref.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if &subclass_axiom.subclass == subclass && &subclass_axiom.superclass == superclass {
                        return Ok(true);
                    }
                }
            }
            
            // Check through equivalent classes
            for axiom in ontology_ref.axioms() {
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
            (ClassExpression::Class(class), _) if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" => Ok(true),
            
            // Everything is subclass of Top
            (_, ClassExpression::Class(class)) if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" => Ok(true),
            
            // Nothing is superclass of Top (except Top itself)
            (ClassExpression::Class(subclass_class), ClassExpression::Class(superclass_class)) 
                if subclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" 
                && superclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" => Ok(false),
            
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

    /// Get the ontology being reasoned over
    pub fn get_ontology(&self) -> Option<&OntologyRef> {
        self.ontology.as_ref()
    }

    /// Check if the ontology is consistent
    pub fn is_consistent(&self) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            // TODO: Implement proper tableau-based consistency checking
            // For now, return a simplified consistency check
            Ok(true) // Placeholder - assume consistent for now
        } else {
            Ok(true)  // Empty ontology is consistent
        }
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            // Create a test individual and check if it can be of the given class
            let test_individual = crate::ontology::Individual::named(
                crate::ontology::IRI::new("http://example.org/test#testIndividual")
            );
            
            // Create a tableau node with the test individual having the class
            // Note: This is a simplified implementation - proper tableau creation needed
            // TODO: Implement proper tableau-based satisfiability checking
            
            // For now, return a simplified check
            match class {
                ClassExpression::Class(c) if c.iri.to_string() == "http://www.w3.org/2002/07/owl#Nothing" => Ok(false),
                _ => Ok(true), // Assume other classes are satisfiable for now
            }
        } else {
            // In empty ontology, all classes except owl:Nothing are satisfiable
            match class {
                ClassExpression::Class(c) if c.iri.to_string() == "http://www.w3.org/2002/07/owl#Nothing" => Ok(false),
                _ => Ok(true)
            }
        }
    }

    /// Check if one class is subsumed by another
    pub fn is_subsumed_by(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        self.is_subclass_of(subclass, superclass)
    }

    /// Get superclasses of a class
    pub fn get_superclasses(&self, class: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
                        if equiv_axiom.classes.iter().any(|c| self.classes_equivalent(c, class).unwrap_or(false)) {
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
                crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing")
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
    pub fn get_subclasses(&self, class: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
                        if equiv_axiom.classes.iter().any(|c| self.classes_equivalent(c, class).unwrap_or(false)) {
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
                crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing")
            ));
            if !self.classes_equivalent(class, &owl_nothing).unwrap_or(false) {
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
            let ontology = ontology_ref.read().unwrap();
            let mut equivalent_classes = Vec::new();
            
            // Check explicit equivalent class axioms
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.iter().any(|c| self.classes_equivalent(c, class).unwrap_or(false)) {
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
                if !self.classes_equivalent(&other_class, class).unwrap_or(false) {
                    // Check if both A ⊑ B and B ⊑ A
                    if self.is_subclass_of(class, &other_class)? && 
                       self.is_subclass_of(&other_class, class)? {
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
            let ontology = ontology_ref.read().unwrap();
            let mut instances = Vec::new();
            
            // Look for explicit class assertions
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(class_assertion) = axiom {
                    if self.classes_equivalent(&class_assertion.class, class)? {
                        instances.push(class_assertion.individual.clone());
                    }
                }
            }
            
            if !direct {
                // Also get instances of subclasses
                let subclasses = self.get_subclasses(class, false)?;
                for subclass in subclasses {
                    let subclass_instances = self.get_instances(&subclass, true)?;
                    instances.extend(subclass_instances);
                }
            }
            
            // Remove duplicates
            instances.sort_by_key(|i| i.iri().map(|iri| iri.to_string()).unwrap_or_default());
            instances.dedup_by_key(|i| i.iri().map(|iri| iri.to_string()).unwrap_or_default());
            
            Ok(instances)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get types of an individual
    pub fn get_types(&self, individual: &Individual, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
    pub fn get_object_property_values(&self, individual: &Individual, property: &ObjectPropertyExpression) -> Result<Vec<Individual>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
    pub fn get_data_property_values(&self, individual: &Individual, property: &DataPropertyExpression) -> Result<Vec<crate::ontology::Literal>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
            self.classification_service.classify(ontology, &mut statistics)
        } else {
            Err(Error::OntologyParsing {
                message: "No ontology loaded for classification".into(),
            })
        }
    }

    /// Realize the ontology (compute instance relationships)
    pub fn realize(&mut self) -> Result<RealizationResult> {
        if let Some(ontology) = &self.ontology {
            let mut statistics = ReasoningStatistics::new();
            self.classification_service.realize(ontology, &mut statistics)
        } else {
            Err(Error::OntologyParsing {
                message: "No ontology loaded for realization".into(),
            })
        }
    }

    /// Check if an axiom is entailed by the ontology
    pub fn check_entailment(&self, axiom: &crate::ontology::Axiom, ontology: &Arc<RwLock<crate::ontology::Ontology>>, stats: &mut ReasoningStatistics) -> Result<bool> {
        self.task_service.check_entailment(axiom, ontology, stats)
    }

    /// Explain why an entailment holds
    pub fn explain_entailment(&self, axiom: &crate::ontology::Axiom) -> Result<Vec<crate::ontology::Axiom>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
            
            // Simple explanation: find axioms that directly support the entailment
            let mut explanation = Vec::new();
            
            match axiom {
                crate::ontology::Axiom::SubClassOf(subclass_axiom) => {
                    // Look for axioms that support this subsumption
                    for ont_axiom in ontology.axioms() {
                        match ont_axiom {
                            crate::ontology::Axiom::SubClassOf(ont_subclass) => {
                                // Direct support
                                if ont_subclass.subclass == subclass_axiom.subclass &&
                                   ont_subclass.superclass == subclass_axiom.superclass {
                                    explanation.push(ont_axiom.clone());
                                }
                                // Transitive support: A ⊑ B, B ⊑ C → A ⊑ C
                                else if ont_subclass.superclass == subclass_axiom.subclass {
                                    explanation.push(ont_axiom.clone());
                                }
                                else if ont_subclass.subclass == subclass_axiom.superclass {
                                    explanation.push(ont_axiom.clone());
                                }
                            }
                            crate::ontology::Axiom::EquivalentClasses(equiv) => {
                                // Equivalence support
                                if equiv.classes.contains(&subclass_axiom.subclass) ||
                                   equiv.classes.contains(&subclass_axiom.superclass) {
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
                                if ont_assertion.individual == class_assertion.individual &&
                                   ont_assertion.class == class_assertion.class {
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
            let ontology = ontology_ref.read().unwrap();
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
                                                    if let crate::ontology::Axiom::ClassAssertion(assertion) = ont_axiom {
                                                        if assertion.individual.iri() == ind1.iri() &&
                                                           (assertion.class == *class1 || assertion.class == *class2) {
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
            let mut ontology = ontology_ref.write().unwrap();
            ontology.add_axiom(axiom);
            
            // Clear cache since ontology has changed
            let mut cache = self.cache_manager.write().unwrap();
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
            let mut ontology = ontology_ref.write().unwrap();
            let original_count = ontology.axioms().len();
            ontology.remove_axiom(axiom);
            let removed = ontology.axioms().len() < original_count;
            
            if removed {
                // Clear cache since ontology has changed
                let mut cache = self.cache_manager.write().unwrap();
                cache.clear_all();
            }
            
            // Return the boolean result properly wrapped
            Ok(removed)
        } else {
            Ok(false)  // No ontology to remove from
        }
    }

    /// Get reasoning statistics
    pub fn get_statistics(&self) -> ReasoningStatistics {
        // For now, return default statistics as a placeholder
        // TODO: Implement proper statistics collection
        ReasoningStatistics::default()
    }

    /// Get the size of the current ontology
    pub fn get_ontology_size(&self) -> usize {
        // For now, return 0 as a placeholder
        // TODO: Implement proper ontology size calculation
        if let Some(ref ontology) = self.ontology {
            ontology.read().unwrap().axioms().len()
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
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| crate::Error::Io { message: format!("Failed to read file: {}", e) })?;

        let ontology = match format {
            crate::ontology::OntologyFormat::Functional => {
                let parser = functional::FunctionalParser::new();
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::Manchester => {
                let parser = manchester::ManchesterParser::new(manchester::ManchesterParserConfig::default());
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::Turtle => {
                let parser = turtle::TurtleParser::new();
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::RdfXml => {
                let parser = rdf_xml::RdfXmlParser::new();
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::OwlXml => {
                let parser = owl_xml::OwlXmlParser::new();
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::NTriples => {
                let parser = ntriples::NTriplesParser::new();
                parser.parse(&content)?
            }
            crate::ontology::OntologyFormat::Auto => {
                // Try to determine format from file extension
                let extension = file_path.extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("");
                
                match extension {
                    "owl" | "xml" => {
                        let parser = owl_xml::OwlXmlParser::new();
                        parser.parse(&content)?
                    }
                    "ttl" => {
                        let parser = turtle::TurtleParser::new();
                        parser.parse(&content)?
                    }
                    "ofn" => {
                        let parser = functional::FunctionalParser::new();
                        parser.parse(&content)?
                    }
                    "rdf" => {
                        let parser = rdf_xml::RdfXmlParser::new();
                        parser.parse(&content)?
                    }
                    "nt" => {
                        let parser = ntriples::NTriplesParser::new();
                        parser.parse(&content)?
                    }
                    _ => {
                        // Default to OWL/XML
                        let parser = owl_xml::OwlXmlParser::new();
                        parser.parse(&content)?
                    }
                }
            }
        };

        self.load_ontology(ontology)
    }

    /// Classify object properties
    pub fn classify_object_properties(&mut self) -> Result<super::results::PropertyClassificationResult> {
        // TODO: Implement object property classification
        Ok(super::results::PropertyClassificationResult::new_object_properties(std::collections::HashMap::new()))
    }

    /// Classify data properties  
    pub fn classify_data_properties(&mut self) -> Result<super::results::PropertyClassificationResult> {
        // TODO: Implement data property classification
        Ok(super::results::PropertyClassificationResult::new_data_properties(std::collections::HashMap::new()))
    }

    /// Get unsatisfiable classes
    pub fn get_unsatisfiable_classes(&self) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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
                crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing")
            ));
            if !unsatisfiable.iter().any(|c| self.classes_equivalent(c, &owl_nothing).unwrap_or(false)) {
                unsatisfiable.push(owl_nothing);
            }
            
            Ok(unsatisfiable)
        } else {
            // In empty ontology, only owl:Nothing is unsatisfiable
            Ok(vec![ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing")
            ))])
        }
    }

    /// Get ontology prefixes
    pub fn get_prefixes(&self) -> Result<std::collections::HashMap<String, String>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
            let mut prefixes = std::collections::HashMap::new();
            
            // Add standard prefixes since ontology prefix extraction is not yet implemented
            // TODO: Extract actual prefixes from the ontology
            
            // Add standard prefixes if not present
            if !prefixes.contains_key("rdf") {
                prefixes.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
            }
            if !prefixes.contains_key("rdfs") {
                prefixes.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
            }
            if !prefixes.contains_key("owl") {
                prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
            }
            if !prefixes.contains_key("xsd") {
                prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
            }
            
            Ok(prefixes)
        } else {
            // Return standard prefixes for empty ontology
            let mut prefixes = std::collections::HashMap::new();
            prefixes.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
            prefixes.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
            prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
            prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
            Ok(prefixes)
        }
    }

    /// Dump DL clauses
    pub fn dump_dl_clauses(&self) -> Result<crate::dl_clauses::DLClauseSet> {
        // TODO: Implement DL clause dumping
        Ok(crate::dl_clauses::DLClauseSet::new())
    }

    /// Process OWLlink request
    pub fn process_owllink_request(&self, _request: &str) -> Result<String> {
        // TODO: Implement OWLlink processing
        Ok("<ok/>".to_string())
    }

    /// Execute SPARQL query
    pub fn execute_sparql_query(&self, _query: &str) -> Result<String> {
        // TODO: Implement SPARQL query execution
        Ok("[]".to_string())
    }

    /// Get DL clauses as string
    pub fn get_dl_clauses_string(&self) -> Result<String> {
        // TODO: Implement DL clause string generation
        Ok(String::new())
    }
    
    // Helper methods for reasoning
    
    /// Check if two classes are equivalent
    fn classes_equivalent(&self, class1: &ClassExpression, class2: &ClassExpression) -> Result<bool> {
        // Simple structural equality check
        Ok(class1 == class2)
    }
    
    /// Check if two object properties are equivalent
    fn object_properties_equivalent(&self, prop1: &ObjectPropertyExpression, prop2: &ObjectPropertyExpression) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }
    
    /// Check if two data properties are equivalent
    fn data_properties_equivalent(&self, prop1: &DataPropertyExpression, prop2: &DataPropertyExpression) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }
    
    /// Filter superclasses to only direct ones (remove transitively implied ones)
    fn filter_direct_superclasses(&self, superclasses: Vec<ClassExpression>) -> Result<Vec<ClassExpression>> {
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
    fn filter_direct_subclasses(&self, subclasses: Vec<ClassExpression>) -> Result<Vec<ClassExpression>> {
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
    fn get_all_inferred_superclasses(&self, class: &ClassExpression, direct_superclasses: Vec<ClassExpression>) -> Result<Vec<ClassExpression>> {
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
                let ontology = ontology_ref.read().unwrap();
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                        if self.classes_equivalent(&subclass_axiom.subclass, &current)? {
                            let superclass = &subclass_axiom.superclass;
                            if !all_superclasses.iter().any(|c| self.classes_equivalent(c, superclass).unwrap_or(false)) {
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
    fn get_all_inferred_subclasses(&self, class: &ClassExpression, direct_subclasses: Vec<ClassExpression>) -> Result<Vec<ClassExpression>> {
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
                let ontology = ontology_ref.read().unwrap();
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                        if self.classes_equivalent(&subclass_axiom.superclass, &current)? {
                            let subclass = &subclass_axiom.subclass;
                            if !all_subclasses.iter().any(|c| self.classes_equivalent(c, subclass).unwrap_or(false)) {
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
    fn get_all_classes_in_ontology(&self, _ontology: &crate::ontology::Ontology) -> Result<Vec<ClassExpression>> {
        self.get_all_classes_in_ontology_internal()
    }
    
    /// Get all classes mentioned in the ontology (internal implementation)
    fn get_all_classes_in_ontology_internal(&self) -> Result<Vec<ClassExpression>> {
        let mut classes = Vec::new();
        
        if let Some(ontology_ref) = &self.ontology {
            let ontology = ontology_ref.read().unwrap();
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

    /// Save DL clauses to file
    pub fn save_dl_clauses<P: AsRef<std::path::Path>>(&self, _path: P) -> Result<()> {
        // TODO: Implement DL clause saving
        Ok(())
    }
}
