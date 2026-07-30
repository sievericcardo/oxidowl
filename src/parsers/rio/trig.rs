//! TriG Parser and Renderer — Turtle with named graphs.

use crate::Result;
use crate::ontology::Ontology;
use crate::parsers::common::OntologySerializer;
use crate::parsers::turtle;

#[derive(Debug, Clone, Default)]
pub struct TriGParser;

impl TriGParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut merged = Ontology::new();
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(merged);
        }

        let rest = self.parse_blocks(trimmed, &mut merged)?;
        let leftover = rest.trim();
        if !leftover.is_empty() {
            let onto = turtle::parse(leftover)?;
            for axiom in onto.axioms() {
                merged.add_axiom(axiom.clone());
            }
        }
        Ok(merged)
    }

    fn parse_blocks<'a>(&self, content: &'a str, merged: &mut Ontology) -> Result<&'a str> {
        let mut rest: &str = content;
        loop {
            let rem = rest.trim();
            if rem.is_empty() {
                return Ok(rem);
            }
            if rem.starts_with("GRAPH") || rem.starts_with("graph") {
                rest = self.parse_graph_block(rem, merged)?;
            } else if rem.starts_with('{') {
                if let Some(end_pos) = Self::find_matching_brace(rem.as_bytes(), 0) {
                    let block = &rem[1..end_pos];
                    let onto = turtle::parse(block)?;
                    for axiom in onto.axioms() {
                        merged.add_axiom(axiom.clone());
                    }
                    rest = &rem[end_pos + 1..];
                } else {
                    rest = &rem[1..];
                }
            } else {
                return Ok(rem);
            }
        }
    }

    fn parse_graph_block<'a>(&self, content: &'a str, _merged: &mut Ontology) -> Result<&'a str> {
        let after_keyword = content[5..].trim_start();
        let rest = if after_keyword.starts_with('<') {
            if let Some(end) = after_keyword.find('>') {
                &after_keyword[end + 1..]
            } else {
                after_keyword
            }
        } else {
            after_keyword
        };

        let trimmed = rest.trim_start();
        if trimmed.starts_with('{') {
            if let Some(end_pos) = Self::find_matching_brace(trimmed.as_bytes(), 0) {
                let block = &trimmed[1..end_pos];
                let _onto = turtle::parse(block)?;
                return Ok(&trimmed[end_pos + 1..]);
            }
        }
        Ok(rest)
    }

    fn find_matching_brace(bytes: &[u8], start: usize) -> Option<usize> {
        let mut depth = 1;
        let mut i = start + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct TriGRenderer;
impl TriGRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let ttl = crate::parsers::turtle::TurtleSerializer::new().serialize(ontology)?;
        Ok(format!("{{ \n{ttl}\n}}"))
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    TriGParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = TriGRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("TriG: {e}")))
}
