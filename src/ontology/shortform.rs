//! ShortFormProvider system — renders entities and IRIs in human-readable
//! compact forms (e.g., fragment, prefix:name, or annotation label).
//!
//! Also provides bidirectional (entity ↔ short form) resolution
//! via the `BidirectionalShortFormProvider` trait.

use crate::ontology::axioms::Entity;
use crate::ontology::vocabulary::{PrefixManager, rdfs, skos};
use crate::ontology::{IRI, OntologyRef};

pub trait ShortFormProvider: Send + Sync {
    fn get_short_form(&self, entity: &Entity) -> String;

    fn dispose(&self) {}
}

/// A short form provider that also supports reverse lookup:
/// resolving a short form string back to an entity.
pub trait BidirectionalShortFormProvider: ShortFormProvider {
    /// Resolve a short form string to an Entity, or `None` if not found.
    fn get_entity(&self, short_form: &str) -> Option<Entity>;
}

pub struct SimpleShortFormProvider;

impl SimpleShortFormProvider {
    fn extract_fragment_or_segment(iri: &IRI) -> String {
        let s = iri.as_str();

        if let Some(pos) = s.rfind('#') {
            let fragment = &s[pos + 1..];
            if !fragment.is_empty() {
                return fragment.to_string();
            }
        }

        let trimmed = s.trim_end_matches('/');
        if let Some(pos) = trimmed.rfind('/') {
            let segment = &trimmed[pos + 1..];
            let before = &trimmed[..pos];
            if !before.ends_with('/') && !segment.is_empty() {
                return segment.to_string();
            }
        }

        s.to_string()
    }
}

impl ShortFormProvider for SimpleShortFormProvider {
    fn get_short_form(&self, entity: &Entity) -> String {
        Self::extract_fragment_or_segment(entity.iri())
    }
}

pub struct QNameShortFormProvider {
    prefix_manager: PrefixManager,
    fallback: SimpleShortFormProvider,
}

impl QNameShortFormProvider {
    #[must_use]
    pub fn new(prefix_manager: PrefixManager) -> Self {
        Self {
            prefix_manager,
            fallback: SimpleShortFormProvider,
        }
    }
}

impl ShortFormProvider for QNameShortFormProvider {
    fn get_short_form(&self, entity: &Entity) -> String {
        if let Some(qname) = self.prefix_manager.shorten(entity.iri().as_str()) {
            qname
        } else {
            self.fallback.get_short_form(entity)
        }
    }
}

impl BidirectionalShortFormProvider for QNameShortFormProvider {
    fn get_entity(&self, short_form: &str) -> Option<Entity> {
        if let Some(expanded) = self.prefix_manager.expand(short_form) {
            let iri = IRI::new(&expanded);
            return Some(Entity::Class(iri));
        }
        None
    }
}

pub struct AnnotationValueShortFormProvider {
    ontology: OntologyRef,
    fallback: Box<dyn ShortFormProvider>,
}

impl AnnotationValueShortFormProvider {
    #[must_use]
    pub fn new(ontology: OntologyRef, fallback: Box<dyn ShortFormProvider>) -> Self {
        Self { ontology, fallback }
    }

    fn find_annotation_value(&self, entity: &Entity, property_iri: &str) -> Option<String> {
        let guard = self.ontology.read().ok()?;
        let target_iri = entity.iri();

        for axiom in guard.axioms() {
            if let crate::ontology::axioms::Axiom::AnnotationAssertion(ann) = axiom {
                let subject_matches = match &ann.subject {
                    crate::ontology::AnnotationSubject::IRI(subj_iri) => subj_iri == target_iri,
                    crate::ontology::AnnotationSubject::AnonymousIndividual(_) => false,
                };
                if subject_matches && ann.property.iri.as_str() == property_iri {
                    if let crate::ontology::AnnotationValue::Literal(lit) = &ann.value {
                        return Some(lit.value.clone());
                    }
                    if let crate::ontology::AnnotationValue::IRI(val_iri) = &ann.value {
                        return Some(val_iri.as_str().to_string());
                    }
                }
            }
        }
        None
    }
}

