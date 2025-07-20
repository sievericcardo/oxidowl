//! Turtle Parser
//!
//! This module implements parsing of OWL 2 ontologies from Turtle format.

use crate::{
    Error, Result,
    ontology::{Ontology, ClassExpression, Individual, NamedIndividual, IRI},
};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Parse an IRI from a string, handling both absolute and relative URIs
fn parse_iri_to_url(uri_str: &str) -> Result<url::Url> {
    // Try to parse as absolute URL first
    if let Ok(url) = url::Url::parse(uri_str) {
        Ok(url)
    } else {
        // If it's not a valid absolute URL, treat it as a simple IRI string
        // This handles relative URIs and other IRI formats
        if uri_str.is_empty() {
            Err(Error::ontology_parsing("Empty IRI string"))
        } else {
            // Create a simple URL-like structure for the IRI
            // Use a dummy base for relative URIs to make them parseable
            let full_uri = if uri_str.starts_with("http://") || uri_str.starts_with("https://") {
                uri_str.to_string()
            } else {
                format!("http://example.org/{}", uri_str.trim_start_matches('#'))
            };
            
            url::Url::parse(&full_uri)
                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))
        }
    }
}

/// Configuration for the Turtle parser
#[derive(Debug, Clone)]
pub struct TurtleParserConfig {
    /// Whether to allow relative IRIs (default: true)
    pub allow_relative_iris: bool,
    
    /// Whether to validate IRIs during parsing (default: true)
    pub validate_iris: bool,
    
    /// Whether to ignore comments (default: true)
    pub ignore_comments: bool,
    
    /// Whether to allow blank node labels (default: true)
    pub allow_blank_nodes: bool,
    
    /// Maximum prefix resolution depth (default: 10)
    pub max_prefix_depth: usize,
    
    /// Whether to perform strict Turtle compliance checking (default: false)
    pub strict_mode: bool,
}

impl Default for TurtleParserConfig {
    fn default() -> Self {
        Self {
            allow_relative_iris: true,
            validate_iris: true,
            ignore_comments: true,
            allow_blank_nodes: true,
            max_prefix_depth: 10,
            strict_mode: false,
        }
    }
}

/// Turtle Parser
#[derive(Debug, Clone)]
pub struct TurtleParser {
    config: TurtleParserConfig,
}

impl TurtleParser {
    /// Create a new Turtle parser with default configuration
    pub fn new() -> Self {
        Self { 
            config: TurtleParserConfig::default(),
        }
    }
    
    /// Create a new Turtle parser with custom configuration
    pub fn with_config(config: TurtleParserConfig) -> Self {
        Self { config }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &TurtleParserConfig {
        &self.config
    }
    
    /// Set a new configuration
    pub fn set_config(&mut self, config: TurtleParserConfig) {
        self.config = config;
    }
}

impl Default for TurtleParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Turtle from string content
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = TurtleParser::new();
    parser.parse_string(content)
}

impl TurtleParser {
    /// Parse Turtle content into an ontology
    pub fn parse_string(&self, content: &str) -> Result<Ontology> {
        let mut ontology = Ontology::new();
        let mut prefixes = std::collections::HashMap::<String, String>::new();
        let mut base_uri: Option<String> = None;
        
        // Split content into lines for basic processing
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip empty lines 
            if trimmed.is_empty() {
                continue;
            }
            
            // Skip comments if configured to do so
            if trimmed.starts_with('#') {
                if self.config.ignore_comments {
                    continue;
                } else {
                    // In strict mode, we might want to preserve comments for validation
                    if self.config.strict_mode {
                        // TODO: Store comment for validation purposes
                    }
                    continue;
                }
            }
            
            // Handle prefix declarations
            if trimmed.starts_with("@prefix") {
                self.parse_prefix_declaration(trimmed, &mut prefixes)?;
                continue;
            }
            
            // Handle base declarations
            if trimmed.starts_with("@base") {
                if let Some(start) = trimmed.find('<') {
                    if let Some(end) = trimmed.find('>') {
                        base_uri = Some(trimmed[start+1..end].to_string());
                    }
                }
                continue;
            }
            
            // Handle basic triple patterns
            if let Ok(triple) = self.parse_triple(trimmed, &prefixes, &base_uri) {
                self.process_triple(&mut ontology, triple)?;
            }
        }
        
