//! Inferred object/data property hierarchies.
//!
//! [`crate::core::reasoner::Reasoner::classify_object_properties`] and the
//! OWL API surface (`OWLReasoner::get_super_object_properties` & friends)
//! both answer from the closures computed here instead of the raw asserted
//! `SubObjectPropertyOf` / `SubDataPropertyOf` edges.
//!
//! # Documented design choice — *strict* supers
//!
//! The OWL API defines `StrictSubObjectPropertyOf(OPE1, OPE2)` as
//! `O ⊨ OPE1 ⊑ OPE2` **and not** `O ⊨ OPE2 ⊑ OPE1`, and
//! `getSuperObjectProperties(pe, false)` returns the *strict* super
//! properties ([`OWLReasoner` javadoc]). Equivalent properties are therefore
//! **not** part of a property's super/sub set — they are answered by
//! `getEquivalentObjectProperties`. The equivalence edges are still part of
//! the underlying graph, so reachability through them is preserved
//! (`A ≡ B`, `B ⊑ C` ⇒ `A`'s strict supers are `{C, top}`).
//!
//! `owl:topObjectProperty` / `owl:topDataProperty` are a strict super of
//! every property, and `owl:bottomObjectProperty` / `owl:bottomDataProperty`
//! a strict sub of every property. The direct filter follows the OWL API
//! `DirectSubObjectPropertyOf` definition: a strict super `S` of `P` is
//! *direct* iff there is no **property name** `T` with
//! `StrictSub(P, T)` and `StrictSub(T, S)`; the intermediates are restricted
//! to named properties (inverse expressions are never intermediates).

use crate::ontology::{
    DataProperty, DataPropertyExpression, IRI, ObjectProperty, ObjectPropertyExpression, Ontology,
};
use std::collections::{HashMap, HashSet};

/// `owl:topObjectProperty` IRI.
pub const TOP_OBJECT_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#topObjectProperty";
/// `owl:bottomObjectProperty` IRI.
pub const BOTTOM_OBJECT_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#bottomObjectProperty";
/// `owl:topDataProperty` IRI.
pub const TOP_DATA_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#topDataProperty";
/// `owl:bottomDataProperty` IRI.
pub const BOTTOM_DATA_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#bottomDataProperty";

/// The `owl:topObjectProperty` expression.
#[must_use]
pub fn top_object_property() -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty {
        iri: IRI::new(TOP_OBJECT_PROPERTY_IRI),
    })
}

/// The `owl:bottomObjectProperty` expression.
#[must_use]
pub fn bottom_object_property() -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty {
        iri: IRI::new(BOTTOM_OBJECT_PROPERTY_IRI),
    })
}

/// The `owl:topDataProperty` expression.
#[must_use]
pub fn top_data_property() -> DataPropertyExpression {
    DataPropertyExpression::DataProperty(DataProperty {
        iri: IRI::new(TOP_DATA_PROPERTY_IRI),
    })
}

/// The `owl:bottomDataProperty` expression.
#[must_use]
pub fn bottom_data_property() -> DataPropertyExpression {
    DataPropertyExpression::DataProperty(DataProperty {
        iri: IRI::new(BOTTOM_DATA_PROPERTY_IRI),
    })
}

/// Structural inverse of an object property expression (`P` ⇄ `inv(P)`).
/// Property chains have no inverse expression.
fn inverse(property: &ObjectPropertyExpression) -> Option<ObjectPropertyExpression> {
    match property {
        ObjectPropertyExpression::ObjectProperty(p) => {
            Some(ObjectPropertyExpression::InverseObjectProperty(p.clone()))
        }
        ObjectPropertyExpression::InverseObjectProperty(p) => {
            Some(ObjectPropertyExpression::ObjectProperty(p.clone()))
        }
        ObjectPropertyExpression::PropertyChain(_) => None,
    }
}

/// Whether the expression is a *named* property (the OWL API restricts the
/// `DirectSubObjectPropertyOf` intermediate to property **names**).
fn is_named(property: &ObjectPropertyExpression) -> bool {
    matches!(property, ObjectPropertyExpression::ObjectProperty(_))
}

/// Generic transitive-closure engine over a directed "sub → super" edge set.
///
/// `N` is either [`ObjectPropertyExpression`] or [`DataPropertyExpression`].
#[derive(Debug, Clone)]
struct Closure<N> {
    /// Every node of the graph (including `top` / `bottom`).
    nodes: HashSet<N>,
    /// `reach[p]` = every node reachable from `p` following `sub → super`
    /// edges, **including** `p` itself (the raw reachability set). The sub
    /// closure is derived from it (`q` is a sub of `p` iff `p ∈ reach[q]`).
    reach: HashMap<N, HashSet<N>>,
}

