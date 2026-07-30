//! JSON-LD Parser and Renderer.

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{IRI, Literal, NamedIndividual, Ontology};
use crate::ontology::{
    AnnotationSubject, AnnotationValue, Class, DataProperty, Individual, ObjectProperty,
};
use serde_json::Value;
use std::collections::HashMap;

fn axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn default_context() -> HashMap<String, String> {
    let mut ctx = HashMap::new();
    ctx.insert("rdf".into(), RDF.into());
    ctx.insert("rdfs".into(), RDFS.into());
    ctx.insert("owl".into(), OWL.into());
    ctx.insert("xsd".into(), XSD.into());
    ctx.insert("type".into(), format!("{RDF}type"));
    ctx.insert("label".into(), format!("{RDFS}label"));
    ctx.insert("comment".into(), format!("{RDFS}comment"));
    ctx
}

fn parse_context(val: &Value, ctx: &mut HashMap<String, String>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                match v {
                    Value::String(s) => {
                        ctx.insert(k.clone(), s.clone());
                    }
                    Value::Object(obj) => {
                        if let Some(id_val) = obj.get("@id") {
                            if let Some(s) = id_val.as_str() {
                                ctx.insert(k.clone(), s.to_string());
                            }
                        }
                        if let Some(type_val) = obj.get("@type") {
                            if let Some(s) = type_val.as_str() {
                                ctx.insert(k.clone(), s.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::String(s) => {
            if let Ok(url_val) = serde_json::from_str::<Value>(s) {
                parse_context(&url_val, ctx);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                parse_context(item, ctx);
            }
        }
        _ => {}
    }
}

fn expand_term(term: &str, ctx: &HashMap<String, String>) -> String {
    if term.starts_with("http://") || term.starts_with("https://") || term.starts_with("_:") {
        return term.to_string();
    }
    if let Some(colon_pos) = term.find(':') {
        let prefix = &term[..colon_pos];
        let local = &term[colon_pos + 1..];
        if let Some(ns) = ctx.get(prefix) {
            return format!("{ns}{local}");
        }
    }
    if let Some(expanded) = ctx.get(term) {
        return expanded.clone();
    }
    term.to_string()
}

fn extract_id(node: &Value) -> Option<String> {
    if let Some(id) = node.get("@id") {
        if let Some(s) = id.as_str() {
            return Some(s.to_string());
        }
    }
    None
}

fn extract_type(node: &Value) -> Vec<String> {
    let mut types = Vec::new();
    if let Some(t) = node.get("@type") {
        match t {
            Value::String(s) => types.push(s.to_string()),
            Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        types.push(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    types
}

fn extract_value(node: &Value) -> Option<(String, Option<String>, Option<String>)> {
    if let Some(val) = node.get("@value") {
        let value_str = val.as_str().map(|s| s.to_string()).unwrap_or_default();
        let lang = node.get("@language").and_then(|v| v.as_str().map(String::from));
        let datatype = node.get("@datatype").and_then(|v| v.as_str().map(String::from));
        Some((value_str, lang, datatype))
    } else {
        None
    }
}

fn is_owl_class_type(t: &str) -> bool {
    t.contains("Class") || t == "owl:Class" || t == format!("{OWL}Class")
}

fn is_owl_object_property_type(t: &str) -> bool {
    t.contains("ObjectProperty") || t == "owl:ObjectProperty" || t == format!("{OWL}ObjectProperty")
}

fn is_owl_data_property_type(t: &str) -> bool {
    t.contains("DataProperty") || t == "owl:DataProperty" || t == format!("{OWL}DataProperty")
}

fn is_owl_annotation_property_type(t: &str) -> bool {
    t.contains("AnnotationProperty")
        || t == "owl:AnnotationProperty"
        || t == format!("{OWL}AnnotationProperty")
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdParser;

impl JsonLdParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let val: Value = serde_json::from_str(content)
            .map_err(|e| crate::Error::ParseError(format!("JSON-LD: {e}")))?;

        let mut ctx = default_context();

        if let Some(c) = val.get("@context") {
            parse_context(c, &mut ctx);
        }

        if let Some(graph) = val.get("@graph").and_then(|v| v.as_array()) {
            let nodes: Vec<&Value> = graph.iter().collect();
            self.parse_graph(&nodes, &mut o, &ctx)?;
        } else if val.is_object() {
            let nodes: Vec<&Value> = vec![&val];
            self.parse_graph(&nodes, &mut o, &ctx)?;
        }

        Ok(o)
    }

    fn parse_graph(
        &self,
        graph: &[&Value],
        o: &mut Ontology,
        ctx: &HashMap<String, String>,
    ) -> Result<()> {
        for node in graph {
            if let Some(id) = extract_id(node) {
                let expanded_id = expand_term(&id, ctx);
                let types = extract_type(node);

                for t in &types {
                    let expanded_type = expand_term(t, ctx);
                    if is_owl_class_type(&expanded_type) {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::Class(IRI::new(&expanded_id)),
                        }));
                    } else if is_owl_object_property_type(&expanded_type) {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::ObjectProperty(IRI::new(&expanded_id)),
                        }));
                    } else if is_owl_data_property_type(&expanded_type) {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::DataProperty(IRI::new(&expanded_id)),
                        }));
                    } else if is_owl_annotation_property_type(&expanded_type) {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::AnnotationProperty(IRI::new(&expanded_id)),
                        }));
                    } else {
                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(&expanded_id),
                        });
                        let class = Class::new(IRI::new(&expanded_type));
                        o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                            id: axiom_id(),
                            individual,
                            class: crate::ontology::ClassExpression::Class(class),
                            annotations: vec![],
                        }));
                    }
                }

                if let Some(obj) = node.as_object() {
                    for (key, val) in obj {
                        if key == "@id" || key == "@type" || key == "@context" {
                            continue;
                        }

                        let expanded_key = expand_term(key, ctx);

                        if let (Some((value_str, lang, datatype)),) = (extract_value(val),) {
                            let lang = lang.or_else(|| {
                                val.get("@language")
                                    .and_then(|v| v.as_str().map(String::from))
                            });
                            let dt = datatype.or_else(|| {
                                val.get("@datatype")
                                    .and_then(|v| v.as_str().map(String::from))
                            });
                            let dt_url = dt.as_ref().and_then(|d| {
                                let expanded = expand_term(d, ctx);
                                url::Url::parse(&expanded).ok()
                            });

                            let literal = Literal {
                                value: value_str,
                                language: lang,
                                datatype: dt_url,
                            };

                            let individual = Individual::Named(NamedIndividual {
                                iri: IRI::new(&expanded_id),
                            });
                            let data_property = DataProperty {
                                iri: IRI::new(&expanded_key),
                            };

                            o.add_axiom(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                                id: axiom_id(),
                                individual,
                                property: crate::ontology::DataPropertyExpression::DataProperty(
                                    data_property,
                                ),
                                value: literal,
                                annotations: vec![],
                            }));
                        } else if let Value::String(s) = val {
                            let expanded_val = expand_term(s, ctx);

                            let individual = Individual::Named(NamedIndividual {
                                iri: IRI::new(&expanded_id),
                            });
                            let target = Individual::Named(NamedIndividual {
                                iri: IRI::new(&expanded_val),
                            });
                            let prop = ObjectProperty::new(IRI::new(&expanded_key))?;
                            o.add_axiom(Axiom::ObjectPropertyAssertion(
                                ObjectPropertyAssertionAxiom {
                                    id: axiom_id(),
                                    source: individual,
                                    target,
                                    property:
                                        crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                            prop,
                                        ),
                                    annotations: vec![],
                                },
                            ));
                        } else if let Value::Object(_inner_obj) = val {
                            if let Some(inner_id) = extract_id(val) {
                                let expanded_inner = expand_term(&inner_id, ctx);
                                let individual = Individual::Named(NamedIndividual {
                                    iri: IRI::new(&expanded_id),
                                });
                                let target = Individual::Named(NamedIndividual {
                                    iri: IRI::new(&expanded_inner),
                                });
                                let prop = ObjectProperty::new(IRI::new(&expanded_key))?;
                                o.add_axiom(Axiom::ObjectPropertyAssertion(
                                    ObjectPropertyAssertionAxiom {
                                        id: axiom_id(),
                                        source: individual,
                                        target,
                                        property:
                                            crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                                prop,
                                            ),
                                        annotations: vec![],
                                    },
                                ));
                            }
                        } else if let Value::Array(arr) = val {
                            self.parse_array_values(&expanded_id, &expanded_key, arr, o, ctx)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_array_values(
        &self,
        subject: &str,
        predicate: &str,
        arr: &[Value],
        o: &mut Ontology,
        ctx: &HashMap<String, String>,
    ) -> Result<()> {
        for val in arr {
            match val {
                Value::String(s) => {
                    let expanded = expand_term(s, ctx);
                    let individual = Individual::Named(NamedIndividual {
                        iri: IRI::new(subject),
                    });
                    let target = Individual::Named(NamedIndividual {
                        iri: IRI::new(&expanded),
                    });
                    let prop = ObjectProperty::new(IRI::new(predicate))?;
                    o.add_axiom(Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                        id: axiom_id(),
                        source: individual,
                        target,
                        property: crate::ontology::ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    }));
                }
                Value::Object(obj) => {
                    if let (Some((value_str, lang, datatype)),) = (extract_value(val),) {
                        let lang = lang.or_else(|| {
                            obj.get("@language")
                                .and_then(|v| v.as_str().map(String::from))
                        });
                        let dt = datatype.or_else(|| {
                            obj.get("@datatype")
                                .and_then(|v| v.as_str().map(String::from))
                        });
                        let dt_url = dt.as_ref().and_then(|d| {
                            let expanded = expand_term(d, ctx);
                            url::Url::parse(&expanded).ok()
                        });

                        let literal = Literal {
                            value: value_str,
                            language: lang,
                            datatype: dt_url,
                        };

                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(subject),
                        });
                        let data_property = DataProperty {
                            iri: IRI::new(predicate),
                        };

                        o.add_axiom(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                            id: axiom_id(),
                            individual,
                            property: crate::ontology::DataPropertyExpression::DataProperty(
                                data_property,
                            ),
                            value: literal,
                            annotations: vec![],
                        }));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct JsonLdRenderer;
