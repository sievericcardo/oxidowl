//! Hitting Set Tree (HST) explanation generator.
//! Computes ALL minimal justifications for an entailment.

use super::converter::SatisfiabilityConverter;
use super::generator::{Explanation, ExplanationGenerator};
use crate::ontology::axioms::{Axiom, AxiomId, AxiomTrait};
use crate::ontology::OntologyRef;
use crate::reasoner_api::ReasonerFactory;
use crate::Result;
use std::collections::HashSet;
use std::sync::Arc;

/// Configuration for HST-based explanation generation.
#[derive(Debug, Clone)]
pub struct HSTConfig {
    pub max_depth: usize,
    pub max_justifications: usize,
}

impl Default for HSTConfig {
    fn default() -> Self { Self { max_depth: 20, max_justifications: 50 } }
}

/// Hitting Set Tree generator — finds multiple minimal justifications.
pub struct HSTExplanationGenerator {
    factory: Arc<dyn ReasonerFactory>,
    config: HSTConfig,
}

impl HSTExplanationGenerator {
    #[must_use]
    pub fn new(factory: Arc<dyn ReasonerFactory>, config: HSTConfig) -> Self {
        Self { factory, config }
    }

    /// Find up to `limit` minimal justifications.
    pub fn find_justifications(
        &self,
        ontology: &OntologyRef,
        entailment: &Axiom,
        limit: usize,
    ) -> Result<Vec<Vec<Axiom>>> {
        let axioms: Vec<Axiom> = {
            let guard = ontology.read().map_err(|e| crate::Error::Internal { message: format!("{e}") })?;
            guard.axioms().to_vec()
        };

        // Step 1: Find one justification using the shrink algorithm
        let _converter = SatisfiabilityConverter;
        let mut justifications = Vec::new();

        // Simple re-implementation of black-box expand-shrink per axiom subset
        let j0 = self.expand_shrink(&axioms, entailment)?;
        if j0.is_empty() {
            return Ok(vec![]);
        }
        justifications.push(j0.clone());

        if limit <= 1 || justifications.len() >= limit {
            return Ok(justifications);
        }

        // Step 2: Build HST to find additional justifications
        let mut visited: HashSet<Vec<u64>> = HashSet::new();
        let j0_ids: Vec<u64> = j0.iter().map(|a| a.axiom_id()).collect();
        visited.insert(j0_ids);

        for _depth in 0..self.config.max_depth {
            if justifications.len() >= limit { break; }
            let last = justifications[justifications.len() - 1].clone();
            for ax in &last {
                let filtered: Vec<Axiom> = axioms.iter()
                    .filter(|a| a.axiom_id() != ax.axiom_id())
                    .cloned()
                    .collect();
                if let Ok(j) = self.expand_shrink(&filtered, entailment) {
                    let j_ids: Vec<u64> = j.iter().map(|a| a.axiom_id()).collect();
                    if !j.is_empty() && visited.insert(j_ids) {
                        justifications.push(j);
                        if justifications.len() >= limit { break; }
                    }
                }
            }
        }

        Ok(justifications)
    }

    fn expand_shrink(&self, axioms: &[Axiom], entailment: &Axiom) -> Result<Vec<Axiom>> {
        if axioms.is_empty() { return Ok(vec![]); }

        // Expand: start with all, remove each to see if still entailed
            let mut essential: HashSet<AxiomId> = HashSet::new();
        for ax in axioms {
            let test_set: Vec<Axiom> = axioms.iter()
                .filter(|a| a.axiom_id() != ax.axiom_id())
                .cloned()
                .collect();
            if test_set.is_empty() { continue; }
            let onto = Self::build_onto(&test_set);
            if let Ok(reasoner) = self.factory.create_reasoner(&onto, &Default::default()) {
                if reasoner.is_entailed(entailment).unwrap_or(false) {
                    // ax is NOT essential
                } else {
                    essential.insert(ax.axiom_id());
                }
            }
        }

        // Shrink: try removing each essential
        let mut minimal: Vec<Axiom> = Vec::new();
        for ax in axioms.iter().filter(|a| essential.contains(&a.axiom_id())) {
            let test_set: Vec<Axiom> = minimal.iter().chain(std::iter::once(ax)).cloned().collect();
            let onto = Self::build_onto(&test_set);
            if let Ok(reasoner) = self.factory.create_reasoner(&onto, &Default::default()) {
                if reasoner.is_entailed(entailment).unwrap_or(false) {
                    minimal.push(ax.clone());
                }
            }
        }

        Ok(minimal)
    }

    fn build_onto(axioms: &[Axiom]) -> OntologyRef {
        let mut o = crate::ontology::Ontology::new();
        for ax in axioms { o.add_axiom(ax.clone()); }
        OntologyRef::new(std::sync::RwLock::new(o))
    }
}

impl ExplanationGenerator for HSTExplanationGenerator {
    fn get_explanation(&self, _entailment: &Axiom) -> Result<Explanation> {
        Err(crate::Error::Unsupported { message: "HST requires explicit ontology".into() })
    }

    fn get_explanations(&self, _entailment: &Axiom, _limit: usize) -> Result<Vec<Explanation>> {
        Err(crate::Error::Unsupported { message: "HST requires explicit ontology".into() })
    }
}
