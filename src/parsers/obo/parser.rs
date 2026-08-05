//! OBO Format Parser — stanza-based parsing for OBO 1.4.

use super::converter::Obo2Owl;
use crate::Result;
use crate::ontology::Ontology;

/// OBO parser configuration.
#[derive(Debug, Clone)]
pub struct OBOParserConfig {
    pub strict: bool,
    pub allow_dangling_references: bool,
    pub resolve_xrefs: bool,
}

impl Default for OBOParserConfig {
    fn default() -> Self {
        Self {
            strict: true,
            allow_dangling_references: false,
            resolve_xrefs: false,
        }
    }
}

/// A single OBO stanza (tag-value block) with its type.
#[derive(Debug, Clone, Default)]
pub struct OBOStanza {
    pub stanza_type: String,
    pub tags: Vec<(String, String)>,
    pub raw_lines: Vec<String>,
}

/// Parses OBO format content into stanzas and converts to OWL.
#[derive(Debug, Clone, Default)]
pub struct OBOParser {
    #[allow(dead_code)]
    config: OBOParserConfig,
}

impl OBOParser {
    #[must_use]
    pub fn new(config: OBOParserConfig) -> Self {
        Self { config }
    }

    /// Parse OBO content into an OWL ontology.
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let stanzas = self.parse_stanzas(content);
        let converter = Obo2Owl::new();
        converter.convert_stanzas(&stanzas)
    }

    /// Parse OBO content into individual stanzas.
    fn parse_stanzas(&self, content: &str) -> Vec<OBOStanza> {
        let mut stanzas = Vec::new();
        let mut current: Option<OBOStanza> = None;
        let _tag_buf = String::new();
        let _value_buf = String::new();
        let _in_value = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('!') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if let Some(stanza) = current.take()
                    && !stanza.tags.is_empty()
                {
                    stanzas.push(stanza);
                }
                let stanza_type = trimmed[1..trimmed.len() - 1].trim().to_string();
                current = Some(OBOStanza {
                    stanza_type,
                    tags: Vec::new(),
                    raw_lines: Vec::new(),
                });
                continue;
            }

            if let Some(ref mut stanza) = current {
                stanza.raw_lines.push(line.to_string());
                // Parse tag: value pairs
                if let Some(colon_pos) = trimmed.find(':') {
                    let tag = trimmed[..colon_pos].trim().to_string();
                    let value = trimmed[colon_pos + 1..].trim().to_string();
                    if !tag.is_empty() {
                        stanza.tags.push((tag, value));
                    }
                }
            }
        }

        if let Some(stanza) = current
            && !stanza.tags.is_empty()
        {
            stanzas.push(stanza);
        }
        stanzas
    }
}

/// Public parse entry point.
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = OBOParser::default();
    parser.parse(content)
}
