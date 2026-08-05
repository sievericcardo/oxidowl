//! RDFa Parser and Serializer — extracts RDF from HTML/XML attributes.

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{
    Class, DataProperty, DataPropertyExpression, Individual, ObjectProperty,
    ObjectPropertyExpression,
};
use crate::ontology::{IRI, Literal, NamedIndividual, Ontology};
use std::collections::HashMap;

fn axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn extract_attr_value(line: &str, attr_name: &str) -> Option<String> {
    let search = format!("{attr_name}=\"");
    if let Some(pos) = line.find(&search) {
        let after = &line[pos + search.len()..];
        if let Some(end) = after.find('\"') {
            return Some(after[..end].to_string());
        }
    }
    None
}

fn resolve_term(term: &str, prefixes: &HashMap<String, String>, vocab: Option<&str>) -> String {
    if term.starts_with("http://") || term.starts_with("https://") || term.starts_with("_:") {
        return term.to_string();
    }
    if let Some(colon_pos) = term.find(':') {
        let prefix = &term[..colon_pos];
        let local = &term[colon_pos + 1..];
        if let Some(ns) = prefixes.get(prefix) {
            return format!("{ns}{local}");
        }
    }
    if let Some(v) = vocab {
        return format!("{v}{term}");
    }
    term.to_string()
}

fn parse_prefix_mapping(content: &str) -> HashMap<String, String> {
    let mut prefixes = HashMap::new();
    prefixes.insert(
        "rdf".into(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
    );
    prefixes.insert(
        "rdfs".into(),
        "http://www.w3.org/2000/01/rdf-schema#".into(),
    );
    prefixes.insert("owl".into(), "http://www.w3.org/2002/07/owl#".into());
    prefixes.insert("xsd".into(), "http://www.w3.org/2001/XMLSchema#".into());

    for line in content.lines() {
        if let Some(prefix_val) = extract_attr_value(line, "prefix") {
            for mapping in prefix_val.split_whitespace() {
                if let Some(colon_pos) = mapping.find(':') {
                    let prefix = &mapping[..colon_pos];
                    let uri = &mapping[colon_pos + 1..];
                    if !uri.is_empty() {
                        prefixes.insert(prefix.to_string(), uri.to_string());
                    }
                }
            }
        }
    }

    prefixes
}

#[derive(Debug, Clone, Default)]
pub struct RDFaParser;

impl RDFaParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let prefixes = parse_prefix_mapping(content);
        let mut current_vocab: Option<String> = None;
        let mut current_subject: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if let Some(vocab) = extract_attr_value(trimmed, "vocab") {
                current_vocab = Some(vocab);
            }

            if let Some(resource) = extract_attr_value(trimmed, "resource") {
                let resolved = resolve_term(&resource, &prefixes, current_vocab.as_deref());
                current_subject = Some(resolved);
            }

            if let Some(about) = extract_attr_value(trimmed, "about") {
                let resolved = resolve_term(&about, &prefixes, current_vocab.as_deref());
                current_subject = Some(resolved);
                o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                    id: axiom_id(),
                    entity: Entity::NamedIndividual(IRI::new(current_subject.as_ref().unwrap())),
                }));
            }

            if let Some(typeof_val) = extract_attr_value(trimmed, "typeof") {
                let resolved = resolve_term(&typeof_val, &prefixes, current_vocab.as_deref());
                if let Some(ref subj) = current_subject {
                    o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                        id: axiom_id(),
                        entity: Entity::Class(IRI::new(&resolved)),
                    }));
                    let individual = Individual::Named(NamedIndividual {
                        iri: IRI::new(subj),
                    });
                    let class = Class::new(IRI::new(&resolved));
                    o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: axiom_id(),
                        individual,
                        class: crate::ontology::ClassExpression::Class(class),
                        annotations: vec![],
                    }));
                } else {
                    o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                        id: axiom_id(),
                        entity: Entity::Class(IRI::new(&resolved)),
                    }));
                }
            }

            if let Some(property) = extract_attr_value(trimmed, "property")
                && let Some(ref subj) = current_subject
            {
                let resolved_prop = resolve_term(&property, &prefixes, current_vocab.as_deref());
                if let Some(content_val) = extract_attr_value(trimmed, "content") {
                    let dt = extract_attr_value(trimmed, "datatype");
                    let dt_url = dt.as_ref().and_then(|d| url::Url::parse(d).ok());
                    let literal = Literal {
                        value: content_val,
                        language: None,
                        datatype: dt_url,
                    };
                    let individual = Individual::Named(NamedIndividual {
                        iri: IRI::new(subj),
                    });
                    let data_property = DataProperty {
                        iri: IRI::new(&resolved_prop),
                    };
                    o.add_axiom(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                        id: axiom_id(),
                        individual,
                        property: DataPropertyExpression::DataProperty(data_property),
                        value: literal,
                        annotations: vec![],
                    }));
                } else if let Some(resource_val) = extract_attr_value(trimmed, "resource") {
                    let resolved_res =
                        resolve_term(&resource_val, &prefixes, current_vocab.as_deref());
                    let individual = Individual::Named(NamedIndividual {
                        iri: IRI::new(subj),
                    });
                    let target = Individual::Named(NamedIndividual {
                        iri: IRI::new(&resolved_res),
                    });
                    let prop = ObjectProperty::new(IRI::new(&resolved_prop))?;
                    o.add_axiom(Axiom::ObjectPropertyAssertion(
                        ObjectPropertyAssertionAxiom {
                            id: axiom_id(),
                            source: individual,
                            target,
                            property: ObjectPropertyExpression::ObjectProperty(prop),
                            annotations: vec![],
                        },
                    ));
                } else {
                    let text_val = Self::extract_text_content(trimmed);
                    if !text_val.is_empty() {
                        let literal = Literal {
                            value: text_val,
                            language: None,
                            datatype: None,
                        };
                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(subj),
                        });
                        let data_property = DataProperty {
                            iri: IRI::new(&resolved_prop),
                        };
                        o.add_axiom(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                            id: axiom_id(),
                            individual,
                            property: DataPropertyExpression::DataProperty(data_property),
                            value: literal,
                            annotations: vec![],
                        }));
                    }
                }
            }

            if let Some(rel) = extract_attr_value(trimmed, "rel")
                && let Some(ref subj) = current_subject
                && let Some(resource_val) = extract_attr_value(trimmed, "resource")
            {
                let resolved_rel = resolve_term(&rel, &prefixes, current_vocab.as_deref());
                let resolved_res = resolve_term(&resource_val, &prefixes, current_vocab.as_deref());
                let individual = Individual::Named(NamedIndividual {
                    iri: IRI::new(subj),
                });
                let target = Individual::Named(NamedIndividual {
                    iri: IRI::new(&resolved_res),
                });
                let prop = ObjectProperty::new(IRI::new(&resolved_rel))?;
                o.add_axiom(Axiom::ObjectPropertyAssertion(
                    ObjectPropertyAssertionAxiom {
                        id: axiom_id(),
                        source: individual,
                        target,
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    },
                ));
            }
        }
        Ok(o)
    }

    fn extract_text_content(line: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        for ch in line.chars() {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            } else if !in_tag {
                result.push(ch);
            }
        }
        result.trim().to_string()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RDFaRenderer;

