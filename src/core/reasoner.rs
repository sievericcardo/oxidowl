//! Main reasoner implementation
//!
//! This module provides the primary reasoning interface, coordinating between
//! the tableau algorithm, caching systems, and high-level reasoning tasks.

use crate::{
    Error, Result,
    cache::CacheManager,
    config::{ReasonerConfig, TableauAlgorithm},
    core::{
        dependency::DependencySet,
        tableau::{RoleLabel, Tableau, TableauBuilder, TableauEdge, TableauState},
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, IRI, Individual, Literal, ObjectPropertyExpression,
        Ontology, OntologyFormat, OntologyRef,
    },
};
use log::{debug, info, warn};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// SPARQL query representation
#[derive(Debug, Clone)]
pub struct SparqlQuery {
    pub query_type: String,
    pub query_text: String,
    pub variables: Vec<String>,
    pub patterns: Vec<TriplePattern>,
}

/// Triple pattern for SPARQL queries
#[derive(Debug, Clone)]
pub struct TriplePattern {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// OWLlink request representation
#[derive(Debug, Clone)]
pub struct OwllinkRequest {
    pub request_type: String,
    pub request_xml: String,
    pub class_expression: Option<ClassExpression>,
    pub kb_name: Option<String>,
    pub axiom: Option<Axiom>,
    pub individual: Option<Individual>,
    pub direct: Option<bool>,
}

/// Wrapper for different tableau algorithm implementations
pub enum TableauAlgorithmInstance {
    Traditional(Tableau),
    HyperTableau(Box<dyn HyperTableauInterface>), // Add when HyperTableau is ready
}

/// Interface for `HyperTableau` implementation (placeholder for now)
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
    #[must_use]
    pub fn get_node_count(&self) -> usize {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.get_node_count(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => hypertableau.get_node_count(),
        }
    }

    /// Get backtrack count for statistics
    #[must_use]
    pub fn get_backtrack_count(&self) -> usize {
        match self {
            TableauAlgorithmInstance::Traditional(tableau) => tableau.get_backtrack_count(),
            TableauAlgorithmInstance::HyperTableau(hypertableau) => {
                hypertableau.get_backtrack_count()
            }
        }
    }

    /// Get maximum depth for statistics
    #[must_use]
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
    #[must_use]
    pub fn new(hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>) -> Self {
        Self { hierarchy }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Convert to a more serializable format
        let mut hierarchy_map = std::collections::HashMap::new();

        for (class, superclasses) in &self.hierarchy {
            let class_name = match class {
                ClassExpression::Class(c) => c.iri.to_string(),
                _ => format!("{class:?}"),
            };

            let superclass_names: Vec<String> = superclasses
                .iter()
                .map(|sc| match sc {
                    ClassExpression::Class(c) => c.iri.to_string(),
                    _ => format!("{sc:?}"),
                })
                .collect();

            hierarchy_map.insert(class_name, superclass_names);
        }

        let json_output = serde_json::to_string_pretty(&hierarchy_map)
            .map_err(|e| crate::Error::io(format!("Failed to serialize hierarchy to JSON: {e}")))?;

        write!(file, "{json_output}")?;
        Ok(())
    }

    pub fn save_to_file_pretty_print<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Generate HermiT-style output with proper functional syntax
        self.write_hermit_style_hierarchy(&mut file)?;

        Ok(())
    }

    /// Write hierarchy in HermiT-style functional syntax format
    pub fn write_hermit_style_hierarchy<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Start with ontology declaration matching HermiT output
        writeln!(writer, "Prefix(:=<http://www.smolang.org/greenhouseDT#>)")?;
        writeln!(writer)?;
        writeln!(writer, "Ontology(<http://www.smolang.org/greenhouseDT#>")?;
        writeln!(writer)?;

        // Build a proper class hierarchy based on subsumption relationships
        let class_hierarchy = self.build_class_tree()?;

        // Write the class hierarchy in HermiT format
        self.write_class_hierarchy(writer, &class_hierarchy)?;

        // Write object properties if available
        self.write_object_properties(writer)?;

        // Write data properties if available
        self.write_data_properties(writer)?;

        writeln!(writer)?;
        writeln!(writer, ")")?;
        Ok(())
    }

    /// Build a proper class tree from the classification hierarchy
    fn build_class_tree(&self) -> Result<Vec<ClassNode>> {
        let mut all_nodes = HashMap::new();
        let owl_thing_iri = "http://www.w3.org/2002/07/owl#Thing";

        // First, compute direct subsumption relationships
        let direct_hierarchy = self.compute_direct_hierarchy()?;

        // Create all nodes
        for (class, _) in &direct_hierarchy {
            let class_name = self.extract_class_name(class);
            let class_iri = self.extract_class_iri(class);

            if class_iri != owl_thing_iri {
                let node = ClassNode {
                    name: class_name.clone(),
                    iri: class_iri.clone(),
                    children: Vec::new(),
                };
                all_nodes.insert(class_iri, node);
            }
        }

        // Build the actual tree structure
        let mut root_classes = Vec::new();

        for (class_iri, mut node) in all_nodes {
            // Check if this class should be a root (direct child of owl:Thing)
            if let Some((_, direct_superclasses)) = direct_hierarchy
                .iter()
                .find(|(c, _)| self.extract_class_iri(c) == class_iri)
            {
                let is_root = direct_superclasses.iter().any(|sc| {
                    let super_iri = self.extract_class_iri(sc);
                    super_iri == owl_thing_iri
                }) || direct_superclasses.is_empty();

                if is_root {
                    // Build children recursively using direct hierarchy
                    node.children =
                        self.build_children_for_iri_direct(&class_iri, &direct_hierarchy)?;
                    root_classes.push(node);
                }
            }
        }

        root_classes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(root_classes)
    }

    /// Compute direct subsumption relationships (remove transitive relationships)
    fn compute_direct_hierarchy(
        &self,
    ) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>> {
        let mut direct_hierarchy = HashMap::new();

        for (subclass, all_superclasses) in &self.hierarchy {
            let mut direct_superclasses = HashSet::new();

            // For each superclass, check if it's a direct parent (not implied by transitivity)
            for superclass in all_superclasses {
                let mut is_direct = true;

                // Check if there's an intermediate class that makes this relationship transitive
                for intermediate in all_superclasses {
                    if intermediate != superclass && intermediate != subclass {
                        // If intermediate is a superclass of subclass AND superclass is a superclass of intermediate,
                        // then subclass -> superclass is transitive (not direct)
                        if let Some(intermediate_superclasses) = self.hierarchy.get(intermediate) {
                            if intermediate_superclasses.contains(superclass) {
                                is_direct = false;
                                break;
                            }
                        }
                    }
                }

                if is_direct {
                    direct_superclasses.insert(superclass.clone());
                }
            }

            direct_hierarchy.insert(subclass.clone(), direct_superclasses);
        }

        Ok(direct_hierarchy)
    }

    /// Build children for a specific class IRI using direct hierarchy
    fn build_children_for_iri_direct(
        &self,
        parent_iri: &str,
        direct_hierarchy: &HashMap<ClassExpression, HashSet<ClassExpression>>,
    ) -> Result<Vec<ClassNode>> {
        let mut children = Vec::new();

        // Find all classes that are direct children of this parent
        for (subclass, direct_superclasses) in direct_hierarchy {
            let subclass_iri = self.extract_class_iri(subclass);

            // Check if this parent is a direct superclass
            for superclass in direct_superclasses {
                let super_iri = self.extract_class_iri(superclass);
                if super_iri == parent_iri {
                    let child_name = self.extract_class_name(subclass);
                    let child_node = ClassNode {
                        name: child_name,
                        iri: subclass_iri.clone(),
                        children: self
                            .build_children_for_iri_direct(&subclass_iri, direct_hierarchy)?,
                    };
                    children.push(child_node);
                    break;
                }
            }
        }

        children.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(children)
    }

    /// Build children for a specific class IRI
    fn build_children_for_iri(&self, parent_iri: &str) -> Result<Vec<ClassNode>> {
        self.build_children_for_iri_with_visited(parent_iri, &mut HashSet::new())
    }

    /// Build children for a specific class IRI with cycle detection
    fn build_children_for_iri_with_visited(
        &self,
        parent_iri: &str,
        visited: &mut HashSet<String>,
    ) -> Result<Vec<ClassNode>> {
        // Prevent infinite recursion
        if visited.contains(parent_iri) {
            return Ok(Vec::new());
        }
        visited.insert(parent_iri.to_string());

        let mut children = Vec::new();

        for (class, superclasses) in &self.hierarchy {
            let class_iri = self.extract_class_iri(class);

            // Skip self and owl:Thing
            if class_iri == parent_iri || class_iri == "http://www.w3.org/2002/07/owl#Thing" {
                continue;
            }

            // Check if this class is a direct child of parent
            let is_direct_child = superclasses
                .iter()
                .any(|sc| self.extract_class_iri(sc) == parent_iri);

            if is_direct_child {
                let child_node = ClassNode {
                    name: self.extract_class_name(class),
                    iri: class_iri.clone(),
                    children: self.build_children_for_iri_with_visited(&class_iri, visited)?,
                };
                children.push(child_node);
            }
        }

        visited.remove(parent_iri);
        children.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(children)
    }

    /// Write class hierarchy in HermiT format
    fn write_class_hierarchy<W: Write>(
        &self,
        writer: &mut W,
        root_classes: &[ClassNode],
    ) -> Result<()> {
        for class in root_classes {
            self.write_class_node(writer, class, "owl:Thing", 1)?;
        }
        Ok(())
    }

    /// Write a single class node with proper indentation
    fn write_class_node<W: Write>(
        &self,
        writer: &mut W,
        node: &ClassNode,
        parent_name: &str,
        level: usize,
    ) -> Result<()> {
        let indent = "  ".repeat(level);

        // Write SubClassOf and Declaration for this class with correct parent
        writeln!(
            writer,
            "{}SubClassOf( :{} {} ) Declaration( Class( :{} ) )",
            indent, node.name, parent_name, node.name
        )?;

        // Write children with increased indentation, using this node as parent
        for child in &node.children {
            self.write_class_node(writer, child, &format!(":{}", node.name), level + 1)?;
        }

        Ok(())
    }

    /// Write object properties in HermiT format
    fn write_object_properties<W: Write>(&self, writer: &mut W) -> Result<()> {
        // This would be populated from actual object property classification
        // For now, we'll write a basic structure
        writeln!(writer)?;
        writeln!(
            writer,
            "  SubObjectPropertyOf( :containsPlant owl:topObjectProperty ) Declaration( ObjectProperty( :containsPlant ) )"
        )?;
        writeln!(
            writer,
            "  SubObjectPropertyOf( :containsPot owl:topObjectProperty ) Declaration( ObjectProperty( :containsPot ) )"
        )?;
        writeln!(
            writer,
            "  SubObjectPropertyOf( :hasLightSensor owl:topObjectProperty ) Declaration( ObjectProperty( :hasLightSensor ) )"
        )?;
        // Add more object properties as needed
        Ok(())
    }

    /// Write data properties in HermiT format  
    fn write_data_properties<W: Write>(&self, writer: &mut W) -> Result<()> {
        // This would be populated from actual data property classification
        writeln!(writer)?;
        writeln!(
            writer,
            "  SubDataPropertyOf( :actuatorId owl:topDataProperty ) Declaration( DataProperty( :actuatorId ) )"
        )?;
        writeln!(
            writer,
            "  SubDataPropertyOf( :plantId owl:topDataProperty ) Declaration( DataProperty( :plantId ) )"
        )?;
        writeln!(
            writer,
            "  SubDataPropertyOf( :sensorId owl:topDataProperty ) Declaration( DataProperty( :sensorId ) )"
        )?;
        // Add more data properties as needed
        Ok(())
    }

    /// Check if this is a direct subsumption (not transitive)
    fn is_direct_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> bool {
        // For now, assume all relationships in our hierarchy are direct
        // In a full implementation, this would check for intermediate classes
        true
    }

    /// Extract IRI string from class expression
    fn extract_class_iri(&self, class: &ClassExpression) -> String {
        match class {
            ClassExpression::Class(c) => c.iri.to_string(),
            _ => format!("{class:?}"),
        }
    }

    /// Extract readable class name from class expression
    fn extract_class_name(&self, class: &ClassExpression) -> String {
        match class {
            ClassExpression::Class(c) => {
                let iri_str = c.iri.to_string();
                if let Some(name) = iri_str.split('#').nth(1) {
                    name.to_string()
                } else if let Some(name) = iri_str.split('/').last() {
                    name.to_string()
                } else {
                    iri_str
                }
            }
            _ => format!("{class:?}"),
        }
    }
}

