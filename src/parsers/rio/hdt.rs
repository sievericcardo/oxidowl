//! HDT (Header-Dictionary-Triples) Parser and Renderer.
//! Compressed RDF format designed for large datasets (billions of triples).

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{IRI, Ontology};
use std::collections::HashMap;

/// HDT Header section containing metadata key-value pairs.
#[derive(Debug, Clone, Default)]
struct HDTHeader {
    metadata: HashMap<String, String>,
}

impl HDTHeader {
    fn parse(content: &str) -> Self {
        let mut header = HDTHeader::default();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(eq) = trimmed.find('=') {
                let key = trimmed[..eq].trim().to_string();
                let val = trimmed[eq + 1..].trim().trim_matches('"').to_string();
                header.metadata.insert(key, val);
            }
        }
        header
    }
}

/// HDT Dictionary section containing shared string tables.
#[derive(Debug, Clone, Default)]
struct HDTDictionary {
    subjects: Vec<String>,
    predicates: Vec<String>,
    objects: Vec<String>,
}

impl HDTDictionary {
    #[allow(dead_code)]
    fn parse(_header_lines: &[&str], content: &str, section_name: &str) -> Vec<String> {
        let marker = format!("{section_name}:");
        let mut entries = Vec::new();
        let mut in_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == marker {
                in_section = true;
                continue;
            }
            if trimmed.starts_with("triples:") {
                break;
            }
            if in_section && !trimmed.is_empty() {
                entries.push(trimmed.to_string());
            }
        }
        entries
    }
}

#[derive(Debug, Clone, Default)]
pub struct HDTParser;

impl HDTParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();

        // Parse HDT sections: header, dictionary, triples
        if !content.starts_with("$HDT") {
            // Fallback: plain text line-by-line parsing
            return self.parse_plain(content);
        }

        let parts: Vec<&str> = content.split("\n\n").collect();

        // Parse header
        let _header = if let Some(hdr_part) = parts.first() {
            HDTHeader::parse(hdr_part)
        } else {
            HDTHeader::default()
        };

        // Parse dictionary
        let mut dict = HDTDictionary::default();
        let sections: Vec<&str> = content.split("\ndictionary:").collect();
        if sections.len() >= 2 {
            let dict_triples = sections[1].split("\ntriples:").collect::<Vec<_>>();
            if !dict_triples.is_empty() {
                let dict_content = dict_triples[0];
                dict.subjects = Self::parse_dict_section(dict_content, "subjects");
                dict.predicates = Self::parse_dict_section(dict_content, "predicates");
                dict.objects = Self::parse_dict_section(dict_content, "objects");
            }
        }

        // Parse triples as (s_id, p_id, o_id) integer triples referencing dictionary
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 3
                && let (Ok(s_idx), Ok(_p_idx), Ok(_o_idx)) = (
                    parts[0].parse::<usize>(),
                    parts[1].parse::<usize>(),
                    parts[2].parse::<usize>(),
                )
            {
                let subject = if s_idx < dict.subjects.len() {
                    dict.subjects[s_idx].clone()
                } else {
                    format!("urn:hdt:subject:{s_idx}")
                };
                let iri = IRI::new(&subject);
                o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                    id: 0,
                    entity: Entity::Class(iri),
                }));
            }
        }

        Ok(o)
    }

    fn parse_dict_section(content: &str, section: &str) -> Vec<String> {
        let marker = format!("{section}:");
        let mut entries = Vec::new();
        let mut in_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == marker {
                in_section = true;
                continue;
            }
            if trimmed.is_empty()
                || trimmed.starts_with("subjects:")
                || trimmed.starts_with("predicates:")
                || trimmed.starts_with("objects:")
            {
                if !trimmed.is_empty() && !trimmed.starts_with(&marker) {
                    break;
                }
                continue;
            }
            if in_section {
                entries.push(trimmed.to_string());
            }
        }
        entries
    }

    fn parse_plain(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let s = parts[0].trim_matches(|c| c == '<' || c == '>');
                o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                    id: 0,
                    entity: Entity::Class(IRI::new(s)),
                }));
            }
        }
        Ok(o)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HDTRenderer;
impl HDTRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::from("$HDT\n");
        let axioms: Vec<_> = ontology
            .axioms()
            .iter()
            .filter_map(|a| {
                if let Axiom::Declaration(d) = a {
                    Some(d.entity.iri().to_string())
                } else {
                    None
                }
            })
            .collect();

        // Header
        buf.push_str(&format!("triples={}\n", axioms.len()));
        buf.push('\n');

        // Dictionary
        let mut dict: Vec<String> = axioms.clone();
        dict.sort();
        dict.dedup();
        buf.push_str("dictionary:\n");
        buf.push_str("subjects:\n");
        for entry in &dict {
            buf.push_str(&format!("  {entry}\n"));
        }
        buf.push_str("predicates:\n  rdf:type\n");
        buf.push_str("objects:\n  owl:Class\n");
        buf.push('\n');

        // Triples
        buf.push_str("triples:\n");
        for iri_str in &axioms {
            if let Some(idx) = dict.iter().position(|d| d == iri_str) {
                buf.push_str(&format!("{idx} 0 0\n"));
            }
        }
        Ok(buf)
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    HDTParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = HDTRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("HDT: {e}")))
}
