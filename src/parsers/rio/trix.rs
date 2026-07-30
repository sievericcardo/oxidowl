//! TriX Parser and Renderer — XML-based RDF with named graphs.

use crate::ontology::{Ontology, IRI};
use crate::ontology::axioms::*;
use crate::Result;
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
pub struct TriXParser;

impl TriXParser {
    #[must_use] pub fn new() -> Self { Self }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let doc = roxmltree::Document::parse(content).map_err(|e| crate::Error::ParseError(format!("TriX: {e}")))?;
        for graph_node in doc.descendants().filter(|n| n.has_tag_name("graph")) {
            for triple_node in graph_node.descendants().filter(|n| n.has_tag_name("triple")) {
                let uris: Vec<String> = triple_node.descendants()
                    .filter(|n| n.has_tag_name("uri"))
                    .filter_map(|n| n.text().map(|s| s.to_string()))
                    .collect();
                if uris.len() >= 3 {
                    o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(IRI::new(&format!("urn:triple:{}_{}", uris[0], uris[1]))) }));
                }
            }
        }
        Ok(o)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TriXRenderer;
impl TriXRenderer {
    #[must_use] pub fn new() -> Self { Self }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::from("<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\">\n  <graph>\n");
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let _ = write!(buf, "    <triple>\n      <uri>{iri}</uri>\n      <uri>http://www.w3.org/1999/02/22-rdf-syntax-ns#type</uri>\n      <uri>http://www.w3.org/2002/07/owl#Class</uri>\n    </triple>\n");
                }
            }
        }
        buf.push_str("  </graph>\n</TriX>\n");
        Ok(buf)
    }
}

pub fn parse(content: &str) -> Result<Ontology> { TriXParser::new().parse(content) }
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = TriXRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("TriX: {e}")))
}
