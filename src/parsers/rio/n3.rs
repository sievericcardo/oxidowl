//! Notation3 (N3) Parser and Renderer.
//! N3 is a superset of Turtle with logical rules.

use crate::Result;
use crate::ontology::Ontology;
use crate::parsers::common::OntologySerializer;
use crate::parsers::turtle;

#[derive(Debug, Clone, Default)]
pub struct N3Parser;

impl N3Parser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        // Parse as Turtle (N3 is mostly compatible)
        turtle::parse(content)
    }
}

#[derive(Debug, Clone, Default)]
pub struct N3Renderer;
impl N3Renderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        turtle::TurtleSerializer::new().serialize(ontology)
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    N3Parser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = N3Renderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("N3: {e}")))
}