impl RDFaRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::from("<!DOCTYPE html>\n<html>\n<head>\n</head>\n<body>\n");

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::Declaration(d) => match &d.entity {
                    Entity::Class(iri) => {
                        buf.push_str(&format!(
                            "  <div vocab=\"{OWL_NS}\" typeof=\"owl:Class\" resource=\"{iri}\"></div>\n"
                        ));
                    }
                    Entity::ObjectProperty(iri) => {
                        buf.push_str(&format!(
                            "  <div vocab=\"{OWL_NS}\" typeof=\"owl:ObjectProperty\" resource=\"{iri}\"></div>\n"
                        ));
                    }
                    Entity::DataProperty(iri) => {
                        buf.push_str(&format!(
                            "  <div vocab=\"{OWL_NS}\" typeof=\"owl:DataProperty\" resource=\"{iri}\"></div>\n"
                        ));
                    }
                    Entity::NamedIndividual(iri) => {
                        buf.push_str(&format!(
                            "  <div typeof=\"owl:NamedIndividual\" resource=\"{iri}\"></div>\n"
                        ));
                    }
                    _ => {}
                },
                Axiom::ClassAssertion(a) => {
                    if let crate::ontology::ClassExpression::Class(class) = &a.class
                        && let Some(iri) = a.individual.iri()
                    {
                        buf.push_str(&format!(
                            "  <div typeof=\"{}\" resource=\"{iri}\"></div>\n",
                            class.iri
                        ));
                    }
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = &a.property
                        && let (Some(source), Some(target)) = (a.source.iri(), a.target.iri())
                    {
                        buf.push_str(&format!(
                            "  <div resource=\"{source}\">\n    <a property=\"{}\" resource=\"{target}\"></a>\n  </div>\n",
                            prop.iri
                        ));
                    }
                }
                Axiom::DataPropertyAssertion(a) => {
                    if let DataPropertyExpression::DataProperty(prop) = &a.property
                        && let Some(iri) = a.individual.iri()
                    {
                        let escaped = Self::html_escape(&a.value.value);
                        buf.push_str(&format!(
                            "  <div resource=\"{iri}\">\n    <span property=\"{}\">{escaped}</span>\n  </div>\n",
                            prop.iri
                        ));
                    }
                }
                _ => {}
            }
        }

        buf.push_str("</body>\n</html>\n");
        Ok(buf)
    }

    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";

pub fn parse(content: &str) -> Result<Ontology> {
    RDFaParser::new().parse(content)
}

pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = RDFaRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("RDFa: {e}")))
}
