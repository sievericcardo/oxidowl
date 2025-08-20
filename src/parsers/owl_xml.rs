//! OWL XML Parser
//!
//! This module implements parsing of OWL 2 ontologies from OWL XML format.

use crate::{
    Error, Result,
    ontology::{
        Axiom, Class, ClassExpression, DeclarationAxiom, Entity, IRI, Individual, NamedIndividual,
        ObjectProperty, ObjectPropertyExpression, Ontology, axioms::DisjointUnionAxiom,
        DataProperty, DataPropertyExpression, DataRange,
    },
};
use std::{
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
};

/// Resolve a potentially relative IRI against a base IRI
fn resolve_iri(iri: &str, base_iri: Option<&url::Url>) -> Result<url::Url> {
    // If it's already an absolute URL, return it as-is
    if let Ok(absolute_url) = url::Url::parse(iri) {
        return Ok(absolute_url);
    }

    // If we have a base IRI, resolve the relative IRI against it
    if let Some(base) = base_iri {
        return base.join(iri).map_err(|e| {
            Error::ontology_parsing(format!(
                "Failed to resolve relative IRI '{iri}' against base '{base}': {e}"
            ))
        });
    }

    // No base IRI provided for relative IRI
    Err(Error::ontology_parsing(format!(
        "Relative IRI '{iri}' provided without base IRI"
    )))
}

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Configuration for the OWL/XML parser
#[derive(Debug, Clone)]
pub struct OwlXmlParserConfig {
    /// Whether to validate XML schema (default: true)
    pub validate_schema: bool,

    /// Whether to validate OWL 2 semantics (default: true)
    pub validate_owl_semantics: bool,

    /// Whether to allow OWL 1 constructs (default: true)
    pub allow_owl1_constructs: bool,

    /// Whether to preserve annotations (default: true)
    pub preserve_annotations: bool,

    /// Maximum nesting depth for class expressions (default: 50)
    pub max_nesting_depth: usize,

    /// Whether to use strict OWL/XML compliance (default: false)
    pub strict_mode: bool,

    /// Whether to ignore unknown elements (default: false)
    pub ignore_unknown_elements: bool,
}

impl Default for OwlXmlParserConfig {
    fn default() -> Self {
        Self {
            validate_schema: true,
            validate_owl_semantics: true,
            allow_owl1_constructs: true,
            preserve_annotations: true,
            max_nesting_depth: 50,
            strict_mode: false,
            ignore_unknown_elements: false,
        }
    }
}

/// OWL/XML Parser
#[derive(Debug, Clone)]
pub struct OwlXmlParser {
    config: OwlXmlParserConfig,
}

impl OwlXmlParser {
    /// Create a new OWL/XML parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: OwlXmlParserConfig::default(),
        }
    }

    /// Create a new OWL/XML parser with custom configuration
    #[must_use]
    pub fn with_config(config: OwlXmlParserConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &OwlXmlParserConfig {
        &self.config
    }

    /// Set a new configuration
    pub fn set_config(&mut self, config: OwlXmlParserConfig) {
        self.config = config;
    }
}

