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
        // For now, return true as a placeholder
        // TODO: Implement proper consistency checking
        Ok(true)
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&self, _class: &ClassExpression) -> Result<bool> {
        // For now, return true as a placeholder
        // TODO: Implement proper satisfiability checking
        Ok(true)
    }

    /// Check if one class is subsumed by another
    pub fn is_subsumed_by(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        self.is_subclass_of(subclass, superclass)
    }

    /// Get superclasses of a class
    pub fn get_superclasses(&self, _class: &ClassExpression, _direct: bool) -> Result<Vec<ClassExpression>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper superclass retrieval
        Ok(Vec::new())
    }

    /// Get subclasses of a class
    pub fn get_subclasses(&self, _class: &ClassExpression, _direct: bool) -> Result<Vec<ClassExpression>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper subclass retrieval
        Ok(Vec::new())
    }

    /// Get equivalent classes
    pub fn get_equivalent_classes(&self, _class: &ClassExpression) -> Result<Vec<ClassExpression>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper equivalent class retrieval
        Ok(Vec::new())
    }

    /// Get instances of a class
    pub fn get_instances(&self, _class: &ClassExpression, _direct: bool) -> Result<Vec<Individual>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper instance retrieval
        Ok(Vec::new())
    }

    /// Get types of an individual
    pub fn get_types(&self, _individual: &Individual, _direct: bool) -> Result<Vec<ClassExpression>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper type retrieval
        Ok(Vec::new())
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(&self, _individual: &Individual, _property: &ObjectPropertyExpression) -> Result<Vec<Individual>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper object property value retrieval
        Ok(Vec::new())
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(&self, _individual: &Individual, _property: &DataPropertyExpression) -> Result<Vec<crate::ontology::Literal>> {
        // For now, return empty list as a placeholder
        // TODO: Implement proper data property value retrieval
        Ok(Vec::new())
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
    pub fn explain_entailment(&self, _axiom: &crate::ontology::Axiom) -> Result<Vec<crate::ontology::Axiom>> {
        // For now, return empty explanation as a placeholder
        // TODO: Implement proper entailment explanation
        Ok(Vec::new())
    }

    /// Explain why the ontology is inconsistent
    pub fn explain_inconsistency(&self) -> Result<Vec<crate::ontology::Axiom>> {
        // For now, return empty explanation as a placeholder
        // TODO: Implement proper inconsistency explanation
        Ok(Vec::new())
    }

    /// Add an axiom to the ontology
    pub fn add_axiom(&mut self, _axiom: crate::ontology::Axiom) -> Result<()> {
        // For now, do nothing as a placeholder
        // TODO: Implement proper axiom addition
        Ok(())
    }

    /// Remove an axiom from the ontology
    pub fn remove_axiom(&mut self, _axiom: &crate::ontology::Axiom) -> Result<bool> {
        // For now, return false as a placeholder
        // TODO: Implement proper axiom removal
        Ok(false)
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
        // TODO: Implement unsatisfiable class detection
        Ok(Vec::new())
    }

    /// Get ontology prefixes
    pub fn get_prefixes(&self) -> Result<std::collections::HashMap<String, String>> {
        // TODO: Implement prefix extraction
        Ok(std::collections::HashMap::new())
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

    /// Save DL clauses to file
    pub fn save_dl_clauses<P: AsRef<std::path::Path>>(&self, _path: P) -> Result<()> {
        // TODO: Implement DL clause saving
        Ok(())
    }
}
