//! RDF/JSON Parser and Renderer.

use crate::ontology::{Ontology, IRI};
use crate::ontology::axioms::*;
use crate::Result;
use serde_json::Value;
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
pub struct RdfJsonParser;

impl RdfJsonParser {
    #[must_use] pub fn new() -> Self { Self }
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let val: Value = serde_json::from_str(content).map_err(|e| crate::Error::ParseError(format!("RDF/JSON: {e}")))?;
        if let Some(obj) = val.as_object() {
            for (subject, predicates) in obj {
                if let Some(pred_obj) = predicates.as_object() {
                    for (_predicate, objects) in pred_obj {
                        if let Some(arr) = objects.as_array() {
                            for obj_val in arr {
                                if let Some(_v) = obj_val.get("value").and_then(|v| v.as_str()) {
                                    let ty = obj_val.get("type").and_then(|v| v.as_str()).unwrap_or("literal");
                                    match ty {
                                        "uri" => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(IRI::new(subject)) }));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(o)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RdfJsonRenderer;
impl RdfJsonRenderer {
    #[must_use] pub fn new() -> Self { Self }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::from("{\n");
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let _ = write!(buf, r#"  "{iri}": {{ "http://www.w3.org/1999/02/22-rdf-syntax-ns#type": [ {{ "type": "uri", "value": "http://www.w3.org/2002/07/owl#Class" }} ] }},"#);
                }
            }
        }
        buf.push_str("\n}\n");
        Ok(buf)
    }
}

pub fn parse(content: &str) -> Result<Ontology> { RdfJsonParser::new().parse(content) }
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = RdfJsonRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("RDF/JSON: {e}")))
}
