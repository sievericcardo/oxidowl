//! GCI absorption for the classification fast path (workstream A8).
//!
//! Absorbs `TBox` general concept inclusions (GCIs) into deterministic clauses
//! that the saturation engine and the dependency-graph propagation can
//! discharge *without* invoking the full tableau:
//!
//! - `A ⊑ ∃R.C` (atomic class, deterministic filler) → a **role-successor**
//!   clause: "whenever `x ∈ A`, `x` has an `R`-successor in `C`".
//! - `∃R.⊤ ⊑ D` / `ObjectPropertyDomain(R, D)` → a **role-domain** clause:
//!   "any `R`-successor source is in `D`".
//! - `⊤ ⊑ ∀R.D` / `ObjectPropertyRange(R, D)` → a **role-range** clause.
//! - `∃R.C ⊑ D` → a **role-range GCI**: fires on any class that implies
//!   `∃R.C'` with `C' ⊑ C`, adding `D`.
//! - `A₁ ⊓ … ⊓ Aₖ ⊑ D` (atomic conjuncts, atomic head) → a **conjunction**
//!   clause: fires on any class subsumed by all of `A₁..Aₖ`.
//!
//! Everything else (disjunction, complement, cardinalities, nominals, …) is
//! classified as genuinely non-deterministic and is pushed to the tableau
//! fallback.  All absorbed rules are *sound* DL-EL rules; the propagation never
//! invents a subsumption that does not hold.

use crate::ontology::{ClassExpression, ObjectPropertyExpression, Ontology, axioms::Axiom};
use std::collections::{HashMap, HashSet};

/// A role-successor clause: `A ⊑ ∃R.C` with `A` atomic and `C` deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleSuccessorClause {
    pub property: ObjectPropertyExpression,
    pub filler: ClassExpression,
}

/// A role-range GCI: `∃R.C ⊑ D` (fires when a class implies `∃R.C'`, `C' ⊑ C`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleRangeGci {
    pub property: ObjectPropertyExpression,
    pub filler: ClassExpression,
    pub head: ClassExpression,
}

/// A conjunction GCI: `A₁ ⊓ … ⊓ Aₖ ⊑ D`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConjunctionGci {
    pub conjuncts: Vec<ClassExpression>,
    pub head: ClassExpression,
}

/// A GCI that could not be absorbed (requires the tableau fallback).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemainingGci {
    pub subclass: ClassExpression,
    pub superclass: ClassExpression,
}

/// Result of absorbing an ontology's GCIs.
#[derive(Debug, Clone, Default)]
pub struct AbsorptionResult {
    /// Role-successor clauses indexed by their (atomic) source class.
    pub role_successors: HashMap<ClassExpression, Vec<RoleSuccessorClause>>,
    /// Role-domain clauses indexed by property.
    pub role_domains: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,
    /// Role-range clauses indexed by property.
    pub role_ranges: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,
    /// Role-range GCIs `∃R.C ⊑ D` (propagation-time firing).
    pub role_range_gcis: Vec<RoleRangeGci>,
    /// Conjunction GCIs `A₁ ⊓ … ⊓ Aₖ ⊑ D`.
    pub conjunction_gcis: Vec<ConjunctionGci>,
    /// GCIs that require the tableau fallback.
    pub remaining_gcis: Vec<RemainingGci>,

    /// Number of `SubClassOf` GCIs inspected.
    pub total_gcis: usize,
    /// Number of GCIs fully absorbed (deterministic).
    pub absorbed_count: usize,
    /// Number of GCIs that remain for the tableau.
    pub remaining_count: usize,
    /// Number of atomic `A ⊑ B` implications (handled by told subsumption).
    pub atomic_implications: usize,
}

impl AbsorptionResult {
    /// Number of absorbed role-successor clauses.
    #[must_use]
    pub fn role_successor_count(&self) -> usize {
        self.role_successors.values().map(Vec::len).sum()
    }

    /// Total number of absorbed deterministic clauses.
    #[must_use]
    pub fn deterministic_clause_count(&self) -> usize {
        self.role_successor_count()
            + self.role_domains.values().map(Vec::len).sum::<usize>()
            + self.role_ranges.values().map(Vec::len).sum::<usize>()
            + self.role_range_gcis.len()
            + self.conjunction_gcis.len()
    }
}

