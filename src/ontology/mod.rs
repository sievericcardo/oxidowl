//! OWL 2 DL Ontology module
//!
//! This module provides the core ontology types and structures for OWL 2 DL reasoning.

use crate::Result;
use std::collections::HashMap;
use url::Url;

pub mod axioms;
pub mod concepts;
pub mod individuals;
pub mod properties;

// Re-export main types
pub use axioms::*;
pub use concepts::*;
pub use individuals::*;
pub use properties::*;
pub use concepts::*;
pub use individuals::*;  
pub use properties::*;

/// IRI (Internationalized Resource Identifier) wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRI {
    value: String,
}

impl IRI {
    /// Create a new IRI from a string
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }

    /// Convert to URL
    pub fn to_url(&self) -> Result<Url> {
        Url::parse(&self.value)
            .map_err(|e| crate::Error::ontology_parsing(format!("Invalid IRI: {}", e)))
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl From<String> for IRI {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl std::fmt::Display for IRI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// Display implementation for ClassExpression to support formatting in HyperTableau
impl std::fmt::Display for ClassExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassExpression::Class(class) => write!(f, "{}", class.iri),
            ClassExpression::ObjectIntersectionOf(operands) => {
                write!(f, "(")?;
                for (i, operand) in operands.iter().enumerate() {
                    if i > 0 { write!(f, " ⊓ ")?; }
                    write!(f, "{}", operand)?;
                }
                write!(f, ")")
            },
            ClassExpression::ObjectUnionOf(operands) => {
                write!(f, "(")?;
                for (i, operand) in operands.iter().enumerate() {
                    if i > 0 { write!(f, " ⊔ ")?; }
                    write!(f, "{}", operand)?;
                }
                write!(f, ")")
            },
            ClassExpression::ObjectComplementOf(operand) => write!(f, "¬{}", operand),
            ClassExpression::ObjectOneOf(individuals) => {
                write!(f, "{{")?;
                for (i, individual) in individuals.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", individual)?;
                }
                write!(f, "}}")
            },
            ClassExpression::ObjectSomeValuesFrom { property, filler } => 
                write!(f, "∃{}.{}", property, filler),
            ClassExpression::ObjectAllValuesFrom { property, filler } => 
                write!(f, "∀{}.{}", property, filler),
            ClassExpression::ObjectHasValue { property, value } => 
                write!(f, "∃{}.{{{}}}", property, value),
            ClassExpression::ObjectHasSelf { property } => 
                write!(f, "∃{}.Self", property),
            ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                write!(f, "≥{} {}.{}", cardinality, property, filler)
            },
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                write!(f, "≤{} {}.{}", cardinality, property, filler)
            },
            ClassExpression::ObjectExactCardinality { cardinality, property, filler } => {
                write!(f, "={} {}.{}", cardinality, property, filler)
            },
            // For data property expressions, just use a simplified representation
            _ => write!(f, "ComplexExpression"),
        }
    }
}

impl std::fmt::Display for ObjectPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectPropertyExpression::ObjectProperty(prop) => write!(f, "{}", prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => write!(f, "{}⁻", prop.iri),
            ObjectPropertyExpression::PropertyChain(chain) => {
                write!(f, "PropertyChain(")?;
                for (i, prop) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ∘ ")?;
                    }
                    write!(f, "{}", prop)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Object Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectProperty {
    pub iri: Url,
}

/// Object property expressions (simple or complex)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObjectPropertyExpression {
    /// Named object property
    ObjectProperty(ObjectProperty),
    
    /// Inverse object property
    InverseObjectProperty(ObjectProperty),
    
    /// Property chain (OWL 2 property composition)
    PropertyChain(Vec<ObjectPropertyExpression>),
}

/// Data Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataProperty {
    pub iri: url::Url,
}

/// Data property expressions (simple or complex)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataPropertyExpression {
    /// Named data property
    DataProperty(DataProperty),
}

impl std::fmt::Display for DataPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataPropertyExpression::DataProperty(prop) => write!(f, "{}", prop.iri),
        }
    }
}

/// Annotation Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnnotationProperty {
    pub iri: url::Url,
}

/// Annotation property expressions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationPropertyExpression {
    /// Named annotation property
    AnnotationProperty(AnnotationProperty),
}

impl std::fmt::Display for AnnotationPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotationPropertyExpression::AnnotationProperty(prop) => write!(f, "{}", prop.iri),
        }
    }
}

/// Literal value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    /// Lexical value
    pub value: String,
    /// Language tag (if present)
    pub language: Option<String>,
    /// Datatype IRI
    pub datatype: Option<url::Url>,
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.value)?;
        if let Some(lang) = &self.language {
            write!(f, "@{}", lang)?;
        } else if let Some(dt) = &self.datatype {
            write!(f, "^^<{}>", dt)?;
        }
        Ok(())
    }
}

/// Data Range (OWL 2 Datatype expression)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataRange {
    /// Named datatype
    Datatype(url::Url),
    /// Intersection of data ranges
    DataIntersectionOf(Vec<DataRange>),
    /// Union of data ranges
    DataUnionOf(Vec<DataRange>),
    /// Complement of a data range
    DataComplementOf(Box<DataRange>),
    /// Enumeration of literals
    DataOneOf(Vec<Literal>),
    /// Datatype restriction
    DatatypeRestriction {
        datatype: url::Url,
        restrictions: Vec<FacetRestriction>,
    },
}

