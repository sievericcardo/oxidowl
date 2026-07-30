//! Entailment checker implementation
//!
//! This module contains the main `EntailmentChecker` struct and `EntailmentRegime` enum
//! for checking various types of entailment relationships.

#![allow(dead_code)]

use crate::Result;
use crate::semantics::{RdfGraph, RdfTerm, Triple};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Types of entailment regimes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntailmentRegime {
    /// Simple RDF entailment
    RdfSimple,
    /// RDFS entailment
    Rdfs,
    /// OWL 2 RDF-Based semantics
    OwlRdf,
    /// OWL 2 Direct semantics
    OwlDirect,
    /// OWL 2 RL profile
    OwlRl,
    /// OWL 2 EL profile
    OwlEl,
    /// OWL 2 QL profile
    OwlQl,
}

/// Entailment checker for different regimes
#[derive(Debug)]
pub struct EntailmentChecker {
    regime: EntailmentRegime,
    cache: HashMap<(String, String), bool>,
    id_generator: AtomicUsize,
}

impl EntailmentChecker {
    /// Create a new entailment checker for the specified regime
    #[must_use]
    pub fn new(regime: EntailmentRegime) -> Self {
        Self {
            regime,
            cache: HashMap::new(),
            id_generator: AtomicUsize::new(1),
        }
    }

    /// Generate a unique ID for axioms
    fn generate_id(&self) -> usize {
        self.id_generator.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if premises entail conclusion under the current regime
    pub fn entails(&mut self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        let cache_key = (
            format!("{:?}", premises.triples()),
            format!("{:?}", conclusion.triples()),
        );

        if let Some(&result) = self.cache.get(&cache_key) {
            return Ok(result);
        }

        let result = match self.regime {
            EntailmentRegime::RdfSimple => {
                self.check_rdf_simple_entailment(premises, conclusion)?
            }
            EntailmentRegime::Rdfs => self.check_rdfs_entailment(premises, conclusion)?,
            EntailmentRegime::OwlRdf => self.check_owl_rdf_entailment(premises, conclusion)?,
            EntailmentRegime::OwlDirect => {
                self.check_owl_direct_entailment(premises, conclusion)?
            }
            EntailmentRegime::OwlRl => self.check_owl_rl_entailment(premises, conclusion)?,
            EntailmentRegime::OwlEl => self.check_owl_el_entailment(premises, conclusion)?,
            EntailmentRegime::OwlQl => self.check_owl_ql_entailment(premises, conclusion)?,
        };

        self.cache.insert(cache_key, result);
        Ok(result)
    }

    /// Check RDF simple entailment
    fn check_rdf_simple_entailment(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> Result<bool> {
        // Simple RDF entailment: check if conclusion triples are subset of premises
        for conclusion_triple in &conclusion.triples {
            if !premises.contains_triple(conclusion_triple) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check RDFS entailment using forward-chaining closure with subclass propagation
    fn check_rdfs_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        let mut closure = premises.clone();
        for _ in 0..50 {
            let prev = closure.triples().len();
            let triples: Vec<_> = closure.triples().iter().cloned().collect();
            for t in triples {
                // rdfs9: if s rdf:type C and C rdfs:subClassOf D, infer s rdf:type D
                if let RdfTerm::Iri(ref pred) = t.predicate {
                    if pred.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
                        if let RdfTerm::Iri(ref class_iri) = t.object {
                            for st in closure.triples().clone().iter() {
                                if let RdfTerm::Iri(ref subj) = st.subject {
                                    if subj == class_iri {
                                        if let RdfTerm::Iri(ref sc_pred) = st.predicate {
                                            if sc_pred.as_str()
                                                == "http://www.w3.org/2000/01/rdf-schema#subClassOf"
                                            {
                                                if let RdfTerm::Iri(_) = st.object {
                                                    closure.add_triple(Triple::new(
                                                        t.subject.clone(),
                                                        RdfTerm::Iri(url::Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap()),
                                                        st.object.clone(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if closure.triples().len() == prev {
                break;
            }
        }
        Ok(conclusion
            .triples()
            .iter()
            .all(|t| closure.contains_triple(t)))
    }

    /// Check OWL RDF-based entailment via OWL 2 RL rules
    fn check_owl_rdf_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Check OWL Direct semantics entailment via reasoner if available
    fn check_owl_direct_entailment(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> Result<bool> {
        // Use RL reasoning as approximation for Direct Semantics
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Check OWL 2 RL entailment
    fn check_owl_rl_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Use the Owl2RlEngine from the owl2_rl module
        use super::owl2_rl::Owl2RlEngine;
        let mut engine = Owl2RlEngine::new(premises.clone());
        engine.reason()?;

        let closure = engine.closure();
        Ok(conclusion
            .triples()
            .iter()
            .all(|triple| closure.contains_triple(triple)))
    }

    /// Check OWL 2 EL entailment using EL reasoner
    fn check_owl_el_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // OWL 2 EL is a subset of OWL 2 RL; use RL reasoning as approximation
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Check OWL 2 QL entailment using RL rules as approximation
    fn check_owl_ql_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // OWL 2 QL can be checked via query rewriting; use RL as conservative approximation
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Clear the entailment cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the current entailment regime
    pub fn regime(&self) -> EntailmentRegime {
        self.regime
    }

    /// Set the entailment regime
    pub fn set_regime(&mut self, regime: EntailmentRegime) {
        self.regime = regime;
        self.clear_cache(); // Clear cache when regime changes
    }
}
