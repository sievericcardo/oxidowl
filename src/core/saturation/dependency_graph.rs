//! Dependency-graph driven classification propagation (workstream A8).
//!
//! This is the Elgot-style / enhanced-traversal core of the classification
//! fast path: we build a dependency graph over the named classes (edge `A → B`
//! when computing `A`'s subsumers requires `B`'s already-computed subsumers,
//! e.g. told `A ⊑ B` or a role-successor `A ⊑ ∃R.B`), condense it into SCCs,
//! and process the SCC condensation in dependency order (superclasses first),
//! iterating each SCC to a local fixpoint and caching the result for every
//! dependent class.  This mirrors HermiT's core optimization of never
//! recomputing a class's subsumers that the ontology structure already fixes.
//!
//! All propagation rules are sound OWL-EL rules; the output is a *lower bound*
//! on the true subsumption relation.  Classes whose derivation is not covered
//! by the absorbed fragment are reported via [`DeterministicClassification`]
//! so the caller can fall back to the full tableau for exactly those classes.

use super::absorption::{AbsorptionResult, collect_named_classes, is_non_deterministic};
use crate::ontology::{ClassExpression, ObjectPropertyExpression};
use std::collections::{HashMap, HashSet};

/// Deterministic classification of one named class.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassDerivation {
    /// All deterministically-derived subsumers (transitive closure).
    pub subsumers: HashSet<ClassExpression>,
    /// Existential restrictions implied by the class, closed under filler
    /// subsumption (`∃R.C` extends to `∃R.X` for every named `X ⊒ C`).
    pub existentials: HashSet<(ObjectPropertyExpression, ClassExpression)>,
}

/// Result of the dependency-graph / Elgot propagation.
#[derive(Debug, Clone, Default)]
pub struct DeterministicClassification {
    /// Per-class deterministic derivations.
    pub derivations: HashMap<ClassExpression, ClassDerivation>,
    /// Number of SCCs in the condensation (1 + number of cycles).
    pub scc_count: usize,
    /// Total fixpoint rounds spent across all SCCs.
    pub rounds: usize,
    /// Classes whose deterministic derivation is incomplete and therefore need
    /// the tableau fallback (disjunction / cardinality / nominal in the
    /// closure, or a class connected to a non-absorbed GCI).
    pub may_need_tableau: HashSet<ClassExpression>,
}

/// The dependency-graph classifier.
#[derive(Debug, Clone, Default)]
pub struct DependencyClassifier;

impl DependencyClassifier {
    /// Run the Elgot-style propagation.
    #[must_use]
    pub fn classify(
        &self,
        classes: &[ClassExpression],
        absorption: &AbsorptionResult,
        told_subsumers: &HashMap<ClassExpression, HashSet<ClassExpression>>,
    ) -> DeterministicClassification {
        // Only named classes are graph nodes.
        let named: Vec<ClassExpression> = classes
            .iter()
            .filter(|c| c.is_named_class())
            .cloned()
            .collect();

        // ── Build the dependency graph ─────────────────────────────────────
        let mut deps: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for a in &named {
            let mut set: HashSet<ClassExpression> = HashSet::new();
            if let Some(sups) = told_subsumers.get(a) {
                for sup in sups {
                    collect_named_classes(sup, &mut set);
                }
            }
            if let Some(successors) = absorption.role_successors.get(a) {
                for clause in successors {
                    collect_named_classes(&clause.filler, &mut set);
                }
            }
            // Equivalent classes are already in told_subsumers (mutual edges),
            // which makes cycles / SCCs appear naturally.
            deps.insert(a.clone(), set);
        }

        let sccs = tarjan_scc(&named, &deps);
        let scc_count = sccs.len();

        // ── Elgot-style propagation: sinks (superclasses) first ────────────
        let mut derivations: HashMap<ClassExpression, ClassDerivation> = HashMap::new();
        let mut rounds = 0usize;

        for scc in &sccs {
            // Single-node SCCs without a self-dependency are computed exactly
            // once: Tarjan emits the condensation in reverse topological order,
            // so every dependency of a singleton is already final when it is
            // processed.  Only genuinely cyclic SCCs need the local fixpoint
            // (and re-verification) loop, which keeps a linear chain O(N) in
            // rounds instead of 2·N.
            if scc.len() == 1 {
                let a = &named[scc[0]];
                let has_self_dep = deps.get(a).is_some_and(|s| s.contains(a));
                if !has_self_dep {
                    rounds += 1;
                    let new_deriv =
                        self.compute_derivation(a, absorption, told_subsumers, &derivations);
                    derivations.insert(a.clone(), new_deriv);
                    continue;
                }
            }
            let mut changed = true;
            while changed {
                changed = false;
                rounds += 1;
                for &node_idx in scc {
                    let a = &named[node_idx];
                    let new_deriv =
                        self.compute_derivation(a, absorption, told_subsumers, &derivations);
                    let old = derivations.get(a);
                    if old.is_none_or(|o| o != &new_deriv) {
                        derivations.insert(a.clone(), new_deriv);
                        changed = true;
                    }
                }
            }
        }

        // ── Classes that still need the tableau ─────────────────────────────
        let may_need_tableau = self.compute_tableau_requirements(classes, &derivations, absorption);

        DeterministicClassification {
            derivations,
            scc_count,
            rounds,
            may_need_tableau,
        }
    }