impl ShortFormProvider for AnnotationValueShortFormProvider {
    fn get_short_form(&self, entity: &Entity) -> String {
        if let Some(label) = self.find_annotation_value(entity, rdfs::LABEL) {
            return label;
        }
        if let Some(label) = self.find_annotation_value(entity, skos::PREF_LABEL) {
            return label;
        }
        self.fallback.get_short_form(entity)
    }
}

pub struct OntologyIRIShortFormProvider {
    inner: Box<dyn ShortFormProvider>,
}

impl OntologyIRIShortFormProvider {
    #[must_use]
    pub fn new(inner: Box<dyn ShortFormProvider>) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn get_short_form_for_iri(&self, iri: &IRI) -> String {
        let synthetic_entity = Entity::Class(iri.clone());
        self.inner.get_short_form(&synthetic_entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::IRI;
    use crate::ontology::axioms::Entity;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_simple_fragment() {
        let prov = SimpleShortFormProvider;
        let entity = Entity::Class(IRI::new("http://example.org/ontology#Person"));
        assert_eq!(prov.get_short_form(&entity), "Person");
    }

    #[test]
    fn test_simple_last_segment() {
        let prov = SimpleShortFormProvider;
        let entity = Entity::Class(IRI::new("http://example.org/Person"));
        assert_eq!(prov.get_short_form(&entity), "Person");
    }

    #[test]
    fn test_simple_fallback_full_iri() {
        let prov = SimpleShortFormProvider;
        let entity = Entity::Class(IRI::new("http://example.org/"));
        assert_eq!(prov.get_short_form(&entity), "http://example.org/");
    }

    #[test]
    fn test_qname_provider() {
        let pm = PrefixManager::new();
        let prov = QNameShortFormProvider::new(pm);
        let entity = Entity::Class(IRI::new("http://www.w3.org/2002/07/owl#Thing"));
        assert_eq!(prov.get_short_form(&entity), "owl:Thing");
    }

    #[test]
    fn test_qname_fallback_to_simple() {
        let pm = PrefixManager::new();
        let prov = QNameShortFormProvider::new(pm);
        let entity = Entity::Class(IRI::new("http://example.org/ontology#Person"));
        assert_eq!(prov.get_short_form(&entity), "Person");
    }

    #[test]
    fn test_annotation_value_label() {
        use crate::ontology::Ontology;
        use crate::ontology::axioms::{AnnotationAssertionAxiom, Axiom};
        use crate::ontology::{
            AnnotationProperty, AnnotationSubject, AnnotationValue as AnnValue, Literal,
        };

        let mut ont = Ontology::new();
        let ann = AnnotationAssertionAxiom {
            id: 1,
            subject: AnnotationSubject::IRI(IRI::new("http://example.org/Person")),
            property: AnnotationProperty {
                iri: IRI::new(rdfs::LABEL),
            },
            value: AnnValue::Literal(Literal::new("Person Label".to_string())),
            annotations: vec![],
        };
        ont.add_axiom(Axiom::AnnotationAssertion(ann));
        let ont_ref = OntologyRef::new(RwLock::new(ont));

        let fallback: Box<dyn ShortFormProvider> = Box::new(SimpleShortFormProvider);
        let prov = AnnotationValueShortFormProvider::new(Arc::clone(&ont_ref), fallback);
        let entity = Entity::Class(IRI::new("http://example.org/Person"));
        assert_eq!(prov.get_short_form(&entity), "Person Label");
    }

    #[test]
    fn test_annotation_value_fallback() {
        use crate::ontology::Ontology;
        let ont = Ontology::new();
        let ont_ref = OntologyRef::new(RwLock::new(ont));

        let fallback: Box<dyn ShortFormProvider> = Box::new(SimpleShortFormProvider);
        let prov = AnnotationValueShortFormProvider::new(Arc::clone(&ont_ref), fallback);
        let entity = Entity::Class(IRI::new("http://example.org/ontology#Person"));
        assert_eq!(prov.get_short_form(&entity), "Person");
    }

    #[test]
    fn test_ontology_iri_short_form() {
        let inner: Box<dyn ShortFormProvider> = Box::new(SimpleShortFormProvider);
        let prov = OntologyIRIShortFormProvider::new(inner);
        let iri = IRI::new("http://example.org/ontology#Person");
        assert_eq!(prov.get_short_form_for_iri(&iri), "Person");
    }
}
