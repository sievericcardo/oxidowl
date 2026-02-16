//! Classification and realization operations
//!
//! This module implements complex reasoning operations like building class hierarchies,
//! property classification, and individual type inference.

use crate::{
    Result,
    cache::CacheManager,
    config::PerformanceConfig,
    core::{
        lock_helpers::{read_lock, write_lock},
        reasoner::{
            datatype_validation::DatatypeValidator,
            parallel_classification::ParallelClassificationScheduler,
            results::{ClassificationResult, PropertyClassificationResult, RealizationResult},
            statistics::ReasoningStatistics,
            tasks::ReasoningTaskService,
        },
        saturation::{SaturationConfig, SaturationEngine, SaturationStatus},
    },
    ontology::{
        ClassExpression, DataPropertyExpression, IRI, Individual, ObjectPropertyExpression,
        Ontology, OntologyRef,
    },
};
use log::{debug, info};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Instant,
};

/// Service for complex reasoning operations like classification and realization
#[derive(Debug)]
pub struct ClassificationService {
    task_service: ReasoningTaskService,
    cache_manager: Arc<RwLock<CacheManager>>,
    datatype_validator: DatatypeValidator,
    saturation_engine: SaturationEngine,
}

impl ClassificationService {
    /// Create a new classification service
    pub fn new(
        task_service: ReasoningTaskService,
        cache_manager: Arc<RwLock<CacheManager>>,
    ) -> Self {
        Self {
            task_service,
            cache_manager,
            datatype_validator: DatatypeValidator::new(),
            saturation_engine: SaturationEngine::new(SaturationConfig::default()),
        }
    }

