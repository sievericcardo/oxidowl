//! Result types for reasoning operations
//!
//! This module contains all the result structures returned by various reasoning operations,
//! including classification results, realisation results, and property classification results.

use crate::{
    Result,
    ontology::{ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression},
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
};

/// Results from reasoning operations
#[derive(Debug, Clone)]
pub enum ReasoningResult {
    Boolean(bool),
    Classes(HashSet<ClassExpression>),
    Individuals(HashSet<Individual>),
    ClassificationResult(ClassificationResult),
    RealisationResult(RealisationResult),
}

/// Classification result containing class hierarchy
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>,
    pub ontology_iri: Option<String>,
    /// Object properties present in the ontology (local name, super-property name)
    pub object_properties: Vec<String>,
    /// Data properties present in the ontology (local name)
    pub data_properties: Vec<String>,
}

impl ClassificationResult {
    #[must_use]
    pub fn new(hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>) -> Self {
        Self {
            hierarchy,
            ontology_iri: None,
            object_properties: Vec::new(),
            data_properties: Vec::new(),
        }
    }

    #[must_use]
    pub fn new_with_iri(
        hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>,
        ontology_iri: Option<String>,
    ) -> Self {
        Self {
            hierarchy,
            ontology_iri,
            object_properties: Vec::new(),
            data_properties: Vec::new(),
        }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
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
        let mut file = File::create(path)?;

        // Generate HermiT-style output with proper functional syntax
        self.write_hermit_style_hierarchy(&mut file)?;

        Ok(())
    }

    /// Write hierarchy in HermiT-style functional syntax format
    pub fn write_hermit_style_hierarchy<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Start with ontology declaration matching HermiT output
        let ontology_iri = self
            .ontology_iri
            .as_deref()
            .unwrap_or("http://example.org/ontology");
        // Build the base IRI for the Prefix declaration (must end with # or /)
        let prefix_base = if ontology_iri.ends_with('#') || ontology_iri.ends_with('/') {
            ontology_iri.to_string()
        } else {
            format!("{ontology_iri}#")
        };
        writeln!(writer, "Prefix(:=<{prefix_base}>)")?;
        writeln!(writer)?;
        writeln!(writer, "Ontology(<{ontology_iri}>")?;
        writeln!(writer)?;

        // Build a proper class hierarchy based on subsumption relationships
        let class_hierarchy = self.build_class_tree()?;

        // Write the class hierarchy in HermiT format
        self.write_class_hierarchy(writer, &class_hierarchy, &prefix_base)?;

        // Write object properties if available
        self.write_object_properties(writer, &prefix_base)?;

        // Write data properties if available
        self.write_data_properties(writer, &prefix_base)?;

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
        for class in direct_hierarchy.keys() {
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

    /// Compute direct subsumption relationships (remove transitive relationships).
    ///
    /// Given that `self.hierarchy` stores the full transitive closure, an edge A→C is
    /// *direct* (belongs to the Hasse diagram) iff C does NOT appear in the ancestor
    /// set of any other superclass B of A.  Collecting those indirect ancestors in one
    /// pass makes this O(n × k) instead of the naïve O(n × k²) triple-loop.
    fn compute_direct_hierarchy(
        &self,
    ) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>> {
        let mut direct_hierarchy = HashMap::new();

        for (subclass, all_superclasses) in &self.hierarchy {
            // Collect every ancestor reachable from `subclass` in ≥ 2 steps.
            // If the stored hierarchy is the full transitive closure, C is non-direct
            // for A whenever C ∈ ancestors(B) for some B ∈ superclasses(A), B ≠ A.
            let mut indirect: HashSet<&ClassExpression> = HashSet::new();
            for intermediate in all_superclasses {
                if intermediate != subclass
                    && let Some(intermediate_supers) = self.hierarchy.get(intermediate) {
                        indirect.extend(intermediate_supers.iter());
                    }
            }

            // Direct superclasses = all_superclasses minus those reachable indirectly.
            let direct_superclasses: HashSet<ClassExpression> = all_superclasses
                .iter()
                .filter(|sc| !indirect.contains(sc))
                .cloned()
                .collect();

            direct_hierarchy.insert(subclass.clone(), direct_superclasses);
        }

        Ok(direct_hierarchy)
    }

    /// Build children for a specific class IRI using direct hierarchy.
    ///
    /// Iterative post-order DFS replaces the former recursive implementation to
    /// avoid stack overflows on deep class hierarchies (e.g. `ore_ont_9881.owl`).
    fn build_children_for_iri_direct(
        &self,
        parent_iri: &str,
        direct_hierarchy: &HashMap<ClassExpression, HashSet<ClassExpression>>,
    ) -> Result<Vec<ClassNode>> {
        // Step 1: build a parent-IRI → [(child_iri, child_name)] index in one pass.
        let mut children_index: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (subclass, direct_superclasses) in direct_hierarchy {
            let subclass_iri = self.extract_class_iri(subclass);
            let child_name = self.extract_class_name(subclass);
            for superclass in direct_superclasses {
                let super_iri = self.extract_class_iri(superclass);
                children_index
                    .entry(super_iri)
                    .or_default()
                    .push((subclass_iri.clone(), child_name.clone()));
            }
        }
        // Pre-sort each entry so that the output order matches the original
        // (children sorted by name at each level).
        for v in children_index.values_mut() {
            v.sort_by(|a, b| a.1.cmp(&b.1));
        }

        // Step 2: iterative DFS rooted at parent_iri.
        // Stack entry: (iri, name, next_child_index, built_children_so_far)
        let root_children = match children_index.get(parent_iri) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => return Ok(vec![]),
        };

        let mut output: Vec<ClassNode> = Vec::new();
        for (start_iri, start_name) in root_children {
            // Each iteration of this outer loop builds one complete subtree.
            let mut stack: Vec<(String, String, usize, Vec<ClassNode>)> =
                vec![(start_iri, start_name, 0, Vec::new())];

            loop {
                let top = stack.last_mut().unwrap();
                let num_children = children_index.get(&top.0).map_or(0, std::vec::Vec::len);

                if top.2 < num_children {
                    // Push the next unvisited child onto the stack.
                    let (child_iri, child_name) = children_index[&top.0][top.2].clone();
                    top.2 += 1;
                    stack.push((child_iri, child_name, 0, Vec::new()));
                } else {
                    // All children of this node have been processed: pop and assemble.
                    let (iri, name, _, built) = stack.pop().unwrap();
                    // Children are already in pre-sorted order from the index.
                    let node = ClassNode { name, iri, children: built };
                    if let Some(parent_frame) = stack.last_mut() {
                        parent_frame.3.push(node);
                    } else {
                        output.push(node);
                        break;
                    }
                }
            }
        }

        output.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(output)
    }

