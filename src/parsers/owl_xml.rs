//! OWL XML Parser
//!
//! This module implements parsing of OWL 2 ontologies from OWL XML format.

use crate::{
    Error, Result,
    ontology::{
        Axiom, Class, ClassExpression, DataProperty, DataPropertyExpression, DataRange,
        DeclarationAxiom, Entity, IRI, Individual, NamedIndividual, ObjectProperty,
        ObjectPropertyExpression, Ontology, axioms::DisjointUnionAxiom,
    },
    parsers::common::OntologySerializer,
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
    // Validate syntax before parsing
    let validator = super::validation::SyntaxValidator::new();
    validator.validate_owl_xml(content)?;

    // Basic OWL XML parser implementation
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| Error::io(format!("Failed to parse XML: {e}")))?;

    let mut ontology = Ontology::new();

    // Find the root element
    let root = doc.root_element();

    // Reject RDF/XML files that are being parsed as OWL/XML
    let root_name = root.tag_name().name();
    if root_name == "RDF"
        || root.tag_name().namespace() == Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
    {
        return Err(Error::ParseError(
            "This appears to be an RDF/XML file, not OWL/XML. Use RDF/XML parser instead."
                .to_string(),
        ));
    }

    // Check if this is a full ontology document or a fragment
    let is_fragment = root_name != "Ontology";

    if is_fragment {
        // Handle OWL/XML fragments (standalone Declaration, ClassAssertion, etc.)
        parse_axiom_element(&root, &mut ontology, &None)?;
        return Ok(ontology);
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
        parse_axiom_element(&child, &mut ontology, &base_iri)?;
    }

    Ok(ontology)
}

