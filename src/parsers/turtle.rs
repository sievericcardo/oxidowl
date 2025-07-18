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

/// Turtle Parser
#[derive(Debug, Clone)]
pub struct TurtleParser {
    // TODO: add a parser configuration
}

impl TurtleParser {
    /// Create a new Turtle parser
    pub fn new() -> Self {
        Self {}
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
        
        // Split content into lines for basic processing
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            
            // Handle prefix declarations
            if trimmed.starts_with("@prefix") {
                self.parse_prefix_declaration(trimmed, &mut prefixes)?;
                continue;
            }
            
            // Handle base declarations
            if trimmed.starts_with("@base") {
                // Parse base declaration (simplified)
                continue;
            }
            
            // Handle basic triple patterns
            if let Ok(triple) = self.parse_triple(trimmed, &prefixes) {
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
        prefixes: &std::collections::HashMap<String, String>
    ) -> Result<Triple> {
        // Very basic triple parsing - splits on whitespace
        let parts: Vec<&str> = line.trim_end_matches('.').split_whitespace().collect();
        
        if parts.len() >= 3 {
            let subject = self.expand_uri(parts[0], prefixes)?;
            let predicate = self.expand_uri(parts[1], prefixes)?;
            let object = if parts[2].starts_with('<') || parts[2].contains(':') {
                TripleObject::Uri(self.expand_uri(parts[2], prefixes)?)
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
        prefixes: &std::collections::HashMap<String, String>
    ) -> Result<String> {
        if uri.starts_with('<') && uri.ends_with('>') {
            // Full URI
            Ok(uri[1..uri.len()-1].to_string())
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
                    _ => Ok(uri.to_string()),
                }
            }
        } else {
            Ok(uri.to_string())
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
                                iri: url::Url::parse(&triple.subject)
                                    .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                            };
                            ontology.add_class(class);
                        }
                        "http://www.w3.org/2002/07/owl#ObjectProperty" => {
                            // Object property declaration
                            let property = crate::ontology::ObjectProperty {
                                iri: url::Url::parse(&triple.subject)
                                    .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                            };
                            ontology.add_object_property(property);
                        }
                        _ => {
                            // Class assertion
                            let individual = crate::ontology::Individual::Named(
                                crate::ontology::NamedIndividual {
                                    iri: url::Url::parse(&triple.subject)
                                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                                        .into(), // Convert URL to IRI
                                }
                            );
                            let class = crate::ontology::Class {
                                iri: url::Url::parse(&class_uri)
                                    .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                            };
                            
                            let axiom = crate::ontology::ClassAssertionAxiom {
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
                        iri: url::Url::parse(&triple.subject)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                    };
                    let superclass = crate::ontology::Class {
                        iri: url::Url::parse(&superclass_uri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                    };
                    
                    let axiom = crate::ontology::SubClassOfAxiom {
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
                        iri: url::Url::parse(&triple.subject)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                    };
                    let superprop = crate::ontology::ObjectProperty {
                        iri: url::Url::parse(&superprop_uri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                    };
                    
                    let axiom = crate::ontology::SubObjectPropertyOfAxiom {
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
                            iri: url::Url::parse(&triple.subject)
                                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                                .into(), // Convert URL to IRI
                        }
                    );
                    let object = crate::ontology::Individual::Named(
                        crate::ontology::NamedIndividual {
                            iri: url::Url::parse(&object_uri)
                                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                                .into(), // Convert URL to IRI
                        }
                    );
                    let property = crate::ontology::ObjectProperty {
                        iri: url::Url::parse(&triple.predicate)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {}", e)))?
                    };
                    
                    let axiom = crate::ontology::ObjectPropertyAssertionAxiom {
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
    for class in ontology.classes() {
        content.push_str(&format!("<{}> rdf:type owl:Class .\n", class));
    }
    content.push_str("\n");
    
    // Write object property declarations
    for prop in ontology.object_properties() {
        content.push_str(&format!("<{}> rdf:type owl:ObjectProperty .\n", prop));
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
                    content.push_str(&format!("<{}> rdf:type <{}> .\n", 
                        assertion.individual.iri, class.iri));
                }
            }
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                if let crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) = &assertion.property {
                    content.push_str(&format!("<{}> <{}> <{}> .\n",
                        assertion.subject.iri, prop.iri, assertion.object.iri));
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
