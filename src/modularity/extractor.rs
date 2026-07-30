//! Module Extractor — extracts ontology modules for a given signature.

use super::locality::{LocalityClass, LocalityEvaluator, SyntacticLocalityEvaluator};
use crate::ontology::axioms::Axiom;
use crate::ontology::Ontology;
use super::axiom_signature;
use std::collections::HashSet;

/// The type of module to extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    /// Upper bound (⊥-module): conservative over-approximation.
    UpperBound,
    /// Lower bound (⊤-module): conservative under-approximation.
    LowerBound,
    /// Star module (∅-module): optimal for many reasoning tasks.
    Star,
}

/// Configuration for module extraction.
#[derive(Debug, Clone)]
pub struct ModuleExtractorConfig {
    pub module_type: ModuleType,
    pub max_iterations: usize,
}

impl Default for ModuleExtractorConfig {
    fn default() -> Self { Self { module_type: ModuleType::Star, max_iterations: 1000 } }
}

/// Extracts a module of an ontology for a given signature — a subset of
/// axioms that preserves all entailments over the signature.
pub struct ModuleExtractor {
    locality_evaluator: Box<dyn LocalityEvaluator>,
    config: ModuleExtractorConfig,
}

impl ModuleExtractor {
    #[must_use]
    pub fn new(locality_evaluator: Box<dyn LocalityEvaluator>, config: ModuleExtractorConfig) -> Self {
        Self { locality_evaluator, config }
    }

    /// Create a syntactic extractor with the given locality class.
    #[must_use]
    pub fn new_syntactic(locality_class: LocalityClass, config: ModuleExtractorConfig) -> Self {
        Self {
            locality_evaluator: Box::new(SyntacticLocalityEvaluator::new(locality_class)),
            config,
        }
    }

    /// Extract a module from the ontology for the given signature.
    ///
    /// Algorithm:
    /// 1. Start with empty module M = ∅
    /// 2. sig_M = signature
    /// 3. For each axiom α not in M:
    ///    If α is NOT local w.r.t. sig_M, add α to M and extend sig_M
    /// 4. Repeat until fixpoint
    #[must_use]
    pub fn extract_module(
        &self,
        ontology: &Ontology,
        signature: &HashSet<crate::ontology::IRI>,
    ) -> Ontology {
        let axioms = ontology.axioms().to_vec();
        let mut module_axioms: Vec<Axiom> = Vec::new();
        let mut module_sig: HashSet<crate::ontology::IRI> = signature.clone();
        let mut in_module: HashSet<usize> = HashSet::new();

        for _iteration in 0..self.config.max_iterations {
            let mut changed = false;
            for (i, axiom) in axioms.iter().enumerate() {
                if in_module.contains(&i) { continue; }
                if !self.locality_evaluator.is_local(axiom, &module_sig) {
                    module_axioms.push(axiom.clone());
                    in_module.insert(i);
                    let ax_sig = axiom_signature(axiom);
                    module_sig.extend(ax_sig);
                    changed = true;
                }
            }
            if !changed { break; }
        }

        let mut result = Ontology::new();
        for ax in module_axioms {
            result.add_axiom(ax);
        }
        result
    }
}
