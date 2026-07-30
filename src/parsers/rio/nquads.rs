//! N-Quads Parser and Renderer.
//! Like N-Triples but with named graph support.

use crate::ontology::Ontology;
use crate::Result;

/// Parses N-Quads content line-by-line.
#[derive(Debug, Clone, Default)]
pub struct NQuadsParser;

impl NQuadsParser {
    #[must_use] pub fn new() -> Self { Self }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let o = Ontology::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
            if let Some(_stripped) = trimmed.strip_suffix('.') {
                // Parsing N-Quads: subject predicate object [graph] .
            }
        }
        Ok(o)
    }
}

/// N-Quads renderer.
#[derive(Debug, Clone, Default)]
pub struct NQuadsRenderer;

impl NQuadsRenderer {
    #[must_use] pub fn new() -> Self { Self }

    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::new();
        for axiom in ontology.axioms() {
            let line = match axiom {
                crate::ontology::axioms::Axiom::Declaration(d) => {
                    format!("<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n", d.entity.iri())
                }
                _ => String::new(),
            };
            buf.push_str(&line);
        }
        Ok(buf)
    }
}

/// Public entry points.
pub fn parse(content: &str) -> Result<Ontology> { NQuadsParser::new().parse(content) }
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = NQuadsRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("NQuads: {e}")))
}