/// Parse a single axiom element (used for both full ontologies and fragments)
fn parse_axiom_element(
    element: &roxmltree::Node,
    ontology: &mut Ontology,
    base_iri: &Option<url::Url>,
) -> Result<()> {
    match element.tag_name().name() {
        "Declaration" => {
            if let Ok(axiom) = parse_declaration(element) {
                ontology.add_axiom(axiom);
            }
        }
        "SubClassOf" => {
            if let Ok(axiom) = parse_subclass_of(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        "DisjointUnion" => {
            if let Ok(axiom) = parse_disjoint_union(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        "EquivalentClasses" => {
            if let Ok(axiom) = parse_equivalent_classes(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        "DisjointClasses" => {
            println!("DEBUG: Found DisjointClasses in XML");
            if let Ok(axiom) = parse_disjoint_classes(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        "ClassAssertion" => {
            if let Ok(axiom) = parse_class_assertion(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        "ObjectPropertyAssertion" => {
            if let Ok(axiom) = parse_object_property_assertion(element) {
                ontology.add_axiom(axiom);
            }
        }
        "SubObjectPropertyOf" => {
            if let Ok(axiom) = parse_sub_object_property_of(element) {
                ontology.add_axiom(axiom);
            }
        }
        "EquivalentObjectProperties" => {
            if let Ok(axiom) = parse_equivalent_object_properties(element) {
                ontology.add_axiom(axiom);
            }
        }
        "DisjointObjectProperties" => {
            if let Ok(axiom) = parse_disjoint_object_properties(element) {
                ontology.add_axiom(axiom);
            }
        }
        "SubDataPropertyOf" => {
            println!("DEBUG: Found SubDataPropertyOf in XML");
            if let Ok(axiom) = parse_sub_data_property_of(element) {
                ontology.add_axiom(axiom);
            }
        }
        "EquivalentDataProperties" => {
            if let Ok(axiom) = parse_equivalent_data_properties(element) {
                ontology.add_axiom(axiom);
            }
        }
        "DisjointDataProperties" => {
            if let Ok(axiom) = parse_disjoint_data_properties(element) {
                ontology.add_axiom(axiom);
            }
        }
        "FunctionalObjectProperty" => {
            if let Ok(axiom) = parse_functional_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "InverseFunctionalObjectProperty" => {
            println!("DEBUG: Found InverseFunctionalObjectProperty in XML");
            if let Ok(axiom) = parse_inverse_functional_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "FunctionalDataProperty" => {
            println!("DEBUG: Found FunctionalDataProperty in XML");
            if let Ok(axiom) = parse_functional_data_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "ObjectPropertyDomain" => {
            println!("DEBUG: Found ObjectPropertyDomain in XML");
            if let Ok(axiom) = parse_object_property_domain(element) {
                ontology.add_axiom(axiom);
            }
        }
        "ObjectPropertyRange" => {
            println!("DEBUG: Found ObjectPropertyRange in XML");
            if let Ok(axiom) = parse_object_property_range(element) {
                ontology.add_axiom(axiom);
            }
        }
        "DataPropertyDomain" => {
            println!("DEBUG: Found DataPropertyDomain in XML");
            if let Ok(axiom) = parse_data_property_domain(element) {
                ontology.add_axiom(axiom);
            }
        }
        "DataPropertyRange" => {
            println!("DEBUG: Found DataPropertyRange in XML");
            if let Ok(axiom) = parse_data_property_range(element) {
                ontology.add_axiom(axiom);
            }
        }
        "TransitiveObjectProperty" => {
            println!("DEBUG: Found TransitiveObjectProperty in XML");
            match parse_transitive_object_property(element) {
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
            if let Ok(axiom) = parse_symmetric_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "ReflexiveObjectProperty" => {
            println!("DEBUG: Found ReflexiveObjectProperty in XML");
            if let Ok(axiom) = parse_reflexive_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "IrreflexiveObjectProperty" => {
            println!("DEBUG: Found IrreflexiveObjectProperty in XML");
            if let Ok(axiom) = parse_irreflexive_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "AsymmetricObjectProperty" => {
            println!("DEBUG: Found AsymmetricObjectProperty in XML");
            if let Ok(axiom) = parse_asymmetric_object_property(element) {
                ontology.add_axiom(axiom);
            }
        }
        "InverseObjectProperties" => {
            println!("DEBUG: Found InverseObjectProperties in XML");
            if let Ok(axiom) = parse_inverse_object_properties(element) {
                ontology.add_axiom(axiom);
            }
        }
        "SameIndividual" => {
            println!("DEBUG: Found SameIndividual in XML");
            if let Ok(axiom) = parse_same_individual(element) {
                println!("DEBUG: Successfully parsed SameIndividual axiom");
                ontology.add_axiom(axiom);
                println!("DEBUG: Added SameIndividual axiom to ontology");
            }
        }
        "DifferentIndividuals" => {
            println!("DEBUG: Found DifferentIndividuals in XML");
            if let Ok(axiom) = parse_different_individuals(element) {
                ontology.add_axiom(axiom);
            }
        }
        "HasKey" => {
            if let Ok(axiom) = parse_has_key(element, base_iri.as_ref()) {
                ontology.add_axiom(axiom);
            }
        }
        _ => {
            // Skip unknown elements or log warning
        }
    }

    Ok(())
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

/// Parse a `DisjointClasses` element
fn parse_disjoint_classes(element: &roxmltree::Node, base_iri: Option<&url::Url>) -> Result<Axiom> {
    let mut class_expressions = Vec::new();

    for child in element.children().filter(roxmltree::Node::is_element) {
        let expr = parse_class_expression(&child, base_iri)?;
        class_expressions.push(expr);
    }

    if class_expressions.len() < 2 {
        return Err(Error::io(
            "DisjointClasses must have at least 2 classes".to_string(),
        ));
    }

    Ok(Axiom::DisjointClasses(
        crate::ontology::DisjointClassesAxiom {
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

/// Parse a `SubDataPropertyOf` element
fn parse_sub_data_property_of(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 2 {
        return Err(Error::io(
            "SubDataPropertyOf must have exactly 2 children".to_string(),
        ));
    }

    let sub_property = parse_data_property_expression(&children[0])?;
    let super_property = parse_data_property_expression(&children[1])?;

    Ok(Axiom::SubDataPropertyOf(
        crate::ontology::SubDataPropertyOfAxiom {
            id: generate_axiom_id(),
            sub_property,
            super_property,
            annotations: Vec::new(),
        },
    ))
}

/// Parse an `EquivalentObjectProperties` element
fn parse_equivalent_object_properties(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.len() < 2 {
        return Err(Error::io(
            "EquivalentObjectProperties must have at least 2 children".to_string(),
        ));
    }

    let mut properties = Vec::new();
    for child in children {
        properties.push(parse_object_property_expression(&child)?);
    }

    Ok(Axiom::EquivalentObjectProperties(
        crate::ontology::EquivalentObjectPropertiesAxiom {
            id: generate_axiom_id(),
            properties,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `DisjointObjectProperties` element
fn parse_disjoint_object_properties(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.len() < 2 {
        return Err(Error::io(
            "DisjointObjectProperties must have at least 2 children".to_string(),
        ));
    }

    let mut properties = Vec::new();
    for child in children {
        properties.push(parse_object_property_expression(&child)?);
    }

    Ok(Axiom::DisjointObjectProperties(
        crate::ontology::DisjointObjectPropertiesAxiom {
            id: generate_axiom_id(),
            properties,
            annotations: Vec::new(),
        },
    ))
}

/// Parse an `EquivalentDataProperties` element
fn parse_equivalent_data_properties(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.len() < 2 {
        return Err(Error::io(
            "EquivalentDataProperties must have at least 2 children".to_string(),
        ));
    }

    let mut properties = Vec::new();
    for child in children {
        properties.push(parse_data_property_expression(&child)?);
    }

    Ok(Axiom::EquivalentDataProperties(
        crate::ontology::EquivalentDataPropertiesAxiom {
            id: generate_axiom_id(),
            properties,
            annotations: Vec::new(),
        },
    ))
}

/// Parse a `DisjointDataProperties` element
fn parse_disjoint_data_properties(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();

    if children.len() < 2 {
        return Err(Error::io(
            "DisjointDataProperties must have at least 2 children".to_string(),
        ));
    }

    let mut properties = Vec::new();
    for child in children {
        properties.push(parse_data_property_expression(&child)?);
    }

    Ok(Axiom::DisjointDataProperties(
        crate::ontology::DisjointDataPropertiesAxiom {
            id: generate_axiom_id(),
            properties,
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

/// Parse an `InverseFunctionalObjectProperty` element
fn parse_inverse_functional_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    if children.len() != 1 {
        return Err(Error::io(
            "InverseFunctionalObjectProperty must have exactly 1 child".to_string(),
        ));
    }

    let property = parse_object_property_expression(&children[0])?;

    Ok(Axiom::InverseFunctionalObjectProperty(
        crate::ontology::InverseFunctionalObjectPropertyAxiom {
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
                    object_properties.push(
                        crate::ontology::ObjectPropertyExpression::ObjectProperty(
                            crate::ontology::ObjectProperty { iri: IRI::new(iri) },
                        ),
                    );
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
                    iri: IRI::new(iri),
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
                let property = ObjectProperty { iri: IRI::new(iri) };
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

/// OWL XML serializer using common infrastructure
pub struct OwlXmlSerializer;

impl OwlXmlSerializer {
    /// Create a new OWL XML serializer
    pub fn new() -> Self {
        Self
    }
}

impl Default for OwlXmlSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl OntologySerializer for OwlXmlSerializer {
    /// Serialize an ontology to OWL XML format string
    fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut output = String::new();

        // Write XML declaration and namespace declarations
        output.push_str("<?xml version=\"1.0\"?>\n");
        output.push_str("<Ontology xmlns=\"http://www.w3.org/2002/07/owl#\"\n");
        output.push_str("         xml:base=\"http://www.w3.org/2002/07/owl#\"\n");
        output.push_str("         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
        output.push_str("         xmlns:xml=\"http://www.w3.org/XML/1998/namespace\"\n");
        output.push_str("         xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"\n");
        output.push_str("         xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"\n");

        // Add ontology IRI and version IRI if present
        if let Some(onto_iri) = ontology.get_iri() {
            output.push_str(&format!("         ontologyIRI=\"{}\"", onto_iri));
            if let Some(version_iri) = &ontology.version_iri {
                output.push_str(&format!("\n         versionIRI=\"{}\"", version_iri));
            }
            output.push_str(">\n");
        } else {
            output.push_str(">\n");
        }

        // Write imports
        for import in &ontology.imports {
            output.push_str(&format!("  <Import>{}</Import>\n", import));
        }

        // Write ontology annotations
        for annotation in &ontology.annotations {
            output.push_str(&format!("  {}\n", serialize_annotation_xml(annotation, 1)));
        }

        // Write axioms
        for axiom in ontology.axioms() {
            output.push_str(&format!("  {}\n", serialize_axiom_xml(axiom)));
        }

        output.push_str("</Ontology>\n");
        Ok(output)
    }
}

/// Save ontology to OWL XML file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = OwlXmlSerializer::new();
    serializer.serialize_to_file(ontology, path)
}

fn serialize_axiom_xml(axiom: &Axiom) -> String {
    match axiom {
        Axiom::Declaration(decl) => {
            format!(
                "<Declaration>{}</Declaration>",
                serialize_entity_xml(&decl.entity)
            )
        }
        Axiom::SubClassOf(axiom) => {
            format!(
                "<SubClassOf>{}{}</SubClassOf>",
                serialize_class_expression_xml(&axiom.subclass),
                serialize_class_expression_xml(&axiom.superclass)
            )
        }
        Axiom::EquivalentClasses(axiom) => {
            let classes_xml: Vec<String> = axiom
                .classes
                .iter()
                .map(serialize_class_expression_xml)
                .collect();
            format!(
                "<EquivalentClasses>{}</EquivalentClasses>",
                classes_xml.join("")
            )
        }
        Axiom::DisjointClasses(axiom) => {
            let classes_xml: Vec<String> = axiom
                .classes
                .iter()
                .map(serialize_class_expression_xml)
                .collect();
            format!(
                "<DisjointClasses>{}</DisjointClasses>",
                classes_xml.join("")
            )
        }
        Axiom::DisjointUnion(axiom) => {
            let mut result = format!(
                "<DisjointUnion>{}",
                serialize_class_expression_xml(&axiom.class)
            );
            for disjoint_class in &axiom.disjoint_classes {
                result.push_str(&serialize_class_expression_xml(disjoint_class));
            }
            result.push_str("</DisjointUnion>");
            result
        }
        Axiom::ClassAssertion(axiom) => {
            format!(
                "<ClassAssertion>{}{}</ClassAssertion>",
                serialize_class_expression_xml(&axiom.class),
                serialize_individual_xml(&axiom.individual)
            )
        }
        Axiom::ObjectPropertyAssertion(axiom) => {
            format!(
                "<ObjectPropertyAssertion>{}{}{}</ObjectPropertyAssertion>",
                match &axiom.property {
                    crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =>
                        serialize_object_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                },
                serialize_individual_xml(&axiom.source),
                serialize_individual_xml(&axiom.target)
            )
        }
        Axiom::SubObjectPropertyOf(axiom) => {
            format!(
                "<SubObjectPropertyOf>{}{}</SubObjectPropertyOf>",
                match &axiom.sub_property {
                    crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =>
                        serialize_object_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                },
                match &axiom.super_property {
                    crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =>
                        serialize_object_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                }
            )
        }
        Axiom::FunctionalObjectProperty(axiom) => match &axiom.property {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                format!(
                    "<FunctionalObjectProperty>{}</FunctionalObjectProperty>",
                    serialize_object_property_xml(prop)
                )
            }
            _ => format!(
                "<FunctionalObjectProperty><!-- Complex property expression --></FunctionalObjectProperty>"
            ),
        },
        Axiom::InverseFunctionalObjectProperty(axiom) => match &axiom.property {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                format!(
                    "<InverseFunctionalObjectProperty>{}</InverseFunctionalObjectProperty>",
                    serialize_object_property_xml(prop)
                )
            }
            _ => format!(
                "<InverseFunctionalObjectProperty><!-- Complex property expression --></InverseFunctionalObjectProperty>"
            ),
        },
        Axiom::DataPropertyAssertion(axiom) => {
            format!(
                "<DataPropertyAssertion>{}{}{}</DataPropertyAssertion>",
                match &axiom.property {
                    crate::ontology::DataPropertyExpression::DataProperty(prop) =>
                        serialize_data_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                },
                serialize_individual_xml(&axiom.individual),
                serialize_literal_xml(&axiom.value)
            )
        }
        Axiom::SubDataPropertyOf(axiom) => {
            format!(
                "<SubDataPropertyOf>{}{}</SubDataPropertyOf>",
                match &axiom.sub_property {
                    crate::ontology::DataPropertyExpression::DataProperty(prop) =>
                        serialize_data_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                },
                match &axiom.super_property {
                    crate::ontology::DataPropertyExpression::DataProperty(prop) =>
                        serialize_data_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                }
            )
        }
        Axiom::FunctionalDataProperty(axiom) => {
            format!(
                "<FunctionalDataProperty>{}</FunctionalDataProperty>",
                match &axiom.property {
                    crate::ontology::DataPropertyExpression::DataProperty(prop) =>
                        serialize_data_property_xml(prop),
                    _ => "<!-- Complex property expression -->".to_string(),
                }
            )
        }
        _ => format!("<!-- Unsupported axiom type: {:?} -->", axiom),
    }
}

fn serialize_entity_xml(entity: &crate::ontology::Entity) -> String {
    match entity {
        crate::ontology::Entity::Class(class) => format!("<Class IRI=\"{}\"/>", class.as_str()),
        crate::ontology::Entity::ObjectProperty(prop) => {
            format!("<ObjectProperty IRI=\"{}\"/>", prop.as_str())
        }
        crate::ontology::Entity::DataProperty(prop) => {
            format!("<DataProperty IRI=\"{}\"/>", prop.as_str())
        }
        crate::ontology::Entity::NamedIndividual(ind) => {
            format!("<NamedIndividual IRI=\"{}\"/>", ind.as_str())
        }
        crate::ontology::Entity::Datatype(dt) => format!("<Datatype IRI=\"{}\"/>", dt.as_str()),
        crate::ontology::Entity::AnnotationProperty(ap) => {
            format!("<AnnotationProperty IRI=\"{}\"/>", ap.as_str())
        }
    }
}

fn serialize_class_expression_xml(ce: &crate::ontology::ClassExpression) -> String {
    match ce {
        crate::ontology::ClassExpression::Class(class) => {
            format!("<Class IRI=\"{}\"/>", class.iri.as_str())
        }
        crate::ontology::ClassExpression::ObjectIntersectionOf(classes) => {
            let classes_xml: Vec<String> =
                classes.iter().map(serialize_class_expression_xml).collect();
            format!(
                "<ObjectIntersectionOf>{}</ObjectIntersectionOf>",
                classes_xml.join("")
            )
        }
        crate::ontology::ClassExpression::ObjectUnionOf(classes) => {
            let classes_xml: Vec<String> =
                classes.iter().map(serialize_class_expression_xml).collect();
            format!("<ObjectUnionOf>{}</ObjectUnionOf>", classes_xml.join(""))
        }
        crate::ontology::ClassExpression::ObjectComplementOf(class) => {
            format!(
                "<ObjectComplementOf>{}</ObjectComplementOf>",
                serialize_class_expression_xml(class)
            )
        }
        _ => format!("<!-- Unsupported class expression: {:?} -->", ce),
    }
}

fn serialize_individual_xml(ind: &crate::ontology::Individual) -> String {
    format!(
        "<NamedIndividual IRI=\"{}\"/>",
        ind.iri().map(|iri| iri.as_str()).unwrap_or("_:anonymous")
    )
}

fn serialize_object_property_xml(prop: &crate::ontology::ObjectProperty) -> String {
    format!("<ObjectProperty IRI=\"{}\"/>", prop.iri.as_str())
}

fn serialize_data_property_xml(prop: &crate::ontology::DataProperty) -> String {
    format!("<DataProperty IRI=\"{}\"/>", prop.iri.as_str())
}

fn serialize_literal_xml(lit: &crate::ontology::Literal) -> String {
    if let Some(datatype) = &lit.datatype {
        format!(
            "<Literal datatypeIRI=\"{}\">{}</Literal>",
            datatype.as_str(),
            lit.value
        )
    } else if let Some(lang) = &lit.language {
        format!("<Literal xml:lang=\"{}\">{}</Literal>", lang, lit.value)
    } else {
        format!("<Literal>{}</Literal>", lit.value)
    }
}

fn serialize_annotation_xml(annotation: &crate::ontology::Annotation, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    match &annotation.value {
        crate::ontology::AnnotationValue::IRI(iri) => {
            format!(
                "{}<Annotation><AnnotationProperty IRI=\"{}\"/><IRI>{}</IRI></Annotation>",
                indent_str, annotation.property.iri, iri
            )
        }
        crate::ontology::AnnotationValue::Literal(lit) => {
            format!(
                "{}<Annotation><AnnotationProperty IRI=\"{}\"/>{}</Annotation>",
                indent_str,
                annotation.property.iri,
                serialize_literal_xml(lit)
            )
        }
        crate::ontology::AnnotationValue::AnonymousIndividual(_) => {
            format!(
                "{}<Annotation><AnnotationProperty IRI=\"{}\"/><!-- Anonymous individual annotation not fully supported --></Annotation>",
                indent_str,
                annotation.property.iri.as_str()
            )
        }
    }
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
            Ok(DataRange::Datatype(IRI::new(
                "http://www.w3.org/2001/XMLSchema#string",
            )))
        }
    }
}
