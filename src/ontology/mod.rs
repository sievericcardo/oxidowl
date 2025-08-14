//! OWL 2 DL Ontology module
//!
//! This module provides the core ontology types and structures for OWL 2 DL reasoning.

use crate::{Error, Result};
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

use std::sync::{Arc, RwLock};

/// Type alias for a thread-safe, shared ontology reference
///
/// This type represents an ontology that can be safely shared across threads
/// and allows for both read and write access through the RwLock.
pub type OntologyRef = Arc<RwLock<Ontology>>;

/// IRI (Internationalized Resource Identifier) wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRI {
    value: String,
}

impl IRI {
    /// Create a new IRI from a string
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }

    /// Convert to URL
    pub fn to_url(&self) -> Result<Url> {
        Url::parse(&self.value)
            .map_err(|e| crate::Error::ontology_parsing(format!("Invalid IRI: {e}")))
    }

    /// Get the string value
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl From<String> for IRI {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<Url> for IRI {
    fn from(url: Url) -> Self {
        Self {
            value: url.to_string(),
        }
    }
}

impl std::fmt::Display for IRI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
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
                    write!(f, "{prop}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl ObjectPropertyExpression {
    /// Get the IRI if this is a simple object property
    #[must_use]
    pub fn iri(&self) -> Option<&url::Url> {
        match self {
            ObjectPropertyExpression::ObjectProperty(prop) => Some(&prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => Some(&prop.iri),
            ObjectPropertyExpression::PropertyChain(_) => None,
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
    pub iri: IRI,
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
    pub iri: IRI,
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

impl Literal {
    /// Create a new literal with just a value
    #[must_use]
    pub fn new(value: String) -> Self {
        Self {
            value,
            language: None,
            datatype: None,
        }
    }

    /// Create a literal with a language tag
    #[must_use]
    pub fn with_language(value: String, language: String) -> Self {
        Self {
            value,
            language: Some(language),
            datatype: None,
        }
    }

    /// Create a literal with a datatype
    #[must_use]
    pub fn with_datatype(value: String, datatype: IRI) -> Self {
        Self {
            value,
            language: None,
            datatype: datatype.to_url().ok(),
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.value)?;
        if let Some(lang) = &self.language {
            write!(f, "@{lang}")?;
        } else if let Some(dt) = &self.datatype {
            write!(f, "^^<{dt}>")?;
        }
        Ok(())
    }
}

/// Data Range (OWL 2 Datatype expression)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataRange {
    /// Named datatype
    Datatype(IRI),
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
        datatype: IRI,
        restrictions: Vec<FacetRestriction>,
    },
}

impl std::fmt::Display for DataRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataRange::Datatype(iri) => write!(f, "{iri}"),
            DataRange::DataIntersectionOf(ranges) => {
                write!(f, "(")?;
                for (i, range) in ranges.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊓ ")?;
                    }
                    write!(f, "{range}")?;
                }
                write!(f, ")")
            }
            DataRange::DataUnionOf(ranges) => {
                write!(f, "(")?;
                for (i, range) in ranges.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊔ ")?;
                    }
                    write!(f, "{range}")?;
                }
                write!(f, ")")
            }
            DataRange::DataComplementOf(range) => write!(f, "¬{range}"),
            DataRange::DataOneOf(literals) => {
                write!(f, "{{")?;
                for (i, literal) in literals.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{literal:?}")?;
                }
                write!(f, "}}")
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions: _,
            } => {
                write!(f, "{datatype}[restrictions]")
            }
        }
    }
}

/// Facet restriction for datatype restrictions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FacetRestriction {
    pub facet: IRI,
    pub value: Literal,
}

/// Annotation subject
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationSubject {
    /// IRI
    IRI(IRI),
    /// Anonymous individual
    AnonymousIndividual(AnonymousIndividual),
}

/// Annotation value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationValue {
    /// IRI
    IRI(IRI),
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

/// Ontology signature containing all entities
#[derive(Debug, Clone, Default)]
pub struct Signature {
    /// All classes in the ontology
    pub classes: Vec<concepts::Class>,
    /// All object properties in the ontology
    pub object_properties: Vec<ObjectProperty>,
    /// All data properties in the ontology
    pub data_properties: Vec<DataProperty>,
    /// All individuals in the ontology
    pub individuals: Vec<individuals::Individual>,
}

impl Signature {
    /// Create a new empty signature
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
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
    /// Next axiom ID
    next_id: u64,
}

