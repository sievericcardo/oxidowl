//! Main reasoner implementation
//!
//! This module provides the primary reasoning interface, coordinating between
//! the tableau algorithm, caching systems, and high-level reasoning tasks.

use crate::{
    cache::{CacheManager},
    config::{ReasonerConfig, TableauAlgorithm},
    core::{
        tableau::{Tableau, TableauBuilder, TableauState},
    },
    ontology::{Ontology, OntologyFormat, ClassExpression, Individual, Axiom, ObjectPropertyExpression, DataPropertyExpression},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use log::{info, warn, debug};

/// Wrapper for different tableau algorithm implementations
pub enum TableauAlgorithmInstance {
    Traditional(Tableau),
    HyperTableau(Box<dyn HyperTableauInterface>), // Add when HyperTableau is ready
}

/// Interface for HyperTableau implementation (placeholder for now)
pub trait HyperTableauInterface: Send + Sync {
    fn run(&mut self) -> Result<TableauState>;
    fn get_node_count(&self) -> usize;
    fn get_backtrack_count(&self) -> usize;
    fn get_max_depth(&self) -> usize;
}

impl TableauAlgorithmInstance {
    /// Run the tableau algorithm
    pub fn run(&mut self) -> Result<TableauState> {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.run(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => hypertableau.run(),
        }
    }
    
    /// Get node count for statistics
    pub fn get_node_count(&self) -> usize {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.get_node_count(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => hypertableau.get_node_count(),
        }
    }
    
    /// Get backtrack count for statistics
    pub fn get_backtrack_count(&self) -> usize {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.get_backtrack_count(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => hypertableau.get_backtrack_count(),
        }
    }
    
    /// Get maximum depth for statistics
    pub fn get_max_depth(&self) -> usize {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.get_max_depth(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => hypertableau.get_max_depth(),
        }
    }
}

/// Common trait for all tableau algorithm implementations
pub trait TableauRunner: Send + Sync {
    /// Run the tableau algorithm for consistency checking
    fn run(&mut self) -> Result<TableauState>;
    
    /// Get node count for statistics
    fn get_node_count(&self) -> usize;
    
    /// Get backtrack count for statistics
    fn get_backtrack_count(&self) -> usize;
    
    /// Get maximum depth for statistics
    fn get_max_depth(&self) -> usize;
    
    /// Check if the tableau is consistent
    fn is_consistent(&self) -> bool;
    
    /// Check if the tableau is completed (no more expansions possible)
    fn is_completed(&self) -> bool;
}

/// Reasoning task types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReasoningTask {
    ConsistencyCheck,
    Satisfiability(ClassExpression),
    Subsumption { subclass: ClassExpression, superclass: ClassExpression },
    Classification,
    Realization,
    InstanceCheck { individual: Individual, class: ClassExpression },
}

/// Results from reasoning operations
#[derive(Debug, Clone)]
pub enum ReasoningResult {
    Boolean(bool),
    Classes(HashSet<ClassExpression>),
    Individuals(HashSet<Individual>),
    ClassificationResult(ClassificationResult),
    RealizationResult(RealizationResult),
}

/// Classification result containing class hierarchy
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>,
}

impl ClassificationResult {
    pub fn new(hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>) -> Self {
        Self { hierarchy }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        writeln!(file, "# Class Hierarchy")?;
        
        for (class, subclasses) in &self.hierarchy {
            writeln!(file, "{:?}:", class)?;
            for subclass in subclasses {
                writeln!(file, "  - {:?}", subclass)?;
            }
        }

        Ok(())
    }
}

/// Realization result containing individual types
#[derive(Debug, Clone)]
pub struct RealizationResult {
    pub types: HashMap<Individual, HashSet<ClassExpression>>,
}

impl RealizationResult {
    pub fn new(types: HashMap<Individual, HashSet<ClassExpression>>) -> Self {
        Self { types }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        writeln!(file, "# Individual Types")?;
        
        for (individual, types) in &self.types {
            writeln!(file, "{:?}:", individual)?;
            for class in types {
                writeln!(file, "  - {:?}", class)?;
            }
        }

        Ok(())
    }
}

/// Factory for creating tableau algorithm instances
pub struct TableauFactory {
    config: ReasonerConfig,
    tableau_builder: TableauBuilder,
}