    /// Return the correct OWL functional-syntax reference for an IRI given the declared prefix base.
    /// Uses `:localname` short form when the IRI starts with `prefix_base`, otherwise `<fullIRI>`.
    fn iri_ref(iri: &str, prefix_base: &str) -> String {
        if let Some(local) = iri.strip_prefix(prefix_base)
            && !local.is_empty()
        {
            return format!(":{local}");
        }
        format!("<{iri}>")
    }

    /// Write class hierarchy in `HermiT` format
    fn write_class_hierarchy<W: Write>(
        &self,
        writer: &mut W,
        root_classes: &[ClassNode],
        prefix_base: &str,
    ) -> Result<()> {
        for class in root_classes {
            self.write_class_node(writer, class, "owl:Thing", 1, prefix_base)?;
        }
        Ok(())
    }

    /// Write a single class node with proper indentation
    fn write_class_node<W: Write>(
        &self,
        writer: &mut W,
        node: &ClassNode,
        parent_ref: &str,
        level: usize,
        prefix_base: &str,
    ) -> Result<()> {
        let indent = "  ".repeat(level);
        let class_ref = Self::iri_ref(&node.iri, prefix_base);

        // Write SubClassOf and Declaration for this class with correct parent
        writeln!(
            writer,
            "{indent}SubClassOf( {class_ref} {parent_ref} ) Declaration( Class( {class_ref} ) )"
        )?;

        // Write children with increased indentation, using this node as parent
        for child in &node.children {
            self.write_class_node(writer, child, &class_ref, level + 1, prefix_base)?;
        }

        Ok(())
    }

    /// Write object properties in `HermiT` format
    fn write_object_properties<W: Write>(&self, writer: &mut W, prefix_base: &str) -> Result<()> {
        if self.object_properties.is_empty() {
            return Ok(());
        }
        writeln!(writer)?;
        let mut sorted = self.object_properties.clone();
        sorted.sort();
        for prop in &sorted {
            let prop_ref = Self::iri_ref(prop, prefix_base);
            writeln!(
                writer,
                "  SubObjectPropertyOf( {prop_ref} owl:topObjectProperty ) Declaration( ObjectProperty( {prop_ref} ) )"
            )?;
        }
        Ok(())
    }

    /// Write data properties in `HermiT` format
    fn write_data_properties<W: Write>(&self, writer: &mut W, prefix_base: &str) -> Result<()> {
        if self.data_properties.is_empty() {
            return Ok(());
        }
        writeln!(writer)?;
        let mut sorted = self.data_properties.clone();
        sorted.sort();
        for prop in &sorted {
            let prop_ref = Self::iri_ref(prop, prefix_base);
            writeln!(
                writer,
                "  SubDataPropertyOf( {prop_ref} owl:topDataProperty ) Declaration( DataProperty( {prop_ref} ) )"
            )?;
        }
        Ok(())
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
                } else if let Some(name) = iri_str.split('/').next_back() {
                    name.to_string()
                } else {
                    iri_str
                }
            }
            _ => format!("{class:?}"),
        }
    }
}

/// Helper structure for building class trees in `HermiT` format
#[derive(Debug, Clone)]
struct ClassNode {
    name: String,
    #[allow(dead_code)]
    #[allow(dead_code)]
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

/// Realisation result containing individual types
#[derive(Debug, Clone)]
pub struct RealisationResult {
    pub types: HashMap<Individual, HashSet<ClassExpression>>,
}

impl RealisationResult {
    #[must_use]
    pub fn new(types: HashMap<Individual, HashSet<ClassExpression>>) -> Self {
        Self { types }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
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
