//! JSON-LD Parser and Renderer.

use crate::ontology::{Ontology, IRI};
use crate::ontology::axioms::*;
use crate::Result;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct JsonLdParser;

impl JsonLdParser {
    #[must_use] pub fn new() -> Self { Self }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let val: Value = serde_json::from_str(content).map_err(|e| crate::Error::ParseError(format!("JSON-LD: {e}")))?;

        if let Some(graph) = val.get("@graph").and_then(|v| v.as_array()) {
            for node in graph {
                if let (Some(id), Some(types)) = (node.get("@id").and_then(|v| v.as_str()), node.get("@type")) {
                    let iri = IRI::new(id);
                    if let Some(type_str) = types.as_str() {
                        if type_str.contains("Class") {
                            o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(iri) }));
                        }
                    } else if let Some(type_arr) = types.as_array() {
                        for t in type_arr {
                            if let Some(ts) = t.as_str() {
                                if ts.contains("Class") || ts.contains("ObjectProperty") {
                                    o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(IRI::new(id)) }));
                                }
                            }
                        }
                    }
                    // Handle @value literals as annotations
                    for (key, val) in node.as_object().into_iter().flat_map(|m| m.iter()) {
                        if key == "@id" || key == "@type" || key == "@context" { continue; }
                        if let Some(v) = val.as_str() {
                            o.add_axiom(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                                id: 0,
                                property: crate::ontology::AnnotationProperty { iri: IRI::new(key) },
                                subject: crate::ontology::AnnotationSubject::IRI(IRI::new(id)),
                                value: crate::ontology::AnnotationValue::Literal(crate::ontology::Literal::new(v.into())),
                                annotations: vec![],
                            }));
                        }
                    }
                }
            }
        }
        Ok(o)
    }
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdRenderer;
impl JsonLdRenderer {
    #[must_use] pub fn new() -> Self { Self }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut items = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    items.push(format!(r#"{{"@id":"{iri}","@type":"owl:Class"}}"#));
                }
            }
        }
        Ok(format!("{{\n  \"@graph\": [\n    {}\n  ]\n}}\n", items.join(",\n    ")))
    }
}

pub fn parse(content: &str) -> Result<Ontology> { JsonLdParser::new().parse(content) }
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = JsonLdRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("JSON-LD: {e}")))
}