    /// Create a new classification service with custom saturation config
    pub fn with_saturation_config(
        task_service: ReasoningTaskService,
        cache_manager: Arc<RwLock<CacheManager>>,
        saturation_config: SaturationConfig,
    ) -> Self {
        Self {
            task_service,
            cache_manager,
            datatype_validator: DatatypeValidator::new(),
            saturation_engine: SaturationEngine::new(saturation_config),
        }
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<ClassificationResult> {
        let start_time = Instant::now();

        info!("Starting classification with saturation-based optimization");

        // Check if we have a cached classification result
        if let Some(cached_result) =
            read_lock(&self.cache_manager, "classification: reading cache")?
                .get_classification_result(ontology)
        {
            debug!("Classification result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = read_lock(ontology, "classification: reading ontology for classify")?;

        // Get all named classes from the ontology
        let signature = ontology_guard.signature()?;
        let mut classes: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        // Add owl:Thing if not present
        let owl_thing = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::owl_thing().to_url()?.into(),
        });
        if !classes.contains(&owl_thing) {
            classes.push(owl_thing.clone());
        }

        // Process inferred classes from complex axioms
        let inferred_classes = self.discover_inferred_classes(&ontology_guard)?;
        classes.extend(inferred_classes);

        info!("Classifying {} classes", classes.len());

        // === Extract told subsumers from axioms ===
        let phase1_start = Instant::now();
        let mut hierarchy = self.extract_told_subsumers(&classes, &ontology_guard)?;
        info!("(told subsumers) completed in {:?}", phase1_start.elapsed());

        // === PHASE 2: Saturation-based subsumption ===
        let phase2_start = Instant::now();
        let saturation_result = self.saturation_engine.saturate_ontology(&ontology_guard)?;
        info!(
            "Phase 2 (saturation) completed in {:?} - {} concepts complete, {} require tableau",
            phase2_start.elapsed(),
            saturation_result.statistics.concepts_complete,
            saturation_result.statistics.concepts_requiring_tableau
        );

        // Add subsumers discovered through saturation
        for (concept, subsumers) in &saturation_result.subsumptions {
            let entry = hierarchy.entry(concept.clone()).or_insert_with(HashSet::new);
            entry.extend(subsumers.clone());
        }

        // === PHASE 3: Tableau expansion for complex cases ===
        let phase3_start = Instant::now();
        let mut tableau_checks = 0;
        let mut tableau_pairs = Vec::new();

        // Identify pairs that need tableau expansion
        for subclass in &classes {
            if let Some(node) = saturation_result.get_node(subclass) {
                if node.status == SaturationStatus::RequiresFullTableau || node.status == SaturationStatus::NonDeterministic {
                    for superclass in &classes {
                        if subclass != superclass {
                            // Check if not already determined by saturation
                            if !saturation_result.subsumes(superclass, subclass) {
                                tableau_pairs.push((subclass.clone(), superclass.clone()));
                            }
                        }
                    }
                }
            }
        }

        info!("Phase 3: {} pairs require tableau expansion", tableau_pairs.len());

        // Store length before consuming tableau_pairs
        let total_tableau_pairs = tableau_pairs.len();

        // Get performance configuration for parallel execution
        let perf_config = PerformanceConfig::from_env();
        let use_parallel = perf_config.enable_lock_free && total_tableau_pairs > 100;
        
        if use_parallel {
            info!("Using parallel classification for {} subsumption checks", total_tableau_pairs);
            
            // Create parallel scheduler
            let scheduler = ParallelClassificationScheduler::new(perf_config);
            
            // Build told subsumers map for dependency tracking
            let mut told_subsumers = std::collections::HashMap::new();
            for (subclass, superclasses) in &hierarchy {
                told_subsumers.insert(subclass.clone(), superclasses.clone());
            }
            
            // Schedule all tasks with priority ordering
            let tasks = scheduler.schedule_classification_tasks(&classes, &told_subsumers);
            
            // Filter to only the pairs that need tableau expansion
            let filtered_tasks: Vec<_> = tasks.into_iter()
                .filter(|task| {
                    tableau_pairs.iter().any(|(s, p)| s == &task.subclass && p == &task.superclass)
                })
                .collect();
            
            // Execute parallel subsumption checks
            let results = scheduler.execute_parallel(filtered_tasks, |sub, sup| {
                self.check_subsumption_from_axioms(&sub, &sup, &ontology_guard)
            })?;
            
            // Collect results into hierarchy
            for result in results {
                if result.holds {
                    let entry = hierarchy.entry(result.subclass).or_insert_with(HashSet::new);
                    entry.insert(result.superclass);
                }
            }
            
            tableau_checks = total_tableau_pairs;
        } else {
            info!("Using sequential classification for {} subsumption checks", total_tableau_pairs);
            
            // Perform tableau expansion for remaining pairs (sequential fallback)
            for (subclass, superclass) in tableau_pairs {
                if self.check_subsumption_from_axioms(&subclass, &superclass, &ontology_guard)? {
                    let entry = hierarchy.entry(subclass).or_insert_with(HashSet::new);
                    entry.insert(superclass);
                }
                tableau_checks += 1;

                if tableau_checks % 100 == 0 {
                    debug!("Tableau checks progress: {}/{}", tableau_checks, total_tableau_pairs);
                }
            }
        }

        info!("Phase 3 (tableau expansion) completed in {:?} with {} checks", 
              phase3_start.elapsed(), tableau_checks);

        // Extract ontology IRI before dropping the read lock
        let ontology_iri = ontology_guard.iri.as_ref().map(|iri| iri.to_string());

        // Drop the read lock before creating result
        drop(ontology_guard);

        let result = ClassificationResult::new_with_iri(hierarchy, ontology_iri);

        // Cache the result
        write_lock(
            &self.cache_manager,
            "classification: storing classification result",
        )?
        .store_classification_result(ontology, result.clone());

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!(
            "Classification completed in {:?} ({}x speedup expected from saturation)",
            reasoning_time,
            if tableau_checks > 0 { classes.len() * classes.len() / tableau_checks } else { 1 }
        );
        Ok(result)
    }

    /// Extract told subsumers directly from axioms
    fn extract_told_subsumers(
        &self,
        classes: &[ClassExpression],
        ontology: &Ontology,
    ) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>> {
        let mut hierarchy = HashMap::new();

        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) => {
                    // Direct told subsumption
                    if classes.contains(&subclass_axiom.subclass) {
                        let entry = hierarchy
                            .entry(subclass_axiom.subclass.clone())
                            .or_insert_with(HashSet::new);
                        entry.insert(subclass_axiom.superclass.clone());
                    }
                }
                crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) => {
                    // Equivalent classes are mutual subsumers
                    for class1 in &equiv_axiom.classes {
                        if classes.contains(class1) {
                            let entry = hierarchy.entry(class1.clone()).or_insert_with(HashSet::new);
                            for class2 in &equiv_axiom.classes {
                                if class1 != class2 {
                                    entry.insert(class2.clone());
                                }
                            }
                        }
                    }
                }
                crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) => {
                    // Disjoint union members are subclasses of the union
                    for disjoint_class in &disjoint_union.disjoint_classes {
                        if classes.contains(disjoint_class) {
                            let entry = hierarchy
                                .entry(disjoint_class.clone())
                                .or_insert_with(HashSet::new);
                            entry.insert(disjoint_union.class.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // Ensure all classes have an entry (even if empty)
        for class in classes {
            hierarchy.entry(class.clone()).or_insert_with(HashSet::new);
        }

        Ok(hierarchy)
    }

    /// Classify object properties
    pub fn classify_object_properties(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting object property classification");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for object property classification",
        )?;

        // Get all object properties from the ontology
        let signature = ontology_guard.signature()?;
        let properties: Vec<ObjectPropertyExpression> = signature
            .object_properties
            .iter()
            .map(|p| ObjectPropertyExpression::ObjectProperty(p.clone()))
            .collect();

        let mut hierarchy = HashMap::new();

        info!("Classifying {} object properties", properties.len());

        // Build property hierarchy using subsumption checks
        for property in &properties {
            let mut superproperties = HashSet::new();

            for superproperty in &properties {
                if property != superproperty {
                    // Check if property is subproperty of superproperty
                    if self.is_subproperty_of(property, superproperty, &ontology_guard)? {
                        superproperties.insert(superproperty.clone());
                    }
                }
            }

            hierarchy.insert(property.clone(), superproperties);
        }

        let result = PropertyClassificationResult::new_object_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Object property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Classify data properties
    pub fn classify_data_properties(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting data property classification");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for data property classification",
        )?;

        // Get all data properties from the ontology
        let signature = ontology_guard.signature()?;
        let properties: Vec<DataPropertyExpression> = signature
            .data_properties
            .iter()
            .map(|p| DataPropertyExpression::DataProperty(p.clone()))
            .collect();

        let mut hierarchy = HashMap::new();

        info!("Classifying {} data properties", properties.len());

        // Build property hierarchy using subsumption checks
        for property in &properties {
            let mut superproperties = HashSet::new();

            for superproperty in &properties {
                if property != superproperty {
                    // Check if property is subproperty of superproperty
                    if self.is_data_subproperty_of(property, superproperty, &ontology_guard)? {
                        superproperties.insert(superproperty.clone());
                    }
                }
            }

            hierarchy.insert(property.clone(), superproperties);
        }

        let result = PropertyClassificationResult::new_data_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Data property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Perform realization (find most specific classes for individuals)
    pub fn realize(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<RealizationResult> {
        let start_time = Instant::now();

        info!("Starting realization");

        // Check if we have a cached realization result
        if let Some(cached_result) = read_lock(
            &self.cache_manager,
            "classification: reading cache for realization",
        )?
        .get_realization_result(ontology)
        {
            debug!("Realization result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard =
            read_lock(ontology, "classification: reading ontology for realization")?;

        // Get all named individuals and classes
        let individuals: Vec<Individual> = ontology_guard
            .signature()
            .expect("Failed to extract ontology signature")
            .individuals
            .clone();

        let classes: Vec<ClassExpression> = ontology_guard
            .signature()
            .expect("Failed to extract ontology signature")
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut realization = HashMap::new();

        info!(
            "Realizing {} individuals against {} classes",
            individuals.len(),
            classes.len()
        );

        for individual in &individuals {
            let mut instance_classes = HashSet::new();

            for class in &classes {
                let mut is_instance = false;

                // Try direct datatype reasoning first
                if let Ok(true) =
                    self.check_instance_with_datatype_reasoning(individual, class, &ontology_guard)
                {
                    is_instance = true;
                }

                // If not determined by datatype reasoning, use tableau
                if !is_instance {
                    is_instance = self
                        .task_service
                        .check_instance(individual, class, ontology, statistics)?;
                }

                if is_instance {
                    instance_classes.insert(class.clone());
                }
            }

            realization.insert(individual.clone(), instance_classes);
        }

        let result = RealizationResult::new(realization);

        // Cache the result
        write_lock(
            &self.cache_manager,
            "classification: storing realization result",
        )?
        .store_realization_result(ontology, result.clone());

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Realization completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Get all unsatisfiable classes (equivalent to owl:Nothing)
    pub fn get_unsatisfiable_classes(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<ClassExpression>> {
        let start_time = Instant::now();

        info!("Finding unsatisfiable classes");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for unsatisfiable classes",
        )?;

        // Get all named classes from the ontology
        let signature = ontology_guard.signature()?;
        let classes: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut unsatisfiable_classes = Vec::new();

        for class in &classes {
            if let ClassExpression::Class(cls) = class {
                if !self.task_service.check_satisfiability(
                    &cls.iri.to_string(),
                    ontology,
                    statistics,
                )? {
                    unsatisfiable_classes.push(class.clone());
                }
            }
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!(
            "Found {} unsatisfiable classes in {reasoning_time:?}",
            unsatisfiable_classes.len()
        );
        Ok(unsatisfiable_classes)
    }

    /// Get all superclasses of a class expression
    pub fn get_superclasses(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        _direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_superclasses",
        )?;
        let mut superclasses = Vec::new();

        // Get all classes from the signature
        for class in &ontology_guard
            .signature()
            .expect("Failed to extract ontology signature")
            .classes
        {
            let class_expr = ClassExpression::Class(class.clone());
            if self.task_service.check_subsumption_expressions(
                concept,
                &class_expr,
                ontology,
                statistics,
            )? && concept != &class_expr
            {
                superclasses.push(class_expr);
            }
        }

        Ok(superclasses)
    }

    /// Get all subclasses of a class expression
    pub fn get_subclasses(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        _direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_subclasses",
        )?;
        let mut subclasses = Vec::new();

        if let ClassExpression::Class(target_class) = concept {
            debug!("Looking for subclasses of: {}", target_class.iri.as_str());

            // Look for SubClassOf axioms in the ontology
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                    // Check if the superclass matches our target
                    if let ClassExpression::Class(super_class) = &subclass_axiom.superclass {
                        if target_class.iri.as_str() == super_class.iri.as_str() {
                            subclasses.push(subclass_axiom.subclass.clone());
                        }
                    }
                }
            }
        }

        Ok(subclasses)
    }

    /// Get all equivalent classes of a class expression
    pub fn get_equivalent_classes(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<ClassExpression>> {
        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_equivalent_classes",
        )?;
        let mut equivalent_classes = Vec::new();

        // Special handling for union queries - check DisjointUnion axioms
        if let ClassExpression::ObjectUnionOf(union_classes) = concept {
            debug!(
                "Processing union query with {} classes",
                union_classes.len()
            );
            // Find any DisjointUnion axiom that matches this union
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                    // Check if the union in the query matches the disjoint classes in this axiom
                    if self.union_matches_disjoint_classes(
                        union_classes,
                        &disjoint_union.disjoint_classes,
                    ) {
                        equivalent_classes.push(disjoint_union.class.clone());
                    }
                }
            }
        }

        // General case: Check all classes from the signature for bidirectional subsumption
        for class in &ontology_guard
            .signature()
            .expect("Failed to extract ontology signature")
            .classes
        {
            let class_expr = ClassExpression::Class(class.clone());
            if concept != &class_expr {
                let subsumes_1_2 = self.task_service.check_subsumption_expressions(
                    concept,
                    &class_expr,
                    ontology,
                    statistics,
                )?;
                let subsumes_2_1 = self.task_service.check_subsumption_expressions(
                    &class_expr,
                    concept,
                    ontology,
                    statistics,
                )?;
                if subsumes_1_2 && subsumes_2_1 {
                    equivalent_classes.push(class_expr);
                }
            }
        }

        Ok(equivalent_classes)
    }

    /// Get all instances of a class expression
    pub fn get_instances(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        _direct: bool,
    ) -> Result<Vec<Individual>> {
        let individuals = {
            let ontology_guard = read_lock(
                ontology,
                "classification: reading ontology for get_instances individuals",
            )?;
            // Get all individuals from the signature
            ontology_guard
                .signature()
                .expect("Failed to extract ontology signature")
                .individuals
                .clone()
        }; // Drop the read lock here

        let mut instances = Vec::new();

        for individual in &individuals {
            // First, try datatype reasoning for certain class expressions
            let mut is_instance = false;

            // Try direct datatype reasoning first
            {
                let ontology_guard = read_lock(
                    ontology,
                    "classification: reading ontology for get_instances datatype reasoning",
                )?;
                if let Ok(true) = self.check_instance_with_datatype_reasoning(
                    individual,
                    concept,
                    &ontology_guard,
                ) {
                    is_instance = true;
                }
            }

            // If not determined by datatype reasoning, use tableau
            if !is_instance {
                is_instance = self
                    .task_service
                    .check_instance(individual, concept, ontology, statistics)?;
            }

            if is_instance {
                instances.push(individual.clone());
            }
        }

        Ok(instances)
    }

    // Private helper methods

    /// Discover inferred classes from complex axioms (equivalent classes, union classes, etc.)
    fn discover_inferred_classes(&self, _ontology: &Ontology) -> Result<Vec<ClassExpression>> {
        let inferred_classes = Vec::new();

        // For now, let's simplify this to avoid complex pattern matching issues
        // The main classification should handle the basic relationships

        Ok(inferred_classes)
    }

    /// Check subsumption using axioms from the ontology
    fn check_subsumption_from_axioms(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        let mut visited = HashSet::new();
        self.check_subsumption_from_axioms_with_visited(
            subclass,
            superclass,
            ontology,
            &mut visited,
        )
    }

    fn check_subsumption_from_axioms_with_visited(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        visited: &mut HashSet<(ClassExpression, ClassExpression)>,
    ) -> Result<bool> {
        // Prevent infinite recursion
        let key = (subclass.clone(), superclass.clone());
        if visited.contains(&key) {
            return Ok(false);
        }
        visited.insert(key);

        // First check for direct SubClassOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                if subclass_axiom.subclass == *subclass && subclass_axiom.superclass == *superclass
                {
                    return Ok(true);
                }
            }
        }

        // Check for equivalent classes
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                let classes = &equiv_axiom.classes;
                if classes.contains(subclass) && classes.contains(superclass) {
                    return Ok(true);
                }

                // If subclass is equivalent to something that is a subclass of superclass
                if classes.contains(subclass) {
                    for equiv_class in classes {
                        if equiv_class != subclass {
                            if self.check_subsumption_from_axioms_with_visited(
                                equiv_class,
                                superclass,
                                ontology,
                                visited,
                            )? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        // Check for DisjointUnion relationships
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                // If subclass is one of the disjoint classes, it's a subclass of the union class
                if disjoint_union.disjoint_classes.contains(subclass) {
                    if disjoint_union.class == *superclass {
                        return Ok(true);
                    }
                    // Also check if the union class is a subclass of superclass
                    if self.check_subsumption_from_axioms_with_visited(
                        &disjoint_union.class,
                        superclass,
                        ontology,
                        visited,
                    )? {
                        return Ok(true);
                    }
                }

                // If subclass is the union class and superclass is owl:Thing or a superclass of the union
                if disjoint_union.class == *subclass {
                    if let ClassExpression::Class(super_cls) = superclass {
                        if super_cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Check for transitive subsumption
        self.check_transitive_subsumption_with_visited(subclass, superclass, ontology, visited)
    }

    /// Check transitive subsumption relationships
    fn check_transitive_subsumption_with_visited(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        visited: &mut HashSet<(ClassExpression, ClassExpression)>,
    ) -> Result<bool> {
        // Prevent infinite recursion
        let key = (subclass.clone(), superclass.clone());
        if visited.contains(&key) {
            return Ok(false);
        }
        visited.insert(key);

        // Use a simple depth-first search to find transitive relationships
        let mut local_visited = HashSet::new();
        let mut stack = vec![subclass.clone()];

        while let Some(current) = stack.pop() {
            if local_visited.contains(&current) {
                continue;
            }
            local_visited.insert(current.clone());

            // Check direct subsumption
            for axiom in ontology.axioms() {
                if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if subclass_axiom.subclass == current {
                        if subclass_axiom.superclass == *superclass {
                            return Ok(true);
                        }
                        // Add to stack for further exploration
                        if !local_visited.contains(&subclass_axiom.superclass) {
                            stack.push(subclass_axiom.superclass.clone());
                        }
                    }
                }
            }
        }

        // Check if subclass is ultimately a subclass of owl:Thing
        if let ClassExpression::Class(super_class) = superclass {
            if super_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                // Everything is a subclass of owl:Thing except owl:Nothing
                if let ClassExpression::Class(sub_class) = subclass {
                    return Ok(sub_class.iri.as_str() != "http://www.w3.org/2002/07/owl#Nothing");
                }
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if a union expression matches the disjoint classes in a `DisjointUnion` axiom
    fn union_matches_disjoint_classes(
        &self,
        union_classes: &[ClassExpression],
        disjoint_classes: &[ClassExpression],
    ) -> bool {
        // Recursively extract all classes from the union (handling nested unions)
        let mut union_iris = HashSet::new();
        for expr in union_classes {
            self.extract_all_union_classes(expr, &mut union_iris);
        }

        let disjoint_iris: HashSet<String> = disjoint_classes
            .iter()
            .filter_map(|expr| {
                if let ClassExpression::Class(class) = expr {
                    Some(class.iri.to_string())
                } else {
                    None
                }
            })
            .collect();

        // The union matches if it contains exactly the same classes as the disjoint union
        union_iris == disjoint_iris && !union_iris.is_empty()
    }

    /// Recursively extract all class IRIs from a union expression (handling nested unions)
    fn extract_all_union_classes(&self, expr: &ClassExpression, result: &mut HashSet<String>) {
        self.extract_all_union_classes_with_depth(expr, result, 0);
    }
    
    /// Maximum recursion depth for union extraction to prevent stack overflow
    const MAX_UNION_EXTRACTION_DEPTH: usize = 500;
    
    fn extract_all_union_classes_with_depth(&self, expr: &ClassExpression, result: &mut HashSet<String>, depth: usize) {
        // Prevent stack overflow on deeply nested unions
        if depth > Self::MAX_UNION_EXTRACTION_DEPTH {
            return;
        }
        
        match expr {
            ClassExpression::Class(class) => {
                result.insert(class.iri.to_string());
            }
            ClassExpression::ObjectUnionOf(union_classes) => {
                for nested_expr in union_classes {
                    self.extract_all_union_classes_with_depth(nested_expr, result, depth + 1);
                }
            }
            _ => {
                // For other expressions, we don't extract classes
            }
        }
    }

    /// Check if one object property is a subproperty of another
    fn is_subproperty_of(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for direct SubObjectPropertyOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom {
                if &sub_axiom.sub_property == subproperty
                    && &sub_axiom.super_property == superproperty
                {
                    return Ok(true);
                }
            }
        }

        // Add sophisticated property chain reasoning
        self.check_property_chain_entailment(subproperty, superproperty, ontology)
    }

    /// Check property chain entailment for complex property relationships
    fn check_property_chain_entailment(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for property chain axioms that might imply this relationship
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom {
                // Check if the superproperty is involved in property chains
                if &sub_axiom.super_property == superproperty {
                    if let ObjectPropertyExpression::PropertyChain(chain) = &sub_axiom.sub_property
                    {
                        // Check if subproperty is part of this chain or can be derived from it
                        if self.property_in_chain_or_derivable(subproperty, chain, ontology)? {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check if a property is in a chain or derivable from it
    fn property_in_chain_or_derivable(
        &self,
        property: &ObjectPropertyExpression,
        chain: &[ObjectPropertyExpression],
        ontology: &Ontology,
    ) -> Result<bool> {
        // Direct membership in chain
        if chain.contains(property) {
            return Ok(true);
        }

        // Check if property is equivalent to the entire chain
        if chain.len() == 1 && &chain[0] == property {
            return Ok(true);
        }

        // Check if property is a subproperty of any property in the chain
        for chain_prop in chain {
            if self.is_subproperty_of(property, chain_prop, ontology)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if one data property is a subproperty of another
    fn is_data_subproperty_of(
        &self,
        subproperty: &DataPropertyExpression,
        superproperty: &DataPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for direct SubDataPropertyOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubDataPropertyOf(sub_axiom) = axiom {
                if &sub_axiom.sub_property == subproperty
                    && &sub_axiom.super_property == superproperty
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get types of a given individual
    pub fn get_types(
        &self,
        individual: &str,
        direct: bool,
        ontology: &OntologyRef,
        _statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting types for individual: {individual}");

        let ontology_guard = read_lock(ontology, "classification: reading ontology for get_types")?;
        let mut types = Vec::new();

        // Find all class assertions for this individual
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                if let crate::ontology::Individual::Named(ind) = &assertion.individual {
                    if ind.iri.as_str() == individual {
                        if let ClassExpression::Class(class) = &assertion.class {
                            types.push(class.iri.to_string());
                        }
                    }
                }
            }
        }

        // If direct is false, also infer types through subclass relationships
        if !direct {
            let mut all_types = HashSet::new();
            for t in &types {
                all_types.insert(t.clone());

                // Get all superclasses of this type
                // Convert string to ClassExpression for the call
                let class_expr = ClassExpression::Class(crate::ontology::Class {
                    iri: crate::ontology::IRI::new(t)
                        .to_url()
                        .expect("Failed to convert IRI to URL")
                        .into(),
                });
                if let Ok(supertypes) =
                    self.get_superclasses(&class_expr, ontology, _statistics, false)
                {
                    for supertype in supertypes {
                        if let ClassExpression::Class(class) = supertype {
                            all_types.insert(class.iri.to_string());
                        }
                    }
                }
            }
            types = all_types.into_iter().collect();
        }

        types.sort();
        Ok(types)
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(
        &self,
        individual: &str,
        property: &str,
        ontology: &OntologyRef,
        _statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting object property values for {individual} -> {property}");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_object_property_values",
        )?;
        let mut values = Vec::new();

        // Find all object property assertions for this individual and property
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) = axiom {
                if let (
                    crate::ontology::Individual::Named(subj),
                    crate::ontology::Individual::Named(obj),
                ) = (&assertion.source, &assertion.target)
                {
                    if subj.iri.as_str() == individual {
                        if let ObjectPropertyExpression::ObjectProperty(prop) = &assertion.property
                        {
                            if prop.iri.as_str() == property {
                                values.push(obj.iri.to_string());
                            }
                        }
                    }
                }
            }
        }

        values.sort();
        Ok(values)
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(
        &self,
        individual: &str,
        property: &str,
        ontology: &OntologyRef,
        _statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting data property values for {individual} -> {property}");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_data_property_values",
        )?;
        let mut values = Vec::new();

        // Find all data property assertions for this individual and property
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom {
                if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                    if subj.iri.as_str() == individual {
                        let DataPropertyExpression::DataProperty(prop) = &assertion.property;
                        if prop.iri.as_str() == property {
                            values.push(assertion.value.to_string());
                        }
                    }
                }
            }
        }

        values.sort();
        Ok(values)
    }

    /// Check if an individual satisfies a datatype restriction based on its data property values
    /// This is a helper for proper datatype reasoning
    fn check_datatype_restriction_satisfaction(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
        data_range: &crate::ontology::DataRange,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Get all data property values for this individual and property
        let individual_iri = individual
            .iri()
            .map_or_else(|| "anonymous".to_string(), |iri| iri.to_string());

        let mut has_matching_value = false;

        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom {
                if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                    if subj.iri.as_str() == individual_iri {
                        // Check if this is the right property
                        if self.data_properties_match(property, &assertion.property) {
                            // Check if the value satisfies the data range
                            if self.literal_satisfies_data_range(&assertion.value, data_range)? {
                                has_matching_value = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(has_matching_value)
    }

    /// Check if two data properties match
    fn data_properties_match(
        &self,
        prop1: &DataPropertyExpression,
        prop2: &DataPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                DataPropertyExpression::DataProperty(p1),
                DataPropertyExpression::DataProperty(p2),
            ) => p1.iri == p2.iri,
        }
    }

    /// Check if a literal value satisfies a data range (with facet restrictions)
    fn literal_satisfies_data_range(
        &self,
        literal: &crate::ontology::Literal,
        data_range: &crate::ontology::DataRange,
    ) -> Result<bool> {
        match data_range {
            crate::ontology::DataRange::Datatype(datatype_iri) => {
                // Validate that literal's datatype matches the specified datatype
                let literal_datatype_url = literal.datatype.as_ref();

                // First check: validate the literal conforms to its declared type
                if !self.datatype_validator.validate_literal(literal)? {
                    return Ok(false);
                }

                // Second check: verify datatype compatibility if literal has a datatype
                if let Some(lit_dt_url) = literal_datatype_url {
                    let lit_dt_iri = IRI::from(lit_dt_url.clone());
                    Ok(self
                        .datatype_validator
                        .datatypes_compatible(&lit_dt_iri, datatype_iri))
                } else {
                    // No datatype means xsd:string, check if compatible with string
                    let xsd_string = IRI::new("http://www.w3.org/2001/XMLSchema#string");
                    Ok(self
                        .datatype_validator
                        .datatypes_compatible(&xsd_string, datatype_iri))
                }
            }
            crate::ontology::DataRange::DatatypeRestriction {
                datatype: _,
                restrictions,
            } => {
                // Check facet restrictions
                for restriction in restrictions {
                    if !self.check_facet_restriction(literal, restriction)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::ontology::DataRange::DataIntersectionOf(ranges) => {
                // Must satisfy all ranges
                for range in ranges {
                    if !self.literal_satisfies_data_range(literal, range)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::ontology::DataRange::DataUnionOf(ranges) => {
                // Must satisfy at least one range
                for range in ranges {
                    if self.literal_satisfies_data_range(literal, range)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            crate::ontology::DataRange::DataComplementOf(range) => {
                // Must NOT satisfy the range
                Ok(!self.literal_satisfies_data_range(literal, range)?)
            }
            crate::ontology::DataRange::DataOneOf(literals) => {
                // Must be one of the specified literals
                // literals here are crate::ontology::Literal
                Ok(literals
                    .iter()
                    .any(|owl_lit| owl_lit.value == literal.value))
            }
        }
    }

    /// Check if a literal satisfies a single facet restriction
    fn check_facet_restriction(
        &self,
        literal: &crate::ontology::Literal,
        restriction: &crate::ontology::FacetRestriction,
    ) -> Result<bool> {
        let facet_iri = restriction.facet.to_string();

        match facet_iri.as_str() {
            "http://www.w3.org/2001/XMLSchema#minInclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a >= b)
            }
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a <= b)
            }
            "http://www.w3.org/2001/XMLSchema#minExclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a > b)
            }
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a < b)
            }
            _ => {
                // For unknown facet types, conservatively return true
                Ok(true)
            }
        }
    }

    /// Compare two numeric values with a comparison function
    fn compare_numeric_values<F>(&self, value1: &str, value2: &str, comparator: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        // Try to parse both values as f64
        match (value1.parse::<f64>(), value2.parse::<f64>()) {
            (Ok(v1), Ok(v2)) => Ok(comparator(v1, v2)),
            _ => {
                // If parsing fails, we can't perform the comparison
                // Conservatively return false for safety
                Ok(false)
            }
        }
    }

    /// Enhanced instance checking that also checks datatype restrictions
    /// This checks if an individual satisfies a class expression based on data property values
    pub fn check_instance_with_datatype_reasoning(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Handle specific class expression types that we can check directly
        match class_expr {
            ClassExpression::Class(cls) => {
                // Strategy 1: Check if individual is directly asserted to be in this class
                if self.check_explicit_class_assertion(individual, cls, ontology)? {
                    return Ok(true);
                }

                // Strategy 2: Check if individual is asserted to be in a subclass of this class
                // Get all asserted types of the individual
                let individual_iri = individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), |iri| iri.to_string());
                let mut asserted_classes = Vec::new();

                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                        if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                            if subj.iri.as_str() == individual_iri {
                                if let ClassExpression::Class(asserted_cls) = &assertion.class {
                                    asserted_classes.push(asserted_cls.iri.clone());
                                }
                            }
                        }
                    }
                }

                // Check if any of the asserted classes is a subclass of the target class
                for asserted_class_iri in &asserted_classes {
                    if self.is_subclass_of_iri(asserted_class_iri, &cls.iri, ontology)? {
                        return Ok(true);
                    }
                }

                // Strategy 3: Check if there's an EquivalentClasses axiom that defines this class
                // with a complex expression we can evaluate
                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                        // Check if our class is in the equivalent classes
                        let our_class_in_equiv = equiv_axiom.classes.iter().any(|c| {
                            if let ClassExpression::Class(eq_cls) = c {
                                eq_cls.iri == cls.iri
                            } else {
                                false
                            }
                        });

                        if our_class_in_equiv {
                            // Check all the other equivalent class expressions
                            for equiv_class in &equiv_axiom.classes {
                                if let ClassExpression::Class(eq_cls) = equiv_class {
                                    // Skip the class itself
                                    if eq_cls.iri == cls.iri {
                                        continue;
                                    }
                                    // Skip owl:Class which appears due to parsing artifacts
                                    if eq_cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Class"
                                    {
                                        continue;
                                    }
                                }

                                // Try to match against this equivalent expression
                                // Only handle complex expressions, not other named classes
                                match equiv_class {
                                    ClassExpression::ObjectIntersectionOf(_)
                                    | ClassExpression::DataSomeValuesFrom { .. }
                                    | ClassExpression::DataHasValue { .. } => {
                                        if self.check_complex_expression(
                                            individual,
                                            equiv_class,
                                            ontology,
                                        )? {
                                            return Ok(true);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                // Strategy 4: Check if the individual satisfies a class that is a subclass of target
                // For example, if individual is ThirstyBasil and we're checking ThirstyPlant,
                // and ThirstyBasil subClassOf ThirstyPlant, then it should return true
                // We need to check all classes that are subclasses of the target
                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                        if let ClassExpression::Class(superclass) = &subclass_axiom.superclass {
                            if superclass.iri == cls.iri {
                                // Found a subclass of our target class
                                // Check if individual is an instance of this subclass
                                if self.check_instance_with_datatype_reasoning(
                                    individual,
                                    &subclass_axiom.subclass,
                                    ontology,
                                )? {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }

                // Couldn't determine membership
                Ok(false)
            }
            // For complex expressions, delegate to specialized handler
            _ => self.check_complex_expression(individual, class_expr, ontology),
        }
    }

    /// Check complex class expressions (non-named classes) directly
    fn check_complex_expression(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        match class_expr {
            ClassExpression::ObjectIntersectionOf(operands) => {
                // Check all operands - must all be true
                for operand in operands {
                    let result = match operand {
                        // Handle named classes in intersections
                        ClassExpression::Class(cls) => {
                            // Check if individual is explicitly asserted to be in this class
                            self.check_explicit_class_assertion(individual, cls, ontology)?
                        }
                        // Recursively check complex expressions
                        _ => self.check_complex_expression(individual, operand, ontology)?,
                    };

                    if !result {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // Check if individual has a data property value that satisfies the restriction
                self.check_datatype_restriction_satisfaction(individual, property, filler, ontology)
            }
            ClassExpression::DataHasValue { property, value } => {
                // Check if individual has this specific data property value
                let individual_iri = individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), |iri| iri.to_string());

                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom
                    {
                        if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                            if subj.iri.as_str() == individual_iri {
                                if self.data_properties_match(property, &assertion.property) {
                                    if assertion.value.value == value.value {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(false)
            }
            _ => {
                // For other class expressions (including named classes reached here),
                // we can't do direct checking
                Ok(false)
            }
        }
    }

    /// Check if an individual is explicitly asserted to be in a named class
    fn check_explicit_class_assertion(
        &self,
        individual: &Individual,
        class: &crate::ontology::Class,
        ontology: &Ontology,
    ) -> Result<bool> {
        let individual_iri = individual
            .iri()
            .map_or_else(|| "anonymous".to_string(), |iri| iri.to_string());

        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                    if subj.iri.as_str() == individual_iri {
                        if let ClassExpression::Class(asserted_class) = &assertion.class {
                            if asserted_class.iri == class.iri {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Check if one class (by IRI) is a subclass of another class (by IRI)
    /// This includes transitive subclass relationships
    fn is_subclass_of_iri(
        &self,
        subclass_iri: &IRI,
        superclass_iri: &IRI,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Direct equality
        if subclass_iri == superclass_iri {
            return Ok(true);
        }

        // Check for direct SubClassOf axiom
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                if let ClassExpression::Class(sub) = &subclass_axiom.subclass {
                    if let ClassExpression::Class(sup) = &subclass_axiom.superclass {
                        if sub.iri == *subclass_iri && sup.iri == *superclass_iri {
                            return Ok(true);
                        }

                        // Transitive check: if subclass_iri -> intermediate -> superclass_iri
                        if sub.iri == *subclass_iri {
                            // Check if this intermediate class is a subclass of the target
                            if self.is_subclass_of_iri(&sup.iri, superclass_iri, ontology)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }
}
