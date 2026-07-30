//! Ontology Modularity — decomposition, locality, and module extraction.

pub mod locality;
pub mod extractor;
pub mod segmenter;
pub mod decomposition;
pub mod decomposer;

use crate::ontology::axioms::Axiom;
use crate::ontology::IRI;
use std::collections::{HashMap, HashSet};

/// Internal wrapper tracking axiom position and its signature.
#[derive(Debug, Clone)]
pub struct AxiomWrapper {
    pub position: usize,
    pub signature: HashSet<IRI>,
}

/// Index: entity IRI → positions of axioms containing that entity.
#[derive(Debug, Clone, Default)]
pub struct SigIndex {
    index: HashMap<IRI, HashSet<usize>>,
}

impl SigIndex {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Build the index from a list of axiom wrappers.
    pub fn build(&mut self, axioms: &[AxiomWrapper]) {
        for ax in axioms {
            for iri in &ax.signature {
                self.index.entry(iri.clone()).or_default().insert(ax.position);
            }
        }
    }

    /// Get axiom positions that mention the given IRI.
    #[must_use]
    pub fn get(&self, iri: &IRI) -> Option<&HashSet<usize>> {
        self.index.get(iri)
    }

    /// Get all axiom positions mentioning any of the given IRIs.
    #[must_use]
    pub fn positions_for_signature(&self, sig: &HashSet<IRI>) -> HashSet<usize> {
        let mut pos = HashSet::new();
        for iri in sig {
            if let Some(set) = self.index.get(iri) {
                pos.extend(set);
            }
        }
        pos
    }
}

/// Extract the signature (set of IRIs) from an axiom.
#[must_use]
pub fn axiom_signature(axiom: &Axiom) -> HashSet<IRI> {
    let mut sig = HashSet::new();
    let _ = crate::searcher::axiom_extract_iris_public(axiom, &mut sig);
    sig
}
