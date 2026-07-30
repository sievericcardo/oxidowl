//! RDFa Parser — extracts RDF from HTML/XML attributes.

use crate::ontology::{Ontology, IRI};
use crate::ontology::axioms::*;
use crate::Result;

#[derive(Debug, Clone, Default)]
pub struct RDFaParser;

impl RDFaParser {
    #[must_use] pub fn new() -> Self { Self }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        // Scan for RDFa attributes: @typeof, @property, @about, @resource, @vocab
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(type_pos) = trimmed.find("typeof=\"") {
                let after = &trimmed[type_pos + 8..];
                if let Some(end) = after.find('\"') {
                    let type_iri = IRI::new(&after[..end]);
                    o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(type_iri) }));
                }
            }
            if let Some(about_pos) = trimmed.find("about=\"") {
                let after = &trimmed[about_pos + 7..];
                if let Some(end) = after.find('\"') {
                    let subject = IRI::new(&after[..end]);
                    o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::NamedIndividual(subject) }));
                }
            }
        }
        Ok(o)
    }
}

pub fn parse(content: &str) -> Result<Ontology> { RDFaParser::new().parse(content) }
