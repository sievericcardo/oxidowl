//! OWL XML Parser
//!
//! This module implements parsing of OWL 2 ontologies from OWL XML format.

use crate::{
    Error, Result,
    ontology::{
        Ontology, ClassExpression, Individual, NamedIndividual, IRI, Axiom, ObjectPropertyExpression, Entity, DeclarationAxiom,
        Class, ObjectProperty,
        axioms::{SubClassOfAxiom, EquivalentClassesAxiom, ClassAssertionAxiom, ObjectPropertyAssertionAxiom, 
                SubObjectPropertyOfAxiom, FunctionalObjectPropertyAxiom}
    },
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write, Read},
    collections::HashMap,
    path::Path,
};
use url::Url;

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// OWL XML Parser
#[derive(Debug, Clone)]
pub struct OWLXMLParser {
    // TODO: add a parser configuration
}

impl OWLXMLParser {
    /// Create a new OWL XML parser
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for OWLXMLParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse OWL XML from string content
pub fn parse(content: &str) -> Result<Ontology> {
    // Basic OWL XML parser implementation
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| Error::io(format!("Failed to parse XML: {}", e)))?;
    
    let mut ontology = Ontology::new();
    
    // Find the root Ontology element
    let root = doc.root_element();
    if root.tag_name().name() != "Ontology" {
        return Err(Error::io("Root element must be Ontology".to_string()));
    }
    
    // Extract ontology IRI if present
    if let Some(iri) = root.attribute("ontologyIRI") {
        if let Ok(url) = url::Url::parse(iri) {
            ontology.iri = Some(url.into());
        }
    }
    