impl Ontology {
    /// Create a new empty ontology
    #[must_use]
    pub fn new() -> Self {
        Self {
            axioms: Vec::new(),
            annotations: Vec::new(),
            iri: None,
            version_iri: None,
            imports: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate next axiom ID
    fn next_axiom_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Set the ontology IRI
    pub fn set_iri(&mut self, iri: IRI) {
        self.iri = Some(iri);
    }

    /// Set the ontology IRI (alternative method name used by adapter)
    pub fn set_ontology_iri(&mut self, iri: Option<IRI>) {
        self.iri = iri;
    }

    /// Set the version IRI  
    pub fn set_version_iri(&mut self, iri: Option<IRI>) {
        self.version_iri = iri;
    }

    /// Get the ontology IRI
    #[must_use]
    pub fn get_iri(&self) -> Option<&IRI> {
        self.iri.as_ref()
    }

    /// Add an axiom to the ontology
    pub fn add_axiom(&mut self, axiom: axioms::Axiom) {
        self.axioms.push(axiom);
    }

    /// Remove an axiom from the ontology
    pub fn remove_axiom(&mut self, axiom: &axioms::Axiom) {
        self.axioms.retain(|a| a != axiom);
    }

    /// Get all axioms
    #[must_use]
    pub fn axioms(&self) -> &[axioms::Axiom] {
        &self.axioms
    }

    /// Get the signature of the ontology
    pub fn signature(&self) -> Result<Signature> {
        let mut signature = Signature::new();

        // Helper function to extract classes from class expressions
        fn extract_classes_from_expression(
            expr: &concepts::ClassExpression,
            classes: &mut Vec<concepts::Class>,
        ) {
            match expr {
                concepts::ClassExpression::Class(class) => {
                    if !classes.iter().any(|c| c.iri == class.iri) {
                        classes.push(class.clone());
                    }
                }
                concepts::ClassExpression::ObjectIntersectionOf(exprs)
                | concepts::ClassExpression::ObjectUnionOf(exprs) => {
                    for expr in exprs {
                        extract_classes_from_expression(expr, classes);
                    }
                }
                concepts::ClassExpression::ObjectComplementOf(expr) => {
                    extract_classes_from_expression(expr, classes);
                }
                concepts::ClassExpression::ObjectSomeValuesFrom { filler, .. }
                | concepts::ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                    extract_classes_from_expression(filler, classes);
                }
                _ => {
                    // Handle other expression types as needed
                }
            }
        }

        println!("Computing signature from {} axioms", self.axioms.len());

        // Extract entities from axioms
        for axiom in &self.axioms {
            let discriminant = std::mem::discriminant(axiom);
            println!("Processing axiom discriminant: {discriminant:?}");
            match axiom {
                axioms::Axiom::Declaration(decl) => {
                    match &decl.entity {
                        axioms::Entity::Class(iri) => {
                            signature.classes.push(concepts::Class { iri: iri.clone() });
                            println!("Added class from declaration: {iri}");
                        }
                        axioms::Entity::ObjectProperty(iri) => {
                            // Try to convert to URL, but continue if it fails (for relative IRIs)
                            if let Ok(url) = iri.to_url() {
                                signature
                                    .object_properties
                                    .push(ObjectProperty { iri: url });
                            }
                        }
                        axioms::Entity::DataProperty(iri) => {
                            signature
                                .data_properties
                                .push(DataProperty { iri: iri.clone() });
                        }
                        axioms::Entity::NamedIndividual(iri) => {
                            signature.individuals.push(individuals::Individual::Named(
                                individuals::NamedIndividual { iri: iri.clone() },
                            ));
                        }
                        axioms::Entity::AnnotationProperty(_prop) => {
                            // Handle annotation property
                        }
                        axioms::Entity::Datatype(_datatype) => {
                            // Handle datatype
                        }
                    }
                }
                axioms::Axiom::SubClassOf(axiom) => {
                    println!("Processing SubClassOf axiom");
                    extract_classes_from_expression(&axiom.subclass, &mut signature.classes);
                    extract_classes_from_expression(&axiom.superclass, &mut signature.classes);
                }
                axioms::Axiom::EquivalentClasses(axiom) => {
                    println!(
                        "Processing EquivalentClasses axiom with {} classes",
                        axiom.classes.len()
                    );
                    for class_expr in &axiom.classes {
                        extract_classes_from_expression(class_expr, &mut signature.classes);
                    }
                }
                axioms::Axiom::ClassAssertion(axiom) => {
                    println!("Processing ClassAssertion axiom");
                    extract_classes_from_expression(&axiom.class, &mut signature.classes);
                    // Also add the individual
                    if !signature.individuals.iter().any(|i| match i {
                        individuals::Individual::Named(named) => {
                            named.iri
                                == match &axiom.individual {
                                    individuals::Individual::Named(named) => named.iri.clone(),
                                    _ => return false,
                                }
                        }
                        _ => false,
                    }) {
                        signature.individuals.push(axiom.individual.clone());
                    }
                }
                axioms::Axiom::DisjointUnion(axiom) => {
                    println!("Processing DisjointUnion axiom");
                    extract_classes_from_expression(&axiom.class, &mut signature.classes);
                    for disjoint_class in &axiom.disjoint_classes {
                        extract_classes_from_expression(disjoint_class, &mut signature.classes);
                    }
                }
                // Handle other axiom types as needed
                axiom => {
                    println!(
                        "Processing other axiom type: {:?}",
                        std::mem::discriminant(axiom)
                    );
                }
            }
        }

        println!(
            "Final signature: {} classes, {} individuals",
            signature.classes.len(),
            signature.individuals.len()
        );
        for class in &signature.classes {
            println!("  Class: {}", class.iri);
        }

        Ok(signature)
    }

    /// Load an ontology from a file using horned-owl for robust parsing
    pub fn from_file_with_horned_owl<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        use horned_owl::io::ParserConfiguration;
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(path.as_ref()).map_err(|e| Error::io(e.to_string()))?;
        let mut reader = BufReader::new(file);
        let config = ParserConfiguration::default();

        // Use horned-owl's RDF parser for all file types (most compatible)
        let result = horned_owl::io::rdf::reader::read(&mut reader, config)
            .map_err(|e| Error::ontology_parsing(format!("Horned-owl parsing error: {e}")))?;

        // Convert the horned-owl ontology to oxidowl ontology using simplified approach
        let mut adapter = crate::adapter::HornedOwlAdapter::new();
        adapter.convert_basic_ontology::<std::rc::Rc<str>>(&result.0)
    }

