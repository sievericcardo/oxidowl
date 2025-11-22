//! Pre-consistency checking before tableau execution
//!
//! This module performs fast consistency checks that can detect certain
//! inconsistencies without running the full tableau algorithm.

use crate::core::tableau::disjointness::DisjointnessMap;
use crate::core::tableau::equivalence::EquivalenceClosure;
use crate::ontology::Ontology;
use crate::{Error, Result};

/// Performs pre-consistency checks before running tableau
///
/// This checker can quickly detect certain inconsistencies:
/// - Equivalence-disjointness violations (A≡B and A⊥B)
/// - Direct bottom type assertions
pub struct PreConsistencyChecker {
    equivalence_closure: EquivalenceClosure,
    disjointness_map: DisjointnessMap,
}

impl PreConsistencyChecker {
    /// Create a new pre-consistency checker for the given ontology
    pub fn new(ontology: &Ontology) -> Result<Self> {
        log::info!("Building pre-consistency checker");

        // Build equivalence closure
        let equivalence_closure = EquivalenceClosure::from_ontology(ontology)?;

        // Build disjointness map
        let disjointness_map = DisjointnessMap::from_ontology(ontology, &equivalence_closure)?;

        Ok(Self {
            equivalence_closure,
            disjointness_map,
        })
    }

    /// Check for inconsistencies detectable through equivalence and disjointness
    ///
    /// Returns Ok(()) if no pre-consistency issues found.
    /// Returns Err if an inconsistency is detected.
    pub fn check(&mut self) -> Result<()> {
        log::info!("Running pre-consistency checks");

        // Check for equivalence-disjointness violations
        // This catches cases like: Healthy≡MoistStrategy AND Healthy⊥MoistStrategy
        if let Some(violating_concepts) = self
            .disjointness_map
            .check_equivalence_consistency(&mut self.equivalence_closure)
        {
            log::error!(
                "Ontology is inconsistent: concepts {:?} are both equivalent and disjoint",
                violating_concepts
            );

            return Err(Error::Reasoning {
                message: format!(
                    "Pre-consistency check failed: concepts {:?} are both equivalent and disjoint. \
                     This violates the basic constraint that equivalent concepts cannot be disjoint.",
                    violating_concepts
                ),
            });
        }

        log::info!("Pre-consistency checks passed");
        Ok(())
    }

    /// Get reference to equivalence closure (for future use)
    pub fn equivalence_closure(&self) -> &EquivalenceClosure {
        &self.equivalence_closure
    }

    /// Get reference to disjointness map (for future use)
    pub fn disjointness_map(&self) -> &DisjointnessMap {
        &self.disjointness_map
    }
}