impl Default for OwlXmlParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse OWL XML from string content
pub fn parse(content: &str) -> Result<Ontology> {
    // Basic OWL XML parser implementation
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| Error::io(format!("Failed to parse XML: {e}")))?;

    let mut ontology = Ontology::new();

    // Find the root Ontology element
    let root = doc.root_element();
    if root.tag_name().name() != "Ontology" {
        return Err(Error::io("Root element must be Ontology".to_string()));
    }

    // Extract ontology IRI if present and use as base for resolving relative IRIs
    let base_iri = if let Some(iri) = root.attribute("ontologyIRI") {
        if let Ok(url) = url::Url::parse(iri) {
            ontology.iri = Some(url.clone().into());
            Some(url)
        } else {
            None
        }
    } else {
        None
    };

    // Parse declarations and axioms
    for child in root.children().filter(roxmltree::Node::is_element) {
        match child.tag_name().name() {
            "Declaration" => {
                if let Ok(axiom) = parse_declaration(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "SubClassOf" => {
                if let Ok(axiom) = parse_subclass_of(&child, base_iri.as_ref()) {
                    ontology.add_axiom(axiom);
                }
            }
            "DisjointUnion" => {
                if let Ok(axiom) = parse_disjoint_union(&child, base_iri.as_ref()) {
                    ontology.add_axiom(axiom);
                }
            }
            "EquivalentClasses" => {
                if let Ok(axiom) = parse_equivalent_classes(&child, base_iri.as_ref()) {
                    ontology.add_axiom(axiom);
                }
            }
            "ClassAssertion" => {
                if let Ok(axiom) = parse_class_assertion(&child, base_iri.as_ref()) {
                    ontology.add_axiom(axiom);
                }
            }
            "ObjectPropertyAssertion" => {
                if let Ok(axiom) = parse_object_property_assertion(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "SubObjectPropertyOf" => {
                if let Ok(axiom) = parse_sub_object_property_of(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "FunctionalObjectProperty" => {
                if let Ok(axiom) = parse_functional_object_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "FunctionalDataProperty" => {
                println!("DEBUG: Found FunctionalDataProperty in XML");
                if let Ok(axiom) = parse_functional_data_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "ObjectPropertyDomain" => {
                println!("DEBUG: Found ObjectPropertyDomain in XML");
                if let Ok(axiom) = parse_object_property_domain(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "ObjectPropertyRange" => {
                println!("DEBUG: Found ObjectPropertyRange in XML");
                if let Ok(axiom) = parse_object_property_range(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "DataPropertyDomain" => {
                println!("DEBUG: Found DataPropertyDomain in XML");
                if let Ok(axiom) = parse_data_property_domain(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "DataPropertyRange" => {
                println!("DEBUG: Found DataPropertyRange in XML");
                if let Ok(axiom) = parse_data_property_range(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "TransitiveObjectProperty" => {
                println!("DEBUG: Found TransitiveObjectProperty in XML");
                match parse_transitive_object_property(&child) {
                    Ok(axiom) => {
                        println!("DEBUG: Successfully parsed TransitiveObjectProperty axiom");
                        ontology.add_axiom(axiom);
                        println!("DEBUG: Added TransitiveObjectProperty axiom to ontology");
                    }
                    Err(e) => {
                        println!("DEBUG: Failed to parse TransitiveObjectProperty: {:?}", e);
                    }
                }
            }
            "SymmetricObjectProperty" => {
                println!("DEBUG: Found SymmetricObjectProperty in XML");
                if let Ok(axiom) = parse_symmetric_object_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "ReflexiveObjectProperty" => {
                println!("DEBUG: Found ReflexiveObjectProperty in XML");
                if let Ok(axiom) = parse_reflexive_object_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "IrreflexiveObjectProperty" => {
                println!("DEBUG: Found IrreflexiveObjectProperty in XML");
                if let Ok(axiom) = parse_irreflexive_object_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "AsymmetricObjectProperty" => {
                println!("DEBUG: Found AsymmetricObjectProperty in XML");
                if let Ok(axiom) = parse_asymmetric_object_property(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "InverseObjectProperties" => {
                println!("DEBUG: Found InverseObjectProperties in XML");
                if let Ok(axiom) = parse_inverse_object_properties(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "SameIndividual" => {
                println!("DEBUG: Found SameIndividual in XML");
                if let Ok(axiom) = parse_same_individual(&child) {
                    println!("DEBUG: Successfully parsed SameIndividual axiom");
                    ontology.add_axiom(axiom);
                    println!("DEBUG: Added SameIndividual axiom to ontology");
                }
            }
            "DifferentIndividuals" => {
                println!("DEBUG: Found DifferentIndividuals in XML");
                if let Ok(axiom) = parse_different_individuals(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "HasKey" => {
                if let Ok(axiom) = parse_has_key(&child, base_iri.as_ref()) {
                    ontology.add_axiom(axiom);
                }
            }
            _ => {
                // Skip unknown elements or log warning
            }
        }
    }

    Ok(ontology)
}

/// Parse a Declaration element
fn parse_declaration(element: &roxmltree::Node) -> Result<Axiom> {
    // Find the entity being declared
    for child in element.children().filter(roxmltree::Node::is_element) {
        match child.tag_name().name() {
            "Class" => {
                if let Some(iri) = child.attribute("IRI") {
                    return Ok(Axiom::Declaration(DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::Class(IRI::new(iri)),
                    }));
                }
            }
            "ObjectProperty" => {
                if let Some(iri) = child.attribute("IRI") {
                    return Ok(Axiom::Declaration(DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::ObjectProperty(IRI::new(iri)),
                    }));
                }
            }
            "DataProperty" => {
                if let Some(iri) = child.attribute("IRI") {
                    return Ok(Axiom::Declaration(DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::DataProperty(IRI::new(iri)),
                    }));
                }
            }
            "NamedIndividual" => {
                if let Some(iri) = child.attribute("IRI") {
                    return Ok(Axiom::Declaration(DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::NamedIndividual(IRI::new(iri)),
                    }));
                }
            }
            _ => {}
        }
    }

    Err(Error::io("Invalid Declaration element".to_string()))
}

/// Parse a `SubClassOf` element
fn parse_subclass_of(element: &roxmltree::Node, base_iri: Option<&url::Url>) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "SubClassOf must have exactly 2 children".to_string(),
        ));
    }

    let subclass = parse_class_expression(&children[0], base_iri)?;
    let superclass = parse_class_expression(&children[1], base_iri)?;

    Ok(Axiom::SubClassOf(crate::ontology::SubClassOfAxiom {
        id: generate_axiom_id(),
        subclass,
        superclass,
        annotations: Vec::new(),
    }))
}

/// Parse an `EquivalentClasses` element
fn parse_equivalent_classes(
    element: &roxmltree::Node,
    base_iri: Option<&url::Url>,
) -> Result<Axiom> {
    let mut class_expressions = Vec::new();

    for child in element.children().filter(roxmltree::Node::is_element) {
        let expr = parse_class_expression(&child, base_iri)?;
        class_expressions.push(expr);
    }

    if class_expressions.len() < 2 {
        return Err(Error::io(
            "EquivalentClasses must have at least 2 classes".to_string(),
        ));
    }

    Ok(Axiom::EquivalentClasses(
        crate::ontology::EquivalentClassesAxiom {
            id: generate_axiom_id(),
            classes: class_expressions,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `DisjointUnion` element
fn parse_disjoint_union(element: &roxmltree::Node, base_iri: Option<&url::Url>) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.len() < 2 {
        return Err(Error::io(
            "DisjointUnion must have at least 2 children (union class + disjoint classes)"
                .to_string(),
        ));
    }

    // First child is the union class
    let union_class = parse_class_expression(&children[0], base_iri)?;

    // Remaining children are the disjoint classes
    let mut disjoint_classes = Vec::new();
    for child in &children[1..] {
        let expr = parse_class_expression(child, base_iri)?;
        disjoint_classes.push(expr);
    }

    Ok(Axiom::DisjointUnion(DisjointUnionAxiom {
        id: generate_axiom_id(),
        class: union_class,
        disjoint_classes,
        annotations: Vec::new(),
    }))
}

/// Parse a `ClassAssertion` element
fn parse_class_assertion(element: &roxmltree::Node, base_iri: Option<&url::Url>) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "ClassAssertion must have exactly 2 children".to_string(),
        ));
    }

    let class_expression = parse_class_expression(&children[0], base_iri)?;
    let individual = parse_individual(&children[1])?;

    Ok(Axiom::ClassAssertion(
        crate::ontology::ClassAssertionAxiom {
            id: generate_axiom_id(),
            class: class_expression,
            individual,
            annotations: Vec::new(),
        },
    ))
}

/// Parse an `ObjectPropertyAssertion` element
fn parse_object_property_assertion(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 3 {
        return Err(Error::io(
            "ObjectPropertyAssertion must have exactly 3 children".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;
    let source = parse_individual(&children[1])?;
    let target = parse_individual(&children[2])?;

    Ok(Axiom::ObjectPropertyAssertion(
        crate::ontology::ObjectPropertyAssertionAxiom {
            id: generate_axiom_id(),
            property,
            source,
            target,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `SubObjectPropertyOf` element
fn parse_sub_object_property_of(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "SubObjectPropertyOf must have exactly 2 children".to_string(),
        ));
    }

    let sub_property = parse_object_property_expression(&children[0])?;
    let super_property = parse_object_property_expression(&children[1])?;

    Ok(Axiom::SubObjectPropertyOf(
        crate::ontology::SubObjectPropertyOfAxiom {
            id: generate_axiom_id(),
            sub_property,
            super_property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `FunctionalObjectProperty` element
fn parse_functional_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "FunctionalObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::FunctionalObjectProperty(
        crate::ontology::FunctionalObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `HasKey` element
fn parse_has_key(element: &roxmltree::Node, _base_iri: Option<&url::Url>) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.is_empty() {
        return Err(Error::io("HasKey must have at least a class".to_string()));
    }

    // First child should be the class
    let class = parse_class_expression(&children[0], _base_iri)?;

    let mut object_properties = Vec::new();
    let mut data_properties = Vec::new();

    // Remaining children are properties
    for child in &children[1..] {
        match child.tag_name().name() {
            "ObjectProperty" => {
                if let Some(iri) = child.attribute("IRI") {
                    if let Ok(url) = url::Url::parse(iri) {
                        object_properties.push(
                            crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                crate::ontology::ObjectProperty { iri: url },
                            ),
                        );
                    }
                }
            }
            "DataProperty" => {
                if let Some(iri) = child.attribute("IRI") {
                    data_properties.push(crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty { iri: IRI::new(iri) },
                    ));
                }
            }
            _ => {
                // Skip unknown elements
            }
        }
    }

    Ok(Axiom::HasKey(crate::ontology::HasKeyAxiom {
        id: generate_axiom_id(),
        class,
        object_properties,
        data_properties,
        annotations: Vec::new(),
    }))
}

/// Parse a `TransitiveObjectProperty` element
fn parse_transitive_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "TransitiveObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::TransitiveObjectProperty(
        crate::ontology::TransitiveObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `SymmetricObjectProperty` element
fn parse_symmetric_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "SymmetricObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::SymmetricObjectProperty(
        crate::ontology::SymmetricObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `ReflexiveObjectProperty` element
fn parse_reflexive_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "ReflexiveObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::ReflexiveObjectProperty(
        crate::ontology::ReflexiveObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `IrreflexiveObjectProperty` element
fn parse_irreflexive_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "IrreflexiveObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::IrreflexiveObjectProperty(
        crate::ontology::IrreflexiveObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `AsymmetricObjectProperty` element
fn parse_asymmetric_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "AsymmetricObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::AsymmetricObjectProperty(
        crate::ontology::AsymmetricObjectPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `InverseObjectProperties` element
fn parse_inverse_object_properties(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "InverseObjectProperties must have exactly 2 children".to_string(),
        ));
    }

    let property1 = parse_object_property_expression(&children[0])?;
    let property2 = parse_object_property_expression(&children[1])?;

    Ok(Axiom::InverseObjectProperties(
        crate::ontology::InverseObjectPropertiesAxiom {
            id: generate_axiom_id(),
            property1,
            property2,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `SameIndividual` element
fn parse_same_individual(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() < 2 {
        return Err(Error::io(
            "SameIndividual must have at least 2 children".to_string(),
        ));
    }

    let mut individuals = Vec::new();
    for child in children {
        let individual = parse_individual(&child)?;
        individuals.push(individual);
    }

    Ok(Axiom::SameIndividual(
        crate::ontology::SameIndividualAxiom {
            id: generate_axiom_id(),
            individuals,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `DifferentIndividuals` element
fn parse_different_individuals(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() < 2 {
        return Err(Error::io(
            "DifferentIndividuals must have at least 2 children".to_string(),
        ));
    }

    let mut individuals = Vec::new();
    for child in children {
        let individual = parse_individual(&child)?;
        individuals.push(individual);
    }

    Ok(Axiom::DifferentIndividuals(
        crate::ontology::DifferentIndividualsAxiom {
            id: generate_axiom_id(),
            individuals,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a class expression
fn parse_class_expression(
    element: &roxmltree::Node,
    base_iri: Option<&url::Url>,
) -> Result<ClassExpression> {
    match element.tag_name().name() {
        "Class" => {
            if let Some(iri) = element.attribute("IRI") {
                let resolved_iri = resolve_iri(iri, base_iri)?;
                Ok(ClassExpression::Class(Class {
                    iri: resolved_iri.into(),
                }))
            } else {
                Err(Error::io("Class element missing IRI attribute".to_string()))
            }
        }
        "ObjectIntersectionOf" => {
            let mut operands = Vec::new();
            for child in element.children().filter(roxmltree::Node::is_element) {
                operands.push(parse_class_expression(&child, base_iri)?);
            }
            Ok(ClassExpression::ObjectIntersectionOf(operands))
        }
        "ObjectUnionOf" => {
            let mut operands = Vec::new();
            for child in element.children().filter(roxmltree::Node::is_element) {
                operands.push(parse_class_expression(&child, base_iri)?);
            }
            Ok(ClassExpression::ObjectUnionOf(operands))
        }
        "ObjectComplementOf" => {
            let children: Vec<_> = element
                .children()
                .filter(roxmltree::Node::is_element)
                .collect();
            if children.len() != 1 {
                return Err(Error::io(
                    "ObjectComplementOf must have exactly 1 child".to_string(),
                ));
            }
            let operand = Box::new(parse_class_expression(&children[0], base_iri)?);
            Ok(ClassExpression::ObjectComplementOf(operand))
        }
        "ObjectSomeValuesFrom" => {
            let children: Vec<_> = element
                .children()
                .filter(roxmltree::Node::is_element)
                .collect();
            if children.len() != 2 {
                return Err(Error::io(
                    "ObjectSomeValuesFrom must have exactly 2 children".to_string(),
                ));
            }
            let property = parse_object_property_expression(&children[0])?;
            let filler = Box::new(parse_class_expression(&children[1], base_iri)?);
            Ok(ClassExpression::ObjectSomeValuesFrom { property, filler })
        }
        "ObjectAllValuesFrom" => {
            let children: Vec<_> = element
                .children()
                .filter(roxmltree::Node::is_element)
                .collect();
            if children.len() != 2 {
                return Err(Error::io(
                    "ObjectAllValuesFrom must have exactly 2 children".to_string(),
                ));
            }
            let property = parse_object_property_expression(&children[0])?;
            let filler = Box::new(parse_class_expression(&children[1], base_iri)?);
            Ok(ClassExpression::ObjectAllValuesFrom { property, filler })
        }
        "ObjectOneOf" => {
            let mut individuals = Vec::new();
            for child in element.children().filter(roxmltree::Node::is_element) {
                individuals.push(parse_individual(&child)?);
            }
            Ok(ClassExpression::ObjectOneOf(individuals))
        }
        _ => Err(Error::io(format!(
            "Unsupported class expression: {}",
            element.tag_name().name()
        ))),
    }
}

/// Parse an object property expression
fn parse_object_property_expression(element: &roxmltree::Node) -> Result<ObjectPropertyExpression> {
    match element.tag_name().name() {
        "ObjectProperty" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                    iri: IRI::new(iri).to_url()?,
                }))
            } else {
                Err(Error::io(
                    "ObjectProperty element missing IRI attribute".to_string(),
                ))
            }
        }
        "ObjectInverseOf" => {
            let children: Vec<_> = element
                .children()
                .filter(roxmltree::Node::is_element)
                .collect();
            if children.len() != 1 {
                return Err(Error::io(
                    "ObjectInverseOf must have exactly 1 child".to_string(),
                ));
            }
            // Parse the child as ObjectProperty, not ObjectPropertyExpression
            if let Some(iri) = children[0].attribute("IRI") {
                let property = ObjectProperty {
                    iri: IRI::new(iri).to_url()?,
                };
                Ok(ObjectPropertyExpression::InverseObjectProperty(property))
            } else {
                Err(Error::io(
                    "ObjectProperty element missing IRI attribute".to_string(),
                ))
            }
        }
        _ => Err(Error::io(format!(
            "Unsupported object property expression: {}",
            element.tag_name().name()
        ))),
    }
}

/// Parse an individual
fn parse_individual(element: &roxmltree::Node) -> Result<Individual> {
    match element.tag_name().name() {
        "NamedIndividual" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(Individual::Named(NamedIndividual {
                    iri: IRI::new(iri).to_url()?.into(),
                }))
            } else {
                Err(Error::io(
                    "NamedIndividual element missing IRI attribute".to_string(),
                ))
            }
        }
        _ => Err(Error::io(format!(
            "Unsupported individual type: {}",
            element.tag_name().name()
        ))),
    }
}

/// Parse OWL XML from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path).map_err(|e| Error::io(format!("Failed to open file: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse(&content)
}

/// Save ontology to OWL XML file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let mut file =
        File::create(path).map_err(|e| Error::io(format!("Failed to create file: {e}")))?;

    // TODO: Implement a better serialization to OWL XML
    writeln!(
        file,
        "<Ontology ontologyIRI=\"{}\">",
        ontology
            .iri
            .as_ref()
            .map_or("http://example.org/ontology", |iri| iri.as_str())
    )?;
    for axiom in ontology.axioms() {
        match axiom {
            Axiom::Declaration(decl) => {
                writeln!(
                    file,
                    "  <Declaration><{} IRI=\"{}\"/></Declaration>",
                    decl.entity.entity_type(),
                    decl.entity.iri()
                )?;
            }
            Axiom::SubClassOf(axiom) => {
                if let (Some(subclass_iri), Some(superclass_iri)) =
                    (axiom.subclass.iri(), axiom.superclass.iri())
                {
                    writeln!(
                        file,
                        "  <SubClassOf><Class IRI=\"{subclass_iri}\"/><Class IRI=\"{superclass_iri}\"/></SubClassOf>"
                    )?;
                }
            }
            Axiom::EquivalentClasses(axiom) => {
                writeln!(file, "  <EquivalentClasses>",)?;
                for class in &axiom.classes {
                    if let Some(class_iri) = class.iri() {
                        writeln!(file, "    <Class IRI=\"{class_iri}\"/>")?;
                    }
                }
                writeln!(file, "  </EquivalentClasses>")?;
            }
            Axiom::DisjointUnion(axiom) => {
                writeln!(file, "  <DisjointUnion>")?;
                if let Some(union_class_iri) = axiom.class.iri() {
                    writeln!(file, "    <Class IRI=\"{union_class_iri}\"/>")?;
                }
                for disjoint_class in &axiom.disjoint_classes {
                    if let Some(class_iri) = disjoint_class.iri() {
                        writeln!(file, "    <Class IRI=\"{class_iri}\"/>")?;
                    }
                }
                writeln!(file, "  </DisjointUnion>")?;
            }
            Axiom::ClassAssertion(axiom) => {
                if let (Some(class_iri), Some(individual_iri)) =
                    (axiom.class.iri(), axiom.individual.iri())
                {
                    writeln!(
                        file,
                        "  <ClassAssertion><Class IRI=\"{class_iri}\"/><NamedIndividual IRI=\"{individual_iri}\"/></ClassAssertion>"
                    )?;
                }
            }
            Axiom::ObjectPropertyAssertion(axiom) => {
                if let (Some(source_iri), Some(target_iri)) =
                    (axiom.source.iri(), axiom.target.iri())
                {
                    if let Some(property_iri) = axiom.property.iri() {
                        writeln!(
                            file,
                            "  <ObjectPropertyAssertion><ObjectProperty IRI=\"{property_iri}\"/><NamedIndividual IRI=\"{source_iri}\"/><NamedIndividual IRI=\"{target_iri}\"/></ObjectPropertyAssertion>"
                        )?;
                    }
                }
            }
            Axiom::SubObjectPropertyOf(axiom) => {
                if let (Some(sub_iri), Some(super_iri)) =
                    (axiom.sub_property.iri(), axiom.super_property.iri())
                {
                    writeln!(
                        file,
                        "  <SubObjectPropertyOf><ObjectProperty IRI=\"{sub_iri}\"/><ObjectProperty IRI=\"{super_iri}\"/></SubObjectPropertyOf>"
                    )?;
                }
            }
            Axiom::FunctionalObjectProperty(axiom) => {
                if let Some(property_iri) = axiom.property.iri() {
                    writeln!(
                        file,
                        "  <FunctionalObjectProperty><ObjectProperty IRI=\"{property_iri}\"/></FunctionalObjectProperty>"
                    )?;
                }
            }
            _ => {
                // TODO: Implement serialization for other axiom types
            }
        }
    }
    writeln!(file, "</Ontology>")?;
    Ok(())
}

/// Parse FunctionalDataProperty axiom
fn parse_functional_data_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "FunctionalDataProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_data_property_expression(&children[0])?;
    Ok(Axiom::FunctionalDataProperty(
        crate::ontology::FunctionalDataPropertyAxiom {
            id: generate_axiom_id(),
            property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse ObjectPropertyDomain axiom
fn parse_object_property_domain(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "ObjectPropertyDomain must have exactly 2 children".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;
    let domain = parse_class_expression(&children[1], None)?;
    
    Ok(Axiom::ObjectPropertyDomain(
        crate::ontology::ObjectPropertyDomainAxiom {
            id: generate_axiom_id(),
            property,
            domain,
            annotations: Vec::new(),
        },
    ))
}

/// Parse ObjectPropertyRange axiom
fn parse_object_property_range(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "ObjectPropertyRange must have exactly 2 children".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;
    let range = parse_class_expression(&children[1], None)?;
    
    Ok(Axiom::ObjectPropertyRange(
        crate::ontology::ObjectPropertyRangeAxiom {
            id: generate_axiom_id(),
            property,
            range,
            annotations: Vec::new(),
        },
    ))
}

/// Parse DataPropertyDomain axiom
fn parse_data_property_domain(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "DataPropertyDomain must have exactly 2 children".to_string(),
        ));
    }

    let property = parse_data_property_expression(&children[0])?;
    let domain = parse_class_expression(&children[1], None)?;
    
    Ok(Axiom::DataPropertyDomain(
        crate::ontology::DataPropertyDomainAxiom {
            id: generate_axiom_id(),
            property,
            domain,
            annotations: Vec::new(),
        },
    ))
}

/// Parse DataPropertyRange axiom
fn parse_data_property_range(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "DataPropertyRange must have exactly 2 children".to_string(),
        ));
    }

    let property = parse_data_property_expression(&children[0])?;
    let range = parse_data_range(&children[1])?;
    
    Ok(Axiom::DataPropertyRange(
        crate::ontology::DataPropertyRangeAxiom {
            id: generate_axiom_id(),
            property,
            range,
            annotations: Vec::new(),
        },
    ))
}

/// Parse data property expression
fn parse_data_property_expression(element: &roxmltree::Node) -> Result<DataPropertyExpression> {
    match element.tag_name().name() {
        "DataProperty" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(DataPropertyExpression::DataProperty(DataProperty {
                    iri: IRI::new(iri),
                }))
            } else {
                Err(Error::io(
                    "DataProperty element missing IRI attribute".to_string(),
                ))
            }
        }
        _ => Err(Error::io(format!(
            "Unsupported data property expression: {}",
            element.tag_name().name()
        ))),
    }
}

/// Parse data range
fn parse_data_range(element: &roxmltree::Node) -> Result<DataRange> {
    match element.tag_name().name() {
        "Datatype" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(DataRange::Datatype(IRI::new(iri)))
            } else {
                Err(Error::io(
                    "Datatype element missing IRI attribute".to_string(),
                ))
            }
        }
        _ => {
            // For now, return a basic string datatype for unknown ranges
            Ok(DataRange::Datatype(IRI::new("http://www.w3.org/2001/XMLSchema#string")))
        }
    }
}
