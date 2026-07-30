//! TriG Parser and Renderer — Turtle with named graphs.

use crate::ontology::Ontology;
use crate::parsers::turtle;
use crate::parsers::common::OntologySerializer;
use crate::Result;

#[derive(Debug, Clone, Default)]
pub struct TriGParser;

impl TriGParser {
    #[must_use] pub fn new() -> Self { Self }
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let cleaned = content.replace("{\n", "").replace("\n}", "");
        turtle::parse(&cleaned)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TriGRenderer;
impl TriGRenderer {
    #[must_use] pub fn new() -> Self { Self }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let ttl = crate::parsers::turtle::TurtleSerializer::new().serialize(ontology)?;
        Ok(format!("{{ \n{ttl}\n}}"))
    }
}

pub fn parse(content: &str) -> Result<Ontology> { TriGParser::new().parse(content) }
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = TriGRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("TriG: {e}")))
}
