//! Basic reasoning tasks
//!
//! This module implements fundamental reasoning operations like consistency checking,
//! satisfiability testing, and subsumption checking.

use crate::{
    Error, Result,
    cache::CacheManager,
    core::reasoner::{
        statistics::ReasoningStatistics,
        tableau::{ReasoningTask, TableauAlgorithmInstance, TableauFactory},
    },
    ontology::{ClassExpression, Individual, Ontology, OntologyRef},
};
use log::{debug, info};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

/// Service for basic reasoning operations (consistency, satisfiability, subsumption)
#[derive(Debug)]
pub struct ReasoningTaskService {
    pub tableau_factory: TableauFactory,
    pub cache_manager: Arc<RwLock<CacheManager>>,
}

impl ReasoningTaskService {
    /// Create a new reasoning task service
    pub fn new(tableau_factory: TableauFactory, cache_manager: Arc<RwLock<CacheManager>>) -> Self {
        Self {
            tableau_factory,
            cache_manager,
        }
    }

    /// Check if the ontology is consistent
    pub fn check_consistency(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_consistency_checks();

        info!("Checking ontology consistency");

        // Check cache first
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_consistency_result(ontology)
        {
            debug!("Consistency result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = ontology.read().unwrap();

        // Build tableau for consistency checking
        let tableau = self
            .tableau_factory
            .create_algorithm_instance(&ontology_guard)?;

        // Run tableau algorithm
        let result = self.run_tableau_consistency_check(tableau, statistics)?;

        // Cache the result
        self.cache_manager
            .write()
            .unwrap()
            .cache_consistency_result(ontology, result);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Consistency check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if a class is satisfiable
    pub fn check_satisfiability(
        &self,
        class_iri: &str,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_satisfiability_checks();

        info!("Checking satisfiability of class: {class_iri}");

        // Handle special OWL classes
        if class_iri.contains("owl#Thing") {
            return Ok(true); // owl:Thing is always satisfiable
        }
        if class_iri.contains("owl#Nothing") {
            return Ok(false); // owl:Nothing is always unsatisfiable
        }

        // Check cache first
        if let Some(class_expr) = self.parse_class_expression(class_iri) {
            if let Some(cached_result) = self
                .cache_manager
                .read()
                .unwrap()
                .get_satisfiability_result(&class_expr)
            {
                debug!("Satisfiability result found in cache for: {class_iri}");
                return Ok(cached_result);
            }
        }

        let ontology_guard = ontology.read().unwrap();

        // Build tableau for satisfiability checking
        let tableau = self
            .tableau_factory
            .create_algorithm_for_satisfiability(&ontology_guard, class_iri)?;

        // Run tableau algorithm
        let result = self.run_tableau_satisfiability_check(tableau, statistics)?;

        // Cache the result
        if let Some(class_expr) = self.parse_class_expression(class_iri) {
            self.cache_manager
                .write()
                .unwrap()
                .cache_satisfiability_result(class_expr, result);
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Satisfiability check for {class_iri} completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if one class subsumes another
    pub fn check_subsumption(
        &self,
        subclass: &str,
        superclass: &str,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_subsumption_checks();

        info!("Checking subsumption: {subclass} ⊑ {superclass}");

        // Check cache first
        if let (Some(sub_expr), Some(sup_expr)) = (
            self.parse_class_expression(subclass),
            self.parse_class_expression(superclass),
        ) {
            if let Some(cached_result) = self
                .cache_manager
                .read()
                .unwrap()
                .get_subsumption_result(&sub_expr, &sup_expr)
            {
                debug!("Subsumption result found in cache");
                return Ok(cached_result);
            }
        }

        let ontology_guard = ontology.read().unwrap();

        // Build tableau for subsumption checking
        let tableau = self.tableau_factory.create_algorithm_for_subsumption(
            &ontology_guard,
            subclass,
            superclass,
        )?;

        // Run tableau algorithm
        let result = self.run_tableau_subsumption_check(tableau, statistics)?;

        // Cache the result
        if let (Some(sub_expr), Some(sup_expr)) = (
            self.parse_class_expression(subclass),
            self.parse_class_expression(superclass),
        ) {
            self.cache_manager
                .write()
                .unwrap()
                .cache_subsumption_result(sub_expr, sup_expr, result);
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Subsumption check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if an individual is an instance of a class
    pub fn check_instance(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();

        info!("Checking instance relationship");

        // Check cache first
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_instance_result(individual, class_expr)
        {
            debug!("Instance result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = ontology.read().unwrap();

        // Build tableau for instance checking
        let tableau = self.tableau_factory.create_for_instance_check(
            &ontology_guard,
            individual,
            class_expr,
        )?;

        // Run tableau algorithm
        let result = self.run_tableau_instance_check(tableau, statistics)?;

        // Cache the result
        self.cache_manager.write().unwrap().store_instance_result(
            individual.clone(),
            class_expr.clone(),
            result,
        );

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Instance check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check subsumption between two class expressions
    pub fn check_subsumption_expressions(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_subsumption_checks();

        // Check cache first
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_subsumption_result(subclass, superclass)
        {
            return Ok(cached_result);
        }

        let ontology_guard = ontology.read().unwrap();

        // For now, convert to strings and use existing tableau methods
        let subclass_str = format!("{subclass:?}");
        let superclass_str = format!("{superclass:?}");

        let tableau = self.tableau_factory.create_algorithm_for_subsumption(
            &ontology_guard,
            &subclass_str,
            &superclass_str,
        )?;

        let result = self.run_tableau_subsumption_check(tableau, statistics)?;

        // Store in cache
        self.cache_manager
            .write()
            .unwrap()
            .cache_subsumption_result(subclass.clone(), superclass.clone(), result);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        Ok(result)
    }

    /// Check if an axiom is entailed by the ontology
    pub fn check_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        let start_time = Instant::now();

        info!("Checking axiom entailment");

        let ontology_guard = ontology.read().unwrap();

        let result = match axiom {
            crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) => {
                // Check if subclass ⊑ superclass is entailed
                let subclass_str = format!("{:?}", subclass_axiom.subclass);
                let superclass_str = format!("{:?}", subclass_axiom.superclass);
                self.check_subsumption(&subclass_str, &superclass_str, ontology, statistics)?
            }
            crate::ontology::axioms::Axiom::ClassAssertion(class_assertion) => {
                // Check if individual ∈ class is entailed
                self.check_instance(
                    &class_assertion.individual,
                    &class_assertion.class,
                    ontology,
                    statistics,
                )?
            }
            _ => {
                // For other axiom types, check if they are explicitly present
                ontology_guard.axioms.contains(axiom)
            }
        };

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Entailment check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    // Private helper methods

    /// Run a tableau consistency check
    fn run_tableau_consistency_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau consistency check");

        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(true),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(false),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau satisfiability check
    fn run_tableau_satisfiability_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau satisfiability check");

        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(true),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(false),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau subsumption check
    fn run_tableau_subsumption_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau subsumption check");

        // For subsumption A ⊑ B, we check if A ⊓ ¬B is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(false), // A ⊓ ¬B is satisfiable, so A ⊄ B
            crate::core::tableau::TableauState::Unsatisfiable => Ok(true), // A ⊓ ¬B is unsatisfiable, so A ⊑ B
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau instance check
    fn run_tableau_instance_check(
        &self,
        mut tableau: Box<dyn crate::core::reasoner::tableau::TableauRunner>,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau instance check");

        // For instance checking a ∈ C, we check if {a} ⊓ ¬C is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(false),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(true),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Parse a class IRI string into a `ClassExpression`
    fn parse_class_expression(&self, class_iri: &str) -> Option<ClassExpression> {
        // For now, assume it's a named class
        // In a full implementation, this would parse complex class expressions
        Some(ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::from(class_iri.to_string()),
        }))
    }
}