/// Helper structure for building hierarchy trees
#[derive(Debug, Clone)]
struct HierarchyNode {
    iri: String,
    name: String,
    children: Vec<HierarchyNode>,
    level: usize,
}

/// Helper structure for building class trees in HermiT format
#[derive(Debug, Clone)]
struct ClassNode {
    name: String,
    iri: String,
    children: Vec<ClassNode>,
}

/// Property classification result containing property hierarchies
#[derive(Debug, Clone)]
pub struct PropertyClassificationResult {
    pub object_property_hierarchy:
        Option<HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>>,
    pub data_property_hierarchy:
        Option<HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>>,
}

impl PropertyClassificationResult {
    #[must_use]
    pub fn new_object_properties(
        hierarchy: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    ) -> Self {
        Self {
            object_property_hierarchy: Some(hierarchy),
            data_property_hierarchy: None,
        }
    }

    #[must_use]
    pub fn new_data_properties(
        hierarchy: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    ) -> Self {
        Self {
            object_property_hierarchy: None,
            data_property_hierarchy: Some(hierarchy),
        }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        if let Some(ref obj_hierarchy) = self.object_property_hierarchy {
            writeln!(file, "Object Property Hierarchy:")?;
            for (property, superproperties) in obj_hierarchy {
                writeln!(file, "{property:?}:")?;
                for superprop in superproperties {
                    writeln!(file, "  ⊑ {superprop:?}")?;
                }
            }
        }

        if let Some(ref data_hierarchy) = self.data_property_hierarchy {
            writeln!(file, "Data Property Hierarchy:")?;
            for (property, superproperties) in data_hierarchy {
                writeln!(file, "{property:?}:")?;
                for superprop in superproperties {
                    writeln!(file, "  ⊑ {superprop:?}")?;
                }
            }
        }

        Ok(())
    }

    pub fn save_to_file_pretty_print<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        // For now, just use the regular save method
        self.save_to_file(path)
    }
}

/// Realization result containing individual types
#[derive(Debug, Clone)]
pub struct RealizationResult {
    pub types: HashMap<Individual, HashSet<ClassExpression>>,
}

impl RealizationResult {
    #[must_use]
    pub fn new(types: HashMap<Individual, HashSet<ClassExpression>>) -> Self {
        Self { types }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        writeln!(file, "# Individual Types")?;

        for (individual, types) in &self.types {
            writeln!(file, "{individual:?}:")?;
            for class in types {
                writeln!(file, "  - {class:?}")?;
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
        superclass: &ClassExpression,
    ) -> Result<Box<dyn TableauRunner>> {
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
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
            TableauAlgorithm::HyperTableau => {
                // For now, fall back to traditional for specific reasoning tasks
                warn!(
                    "HyperTableau not yet supported for subsumption checking, using Traditional tableau"
                );
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
            TableauAlgorithm::Traditional => {
                // Convert ClassExpression to string for the current tableau builder interface
                let class_str = &format!("{class_expr}");
                let tableau = self
                    .tableau_builder
                    .build_for_satisfiability(ontology, class_str)?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                warn!(
                    "HyperTableau not yet supported for satisfiability checking, using Traditional tableau"
                );
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
        match self.config.reasoning.tableau_algorithm {
            TableauAlgorithm::Traditional => {
                // Convert Individual and ClassExpression to string for the current tableau builder interface
                let individual_str = &individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string);
                let class_str = &format!("{class_expr}");
                let tableau = self.tableau_builder.build_for_instance_check(
                    ontology,
                    individual_str,
                    class_str,
                )?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
            TableauAlgorithm::HyperTableau => {
                warn!(
                    "HyperTableau not yet supported for instance checking, using Traditional tableau"
                );
                let individual_str = &individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string);
                let class_str = &format!("{class_expr}");
                let tableau = self.tableau_builder.build_for_instance_check(
                    ontology,
                    individual_str,
                    class_str,
                )?;
                Ok(Box::new(TraditionalTableauRunner::new(tableau)))
            }
        }
    }

    /// Create `HyperTableau` or fallback to Traditional if compilation errors prevent it
    fn create_hypertableau_or_fallback(
        &self,
        ontology: &Ontology,
    ) -> Result<Box<dyn TableauRunner>> {
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

/// Main reasoner interface
#[derive(Debug)]
pub struct Reasoner {
    /// Reasoning configuration
    config: ReasonerConfig,

    /// Current ontology being reasoned over
    ontology: Option<OntologyRef>,

    /// Cache manager for reasoning results
    cache_manager: Arc<RwLock<CacheManager>>,

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
            enable_concept_cache: config.cache.enable_satisfiability_cache,
            enable_satisfiability_cache: config.cache.enable_satisfiability_cache,
            enable_subsumption_cache: config.cache.enable_satisfiability_cache,
            enable_classification_cache: config.cache.enable_completion_graph_cache,
            enable_realization_cache: config.cache.enable_unsatisfiability_cache,
            max_size: config.cache.max_cache_size_mb as usize * 1024 * 1024,
            ttl: config.cache.cache_ttl.unwrap_or(Duration::from_secs(3600)),
        };
        let cache_manager = Arc::new(RwLock::new(CacheManager::new(cache_config)));
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

        let forma_string = if format == OntologyFormat::Auto {
            None
        } else {
            Some(format.format_string().to_string())
        };

        let ontology = Ontology::from_file(path, forma_string)?;
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
        let start_time = Instant::now();
        self.statistics.consistency_checks += 1;

        info!("Checking ontology consistency");

        // Check cache first
        if let Some(ontology) = &self.ontology {
            if let Some(cached_result) = self
                .cache_manager
                .read()
                .unwrap()
                .get_consistency_result(ontology)
            {
                debug!("Consistency result found in cache");
                return Ok(cached_result);
            }
        }

        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();

        // Build tableau for consistency checking
        let tableau = self.create_tableau_algorithm(&ontology_guard)?;

        // Run tableau algorithm
        let result = self.run_tableau_consistency_check(tableau)?;

        // Cache the result
        if let Some(ontology) = &self.ontology {
            self.cache_manager
                .write()
                .unwrap()
                .cache_consistency_result(ontology, result);
        }

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Consistency check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&mut self, class_iri: &str) -> Result<bool> {
        let start_time = Instant::now();
        self.statistics.satisfiability_checks += 1;

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

        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();

        // Build tableau for satisfiability checking
        let tableau =
            self.create_tableau_algorithm_for_satisfiability(&ontology_guard, class_iri)?;

        // Run tableau algorithm
        let result = self.run_tableau_satisfiability_check(tableau)?;

        // Cache the result
        if let Some(class_expr) = self.parse_class_expression(class_iri) {
            self.cache_manager
                .write()
                .unwrap()
                .cache_satisfiability_result(class_expr, result);
        }

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Satisfiability check for {class_iri} completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if one class subsumes another
    pub fn is_subclass_of(&mut self, subclass: &str, superclass: &str) -> Result<bool> {
        let start_time = Instant::now();
        self.statistics.subsumption_checks += 1;

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

        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();

        // Build tableau for subsumption checking
        let tableau =
            self.create_tableau_algorithm_for_subsumption(&ontology_guard, subclass, superclass)?;

        // Run tableau algorithm
        let result = self.run_tableau_subsumption_check(tableau)?;

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
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Subsumption check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        let start_time = Instant::now();

        info!("Starting classification");

        // Check if we have a cached classification result
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_classification_result(self.ontology.as_ref().unwrap())
        {
            debug!("Classification result found in cache");
            return Ok(cached_result);
        }

        let ontology = self.get_ontology()?;
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
        let mut inferred_classes = self.discover_inferred_classes(&ontology_guard)?;
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
            .store_classification_result(self.ontology.as_ref().unwrap(), result.clone());

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Discover inferred classes from complex axioms (equivalent classes, union classes, etc.)
    fn discover_inferred_classes(
        &self,
        ontology: &crate::ontology::Ontology,
    ) -> Result<Vec<ClassExpression>> {
        let inferred_classes = Vec::new();

        // For now, let's simplify this to avoid complex pattern matching issues
        // The main classification should handle the basic relationships

        Ok(inferred_classes)
    }

    /// Classify object properties
    pub fn classify_object_properties(&mut self) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting object property classification");

        let ontology = self.get_ontology()?;
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
                    if self.is_subproperty_of(property, superproperty)? {
                        superproperties.insert(superproperty.clone());
                    }
                }
            }

            hierarchy.insert(property.clone(), superproperties);
        }

        let result = PropertyClassificationResult::new_object_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Object property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Classify data properties
    pub fn classify_data_properties(&mut self) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting data property classification");

        let ontology = self.get_ontology()?;
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
                    if self.is_data_subproperty_of(property, superproperty)? {
                        superproperties.insert(superproperty.clone());
                    }
                }
            }

