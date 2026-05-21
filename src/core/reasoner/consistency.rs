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

use crate::Result;
use crate::core::tableau::disjointness::DisjointnessMap;
use crate::core::tableau::equivalence::EquivalenceClosure;
use crate::ontology::Ontology;

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

    /// Check for inconsistencies detectable through fast O(n) axiom scans.
    ///
    /// Returns Ok(()) if no pre-consistency issues found.
    /// Returns Err if an inconsistency is detected.
    ///
    /// NOTE: EquivalentClasses(A,B) + DisjointClasses(A,B) does NOT make the
    /// ontology inconsistent - it makes classes A and B unsatisfiable (incoherent).
    /// An ontology is still consistent if it has unsatisfiable classes.
    /// Therefore, we no longer check for equivalence-disjointness violations here.
    pub fn check(&mut self, ontology: &Ontology) -> Result<()> {
        use crate::ontology::ClassExpression;
        use crate::ontology::ObjectPropertyExpression;

        log::info!("Running pre-consistency checks");

        // Check 1: ClassAssertion(owl:Nothing :x) is directly inconsistent.
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::ClassAssertion(ca) = axiom {
                if let ClassExpression::Class(cls) = &ca.class {
                    if cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                        return Err(crate::error::Error::reasoning(
                            "Inconsistency: individual asserted to be in owl:Nothing",
                        ));
                    }
                }
            }
        }

        // Check 2: SubClassOf(owl:Thing owl:Nothing) is tautologically inconsistent.
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(sa) = axiom {
                let sub_is_thing = matches!(&sa.subclass,
                    ClassExpression::Class(c)
                        if c.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing"
                );
                let sup_is_nothing = matches!(&sa.superclass,
                    ClassExpression::Class(c)
                        if c.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing"
                );
                if sub_is_thing && sup_is_nothing {
                    return Err(crate::error::Error::reasoning(
                        "Inconsistency: SubClassOf(owl:Thing owl:Nothing)",
                    ));
                }
            }
        }

        // Check 3: FunctionalObjectProperty violated by two distinct targets whose
        // merged class memberships are mutually exclusive via SubClassOf(X ObjectComplementOf(Y)).
        let mut functional_props: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::FunctionalObjectProperty(fp) = axiom {
                if let ObjectPropertyExpression::ObjectProperty(op) = &fp.property {
                    functional_props.insert(op.iri.as_str().to_string());
                }
            }
        }

        if !functional_props.is_empty() {
            // Group ObjectPropertyAssertions by (property_iri, source_key).
            let mut prop_targets: std::collections::HashMap<(String, String), Vec<String>> =
                std::collections::HashMap::new();
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(opa) = axiom {
                    if let ObjectPropertyExpression::ObjectProperty(op) = &opa.property {
                        let prop_iri = op.iri.as_str().to_string();
                        if functional_props.contains(&prop_iri) {
                            let src = format!("{:?}", opa.source);
                            let tgt = format!("{:?}", opa.target);
                            prop_targets.entry((prop_iri, src)).or_default().push(tgt);
                        }
                    }
                }
            }

            // Collect class memberships of each individual (key = Debug string of Individual).
            let mut ind_classes: std::collections::HashMap<String, Vec<ClassExpression>> =
                std::collections::HashMap::new();
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(ca) = axiom {
                    let ind_key = format!("{:?}", ca.individual);
                    ind_classes.entry(ind_key).or_default().push(ca.class.clone());
                }
            }

            for ((_, _), targets) in &prop_targets {
                // Need at least 2 distinct targets for a violation.
                let distinct: std::collections::HashSet<&String> = targets.iter().collect();
                if distinct.len() < 2 {
                    continue;
                }
                // Merge class memberships of all targets (they are forced equal by functionality).
                let mut merged: Vec<ClassExpression> = Vec::new();
                for tgt in &distinct {
                    if let Some(cls) = ind_classes.get(*tgt) {
                        merged.extend_from_slice(cls);
                    }
                }
                // Check if any pair (ci, cj) in merged satisfies SubClassOf(ci ObjectComplementOf(cj)).
                for ci in &merged {
                    for cj in &merged {
                        if ci == cj {
                            continue;
                        }
                        for ax in ontology.axioms() {
                            if let crate::ontology::Axiom::SubClassOf(sub) = ax {
                                if &sub.subclass == ci {
                                    if let ClassExpression::ObjectComplementOf(inner) = &sub.superclass {
                                        if inner.as_ref() == cj {
                                            return Err(crate::error::Error::reasoning(
                                                "Inconsistency: functional property forces individuals \
                                                 with mutually exclusive class memberships to be equal",
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

        log::info!("Pre-consistency checks passed");
        Ok(())
    }

    /// Get reference to equivalence closure (for future use)
    #[must_use]
    pub fn equivalence_closure(&self) -> &EquivalenceClosure {
        &self.equivalence_closure
    }

    /// Get reference to disjointness map (for future use)
    #[must_use]
    pub fn disjointness_map(&self) -> &DisjointnessMap {
        &self.disjointness_map
    }
}
