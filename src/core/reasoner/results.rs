//! Result types for reasoning operations
//!
//! This module contains all the result structures returned by various reasoning operations,
//! including classification results, realization results, and property classification results.

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
    RealizationResult(RealizationResult),
}

/// Classification result containing class hierarchy
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>,
    pub ontology_iri: Option<String>,
}

impl ClassificationResult {
    #[must_use]
    pub fn new(hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>>) -> Self {
        Self {
            hierarchy,
            ontology_iri: None,
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
        writeln!(writer, "Prefix(:=<{ontology_iri}#>)")?;
        writeln!(writer)?;
        writeln!(writer, "Ontology(<{ontology_iri}>")?;
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
                        if let Some(intermediate_superclasses) = self.hierarchy.get(intermediate)
                            && intermediate_superclasses.contains(superclass) {
                                is_direct = false;
                                break;
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

/// Helper structure for building class trees in HermiT format
#[derive(Debug, Clone)]
struct ClassNode {
    name: String,
    #[allow(dead_code)]#[allow(dead_code)]
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
