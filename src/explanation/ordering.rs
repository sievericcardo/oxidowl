//! Explanation Ordering — rank and sort explanations by preference.
//!
//! Multiple ordering strategies can be composed for flexible
//! explanation ranking.

use crate::explanation::generator::Explanation;

/// Orders multiple explanations from most to least preferred.
pub trait ExplanationOrderer: Send + Sync {
    fn order(&self, explanations: Vec<Explanation>) -> Vec<Explanation>;
}

/// Orders by preference for smaller justifications (fewer axioms).
pub struct JustificationSizeOrderer;

impl ExplanationOrderer for JustificationSizeOrderer {
    fn order(&self, mut explanations: Vec<Explanation>) -> Vec<Explanation> {
        explanations.sort_by(|a, b| a.justification.len().cmp(&b.justification.len()));
        explanations
    }
}

/// Composite orderer that applies multiple ordering strategies in sequence.
pub struct CompositeExplanationOrderer {
    orderers: Vec<Box<dyn ExplanationOrderer>>,
}

impl CompositeExplanationOrderer {
    #[must_use]
    pub fn new(orderers: Vec<Box<dyn ExplanationOrderer>>) -> Self {
        Self { orderers }
    }
}

impl ExplanationOrderer for CompositeExplanationOrderer {
    fn order(&self, mut explanations: Vec<Explanation>) -> Vec<Explanation> {
        for orderer in &self.orderers {
            explanations = orderer.order(explanations);
        }
        explanations
    }
}

/// Called during explanation search to track progress.
pub trait ExplanationProgressMonitor: Send + Sync {
    fn found_explanation(&self, index: usize, explanation: &Explanation);
    fn progress_update(&self, found: usize, estimated_total: Option<usize>);
    fn is_cancelled(&self) -> bool;
}

/// A no-op progress monitor.
pub struct SilentExplanationProgressMonitor;

impl ExplanationProgressMonitor for SilentExplanationProgressMonitor {
    fn found_explanation(&self, _index: usize, _explanation: &Explanation) {}
    fn progress_update(&self, _found: usize, _estimated_total: Option<usize>) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}
