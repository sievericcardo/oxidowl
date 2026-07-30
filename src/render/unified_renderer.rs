use crate::ontology::*;

/// Unified trait for rendering individual OWL objects to strings.
/// Implemented by all syntax renderers.
pub trait OWLObjectRenderer {
    fn render_class_expression(&self, expr: &ClassExpression) -> String;
    fn render_axiom(&self, axiom: &Axiom) -> String;
    fn render_entity(&self, entity: &Entity) -> String;
    fn render_individual(&self, individual: &Individual) -> String;
    fn render_literal(&self, literal: &Literal) -> String;
    fn render_data_range(&self, data_range: &DataRange) -> String;
}

/// Simple implementation of OWLObjectRenderer that uses debug formatting.
pub struct DebugOWLObjectRenderer;

impl OWLObjectRenderer for DebugOWLObjectRenderer {
    fn render_class_expression(&self, expr: &ClassExpression) -> String {
        format!("{expr:?}")
    }

    fn render_axiom(&self, axiom: &Axiom) -> String {
        format!("{axiom:?}")
    }

    fn render_entity(&self, entity: &Entity) -> String {
        match entity {
            Entity::Class(iri) => format!("Class({iri})"),
            Entity::ObjectProperty(iri) => format!("ObjectProperty({iri})"),
            Entity::DataProperty(iri) => format!("DataProperty({iri})"),
            Entity::AnnotationProperty(iri) => format!("AnnotationProperty({iri})"),
            Entity::NamedIndividual(iri) => format!("NamedIndividual({iri})"),
            Entity::Datatype(iri) => format!("Datatype({iri})"),
        }
    }

    fn render_individual(&self, individual: &Individual) -> String {
        match individual {
            Individual::Named(ni) => format!("NamedIndividual({})", ni.iri),
            Individual::Anonymous(ai) => format!("AnonymousIndividual({})", ai.id),
        }
    }

    fn render_literal(&self, literal: &Literal) -> String {
        match &literal.language {
            Some(lang) => format!("\"{}\"@{}", literal.value, lang),
            None => match &literal.datatype {
                Some(dt) => format!("\"{}\"^^{}", literal.value, dt),
                None => format!("\"{}\"", literal.value),
            },
        }
    }

    fn render_data_range(&self, data_range: &DataRange) -> String {
        format!("{data_range:?}")
    }
}