    /// Compute the deterministic derivation of one class from cached results.
    fn compute_derivation(
        &self,
        a: &ClassExpression,
        absorption: &AbsorptionResult,
        told_subsumers: &HashMap<ClassExpression, HashSet<ClassExpression>>,
        derivations: &HashMap<ClassExpression, ClassDerivation>,
    ) -> ClassDerivation {
        let mut deriv = ClassDerivation {
            subsumers: told_subsumers.get(a).cloned().unwrap_or_default(),
            existentials: HashSet::new(),
        };

        // R3: top-level existential restrictions from told subsumers.
        for sup in told_subsumers.get(a).into_iter().flatten() {
            if let ClassExpression::ObjectSomeValuesFrom { property, filler } = sup {
                deriv
                    .existentials
                    .insert((property.clone(), (**filler).clone()));
            }
        }

        // R2 / R4: named told superclasses contribute their cached subsumers
        // and existentials (existential propagation downwards).  These cached
        // derivations are *already transitively closed*, so we do not need to
        // re-close their members below.
        for sup in told_subsumers.get(a).into_iter().flatten() {
            if let ClassExpression::Class(_) = sup
                && let Some(d) = derivations.get(sup)
            {
                deriv.subsumers.extend(d.subsumers.iter().cloned());
                deriv.existentials.extend(d.existentials.iter().cloned());
            }
        }

        // Role-successor clauses absorbed from `A ⊑ ∃R.C`.
        if let Some(successors) = absorption.role_successors.get(a) {
            for clause in successors {
                deriv
                    .existentials
                    .insert((clause.property.clone(), clause.filler.clone()));
            }
        }

        // Subsumers added by the rules below (R5/R7/R9) or contained in complex
        // told subsumers need their cached subsumers propagated too.  This is
        // tracked in `newly_added` so the closure pass is O(new) rather than
        // O(|subsumers|) per class — keeping the whole propagation O(N·D)
        // instead of O(N³) on deep hierarchies (workstream A8).
        let mut newly_added: Vec<ClassExpression> = Vec::new();

        // R5: role-domain firing — `A ⊑ ∃R.C` and `∃R.⊤ ⊑ D` ⇒ `A ⊑ D`.
        // Fires on *any* existential regardless of filler, so no filler
        // closure is required.
        for (r, _c) in &deriv.existentials {
            if let Some(domains) = absorption.role_domains.get(r) {
                for d in domains {
                    if deriv.subsumers.insert(d.clone()) {
                        newly_added.push(d.clone());
                    }
                }
            }
        }

        // R7: role-range GCI firing — `A ⊑ ∃R.C`, `C ⊑ X`, `∃R.X ⊑ E` ⇒ `A ⊑ E`.
        // `C ⊑ X` is checked structurally against the cached derivations
        // (`class_subsumed_by`), avoiding the O(n³) materialisation of
        // `∃R.X` for every named `X ⊒ C`.
        for (r, c) in &deriv.existentials {
            for gci in &absorption.role_range_gcis {
                if &gci.property == r && class_subsumed_by(c, &gci.filler, derivations) {
                    if deriv.subsumers.insert(gci.head.clone()) {
                        newly_added.push(gci.head.clone());
                    }
                }
            }
        }

        // R9: conjunction GCI firing — `A ⊑ A₁ … Aₖ`, `A₁ ⊓ … ⊓ Aₖ ⊑ E` ⇒ `A ⊑ E`.
        for gci in &absorption.conjunction_gcis {
            if gci
                .conjuncts
                .iter()
                .all(|conj| deriv.subsumers.contains(conj))
            {
                if deriv.subsumers.insert(gci.head.clone()) {
                    newly_added.push(gci.head.clone());
                }
            }
        }

        // R2 closure: propagate the cached subsumers of every *newly added*
        // named subsumer (members reached via R2 are already closed).
        let mut work: Vec<ClassExpression> = newly_added
            .into_iter()
            .filter(|c| c.is_named_class())
            .collect();
        let mut seen: HashSet<ClassExpression> = work.iter().cloned().collect();
        let mut i = 0usize;
        while i < work.len() {
            let c = work[i].clone();
            i += 1;
            if let Some(d) = derivations.get(&c) {
                for x in &d.subsumers {
                    if x.is_named_class()
                        && deriv.subsumers.insert(x.clone())
                        && seen.insert(x.clone())
                    {
                        work.push(x.clone());
                    }
                }
            }
        }

        deriv
    }

