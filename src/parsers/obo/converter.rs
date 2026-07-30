//! OBO ↔ OWL Converters.
//!
//! `Obo2Owl` converts OBO stanzas to OWL axioms.
//! `Owl2Obo` converts OWL axioms back to OBO stanzas.

use crate::ontology::{
    Class, ClassExpression, Individual,
    ObjectProperty, ObjectPropertyExpression, Ontology, AnnotationValue,
    AnnotationProperty, IRI,
};
use crate::ontology::axioms::*;
use crate::Result;
use std::collections::HashMap;
use std::fmt::Write;

/// Converts OBO stanzas to OWL axioms.
pub struct Obo2Owl {
    iri_prefix: String,
}

impl Obo2Owl {
    #[must_use]
    pub fn new() -> Self { Self { iri_prefix: "http://purl.obolibrary.org/obo/".into() } }

    /// Convert a list of stanzas into an OWL ontology.
    pub fn convert_stanzas(&self, stanzas: &[super::parser::OBOStanza]) -> Result<Ontology> {
        let mut ontology = Ontology::new();
        let mut class_map: HashMap<String, ClassExpression> = HashMap::new();

        for stanza in stanzas {
            let tags = &stanza.tags;
            let id = self.get_tag(tags, "id");
            if id.is_empty() { continue; }

            match stanza.stanza_type.as_str() {
                "Term" => {
                    let iri = self.expand(&id);
                    let cls = ClassExpression::Class(Class { iri: iri.clone() });
                    class_map.insert(id.clone(), cls.clone());
                    ontology.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::Class(iri.clone()) }));

                    // name → rdfs:label
                    if let Some(name) = self.get_tag_opt(tags, "name") {
                        ontology.add_axiom(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                            id: 0,
                            property: AnnotationProperty { iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label") },
                            subject: crate::ontology::AnnotationSubject::IRI(iri.clone()),
                            value: AnnotationValue::Literal(crate::ontology::Literal::new(name.to_string())),
                            annotations: vec![],
                        }));
                    }

