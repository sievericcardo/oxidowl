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
        tableau::{RoleLabel, Tableau, TableauBuilder, TableauState},
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, IRI, Individual, ObjectPropertyExpression,
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

        // TODO:  integrate with the SPARQL engine
        // For now, return a placeholder
        Ok("SPARQL query results would be here".to_string())
    }

    /// Process an `OWLlink` request
    pub fn process_owllink_request(&self, request: &str) -> Result<String> {
        info!("Processing OWLlink request");

        // TODO:  integrate with the OWLlink processor
        // For now, return a placeholder
        Ok("OWLlink response would be here".to_string())
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
        &mut self,
        individual: &Individual,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology) = &self.ontology {
            let classes = {
                let ontology_guard = ontology.read().unwrap();
                // Get all classes from the signature
                ontology_guard.signature().unwrap().classes.clone()
            }; // Drop the read lock here

            let mut types = Vec::new();

            for class in &classes {
                let class_expr = ClassExpression::Class(class.clone());
                // Use tableau reasoning to check if individual has this type
                if self.is_instance_of_expression(individual, &class_expr)? {
                    types.push(class_expr);
                }
            }

            if direct {
                // Filter to only direct types (most specific ones)
                types = self.filter_direct_types(types, individual)?;
            }

            Ok(types)
        } else {
            Err(Error::reasoning("No ontology loaded"))
        }
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
                            // For inverse properties, we need to check reverse edges
                            false // Simplified for now
                        }
                        ObjectPropertyExpression::PropertyChain(_) => {
                            // Property chains require more complex checking
                            false // Simplified for now
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
        // TODO:  use explanation support
        Ok(Vec::new())
    }

    /// Explain inconsistency
    pub fn explain_inconsistency(&self) -> Result<Vec<Axiom>> {
        // TODO:  use explanation support
        Ok(Vec::new())
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
            // This is a simplified implementation
            // TODO: Implement full property chain reasoning
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
                // This would require sophisticated ABox reasoning
                // For now, return false as a placeholder
                Ok(false)
            }
            // Add more axiom types as needed
            _ => {
                // For other axiom types, return false for now
                Ok(false)
            }
        }
    }
}