impl JsonLdRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut items = Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::Declaration(d) => match &d.entity {
                    Entity::Class(iri) => {
                        items.push(format!(r#"{{"@id":"{iri}","@type":"owl:Class"}}"#));
                    }
                    Entity::ObjectProperty(iri) => {
                        items.push(format!(
                            r#"{{"@id":"{iri}","@type":"owl:ObjectProperty"}}"#
                        ));
                    }
                    Entity::DataProperty(iri) => {
                        items.push(format!(r#"{{"@id":"{iri}","@type":"owl:DataProperty"}}"#));
                    }
                    Entity::NamedIndividual(iri) => {
                        items.push(format!(
                            r#"{{"@id":"{iri}","@type":"owl:NamedIndividual"}}"#
                        ));
                    }
                    _ => {}
                },
                Axiom::ClassAssertion(a) => {
                    if let crate::ontology::ClassExpression::Class(class) = &a.class
                        && let Some(iri) = a.individual.iri()
                    {
                        items.push(format!(
                            r#"{{"@id":"{iri}","@type":"{}"}}"#,
                            class.iri
                        ));
                    }
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    if let crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =
                        &a.property
                        && let (Some(source), Some(target)) = (a.source.iri(), a.target.iri())
                    {
                        items.push(format!(
                            r#"{{"@id":"{source}","{}":{{"@id":"{target}"}}}}"#,
                            prop.iri
                        ));
                    }
                }
                Axiom::DataPropertyAssertion(a) => {
                    if let crate::ontology::DataPropertyExpression::DataProperty(prop) =
                        &a.property
                        && let Some(iri) = a.individual.iri()
                    {
                        items.push(format!(
                            r#"{{"@id":"{iri}","{}":{{"@value":"{}"}}}}"#,
                            prop.iri,
                            a.value.value.replace('"', "\\\"")
                        ));
                    }
                }
                Axiom::AnnotationAssertion(a) => {
                    if let AnnotationSubject::IRI(iri) = &a.subject {
                        if let AnnotationValue::Literal(lit) = &a.value {
                            items.push(format!(
                                r#"{{"@id":"{iri}","{}":{{"@value":"{}"}}}}"#,
                                a.property.iri,
                                lit.value.replace('"', "\\\"")
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(format!(
            "{{\n  \"@graph\": [\n    {}\n  ]\n}}\n",
            items.join(",\n    ")
        ))
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    JsonLdParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = JsonLdRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("JSON-LD: {e}")))
}