impl TableauFactory {
    pub fn new(config: ReasonerConfig) -> Result<Self> {
        Ok(Self {
            tableau_builder: TableauBuilder::new(&config.reasoning)?,
            config,
        })
    }
    
    /// Create a tableau runner based on the configured algorithm
    pub fn create_for_consistency(&self, ontology: &Ontology) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                let tableau = self.tableau_builder.build_for_consistency(ontology)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                // Try to create HyperTableau, fall back to Traditional if compilation errors prevent it
                warn!("HyperTableau not yet fully integrated, falling back to Traditional tableau");
                let tableau = self.tableau_builder.build_for_consistency(ontology)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }
    
    /// Create a tableau runner for subsumption checking
    pub fn create_for_subsumption(
        &self, 
        ontology: &Ontology, 
        subclass: &ClassExpression, 
        superclass: &ClassExpression
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                // Convert ClassExpression to string for the current tableau builder interface
                let subclass_str = &format!("{}", subclass);
                let superclass_str = &format!("{}", superclass);
                let tableau = self.tableau_builder.build_for_subsumption(ontology, subclass_str, superclass_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                // For now, fall back to traditional for specific reasoning tasks
                warn!("HyperTableau not yet supported for subsumption checking, using Traditional tableau");
                let subclass_str = &format!("{}", subclass);
                let superclass_str = &format!("{}", superclass);
                let tableau = self.tableau_builder.build_for_subsumption(ontology, subclass_str, superclass_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }
    
    /// Create a tableau runner for satisfiability checking
    pub fn create_for_satisfiability(
        &self, 
        ontology: &Ontology, 
        class_expr: &ClassExpression
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                // Convert ClassExpression to string for the current tableau builder interface
                let class_str = &format!("{}", class_expr);
                let tableau = self.tableau_builder.build_for_satisfiability(ontology, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                warn!("HyperTableau not yet supported for satisfiability checking, using Traditional tableau");
                let class_str = &format!("{}", class_expr);
                let tableau = self.tableau_builder.build_for_satisfiability(ontology, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }
    
    /// Create a tableau runner for instance checking
    pub fn create_for_instance_check(
        &self, 
        ontology: &Ontology, 
        individual: &Individual, 
        class_expr: &ClassExpression
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                // Convert Individual and ClassExpression to string for the current tableau builder interface
                let individual_str = &individual.iri().map(|i| i.to_string()).unwrap_or_else(|| "anonymous".to_string());
                let class_str = &format!("{}", class_expr);
                let tableau = self.tableau_builder.build_for_instance_check(ontology, individual_str, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                warn!("HyperTableau not yet supported for instance checking, using Traditional tableau");
                let individual_str = &individual.iri().map(|i| i.to_string()).unwrap_or_else(|| "anonymous".to_string());
                let class_str = &format!("{}", class_expr);
                let tableau = self.tableau_builder.build_for_instance_check(ontology, individual_str, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }
    
    /// Create HyperTableau or fallback to Traditional if compilation errors prevent it
    fn create_hypertableau_or_fallback(&self, ontology: &Ontology) -> Result<Box<dyn TableauRunner>> {
        warn!("HyperTableau not yet fully integrated, falling back to Traditional tableau");
        let tableau = self.tableau_builder.build_for_consistency(ontology)?;
        Ok(Box::new(TraditionalTableauRunner::new(tableau)))
    }
}

/// Traditional tableau runner wrapper
pub struct TraditionalTableauRunner {
    tableau: Tableau,
}

impl TraditionalTableauRunner {
    pub fn new(tableau: Tableau) -> Self {
        Self { tableau }
    }
}

impl TableauRunner for TraditionalTableauRunner {
    fn run(&mut self) -> Result<TableauState> {
        self.tableau.run()
    }
    
    fn get_node_count(&self) -> usize {
        self.tableau.get_node_count()
    }
    
    fn get_backtrack_count(&self) -> usize {
        self.tableau.get_backtrack_count()
    }
    
    fn get_max_depth(&self) -> usize {
        self.tableau.get_max_depth()
    }
    
    fn is_consistent(&self) -> bool {
        // Check if the tableau reached a consistent state
        !matches!(self.tableau.get_state(), TableauState::Unsatisfiable)
    }
    
    fn is_completed(&self) -> bool {
        // Check if the tableau has completed processing
        matches!(self.tableau.get_state(), TableauState::Satisfiable | TableauState::Unsatisfiable)
    }
}

/// Main reasoner interface
#[derive(Debug)]
pub struct Reasoner {
    /// Reasoning configuration
    config: ReasonerConfig,
    
    /// Current ontology being reasoned over
    ontology: Option<Arc<RwLock<Ontology>>>,
    
    /// Cache manager for reasoning results
    cache_manager: Arc<CacheManager>,
    
    /// Tableau builder for constructing reasoning problems
    tableau_builder: TableauBuilder,
    
    /// Statistics about reasoning operations
    statistics: ReasoningStatistics,
}

/// Statistics about reasoning operations
#[derive(Debug, Default, Clone)]
pub struct ReasoningStatistics {
    /// Number of consistency checks performed
    pub consistency_checks: u64,
    
    /// Number of satisfiability checks performed
    pub satisfiability_checks: u64,
    
    /// Number of subsumption checks performed
    pub subsumption_checks: u64,
    
    /// Total reasoning time
    pub total_reasoning_time: Duration,
    
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    
    /// Number of tableau nodes created
    pub tableau_nodes_created: u64,
    
    /// Number of backtracking operations
    pub backtracking_operations: u64,
    
    /// Maximum tableau depth reached
    pub max_tableau_depth: usize,
}

impl Reasoner {
    /// Create a new reasoner with the given configuration
    pub fn new(config: ReasonerConfig) -> Result<Self> {
        let cache_config = crate::cache::CacheConfig {
            enable_consistency_cache: config.cache.enable_consistency_cache,
            enable_satisfiability_cache: config.cache.enable_satisfiability_cache,
            enable_subsumption_cache: config.cache.enable_subsumption_cache,
            enable_instance_cache: config.cache.enable_instance_cache,
            enable_classification_cache: config.cache.enable_classification_cache,
            enable_realization_cache: config.cache.enable_realization_cache,
            cache_size: config.cache.cache_size,
            ttl: config.cache.ttl,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));
        let tableau_builder = TableauBuilder::new(&config.reasoning)?;
        
        Ok(Self {
            config,
            ontology: None,
            cache_manager,
            tableau_builder,
            statistics: ReasoningStatistics::default(),
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
        
        let ontology = Ontology::from_file(path, Some(format.to_string()))?;
        self.ontology = Some(Arc::new(RwLock::new(ontology)));
        
        // Clear caches when new ontology is loaded
        self.cache_manager.write().unwrap().clear_all();
        
        let load_time = start_time.elapsed();
        info!("Ontology loaded in {:?}", load_time);
        
        Ok(())
    }

    /// Load an ontology from memory
    pub fn load_ontology(&mut self, ontology: Ontology) -> Result<()> {
        info!("Loading ontology from memory");
        self.ontology = Some(Arc::new(RwLock::new(ontology)));
        self.cache_manager.clear_all();
        Ok(())
    }

    /// Get the current ontology
    pub fn get_ontology(&self) -> Result<Arc<RwLock<Ontology>>> {
        self.ontology
            .clone()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))
    }

    /// Check if the current ontology is consistent
    pub fn is_consistent(&mut self) -> Result<bool> {
        let start_time = Instant::now();
        self.statistics.consistency_checks += 1;
        
        info!("Checking ontology consistency");
        
        // Check cache first
        if let Some(ontology) = &self.ontology {
            if let Some(cached_result) = self.cache_manager.read().unwrap().get_consistency_result(ontology) {
                debug!("Consistency result found in cache");
                return Ok(cached_result);
            }
        }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Build tableau for consistency checking
        let mut tableau = self.create_tableau_algorithm(&ontology_guard)?;
        
        // Run tableau algorithm
        let result = self.run_tableau_consistency_check(tableau)?;
        
        // Cache the result
        if let Some(ontology) = &self.ontology {
            self.cache_manager.write().unwrap().cache_consistency_result(ontology, result);
        }
        
        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;
        
        info!("Consistency check completed in {:?}: {}", reasoning_time, result);
        Ok(result)
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&mut self, class_iri: &str) -> Result<bool> {
        let start_time = Instant::now();
        self.statistics.satisfiability_checks += 1;
        
        info!("Checking satisfiability of class: {}", class_iri);
        
        // Handle special OWL classes
        if class_iri.contains("owl#Thing") {
            return Ok(true); // owl:Thing is always satisfiable
        }
        if class_iri.contains("owl#Nothing") {
            return Ok(false); // owl:Nothing is always unsatisfiable
        }
        
        // Check cache first
        // TODO: Implement proper class expression parsing
        // if let Some(class_expr) = self.parse_class_expression(class_iri) {
        //     if let Some(cached_result) = self.cache_manager.read().unwrap().get_satisfiability_result(&class_expr) {
        //         debug!("Satisfiability result found in cache for: {}", class_iri);
        //         return Ok(cached_result);
        //     }
        // }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Build tableau for satisfiability checking
        let tableau = self.create_tableau_algorithm_for_satisfiability(&ontology_guard, class_iri)?;
        
        // Run tableau algorithm
        let result = self.run_tableau_satisfiability_check(tableau)?;
        
        // Cache the result
        // TODO: Implement proper class expression parsing for caching
        // self.cache_manager.write().unwrap().cache_satisfiability_result(class_expr, result);
        
        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;
        
        info!("Satisfiability check for {} completed in {:?}: {}", class_iri, reasoning_time, result);
        Ok(result)
    }

    /// Check if one class subsumes another
    pub fn is_subclass_of(&mut self, subclass: &str, superclass: &str) -> Result<bool> {
        let start_time = Instant::now();
        self.statistics.subsumption_checks += 1;
        
        info!("Checking subsumption: {} ⊑ {}", subclass, superclass);
        
        // Check cache first
        // TODO: Implement proper class expression parsing for caching
        // if let Some(cached_result) = self.cache_manager.read().unwrap().get_subsumption_result(subclass, superclass) {
        //     debug!("Subsumption result found in cache");
        //     return Ok(cached_result);
        // }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Build tableau for subsumption checking
        let tableau = self.create_tableau_algorithm_for_subsumption(&ontology_guard, subclass, superclass)?;
        
        // Run tableau algorithm
        let result = self.run_tableau_subsumption_check(tableau)?;
        
        // Cache the result
        self.cache_manager.store_subsumption_result(
            subclass.to_string(),
            superclass.to_string(),
            result,
        );
        
        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;
        
        info!("Subsumption check completed in {:?}: {}", reasoning_time, result);
        Ok(result)
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        let start_time = Instant::now();
        
        info!("Starting classification");
        
        // Check if we have a cached classification result
        if let Some(cached_result) = self.cache_manager.get_classification_result() {
            debug!("Classification result found in cache");
            return Ok(cached_result);
        }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Get all named classes from the ontology
        let classes: Vec<ClassExpression> = ontology_guard
            .signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut hierarchy = HashMap::new();
        let total_pairs = classes.len() * classes.len();
        let mut checked_pairs = 0;

        info!("Classifying {} classes ({} subsumption checks)", classes.len(), total_pairs);

        // Perform pairwise subsumption checks
        for subclass in &classes {
            let mut superclasses = HashSet::new();

            for superclass in &classes {
                if subclass != superclass {
                    if self.is_subclass_of_expressions(subclass, superclass)? {
                        superclasses.insert(superclass.clone());
                    }
                }
                checked_pairs += 1;

                if checked_pairs % 1000 == 0 {
                    info!("Classification progress: {}/{} checks completed", checked_pairs, total_pairs);
                }
            }

            hierarchy.insert(subclass.clone(), superclasses);
        }

        let result = ClassificationResult::new(hierarchy);
        
        // Cache the result
        self.cache_manager.store_classification_result(result.clone());
        
        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;
        
        info!("Classification completed in {:?}", reasoning_time);
        Ok(result)
    }

    /// Perform realization (find most specific classes for individuals)
    pub fn realize(&mut self) -> Result<RealizationResult> {
        let start_time = Instant::now();
        
        info!("Starting realization");
        
        // Check if we have a cached realization result
        if let Some(cached_result) = self.cache_manager.get_realization_result() {
            debug!("Realization result found in cache");
            return Ok(cached_result);
        }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Get all named individuals and classes
        let individuals: Vec<Individual> = ontology_guard
            .signature
            .individuals
            .iter()
            .cloned()
            .collect();

        let classes: Vec<ClassExpression> = ontology_guard
            .signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut realization = HashMap::new();

        info!("Realizing {} individuals against {} classes", individuals.len(), classes.len());

        for individual in &individuals {
            let mut instance_classes = HashSet::new();

            for class in &classes {
                if self.is_instance_of_expression(individual, class)? {
                    instance_classes.insert(class.clone());
                }
            }

            realization.insert(individual.clone(), instance_classes);
        }

        let result = RealizationResult::new(realization);
        
        // Cache the result
        self.cache_manager.store_realization_result(result.clone());
        
        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;
        
        info!("Realization completed in {:?}", reasoning_time);
        Ok(result)
    }

    /// Check if an individual is an instance of a class
    pub fn is_instance_of(&mut self, individual: &str, class: &str) -> Result<bool> {
        info!("Checking instance relationship: {} ∈ {}", individual, class);
        
        // Check cache first
        if let Some(cached_result) = self.cache_manager.get_instance_result(individual, class) {
            debug!("Instance result found in cache");
            return Ok(cached_result);
        }
        
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        
        // Build tableau for instance checking
        let tableau = self.create_tableau_algorithm_for_instance_check(&ontology_guard, individual, class)?;
        
        // Run tableau algorithm
        let result = self.run_tableau_instance_check(tableau)?;
        
        // Cache the result
        self.cache_manager.store_instance_result(
            individual.to_string(),
            class.to_string(),
            result,
        );
        
        Ok(result)
    }

    /// Execute a SPARQL query against the ontology
    pub fn execute_sparql_query(&self, query: &str) -> Result<String> {
        info!("Executing SPARQL query");
        
        // TODO:  integrate with the SPARQL engine
        // For now, return a placeholder
        Ok("SPARQL query results would be here".to_string())
    }

    /// Process an OWLlink request
    pub fn process_owllink_request(&self, request: &str) -> Result<String> {
        info!("Processing OWLlink request");
        
        // TODO:  integrate with the OWLlink processor
        // For now, return a placeholder
        Ok("OWLlink response would be here".to_string())
    }

    /// Get reasoning statistics
    pub fn get_statistics(&self) -> &ReasoningStatistics {
        &self.statistics
    }

    /// Reset reasoning statistics
    pub fn reset_statistics(&mut self) {
        self.statistics = ReasoningStatistics::default();
    }

    /// Check subsumption between two class expressions
    pub fn is_subsumed_by(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        // Check cache first
        if let Some(cached_result) = self.cache_manager.subsumption().get(subclass, superclass) {
            return Ok(cached_result);
        }

        // Use tableau to check subsumption
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        let tableau = self.tableau_builder.build_for_consistency(&ontology_guard)?;
        let result = tableau.check_subsumption(subclass, superclass)?;

        // Store in cache
        self.cache_manager.subsumption().put(subclass.clone(), superclass.clone(), result);

        Ok(result)
    }

    /// Get all superclasses of a class expression
    pub fn get_superclasses(&self, concept: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut superclasses = Vec::new();
            
            // Get all classes from the signature
            for class in &ontology_guard.signature().classes {
                let class_expr = ClassExpression::Class(class.clone());
                if self.is_subsumed_by(concept, &class_expr)? && concept != &class_expr {
                    superclasses.push(class_expr);
                }
            }
            
            Ok(superclasses)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all subclasses of a class expression
    pub fn get_subclasses(&self, concept: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut subclasses = Vec::new();
            
            // Get all classes from the signature
            for class in &ontology_guard.signature().classes {
                let class_expr = ClassExpression::Class(class.clone());
                if self.is_subsumed_by(&class_expr, concept)? && concept != &class_expr {
                    subclasses.push(class_expr);
                }
            }
            
            Ok(subclasses)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all equivalent classes of a class expression
    pub fn get_equivalent_classes(&self, concept: &ClassExpression) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut equivalent_classes = Vec::new();
            
            // Get all classes from the signature
            for class in &ontology_guard.signature().classes {
                let class_expr = ClassExpression::Class(class.clone());
                if concept != &class_expr {
                    let subsumes_1_2 = self.is_subsumed_by(concept, &class_expr)?;
                    let subsumes_2_1 = self.is_subsumed_by(&class_expr, concept)?;
                    if subsumes_1_2 && subsumes_2_1 {
                        equivalent_classes.push(class_expr);
                    }
                }
            }
            
            Ok(equivalent_classes)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all instances of a class expression
    pub fn get_instances(&self, concept: &ClassExpression, direct: bool) -> Result<Vec<Individual>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut instances = Vec::new();
            
            // Get all individuals from the signature
            for individual in &ontology_guard.signature().individuals {
                // TODO: use tableau reasoning
                instances.push(individual.clone());
            }
            
            Ok(instances)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all types of an individual
    pub fn get_types(&self, individual: &Individual, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut types = Vec::new();
            
            // Get all classes from the signature
            for class in &ontology_guard.signature().classes {
                let class_expr = ClassExpression::Class(class.clone());
                // TODO: use tableau reasoning
                types.push(class_expr);
            }
            
            Ok(types)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(&self, individual: &Individual, property: &ObjectPropertyExpression) -> Result<Vec<Individual>> {
        if let Some(ontology) = &self.ontology {
            // TODO:  use tableau reasoning
            Ok(Vec::new())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(&self, individual: &Individual, property: &DataPropertyExpression) -> Result<Vec<String>> {
        if let Some(ontology) = &self.ontology {
            // TODO:  use tableau reasoning
            Ok(Vec::new())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Add axiom for incremental reasoning
    pub fn add_axiom(&mut self, axiom: &Axiom) -> Result<()> {
        if let Some(ontology) = &mut self.ontology {
            let mut ontology_guard = ontology.write().unwrap();
            ontology_guard.add_axiom(axiom.clone());
            self.cache_manager.clear_all();
            Ok(())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Remove axiom for incremental reasoning
    pub fn remove_axiom(&mut self, axiom: &Axiom) -> Result<()> {
        if let Some(ontology) = &mut self.ontology {
            let mut ontology_guard = ontology.write().unwrap();
            ontology_guard.remove_axiom(axiom);
            self.cache_manager.clear_all();
            Ok(())
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get ontology size
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
        self.cache_manager.clear_all();
        Ok(())
    }

    /// Explain entailment
    pub fn explain_entailment(&self, axiom: &Axiom) -> Result<Vec<Axiom>> {
        // TODO:  use explanation support
        Ok(Vec::new())
    }

    /// Explain inconsistency
    pub fn explain_inconsistency(&self) -> Result<Vec<Axiom>> {
        // TODO:  use explanation support
        Ok(Vec::new())
    }

    // Private methods for running tableau algorithms

    fn run_tableau_consistency_check(&mut self, mut tableau: TableauAlgorithmInstance) -> Result<bool> {
        debug!("Running tableau consistency check");
        
        let result = tableau.run()?;
        
        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = 
            self.statistics.max_tableau_depth.max(tableau.get_max_depth());
        
        match result {
            TableauState::Satisfiable => Ok(true),
            TableauState::Unsatisfiable => Ok(false),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    fn run_tableau_satisfiability_check(&mut self, mut tableau: TableauAlgorithmInstance) -> Result<bool> {
        debug!("Running tableau satisfiability check");
        
        let result = tableau.run()?;
        
        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = 
            self.statistics.max_tableau_depth.max(tableau.get_max_depth());
        
        match result {
            TableauState::Satisfiable => Ok(true),
            TableauState::Unsatisfiable => Ok(false),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    fn run_tableau_subsumption_check(&mut self, mut tableau: TableauAlgorithmInstance) -> Result<bool> {
        debug!("Running tableau subsumption check");
        
        // For subsumption A ⊑ B, we check if A ⊓ ¬B is unsatisfiable
        let result = tableau.run()?;
        
        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = 
            self.statistics.max_tableau_depth.max(tableau.get_max_depth());
        
        match result {
            TableauState::Satisfiable => Ok(false), // A ⊓ ¬B is satisfiable, so A ⊄ B
            TableauState::Unsatisfiable => Ok(true), // A ⊓ ¬B is unsatisfiable, so A ⊑ B
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    /// Run a tableau instance check for individuals
    fn run_tableau_instance_check(&mut self, mut tableau: TableauAlgorithmInstance) -> Result<bool> {
        debug!("Running tableau instance check");
        
        // For instance checking a ∈ C, we check if {a} ⊓ ¬C is unsatisfiable
        let result = tableau.run()?;
        
        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = 
            self.statistics.max_tableau_depth.max(tableau.get_max_depth());
        
        match result {
            TableauState::Satisfiable => Ok(false),
            TableauState::Unsatisfiable => Ok(true),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    /// Check if one class expression is a subclass of another
    fn is_subclass_of_expressions(&mut self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        // For now, delegate to existing tableau-based subsumption checking
        if let (ClassExpression::Class(sub), ClassExpression::Class(sup)) = (subclass, superclass) {
            self.is_subclass_of(&sub.iri.to_string(), &sup.iri.to_string())
        } else {
            // For complex expressions, we'd need more sophisticated reasoning
            // TODO: implement proper complex expression reasoning
            Ok(false)
        }
    }

    /// Check if an individual is an instance of a class expression
    fn is_instance_of_expression(&mut self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        // For now, delegate to existing instance checking for named classes
        if let ClassExpression::Class(cls) = class {
            self.is_instance_of(&individual.iri().map(|i| i.to_string()).unwrap_or_else(|| "anonymous".to_string()), &cls.iri.to_string())
        } else {
            // For complex expressions, we'd need more sophisticated reasoning
            // TODO: implement proper complex expression reasoning
            Ok(false)
        }
    }

    // Private methods for tableau algorithm creation

    /// Create a tableau algorithm instance based on configuration
    fn create_tableau_algorithm(&self, ontology: &Ontology) -> Result<TableauAlgorithmInstance> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                let tableau = self.tableau_builder.build_for_consistency(ontology)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                // Try to create HyperTableau, fall back to Traditional if not available
                self.create_hypertableau_instance(ontology)
            }
        }
    }
    
    /// Create a tableau algorithm instance for subsumption checking
    fn create_tableau_algorithm_for_subsumption(
        &self,
        ontology: &Ontology,
        subclass: &str,
        superclass: &str,
    ) -> Result<TableauAlgorithmInstance> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                let tableau = self.tableau_builder.build_for_subsumption(ontology, subclass, superclass)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!("HyperTableau not yet supported for subsumption checking, using Traditional tableau");
                let tableau = self.tableau_builder.build_for_subsumption(ontology, subclass, superclass)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
        }
    }
    
    /// Create a tableau algorithm instance for satisfiability checking
    fn create_tableau_algorithm_for_satisfiability(
        &self,
        ontology: &Ontology,
        class_iri: &str,
    ) -> Result<TableauAlgorithmInstance> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                let tableau = self.tableau_builder.build_for_satisfiability(ontology, class_iri)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!("HyperTableau not yet supported for satisfiability checking, using Traditional tableau");
                let tableau = self.tableau_builder.build_for_satisfiability(ontology, class_iri)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
        }
    }
    
    /// Create a tableau algorithm instance for instance checking
    fn create_tableau_algorithm_for_instance_check(
        &self,
        ontology: &Ontology,
        individual: &str,
        class: &str,
    ) -> Result<TableauAlgorithmInstance> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                let tableau = self.tableau_builder.build_for_instance_check(ontology, individual, class)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!("HyperTableau not yet supported for instance checking, using Traditional tableau");
                let tableau = self.tableau_builder.build_for_instance_check(ontology, individual, class)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
        }
    }

    /// Create a HyperTableau instance or fall back to Traditional
    fn create_hypertableau_instance(&self, ontology: &Ontology) -> Result<TableauAlgorithmInstance> {
        use crate::core::hypertableau::HyperTableau;
        use crate::core::blocking::AnywhereBlocking;
        
        info!("Creating HyperTableau instance for reasoning");
        
        // Create a blocking checker (use default for now)
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
        // Create HyperTableau instance
        let mut hypertableau = HyperTableau::new(self.config.reasoning.clone(), blocking_checker)?;
        
        // Initialize with the ontology
        hypertableau.initialize(ontology)?;
        
        Ok(TableauAlgorithmInstance::HyperTableau(Box::new(hypertableau)))
    }
}