    /// Convert a horned-owl ontology to oxidowl ontology with full SWRL support
    pub fn from_horned_owl_with_swrl<A>(
        horned_ontology: horned_owl::ontology::set::SetOntology<A>,
        _prefix_mapping: curie::PrefixMapping,
    ) -> Result<Self>
    where
        A: horned_owl::model::ForIRI + Clone + std::fmt::Display + std::hash::Hash + Eq,
    {
        let mut adapter = crate::adapter::HornedOwlAdapter::new();
        adapter.convert_ontology_with_swrl::<std::rc::Rc<str>>(&horned_ontology)
    }

    /// Load an ontology from a file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P, format: Option<String>) -> Result<Self> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path.as_ref()).map_err(|e| Error::io(e.to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| Error::io(e.to_string()))?;

        // Parse based on format or file extension
        let format = format.unwrap_or_else(|| {
            path.as_ref()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or_else(|| "owl".to_string(), str::to_lowercase)
        });

        match format.as_str() {
            "owl" | "xml" => {
                // Use OWL XML parser
                crate::parsers::owl_xml::parse(&contents)
            }
            "ttl" | "turtle" => {
                // Use Turtle parser
                crate::parsers::turtle::parse(&contents)
            }
            "rdf" | "rdfxml" => {
                // Use RDF/XML parser
                crate::parsers::rdf_xml::parse(&contents)
            }
            "nt" | "ntriples" => {
                // Use N-Triples parser
                crate::parsers::ntriples::parse(&contents)
            }
            "functional" | "func" | "ofn" => {
                // Use Functional syntax parser
                crate::parsers::functional::parse(&contents)
            }
            _ => {
                // Default to OWL XML
                crate::parsers::owl_xml::parse(&contents)
            }
        }
    }

    /// Add a class (placeholder for compatibility)
    pub fn add_class(&mut self, class: concepts::Class) {
        // This creates a declaration axiom for the class
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: axioms::Entity::Class(class.iri),
        });
        self.add_axiom(axiom);
    }

    /// Add an object property (placeholder for compatibility)
    pub fn add_object_property(&mut self, property: ObjectProperty) {
        // This creates a declaration axiom for the property
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: axioms::Entity::ObjectProperty(property.iri.into()),
        });
        self.add_axiom(axiom);
    }

    /// Add an individual and its declaration axiom
    pub fn add_individual(&mut self, subject: IRI, individual: individuals::Individual) {
        // Add a declaration axiom for the individual
        let declaration = axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: match individual {
                individuals::Individual::Named(ref named) => {
                    axioms::Entity::NamedIndividual(named.iri.clone())
                }
                individuals::Individual::Anonymous(_) => {
                    // Anonymous individuals are not typically declared
                    return;
                }
            },
        };

        self.add_axiom(axioms::Axiom::Declaration(declaration));

        // Also store in internal tracking if needed
        // For now, the axiom storage is sufficient
    }

    /// Get classes by extracting them from declaration axioms
    #[must_use]
    pub fn classes(&self) -> Vec<(IRI, concepts::Class)> {
        let mut classes = Vec::new();

        for axiom in &self.axioms {
            if let axioms::Axiom::Declaration(decl) = axiom {
                if let axioms::Entity::Class(iri) = &decl.entity {
                    let class = concepts::Class { iri: iri.clone() };
                    classes.push((iri.clone(), class));
                }
            }
        }

        classes
    }

    /// Extract individuals from the axioms
    #[must_use]
    pub fn individuals(&self) -> Vec<(IRI, individuals::Individual)> {
        let mut individuals = Vec::new();

        for axiom in &self.axioms {
            match axiom {
                // Extract from declaration axioms
                axioms::Axiom::Declaration(decl) => {
                    if let axioms::Entity::NamedIndividual(iri) = &decl.entity {
                        let individual = individuals::Individual::named(iri.clone());
                        individuals.push((iri.clone(), individual));
                    }
                }
                // Extract from class assertion axioms
                axioms::Axiom::ClassAssertion(assertion) => {
                    let iri = match &assertion.individual {
                        individuals::Individual::Named(named) => &named.iri,
                        individuals::Individual::Anonymous(_) => continue, // Skip anonymous
                    };

                    // Only add if not already present
                    if !individuals
                        .iter()
                        .any(|(existing_iri, _)| existing_iri == iri)
                    {
                        individuals.push((iri.clone(), assertion.individual.clone()));
                    }
                }
                // Extract from object property assertion axioms
                axioms::Axiom::ObjectPropertyAssertion(assertion) => {
                    // Extract source
                    if let individuals::Individual::Named(named) = &assertion.source {
                        if !individuals
                            .iter()
                            .any(|(existing_iri, _)| existing_iri == &named.iri)
                        {
                            individuals.push((named.iri.clone(), assertion.source.clone()));
                        }
                    }

                    // Extract target
                    if let individuals::Individual::Named(named) = &assertion.target {
                        if !individuals
                            .iter()
                            .any(|(existing_iri, _)| existing_iri == &named.iri)
                        {
                            individuals.push((named.iri.clone(), assertion.target.clone()));
                        }
                    }
                }
                _ => {
                    // TODO: Extract individuals from other axiom types as needed
                }
            }
        }

        individuals
    }

    /// Get object properties by extracting them from declaration axioms
    #[must_use]
    pub fn object_properties(&self) -> Vec<ObjectProperty> {
        let mut properties = Vec::new();

        for axiom in &self.axioms {
            if let axioms::Axiom::Declaration(decl) = axiom {
                if let axioms::Entity::ObjectProperty(iri) = &decl.entity {
                    // Need to convert IRI to URL for ObjectProperty
                    if let Ok(url) = iri.to_url() {
                        let property = ObjectProperty { iri: url };
                        properties.push(property);
                    }
                }
            }
        }

        properties
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
    #[must_use]
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
    #[must_use]
    pub fn media_type(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "",
            OntologyFormat::Functional => "text/owl-functional",
            OntologyFormat::OwlXml => "application/owl+xml",
            OntologyFormat::RdfXml => "application/rdf+xml",
            OntologyFormat::Turtle => "text/turtle",
            OntologyFormat::NTriples => "application/n-triples",
            OntologyFormat::Manchester => "text/owl-manchester",
        }
    }

    /// Get the format string for parsing
    #[must_use]
    pub fn format_string(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "auto",
            OntologyFormat::Functional => "functional",
            OntologyFormat::OwlXml => "owl",
            OntologyFormat::RdfXml => "rdf",
            OntologyFormat::Turtle => "ttl",
            OntologyFormat::NTriples => "nt",
            OntologyFormat::Manchester => "omn",
        }
    }

    /// Try to detect format from file extension
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "owx" => Some(OntologyFormat::OwlXml),
            "owl" | "ofn" => Some(OntologyFormat::Functional),
            "rdf" => Some(OntologyFormat::RdfXml),
            "ttl" => Some(OntologyFormat::Turtle),
            "nt" => Some(OntologyFormat::NTriples),
            "omn" => Some(OntologyFormat::Manchester),
            _ => None,
        }
    }
}
