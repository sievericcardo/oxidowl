//! Explanation Generator trait and implementations.

use crate::ontology::axioms::Axiom;
use crate::Result;
use std::sync::RwLock;
use std::time::Duration;

/// A minimal justification for an entailment.
#[derive(Debug, Clone)]
pub struct Explanation {
    pub entailment: Axiom,
    pub justification: Vec<Axiom>,
    pub is_minimal: bool,
    pub computation_time: Duration,
}

/// Generates explanations for why a reasoner entails something.
pub trait ExplanationGenerator: Send + Sync {
    fn get_explanation(&self, entailment: &Axiom) -> Result<Explanation>;
    fn get_explanations(&self, entailment: &Axiom, limit: usize) -> Result<Vec<Explanation>>;
    fn get_all_explanations(&self, entailment: &Axiom) -> Result<Vec<Explanation>> {
        self.get_explanations(entailment, usize::MAX)
    }
}

// ── SingleExplanationGenerator ───────────────────────────────────────────────

/// Wraps an ExplanationGenerator with lazy caching of a single result.
pub struct SingleExplanationGenerator {
    inner: Box<dyn ExplanationGenerator>,
    cached: RwLock<Option<Explanation>>,
    entailment: Axiom,
}

impl SingleExplanationGenerator {
    #[must_use]
    pub fn new(inner: Box<dyn ExplanationGenerator>, entailment: Axiom) -> Self {
        Self { inner, cached: RwLock::new(None), entailment }
    }

    /// Get the explanation, computing only once (lazy).
    pub fn get_explanation(&self) -> Result<Explanation> {
        if let Some(ref cached) = *self.cached.read().unwrap_or_else(|e| e.into_inner()) {
            return Ok(cached.clone());
        }
        let explanation = self.inner.get_explanation(&self.entailment)?;
        let mut guard = self.cached.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(explanation.clone());
        Ok(explanation)
    }
}
