//! N-Quads Parser and Renderer.
//! Like N-Triples but with named graph support.

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{IRI, Literal, NamedIndividual, Ontology};
use crate::ontology::{
    Class, DataProperty, DataPropertyExpression, Individual, ObjectProperty,
    ObjectPropertyExpression,
};

fn axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DataProperty";

fn parse_term(token: &str) -> (String, bool) {
    let trimmed = token.trim();
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        (trimmed[1..trimmed.len() - 1].to_string(), false)
    } else if trimmed.starts_with("_:") {
        (trimmed.to_string(), false)
    } else if trimmed.starts_with('"') {
        (trimmed.to_string(), true)
    } else if trimmed.contains(':') {
        (trimmed.to_string(), false)
    } else {
        (trimmed.to_string(), false)
    }
}

fn parse_quoted_literal(token: &str) -> (String, Option<String>, Option<url::Url>) {
    let trimmed = token.trim();
    if let Some(caret_pos) = trimmed.rfind("^^") {
        let value_part = &trimmed[..caret_pos].trim();
        let dt_part = &trimmed[caret_pos + 2..].trim();
        let value = value_part
            .trim_matches('"')
            .to_string();
        let dt_str = if dt_part.starts_with('<') && dt_part.ends_with('>') {
            dt_part[1..dt_part.len() - 1].to_string()
        } else {
            dt_part.to_string()
        };
        let dt = url::Url::parse(&dt_str).ok();
        (value, None, dt)
    } else if let Some(at_pos) = trimmed.rfind('@') {
        let value_part = &trimmed[..at_pos].trim();
        let lang_part = &trimmed[at_pos + 1..].trim();
        let value = value_part
            .trim_matches('"')
            .to_string();
        (value, Some(lang_part.to_string()), None)
    } else {
        let value = trimmed
            .trim_matches('"')
            .to_string();
        (value, None, None)
    }
}

/// Parses N-Quads content line-by-line.
#[derive(Debug, Clone, Default)]
pub struct NQuadsParser;

impl NQuadsParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let mut current_line = String::new();
        let mut in_literal = false;
        let mut _in_rdf_star = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if in_literal {
                current_line.push(' ');
                current_line.push_str(trimmed);
                if trimmed.contains('"') {
                    in_literal = false;
                }
                if !trimmed.ends_with('.') {
                    continue;
                }
            }

            current_line = trimmed.to_string();
            let quote_count = current_line.matches('"').count();
            if !quote_count.is_multiple_of(2) {
                in_literal = true;
                continue;
            }
            in_literal = false;

            if current_line.contains("<<") && current_line.contains(">>") {
                _in_rdf_star = true;
            }

            let line_content = if let Some(stripped) = current_line.strip_suffix('.') {
                stripped
            } else {
                &current_line
            };

            let parts: Vec<&str> = line_content.split_whitespace().collect();

            if parts.len() < 3 {
                continue;
            }

            let mut idx = 0;
            let mut subject_str = parts[idx].to_string();
            idx += 1;

            if subject_str.starts_with("<<") {
                let mut star_content = subject_str.clone();
                star_content.push(' ');
                while idx < parts.len() && !star_content.contains(">>") {
                    star_content.push(' ');
                    star_content.push_str(parts[idx]);
                    idx += 1;
                }
                subject_str = star_content;
            }

            if idx >= parts.len() {
                continue;
            }
            let predicate_str = parts[idx].to_string();
            idx += 1;

            if idx >= parts.len() {
                continue;
            }

            let mut object_str = parts[idx].to_string();
            idx += 1;

            if object_str.starts_with('"') {
                while idx < parts.len() {
                    let next = parts[idx];
                    if next.ends_with('"') || next.contains("^^") || next.starts_with('@') {
                        object_str.push(' ');
                        object_str.push_str(next);
                        idx += 1;
                        break;
                    }
                    object_str.push(' ');
                    object_str.push_str(next);
                    idx += 1;
                    if next.ends_with('"') {
                        break;
                    }
                }
            }

            let _graph = if idx < parts.len() {
                let g = parts[idx].to_string();
                Some(g)
            } else {
                None
            };

            if subject_str.starts_with("<<") {
                continue;
            }

            let (subject, _) = parse_term(&subject_str);
            let (predicate, _) = parse_term(&predicate_str);
            let (object, is_literal) = parse_term(&object_str);

            if predicate == RDF_TYPE {
                match object.as_str() {
                    OWL_CLASS => {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::Class(IRI::new(&subject)),
                        }));
                    }
                    OWL_OBJECT_PROPERTY => {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::ObjectProperty(IRI::new(&subject)),
                        }));
                    }
                    OWL_DATA_PROPERTY => {
                        o.add_axiom(Axiom::Declaration(DeclarationAxiom {
                            id: axiom_id(),
                            entity: Entity::DataProperty(IRI::new(&subject)),
                        }));
                    }
                    _ => {
                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(&subject),
                        });
                        let class = Class::new(IRI::new(&object));
                        o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                            id: axiom_id(),
                            individual,
                            class: crate::ontology::ClassExpression::Class(class),
                            annotations: vec![],
                        }));
                    }
                }
            } else if is_literal {
                let (value, lang, datatype) = parse_quoted_literal(&object_str);
                let individual = Individual::Named(NamedIndividual {
                    iri: IRI::new(&subject),
                });
                let data_property = DataProperty {
                    iri: IRI::new(&predicate),
                };
                let literal = Literal {
                    value,
                    language: lang,
                    datatype,
                };
                o.add_axiom(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                    id: axiom_id(),
                    individual,
                    property: DataPropertyExpression::DataProperty(data_property),
                    value: literal,
                    annotations: vec![],
                }));
            } else {
                let individual = Individual::Named(NamedIndividual {
                    iri: IRI::new(&subject),
                });
                let target = Individual::Named(NamedIndividual {
                    iri: IRI::new(&object),
                });
                let prop = ObjectProperty::new(IRI::new(&predicate))?;
                o.add_axiom(Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                    id: axiom_id(),
                    source: individual,
                    target,
                    property: ObjectPropertyExpression::ObjectProperty(prop),
                    annotations: vec![],
                }));
            }
        }

        Ok(o)
    }
}

