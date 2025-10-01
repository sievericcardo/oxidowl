//! Tableau algorithm integration and factories
//!
//! This module provides wrappers and factories for the traditional tableau algorithm.

use crate::{
    Error, Result,
    config::{ReasonerConfig, TableauAlgorithm},
    core::{
        dependency::DependencySet,
        tableau::{RoleLabel, Tableau, TableauBuilder, TableauEdge, TableauState},
    },
    ontology::{Axiom, ClassExpression, Individual, Ontology, OntologyRef},
};
use log::warn;
use std::sync::Arc;

/// Wrapper for tableau algorithm implementation
pub struct TableauAlgorithmInstance {
    tableau: Tableau,
}

impl TableauAlgorithmInstance {
    /// Create a new instance with a tableau
    pub fn new(tableau: Tableau) -> Self {
        Self { tableau }
    }

    /// Run the tableau algorithm
    pub fn run(&mut self) -> Result<TableauState> {
        self.tableau.run()
    }

    /// Get node count for statistics
    #[must_use]
    pub fn get_node_count(&self) -> usize {
        self.tableau.get_node_count()
    }

    /// Get backtrack count for statistics
    #[must_use]
    pub fn get_backtrack_count(&self) -> usize {
        self.tableau.get_backtrack_count()
    }

    /// Get maximum depth for statistics
    #[must_use]
    pub fn get_max_depth(&self) -> usize {
        self.tableau.get_max_depth()
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

/// Factory for creating and configuring tableau algorithms
#[derive(Debug)]
pub struct TableauFactory {
    /// Builder for creating tableau instances
    pub tableau_builder: TableauBuilder,
    /// Configuration for the reasoner
    pub config: ReasonerConfig,
}

impl TableauFactory {
    pub fn new(config: ReasonerConfig) -> Result<Self> {
        Ok(Self {
            tableau_builder: TableauBuilder::new(config.reasoning.clone()),
            config,
        })
    }

    /// Create a tableau runner for consistency checking
    pub fn create_for_consistency(&self, ontology: &Ontology) -> Result<Box<dyn TableauRunner>> {
        let tableau = self.tableau_builder.build_for_consistency(ontology)?;
        Ok(Box::new(TraditionalTableauRunner::new(tableau)))
    }

    /// Create a tableau runner for subsumption checking
    pub fn create_for_subsumption(
        &self,
        ontology: &Ontology,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        // Convert ClassExpression to string for the current tableau builder interface
        let subclass_str = &format!("{subclass}");
        let superclass_str = &format!("{superclass}");
        let tableau =
            self.tableau_builder
                .build_for_subsumption(ontology, subclass_str, superclass_str)?;
        Ok(Box::new(TraditionalTableauRunner::new(tableau)))
    }

    /// Create a tableau runner for satisfiability checking
    pub fn create_for_satisfiability(
        &self,
        ontology: &Ontology,
        class_expr: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        // Convert ClassExpression to string for the current tableau builder interface
        let class_str = &format!("{class_expr}");
        let tableau = self
            .tableau_builder
            .build_for_satisfiability(ontology, class_str)?;
        Ok(Box::new(TraditionalTableauRunner::new(tableau)))
    }

    /// Create a tableau runner for instance checking
    pub fn create_for_instance_check(
        &self,
        ontology: &Ontology,
        individual: &Individual,
        class_expr: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        // Convert Individual and ClassExpression to string for the current tableau builder interface
        let individual_str = &individual
            .iri()
            .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string);
        let class_str = &format!("{class_expr}");
        let tableau =
            self.tableau_builder
                .build_for_instance_check(ontology, individual_str, class_str)?;
        Ok(Box::new(TraditionalTableauRunner::new(tableau)))
    }

    /// Create a tableau algorithm instance based on configuration
    pub fn create_algorithm_instance(
        &self,
        ontology: &Ontology,
    ) -> Result<TableauAlgorithmInstance> {
        let tableau = self.tableau_builder.build_for_consistency(ontology)?;
        Ok(TableauAlgorithmInstance::new(tableau))
    }

    /// Create a tableau algorithm instance for subsumption checking
    pub fn create_algorithm_for_subsumption(
        &self,
        ontology: &Ontology,
        subclass: &str,
        superclass: &str,
    ) -> Result<TableauAlgorithmInstance> {
        let tableau = self
            .tableau_builder
            .build_for_subsumption(ontology, subclass, superclass)?;
        Ok(TableauAlgorithmInstance::new(tableau))
    }

    /// Create a tableau algorithm instance for satisfiability checking
    pub fn create_algorithm_for_satisfiability(
        &self,
        ontology: &Ontology,
        class_iri: &str,
    ) -> Result<TableauAlgorithmInstance> {
        let tableau = self
            .tableau_builder
            .build_for_satisfiability(ontology, class_iri)?;
        Ok(TableauAlgorithmInstance::new(tableau))
    }

    /// Create a tableau algorithm instance for instance checking
    pub fn create_algorithm_for_instance_check(
        &self,
        ontology: &Ontology,
        individual: &str,
        class: &str,
    ) -> Result<TableauAlgorithmInstance> {
        let tableau = self
            .tableau_builder
            .build_for_instance_check(ontology, individual, class)?;
        Ok(TableauAlgorithmInstance::new(tableau))
    }

    /// Get access to the underlying tableau builder
    pub fn tableau_builder(&self) -> &TableauBuilder {
        &self.tableau_builder
    }
}

/// Traditional tableau runner wrapper
pub struct TraditionalTableauRunner {
    tableau: Tableau,
}

impl TraditionalTableauRunner {
    #[must_use]
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
        matches!(
            self.tableau.get_state(),
            TableauState::Satisfiable | TableauState::Unsatisfiable
        )
    }
}

/// Reasoning task types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReasoningTask {
    ConsistencyCheck,
    Satisfiability(ClassExpression),
    Subsumption {
        subclass: ClassExpression,
        superclass: ClassExpression,
    },
    Classification,
    Realization,
    InstanceCheck {
        individual: Individual,
        class: ClassExpression,
    },
}