    /// Determine which classes still need the tableau fallback.
    fn compute_tableau_requirements(
        &self,
        classes: &[ClassExpression],
        derivations: &HashMap<ClassExpression, ClassDerivation>,
        absorption: &AbsorptionResult,
    ) -> HashSet<ClassExpression> {
        let mut need: HashSet<ClassExpression> = HashSet::new();

        // 1. Non-deterministic constructs in the closure.
        for a in classes {
            if let Some(d) = derivations.get(a) {
                let nondet = d.subsumers.iter().any(is_non_deterministic)
                    || d.existentials.iter().any(|(_, c)| is_non_deterministic(c));
                if nondet {
                    need.insert(a.clone());
                }
            }
        }

        // 2. Classes connected to a non-absorbed GCI.  Only the *subclass*
        // side can fire a GCI and add subsumptions to a class, so only classes
        // subsumed-by (directly or through an existential filler) a class
        // mentioned on the subclass side are flagged.
        let mut touched: HashSet<ClassExpression> = HashSet::new();
        for gci in &absorption.remaining_gcis {
            collect_named_classes(&gci.subclass, &mut touched);
        }
        for a in classes {
            if touched.contains(a) {
                need.insert(a.clone());
            } else if let Some(d) = derivations.get(a) {
                let fires_for_a = d.subsumers.iter().any(|s| touched.contains(s))
                    || d.existentials.iter().any(|(_, c)| touched.contains(c));
                if fires_for_a {
                    // A is subsumed by a class involved in a remaining GCI, or
                    // implies an existential whose filler it mentions: the GCI
                    // could fire for A, so its derivation may be incomplete.
                    need.insert(a.clone());
                }
            }
        }

        need
    }
}