/// N-Quads renderer.
#[derive(Debug, Clone, Default)]
pub struct NQuadsRenderer;

impl NQuadsRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::new();
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::Declaration(d) => {
                    let type_iri = match &d.entity {
                        Entity::Class(_) => OWL_CLASS,
                        Entity::ObjectProperty(_) => OWL_OBJECT_PROPERTY,
                        Entity::DataProperty(_) => OWL_DATA_PROPERTY,
                        Entity::NamedIndividual(_) => {
                            "http://www.w3.org/2002/07/owl#NamedIndividual"
                        }
                        _ => continue,
                    };
                    buf.push_str(&format!(
                        "<{}> <{RDF_TYPE}> <{type_iri}> .\n",
                        d.entity.iri()
                    ));
                }
                Axiom::ClassAssertion(a) => {
                    if let crate::ontology::ClassExpression::Class(class) = &a.class
                        && let Some(iri) = a.individual.iri()
                    {
                        buf.push_str(&format!(
                            "<{iri}> <{RDF_TYPE}> <{}> .\n",
                            class.iri
                        ));
                    }
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = &a.property
                        && let (Some(source), Some(target)) =
                            (a.source.iri(), a.target.iri())
                    {
                        buf.push_str(&format!(
                            "<{source}> <{}> <{target}> .\n",
                            prop.iri
                        ));
                    }
                }
                Axiom::DataPropertyAssertion(a) => {
                    if let DataPropertyExpression::DataProperty(prop) = &a.property
                        && let Some(iri) = a.individual.iri()
                    {
                        let escaped = a.value.value.replace('\\', "\\\\").replace('"', "\\\"");
                        if let Some(lang) = &a.value.language {
                            buf.push_str(&format!(
                                "<{iri}> <{}> \"{escaped}\"@{lang} .\n",
                                prop.iri
                            ));
                        } else if let Some(dt) = &a.value.datatype {
                            buf.push_str(&format!(
                                "<{iri}> <{}> \"{escaped}\"^^<{dt}> .\n",
                                prop.iri
                            ));
                        } else {
                            buf.push_str(&format!(
                                "<{iri}> <{}> \"{escaped}\" .\n",
                                prop.iri
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(buf)
    }
}

/// Public entry points.
pub fn parse(content: &str) -> Result<Ontology> {
    NQuadsParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = NQuadsRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("NQuads: {e}")))
}
