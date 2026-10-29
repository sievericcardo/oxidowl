//! Tableau algorithm integration and factories
//!
//! This module provides wrappers and factories for the traditional tableau algorithm.

use crate::{
    Result,
    config::{ReasonerConfig, TableauAlgorithm},
    core::tableau::{ConceptLabel, Tableau, TableauBuilder, TableauState},
    ontology::{ClassExpression, Individual, Literal, Ontology},
};
use std::collections::HashMap;

/// A read-only snapshot of a completed tableau, used by the semantic entailment
/// checks (workstream A7) to answer property-axiom and assertion entailments
/// from the materialised closure: edges, nominal merging (`SameIndividual` /
/// functional / `HasKey`), and data values.
///
/// This is the reasoner-API surface accessor for the completed tableau. It is
/// derived entirely from the *public* node/edge accessors of the tableau module
/// (the merge map is reconstructed from the `Nominal` node labels, which the
/// executor keeps consistent after every merge), so it never reaches into the
/// tableau's private state.
#[derive(Debug, Clone)]
pub struct TableauSnapshot {
    /// Whether the completed tableau run ended in a satisfiable (consistent)
    /// state. An unsatisfiable tableau means the ontology is inconsistent and
    /// every axiom is vacuously entailed.
    pub satisfiable: bool,
    /// Object-property edges `(from, to, role_name)` where `role_name` is the
    /// canonical Display form (`IRI` or `inv(IRI)`).
    pub edges: Vec<(usize, usize, String)>,
    /// Individual Display name → canonical node id after `SameIndividual` /
    /// functional / `HasKey` merging. Reconstructed from the `Nominal` labels of
    /// every node, which the executor keeps consistent across merges.
    pub individual_nodes: HashMap<String, usize>,
    /// Data-property edges `(from_node, role_name, literal value)` — an edge
    /// from an individual node to a data-successor node carrying a `DataValue`
    /// label.
    pub data_edges: Vec<(usize, String, Literal)>,
}

impl TableauSnapshot {
    /// Whether the completed tableau contains an edge `from --role--> to`.
    #[must_use]
    pub fn has_edge(&self, from: usize, to: usize, role: &str) -> bool {
        self.edges
            .iter()
            .any(|(f, t, r)| *f == from && *t == to && r == role)
    }

    /// Whether the completed tableau has a data edge `from --role--> v` with a
    /// value equal (in the datatype value space) to `value`.
    #[must_use]
    pub fn has_data_edge_value(&self, from: usize, role: &str, value: &Literal) -> bool {
        use crate::core::reasoner::datatype_validation::DatatypeValidator;
        self.data_edges.iter().any(|(f, r, v)| {
            *f == from && r == role && DatatypeValidator::literal_values_equal(v, value)
        })
    }

    /// The canonical node id of an individual in the completed tableau, if it
    /// appears in the ontology's `ABox` (nominal merging applied).
    #[must_use]
    pub fn node_of(&self, individual: &Individual) -> Option<usize> {
        self.individual_nodes.get(&individual.to_string()).copied()
    }
}

/// Wrapper for tableau algorithm implementation
pub struct TableauAlgorithmInstance {
    tableau: Tableau,
}

impl TableauAlgorithmInstance {
    /// Create a new instance with a tableau
    #[must_use]
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

    /// Whether the completed tableau run ended in a satisfiable state.
    #[must_use]
    pub fn is_satisfiable(&self) -> bool {
        self.tableau.is_satisfiable()
    }

    /// Take a read-only snapshot of the completed tableau (workstream A7).
    ///
    /// The individual→node map is reconstructed from the `Nominal` labels so no
    /// access to the tableau's private `individual_map` is required; the
    /// executor guarantees that after any `merge_nodes` the surviving node
    /// carries the nominal labels of both merged nodes.
    #[must_use]
    pub fn snapshot(&self) -> TableauSnapshot {
        let mut individual_nodes: HashMap<String, usize> = HashMap::new();
        for (i, node) in self.tableau.nodes().iter().enumerate() {
            for concept in &node.concepts {
                if let ConceptLabel::Nominal(name) = concept {
                    individual_nodes.insert(name.clone(), i);
                }
            }
        }

        let edges = self
            .tableau
            .edges()
            .iter()
            .map(|e| (e.from, e.to, e.role.to_string()))
            .collect();

        let mut data_edges: Vec<(usize, String, Literal)> = Vec::new();
        for edge in self.tableau.edges() {
            if let Some(target) = self.tableau.nodes().get(edge.to) {
                for concept in &target.concepts {
                    if let ConceptLabel::DataValue(lit) = concept {
                        data_edges.push((edge.from, edge.role.to_string(), lit.clone()));
                    }
                }
            }
        }

        TableauSnapshot {
            satisfiable: self.tableau.is_satisfiable(),
            edges,
            individual_nodes,
            data_edges,
        }
    }

    /// Whether the node carries a concept label equal to the given class
    /// expression. The tableau materialises class labels as
    /// `ConceptLabel::Complex(expr)` (from `ClassAssertion`, Domain/Range
    /// propagation, concept unfolding) or `ConceptLabel::Atomic(name)` for
    /// atomic classes.
    #[must_use]
    pub fn node_labeled_as(&self, node: usize, class: &ClassExpression) -> bool {
        match self.tableau.nodes().get(node) {
            None => false,
            Some(n) => n.concepts.iter().any(|c| match c {
                ConceptLabel::Complex(expr) => expr.as_ref() == class,
                ConceptLabel::Atomic(name) => {
                    matches!(class, ClassExpression::Class(cls) if cls.iri.as_str() == name.as_str())
                }
                _ => false,
            }),
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
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Hypertableau => {
                log::info!("Creating hypertableau runner for consistency checking");
                let runner =
                    super::hypertableau_adapter::HypertableauRunner::for_consistency(ontology)?;
                Ok(Box::new(runner))
            }
            TableauAlgorithm::Traditional | TableauAlgorithm::ProfileOptimised => {
                log::info!("Creating traditional tableau runner for consistency checking");
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
        superclass: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Hypertableau => {
                log::info!("Creating hypertableau runner for subsumption checking");
                let runner = super::hypertableau_adapter::HypertableauRunner::for_subsumption(
                    ontology, subclass, superclass,
                )?;
                Ok(Box::new(runner))
            }
            TableauAlgorithm::Traditional | TableauAlgorithm::ProfileOptimised => {
                log::info!("Creating traditional tableau runner for subsumption checking");
                // Convert ClassExpression to string for the current tableau builder interface
                let subclass_str = &format!("{subclass}");
                let superclass_str = &format!("{superclass}");
                let tableau = self.tableau_builder.build_for_subsumption(
                    ontology,
                    subclass_str,
                    superclass_str,
                )?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }

    /// Create a tableau runner for satisfiability checking
    pub fn create_for_satisfiability(
        &self,
        ontology: &Ontology,
        class_expr: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Hypertableau => {
                log::info!("Creating hypertableau runner for satisfiability checking");
                let runner = super::hypertableau_adapter::HypertableauRunner::for_satisfiability(
                    ontology, class_expr,
                )?;
                Ok(Box::new(runner))
            }
            TableauAlgorithm::Traditional | TableauAlgorithm::ProfileOptimised => {
                log::info!("Creating traditional tableau runner for satisfiability checking");
                // Convert ClassExpression to string for the current tableau builder interface
                let class_str = &format!("{class_expr}");
                let tableau = self
                    .tableau_builder
                    .build_for_satisfiability(ontology, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
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
    #[must_use]
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
    Realisation,
    InstanceCheck {
        individual: Individual,
        class: ClassExpression,
    },
}