/// Check whether `expr ⊑ target` is deterministically known.
///
/// Sound by construction: only returns `true` for actual subsumptions.  Used
/// for complex fillers where no cached per-class derivation exists.
fn class_subsumed_by(
    expr: &ClassExpression,
    target: &ClassExpression,
    derivations: &HashMap<ClassExpression, ClassDerivation>,
) -> bool {
    if expr == target {
        return true;
    }
    match expr {
        ClassExpression::Class(_) => derivations
            .get(expr)
            .is_some_and(|d| d.subsumers.contains(target)),
        ClassExpression::ObjectIntersectionOf(parts) => parts
            .iter()
            .any(|p| class_subsumed_by(p, target, derivations)),
        _ => false,
    }
}

/// Iterative Tarjan SCC, returning components in *reverse topological order*
/// of the condensation (sinks / superclasses first).
fn tarjan_scc(
    nodes: &[ClassExpression],
    deps: &HashMap<ClassExpression, HashSet<ClassExpression>>,
) -> Vec<Vec<usize>> {
    let n = nodes.len();
    if n == 0 {
        return Vec::new();
    }
    let mut pos: HashMap<&ClassExpression, usize> = HashMap::with_capacity(n);
    for (i, c) in nodes.iter().enumerate() {
        pos.insert(c, i);
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, c) in nodes.iter().enumerate() {
        if let Some(neigh) = deps.get(c) {
            for nb in neigh {
                if let Some(&j) = pos.get(nb) {
                    adj[i].push(j);
                }
            }
        }
    }

    let mut index = 0usize;
    let mut idx = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for s in 0..n {
        if idx[s] != usize::MAX {
            continue;
        }
        let mut call: Vec<(usize, usize)> = vec![(s, 0)];
        idx[s] = index;
        low[s] = index;
        index += 1;
        stack.push(s);
        on_stack[s] = true;

        while let Some(&mut (u, ref mut next)) = call.last_mut() {
            if *next < adj[u].len() {
                let v = adj[u][*next];
                *next += 1;
                if idx[v] == usize::MAX {
                    idx[v] = index;
                    low[v] = index;
                    index += 1;
                    stack.push(v);
                    on_stack[v] = true;
                    call.push((v, 0));
                } else if on_stack[v] {
                    low[u] = low[u].min(idx[v]);
                }
            } else {
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[u]);
                }
                if low[u] == idx[u] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        comp.push(w);
                        if w == u {
                            break;
                        }
                    }
                    sccs.push(comp);
                }
            }
        }
    }
    sccs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{
        Class, ObjectPropertyExpression, Ontology,
        axioms::{Axiom, SubClassOfAxiom},
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

    fn sub(sub: ClassExpression, sup: ClassExpression, id: u64) -> Axiom {
        Axiom::SubClassOf(SubClassOfAxiom {
            id,
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        })
    }

    #[test]
    fn test_linear_chain_propagation() {
        let mut ont = Ontology::new();
        for i in 0..4 {
            ont.add_axiom(sub(
                class(&format!("C{i}")),
                class(&format!("C{}", i + 1)),
                i as u64 + 1,
            ));
        }
        let absorption = super::super::absorption::absorb(&ont);
        let classes: Vec<ClassExpression> = (0..5).map(|i| class(&format!("C{i}"))).collect();
        let mut told: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for c in &classes {
            told.entry(c.clone()).or_default();
        }
        told.entry(class("C0")).or_default().insert(class("C1"));
        told.entry(class("C1")).or_default().insert(class("C2"));
        told.entry(class("C2")).or_default().insert(class("C3"));
        told.entry(class("C3")).or_default().insert(class("C4"));

        let dc = DependencyClassifier.classify(&classes, &absorption, &told);
        let c0 = &dc.derivations[&class("C0")];
        assert!(c0.subsumers.contains(&class("C1")));
        assert!(c0.subsumers.contains(&class("C4")));
        assert!(dc.may_need_tableau.is_empty());
    }

    #[test]
    fn test_role_successor_domain_propagation() {
        // A ⊑ ∃R.B ; Domain(R) = D  ⇒  A ⊑ D
        let mut ont = Ontology::new();
        ont.add_axiom(sub(
            class("A"),
            ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            },
            1,
        ));
        ont.add_axiom(Axiom::ObjectPropertyDomain(
            crate::ontology::axioms::ObjectPropertyDomainAxiom {
                id: 2,
                property: prop("R"),
                domain: class("D"),
                annotations: vec![],
            },
        ));

        let absorption = super::super::absorption::absorb(&ont);
        let classes = vec![class("A"), class("B"), class("D")];
        let mut told: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for c in &classes {
            told.entry(c.clone()).or_default();
        }
        told.entry(class("A"))
            .or_default()
            .insert(ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            });

        let dc = DependencyClassifier.classify(&classes, &absorption, &told);
        let a = &dc.derivations[&class("A")];
        assert!(
            a.subsumers.contains(&class("D")),
            "A ⊑ D expected, got {:?}",
            a.subsumers
        );
        assert!(dc.may_need_tableau.is_empty());
    }

    #[test]
    fn test_role_range_gci_propagation() {
        // A ⊑ ∃R.B ; ∃R.B ⊑ E  ⇒  A ⊑ E
        let mut ont = Ontology::new();
        ont.add_axiom(sub(
            class("A"),
            ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            },
            1,
        ));
        ont.add_axiom(sub(
            ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            },
            class("E"),
            2,
        ));

        let absorption = super::super::absorption::absorb(&ont);
        let classes = vec![class("A"), class("B"), class("E")];
        let mut told: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for c in &classes {
            told.entry(c.clone()).or_default();
        }
        told.entry(class("A"))
            .or_default()
            .insert(ClassExpression::ObjectSomeValuesFrom {
                property: prop("R"),
                filler: Box::new(class("B")),
            });

        let dc = DependencyClassifier.classify(&classes, &absorption, &told);
        let a = &dc.derivations[&class("A")];
        assert!(
            a.subsumers.contains(&class("E")),
            "A ⊑ E expected, got {:?}",
            a.subsumers
        );
    }

    #[test]
    fn test_union_flagged_for_tableau() {
        let mut ont = Ontology::new();
        ont.add_axiom(sub(
            class("C0"),
            ClassExpression::ObjectUnionOf(vec![class("C1"), class("C2")]),
            1,
        ));
        let absorption = super::super::absorption::absorb(&ont);
        assert_eq!(absorption.remaining_gcis.len(), 1);

        let classes = vec![class("C0"), class("C1"), class("C2")];
        let mut told: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for c in &classes {
            told.entry(c.clone()).or_default();
        }
        told.entry(class("C0"))
            .or_default()
            .insert(ClassExpression::ObjectUnionOf(vec![
                class("C1"),
                class("C2"),
            ]));

        let dc = DependencyClassifier.classify(&classes, &absorption, &told);
        assert!(dc.may_need_tableau.contains(&class("C0")));
        assert!(!dc.may_need_tableau.contains(&class("C1")));
    }

    #[test]
    fn test_conjunction_gci() {
        // A ⊑ B ; A ⊑ C ; B ⊓ C ⊑ E  ⇒  A ⊑ E
        let mut ont = Ontology::new();
        ont.add_axiom(sub(class("A"), class("B"), 1));
        ont.add_axiom(sub(class("A"), class("C"), 2));
        ont.add_axiom(sub(
            ClassExpression::ObjectIntersectionOf(vec![class("B"), class("C")]),
            class("E"),
            3,
        ));

        let absorption = super::super::absorption::absorb(&ont);
        let classes = vec![class("A"), class("B"), class("C"), class("E")];
        let mut told: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        for c in &classes {
            told.entry(c.clone()).or_default();
        }
        told.entry(class("A")).or_default().insert(class("B"));
        told.entry(class("A")).or_default().insert(class("C"));

        let dc = DependencyClassifier.classify(&classes, &absorption, &told);
        let a = &dc.derivations[&class("A")];
        assert!(
            a.subsumers.contains(&class("E")),
            "A ⊑ E expected, got {:?}",
            a.subsumers
        );
    }
}