        Ok(ontology)
    }
    
    /// Parse a prefix declaration
    fn parse_prefix_declaration(
        &self, 
        line: &str, 
        prefixes: &mut std::collections::HashMap<String, String>
    ) -> Result<()> {
        // Basic prefix parsing: @prefix prefix: <uri> .
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let prefix_name = parts[1].trim_end_matches(':');
            let uri = parts[2].trim_matches(['<', '>', '.'].as_ref());
            prefixes.insert(prefix_name.to_string(), uri.to_string());
        }
        Ok(())
    }
    
    /// Parse a basic triple
    fn parse_triple(
        &self, 
        line: &str, 
        prefixes: &std::collections::HashMap<String, String>,
        base_uri: &Option<String>
    ) -> Result<Triple> {
        // Very basic triple parsing - splits on whitespace
        let parts: Vec<&str> = line.trim_end_matches('.').split_whitespace().collect();
        
        if parts.len() >= 3 {
            let subject = self.expand_uri(parts[0], prefixes, base_uri)?;
            let predicate = self.expand_uri(parts[1], prefixes, base_uri)?;
            let object = if parts[2].starts_with('<') || parts[2].contains(':') {
                TripleObject::Uri(self.expand_uri(parts[2], prefixes, base_uri)?)
            } else {
                TripleObject::Literal(parts[2].to_string())
            };
            
            Ok(Triple { subject, predicate, object })
        } else {
            Err(Error::ontology_parsing("Invalid triple format"))
        }
    }
    
    /// Expand a prefixed URI
    fn expand_uri(
        &self, 
        uri: &str, 
        prefixes: &std::collections::HashMap<String, String>,
        base_uri: &Option<String>
    ) -> Result<String> {
        if uri.starts_with('<') && uri.ends_with('>') {
            // Full URI in angle brackets
            let inner_uri = &uri[1..uri.len()-1];
            
            // Check if it's a relative URI that needs base resolution
            if !inner_uri.contains("://") && !inner_uri.starts_with("http") {
                if let Some(base) = base_uri {
                    Ok(format!("{}{}", base, inner_uri))
                } else {
                    Ok(inner_uri.to_string())
                }
            } else {
                Ok(inner_uri.to_string())
            }
        } else if let Some(colon_pos) = uri.find(':') {
            // Prefixed URI
            let prefix = &uri[..colon_pos];
            let local = &uri[colon_pos+1..];
            
            if let Some(base) = prefixes.get(prefix) {
                Ok(format!("{}{}", base, local))
            } else {
                // Default prefixes
                match prefix {
                    "rdf" => Ok(format!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", local)),
                    "rdfs" => Ok(format!("http://www.w3.org/2000/01/rdf-schema#{}", local)),
                    "owl" => Ok(format!("http://www.w3.org/2002/07/owl#{}", local)),
                    _ => {
                        // If we have a base URI and this looks like a relative reference
                        if let Some(base) = base_uri {
                            Ok(format!("{}{}", base, uri))
                        } else {
                            Ok(uri.to_string())
                        }
                    }
                }
            }
        } else {
            // Bare local name - use base URI if available
            if let Some(base) = base_uri {
                Ok(format!("{}{}", base, uri))
            } else {
                Ok(uri.to_string())
            }
        }
    }
    
    /// Process a parsed triple into the ontology
    fn process_triple(&self, ontology: &mut Ontology, triple: Triple) -> Result<()> {
        match triple.predicate.as_str() {
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" => {
                if let TripleObject::Uri(class_uri) = triple.object {
                    match class_uri.as_str() {
                        "http://www.w3.org/2002/07/owl#Class" => {
                            // Class declaration
                            let class = crate::ontology::Class {
                                iri: parse_iri_to_url(&triple.subject)?.into()
                            };
                            ontology.add_class(class);
                        }
                        "http://www.w3.org/2002/07/owl#ObjectProperty" => {
                            // Object property declaration
                            let property = crate::ontology::ObjectProperty {
                                iri: parse_iri_to_url(&triple.subject)?
                            };
                            ontology.add_object_property(property);
                        }
                        _ => {
                            // Class assertion
                            let individual = crate::ontology::Individual::Named(
                                crate::ontology::NamedIndividual {
                                    iri: parse_iri_to_url(&triple.subject)?.into()
                                }
                            );
                            let class = crate::ontology::Class {
                                iri: parse_iri_to_url(&class_uri)?.into()
                            };
                            
                            let axiom = crate::ontology::ClassAssertionAxiom {
                                id: generate_axiom_id(),
                                individual,
                                class: ClassExpression::Class(class),
                                annotations: vec![],
                            };
                            ontology.add_axiom(crate::ontology::Axiom::ClassAssertion(axiom));
                        }
                    }
                }
            }
            "http://www.w3.org/2000/01/rdf-schema#subClassOf" => {
                if let TripleObject::Uri(superclass_uri) = triple.object {
                    let subclass = crate::ontology::Class {
                        iri: parse_iri_to_url(&triple.subject)?.into()
                    };
                    let superclass = crate::ontology::Class {
                        iri: parse_iri_to_url(&superclass_uri)?.into()
                    };
                    
                    let axiom = crate::ontology::SubClassOfAxiom {
                        id: generate_axiom_id(),
                        subclass: ClassExpression::Class(subclass),
                        superclass: ClassExpression::Class(superclass),
                        annotations: vec![],
                    };
                    ontology.add_axiom(crate::ontology::Axiom::SubClassOf(axiom));
                }
            }
            "http://www.w3.org/2000/01/rdf-schema#subPropertyOf" => {
                if let TripleObject::Uri(superprop_uri) = triple.object {
                    let subprop = crate::ontology::ObjectProperty {
                        iri: parse_iri_to_url(&triple.subject)?
                    };
                    let superprop = crate::ontology::ObjectProperty {
                        iri: parse_iri_to_url(&superprop_uri)?
                    };
                    
                    let axiom = crate::ontology::SubObjectPropertyOfAxiom {
                        id: generate_axiom_id(),
                        sub_property: crate::ontology::ObjectPropertyExpression::ObjectProperty(subprop),
                        super_property: crate::ontology::ObjectPropertyExpression::ObjectProperty(superprop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(crate::ontology::Axiom::SubObjectPropertyOf(axiom));
                }
            }
            _ => {
                // Handle other property assertions
                if let TripleObject::Uri(object_uri) = triple.object {
                    let subject = crate::ontology::Individual::Named(
                        crate::ontology::NamedIndividual {
                            iri: parse_iri_to_url(&triple.subject)?.into()
                        }
                    );
                    let object = crate::ontology::Individual::Named(
                        crate::ontology::NamedIndividual {
                            iri: parse_iri_to_url(&object_uri)?.into()
                        }
                    );
                    let property = crate::ontology::ObjectProperty {
                        iri: parse_iri_to_url(&triple.predicate)?
                    };
                    
                    let axiom = crate::ontology::ObjectPropertyAssertionAxiom {
                        id: generate_axiom_id(),
                        property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                        source: subject,
                        target: object,
                        annotations: vec![],
                    };
                    ontology.add_axiom(crate::ontology::Axiom::ObjectPropertyAssertion(axiom));
                }
            }
        }
        
        Ok(())
    }
}

/// Represents a parsed RDF triple
#[derive(Debug, Clone)]
struct Triple {
    subject: String,
    predicate: String,
    object: TripleObject,
}

/// Object part of an RDF triple
#[derive(Debug, Clone)]
enum TripleObject {
    Uri(String),
    Literal(String),
}

/// Parse Turtle from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path)
        .map_err(|e| Error::io(format!("Failed to open file: {}", e)))?;
    
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {}", e)))?;
    
    parse(&content)
}