                    // def → rdfs:comment
                    if let Some(def) = self.get_tag_opt(tags, "def") {
                        let clean = def.trim_matches('"').split('[').next().unwrap_or("").trim();
                        if !clean.is_empty() {
                            ontology.add_axiom(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                                id: 0,
                                property: AnnotationProperty { iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#comment") },
                                subject: crate::ontology::AnnotationSubject::IRI(iri.clone()),
                                value: AnnotationValue::Literal(crate::ontology::Literal::new(clean.to_string())),
                                annotations: vec![],
                            }));
                        }
                    }

                    // is_a → SubClassOf
                    for (_tag, val) in tags.iter().filter(|(t, _)| t == "is_a") {
                        let parent_id = val.split('!').next().unwrap_or("").trim();
                        if parent_id.is_empty() { continue; }
                        let parent_iri = self.expand(parent_id);
                        let parent = ClassExpression::Class(Class { iri: parent_iri });
                        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                            id: 0, subclass: cls.clone(), superclass: parent, annotations: vec![],
                        }));
                    }

                    // relationship: R P → SubClassOf(self, SomeValuesFrom(R, P))
                    for (_tag, val) in tags.iter().filter(|(t, _)| t == "relationship") {
                        let parts: Vec<&str> = val.splitn(2, ' ').collect();
                        if parts.len() < 2 { continue; }
                        let rel = parts[0].trim();
                        let target = parts[1].split('!').next().unwrap_or("").trim();
                        if target.is_empty() { continue; }
                        let prop = ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: self.expand(rel) });
                        let filler = ClassExpression::Class(Class { iri: self.expand(target) });
                        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                            id: 0,
                            subclass: cls.clone(),
                            superclass: ClassExpression::ObjectSomeValuesFrom { property: prop, filler: Box::new(filler) },
                            annotations: vec![],
                        }));
                    }

                    // intersection_of → EquivalentClasses
                    let inter_tags: Vec<&String> = tags.iter().filter(|(t, _)| t == "intersection_of").map(|(_, v)| v).collect();
                    if !inter_tags.is_empty() {
                        let mut equivalents = vec![cls.clone()];
                        for val in &inter_tags {
                            let inter_id = val.split('!').next().unwrap_or("").trim();
                            if inter_id.is_empty() { continue; }
                            if let Some(space_pos) = inter_id.find(' ') {
                                let rel = &inter_id[..space_pos];
                                let tgt = inter_id[space_pos+1..].trim();
                                let prop = ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: self.expand(rel) });
                                let filler = ClassExpression::Class(Class { iri: self.expand(tgt) });
                                equivalents.push(ClassExpression::ObjectSomeValuesFrom { property: prop, filler: Box::new(filler) });
                            } else {
                                equivalents.push(ClassExpression::Class(Class { iri: self.expand(inter_id) }));
                            }
                        }
                        if equivalents.len() > 1 {
                            ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                                id: 0, classes: equivalents, annotations: vec![],
                            }));
                        }
                    }

                    // disjoint_from → DisjointClasses
                    for (_tag, val) in tags.iter().filter(|(t, _)| t == "disjoint_from") {
                        let d_id = val.split('!').next().unwrap_or("").trim();
                        if d_id.is_empty() { continue; }
                        ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
                            id: 0,
                            classes: vec![cls.clone(), ClassExpression::Class(Class { iri: self.expand(d_id) })],
                            annotations: vec![],
                        }));
                    }

                    // is_obsolete: true → owl:deprecated
                    if self.get_tag_opt(tags, "is_obsolete").is_some_and(|v| v == "true") {
                        ontology.add_axiom(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                            id: 0,
                            property: AnnotationProperty { iri: IRI::new("http://www.w3.org/2002/07/owl#deprecated") },
                            subject: crate::ontology::AnnotationSubject::IRI(iri.clone()),
                            value: AnnotationValue::Literal(crate::ontology::Literal::new("true".into())),
                            annotations: vec![],
                        }));
                    }
                }
                "Typedef" => {
                    let iri = self.expand(&id);
                    ontology.add_axiom(Axiom::Declaration(DeclarationAxiom { id: 0, entity: Entity::ObjectProperty(iri.clone()) }));
                    let prop = ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: iri.clone() });

                    // is_a → SubObjectPropertyOf
                    for (_tag, val) in tags.iter().filter(|(t, _)| t == "is_a") {
                        let parent_id = val.split('!').next().unwrap_or("").trim();
                        if parent_id.is_empty() { continue; }
                        ontology.add_axiom(Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                            id: 0,
                            sub_property: prop.clone(),
                            super_property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: self.expand(parent_id) }),
                            annotations: vec![],
                        }));
                    }

                    if self.get_tag_opt(tags, "is_transitive").is_some_and(|v| v == "true") {
                        ontology.add_axiom(Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom { id: 0, property: prop.clone(), annotations: vec![] }));
                    }
                    if self.get_tag_opt(tags, "is_symmetric").is_some_and(|v| v == "true") {
                        ontology.add_axiom(Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom { id: 0, property: prop.clone(), annotations: vec![] }));
                    }
                    if self.get_tag_opt(tags, "is_cyclic").is_some_and(|v| v == "true") {
                        ontology.add_axiom(Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom { id: 0, property: prop.clone(), annotations: vec![] }));
                    }
                }
                "Instance" => {
                    let iri = self.expand(&id);
                    let ind = Individual::Named(crate::ontology::NamedIndividual { iri: iri.clone() });
                    // instance_of → ClassAssertion
                    for (_tag, val) in tags.iter().filter(|(t, _)| t == "instance_of") {
                        let cls_id = val.trim();
                        if cls_id.is_empty() { continue; }
                        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                            id: 0,
                            class: ClassExpression::Class(Class { iri: self.expand(cls_id) }),
                            individual: ind.clone(),
                            annotations: vec![],
                        }));
                    }
                }
                _ => {}
            }
        }
        Ok(ontology)
    }

    fn get_tag(&self, tags: &[(String, String)], name: &str) -> String {
        self.get_tag_opt(tags, name).unwrap_or("").to_string()
    }

    fn get_tag_opt<'a>(&self, tags: &'a [(String, String)], name: &str) -> Option<&'a str> {
        tags.iter().find(|(t, _)| t == name).map(|(_, v)| v.as_str())
    }

    fn expand(&self, id: &str) -> IRI {
        if id.starts_with("http") { IRI::new(id) }
        else { IRI::new(&format!("{}{}", self.iri_prefix, id.replace(':', "_"))) }
    }
}

// ── Owl2Obo ─────────────────────────────────────────────────────────────────

/// Converts OWL axioms back to OBO stanzas.
pub struct Owl2Obo;

impl Owl2Obo {
    #[must_use]
    pub fn new() -> Self { Self }

    /// Serialize an ontology to OBO format string.
    pub fn serialize(&self, ontology: &Ontology) -> String {
        let mut buf = String::new();
        buf.push_str("format-version: 1.4\n");
        buf.push_str("data-version: generated\n\n");

        let axioms = ontology.axioms();
        for axiom in axioms {
            match axiom {
                Axiom::Declaration(d) => {
                    match &d.entity {
                        Entity::Class(iri) => {
                            let id = self.shorten(iri);
                            let _ = write!(buf, "[Term]\nid: {id}\nname: {id}\n\n");
                        }
                        Entity::ObjectProperty(iri) => {
                            let id = self.shorten(iri);
                            let _ = write!(buf, "[Typedef]\nid: {id}\nname: {id}\n\n");
                        }
                        _ => {}
                    }
                }
                Axiom::SubClassOf(a) => {
                    if let (ClassExpression::Class(sub), ClassExpression::Class(sup)) = (&a.subclass, &a.superclass) {
                        let sid = self.shorten(&sub.iri);
                        let pid = self.shorten(&sup.iri);
                        let _ = write!(buf, "[Term]\nid: {sid}\nis_a: {pid}\n\n");
                    }
                }
                _ => {}
            }
        }
        buf
    }

    fn shorten(&self, iri: &IRI) -> String {
        let s = iri.as_str();
        if let Some(pos) = s.rfind('/') {
            s[pos+1..].to_string()
        } else if let Some(pos) = s.rfind('#') {
            s[pos+1..].to_string()
        } else {
            s.to_string()
        }
    }
}
