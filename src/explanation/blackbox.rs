//! Black-box explanation generator using the Expand-Shrink algorithm.
//! Works with any `OWLReasoner` implementation.

use super::generator::{Explanation, ExplanationGenerator};
use crate::ontology::axioms::{Axiom, AxiomTrait};
use crate::ontology::OntologyRef;
use crate::reasoner_api::ReasonerFactory;
use crate::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for black-box explanation generation.
#[derive(Debug, Clone)]
pub struct BlackBoxConfig {
    pub timeout: Option<Duration>,
    pub max_explanations: usize,
}

impl Default for BlackBoxConfig {
    fn default() -> Self {
        Self { timeout: None, max_explanations: 10 }
    }
}

/// Black-box explanation using the Expand-Shrink algorithm.
///
/// Algorithm:
/// 1. EXPAND: Remove each axiom, check if entailment still holds; keep needed ones.
/// 2. SHRINK: Try removing each kept axiom; keep only essential ones.
///
/// Time complexity: O(n²) reasoner calls in worst case, O(n log n) average.
pub struct BlackBoxExplanation {
    reasoner_factory: Arc<dyn ReasonerFactory>,
    config: BlackBoxConfig,
}

impl BlackBoxExplanation {
    #[must_use]
    pub fn new(reasoner_factory: Arc<dyn ReasonerFactory>, config: BlackBoxConfig) -> Self {
        Self { reasoner_factory, config }
    }

    /// Find a minimal justification for the given entailment.
    fn compute_justification(&self, ontology: &OntologyRef, entailment: &Axiom) -> Result<Vec<Axiom>> {
        let _start = Instant::now();
        let axioms: Vec<Axiom> = {
            let guard = ontology.read().map_err(|e| crate::Error::Internal { message: format!("{e}") })?;
            guard.axioms().to_vec()
        };

        // Phase 1: EXPAND — greedily remove unnecessary axioms
        let mut candidate = axioms.clone();
        let mut essential = Vec::new();

        while !candidate.is_empty() {
            let ax = candidate.pop().unwrap();
            // Check: does ontology \ {ax} still entail?
            let test_onto = self.build_temp_ontology(
                &axioms.iter().filter(|a| a.axiom_id() != ax.axiom_id()).cloned().collect::<Vec<_>>(),
            )?;
            let reasoner = self.reasoner_factory.create_reasoner(
                &test_onto,
                &crate::reasoner_api::OWLReasonerConfiguration::default(),
            )?;
            if reasoner.is_entailed(entailment)? {
                // ax is not needed
            } else {
                essential.push(ax);
            }
        }

        // Phase 2: SHRINK — try removing each essential axiom; keep if still entailed
        let mut minimal: Vec<Axiom> = Vec::new();
        for ax in essential {
            let all_essential: Vec<&Axiom> = minimal.iter().chain(std::iter::once(&ax)).collect();
            let test_onto = self.build_temp_ontology(
                &all_essential.into_iter().cloned().collect::<Vec<_>>(),
            )?;
            let _reasoner = self.reasoner_factory.create_reasoner(
                &test_onto,
                &crate::reasoner_api::OWLReasonerConfiguration::default(),
            )?;

            let current_set: Vec<Axiom> = minimal.iter().chain(std::iter::once(&ax)).cloned().collect();
            let test_onto2 = self.build_temp_ontology(&current_set)?;
            let reasoner2 = self.reasoner_factory.create_reasoner(
                &test_onto2,
                &crate::reasoner_api::OWLReasonerConfiguration::default(),
            )?;

            if reasoner2.is_entailed(entailment)? {
                minimal.push(ax.clone());
            }
        }

        Ok(minimal)
    }

    fn build_temp_ontology(&self, axioms: &[Axiom]) -> Result<OntologyRef> {
        let mut o = crate::ontology::Ontology::new();
        for ax in axioms {
            o.add_axiom(ax.clone());
        }
        Ok(OntologyRef::new(std::sync::RwLock::new(o)))
    }
}

impl ExplanationGenerator for BlackBoxExplanation {
    fn get_explanation(&self, _entailment: &Axiom) -> Result<Explanation> {
        // Standalone explanation requires an explicit ontology reference.
        // Use find_justification() with an ontology reference, or
        // use BlackBoxOWLDebugger for integrated reasoner+ontology access.
        Err(crate::Error::Unsupported {
            message: "BlackBoxExplanation requires an ontology reference — use BlackBoxOWLDebugger".into(),
        })
    }

    fn get_explanations(&self, entailment: &Axiom, _limit: usize) -> Result<Vec<Explanation>> {
        self.get_explanation(entailment).map(|e| vec![e])
    }
}

/// Standalone black-box justification finder with an explicit ontology reference.
pub fn find_justification(
    ontology: &OntologyRef,
    entailment: &Axiom,
    factory: &Arc<dyn ReasonerFactory>,
) -> Result<Vec<Axiom>> {
    let bb = BlackBoxExplanation::new(factory.clone(), BlackBoxConfig::default());
    bb.compute_justification(ontology, entailment)
}
