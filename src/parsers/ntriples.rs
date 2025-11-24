//! N-Triples Parser
//!
//! This module implements parsing of OWL 2 ontologies from N-Triples format.

use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use super::common::OntologySerializer;
use crate::{
    Error, Result,
    ontology::{IRI, Ontology},
};

/// N-Triples Parser
#[derive(Debug, Clone)]
pub struct NTriplesParser {
    // Parser configuration could go here
}

impl NTriplesParser {
    /// Create a new N-Triples parser
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NTriplesParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse N-Triples from string content
pub fn parse(content: &str) -> Result<Ontology> {
    let mut ontology = Ontology::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue; // Skip empty lines and comments
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(Error::ontology_parsing(
                "Invalid N-Triples format".to_string(),
            ));
        }

        let subject = IRI::from(parts[0].to_string());
        let predicate = IRI::from(parts[1].to_string());
        let object = parts[2];

        if predicate.as_str() == "rdf:type" {
            if object.starts_with('<') && object.ends_with('>') {
                let class = crate::ontology::Class {
                    iri: IRI::from(object.to_string()),
                };
                ontology.add_class(class);
            } else if object.starts_with("_:") {
                let individual =
                    crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                        iri: IRI::from(object.to_string()),
                    });
                ontology.add_individual(subject, individual);
            }
        } else {
            // Handle other predicates as needed
        }
    }

    Ok(ontology)
}

/// Parse N-Triples from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path).map_err(|e| Error::io(format!("Failed to open file: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse(&content)
}

/// N-Triples format serializer implementing the common serialization interface
#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl NTriplesSerializer {
    /// Create a new NTriplesSerializer instance
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl OntologySerializer for NTriplesSerializer {
    fn serialize(&self, ontology: &Ontology) -> std::result::Result<String, Error> {
        let mut content = String::new();

        for (subject, class) in ontology.classes() {
            content.push_str(&format!("{} rdf:type {} .\n", subject, class.iri));
        }

        for (subject, individual) in ontology.individuals() {
            if let Some(iri) = individual.iri() {
                content.push_str(&format!("{iri} rdf:type Individual .\n"));
            }
        }

        Ok(content)
    }
}

/// Save ontology to N-Triples file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = NTriplesSerializer::new();
    serializer.serialize_to_file(ontology, path)
}