/// Save ontology to Turtle file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let mut content = String::new();
    
    // Add standard prefixes
    content.push_str("@prefix : <http://example.org/> .\n");
    content.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
    content.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
    content.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    content.push_str("\n");
    
    // Write ontology declaration
    if let Some(iri) = ontology.get_iri() {
        content.push_str(&format!("<{}> rdf:type owl:Ontology .\n\n", iri));
    }
    
    // Write class declarations
    for (iri, _class) in ontology.classes() {
        content.push_str(&format!("<{}> rdf:type owl:Class .\n", iri));
    }
    content.push_str("\n");
    
    // Write object property declarations
    for prop in ontology.object_properties() {
        content.push_str(&format!("<{}> rdf:type owl:ObjectProperty .\n", prop.iri));
    }
    content.push_str("\n");
    
    // Write axioms (basic serialization)
    for axiom in ontology.axioms() {
        match axiom {
            crate::ontology::Axiom::SubClassOf(sub) => {
                if let (ClassExpression::Class(subclass), ClassExpression::Class(superclass)) = 
                    (&sub.subclass, &sub.superclass) {
                    content.push_str(&format!("<{}> rdfs:subClassOf <{}> .\n", 
                        subclass.iri, superclass.iri));
                }
            }
            crate::ontology::Axiom::ClassAssertion(assertion) => {
                if let ClassExpression::Class(class) = &assertion.class {
                    if let Some(individual_iri) = assertion.individual.iri() {
                        content.push_str(&format!("<{}> rdf:type <{}> .\n", 
                            individual_iri, class.iri));
                    }
                }
            }
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                if let crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) = &assertion.property {
                    if let (Some(source_iri), Some(target_iri)) = (assertion.source.iri(), assertion.target.iri()) {
                        content.push_str(&format!("<{}> <{}> <{}> .\n",
                            source_iri, prop.iri, target_iri));
                    }
                }
            }
            _ => {
                // Skip complex axioms for now
            }
        }
    }
    
    std::fs::write(path, content)
        .map_err(|e| Error::io(format!("Failed to write file: {}", e)))
}
