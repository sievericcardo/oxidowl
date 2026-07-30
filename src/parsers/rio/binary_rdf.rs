//! Binary RDF Parser and Renderer.
//! Compact binary serialization format for RDF data.

use crate::Result;
use crate::ontology::Ontology;

#[derive(Debug, Clone, Default)]
pub struct BinaryRdfParser;

impl BinaryRdfParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, data: &[u8]) -> Result<Ontology> {
        let mut o = Ontology::new();
        if data.len() < 4 {
            return Ok(o);
        }

        // Parse BRDF header: magic "BRDF" + varint triple count
        if &data[..4] != b"BRDF" {
            // Fallback: treat as UTF-8 wrapped triples
            return self.parse_text(data);
        }

        let mut pos = 4;
        let triple_count = self.read_varint(data, &mut pos);
        let dict_size = self.read_varint(data, &mut pos);

        // Read dictionary (string table)
        let mut dict = Vec::with_capacity(dict_size as usize);
        for _ in 0..dict_size as usize {
            let len = self.read_varint(data, &mut pos) as usize;
            if pos + len > data.len() {
                break;
            }
            dict.push(String::from_utf8_lossy(&data[pos..pos + len]).to_string());
            pos += len;
        }

        // Read triples as (s,p,o) integer IDs referencing dictionary
        for _ in 0..triple_count as usize {
            if pos + 6 > data.len() {
                break;
            }
            let s = self.read_varint(data, &mut pos) as usize;
            let p = self.read_varint(data, &mut pos) as usize;
            let o_idx = self.read_varint(data, &mut pos) as usize;
            if s < dict.len() && p < dict.len() && o_idx < dict.len() {
                let iri = crate::ontology::IRI::new(&dict[s]);
                o.add_axiom(crate::ontology::axioms::Axiom::Declaration(
                    crate::ontology::axioms::DeclarationAxiom {
                        id: 0,
                        entity: crate::ontology::axioms::Entity::Class(iri),
                    },
                ));
            }
        }
        Ok(o)
    }

    fn parse_text(&self, data: &[u8]) -> Result<Ontology> {
        let mut o = Ontology::new();
        if let Ok(s) = std::str::from_utf8(data) {
            for line in s.lines() {
                if let Some(stripped) = line.trim().strip_suffix('.') {
                    let parts: Vec<&str> = stripped.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let iri = crate::ontology::IRI::new(
                            parts[0].trim_matches(|c| c == '<' || c == '>'),
                        );
                        o.add_axiom(crate::ontology::axioms::Axiom::Declaration(
                            crate::ontology::axioms::DeclarationAxiom {
                                id: 0,
                                entity: crate::ontology::axioms::Entity::Class(iri),
                            },
                        ));
                    }
                }
            }
        }
        Ok(o)
    }

    fn read_varint(&self, data: &[u8], pos: &mut usize) -> u64 {
        let mut result: u64 = 0;
        let mut shift = 0;
        while *pos < data.len() && shift < 64 {
            let byte = data[*pos];
            *pos += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        result
    }
}

#[derive(Debug, Clone, Default)]
pub struct BinaryRdfRenderer;
impl BinaryRdfRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        // Header
        buf.extend_from_slice(b"BRDF");

        let axioms: Vec<_> = ontology
            .axioms()
            .iter()
            .filter_map(|a| {
                if let crate::ontology::axioms::Axiom::Declaration(d) = a {
                    Some(d.entity.iri().to_string())
                } else {
                    None
                }
            })
            .collect();

        // Write varint: triple count = axiom count * 3 (one IRI per position)
        let count = axioms.len() as u64;
        Self::write_varint(&mut buf, count);

        // Build dictionary from unique IRIs
        let mut dict: Vec<String> = axioms.clone();
        dict.sort();
        dict.dedup();
        Self::write_varint(&mut buf, dict.len() as u64);
        for entry in &dict {
            Self::write_varint(&mut buf, entry.len() as u64);
            buf.extend_from_slice(entry.as_bytes());
        }

        // Write triples as dictionary indices
        for iri_str in &axioms {
            if let Some(idx) = dict.iter().position(|d| d == iri_str) {
                Self::write_varint(&mut buf, idx as u64);
                Self::write_varint(&mut buf, 0); // predicate placeholder
                Self::write_varint(&mut buf, 0); // object placeholder
            }
        }
        buf
    }

    fn write_varint(buf: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            buf.push((value as u8) | 0x80);
            value >>= 7;
        }
        buf.push(value as u8);
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    BinaryRdfParser::new().parse(content.as_bytes())
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = BinaryRdfRenderer::new().serialize(ontology);
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("BinaryRDF: {e}")))
}
