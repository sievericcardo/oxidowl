//! RDF/JSON Parser and Renderer.

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{IRI, Literal, NamedIndividual, Ontology};
use crate::ontology::{
    Class, DataProperty, Individual, ObjectProperty, ObjectPropertyExpression,
    DataPropertyExpression,
};
use serde_json::Value;
use std::fmt::Write;

fn axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DataProperty";
const OWL_NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";

fn parse_datatype(dt_str: &str) -> Option<url::Url> {
    if dt_str.is_empty() {
        return None;
    }
    url::Url::parse(dt_str).ok()
}

#[derive(Debug, Clone, Default)]
pub struct RdfJsonParser;

impl RdfJsonParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let val: Value = serde_json::from_str(content)
            .map_err(|e| crate::Error::ParseError(format!("RDF/JSON: {e}")))?;
        if let Some(obj) = val.as_object() {
            for (subject, predicates) in obj {
                if let Some(pred_obj) = predicates.as_object() {
                    for (predicate, objects) in pred_obj {
                        if let Some(arr) = objects.as_array() {
                            for obj_val in arr {
                                let value = obj_val
                                    .get("value")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let ty = obj_val
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("literal");
                                let lang = obj_val
                                    .get("lang")
                                    .and_then(|v| v.as_str())
                                    .map(String::from);
                                let datatype = obj_val
                                    .get("datatype")
                                    .and_then(|v| v.as_str())
                                    .and_then(parse_datatype);

                                if predicate == RDF_TYPE {
                                    match value {
                                        OWL_CLASS => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                                                id: axiom_id(),
                                                entity: Entity::Class(IRI::new(subject)),
                                            }));
                                        }
                                        OWL_OBJECT_PROPERTY => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                                                id: axiom_id(),
                                                entity: Entity::ObjectProperty(IRI::new(subject)),
                                            }));
                                        }
                                        OWL_DATA_PROPERTY => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                                                id: axiom_id(),
                                                entity: Entity::DataProperty(IRI::new(subject)),
                                            }));
                                        }
                                        OWL_NAMED_INDIVIDUAL => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                                                id: axiom_id(),
                                                entity: Entity::NamedIndividual(IRI::new(subject)),
                                            }));
                                        }
                                        _ => {
                                            let individual = Individual::Named(NamedIndividual {
                                                iri: IRI::new(subject),
                                            });
                                            let class = Class::new(IRI::new(value));
                                            o.add_axiom(Axiom::ClassAssertion(
                                                ClassAssertionAxiom {
                                                    id: axiom_id(),
                                                    individual,
                                                    class:
                                                        crate::ontology::ClassExpression::Class(
                                                            class,
                                                        ),
                                                    annotations: vec![],
                                                },
                                            ));
                                        }
                                    }
                                } else {
                                    match ty {
                                        "literal" => {
                                            let individual = Individual::Named(NamedIndividual {
                                                iri: IRI::new(subject),
                                            });
                                            let data_property = DataProperty {
                                                iri: IRI::new(predicate),
                                            };
                                            let literal = Literal {
                                                value: value.to_string(),
                                                language: lang,
                                                datatype,
                                            };
                                            o.add_axiom(Axiom::DataPropertyAssertion(
                                                DataPropertyAssertionAxiom {
                                                    id: axiom_id(),
                                                    individual,
                                                    property: DataPropertyExpression::DataProperty(
                                                        data_property,
                                                    ),
                                                    value: literal,
                                                    annotations: vec![],
                                                },
                                            ));
                                        }
                                        "uri" => {
                                            let individual = Individual::Named(NamedIndividual {
                                                iri: IRI::new(subject),
                                            });
                                            let target = Individual::Named(NamedIndividual {
                                                iri: IRI::new(value),
                                            });
                                            let prop =
                                                ObjectProperty::new(IRI::new(predicate))?;
                                            o.add_axiom(Axiom::ObjectPropertyAssertion(
                                                ObjectPropertyAssertionAxiom {
                                                    id: axiom_id(),
                                                    source: individual,
                                                    target,
                                                    property:
                                                        ObjectPropertyExpression::ObjectProperty(
                                                            prop,
                                                        ),
                                                    annotations: vec![],
                                                },
                                            ));
                                        }
                                        "bnode" => {
                                            o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                                                id: axiom_id(),
                                                entity: Entity::NamedIndividual(IRI::new(
                                                    subject,
                                                )),
                                            }));
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
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::from("{\n");
        let mut first = true;
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::Declaration(d) => match &d.entity {
                    Entity::Class(iri) => {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{RDF_TYPE}": [ {{ "type": "uri", "value": "{OWL_CLASS}" }} ] }}"#
                        );
                    }
                    Entity::ObjectProperty(iri) => {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{RDF_TYPE}": [ {{ "type": "uri", "value": "{OWL_OBJECT_PROPERTY}" }} ] }}"#
                        );
                    }
                    Entity::DataProperty(iri) => {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{RDF_TYPE}": [ {{ "type": "uri", "value": "{OWL_DATA_PROPERTY}" }} ] }}"#
                        );
                    }
                    Entity::NamedIndividual(iri) => {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{RDF_TYPE}": [ {{ "type": "uri", "value": "{OWL_NAMED_INDIVIDUAL}" }} ] }}"#
                        );
                    }
                    _ => {}
                },
                Axiom::ClassAssertion(a) => {
                    if let crate::ontology::ClassExpression::Class(class) = &a.class
                        && let Some(iri) = a.individual.iri()
                    {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{RDF_TYPE}": [ {{ "type": "uri", "value": "{}" }} ] }}"#,
                            class.iri
                        );
                    }
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = &a.property
                        && let (Some(source), Some(target)) =
                            (a.source.iri(), a.target.iri())
                    {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let _ = write!(
                            buf,
                            r#"  "{source}": {{ "{}": [ {{ "type": "uri", "value": "{target}" }} ] }}"#,
                            prop.iri
                        );
                    }
                }
                Axiom::DataPropertyAssertion(a) => {
                    if let DataPropertyExpression::DataProperty(prop) = &a.property
                        && let Some(iri) = a.individual.iri()
                    {
                        if !first {
                            buf.push_str(",\n");
                        }
                        first = false;
                        let escaped_value = a.value.value.replace('\\', "\\\\").replace('"', "\\\"");
                        let _ = write!(
                            buf,
                            r#"  "{iri}": {{ "{}": [ {{ "type": "literal", "value": "{escaped_value}" }} ] }}"#,
                            prop.iri
                        );
                    }
                }
                _ => {}
            }
        }
        buf.push_str("\n}\n");
        Ok(buf)
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    RdfJsonParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = RdfJsonRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("RDF/JSON: {e}")))
}