/// Returns `true` if `expr` contains a construct that genuinely requires the
/// tableau: disjunction, complement, cardinalities (≥ 2 / ≤ / =), nominals,
/// `hasValue` / `hasSelf`, or data cardinalities.
#[must_use]
pub fn is_non_deterministic(expr: &ClassExpression) -> bool {
    match expr {
        ClassExpression::Class(_) => false,
        ClassExpression::ObjectIntersectionOf(parts) => parts.iter().any(is_non_deterministic),
        ClassExpression::ObjectUnionOf(_) => true,
        ClassExpression::ObjectComplementOf(_) => true,
        ClassExpression::ObjectOneOf(_) => true,
        ClassExpression::ObjectSomeValuesFrom { filler, .. } => is_non_deterministic(filler),
        ClassExpression::ObjectAllValuesFrom { filler, .. } => is_non_deterministic(filler),
        ClassExpression::ObjectHasValue { .. } => true,
        ClassExpression::ObjectHasSelf { .. } => true,
        ClassExpression::ObjectMinCardinality {
            cardinality,
            filler,
            ..
        } => *cardinality >= 2 || is_non_deterministic(filler),
        ClassExpression::ObjectMaxCardinality { .. } => true,
        ClassExpression::ObjectExactCardinality { .. } => true,
        ClassExpression::DataSomeValuesFrom { .. } => false,
        ClassExpression::DataAllValuesFrom { .. } => false,
        ClassExpression::DataHasValue { .. } => true,
        ClassExpression::DataMinCardinality { .. } => true,
        ClassExpression::DataMaxCardinality { .. } => true,
        ClassExpression::DataExactCardinality { .. } => true,
    }
}

/// Collect every named class mentioned anywhere inside `expr` (recursively).
pub fn collect_named_classes<S: std::hash::BuildHasher>(
    expr: &ClassExpression,
    out: &mut HashSet<ClassExpression, S>,
) {
    match expr {
        ClassExpression::Class(_) => {
            out.insert(expr.clone());
        }
        ClassExpression::ObjectIntersectionOf(parts) | ClassExpression::ObjectUnionOf(parts) => {
            for part in parts {
                collect_named_classes(part, out);
            }
        }
        ClassExpression::ObjectSomeValuesFrom { filler, .. }
        | ClassExpression::ObjectAllValuesFrom { filler, .. }
        | ClassExpression::ObjectMinCardinality { filler, .. }
        | ClassExpression::ObjectMaxCardinality { filler, .. }
        | ClassExpression::ObjectExactCardinality { filler, .. }
        | ClassExpression::ObjectComplementOf(filler) => {
            collect_named_classes(filler, out);
        }
        _ => {}
    }
}

/// Is `expr` the top class (`owl:Thing`)?
#[must_use]
pub fn is_top(expr: &ClassExpression) -> bool {
    matches!(expr, ClassExpression::Class(c) if c.is_thing())
}

/// Absorb the ontology's GCIs into deterministic clauses.
#[must_use]
pub fn absorb(ontology: &Ontology) -> AbsorptionResult {
    let mut result = AbsorptionResult::default();

    for axiom in ontology.axioms() {
        match axiom {
            Axiom::SubClassOf(ax) => {
                result.total_gcis += 1;
                absorb_gci(&ax.subclass, &ax.superclass, &mut result);
            }
            Axiom::ObjectPropertyDomain(ax) => {
                result
                    .role_domains
                    .entry(ax.property.clone())
                    .or_default()
                    .push(ax.domain.clone());
                result.absorbed_count += 1;
            }
            Axiom::ObjectPropertyRange(ax) => {
                result
                    .role_ranges
                    .entry(ax.property.clone())
                    .or_default()
                    .push(ax.range.clone());
                result.absorbed_count += 1;
            }
            Axiom::EquivalentClasses(_) => {
                // Handled by told subsumption (mutual subsumers).
                result.atomic_implications += 1;
                result.absorbed_count += 1;
            }
            _ => {}
        }
    }

    result
}

