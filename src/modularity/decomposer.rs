//! Atomic Decomposer — configuration wrapper for decomposition.

use super::decomposition::{AtomicDecomposition, compute_atomic_decomposition};
use super::locality::LocalityClass;
use crate::ontology::Ontology;
use std::time::Duration;

/// Configuration for the atomic decomposer.
#[derive(Debug, Clone)]
pub struct DecomposerConfig {
    pub locality: LocalityClass,
    pub timeout: Option<Duration>,
}

impl Default for DecomposerConfig {
    fn default() -> Self {
        Self { locality: LocalityClass::Star, timeout: None }
    }
}

/// Orchestrates the atomic decomposition computation.
#[derive(Debug, Clone)]
pub struct AtomicDecomposer {
    #[allow(dead_code)]
    config: DecomposerConfig,
}

impl AtomicDecomposer {
    #[must_use]
    pub fn new(config: DecomposerConfig) -> Self { Self { config } }

    /// Decompose an ontology into atoms using signature-based locality.
    #[must_use]
    pub fn decompose(&self, ontology: &Ontology) -> AtomicDecomposition {
        compute_atomic_decomposition(ontology)
    }
}

impl Default for AtomicDecomposer {
    fn default() -> Self { Self::new(DecomposerConfig::default()) }
}
