//! Pre-consistency checking before tableau execution
//!
//! This module performs fast consistency checks that can detect certain
//! inconsistencies without running the full tableau algorithm.
//!
//! IMPORTANT: Consistency vs. Coherence
//! - **Consistent**: The ontology has at least one valid model (no contradictions about individuals)
//! - **Coherent**: All named classes can potentially have instances (no unsatisfiable classes)
//!
//! EquivalentClasses(A,B) + DisjointClasses(A,B) makes A and B **unsatisfiable** (incoherent),
//! but the ontology remains **consistent** (there exists a model where A and B are both empty).
//! Therefore, we do not check for equivalence-disjointness violations in pre-consistency checks.

use crate::core::tableau::disjointness::DisjointnessMap;
use crate::core::tableau::equivalence::EquivalenceClosure;
use crate::ontology::Ontology;
use crate::Result;

/// Performs pre-consistency checks before running tableau
///
/// This checker can quickly detect certain inconsistencies that involve
/// direct contradictions about individuals, functional properties, or
/// cardinality constraints.
///
/// NOTE: This checker does NOT detect incoherence (unsatisfiable classes).
/// Unsatisfiable classes do not make an ontology inconsistent; they only
/// make it incoherent. Use `is_class_satisfiable()` to detect unsatisfiable classes.
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
    ///
    /// NOTE: EquivalentClasses(A,B) + DisjointClasses(A,B) does NOT make the
    /// ontology inconsistent - it makes classes A and B unsatisfiable (incoherent).
    /// An ontology is still consistent if it has unsatisfiable classes.
    /// Therefore, we no longer check for equivalence-disjointness violations here.
    pub fn check(&mut self) -> Result<()> {
        log::info!("Running pre-consistency checks");

        // Currently, there are no fast pre-consistency checks that can detect
        // actual inconsistencies (as opposed to incoherence/unsatisfiable classes).
        //
        // Future checks could include:
        // - Direct contradictory assertions about individuals
        // - Violations of functional properties
        // - Cardinality constraint violations
        //
        // But for now, we defer all consistency checking to the tableau algorithm.

        log::info!("Pre-consistency checks passed (no fast checks implemented)");
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
