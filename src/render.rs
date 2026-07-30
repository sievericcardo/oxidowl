//! Unified OWL Object Renderer — renders individual axioms and class expressions
//! in human-readable syntax.
//!
//! Equivalent to OWL API v5's `OWLObjectRenderer` interface.

use crate::ontology::axioms::Axiom;
use crate::ontology::concepts::ClassExpression;
use crate::ontology::shortform::{ShortFormProvider, SimpleShortFormProvider};

/// Renders individual OWL objects (axioms, class expressions) to strings.
pub trait OWLObjectRenderer: Send + Sync {
    /// Set the short form provider for rendering entity names.
    fn set_short_form_provider(&mut self, provider: Box<dyn ShortFormProvider>);

    /// Render an axiom as a human-readable string.
    fn render_axiom(&self, axiom: &Axiom) -> String;

    /// Render a class expression as a human-readable string.
    fn render_class_expression(&self, ce: &ClassExpression) -> String;
}

/// Renders axioms in a concise, one-line format using short form provider.
pub struct ConciseObjectRenderer {
    short_form: Box<dyn ShortFormProvider>,
}

impl ConciseObjectRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            short_form: Box::new(SimpleShortFormProvider),
        }
    }
}

impl Default for ConciseObjectRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OWLObjectRenderer for ConciseObjectRenderer {
    fn set_short_form_provider(&mut self, provider: Box<dyn ShortFormProvider>) {
        self.short_form = provider;
    }

    fn render_axiom(&self, axiom: &Axiom) -> String {
        match axiom {
            Axiom::SubClassOf(a) => format!(
                "SubClassOf({}, {})",
                self.render_class_expression(&a.subclass),
                self.render_class_expression(&a.superclass)
            ),
            Axiom::EquivalentClasses(a) => {
                let parts: Vec<_> = a
                    .classes
                    .iter()
                    .map(|c| self.render_class_expression(c))
                    .collect();
                format!("EquivalentClasses({})", parts.join(" "))
            }
            Axiom::DisjointClasses(a) => {
                let parts: Vec<_> = a
                    .classes
                    .iter()
                    .map(|c| self.render_class_expression(c))
                    .collect();
                format!("DisjointClasses({})", parts.join(" "))
            }
            Axiom::ClassAssertion(a) => format!(
                "ClassAssertion({}, {:?})",
                self.render_class_expression(&a.class),
                a.individual
            ),
            Axiom::Declaration(d) => format!("Declaration({:?})", d.entity),
            other => format!("{other:?}"),
        }
    }

    fn render_class_expression(&self, ce: &ClassExpression) -> String {
        match ce {
            ClassExpression::Class(c) => {
                let entity = crate::ontology::axioms::Entity::Class(c.iri.clone());
                self.short_form.get_short_form(&entity)
            }
            ClassExpression::ObjectIntersectionOf(ops) => {
                let parts: Vec<_> = ops.iter().map(|c| self.render_class_expression(c)).collect();
                format!("({})", parts.join(" and "))
            }
            ClassExpression::ObjectUnionOf(ops) => {
                let parts: Vec<_> = ops.iter().map(|c| self.render_class_expression(c)).collect();
                format!("({})", parts.join(" or "))
            }
            ClassExpression::ObjectComplementOf(op) => {
                format!("not {}", self.render_class_expression(op))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!(
                    "{:?} some {}",
                    property,
                    self.render_class_expression(filler)
                )
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!(
                    "{:?} only {}",
                    property,
                    self.render_class_expression(filler)
                )
            }
            other => format!("{other:?}"),
        }
    }
}