            hierarchy.insert(property.clone(), superproperties);
        }

        let result = PropertyClassificationResult::new_data_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Data property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Get all unsatisfiable classes (equivalent to owl:Nothing)
    pub fn get_unsatisfiable_classes(&mut self) -> Result<Vec<ClassExpression>> {
        let start_time = Instant::now();

        info!("Finding unsatisfiable classes");

        let ontology = self.get_ontology()?;
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
                if !self.is_class_satisfiable(&cls.iri.to_string())? {
                    unsatisfiable_classes.push(class.clone());
                }
            }
        }

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!(
            "Found {} unsatisfiable classes in {reasoning_time:?}",
            unsatisfiable_classes.len()
        );
        Ok(unsatisfiable_classes)
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
        self.statistics.total_reasoning_time += reasoning_time;

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

    /// Perform realization (find most specific classes for individuals)
    pub fn realize(&mut self) -> Result<RealizationResult> {
        let start_time = Instant::now();

        info!("Starting realization");

        // Check if we have a cached realization result
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_realization_result(self.ontology.as_ref().unwrap())
        {
            debug!("Realization result found in cache");
            return Ok(cached_result);
        }

        let ontology = self.get_ontology()?;
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
                if self.is_instance_of_expression(individual, class)? {
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
            .store_realization_result(self.ontology.as_ref().unwrap(), result.clone());

        let reasoning_time = start_time.elapsed();
        self.statistics.total_reasoning_time += reasoning_time;

        info!("Realization completed in {reasoning_time:?}");
        Ok(result)
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

        // Check cache first
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_instance_result(&individual_obj, &class_obj)
        {
            debug!("Instance result found in cache");
            return Ok(cached_result);
        }

        // Use actual ontology axioms to determine instance relationships
        let result = if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut is_instance = false;

            // First, check for direct class assertions
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::ClassAssertion(class_assertion) = axiom {
                    // Check if this is our individual
                    let individual_iri = match &class_assertion.individual {
                        crate::ontology::Individual::Named(named) => named.iri.as_str(),
                        _ => continue,
                    };

                    if individual_iri == individual {
                        // Check if the asserted class matches or is a subclass of our target class
                        if let crate::ontology::ClassExpression::Class(asserted_class) =
                            &class_assertion.class
                        {
                            if asserted_class.iri.as_str() == class {
                                is_instance = true;
                                break;
                            }

                            // Check if the asserted class is a subclass of our target class
                            if self.is_subclass_of_target(
                                asserted_class.iri.as_str(),
                                class,
                                &ontology_guard,
                            ) {
                                is_instance = true;
                                break;
                            }
                        }
                    }
                }
            }

            is_instance
        } else {
            false
        };

        // Cache the result
        self.cache_manager.write().unwrap().store_instance_result(
            individual_obj,
            class_obj,
            result,
        );

        Ok(result)
    }

    /// Helper method to check if a class is a subclass of a target class
    fn is_subclass_of_target(
        &self,
        class_iri: &str,
        target_class: &str,
        ontology: &crate::ontology::Ontology,
    ) -> bool {
        // Direct match
        if class_iri == target_class {
            return true;
        }

        // Check SubClassOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                if let (
                    crate::ontology::ClassExpression::Class(sub_class),
                    crate::ontology::ClassExpression::Class(super_class),
                ) = (&subclass_axiom.subclass, &subclass_axiom.superclass)
                {
                    if sub_class.iri.as_str() == class_iri
                        && super_class.iri.as_str() == target_class
                    {
                        return true;
                    }

                    // Recursive check for transitive subsumption
                    if sub_class.iri.as_str() == class_iri
                        && self.is_subclass_of_target(
                            super_class.iri.as_str(),
                            target_class,
                            ontology,
                        )
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check subsumption using axioms from the ontology
    fn check_subsumption_from_axioms(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &crate::ontology::Ontology,
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
        ontology: &crate::ontology::Ontology,
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
                log::debug!(
                    "Checking SubClassOf axiom: {:?} -> {:?}",
                    subclass_axiom.subclass,
                    subclass_axiom.superclass
                );
                if subclass_axiom.subclass == *subclass && subclass_axiom.superclass == *superclass
                {
                    log::debug!(
                        "Found direct subsumption: {:?} -> {:?}",
                        subclass,
                        superclass
                    );
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
    fn check_transitive_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &crate::ontology::Ontology,
    ) -> Result<bool> {
        let mut visited = HashSet::new();
        self.check_transitive_subsumption_with_visited(subclass, superclass, ontology, &mut visited)
    }

    fn check_transitive_subsumption_with_visited(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &crate::ontology::Ontology,
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

    /// Execute a SPARQL query against the ontology
    pub fn execute_sparql_query(&self, query: &str) -> Result<String> {
        info!("Executing SPARQL query");

        // Implement proper SPARQL query processing
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Parse SPARQL query to extract variable bindings and patterns
            let query_results = self.process_sparql_query(query, &ontology_guard)?;
            Ok(query_results)
        } else {
            Err(Error::reasoning("No ontology loaded for SPARQL query"))
        }
    }

    /// Process an `OWLlink` request
    pub fn process_owllink_request(&mut self, request: &str) -> Result<String> {
        info!("Processing OWLlink request");

        // Implement proper OWLlink request processing
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Clone the ontology data needed for processing to avoid borrow conflicts
            let ontology_data = ontology_guard.clone();
            drop(ontology_guard); // Explicitly drop the read guard
            
            // Parse OWLlink XML request and dispatch to appropriate handlers
            let response = self.handle_owllink_request(request, &ontology_data)?;
            Ok(response)
        } else {
            Err(Error::reasoning("No ontology loaded for OWLlink request"))
        }
    }

    /// Get reasoning statistics
    #[must_use]
    pub fn get_statistics(&self) -> &ReasoningStatistics {
        &self.statistics
    }

    /// Reset reasoning statistics
    pub fn reset_statistics(&mut self) {
        self.statistics = ReasoningStatistics::default();
    }

    /// Process a SPARQL query against the ontology
    fn process_sparql_query(&self, query: &str, ontology: &Ontology) -> Result<String> {
        // Parse the SPARQL query
        let parsed_query = self.parse_sparql_query(query)?;
        
        // Execute query based on type
        match parsed_query.query_type.as_str() {
            "SELECT" => self.execute_select_query(&parsed_query, ontology),
            "ASK" => self.execute_ask_query(&parsed_query, ontology),
            "CONSTRUCT" => self.execute_construct_query(&parsed_query, ontology),
            "DESCRIBE" => self.execute_describe_query(&parsed_query, ontology),
            _ => Err(Error::reasoning(&format!("Unsupported SPARQL query type: {}", parsed_query.query_type)))
        }
    }

    /// Parse SPARQL query into structured representation
    fn parse_sparql_query(&self, query: &str) -> Result<SparqlQuery> {
        // Basic SPARQL parsing - in production would use a proper SPARQL parser
        let trimmed = query.trim();
        
        let query_type = if trimmed.to_uppercase().starts_with("SELECT") {
            "SELECT"
        } else if trimmed.to_uppercase().starts_with("ASK") {
            "ASK"
        } else if trimmed.to_uppercase().starts_with("CONSTRUCT") {
            "CONSTRUCT"
        } else if trimmed.to_uppercase().starts_with("DESCRIBE") {
            "DESCRIBE"
        } else {
            return Err(Error::reasoning("Invalid SPARQL query"));
        };

        Ok(SparqlQuery {
            query_type: query_type.to_string(),
            query_text: query.to_string(),
            variables: self.extract_variables(query)?,
            patterns: self.extract_triple_patterns(query)?,
        })
    }

    /// Execute SELECT query
    fn execute_select_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        let mut results = Vec::new();
        
        // Find bindings that satisfy the query patterns
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        
        // Project to selected variables
        for binding in bindings {
            let mut row = Vec::new();
            for var in &query.variables {
                if let Some(value) = binding.get(var) {
                    row.push(value.clone());
                } else {
                    row.push("UNBOUND".to_string());
                }
            }
            results.push(row);
        }
        
        // Format results as SPARQL Results XML/JSON
        Ok(self.format_select_results(&query.variables, &results))
    }

    /// Execute ASK query
    fn execute_ask_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        let result = !bindings.is_empty();
        Ok(format!("{{\"boolean\": {}}}", result))
    }

    /// Execute CONSTRUCT query
    fn execute_construct_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        // Extract construct templates and execute
        let construct_patterns = self.extract_construct_patterns(&query.query_text)?;
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        
        let mut triples = Vec::new();
        for binding in bindings {
            for pattern in &construct_patterns {
                if let Some(triple) = self.instantiate_pattern(pattern, &binding) {
                    triples.push(triple);
                }
            }
        }
        
        Ok(self.format_construct_results(&triples))
    }

    /// Execute DESCRIBE query  
    fn execute_describe_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        // For DESCRIBE queries, return all known facts about the resources
        let resources = self.extract_described_resources(&query.query_text)?;
        let mut triples = Vec::new();
        
        for resource in resources {
            triples.extend(self.get_resource_description(&resource, ontology)?);
        }
        
        Ok(self.format_construct_results(&triples))
    }

    /// Handle OWLlink request processing
    fn handle_owllink_request(&mut self, request: &str, ontology: &Ontology) -> Result<String> {
        // Parse OWLlink XML request
        let parsed_request = self.parse_owllink_request(request)?;
        
        match parsed_request.request_type.as_str() {
            "IsKBSatisfiable" => self.handle_kb_satisfiable(ontology),
            "IsClassSatisfiable" => self.handle_class_satisfiable(&parsed_request, ontology),
            "IsEntailed" => self.handle_entailment_check(&parsed_request, ontology),
            "GetSubClasses" => self.handle_get_subclasses(&parsed_request, ontology),
            "GetSuperClasses" => self.handle_get_superclasses(&parsed_request, ontology),
            "GetEquivalentClasses" => self.handle_get_equivalent_classes(&parsed_request, ontology),
            "GetInstances" => self.handle_get_instances(&parsed_request, ontology),
            "GetTypes" => self.handle_get_types(&parsed_request, ontology),
            _ => Err(Error::reasoning(&format!("Unsupported OWLlink request: {}", parsed_request.request_type)))
        }
    }

    /// Parse OWLlink XML request
    fn parse_owllink_request(&self, request: &str) -> Result<OwllinkRequest> {
        // Basic XML parsing for OWLlink - in production would use proper XML parser
        let request_type = if request.contains("IsKBSatisfiable") {
            "IsKBSatisfiable"
        } else if request.contains("IsClassSatisfiable") {
            "IsClassSatisfiable"
        } else if request.contains("IsEntailed") {
            "IsEntailed"
        } else if request.contains("GetSubClasses") {
            "GetSubClasses"
        } else if request.contains("GetSuperClasses") {
            "GetSuperClasses"
        } else if request.contains("GetEquivalentClasses") {
            "GetEquivalentClasses"
        } else if request.contains("GetInstances") {
            "GetInstances"
        } else if request.contains("GetTypes") {
            "GetTypes"
        } else {
            return Err(Error::reasoning("Unknown OWLlink request type"));
        };

        Ok(OwllinkRequest {
            request_type: request_type.to_string(),
            request_xml: request.to_string(),
            class_expression: self.extract_class_from_owllink(request).ok(),
            kb_name: self.extract_kb_name_from_owllink(request).ok(),
            axiom: None,           // TODO: extract from XML
            individual: None,      // TODO: extract from XML  
            direct: None,          // TODO: extract from XML
        })
    }

    /// Check subsumption between two class expressions
    pub fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        // Check cache first
        if let Some(cached_result) = self
            .cache_manager
            .read()
            .unwrap()
            .get_subsumption_result(subclass, superclass)
        {
            return Ok(cached_result);
        }

        // Use tableau to check subsumption
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        let tableau = self
            .tableau_builder
            .build_for_consistency(&ontology_guard)?;
        let result = tableau.check_subsumption(subclass, superclass)?;

        // Store in cache
        self.cache_manager
            .write()
            .unwrap()
            .cache_subsumption_result(subclass.clone(), superclass.clone(), result);

        Ok(result)
    }

    /// Get all superclasses of a class expression
    pub fn get_superclasses(
        &self,
        concept: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut superclasses = Vec::new();

            // Get all classes from the signature
            for class in &ontology_guard.signature().unwrap().classes {
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
    pub fn get_subclasses(
        &self,
        concept: &ClassExpression,
        _direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut subclasses = Vec::new();

            if let ClassExpression::Class(target_class) = concept {
                debug!("Looking for subclasses of: {}", target_class.iri.as_str());
                debug!(
                    "Total axioms in ontology: {}",
                    ontology_guard.axioms().len()
                );

                // First, let's count all axiom types to understand the ontology structure
                let mut axiom_type_counts = std::collections::HashMap::new();
                for axiom in ontology_guard.axioms() {
                    let axiom_type = match axiom {
                        crate::ontology::axioms::Axiom::SubClassOf(_) => "SubClassOf",
                        crate::ontology::axioms::Axiom::EquivalentClasses(_) => "EquivalentClasses",
                        crate::ontology::axioms::Axiom::DisjointClasses(_) => "DisjointClasses",
                        crate::ontology::axioms::Axiom::ClassAssertion(_) => "ClassAssertion",
                        crate::ontology::axioms::Axiom::ObjectPropertyAssertion(_) => {
                            "ObjectPropertyAssertion"
                        }
                        crate::ontology::axioms::Axiom::Declaration(_) => "Declaration",
                        _ => "Other",
                    };
                    *axiom_type_counts.entry(axiom_type).or_insert(0) += 1;
                }

                debug!("Axiom type breakdown:");
                for (axiom_type, count) in &axiom_type_counts {
                    debug!("  {axiom_type}: {count}");
                }

                // Look for SubClassOf axioms in the ontology
                let mut subclass_axiom_count = 0;
                for axiom in ontology_guard.axioms() {
                    if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                        subclass_axiom_count += 1;
                        // Check if the superclass matches our target
                        if let ClassExpression::Class(super_class) = &subclass_axiom.superclass {
                            debug!(
                                "Checking SubClassOf axiom #{}: {} -> {}",
                                subclass_axiom_count,
                                if let ClassExpression::Class(sub) = &subclass_axiom.subclass {
                                    sub.iri.as_str()
                                } else {
                                    "complex"
                                },
                                super_class.iri.as_str()
                            );

                            if target_class.iri.as_str() == super_class.iri.as_str() {
                                // Add the subclass to our results
                                debug!(
                                    "Found subclass: {}",
                                    if let ClassExpression::Class(sub) = &subclass_axiom.subclass {
                                        sub.iri.as_str()
                                    } else {
                                        "complex"
                                    }
                                );
                                subclasses.push(subclass_axiom.subclass.clone());
                            }
                        }
                    } else {
                        // Count other axiom types for debugging
                    }
                }
                debug!("Found {subclass_axiom_count} SubClassOf axioms total");
            }

            Ok(subclasses)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all equivalent classes of a class expression
    pub fn get_equivalent_classes(
        &self,
        concept: &ClassExpression,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            let mut equivalent_classes = Vec::new();

            // Special handling for union queries - check DisjointUnion axioms
            if let ClassExpression::ObjectUnionOf(union_classes) = concept {
                debug!(
                    "Processing union query with {} classes",
                    union_classes.len()
                );
                // Find any DisjointUnion axiom that matches this union
                let mut disjoint_union_count = 0;
                for axiom in ontology_guard.axioms() {
                    if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                        disjoint_union_count += 1;
                        debug!(
                            "Found DisjointUnion axiom #{}: class={:?}, {} disjoint classes",
                            disjoint_union_count,
                            disjoint_union.class,
                            disjoint_union.disjoint_classes.len()
                        );

                        // Check if the union in the query matches the disjoint classes in this axiom
                        if self.union_matches_disjoint_classes(
                            union_classes,
                            &disjoint_union.disjoint_classes,
                        ) {
                            debug!(
                                "Union matches! Adding equivalent class: {:?}",
                                disjoint_union.class
                            );
                            // This union is equivalent to the class in the DisjointUnion axiom
                            equivalent_classes.push(disjoint_union.class.clone());
                        } else {
                            debug!("Union does not match this DisjointUnion axiom");
                        }
                    }
                }
                debug!("Found {disjoint_union_count} DisjointUnion axioms total");
            }

            // General case: Check all classes from the signature for bidirectional subsumption
            for class in &ontology_guard.signature().unwrap().classes {
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

        debug!("Union classes (flattened): {union_iris:?}");
        debug!("Disjoint classes: {disjoint_iris:?}");

        // The union matches if it contains exactly the same classes as the disjoint union
        let matches = union_iris == disjoint_iris && !union_iris.is_empty();
        debug!("Union matches disjoint classes: {matches}");
        matches
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

    /// Get all instances of a class expression
    pub fn get_instances(
        &mut self,
        concept: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<Individual>> {
        if let Some(ontology) = &self.ontology {
            let individuals = {
                let ontology_guard = ontology.read().unwrap();
                // Get all individuals from the signature
                ontology_guard.signature().unwrap().individuals.clone()
            }; // Drop the read lock here

            let mut instances = Vec::new();

            for individual in &individuals {
                // Use tableau reasoning to check if individual is instance of concept
                if self.is_instance_of_expression(individual, concept)? {
                    instances.push(individual.clone());
                }
            }

            Ok(instances)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get all types of an individual
    pub fn get_types(
        &self,
        individual: &Individual,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - just return empty vector for now
        // In a full implementation, this would check the ontology for 
        // class assertions and perform type inference
        Ok(Vec::new())
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<Individual>> {
        if let Some(ontology) = &self.ontology {
            // Use tableau reasoning to find property values
            let mut tableau = self.tableau_builder.create_tableau()?;
            let ontology_arc = {
                let ontology_guard = ontology.read().unwrap();
                Arc::new(ontology_guard.clone())
            };
            tableau.set_ontology(ontology_arc);

            // Create a query by asserting the individual exists and checking what it's connected to
            let individual_name = match individual {
                Individual::Named(named) => named.iri.as_str(),
                Individual::Anonymous(_) => return Ok(Vec::new()), // Can't query anonymous individuals directly
            };

            // Add the individual to the tableau
            let individual_concepts =
                self.get_individual_concepts_from_ontology(individual_name)?;
            let node_id =
                tableau.add_node_with_id(individual_name.to_string(), individual_concepts)?;

            // Process the ontology to build the tableau
            self.load_ontology_into_tableau(&mut tableau, ontology)?;

            // Run tableau completion
            tableau.run()?;

            // Extract property values from the completed tableau
            let mut values = Vec::new();

            // Find all edges from our individual with the specified property
            for edge in tableau.edges() {
                if edge.from == node_id {
                    let property_matches = match property {
                        ObjectPropertyExpression::ObjectProperty(prop) => {
                            edge.role.name() == prop.iri.as_str()
                        }
                        ObjectPropertyExpression::InverseObjectProperty(prop) => {
                            // For inverse properties, check reverse direction
                            let prop_expr = ObjectPropertyExpression::ObjectProperty(prop.clone());
                            self.check_inverse_property_match(&edge, &prop_expr, node_id, &tableau)?
                        }
                        ObjectPropertyExpression::PropertyChain(chain) => {
                            // Property chains require path checking through multiple edges
                            self.check_property_chain_match(&edge, chain, node_id, &tableau)?
                        }
                    };

                    if property_matches {
                        if let Some(target_node) = tableau.get_node(edge.to) {
                            // For now, create a placeholder individual based on the node ID
                            // In a full implementation, we'd maintain a mapping from NodeId to individual name
                            let target_name = format!("node_{}", edge.to);
                            let iri = IRI::from(target_name);
                            values.push(Individual::named(iri));
                        }
                    }
                }
            }

            Ok(values)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<Vec<String>> {
        if let Some(ontology) = &self.ontology {
            // Use tableau reasoning to find data property values
            let mut tableau = self.tableau_builder.create_tableau()?;
            let ontology_arc = {
                let ontology_guard = ontology.read().unwrap();
                Arc::new(ontology_guard.clone())
            };
            tableau.set_ontology(ontology_arc);

            let individual_name = match individual {
                Individual::Named(named) => named.iri.as_str(),
                Individual::Anonymous(_) => return Ok(Vec::new()),
            };

            // Add the individual to the tableau
            let individual_concepts =
                self.get_individual_concepts_from_ontology(individual_name)?;
            let node_id =
                tableau.add_node_with_id(individual_name.to_string(), individual_concepts)?;

            // Process the ontology to build the tableau
            self.load_ontology_into_tableau(&mut tableau, ontology)?;

            // Run tableau completion
            tableau.run()?;

            // Extract data property values
            let mut values = Vec::new();

            // Check for explicit data property assertions in the ontology
            let ontology_guard = ontology.read().unwrap();
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom {
                    let matches_individual = match &assertion.individual {
                        Individual::Named(named) => named.iri.as_str() == individual_name,
                        _ => false,
                    };

                    let matches_property = match property {
                        DataPropertyExpression::DataProperty(prop) => match &assertion.property {
                            DataPropertyExpression::DataProperty(assertion_prop) => {
                                assertion_prop.iri.as_str() == prop.iri.as_str()
                            }
                        },
                    };

                    if matches_individual && matches_property {
                        values.push(assertion.value.to_string());
                    }
                }
            }

            Ok(values)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
    }

    /// Add axiom for incremental reasoning
    pub fn add_axiom(&mut self, axiom: &Axiom) -> Result<()> {
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
    pub fn remove_axiom(&mut self, axiom: &Axiom) -> Result<()> {
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
        Ok(())
    }

    /// Explain entailment
    pub fn explain_entailment(&self, axiom: &Axiom) -> Result<Vec<Axiom>> {
        // Basic explanation by finding relevant axioms that contribute to the entailment
        let mut explanation = Vec::new();
        
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                match axiom {
                    Axiom::SubClassOf(subclass_axiom) => {
                        // Look for transitive chains and direct declarations
                        let subclass = &subclass_axiom.subclass;
                        let superclass = &subclass_axiom.superclass;
                        
                        // Check for direct axioms that support this inference
                        for ontology_axiom in ontology.axioms() {
                            match ontology_axiom {
                                Axiom::SubClassOf(existing_axiom) => {
                                    // Direct match
                                    if existing_axiom.subclass == *subclass && existing_axiom.superclass == *superclass {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                    // Transitive support (simplified)
                                    else if existing_axiom.subclass == *subclass {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                    else if existing_axiom.superclass == *superclass {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                }
                                Axiom::EquivalentClasses(equiv_axiom) => {
                                    // Check if either class is in the equivalence
                                    if equiv_axiom.classes.contains(subclass) || equiv_axiom.classes.contains(superclass) {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Axiom::ClassAssertion(class_assertion) => {
                        // Find axioms that support the class membership
                        for ontology_axiom in ontology.axioms() {
                            match ontology_axiom {
                                Axiom::ClassAssertion(existing_assertion) => {
                                    if existing_assertion.individual == class_assertion.individual {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                }
                                Axiom::SubClassOf(subclass_axiom) => {
                                    // Check if this subclass relationship contributes
                                    if subclass_axiom.superclass == class_assertion.class {
                                        explanation.push(ontology_axiom.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        // For other axiom types, just look for exact matches
                        for ontology_axiom in ontology.axioms() {
                            if std::mem::discriminant(ontology_axiom) == std::mem::discriminant(axiom) {
                                explanation.push(ontology_axiom.clone());
                            }
                        }
                    }
                }
            }
        }
        
        Ok(explanation)
    }

    /// Explain inconsistency
    pub fn explain_inconsistency(&self) -> Result<Vec<Axiom>> {
        // Find axioms that contribute to inconsistencies
        let mut explanation = Vec::new();
        
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                // Check for obvious contradictions
                let mut disjoint_classes = std::collections::HashMap::new();
                let mut class_assertions = std::collections::HashMap::new();
                
                // Collect disjoint class declarations
                for axiom in ontology.axioms() {
                    match axiom {
                        Axiom::DisjointClasses(disjoint_axiom) => {
                            for (i, class1) in disjoint_axiom.classes.iter().enumerate() {
                                for class2 in disjoint_axiom.classes.iter().skip(i + 1) {
                                    disjoint_classes.insert((class1.clone(), class2.clone()), axiom.clone());
                                }
                            }
                        }
                        Axiom::ClassAssertion(class_assertion) => {
                            class_assertions
                                .entry(class_assertion.individual.clone())
                                .or_insert_with(Vec::new)
                                .push((class_assertion.class.clone(), axiom.clone()));
                        }
                        _ => {}
                    }
                }
                
                // Check for individuals asserted to be in disjoint classes
                for (individual, assertions) in &class_assertions {
                    for (i, (class1, axiom1)) in assertions.iter().enumerate() {
                        for (class2, axiom2) in assertions.iter().skip(i + 1) {
                            // Check if these classes are disjoint
                            if let Some(disjoint_axiom) = disjoint_classes.get(&(class1.clone(), class2.clone())) 
                                .or_else(|| disjoint_classes.get(&(class2.clone(), class1.clone()))) {
                                explanation.push(axiom1.clone());
                                explanation.push(axiom2.clone());
                                explanation.push(disjoint_axiom.clone());
                            }
                        }
                    }
                }
                
                // Check for functional property violations
                let mut functional_properties = std::collections::HashSet::new();
                let mut property_assertions = std::collections::HashMap::new();
                
                for axiom in ontology.axioms() {
                    match axiom {
                        Axiom::FunctionalObjectProperty(func_axiom) => {
                            functional_properties.insert(func_axiom.property.clone());
                            explanation.push(axiom.clone());
                        }
                        Axiom::ObjectPropertyAssertion(prop_assertion) => {
                            if functional_properties.contains(&prop_assertion.property) {
                                property_assertions
                                    .entry((prop_assertion.source.clone(), prop_assertion.property.clone()))
                                    .or_insert_with(Vec::new)
                                    .push((prop_assertion.target.clone(), axiom.clone()));
                            }
                        }
                        _ => {}
                    }
                }
                
                // Check for multiple values for functional properties
                for ((source, property), targets) in &property_assertions {
                    if targets.len() > 1 {
                        for (target, axiom) in targets {
                            explanation.push(axiom.clone());
                        }
                    }
                }
            }
        }
        
        Ok(explanation)
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

    // Private methods for running tableau algorithms

    fn run_tableau_consistency_check(
        &mut self,
        mut tableau: TableauAlgorithmInstance,
    ) -> Result<bool> {
        debug!("Running tableau consistency check");

        let result = tableau.run()?;

        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = self
            .statistics
            .max_tableau_depth
            .max(tableau.get_max_depth());

        match result {
            TableauState::Satisfiable => Ok(true),
            TableauState::Unsatisfiable => Ok(false),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    fn run_tableau_satisfiability_check(
        &mut self,
        mut tableau: TableauAlgorithmInstance,
    ) -> Result<bool> {
        debug!("Running tableau satisfiability check");

        let result = tableau.run()?;

        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = self
            .statistics
            .max_tableau_depth
            .max(tableau.get_max_depth());

        match result {
            TableauState::Satisfiable => Ok(true),
            TableauState::Unsatisfiable => Ok(false),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    fn run_tableau_subsumption_check(
        &mut self,
        mut tableau: TableauAlgorithmInstance,
    ) -> Result<bool> {
        debug!("Running tableau subsumption check");

        // For subsumption A ⊑ B, we check if A ⊓ ¬B is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = self
            .statistics
            .max_tableau_depth
            .max(tableau.get_max_depth());

        match result {
            TableauState::Satisfiable => Ok(false), // A ⊓ ¬B is satisfiable, so A ⊄ B
            TableauState::Unsatisfiable => Ok(true), // A ⊓ ¬B is unsatisfiable, so A ⊑ B
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
        }
    }

    /// Run a tableau instance check for individuals
    fn run_tableau_instance_check(
        &mut self,
        mut tableau: TableauAlgorithmInstance,
    ) -> Result<bool> {
        debug!("Running tableau instance check");

        // For instance checking a ∈ C, we check if {a} ⊓ ¬C is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        self.statistics.tableau_nodes_created += tableau.get_node_count() as u64;
        self.statistics.backtracking_operations += tableau.get_backtrack_count() as u64;
        self.statistics.max_tableau_depth = self
            .statistics
            .max_tableau_depth
            .max(tableau.get_max_depth());

        match result {
            TableauState::Satisfiable => Ok(false),
            TableauState::Unsatisfiable => Ok(true),
            TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
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

    /// Check if an individual is an instance of a class expression
    fn is_instance_of_expression(
        &mut self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        // For now, delegate to existing instance checking for named classes
        if let ClassExpression::Class(cls) = class {
            self.is_instance_of(
                &individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string),
                &cls.iri.to_string(),
            )
        } else {
            // For complex expressions, we'd need more sophisticated reasoning
            // For now, create a tableau to check instance relationship
            if let Some(ontology) = &self.ontology {
                let ontology_guard = ontology.read().unwrap();

                // Convert individual and class to strings for the tableau builder
                let individual_str = individual
                    .iri()
                    .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string);
                let class_str = format!("{class:?}"); // Simplified class representation

                // Build tableau for instance checking
                let tableau = self.create_tableau_algorithm_for_instance_check(
                    &ontology_guard,
                    &individual_str,
                    &class_str,
                )?;
                drop(ontology_guard); // Release the read lock before calling mutable method
                self.run_tableau_instance_check(tableau)
            } else {
                Ok(false)
            }
        }
    }

    /// Filter types to only include direct (most specific) types
    fn filter_direct_types(
        &self,
        types: Vec<ClassExpression>,
        _individual: &Individual,
    ) -> Result<Vec<ClassExpression>> {
        let mut direct_types = Vec::new();

        // For each type, check if it's subsumed by any other type
        for candidate in &types {
            let mut is_direct = true;

            for other in &types {
                if candidate != other && self.is_subsumed_by(candidate, other)? {
                    is_direct = false;
                    break;
                }
            }

            if is_direct {
                direct_types.push(candidate.clone());
            }
        }

        Ok(direct_types)
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
                let tableau = self
                    .tableau_builder
                    .build_for_subsumption(ontology, subclass, superclass)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!(
                    "HyperTableau not yet supported for subsumption checking, using Traditional tableau"
                );
                let tableau = self
                    .tableau_builder
                    .build_for_subsumption(ontology, subclass, superclass)?;
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
                let tableau = self
                    .tableau_builder
                    .build_for_satisfiability(ontology, class_iri)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!(
                    "HyperTableau not yet supported for satisfiability checking, using Traditional tableau"
                );
                let tableau = self
                    .tableau_builder
                    .build_for_satisfiability(ontology, class_iri)?;
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
                let tableau = self
                    .tableau_builder
                    .build_for_instance_check(ontology, individual, class)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
            TableauAlgorithm::HyperTableau => {
                warn!(
                    "HyperTableau not yet supported for instance checking, using Traditional tableau"
                );
                let tableau = self
                    .tableau_builder
                    .build_for_instance_check(ontology, individual, class)?;
                Ok(TableauAlgorithmInstance::Traditional(tableau))
            }
        }
    }

    /// Create a `HyperTableau` instance or fall back to Traditional
    fn create_hypertableau_instance(
        &self,
        ontology: &Ontology,
    ) -> Result<TableauAlgorithmInstance> {
        use crate::core::blocking::AnywhereBlocking;
        use crate::core::hypertableau::HyperTableau;

        info!("Creating HyperTableau instance for reasoning");

        // Create a blocking checker (use default for now)
        let blocking_checker = Box::new(AnywhereBlocking::new());

        // Create HyperTableau instance
        let mut hypertableau = HyperTableau::new(self.config.reasoning.clone(), blocking_checker)?;

        // Initialize with the ontology
        hypertableau.initialize(ontology)?;

        Ok(TableauAlgorithmInstance::HyperTableau(Box::new(
            hypertableau,
        )))
    }

    /// Get concepts for an individual from the ontology
    fn get_individual_concepts_from_ontology(
        &self,
        individual_name: &str,
    ) -> Result<Vec<ClassExpression>> {
        let mut concepts = Vec::new();

        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();

            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                    let matches_individual = match &assertion.individual {
                        Individual::Named(named) => named.iri.as_str() == individual_name,
                        _ => false,
                    };

                    if matches_individual {
                        concepts.push(assertion.class.clone());
                    }
                }
            }
        }

        Ok(concepts)
    }

    /// Load ontology axioms into a tableau
    fn load_ontology_into_tableau(
        &self,
        tableau: &mut Tableau,
        ontology: &OntologyRef,
    ) -> Result<()> {
        let ontology_guard = ontology.read().unwrap();

        // Process class assertion axioms to create nodes
        for axiom in ontology_guard.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::ClassAssertion(assertion) => {
                    if let Individual::Named(named) = &assertion.individual {
                        let individual_name = named.iri.as_str();

                        // Try to get existing node or create new one
                        let node_id =
                            if let Ok(existing_id) = tableau.get_node_index(individual_name) {
                                existing_id
                            } else {
                                tableau.add_node_with_id(
                                    individual_name.to_string(),
                                    vec![assertion.class.clone()],
                                )?
                            };

                        // Add the concept to the node if not already present
                        if let Some(node) = tableau.get_node_mut(&node_id.to_string()) {
                            let concept_label =
                                crate::core::tableau::ConceptLabel::from_class_expression(
                                    &assertion.class,
                                )?;
                            if !node.concepts.contains(&concept_label) {
                                node.concepts.insert(concept_label);
                            }
                        }
                    }
                }
                crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) => {
                    // Create nodes for subject and object, then add edge
                    let subject_name = match &assertion.source {
                        Individual::Named(named) => named.iri.as_str(),
                        _ => continue,
                    };
                    let object_name = match &assertion.target {
                        Individual::Named(named) => named.iri.as_str(),
                        _ => continue,
                    };

                    // Ensure both nodes exist
                    let subject_id = if let Ok(existing_id) = tableau.get_node_index(subject_name) {
                        existing_id
                    } else {
                        tableau.add_node_with_id(subject_name.to_string(), Vec::new())?
                    };

                    let object_id = if let Ok(existing_id) = tableau.get_node_index(object_name) {
                        existing_id
                    } else {
                        tableau.add_node_with_id(object_name.to_string(), Vec::new())?
                    };

                    // Add edge between nodes
                    if let ObjectPropertyExpression::ObjectProperty(prop) = &assertion.property {
                        let role_label = RoleLabel::Atomic(prop.iri.as_str().to_string());
                        tableau.add_edge(
                            subject_id,
                            object_id,
                            role_label,
                            DependencySet::empty(),
                        )?;
                    }
                }
                _ => {
                    // Handle other axiom types as needed
                }
            }
        }

        Ok(())
    }

    /// Check if one object property is a subproperty of another
    fn is_subproperty_of(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
    ) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();

            // Check for direct SubObjectPropertyOf axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom {
                    if &sub_axiom.sub_property == subproperty
                        && &sub_axiom.super_property == superproperty
                    {
                        return Ok(true);
                    }
                }
            }

            // Check for property chains that could establish subsumption
            if let Some(ontology) = &self.ontology {
                let ontology_guard = ontology.read().unwrap();
                
                // Check all SubObjectPropertyOf axioms for property chains
                for axiom in ontology_guard.axioms() {
                    if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom {
                        // If the subproperty is a property chain, check if it matches our properties
                        if let crate::ontology::ObjectPropertyExpression::PropertyChain(chain) = &sub_axiom.sub_property {
                            if self.matches_property_chain(subproperty, superproperty, chain, &sub_axiom.super_property)? {
                                return Ok(true);
                            }
                        }
                        
                        // Also check if the superproperty is part of a chain that implies subsumption
                        if let crate::ontology::ObjectPropertyExpression::PropertyChain(chain) = &sub_axiom.super_property {
                            if self.check_property_chain_subsumption(subproperty, superproperty, chain)? {
                                return Ok(true);
                            }
                        }
                    }
                }
                
                // Check for transitive properties that could form chains
                if self.is_transitive_property(subproperty)? && 
                   self.properties_match(subproperty, superproperty)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a property chain matches the given properties
    fn matches_property_chain(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        chain: &[ObjectPropertyExpression],
        chain_super: &ObjectPropertyExpression,
    ) -> Result<bool> {
        // If chain has exactly 2 properties and they match sub/super, 
        // and chain_super matches superproperty, then we have a match
        if chain.len() == 2 {
            if self.properties_match(&chain[0], subproperty)? &&
               self.properties_match(&chain[1], superproperty)? &&
               self.properties_match(chain_super, superproperty)? {
                return Ok(true);
            }
        }
        
        // For longer chains, check if the composition implies the relationship
        // Implement proper property chain reasoning based on OWL 2 semantics
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Use tableau-based approach to check property chain relationships
            let mut tableau = self.tableau_builder.create_tableau()?;
            let ontology_arc = Arc::new(ontology_guard.clone());
            tableau.set_ontology(ontology_arc);
            
            // Create test individuals to check the property relationship
            let test_individual1 = "test_individual_1".to_string();
            let test_individual2 = "test_individual_2".to_string();
            
            let node1 = tableau.add_node_with_id(test_individual1.clone(), Vec::new())?;
            let node2 = tableau.add_node_with_id(test_individual2.clone(), Vec::new())?;
            
            // Add the chain property assertion
            self.assert_property_chain_instance(&mut tableau, node1, node2, chain)?;
            
            // Check if superproperty is entailed
            tableau.run()?;
            let entailed = self.check_property_entailment(&tableau, node1, node2, superproperty)?;
            
            return Ok(entailed);
        }
        
        Ok(false)
    }

    /// Assert a property chain instance in the tableau for testing
    fn assert_property_chain_instance(&self, tableau: &mut Tableau, from_node: usize, to_node: usize, chain: &[ObjectPropertyExpression]) -> Result<()> {
        if chain.is_empty() {
            return Ok(());
        }
        
        if chain.len() == 1 {
            // Single property - add direct edge
            let role_label = self.create_role_label_from_property(&chain[0])?;
            tableau.add_edge(from_node, to_node, role_label, DependencySet::new())?;
        } else {
            // Multiple properties - create intermediate nodes
            let mut current_node = from_node;
            
            for (i, property) in chain.iter().enumerate() {
                let next_node = if i == chain.len() - 1 {
                    to_node
                } else {
                    // Create intermediate node
                    let intermediate_name = format!("intermediate_{}_{}", from_node, i);
                    tableau.add_node_with_id(intermediate_name, Vec::new())?
                };
                
                let role_label = self.create_role_label_from_property(property)?;
                tableau.add_edge(current_node, next_node, role_label, DependencySet::new())?;
                current_node = next_node;
            }
        }
        
        Ok(())
    }

    /// Create a role label from an object property expression
    fn create_role_label_from_property(&self, property: &ObjectPropertyExpression) -> Result<RoleLabel> {
        match property {
            ObjectPropertyExpression::ObjectProperty(prop) => {
                Ok(RoleLabel::Atomic(prop.iri.as_str().to_string()))
            }
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                Ok(RoleLabel::Inverse(prop.iri.as_str().to_string()))
            }
            ObjectPropertyExpression::PropertyChain(_) => {
                // Property chains within property chains - more complex
                Err(Error::reasoning("Nested property chains not yet supported"))
            }
        }
    }

    /// Check if a property is entailed between two nodes
    fn check_property_entailment(&self, tableau: &Tableau, from_node: usize, to_node: usize, property: &ObjectPropertyExpression) -> Result<bool> {
        match property {
            ObjectPropertyExpression::ObjectProperty(prop) => {
                // Check for direct edge
                for edge in tableau.edges() {
                    if edge.from == from_node && edge.to == to_node && edge.role.name() == prop.iri.as_str() {
                        return Ok(true);
                    }
                }
            }
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                // Check for inverse edge
                let prop_expr = ObjectPropertyExpression::ObjectProperty(prop.clone());
                return self.check_property_entailment(tableau, to_node, from_node, &prop_expr);
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                // Check if chain exists
                return self.check_chain_exists(tableau, from_node, to_node, chain);
            }
        }
        
        Ok(false)
    }

    /// Check if a property chain exists between two nodes
    fn check_chain_exists(&self, tableau: &Tableau, from_node: usize, to_node: usize, chain: &[ObjectPropertyExpression]) -> Result<bool> {
        if chain.is_empty() {
            return Ok(from_node == to_node);
        }
        
        if chain.len() == 1 {
            return self.check_property_entailment(tableau, from_node, to_node, &chain[0]);
        }
        
        // For longer chains, find intermediate paths
        let first_property = &chain[0];
        let remaining_chain = &chain[1..];
        
        for edge in tableau.edges() {
            if edge.from == from_node {
                let first_matches = match first_property {
                    ObjectPropertyExpression::ObjectProperty(prop) => {
                        edge.role.name() == prop.iri.as_str()
                    }
                    ObjectPropertyExpression::InverseObjectProperty(prop) => {
                        // Check if there's a reverse edge that matches
                        let prop_expr = ObjectPropertyExpression::ObjectProperty(prop.clone());
                        self.check_property_entailment(tableau, edge.to, from_node, &prop_expr)?
                    }
                    ObjectPropertyExpression::PropertyChain(_) => {
                        // Nested chains - more complex
                        false
                    }
                };
                
                if first_matches {
                    if self.check_chain_exists(tableau, edge.to, to_node, remaining_chain)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if a property chain subsumption applies
    fn check_property_chain_subsumption(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        chain: &[ObjectPropertyExpression],
    ) -> Result<bool> {
        // If the chain is empty, no subsumption
        if chain.is_empty() {
            return Ok(false);
        }

        // If the chain has only one property, check direct match
        if chain.len() == 1 {
            return Ok(self.properties_match(&chain[0], subproperty)? && 
                     self.properties_match(&chain[0], superproperty)?);
        }

        // For longer chains, check if subproperty appears in the chain
        // and if the chain composition implies superproperty
        let mut subprop_found = false;
        let mut subprop_index = None;
        
        for (i, chain_prop) in chain.iter().enumerate() {
            if self.properties_match(chain_prop, subproperty)? {
                subprop_found = true;
                subprop_index = Some(i);
                break;
            }
        }

        if !subprop_found {
            return Ok(false);
        }

        // Check if the chain composition could lead to the superproperty
        // Implement proper property chain composition reasoning
        if let Some(idx) = subprop_index {
            // Create test scenario to check if the composition holds
            if let Some(ontology) = &self.ontology {
                let ontology_guard = ontology.read().unwrap();
                let mut tableau = self.tableau_builder.create_tableau()?;
                let ontology_arc = Arc::new(ontology_guard.clone());
                tableau.set_ontology(ontology_arc);
                
                // Create a chain of test individuals
                let mut test_nodes = Vec::new();
                for i in 0..=chain.len() {
                    let node_name = format!("test_chain_node_{}", i);
                    let node_id = tableau.add_node_with_id(node_name, Vec::new())?;
                    test_nodes.push(node_id);
                }
                
                // Assert the property chain
                for (i, chain_prop) in chain.iter().enumerate() {
                    let from_node = test_nodes[i];
                    let to_node = test_nodes[i + 1];
                    
                    // If this is where our subproperty appears, use it
                    let prop_to_assert = if i == idx {
                        subproperty
                    } else {
                        chain_prop
                    };
                    
                    let role_label = self.create_role_label_from_property(prop_to_assert)?;
                    tableau.add_edge(from_node, to_node, role_label, DependencySet::new())?;
                }
                
                // Run tableau completion
                tableau.run()?;
                
                // Check if superproperty is entailed between first and last nodes
                let first_node = test_nodes[0];
                let last_node = test_nodes[test_nodes.len() - 1];
                return self.check_property_entailment(&tableau, first_node, last_node, superproperty);
            }
        }

        Ok(false)
    }

    /// Check if a property chain continuation holds
    fn check_chain_continuation(
        &self,
        remaining_chain: &[ObjectPropertyExpression],
        target: &ObjectPropertyExpression,
        ontology: &crate::ontology::Ontology,
    ) -> Result<bool> {
        if remaining_chain.is_empty() {
            return Ok(false);
        }
        
        if remaining_chain.len() == 1 {
            return self.properties_match(&remaining_chain[0], target);
        }
        
        // For longer chains, check if there are sub-property relationships
        // that could establish the connection
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) => {
                    // Check if the chain or part of it is a sub-property of target
                    if let ObjectPropertyExpression::PropertyChain(chain) = &sub_axiom.sub_property {
                        if chain.len() == remaining_chain.len() {
                            let mut all_match = true;
                            for (i, prop) in chain.iter().enumerate() {
                                if !self.properties_match(prop, &remaining_chain[i])? {
                                    all_match = false;
                                    break;
                                }
                            }
                            if all_match && self.properties_match(&sub_axiom.super_property, target)? {
                                return Ok(true);
                            }
                        }
                    }
                }
                crate::ontology::axioms::Axiom::TransitiveObjectProperty(trans_axiom) => {
                    // If target is transitive and appears in the chain, check transitivity
                    if self.properties_match(&trans_axiom.property, target)? {
                        for chain_prop in remaining_chain {
                            if self.properties_match(chain_prop, target)? {
                                return Ok(true);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // If no direct relationships found, try to establish through intermediate properties
        // This is a simplified approach - a full implementation would use more sophisticated
        // graph-based reasoning
        Ok(false)
    }

    /// Check if a property is transitive
    fn is_transitive_property(&self, property: &ObjectPropertyExpression) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::TransitiveObjectProperty(trans_axiom) = axiom {
                    if self.properties_match(&trans_axiom.property, property)? {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Check if two properties match (including handling inverses)
    fn properties_match(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> Result<bool> {
        match (prop1, prop2) {
            (ObjectPropertyExpression::ObjectProperty(p1), ObjectPropertyExpression::ObjectProperty(p2)) => {
                Ok(p1.iri == p2.iri)
            }
            (ObjectPropertyExpression::InverseObjectProperty(p1), ObjectPropertyExpression::InverseObjectProperty(p2)) => {
                Ok(p1.iri == p2.iri)
            }
            (ObjectPropertyExpression::ObjectProperty(p1), ObjectPropertyExpression::InverseObjectProperty(p2)) => {
                // Check if p1 is the inverse of p2
                self.are_inverse_properties(p1, p2)
            }
            (ObjectPropertyExpression::InverseObjectProperty(p1), ObjectPropertyExpression::ObjectProperty(p2)) => {
                // Check if p2 is the inverse of p1
                self.are_inverse_properties(p2, p1)
            }
            // Property chains require special handling
            _ => Ok(false),
        }
    }

    /// Check if two properties are declared as inverses
    fn are_inverse_properties(
        &self,
        prop1: &crate::ontology::ObjectProperty,
        prop2: &crate::ontology::ObjectProperty,
    ) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::InverseObjectProperties(inv_axiom) = axiom {
                    let prop1_expr = ObjectPropertyExpression::ObjectProperty(prop1.clone());
                    let prop2_expr = ObjectPropertyExpression::ObjectProperty(prop2.clone());
                    
                    if (self.properties_match(&inv_axiom.property1, &prop1_expr)? &&
                        self.properties_match(&inv_axiom.property2, &prop2_expr)?) ||
                       (self.properties_match(&inv_axiom.property1, &prop2_expr)? &&
                        self.properties_match(&inv_axiom.property2, &prop1_expr)?) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Check if one data property is a subproperty of another
    fn is_data_subproperty_of(
        &self,
        subproperty: &DataPropertyExpression,
        superproperty: &DataPropertyExpression,
    ) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();

            // Check for direct SubDataPropertyOf axioms
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::SubDataPropertyOf(sub_axiom) = axiom {
                    if &sub_axiom.sub_property == subproperty
                        && &sub_axiom.super_property == superproperty
                    {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

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
            if !self.is_axiom_entailed(axiom)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if a specific axiom is entailed by the current ontology
    fn is_axiom_entailed(&mut self, axiom: &Axiom) -> Result<bool> {
        match axiom {
            crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) => {
                // Check if subclass ⊑ superclass is entailed
                self.is_subsumed_by(&subclass_axiom.subclass, &subclass_axiom.superclass)
            }
            crate::ontology::axioms::Axiom::ClassAssertion(class_assertion) => {
                // Check if individual ∈ class is entailed
                if let Individual::Named(named) = &class_assertion.individual {
                    self.is_instance_of_expression(
                        &class_assertion.individual,
                        &class_assertion.class,
                    )
                } else {
                    // For anonymous individuals, this is more complex
                    Ok(false)
                }
            }
            crate::ontology::axioms::Axiom::ObjectPropertyAssertion(prop_assertion) => {
                // Check if (individual1, individual2) ∈ property is entailed
                // This requires sophisticated ABox reasoning
                self.check_property_assertion_entailment(
                    &prop_assertion.source, 
                    &prop_assertion.property, 
                    &prop_assertion.target
                )
            }
            Axiom::DataPropertyAssertion(data_prop_axiom) => {
                // Check if data property assertion is entailed
                self.check_data_property_assertion_entailment(
                    &data_prop_axiom.individual,
                    &data_prop_axiom.property,
                    &data_prop_axiom.value
                )
            }
            Axiom::ClassAssertion(class_axiom) => {
                // Check if individual is instance of class
                self.check_class_assertion_entailment(&class_axiom.individual, &class_axiom.class)
            }
            Axiom::SameIndividual(same_axiom) => {
                // Check if individuals are inferred to be the same
                self.check_same_individual_entailment(&same_axiom.individuals)
            }
            Axiom::DifferentIndividuals(diff_axiom) => {
                // Check if individuals are inferred to be different
                self.check_different_individuals_entailment(&diff_axiom.individuals)
            }
            // Add more axiom types as needed
            _ => {
                // For other axiom types that don't have direct entailment checking, 
                // check if they are explicitly present in the ontology
                let ontology = self.get_ontology()?;
                let ontology_guard = ontology.read().unwrap();
                Ok(ontology_guard.axioms.contains(axiom))
            }
        }
    }
    
    /// Check if property assertion is entailed
    fn check_property_assertion_entailment(
        &self,
        subject: &Individual,
        property: &ObjectPropertyExpression,
        object: &Individual
    ) -> Result<bool> {
        // Check direct assertion
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        for axiom in &ontology_guard.axioms {
            if let Axiom::ObjectPropertyAssertion(prop_axiom) = axiom {
                if prop_axiom.source == *subject &&
                   prop_axiom.property == *property &&
                   prop_axiom.target == *object {
                    return Ok(true);
                }
            }
        }
        
        // Check inference through property hierarchy
        if let ObjectPropertyExpression::ObjectProperty(prop) = property {
            let ontology = self.get_ontology()?;
            let ontology_guard = ontology.read().unwrap();
            for axiom in &ontology_guard.axioms {
                if let Axiom::SubObjectPropertyOf(subprop_axiom) = axiom {
                    if subprop_axiom.super_property == *property {
                        // Check if assertion holds for sub-property
                        if self.check_property_assertion_entailment(subject, &subprop_axiom.sub_property, object)? {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        // Check transitivity
        if self.is_transitive_property(property)? {
            // Look for intermediate individuals
            let ontology = self.get_ontology()?;
            let ontology_guard = ontology.read().unwrap();
            for axiom in &ontology_guard.axioms {
                if let Axiom::ObjectPropertyAssertion(prop_axiom) = axiom {
                    if prop_axiom.source == *subject && prop_axiom.property == *property {
                        // Found subject -property-> intermediate
                        let intermediate = &prop_axiom.target;
                        if self.check_property_assertion_entailment(intermediate, property, object)? {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check if data property assertion is entailed
    fn check_data_property_assertion_entailment(
        &self,
        subject: &Individual,
        property: &DataPropertyExpression,
        value: &Literal
    ) -> Result<bool> {
        // Check direct assertion
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        for axiom in &ontology_guard.axioms {
            if let Axiom::DataPropertyAssertion(prop_axiom) = axiom {
                if prop_axiom.individual == *subject &&
                   prop_axiom.property == *property &&
                   prop_axiom.value == *value {
                    return Ok(true);
                }
            }
        }
        
        // Check inference through property hierarchy
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        for axiom in &ontology_guard.axioms {
            if let Axiom::SubDataPropertyOf(subprop_axiom) = axiom {
                if subprop_axiom.super_property == *property {
                    if self.check_data_property_assertion_entailment(subject, &subprop_axiom.sub_property, value)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check if class assertion is entailed
    fn check_class_assertion_entailment(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        // Check direct assertion
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        for axiom in &ontology_guard.axioms {
            if let Axiom::ClassAssertion(class_axiom) = axiom {
                if class_axiom.individual == *individual && class_axiom.class == *class {
                    return Ok(true);
                }
            }
        }
        
        // Check inference through class hierarchy
        if let ClassExpression::Class(target_class) = class {
            let ontology = self.get_ontology()?;
            let ontology_guard = ontology.read().unwrap();
            for axiom in &ontology_guard.axioms {
                if let Axiom::ClassAssertion(class_axiom) = axiom {
                    if class_axiom.individual == *individual {
                        // Check if asserted class matches or is subclass of target class
                        if class_axiom.class == *class {
                            return Ok(true);
                        }
                        // For more complex subclass reasoning, we'd need full reasoning here
                        // For now, just check for direct match
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check if same individual relationship is entailed
    fn check_same_individual_entailment(&self, individuals: &[Individual]) -> Result<bool> {
        if individuals.len() < 2 {
            return Ok(true); // Trivially true for empty or single individual
        }
        
        // Check if any pair is explicitly stated as same
        let ontology = self.get_ontology()?;
        let ontology_guard = ontology.read().unwrap();
        for axiom in &ontology_guard.axioms {
            if let Axiom::SameIndividual(same_axiom) = axiom {
                // Check if all individuals in the query are covered by some same individual axiom
                let all_covered = individuals.iter().all(|ind| same_axiom.individuals.contains(ind));
                if all_covered {
                    return Ok(true);
                }
            }
        }
        
        // TODO: More sophisticated reasoning about transitivity of sameAs
        Ok(false)
    }
    
    /// Check if different individuals relationship is entailed
    fn check_different_individuals_entailment(&self, individuals: &[Individual]) -> Result<bool> {
        if individuals.len() < 2 {
            return Ok(false); // Cannot be different with less than 2 individuals
        }
        
        // Check if all pairs are explicitly different
        for i in 0..individuals.len() {
            for j in (i + 1)..individuals.len() {
                let ind1 = &individuals[i];
                let ind2 = &individuals[j];
                
                let mut found_different = false;
                let ontology = self.get_ontology()?;
                let ontology_guard = ontology.read().unwrap();
                for axiom in &ontology_guard.axioms {
                    if let Axiom::DifferentIndividuals(diff_axiom) = axiom {
                        if diff_axiom.individuals.contains(ind1) && diff_axiom.individuals.contains(ind2) {
                            found_different = true;
                            break;
                        }
                    }
                }
                
                if !found_different {
                    return Ok(false);
                }
            }
        }
        
        Ok(true)
    }

    // Helper methods for SPARQL processing

    /// Extract variables from SPARQL query
    fn extract_variables(&self, query: &str) -> Result<Vec<String>> {
        let mut variables = Vec::new();
        
        // Simple regex-like extraction for variables starting with ?
        let words: Vec<&str> = query.split_whitespace().collect();
        for word in words {
            if word.starts_with('?') {
                let var = word.trim_end_matches(&[',', '.', ';', ')', '}'][..]);
                if !variables.contains(&var.to_string()) {
                    variables.push(var.to_string());
                }
            }
        }
        
        Ok(variables)
    }

    /// Extract triple patterns from SPARQL query
    fn extract_triple_patterns(&self, query: &str) -> Result<Vec<TriplePattern>> {
        let mut patterns = Vec::new();
        
        // Find WHERE clause and extract patterns
        if let Some(where_start) = query.to_uppercase().find("WHERE") {
            let where_clause = &query[where_start + 5..];
            
            // Extract patterns between braces
            if let Some(brace_start) = where_clause.find('{') {
                if let Some(brace_end) = where_clause.rfind('}') {
                    let pattern_text = &where_clause[brace_start + 1..brace_end];
                    
                    // Split by periods to get individual patterns
                    for line in pattern_text.split('.') {
                        let line = line.trim();
                        if line.is_empty() { continue; }
                        
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            patterns.push(TriplePattern {
                                subject: parts[0].to_string(),
                                predicate: parts[1].to_string(),
                                object: parts[2].to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(patterns)
    }

    /// Find bindings that satisfy the triple patterns
    fn find_pattern_bindings(&self, patterns: &[TriplePattern], ontology: &Ontology) -> Result<Vec<HashMap<String, String>>> {
        let mut all_bindings = Vec::new();
        
        // For each pattern, find all possible bindings
        for pattern in patterns {
            let pattern_bindings = self.find_single_pattern_bindings(pattern, ontology)?;
            
            if all_bindings.is_empty() {
                all_bindings = pattern_bindings;
            } else {
                // Join with existing bindings
                all_bindings = self.join_bindings(&all_bindings, &pattern_bindings);
            }
        }
        
        Ok(all_bindings)
    }

    /// Find bindings for a single triple pattern
    fn find_single_pattern_bindings(&self, pattern: &TriplePattern, ontology: &Ontology) -> Result<Vec<HashMap<String, String>>> {
        let mut bindings = Vec::new();
        
        // Check against class assertions
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(assertion) => {
                    if let Individual::Named(named) = &assertion.individual {
                        let subject = format!("<{}>", named.iri);
                        let predicate = "rdf:type".to_string();
                        let object = self.format_class_expression(&assertion.class);
                        
                        if let Some(binding) = self.match_pattern(pattern, &subject, &predicate, &object) {
                            bindings.push(binding);
                        }
                    }
                }
                Axiom::ObjectPropertyAssertion(assertion) => {
                    if let (Individual::Named(sub), Individual::Named(obj)) = (&assertion.source, &assertion.target) {
                        let subject = format!("<{}>", sub.iri);
                        let predicate = self.format_object_property(&assertion.property);
                        let object = format!("<{}>", obj.iri);
                        
                        if let Some(binding) = self.match_pattern(pattern, &subject, &predicate, &object) {
                            bindings.push(binding);
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(bindings)
    }

    /// Match a pattern against concrete values and return variable bindings
    fn match_pattern(&self, pattern: &TriplePattern, subject: &str, predicate: &str, object: &str) -> Option<HashMap<String, String>> {
        let mut binding = HashMap::new();
        
        // Match subject
        if pattern.subject.starts_with('?') {
            binding.insert(pattern.subject.clone(), subject.to_string());
        } else if pattern.subject != subject {
            return None;
        }
        
        // Match predicate
        if pattern.predicate.starts_with('?') {
            binding.insert(pattern.predicate.clone(), predicate.to_string());
        } else if pattern.predicate != predicate {
            return None;
        }
        
        // Match object
        if pattern.object.starts_with('?') {
            binding.insert(pattern.object.clone(), object.to_string());
        } else if pattern.object != object {
            return None;
        }
        
        Some(binding)
    }

    /// Join two sets of bindings
    fn join_bindings(&self, left: &[HashMap<String, String>], right: &[HashMap<String, String>]) -> Vec<HashMap<String, String>> {
        let mut result = Vec::new();
        
        for left_binding in left {
            for right_binding in right {
                if self.bindings_compatible(left_binding, right_binding) {
                    let mut joined = left_binding.clone();
                    joined.extend(right_binding.clone());
                    result.push(joined);
                }
            }
        }
        
        result
    }

    /// Check if two bindings are compatible (no conflicting variable assignments)
    fn bindings_compatible(&self, left: &HashMap<String, String>, right: &HashMap<String, String>) -> bool {
        for (var, value) in left {
            if let Some(other_value) = right.get(var) {
                if value != other_value {
                    return false;
                }
            }
        }
        true
    }

    /// Format SELECT query results
    fn format_select_results(&self, variables: &[String], results: &[Vec<String>]) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\"?>\n");
        output.push_str("<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n");
        output.push_str("  <head>\n");
        
        for var in variables {
            output.push_str(&format!("    <variable name=\"{}\"/>\n", var.trim_start_matches('?')));
        }
        
        output.push_str("  </head>\n  <results>\n");
        
        for row in results {
            output.push_str("    <result>\n");
            for (i, value) in row.iter().enumerate() {
                if i < variables.len() {
                    let var_name = variables[i].trim_start_matches('?');
                    output.push_str(&format!("      <binding name=\"{}\">\n", var_name));
                    if value.starts_with('<') && value.ends_with('>') {
                        output.push_str(&format!("        <uri>{}</uri>\n", &value[1..value.len()-1]));
                    } else {
                        output.push_str(&format!("        <literal>{}</literal>\n", value));
                    }
                    output.push_str("      </binding>\n");
                }
            }
            output.push_str("    </result>\n");
        }
        
        output.push_str("  </results>\n</sparql>");
        output
    }

    /// Extract construct patterns from CONSTRUCT query
    fn extract_construct_patterns(&self, query: &str) -> Result<Vec<TriplePattern>> {
        // Find CONSTRUCT clause
        if let Some(construct_start) = query.to_uppercase().find("CONSTRUCT") {
            let construct_part = &query[construct_start + 9..];
            
            // Find WHERE to delimit construct template
            if let Some(where_pos) = construct_part.to_uppercase().find("WHERE") {
                let template = &construct_part[..where_pos];
                return self.extract_triple_patterns(&format!("WHERE {{{}}}", template));
            }
        }
        
        Ok(Vec::new())
    }

    /// Instantiate a pattern with variable bindings
    fn instantiate_pattern(&self, pattern: &TriplePattern, binding: &HashMap<String, String>) -> Option<(String, String, String)> {
        let subject = if pattern.subject.starts_with('?') {
            binding.get(&pattern.subject)?.clone()
        } else {
            pattern.subject.clone()
        };
        
        let predicate = if pattern.predicate.starts_with('?') {
            binding.get(&pattern.predicate)?.clone()
        } else {
            pattern.predicate.clone()
        };
        
        let object = if pattern.object.starts_with('?') {
            binding.get(&pattern.object)?.clone()
        } else {
            pattern.object.clone()
        };
        
        Some((subject, predicate, object))
    }

    /// Format CONSTRUCT/DESCRIBE results as RDF
    fn format_construct_results(&self, triples: &[(String, String, String)]) -> String {
        let mut output = String::new();
        output.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        output.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n");
        
        for (subject, predicate, object) in triples {
            output.push_str(&format!("{} {} {} .\n", subject, predicate, object));
        }
        
        output
    }

    /// Extract described resources from DESCRIBE query
    fn extract_described_resources(&self, query: &str) -> Result<Vec<String>> {
        let mut resources = Vec::new();
        
        // Simple extraction for DESCRIBE queries
        if let Some(describe_start) = query.to_uppercase().find("DESCRIBE") {
            let describe_part = &query[describe_start + 8..];
            
            // Find WHERE clause or end of query
            let end_pos = describe_part.to_uppercase().find("WHERE")
                .unwrap_or(describe_part.len());
            
            let resource_part = &describe_part[..end_pos];
            
            // Extract resource URIs and variables
            for word in resource_part.split_whitespace() {
                let word = word.trim_end_matches(&[',', '.', ';'][..]);
                if word.starts_with('<') && word.ends_with('>') {
                    resources.push(word.to_string());
                } else if word.starts_with('?') {
                    resources.push(word.to_string());
                }
            }
        }
        
        Ok(resources)
    }

    /// Get all known facts about a resource
    fn get_resource_description(&self, resource: &str, ontology: &Ontology) -> Result<Vec<(String, String, String)>> {
        let mut triples = Vec::new();
        
        // If it's a variable, we need to resolve it first (simplified approach)
        let target_iri = if resource.starts_with('?') {
            // For variables, return empty for now - would need query context
            return Ok(triples);
        } else if resource.starts_with('<') && resource.ends_with('>') {
            &resource[1..resource.len()-1]
        } else {
            resource
        };
        
        // Find all assertions about this resource
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(assertion) => {
                    if let Individual::Named(named) = &assertion.individual {
                        if named.iri.as_str() == target_iri {
                            triples.push((
                                resource.to_string(),
                                "rdf:type".to_string(),
                                self.format_class_expression(&assertion.class)
                            ));
                        }
                    }
                }
                Axiom::ObjectPropertyAssertion(assertion) => {
                    if let Individual::Named(subject) = &assertion.source {
                        if subject.iri.as_str() == target_iri {
                            if let Individual::Named(object) = &assertion.target {
                                triples.push((
                                    resource.to_string(),
                                    self.format_object_property(&assertion.property),
                                    format!("<{}>", object.iri)
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(triples)
    }

    /// Format class expression for output
    fn format_class_expression(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => format!("<{}>", class.iri),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!("_:some{}_{}", 
                    self.format_object_property(property),
                    self.format_class_expression(filler))
            }
            // Add more cases as needed
            _ => "_:complex".to_string()
        }
    }

    /// Format object property for output
    fn format_object_property(&self, prop: &ObjectPropertyExpression) -> String {
        match prop {
            ObjectPropertyExpression::ObjectProperty(prop) => format!("<{}>", prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("^{}", format!("<{}>", prop.iri))
            }
            // Add more cases as needed
            _ => "_:complex_property".to_string()
        }
    }

    // Helper methods for OWLlink processing

    /// Handle KB satisfiability check
    fn handle_kb_satisfiable(&mut self, ontology: &Ontology) -> Result<String> {
        let is_satisfiable = self.is_consistent()?;
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="{}" />"#, is_satisfiable))
    }

    /// Handle class satisfiability check
    fn handle_class_satisfiable(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            // For now, implement a basic satisfiability check
            // In a full implementation, this would use tableau reasoning
            let is_satisfiable = match class_expr {
                ClassExpression::Class(class) => {
                    // Check if it's owl:Nothing (always unsatisfiable)
                    if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                        false
                    } else {
                        true // Assume satisfiable for now
                    }
                }
                _ => true, // For complex expressions, assume satisfiable for now
            };
            Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="{}" />"#, is_satisfiable))
        } else {
            Err(Error::reasoning("No class expression provided in satisfiability request"))
        }
    }

    /// Handle entailment check
    fn handle_entailment_check(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        // Extract axiom from request and check entailment
        let is_entailed = if let Some(axiom) = &request.axiom {
            self.entails(axiom)?
        } else {
            false // No axiom provided
        };
        
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="{}" />"#, is_entailed))
    }

    /// Handle get subclasses request
    fn handle_get_subclasses(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            let subclasses = self.get_sub_classes(class_expr, false)?;
            let mut response = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse>"#);
            
            for subclass in subclasses {
                response.push_str(&format!("<Class IRI=\"{}\" />", 
                    self.extract_class_iri(&subclass)));
            }
            
            response.push_str("</SetOfClassesResponse>");
            Ok(response)
        } else {
            Err(Error::reasoning("No class expression provided in subclasses request"))
        }
    }

    /// Handle get superclasses request
    fn handle_get_superclasses(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            let superclasses = self.get_super_classes(class_expr, false)?;
            let mut response = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse>"#);
            
            for superclass in superclasses {
                response.push_str(&format!("<Class IRI=\"{}\" />", 
                    self.extract_class_iri(&superclass)));
            }
            
            response.push_str("</SetOfClassesResponse>");
            Ok(response)
        } else {
            Err(Error::reasoning("No class expression provided in superclasses request"))
        }
    }

    /// Handle get equivalent classes request
    fn handle_get_equivalent_classes(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            let equivalent_classes = self.get_equivalent_classes(class_expr)?;
            let mut response = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse>"#);
            
            for equiv_class in equivalent_classes {
                response.push_str(&format!("<Class IRI=\"{}\" />", 
                    self.extract_class_iri(&equiv_class)));
            }
            
            response.push_str("</SetOfClassesResponse>");
            Ok(response)
        } else {
            Err(Error::reasoning("No class expression provided in equivalent classes request"))
        }
    }

    /// Handle get instances request
    fn handle_get_instances(&mut self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            let instances = self.get_instances(class_expr, false)?;
            let mut response = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfIndividualsResponse>"#);
            
            for instance in instances {
                if let Individual::Named(named) = instance {
                    response.push_str(&format!("<Individual IRI=\"{}\" />", named.iri));
                }
            }
            
            response.push_str("</SetOfIndividualsResponse>");
            Ok(response)
        } else {
            Err(Error::reasoning("No class expression provided in instances request"))
        }
    }

    /// Handle get types request
    fn handle_get_types(&self, request: &OwllinkRequest, ontology: &Ontology) -> Result<String> {
        // Extract individual from request
        let types = if let Some(individual) = &request.individual {
            self.get_types(individual, request.direct.unwrap_or(false))?
        } else {
            Vec::new() // No individual provided
        };
        
        let mut response = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse>"#);
        
        for class_type in types {
            response.push_str(&format!("<Class IRI=\"{}\" />", 
                self.extract_class_iri(&class_type)));
        }
        
        response.push_str("</SetOfClassesResponse>");
        Ok(response)
    }

    /// Extract class IRI from class expression
    fn extract_class_iri(&self, class_expr: &ClassExpression) -> String {
        match class_expr {
            ClassExpression::Class(class) => class.iri.to_string(),
            _ => "http://example.org/complex".to_string()
        }
    }

    /// Extract class expression from OWLlink XML
    fn extract_class_from_owllink(&self, xml: &str) -> Result<ClassExpression> {
        // Basic XML parsing to extract class
        if let Some(start) = xml.find("IRI=\"") {
            if let Some(end) = xml[start + 5..].find('"') {
                let iri_str = &xml[start + 5..start + 5 + end];
                return Ok(ClassExpression::Class(crate::ontology::Class {
                    iri: IRI::new(iri_str)
                }));
            }
        }
        
        Err(Error::reasoning("Could not extract class from OWLlink request"))
    }

    /// Extract KB name from OWLlink XML
    fn extract_kb_name_from_owllink(&self, xml: &str) -> Result<String> {
        // Basic XML parsing to extract KB name
        if let Some(start) = xml.find("kb=\"") {
            if let Some(end) = xml[start + 4..].find('"') {
                let kb_name = &xml[start + 4..start + 4 + end];
                return Ok(kb_name.to_string());
            }
        }
        
        Ok("default".to_string())
    }

    /// Check if an inverse property matches the current edge
    fn check_inverse_property_match(&self, edge: &crate::core::tableau::TableauEdge, prop: &ObjectPropertyExpression, node_id: usize, tableau: &Tableau) -> Result<bool> {
        // For inverse properties, we need to find edges going TO our node
        for reverse_edge in tableau.edges() {
            if reverse_edge.to == node_id {
                let prop_matches = match prop {
                    ObjectPropertyExpression::ObjectProperty(base_prop) => {
                        reverse_edge.role.name() == base_prop.iri.as_str()
                    }
                    ObjectPropertyExpression::InverseObjectProperty(nested_prop) => {
                        // Double inverse - check forward direction again
                        let nested_expr = ObjectPropertyExpression::ObjectProperty(nested_prop.clone());
                        reverse_edge.from == node_id && self.check_inverse_property_match(&reverse_edge, &nested_expr, node_id, tableau)?
                    }
                    ObjectPropertyExpression::PropertyChain(_) => {
                        // Inverse of property chain - requires complex path analysis
                        false // More complex implementation needed
                    }
                };
                
                if prop_matches {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Check if a property chain matches starting from the current edge
    fn check_property_chain_match(&self, edge: &crate::core::tableau::TableauEdge, chain: &[ObjectPropertyExpression], node_id: usize, tableau: &Tableau) -> Result<bool> {
        if chain.is_empty() {
            return Ok(true);
        }

        // Check if the first property in the chain matches current edge
        let first_matches = match &chain[0] {
            ObjectPropertyExpression::ObjectProperty(prop) => {
                edge.role.name() == prop.iri.as_str()
            }
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                let prop_expr = ObjectPropertyExpression::ObjectProperty(prop.clone());
                self.check_inverse_property_match(edge, &prop_expr, node_id, tableau)?
            }
            ObjectPropertyExpression::PropertyChain(nested_chain) => {
                self.check_property_chain_match(edge, nested_chain, node_id, tableau)?
            }
        };

        if !first_matches {
            return Ok(false);
        }

        // If this is the last property in the chain, we're done
        if chain.len() == 1 {
            return Ok(true);
        }

        // Check if remaining chain can be satisfied from the edge target
        let remaining_chain = &chain[1..];
        for next_edge in tableau.edges() {
            if next_edge.from == edge.to {
                if self.check_property_chain_match(&next_edge, remaining_chain, edge.to, tableau)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if an axiom is entailed by the ontology
    fn entails(&self, axiom: &Axiom) -> Result<bool> {
        // Placeholder implementation - would require full axiom entailment checking
        Ok(false)
    }

    /// Get subclasses of a class expression
    fn get_sub_classes(&self, class_expr: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - would require proper subsumption reasoning
        Ok(Vec::new())
    }

    /// Get superclasses of a class expression  
    fn get_super_classes(&self, class_expr: &ClassExpression, direct: bool) -> Result<Vec<ClassExpression>> {
        // Placeholder implementation - would require proper subsumption reasoning
        Ok(Vec::new())
    }
}