impl<N: Clone + Eq + std::hash::Hash> Closure<N> {
    fn compute(nodes: HashSet<N>, edges: &[(N, N)]) -> Self {
        let mut forward: HashMap<N, HashSet<N>> = HashMap::new();
        for node in &nodes {
            forward.entry(node.clone()).or_default();
        }
        for (sub, sup) in edges {
            if nodes.contains(sub) && nodes.contains(sup) {
                forward.entry(sub.clone()).or_default().insert(sup.clone());
            }
        }

        // Reachability from every node (iterative DFS; cycles are expected —
        // equivalence makes the graph bidirectional inside a component).
        let mut reach: HashMap<N, HashSet<N>> = HashMap::with_capacity(nodes.len());
        for start in &nodes {
            let mut seen: HashSet<N> = HashSet::new();
            let mut stack = vec![start.clone()];
            seen.insert(start.clone());
            while let Some(current) = stack.pop() {
                if let Some(supers) = forward.get(&current) {
                    for sup in supers {
                        if seen.insert(sup.clone()) {
                            stack.push(sup.clone());
                        }
                    }
                }
            }
            reach.insert(start.clone(), seen);
        }

        Self { nodes, reach }
    }

    /// `p`'s equivalence class excluding `p` itself (mutual reachability).
    fn equivalents(&self, p: &N) -> HashSet<N> {
        let Some(forward) = self.reach.get(p) else {
            return HashSet::new();
        };
        forward
            .iter()
            .filter(|q| *q != p && self.reach.get(*q).is_some_and(|back| back.contains(p)))
            .cloned()
            .collect()
    }

    /// The strict super set of `p`: every reachable node except `p` and its
    /// equivalents (OWL API *strict* semantics, see the module docs).
    fn strict_supers(&self, p: &N) -> HashSet<N> {
        let Some(forward) = self.reach.get(p) else {
            return HashSet::new();
        };
        let equivalents = self.equivalents(p);
        forward
            .iter()
            .filter(|q| *q != p && !equivalents.contains(*q))
            .cloned()
            .collect()
    }

    /// The strict sub set of `p` (mirror of [`Self::strict_supers`]):
    /// every node whose reachability set contains `p`.
    fn strict_subs(&self, p: &N) -> HashSet<N> {
        if !self.reach.contains_key(p) {
            return HashSet::new();
        }
        let equivalents = self.equivalents(p);
        self.reach
            .iter()
            .filter(|(q, supers)| *q != p && supers.contains(p) && !equivalents.contains(*q))
            .map(|(q, _)| q.clone())
            .collect()
    }

    /// Whether `higher` is a *strict* super of `lower` (`lower ⊑ higher` but
    /// not `higher ⊑ lower`) — the OWL API `StrictSub…Of(lower, higher)`.
    fn strictly_above(&self, higher: &N, lower: &N) -> bool {
        self.reach
            .get(lower)
            .is_some_and(|low| low.contains(higher))
            && self
                .reach
                .get(higher)
                .is_some_and(|high| !high.contains(lower))
    }
}

/// Inferred object-property hierarchy over the ontology's `RBox`.
#[derive(Debug, Clone)]
pub struct ObjectPropertyHierarchy {
    closure: Closure<ObjectPropertyExpression>,
}

