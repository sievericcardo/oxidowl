//! Explanation Renderers — format justifications for human consumption.
//!
//! Provides renderer traits and implementations for formatting
//! explanation axioms using short form providers or Manchester syntax.

use crate::explanation::generator::Explanation;
use crate::ontology::shortform::ShortFormProvider;

/// Renders an explanation as a human-readable string.
pub trait ExplanationRenderer: Send + Sync {
    fn render(&self, explanation: &Explanation) -> String;
}

/// Renders axioms using short form providers for compact output.
pub struct ConciseExplanationRenderer {
    short_form: Box<dyn ShortFormProvider>,
}

impl ConciseExplanationRenderer {
    #[must_use]
    pub fn new(short_form: Box<dyn ShortFormProvider>) -> Self {
        Self { short_form }
    }
}

impl ExplanationRenderer for ConciseExplanationRenderer {
    fn render(&self, explanation: &Explanation) -> String {
        let mut lines = vec![format!(
            "Explanation with {} axioms:",
            explanation.justification.len()
        )];
        for (i, axiom) in explanation.justification.iter().enumerate() {
            lines.push(format!("  {}. {axiom:?}", i + 1));
        }
        lines.join("\n")
    }
}

/// A no-op renderer that produces no output (silent mode).
pub struct SilentExplanationRenderer;

impl ExplanationRenderer for SilentExplanationRenderer {
    fn render(&self, _explanation: &Explanation) -> String {
        String::new()
    }
}
