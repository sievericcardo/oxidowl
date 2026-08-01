//! TriX Parser and Renderer — XML-based RDF with named graphs.

use crate::Result;
use crate::ontology::axioms::*;
use crate::ontology::{IRI, NamedIndividual, Ontology};
use crate::ontology::Individual;
use std::fmt::Write;

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DataProperty";

fn axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone, Default)]
pub struct TriXParser;

impl TriXParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Ontology> {
        let mut o = Ontology::new();
        let doc = roxmltree::Document::parse(content)
            .map_err(|e| crate::Error::ParseError(format!("TriX: {e}")))?;

        for graph_node in doc.descendants().filter(|n| n.has_tag_name("graph")) {
            for triple_node in graph_node
                .descendants()
                .filter(|n| n.has_tag_name("triple"))
            {
                let uris: Vec<String> = triple_node
                    .descendants()
                    .filter(|n| n.has_tag_name("uri"))
                    .filter_map(|n| n.text().map(std::string::ToString::to_string))
                    .collect();
                if uris.len() >= 3 {
                    let subject = &uris[0];
                    let predicate = &uris[1];
                    let object = &uris[2];

                    if predicate == RDF_TYPE {
                        match object.as_str() {
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
                            _ => {
                                let individual = Individual::Named(NamedIndividual {
                                    iri: IRI::new(subject),
                                });
                                let class = crate::ontology::Class::new(IRI::new(object));
                                o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                                    id: axiom_id(),
                                    individual,
                                    class: crate::ontology::ClassExpression::Class(class),
                                    annotations: vec![],
                                }));
                            }
                        }
                    } else {
                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(subject),
                        });
                        let target = Individual::Named(NamedIndividual {
                            iri: IRI::new(object),
                        });
                        let prop = crate::ontology::ObjectProperty::new(IRI::new(predicate))?;
                        o.add_axiom(Axiom::ObjectPropertyAssertion(
                            ObjectPropertyAssertionAxiom {
                                id: axiom_id(),
                                source: individual,
                                target,
                                property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                    prop,
                                ),
                                annotations: vec![],
                            },
                        ));
                    }
                }
            }
        }
        Ok(o)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TriXRenderer;
impl TriXRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf =
            String::from("<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\">\n  <graph>\n");
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::Declaration(d) => match &d.entity {
                    Entity::Class(iri) => {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{iri}</uri>\n      <uri>{RDF_TYPE}</uri>\n      <uri>{OWL_CLASS}</uri>\n    </triple>\n"
                        );
                    }
                    Entity::ObjectProperty(iri) => {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{iri}</uri>\n      <uri>{RDF_TYPE}</uri>\n      <uri>{OWL_OBJECT_PROPERTY}</uri>\n    </triple>\n"
                        );
                    }
                    Entity::DataProperty(iri) => {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{iri}</uri>\n      <uri>{RDF_TYPE}</uri>\n      <uri>{OWL_DATA_PROPERTY}</uri>\n    </triple>\n"
                        );
                    }
                    _ => {}
                },
                Axiom::ClassAssertion(a) => {
                    if let crate::ontology::ClassExpression::Class(class) = &a.class
                        && let Some(source_iri) = a.individual.iri()
                    {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{source_iri}</uri>\n      <uri>{RDF_TYPE}</uri>\n      <uri>{}</uri>\n    </triple>\n",
                            class.iri
                        );
                    }
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    if let crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =
                        &a.property
                        && let (Some(source_iri), Some(target_iri)) =
                            (a.source.iri(), a.target.iri())
                    {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{source_iri}</uri>\n      <uri>{}</uri>\n      <uri>{target_iri}</uri>\n    </triple>\n",
                            prop.iri
                        );
                    }
                }
                Axiom::DataPropertyAssertion(a) => {
                    if let crate::ontology::DataPropertyExpression::DataProperty(prop) =
                        &a.property
                        && let Some(source_iri) = a.individual.iri()
                    {
                        let _ = write!(
                            buf,
                            "    <triple>\n      <uri>{source_iri}</uri>\n      <uri>{}</uri>\n      <plainLiteral>{}</plainLiteral>\n    </triple>\n",
                            prop.iri, a.value.value
                        );
                    }
                }
                _ => {}
            }
        }
        buf.push_str("  </graph>\n</TriX>\n");
        Ok(buf)
    }
}

pub fn parse(content: &str) -> Result<Ontology> {
    TriXParser::new().parse(content)
}
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = TriXRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("TriX: {e}")))
}
