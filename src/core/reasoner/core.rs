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
        // For now, provide a stub implementation that delegates to the original logic
        // In a complete implementation, this would use the tableau algorithm
        Ok(false) // Placeholder - needs proper tableau implementation
    }

    /// Get superclasses of a given class (returns ClassExpression vector like the original)
    pub fn get_superclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - needs proper classification algorithm
        Ok(vec![])
    }

    /// Get subclasses of a given class (returns ClassExpression vector like the original)
    pub fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - needs proper classification algorithm
        Ok(vec![])
    }

    /// Get equivalent classes of a given class (returns ClassExpression vector like the original)
    pub fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - needs proper classification algorithm
        Ok(vec![])
    }

    /// Get instances of a given class (returns Individual vector like the original)
    pub fn get_instances(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<Individual>> {
        // Placeholder implementation - needs proper realization algorithm
        Ok(vec![])
    }

    /// Get types of a given individual (returns ClassExpression vector like the original)
    pub fn get_types(
        &self,
        individual: &Individual,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - needs proper realization algorithm
        Ok(vec![])
    }

    /// Get object property values for an individual (returns Individual vector like the original)
    pub fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<Individual>> {
        // Placeholder implementation - needs proper property value retrieval
        Ok(vec![])
    }

    /// Get data property values for an individual (returns Literal vector like the original)
    pub fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<Vec<crate::ontology::Literal>> {
        // Placeholder implementation - needs proper property value retrieval
        Ok(vec![])
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