impl ObjectPropertyHierarchy {
    /// Compute the hierarchy from the ontology's axioms.
    ///
    /// Graph:
    /// * nodes — every named object property of the signature, every inverse
    ///   expression introduced by `InverseObjectProperties` / the hierarchy
    ///   axioms, plus `owl:topObjectProperty` and `owl:bottomObjectProperty`;
    /// * `SubObjectPropertyOf(P, Q)` — edge `P → Q` (property-chain subs are
    ///   skipped: a chain's internals are not property-hierarchy edges);
    /// * `EquivalentObjectProperties` — bidirectional edges;
    /// * `InverseObjectProperties(P, Q)` — `P ≡ inv(Q)` and `Q ≡ inv(P)`
    ///   (bidirectional edges `P ↔ inv(Q)`, `Q ↔ inv(P)`);
    /// * every sub/equivalent edge `X → Y` also yields `inv(X) → inv(Y)`
    ///   when both inverse nodes exist (inverse is an order embedding);
    /// * `top` is a super of every node, `bottom` a sub of every node.
    #[must_use]
    pub fn compute(ontology: &Ontology) -> Self {
        let mut nodes: HashSet<ObjectPropertyExpression> = HashSet::new();

        // Named properties from the signature (all axiom types covered).
        if let Ok(signature) = ontology.signature() {
            for property in &signature.object_properties {
                nodes.insert(ObjectPropertyExpression::ObjectProperty(property.clone()));
            }
        }

        // Property expressions occurring in the hierarchy axioms (inverse
        // expressions such as `inv(Q)` are introduced here).
        let mut edges: Vec<(ObjectPropertyExpression, ObjectPropertyExpression)> = Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubObjectPropertyOf(a) => {
                    nodes.insert(a.sub_property.clone());
                    nodes.insert(a.super_property.clone());
                    // Chain internals are not hierarchy edges; the chain
                    // expression itself is skipped as a node as well (it is
                    // not an entity of the property hierarchy).
                    if !matches!(a.sub_property, ObjectPropertyExpression::PropertyChain(_))
                        && !matches!(a.super_property, ObjectPropertyExpression::PropertyChain(_))
                    {
                        edges.push((a.sub_property.clone(), a.super_property.clone()));
                    }
                }
                crate::ontology::axioms::Axiom::EquivalentObjectProperties(a) => {
                    for p in &a.properties {
                        nodes.insert(p.clone());
                    }
                    for i in 0..a.properties.len() {
                        for j in (i + 1)..a.properties.len() {
                            edges.push((a.properties[i].clone(), a.properties[j].clone()));
                            edges.push((a.properties[j].clone(), a.properties[i].clone()));
                        }
                    }
                }
                crate::ontology::axioms::Axiom::InverseObjectProperties(a) => {
                    nodes.insert(a.property1.clone());
                    nodes.insert(a.property2.clone());
                    if let (Some(inv1), Some(inv2)) = (inverse(&a.property1), inverse(&a.property2))
                    {
                        nodes.insert(inv1.clone());
                        nodes.insert(inv2.clone());
                        // `P ≡ inv(Q)` and `Q ≡ inv(P)`.
                        edges.push((a.property1.clone(), inv2.clone()));
                        edges.push((inv2, a.property1.clone()));
                        edges.push((a.property2.clone(), inv1.clone()));
                        edges.push((inv1, a.property2.clone()));
                    }
                }
                _ => {}
            }
        }

        // Inverse expansion of every collected edge (`X ⊑ Y` ⇒
        // `inv(X) ⊑ inv(Y)`), only where both inverse nodes already exist.
        let direct_edges = edges.clone();
        for (sub, sup) in &direct_edges {
            if let (Some(inv_sub), Some(inv_sup)) = (inverse(sub), inverse(sup))
                && nodes.contains(&inv_sub)
                && nodes.contains(&inv_sup)
            {
                edges.push((inv_sub, inv_sup));
            }
        }

        // `top` / `bottom` are always present.
        nodes.insert(top_object_property());
        nodes.insert(bottom_object_property());
        let top = top_object_property();
        let bottom = bottom_object_property();
        let all: Vec<ObjectPropertyExpression> = nodes.iter().cloned().collect();
        for node in &all {
            if *node != top {
                edges.push((node.clone(), top.clone()));
            }
            if *node != bottom {
                edges.push((bottom.clone(), node.clone()));
            }
        }

        Self {
            closure: Closure::compute(nodes, &edges),
        }
    }

    /// Every node of the hierarchy (named properties, inverse expressions,
    /// `top` and `bottom`).
    #[must_use]
    pub fn nodes(&self) -> &HashSet<ObjectPropertyExpression> {
        &self.closure.nodes
    }

    /// The **strict** super properties of `property` (transitively closed,
    /// equivalents excluded — see the module docs). Empty for unknown
    /// properties.
    #[must_use]
    pub fn supers(&self, property: &ObjectPropertyExpression) -> HashSet<ObjectPropertyExpression> {
        self.closure.strict_supers(property)
    }

    /// The **strict** sub properties of `property`.
    #[must_use]
    pub fn subs(&self, property: &ObjectPropertyExpression) -> HashSet<ObjectPropertyExpression> {
        self.closure.strict_subs(property)
    }

    /// The properties equivalent to `property` (excluding itself).
    #[must_use]
    pub fn equivalents(
        &self,
        property: &ObjectPropertyExpression,
    ) -> HashSet<ObjectPropertyExpression> {
        self.closure.equivalents(property)
    }

    /// The *direct* super properties: strict supers `S` for which no named
    /// property `T` sits strictly between the queried property and `S`
    /// (OWL API `DirectSubObjectPropertyOf`).
    #[must_use]
    pub fn direct_supers(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Vec<ObjectPropertyExpression> {
        let supers = self.supers(property);
        supers
            .iter()
            .filter(|s| {
                !supers
                    .iter()
                    .any(|t| t != *s && is_named(t) && self.closure.strictly_above(s, t))
            })
            .cloned()
            .collect()
    }

    /// The *direct* sub properties (mirror of [`Self::direct_supers`]).
    #[must_use]
    pub fn direct_subs(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Vec<ObjectPropertyExpression> {
        let subs = self.subs(property);
        subs.iter()
            .filter(|s| {
                !subs
                    .iter()
                    .any(|t| t != *s && is_named(t) && self.closure.strictly_above(t, s))
            })
            .cloned()
            .collect()
    }
}

/// Inferred data-property hierarchy (no inverses / chains in OWL 2 DL).
#[derive(Debug, Clone)]
pub struct DataPropertyHierarchy {
    closure: Closure<DataPropertyExpression>,
}

impl DataPropertyHierarchy {
    /// Compute the hierarchy: `SubDataPropertyOf` edges,
    /// `EquivalentDataProperties` bidirectional edges, `topDataProperty` as
    /// super of everything and `bottomDataProperty` as sub of everything.
    #[must_use]
    pub fn compute(ontology: &Ontology) -> Self {
        let mut nodes: HashSet<DataPropertyExpression> = HashSet::new();
        if let Ok(signature) = ontology.signature() {
            for property in &signature.data_properties {
                nodes.insert(DataPropertyExpression::DataProperty(property.clone()));
            }
        }

        let mut edges: Vec<(DataPropertyExpression, DataPropertyExpression)> = Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubDataPropertyOf(a) => {
                    nodes.insert(a.sub_property.clone());
                    nodes.insert(a.super_property.clone());
                    edges.push((a.sub_property.clone(), a.super_property.clone()));
                }
                crate::ontology::axioms::Axiom::EquivalentDataProperties(a) => {
                    for p in &a.properties {
                        nodes.insert(p.clone());
                    }
                    for i in 0..a.properties.len() {
                        for j in (i + 1)..a.properties.len() {
                            edges.push((a.properties[i].clone(), a.properties[j].clone()));
                            edges.push((a.properties[j].clone(), a.properties[i].clone()));
                        }
                    }
                }
                _ => {}
            }
        }

        nodes.insert(top_data_property());
        nodes.insert(bottom_data_property());
        let top = top_data_property();
        let bottom = bottom_data_property();
        let all: Vec<DataPropertyExpression> = nodes.iter().cloned().collect();
        for node in &all {
            if *node != top {
                edges.push((node.clone(), top.clone()));
            }
            if *node != bottom {
                edges.push((bottom.clone(), node.clone()));
            }
        }

        Self {
            closure: Closure::compute(nodes, &edges),
        }
    }

    /// Every node of the hierarchy.
    #[must_use]
    pub fn nodes(&self) -> &HashSet<DataPropertyExpression> {
        &self.closure.nodes
    }

    /// The **strict** super data properties (transitively closed).
    #[must_use]
    pub fn supers(&self, property: &DataPropertyExpression) -> HashSet<DataPropertyExpression> {
        self.closure.strict_supers(property)
    }

    /// The **strict** sub data properties.
    #[must_use]
    pub fn subs(&self, property: &DataPropertyExpression) -> HashSet<DataPropertyExpression> {
        self.closure.strict_subs(property)
    }

    /// The data properties equivalent to `property` (excluding itself).
    #[must_use]
    pub fn equivalents(
        &self,
        property: &DataPropertyExpression,
    ) -> HashSet<DataPropertyExpression> {
        self.closure.equivalents(property)
    }

    /// The *direct* super data properties (OWL API
    /// `DirectSubDataPropertyOf`).
    #[must_use]
    pub fn direct_supers(&self, property: &DataPropertyExpression) -> Vec<DataPropertyExpression> {
        let supers = self.supers(property);
        supers
            .iter()
            .filter(|s| {
                !supers
                    .iter()
                    .any(|t| t != *s && self.closure.strictly_above(s, t))
            })
            .cloned()
            .collect()
    }

    /// The *direct* sub data properties.
    #[must_use]
    pub fn direct_subs(&self, property: &DataPropertyExpression) -> Vec<DataPropertyExpression> {
        let subs = self.subs(property);
        subs.iter()
            .filter(|s| {
                !subs
                    .iter()
                    .any(|t| t != *s && self.closure.strictly_above(t, s))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::axioms::{Axiom, InverseObjectPropertiesAxiom, SubObjectPropertyOfAxiom};

    fn obj(name: &str) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(name),
        })
    }

    fn data(name: &str) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(DataProperty {
            iri: IRI::new(name),
        })
    }

    fn sub_obj(sub: ObjectPropertyExpression, sup: ObjectPropertyExpression) -> Axiom {
        Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
            id: 1,
            sub_property: sub,
            super_property: sup,
            annotations: vec![],
        })
    }

    #[test]
    fn chain_supers_are_transitive_and_direct_is_filtered() {
        let a = obj("urn:a");
        let b = obj("urn:b");
        let c = obj("urn:c");
        let ontology = Ontology::new();
        let mut ontology = ontology;
        ontology.add_axiom(sub_obj(a.clone(), b.clone()));
        ontology.add_axiom(sub_obj(b.clone(), c.clone()));

        let hierarchy = ObjectPropertyHierarchy::compute(&ontology);
        let supers = hierarchy.supers(&a);
        assert!(supers.contains(&b) && supers.contains(&c), "got {supers:?}");
        assert!(
            supers.contains(&top_object_property()),
            "top must be a super of everything"
        );
        let direct = hierarchy.direct_supers(&a);
        assert_eq!(direct, vec![b.clone()], "only B is a direct super of A");

        let subs = hierarchy.subs(&c);
        assert!(subs.contains(&a) && subs.contains(&b), "got {subs:?}");
        assert!(subs.contains(&bottom_object_property()));
        let direct_subs = hierarchy.direct_subs(&c);
        assert_eq!(direct_subs, vec![b.clone()], "only B is a direct sub of C");
    }

    #[test]
    fn inverse_axiom_creates_inverse_nodes() {
        use crate::ontology::axioms::InverseObjectPropertiesAxiom;
        let p = obj("urn:p");
        let q = obj("urn:q");
        let s = obj("urn:s");
        let mut ontology = Ontology::new();
        ontology.add_axiom(sub_obj(p.clone(), q.clone()));
        ontology.add_axiom(Axiom::InverseObjectProperties(
            InverseObjectPropertiesAxiom {
                id: 2,
                property1: p.clone(),
                property2: s.clone(),
                annotations: vec![],
            },
        ));

        let hierarchy = ObjectPropertyHierarchy::compute(&ontology);
        let inv_s = inverse(&s).expect("inverse of a named property");
        let inv_p = inverse(&p).expect("inverse of a named property");
        // `inv(S) ≡ P ⊑ Q`
        assert!(hierarchy.supers(&inv_s).contains(&q), "inv(S) ⊑ Q");
        // `Q ⊑ …` and `inv(S)` is a strict sub of `Q`
        assert!(hierarchy.subs(&q).contains(&inv_s), "inv(S) sub of Q");
        // equivalence surfaces through the equivalents map
        assert!(hierarchy.equivalents(&p).contains(&inv_s), "P ≡ inv(S)");
        assert!(hierarchy.equivalents(&s).contains(&inv_p), "S ≡ inv(P)");
        // strict supers of P must NOT contain its equivalent inv(S)
        assert!(
            !hierarchy.supers(&p).contains(&inv_s),
            "equivalents are not strict supers"
        );
    }

    #[test]
    fn data_property_chain_and_top_bottom() {
        let a = data("urn:a");
        let b = data("urn:b");
        let c = data("urn:c");
        let mut ontology = Ontology::new();
        ontology.add_axiom(Axiom::SubDataPropertyOf(
            crate::ontology::axioms::SubDataPropertyOfAxiom {
                id: 1,
                sub_property: a.clone(),
                super_property: b.clone(),
                annotations: vec![],
            },
        ));
        ontology.add_axiom(Axiom::SubDataPropertyOf(
            crate::ontology::axioms::SubDataPropertyOfAxiom {
                id: 2,
                sub_property: b.clone(),
                super_property: c.clone(),
                annotations: vec![],
            },
        ));

        let hierarchy = DataPropertyHierarchy::compute(&ontology);
        let supers = hierarchy.supers(&a);
        assert!(supers.contains(&b) && supers.contains(&c));
        assert!(supers.contains(&top_data_property()));
        assert_eq!(hierarchy.direct_supers(&a), vec![b.clone()]);
        assert!(hierarchy.subs(&a).contains(&bottom_data_property()));
        assert!(hierarchy.subs(&bottom_data_property()).is_empty());
        assert!(hierarchy.supers(&top_data_property()).is_empty());
    }

    #[test]
    fn unused_inverse_import_is_kept_alive() {
        // Guards the `InverseObjectPropertiesAxiom` import above from being
        // trimmed by an over-eager cleanup.
        let _ = std::any::type_name::<InverseObjectPropertiesAxiom>();
    }
}