    // Parse declarations and axioms
    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "Declaration" => {
                if let Ok(axiom) = parse_declaration(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "SubClassOf" => {
                if let Ok(axiom) = parse_subclass_of(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "EquivalentClasses" => {
                if let Ok(axiom) = parse_equivalent_classes(&child) {
                    ontology.add_axiom(axiom);
                }
            }
            "ClassAssertion" => {
                if let Ok(axiom) = parse_class_assertion(&child) {
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
    for child in element.children().filter(|n| n.is_element()) {
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

/// Parse a SubClassOf element
fn parse_subclass_of(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
    if children.len() != 2 {
        return Err(Error::io("SubClassOf must have exactly 2 children".to_string()));
    }
    
    let subclass = parse_class_expression(&children[0])?;
    let superclass = parse_class_expression(&children[1])?;
    
    Ok(Axiom::SubClassOf(crate::ontology::SubClassOfAxiom {
        id: generate_axiom_id(),
        subclass,
        superclass,
        annotations: Vec::new(),
    }))
}

/// Parse an EquivalentClasses element
fn parse_equivalent_classes(element: &roxmltree::Node) -> Result<Axiom> {
    let mut class_expressions = Vec::new();
    
    for child in element.children().filter(|n| n.is_element()) {
        let expr = parse_class_expression(&child)?;
        class_expressions.push(expr);
    }
    
    if class_expressions.len() < 2 {
        return Err(Error::io("EquivalentClasses must have at least 2 classes".to_string()));
    }
    
    Ok(Axiom::EquivalentClasses(crate::ontology::EquivalentClassesAxiom {
        id: generate_axiom_id(),
        classes: class_expressions,
        annotations: Vec::new(),
    }))
}

/// Parse a ClassAssertion element
fn parse_class_assertion(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
    if children.len() != 2 {
        return Err(Error::io("ClassAssertion must have exactly 2 children".to_string()));
    }
    
    let class_expression = parse_class_expression(&children[0])?;
    let individual = parse_individual(&children[1])?;
    
    Ok(Axiom::ClassAssertion(crate::ontology::ClassAssertionAxiom {
        id: generate_axiom_id(),
        class: class_expression,
        individual,
        annotations: Vec::new(),
    }))
}

/// Parse an ObjectPropertyAssertion element
fn parse_object_property_assertion(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
    if children.len() != 3 {
        return Err(Error::io("ObjectPropertyAssertion must have exactly 3 children".to_string()));
    }
    
    let property = parse_object_property_expression(&children[0])?;
    let source = parse_individual(&children[1])?;
    let target = parse_individual(&children[2])?;
    
    Ok(Axiom::ObjectPropertyAssertion(crate::ontology::ObjectPropertyAssertionAxiom {
        id: generate_axiom_id(),
        property,
        source: source,
        target: target,
        annotations: Vec::new(),
    }))
}

/// Parse a SubObjectPropertyOf element
fn parse_sub_object_property_of(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
    if children.len() != 2 {
        return Err(Error::io("SubObjectPropertyOf must have exactly 2 children".to_string()));
    }
    
    let sub_property = parse_object_property_expression(&children[0])?;
    let super_property = parse_object_property_expression(&children[1])?;
    
    Ok(Axiom::SubObjectPropertyOf(crate::ontology::SubObjectPropertyOfAxiom {
        id: generate_axiom_id(),
        sub_property,
        super_property,
        annotations: Vec::new(),
    }))
}

/// Parse a FunctionalObjectProperty element
fn parse_functional_object_property(element: &roxmltree::Node) -> Result<Axiom> {
    let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
    if children.len() != 1 {
        return Err(Error::io("FunctionalObjectProperty must have exactly 1 child".to_string()));
    }
    
    let property = parse_object_property_expression(&children[0])?;
    
    Ok(Axiom::FunctionalObjectProperty(crate::ontology::FunctionalObjectPropertyAxiom {
        id: generate_axiom_id(),
        property,
        annotations: Vec::new(),
    }))
}

/// Parse a class expression
fn parse_class_expression(element: &roxmltree::Node) -> Result<ClassExpression> {
    match element.tag_name().name() {
        "Class" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(ClassExpression::Class(Class {
                    iri: IRI::new(iri).to_url()?.into(),
                }))
            } else {
                Err(Error::io("Class element missing IRI attribute".to_string()))
            }
        }
        "ObjectIntersectionOf" => {
            let mut operands = Vec::new();
            for child in element.children().filter(|n| n.is_element()) {
                operands.push(parse_class_expression(&child)?);
            }
            Ok(ClassExpression::ObjectIntersectionOf(operands))
        }
        "ObjectUnionOf" => {
            let mut operands = Vec::new();
            for child in element.children().filter(|n| n.is_element()) {
                operands.push(parse_class_expression(&child)?);
            }
            Ok(ClassExpression::ObjectUnionOf(operands))
        }
        "ObjectComplementOf" => {
            let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
            if children.len() != 1 {
                return Err(Error::io("ObjectComplementOf must have exactly 1 child".to_string()));
            }
            let operand = Box::new(parse_class_expression(&children[0])?);
            Ok(ClassExpression::ObjectComplementOf(operand))
        }
        "ObjectSomeValuesFrom" => {
            let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
            if children.len() != 2 {
                return Err(Error::io("ObjectSomeValuesFrom must have exactly 2 children".to_string()));
            }
            let property = parse_object_property_expression(&children[0])?;
            let filler = Box::new(parse_class_expression(&children[1])?);
            Ok(ClassExpression::ObjectSomeValuesFrom { property, filler })
        }
        "ObjectAllValuesFrom" => {
            let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
            if children.len() != 2 {
                return Err(Error::io("ObjectAllValuesFrom must have exactly 2 children".to_string()));
            }
            let property = parse_object_property_expression(&children[0])?;
            let filler = Box::new(parse_class_expression(&children[1])?);
            Ok(ClassExpression::ObjectAllValuesFrom { property, filler })
        }
        "ObjectOneOf" => {
            let mut individuals = Vec::new();
            for child in element.children().filter(|n| n.is_element()) {
                individuals.push(parse_individual(&child)?);
            }
            Ok(ClassExpression::ObjectOneOf(individuals))
        }
        _ => {
            Err(Error::io(format!("Unsupported class expression: {}", element.tag_name().name())))
        }
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
                Err(Error::io("ObjectProperty element missing IRI attribute".to_string()))
            }
        }
        "ObjectInverseOf" => {
            let children: Vec<_> = element.children().filter(|n| n.is_element()).collect();
            if children.len() != 1 {
                return Err(Error::io("ObjectInverseOf must have exactly 1 child".to_string()));
            }
            // Parse the child as ObjectProperty, not ObjectPropertyExpression
            if let Some(iri) = children[0].attribute("IRI") {
                let property = ObjectProperty {
                    iri: IRI::new(iri).to_url()?,
                };
                Ok(ObjectPropertyExpression::InverseObjectProperty(property))
            } else {
                Err(Error::io("ObjectProperty element missing IRI attribute".to_string()))
            }
        }
        _ => {
            Err(Error::io(format!("Unsupported object property expression: {}", element.tag_name().name())))
        }
    }
}

/// Parse an individual
fn parse_individual(element: &roxmltree::Node) -> Result<Individual> {
    match element.tag_name().name() {
        "NamedIndividual" => {
            if let Some(iri) = element.attribute("IRI") {
                Ok(Individual::Named(NamedIndividual { iri: IRI::new(iri).to_url()?.into() }))
            } else {
                Err(Error::io("NamedIndividual element missing IRI attribute".to_string()))
            }
        }
        _ => {
            Err(Error::io(format!("Unsupported individual type: {}", element.tag_name().name())))
        }
    }
}

/// Parse OWL XML from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path)
        .map_err(|e| Error::io(format!("Failed to open file: {}", e)))?;
    
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {}", e)))?;
    
    parse(&content)
}

/// Save ontology to OWL XML file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let mut file = File::create(path)
        .map_err(|e| Error::io(format!("Failed to create file: {}", e)))?;

    // TODO: Implement a better serialization to OWL XML
    writeln!(file, "<Ontology ontologyIRI=\"{}\">", ontology.iri.as_ref().map_or("http://example.org/ontology", |iri| iri.as_str()))?;
    for axiom in ontology.axioms() {
        match axiom {
            Axiom::Declaration(decl) => {
                writeln!(file, "  <Declaration><{} IRI=\"{}\"/></Declaration>", decl.entity.entity_type(), decl.entity.iri)?;
            }
            Axiom::SubClassOf(axiom) => {
                if let (Some(subclass_iri), Some(superclass_iri)) = (axiom.subclass.iri(), axiom.superclass.iri()) {
                    writeln!(file, "  <SubClassOf><Class IRI=\"{}\"/><Class IRI=\"{}\"/></SubClassOf>", subclass_iri, superclass_iri)?;
                }
            }
            Axiom::EquivalentClasses(axiom) => {
                writeln!(file, "  <EquivalentClasses>",)?;
                for class in &axiom.classes {
                    if let Some(class_iri) = class.iri() {
                        writeln!(file, "    <Class IRI=\"{}\"/>", class_iri)?;
                    }
                }
                writeln!(file, "  </EquivalentClasses>")?;
            }
            Axiom::ClassAssertion(axiom) => {
                if let (Some(class_iri), Some(individual_iri)) = (axiom.class.iri(), axiom.individual.iri()) {
                    writeln!(file, "  <ClassAssertion><Class IRI=\"{}\"/><NamedIndividual IRI=\"{}\"/></ClassAssertion>", class_iri, individual_iri)?;
                }
            }
            Axiom::ObjectPropertyAssertion(axiom) => {
                if let (Some(source_iri), Some(target_iri)) = (axiom.source.iri(), axiom.target.iri()) {
                    if let Some(property_iri) = axiom.property.iri() {
                        writeln!(file, "  <ObjectPropertyAssertion><ObjectProperty IRI=\"{}\"/><NamedIndividual IRI=\"{}\"/><NamedIndividual IRI=\"{}\"/></ObjectPropertyAssertion>", 
                            property_iri, source_iri, target_iri)?;
                    }
                }
            }
            Axiom::SubObjectPropertyOf(axiom) => {
                if let (Some(sub_iri), Some(super_iri)) = (axiom.sub_property.iri(), axiom.super_property.iri()) {
                    writeln!(file, "  <SubObjectPropertyOf><ObjectProperty IRI=\"{}\"/><ObjectProperty IRI=\"{}\"/></SubObjectPropertyOf>", sub_iri, super_iri)?;
                }
            }
            Axiom::FunctionalObjectProperty(axiom) => {
                if let Some(property_iri) = axiom.property.iri() {
                    writeln!(file, "  <FunctionalObjectProperty><ObjectProperty IRI=\"{}\"/></FunctionalObjectProperty>", property_iri)?;
                }
            }
        }
    }
    writeln!(file, "</Ontology>")?;
    Ok(())
}
