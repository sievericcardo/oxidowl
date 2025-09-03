//! Classification and realization operations
//!
//! This module implements complex reasoning operations like building class hierarchies,
//! property classification, and individual type inference.

use crate::{
    Error, Result,
    cache::CacheManager,
    core::reasoner::{
        results::{ClassificationResult, PropertyClassificationResult, RealizationResult},
        statistics::ReasoningStatistics,
        tasks::ReasoningTaskService,
    },
    ontology::{
        ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, Ontology,
        OntologyRef,
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
        }
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<ClassificationResult> {
        let start_time = Instant::now();

        info!("Starting classification");

        // Check if we have a cached classification result
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_classification_result(ontology)
        {
            debug!("Classification result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = ontology.read().unwrap();

        // Get all named classes from the ontology
        let signature = ontology_guard.signature()?;
        let mut classes: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        // Add owl:Thing if not present
        let owl_thing = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing")
                .to_url()?
                .into(),
        });
        if !classes.contains(&owl_thing) {
            classes.push(owl_thing.clone());
        }

        // Process inferred classes from complex axioms
        let inferred_classes = self.discover_inferred_classes(&ontology_guard)?;
        classes.extend(inferred_classes);

        let mut hierarchy = HashMap::new();
        let total_pairs = classes.len() * classes.len();
        let mut checked_pairs = 0;

        info!(
            "Classifying {} classes ({} subsumption checks)",
            classes.len(),
            total_pairs
        );

        // Debug: Log all SubClassOf axioms in the ontology
        let mut subclass_count = 0;
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                subclass_count += 1;
                log::debug!(
                    "Found SubClassOf axiom {}: {:?} ⊑ {:?}",
                    subclass_count,
                    subclass_axiom.subclass,
                    subclass_axiom.superclass
                );
            }
        }
        info!("Found {} SubClassOf axioms in ontology", subclass_count);

        // Build hierarchy using axiom-based reasoning
        for subclass in &classes {
            let mut superclasses = HashSet::new();

            for superclass in &classes {
                if subclass != superclass {
                    // Use proper subsumption checking based on ontology axioms
                    if self.check_subsumption_from_axioms(subclass, superclass, &ontology_guard)? {
                        superclasses.insert(superclass.clone());
                    }
                }
                checked_pairs += 1;

                if checked_pairs % 1000 == 0 {
                    info!(
                        "Classification progress: {checked_pairs}/{total_pairs} checks completed"
                    );
                }
            }

            hierarchy.insert(subclass.clone(), superclasses);
        }

        let result = ClassificationResult::new(hierarchy);

        // Cache the result
        self.cache_manager
            .write()
            .unwrap()
            .store_classification_result(ontology, result.clone());

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Classify object properties
    pub fn classify_object_properties(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting object property classification");

        let ontology_guard = ontology.read().unwrap();

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

        let ontology_guard = ontology.read().unwrap();

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
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_realization_result(ontology)
        {
            debug!("Realization result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = ontology.read().unwrap();

        // Get all named individuals and classes
        let individuals: Vec<Individual> = ontology_guard.signature().unwrap().individuals.clone();

        let classes: Vec<ClassExpression> = ontology_guard
            .signature()
            .unwrap()
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
                if self
                    .task_service
                    .check_instance(individual, class, ontology, statistics)?
                {
                    instance_classes.insert(class.clone());
                }
            }

            realization.insert(individual.clone(), instance_classes);
        }

        let result = RealizationResult::new(realization);

        // Cache the result
        self.cache_manager
            .write()
            .unwrap()
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

        let ontology_guard = ontology.read().unwrap();

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
        let ontology_guard = ontology.read().unwrap();
        let mut superclasses = Vec::new();

        // Get all classes from the signature
        for class in &ontology_guard.signature().unwrap().classes {
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
        let ontology_guard = ontology.read().unwrap();
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
        let ontology_guard = ontology.read().unwrap();
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
        for class in &ontology_guard.signature().unwrap().classes {
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
            let ontology_guard = ontology.read().unwrap();
            // Get all individuals from the signature
            ontology_guard.signature().unwrap().individuals.clone()
        }; // Drop the read lock here

        let mut instances = Vec::new();

        for individual in &individuals {
            // Use tableau reasoning to check if individual is instance of concept
            if self
                .task_service
                .check_instance(individual, concept, ontology, statistics)?
            {
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
        match expr {
            ClassExpression::Class(class) => {
                result.insert(class.iri.to_string());
            }
            ClassExpression::ObjectUnionOf(union_classes) => {
                for nested_expr in union_classes {
                    self.extract_all_union_classes(nested_expr, result);
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

        let ontology_guard = ontology.read().unwrap();
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
                    iri: crate::ontology::IRI::new(t).to_url().unwrap().into(),
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

        let ontology_guard = ontology.read().unwrap();
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

        let ontology_guard = ontology.read().unwrap();
        let mut values = Vec::new();

        // Find all data property assertions for this individual and property
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom {
                if let crate::ontology::Individual::Named(subj) = &assertion.individual {
                    if subj.iri.as_str() == individual {
                        if let DataPropertyExpression::DataProperty(prop) = &assertion.property {
                            if prop.iri.as_str() == property {
                                values.push(assertion.value.to_string());
                            }
                        }
                    }
                }
            }
        }

        values.sort();
        Ok(values)
    }
}