/// Absorb a single `C ⊑ D` GCI.
fn absorb_gci(
    subclass: &ClassExpression,
    superclass: &ClassExpression,
    result: &mut AbsorptionResult,
) {
    // Anything non-deterministic on either side cannot be absorbed safely.
    if is_non_deterministic(subclass) || is_non_deterministic(superclass) {
        result.remaining_gcis.push(RemainingGci {
            subclass: subclass.clone(),
            superclass: superclass.clone(),
        });
        result.remaining_count += 1;
        return;
    }

    // `A ⊑ ∃R.C` — role-successor absorption.
    if let ClassExpression::Class(_) = subclass
        && let ClassExpression::ObjectSomeValuesFrom { property, filler } = superclass
    {
        result
            .role_successors
            .entry(subclass.clone())
            .or_default()
            .push(RoleSuccessorClause {
                property: property.clone(),
                filler: (**filler).clone(),
            });
        result.absorbed_count += 1;
        return;
    }

    // `∃R.⊤ ⊑ D` — role-domain absorption.
    if let ClassExpression::ObjectSomeValuesFrom { property, filler } = subclass
        && is_top(filler)
        && let ClassExpression::Class(_) = superclass
    {
        result
            .role_domains
            .entry(property.clone())
            .or_default()
            .push(superclass.clone());
        result.absorbed_count += 1;
        return;
    }

    // `∃R.C ⊑ D` — role-range GCI absorption.
    if let ClassExpression::ObjectSomeValuesFrom { property, filler } = subclass
        && let ClassExpression::Class(_) = superclass
    {
        result.role_range_gcis.push(RoleRangeGci {
            property: property.clone(),
            filler: (**filler).clone(),
            head: superclass.clone(),
        });
        result.absorbed_count += 1;
        return;
    }

    // `⊤ ⊑ ∀R.C` — role-range absorption.
    if is_top(subclass)
        && let ClassExpression::ObjectAllValuesFrom { property, filler } = superclass
    {
        result
            .role_ranges
            .entry(property.clone())
            .or_default()
            .push((**filler).clone());
        result.absorbed_count += 1;
        return;
    }

    // `A₁ ⊓ … ⊓ Aₖ ⊑ D` — conjunction absorption (all conjuncts and head atomic).
    if let ClassExpression::ObjectIntersectionOf(conjuncts) = subclass
        && conjuncts.iter().all(ClassExpression::is_named_class)
        && let ClassExpression::Class(_) = superclass
    {
        result.conjunction_gcis.push(ConjunctionGci {
            conjuncts: conjuncts.clone(),
            head: superclass.clone(),
        });
        result.absorbed_count += 1;
        return;
    }

    // `A ⊑ B` — atomic implication (told subsumption handles it).
    if let ClassExpression::Class(_) = subclass
        && let ClassExpression::Class(_) = superclass
    {
        result.atomic_implications += 1;
        result.absorbed_count += 1;
        return;
    }

    // `A ⊑ D` with `A` atomic and `D` a deterministic complex expression
    // (e.g. `∀R.C`, a conjunction, a nested existential).  The told-subsumption
    // layer / saturation already exposes `D` in the class's superclass set, and
    // the propagation derives its cross-class consequences.  Nothing more to do.
    if let ClassExpression::Class(_) = subclass {
        result.absorbed_count += 1;
        return;
    }

    // Anything else — genuinely needs the tableau.
    result.remaining_gcis.push(RemainingGci {
        subclass: subclass.clone(),
        superclass: superclass.clone(),
    });
    result.remaining_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{
        Class, ObjectPropertyExpression,
        axioms::{Axiom, ObjectPropertyDomainAxiom, SubClassOfAxiom},
    };

    fn class(name: &str) -> ClassExpression {
        ClassExpression::Class(Class {
            iri: crate::ontology::IRI::new(name),
        })
    }

    fn prop(name: &str) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
            iri: crate::ontology::IRI::new(name),
        })
    }

    #[test]
    fn test_absorb_role_successor() {
        let mut ont = Ontology::new();
        ont.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: class("A"),
            superclass: ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            },
            annotations: vec![],
        }));
        let res = absorb(&ont);
        assert_eq!(res.role_successor_count(), 1);
        assert_eq!(res.remaining_count, 0);
        assert_eq!(res.remaining_gcis.len(), 0);
        let clauses = &res.role_successors[&class("A")];
        assert_eq!(clauses[0].filler, class("B"));
    }

    #[test]
    fn test_absorb_role_domain() {
        let mut ont = Ontology::new();
        ont.add_axiom(Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
            id: 1,
            property: prop("R"),
            domain: class("D"),
            annotations: vec![],
        }));
        let res = absorb(&ont);
        assert_eq!(res.role_domains.get(&prop("R")).map(Vec::len), Some(1));
    }

    #[test]
    fn test_absorb_range_gci() {
        let mut ont = Ontology::new();
        ont.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            },
            superclass: class("D"),
            annotations: vec![],
        }));
        let res = absorb(&ont);
        assert_eq!(res.role_range_gcis.len(), 1);
        assert_eq!(res.remaining_count, 0);
    }

    #[test]
    fn test_non_deterministic_union_not_absorbed() {
        let mut ont = Ontology::new();
        ont.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: class("A"),
            superclass: ClassExpression::ObjectUnionOf(vec![class("B"), class("C")]),
            annotations: vec![],
        }));
        let res = absorb(&ont);
        assert_eq!(res.remaining_gcis.len(), 1);
        assert_eq!(res.remaining_count, 1);
    }

    #[test]
    fn test_is_non_deterministic() {
        assert!(!is_non_deterministic(&class("A")));
        assert!(!is_non_deterministic(
            &ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            }
        ));
        assert!(is_non_deterministic(&ClassExpression::ObjectUnionOf(vec![
            class("A")
        ])));
        assert!(is_non_deterministic(&ClassExpression::ObjectOneOf(vec![
            crate::ontology::Individual::named(crate::ontology::IRI::new("a"))
        ])));
        assert!(is_non_deterministic(
            &ClassExpression::ObjectExactCardinality {
                property: prop("R"),
                cardinality: 1,
                filler: Box::new(class("A")),
            }
        ));
    }
}