/// Facet restriction for datatype restrictions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FacetRestriction {
    pub facet: url::Url,
    pub value: Literal,
}

/// Annotation subject
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationSubject {
    /// IRI
    IRI(url::Url),
    /// Anonymous individual
    AnonymousIndividual(AnonymousIndividual),
}

/// Annotation value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationValue {
    /// IRI
    IRI(url::Url),
    /// Anonymous individual
    AnonymousIndividual(AnonymousIndividual),
    /// Literal
    Literal(Literal),
}

/// Annotation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Annotation {
    pub property: AnnotationProperty,
    pub value: AnnotationValue,
}

/// Main ontology structure containing all axioms and metadata
#[derive(Debug, Clone)]
pub struct Ontology {
    /// All axioms
    pub axioms: Vec<axioms::Axiom>,
    /// Ontology annotations
    pub annotations: Vec<Annotation>,
    /// Ontology IRI
    pub iri: Option<IRI>,
    /// Version IRI
    pub version_iri: Option<IRI>,
    /// Imports
    pub imports: Vec<IRI>,
}

impl Ontology {
    /// Create a new empty ontology
    pub fn new() -> Self {
        Self {
            axioms: Vec::new(),
            annotations: Vec::new(),
            iri: None,
            version_iri: None,
            imports: Vec::new(),
        }
    }
    
    /// Set the ontology IRI
    pub fn set_iri(&mut self, iri: IRI) {
        self.iri = Some(iri);
    }
    
    /// Get the ontology IRI
    pub fn get_iri(&self) -> Option<&IRI> {
        self.iri.as_ref()
    }
    
    /// Add an axiom to the ontology
    pub fn add_axiom(&mut self, axiom: axioms::Axiom) {
        self.axioms.push(axiom);
    }
    
    /// Get all axioms
    pub fn axioms(&self) -> &[axioms::Axiom] {
        &self.axioms
    }
    
    /// Add a class (placeholder for compatibility)
    pub fn add_class(&mut self, class: concepts::Class) {
        // This creates a declaration axiom for the class
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: 0, // TODO: proper ID generation
            entity: axioms::Entity::Class(class.iri), // TODO: proper conversion
        });
        self.add_axiom(axiom);
    }
    
    /// Add an object property (placeholder for compatibility)
    pub fn add_object_property(&mut self, property: properties::ObjectProperty) {
        // This creates a declaration axiom for the property
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: 0, // TODO: proper ID generation
            entity: axioms::Entity::ObjectProperty(property.iri), // TODO: proper conversion
        });
        self.add_axiom(axiom);
    }
    
    /// Add an individual (placeholder for compatibility)
    pub fn add_individual(&mut self, _subject: IRI, _individual: individuals::Individual) {
        // TODO: implement proper individual handling
    }
    
    /// Get classes (placeholder for compatibility)
    pub fn classes(&self) -> Vec<(IRI, concepts::Class)> {
        // TODO: extract classes from axioms
        vec![]
    }
    
    /// Get individuals (placeholder for compatibility)
    pub fn individuals(&self) -> Vec<(IRI, individuals::Individual)> {
        // TODO: extract individuals from axioms
        vec![]
    }
    
    /// Get object properties (placeholder for compatibility)
    pub fn object_properties(&self) -> Vec<properties::ObjectProperty> {
        // TODO: extract object properties from axioms
        vec![]
    }
}

impl Default for Ontology {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported ontology formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OntologyFormat {
    /// Auto-detect format
    Auto,
    /// OWL Functional Syntax
    Functional,
    /// OWL/XML
    OwlXml,
    /// RDF/XML
    RdfXml,
    /// Turtle
    Turtle,
    /// N-Triples
    NTriples,
    /// Manchester Syntax
    Manchester,
}

impl OntologyFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "",
            OntologyFormat::Functional => "owx",
            OntologyFormat::OwlXml => "owl",
            OntologyFormat::RdfXml => "rdf",
            OntologyFormat::Turtle => "ttl",
            OntologyFormat::NTriples => "nt",
            OntologyFormat::Manchester => "omn",
        }
    }
    
    /// Get the media type for this format
    pub fn media_type(&self) -> &'static str {
        match self {
            OntologyFormat::Functional => "text/owl-functional",
            OntologyFormat::OwlXml => "application/owl+xml",
            OntologyFormat::RdfXml => "application/rdf+xml",
            OntologyFormat::Turtle => "text/turtle",
            OntologyFormat::NTriples => "application/n-triples",
            OntologyFormat::Manchester => "text/owl-manchester",
        }
    }
    
    /// Try to detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "owx" => Some(OntologyFormat::Functional),
            "owl" => Some(OntologyFormat::OwlXml),
            "rdf" => Some(OntologyFormat::RdfXml),
            "ttl" => Some(OntologyFormat::Turtle),
            "nt" => Some(OntologyFormat::NTriples),
            "omn" => Some(OntologyFormat::Manchester),
            _ => None,
        }
    }
}