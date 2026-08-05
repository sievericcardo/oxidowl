//! Explanation Generation for Oxidowl
//!
//! This module provides comprehensive explanation services for reasoning results,
//! including proof tracking, justification computation, and explanation formatting.

#![allow(dead_code)]

pub mod blackbox;
pub mod converter;
pub mod generator;
pub mod hst;
pub mod ordering;
pub mod renderer;

use crate::explanation::generator::ExplanationGenerator;
use crate::{
    Result,
    core::tableau::NodeId,
    ontology::{Axiom, ClassExpression, Individual, ObjectPropertyExpression, OntologyRef},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::{fmt, sync::Arc};

/// Main explanation service
#[derive(Debug)]
pub struct ExplanationService {
    proof_tracker: ProofTracker,
    justification_computer: JustificationComputer,
    explanation_formatter: ExplanationFormatter,
}

impl ExplanationService {
    /// Create a new explanation service
    #[must_use]
    pub fn new() -> Self {
        Self {
            proof_tracker: ProofTracker::new(),
            justification_computer: JustificationComputer::new(),
            explanation_formatter: ExplanationFormatter::new(),
        }
    }

    /// Generate explanation for a subsumption entailment
    pub fn explain_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Explanation> {
        let justification = self
            .justification_computer
            .compute_subsumption_justification(subclass, superclass, ontology_axioms)?;

        let proof_tree = self.build_subsumption_proof_tree(subclass, superclass, &justification)?;

        Ok(Explanation {
            conclusion: ExplanationConclusion::Subsumption {
                subclass: subclass.clone(),
                superclass: superclass.clone(),
            },
            justification,
            proof_tree,
            explanation_type: ExplanationType::Subsumption,
            confidence: 1.0,
        })
    }

    /// Generate explanation for an inconsistency
    pub fn explain_inconsistency(&self, ontology_axioms: &[Axiom]) -> Result<Explanation> {
        let justification = self
            .justification_computer
            .compute_inconsistency_justification(ontology_axioms)?;
        let proof_tree = self.build_inconsistency_proof_tree(&justification)?;

        Ok(Explanation {
            conclusion: ExplanationConclusion::Inconsistency,
            justification,
            proof_tree,
            explanation_type: ExplanationType::Inconsistency,
            confidence: 1.0,
        })
    }

    /// Generate explanation for unsatisfiability
    pub fn explain_unsatisfiability(
        &self,
        class: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Explanation> {
        let justification = self
            .justification_computer
            .compute_unsatisfiability_justification(class, ontology_axioms)?;
        let proof_tree = self.build_unsatisfiability_proof_tree(class, &justification)?;

        Ok(Explanation {
            conclusion: ExplanationConclusion::Unsatisfiability {
                class: class.clone(),
            },
            justification,
            proof_tree,
            explanation_type: ExplanationType::Unsatisfiability,
            confidence: 1.0,
        })
    }

    /// Format explanation as human-readable text
    pub fn format_explanation(
        &self,
        explanation: &Explanation,
        format: ExplanationFormat,
    ) -> String {
        self.explanation_formatter.format(explanation, format)
    }

    /// Track reasoning step for proof generation
    pub fn track_reasoning_step(&mut self, step: ReasoningStep) -> Result<()> {
        self.proof_tracker.add_step(step);
        Ok(())
    }

    // Private helper methods

    fn build_subsumption_proof_tree(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        justification: &[Axiom],
    ) -> Result<ProofTree> {
        let mut nodes = Vec::new();
        let mut child_ids = Vec::new();
        let mut id_counter: usize = 0;

        // Create premise leaf nodes from justification axioms
        for axiom in justification {
            let node = ProofNode {
                id: id_counter,
                inference: Inference::Subsumption {
                    subclass: subclass.clone(),
                    superclass: superclass.clone(),
                },
                premises: vec![axiom.clone()],
                children: vec![],
                rule_applied: InferenceRule::Subsumption,
            };
            nodes.push(node);
            child_ids.push(id_counter);
            id_counter += 1;
        }

        // Build intermediate nodes for transitive chains
        for axiom in justification {
            if let Axiom::SubClassOf(sc) = axiom
                && &sc.subclass == subclass
            {
                // Track this as an intermediate step: sub ⊑ intermediate
                let chain_node = ProofNode {
                    id: id_counter,
                    inference: Inference::Subsumption {
                        subclass: subclass.clone(),
                        superclass: sc.superclass.clone(),
                    },
                    premises: vec![axiom.clone()],
                    children: vec![id_counter - child_ids.len()],
                    rule_applied: InferenceRule::Subsumption,
                };
                nodes.push(chain_node);
                id_counter += 1;

                if &sc.superclass == superclass {
                    // Build final conclusion node
                    let child_refs: Vec<usize> = (0..id_counter).collect();
                    let root = ProofNode {
                        id: id_counter,
                        inference: Inference::Subsumption {
                            subclass: subclass.clone(),
                            superclass: superclass.clone(),
                        },
                        premises: justification.to_vec(),
                        children: child_refs,
                        rule_applied: InferenceRule::Subsumption,
                    };
                    return Ok(ProofTree { root, nodes });
                }
            }
        }

        // Fallback: simple root with all justification axioms as children
        let root = ProofNode {
            id: id_counter,
            inference: Inference::Subsumption {
                subclass: subclass.clone(),
                superclass: superclass.clone(),
            },
            premises: justification.to_vec(),
            children: child_ids,
            rule_applied: InferenceRule::Subsumption,
        };

        Ok(ProofTree { root, nodes })
    }

    fn build_inconsistency_proof_tree(&self, justification: &[Axiom]) -> Result<ProofTree> {
        let mut nodes = Vec::new();
        let mut child_ids = Vec::new();

        // Create premise leaves from justification axioms
        for (i, axiom) in justification.iter().enumerate() {
            let node = ProofNode {
                id: i,
                inference: Inference::Inconsistency,
                premises: vec![axiom.clone()],
                children: vec![],
                rule_applied: InferenceRule::Contradiction,
            };
            nodes.push(node);
            child_ids.push(i);
        }

        // Build clash derivation chain
        let mut derivation_children: Vec<usize> = Vec::new();

        // Pattern 1: Disjoint ∩ Equivalent classes
        let has_disjoint = justification
            .iter()
            .any(|ax| matches!(ax, Axiom::DisjointClasses(_)));
        let has_equiv = justification
            .iter()
            .any(|ax| matches!(ax, Axiom::EquivalentClasses(_)));
        if has_disjoint && has_equiv {
            let clash_node = ProofNode {
                id: nodes.len(),
                inference: Inference::TableauRule {
                    rule: "Clash:DisjointEquivalent".into(),
                    node: "root".into(),
                },
                premises: justification.to_vec(),
                children: child_ids.clone(),
                rule_applied: InferenceRule::Disjunction,
            };
            nodes.push(clash_node);
            derivation_children.push(nodes.len() - 1);
        }

        // Pattern 2: Self-complement subclass
        let has_complement = justification.iter().any(|ax| {
            if let Axiom::SubClassOf(sc) = ax {
                matches!(sc.superclass, ClassExpression::ObjectComplementOf(_))
            } else {
                false
            }
        });
        if has_complement {
            let clash_node = ProofNode {
                id: nodes.len(),
                inference: Inference::TableauRule {
                    rule: "Clash:ComplementSubsumption".into(),
                    node: "root".into(),
                },
                premises: justification.to_vec(),
                children: child_ids.clone(),
                rule_applied: InferenceRule::Contradiction,
            };
            nodes.push(clash_node);
            derivation_children.push(nodes.len() - 1);
        }

        // Pattern 3: Cardinality contradiction
        let has_card = justification.iter().any(|ax| {
            if let Axiom::SubClassOf(sc) = ax {
                matches!(sc.subclass, ClassExpression::ObjectIntersectionOf(_))
            } else {
                false
            }
        });
        if has_card {
            let clash_node = ProofNode {
                id: nodes.len(),
                inference: Inference::TableauRule {
                    rule: "Clash:Cardinality".into(),
                    node: "root".into(),
                },
                premises: justification.to_vec(),
                children: child_ids.clone(),
                rule_applied: InferenceRule::Contradiction,
            };
            nodes.push(clash_node);
            derivation_children.push(nodes.len() - 1);
        }

        // Root node: the inconsistency conclusion
        let root_children = if derivation_children.is_empty() {
            child_ids
        } else {
            derivation_children
        };

        let root = ProofNode {
            id: nodes.len(),
            inference: Inference::Inconsistency,
            premises: justification.to_vec(),
            children: root_children,
            rule_applied: InferenceRule::Contradiction,
        };

        Ok(ProofTree { root, nodes })
    }

    fn build_unsatisfiability_proof_tree(
        &self,
        class: &ClassExpression,
        justification: &[Axiom],
    ) -> Result<ProofTree> {
        let root = ProofNode {
            id: 0,
            inference: Inference::Unsatisfiability {
                class: class.clone(),
            },
            premises: justification.to_vec(),
            children: vec![],
            rule_applied: InferenceRule::Unsatisfiability,
        };

        Ok(ProofTree {
            root,
            nodes: vec![],
        })
    }
}

impl Default for ExplanationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete explanation for a reasoning result
#[derive(Debug, Clone)]
pub struct Explanation {
    /// The conclusion being explained
    pub conclusion: ExplanationConclusion,
    /// Minimal set of axioms needed for the conclusion
    pub justification: Vec<Axiom>,
    /// Proof tree showing the reasoning steps
    pub proof_tree: ProofTree,
    /// Type of explanation
    pub explanation_type: ExplanationType,
    /// Confidence in the explanation (0.0 to 1.0)
    pub confidence: f64,
}

/// Types of conclusions that can be explained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExplanationConclusion {
    /// Subsumption relationship
    Subsumption {
        subclass: ClassExpression,
        superclass: ClassExpression,
    },
    /// Inconsistency in the ontology
    Inconsistency,
    /// Unsatisfiable class
    Unsatisfiability { class: ClassExpression },
    /// Instance relationship
    InstanceOf {
        individual: Individual,
        class: ClassExpression,
    },
}

/// Types of explanations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplanationType {
    /// Explains why a subsumption holds
    Subsumption,
    /// Explains why the ontology is inconsistent
    Inconsistency,
    /// Explains why a class is unsatisfiable
    Unsatisfiability,
    /// Explains why an individual is an instance of a class
    InstanceOf,
}

/// Proof tree structure
#[derive(Debug, Clone)]
pub struct ProofTree {
    /// Root node of the proof tree
    pub root: ProofNode,
    /// All nodes in the tree
    pub nodes: Vec<ProofNode>,
}

/// Individual node in a proof tree
#[derive(Debug, Clone)]
pub struct ProofNode {
    /// Unique identifier for this node
    pub id: usize,
    /// The inference made at this node
    pub inference: Inference,
    /// Premises used for this inference
    pub premises: Vec<Axiom>,
    /// Child nodes
    pub children: Vec<usize>,
    /// Inference rule applied
    pub rule_applied: InferenceRule,
}

/// Types of inferences that can appear in proof trees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Inference {
    /// Subsumption inference
    Subsumption {
        subclass: ClassExpression,
        superclass: ClassExpression,
    },
    /// Inconsistency detection
    Inconsistency,
    /// Unsatisfiability detection
    Unsatisfiability { class: ClassExpression },
    /// Tableau rule application
    TableauRule { rule: String, node: String },
}

/// Inference rules used in reasoning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceRule {
    /// Direct subsumption
    Subsumption,
    /// Contradiction detection
    Contradiction,
    /// Unsatisfiability detection
    Unsatisfiability,
    /// Conjunction rule
    Conjunction,
    /// Disjunction rule
    Disjunction,
    /// Existential rule
    Existential,
    /// Universal rule
    Universal,
}

/// Justification computer for finding minimal axiom sets
pub struct JustificationComputer {
    cache: std::sync::Mutex<HashMap<String, Vec<Axiom>>>,
    reasoner_factory: Option<Arc<dyn crate::reasoner_api::ReasonerFactory>>,
}

impl std::fmt::Debug for JustificationComputer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JustificationComputer")
            .field("cache", &self.cache)
            .field(
                "reasoner_factory",
                &self.reasoner_factory.as_ref().map(|_| "ReasonerFactory"),
            )
            .finish()
    }
}

impl JustificationComputer {
    /// Create a new justification computer
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: std::sync::Mutex::new(HashMap::new()),
            reasoner_factory: None,
        }
    }

    /// Create a new justification computer with a reasoner factory
    /// for proper entailment checking.
    #[must_use]
    pub fn new_with_factory(
        reasoner_factory: Arc<dyn crate::reasoner_api::ReasonerFactory>,
    ) -> Self {
        Self {
            cache: std::sync::Mutex::new(HashMap::new()),
            reasoner_factory: Some(reasoner_factory),
        }
    }

    /// Compute justification for a subsumption.
    /// Tries reasoner-backed computation via BlackBoxExplanation first,
    /// then falls back to structural deletion-based minimization.
    pub fn compute_subsumption_justification(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Vec<Axiom>> {
        // Attempt reasoner-backed justification via BlackBoxExplanation
        if let Some(just) =
            self.compute_justification_with_reasoner(subclass, superclass, ontology_axioms)?
        {
            return Ok(just);
        }

        // Fallback: structural deletion-based MUS minimization
        let relevant_axioms = self.find_relevant_axioms(subclass, superclass, ontology_axioms);

        if relevant_axioms.is_empty() {
            return Ok(Vec::new());
        }

        let mut minimal_set = relevant_axioms.clone();
        let mut index = 0;

        while index < minimal_set.len() {
            let removed = minimal_set.remove(index);

            if self.still_entails_without(&minimal_set, subclass, superclass) {
                // axiom not essential
            } else {
                minimal_set.insert(index, removed);
                index += 1;
            }
        }

        Ok(minimal_set)
    }

    /// Compute justification using BlackBoxExplanation (reasoner-backed).
    /// Returns `None` if no reasoner factory is set or the result is empty.
    fn compute_justification_with_reasoner(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Option<Vec<Axiom>>> {
        let Some(factory) = &self.reasoner_factory else {
            return Ok(None);
        };

        let mut o = crate::ontology::Ontology::new();
        for ax in ontology_axioms {
            o.add_axiom(ax.clone());
        }
        let onto = OntologyRef::new(std::sync::RwLock::new(o));

        let entailment = Axiom::SubClassOf(crate::ontology::axioms::SubClassOfAxiom {
            id: 0,
            subclass: subclass.clone(),
            superclass: superclass.clone(),
            annotations: vec![],
        });

        let bb = blackbox::BlackBoxExplanation::new_with_ontology(
            onto,
            factory.clone(),
            blackbox::BlackBoxConfig::default(),
        );
        let explanation = bb.get_explanation(&entailment)?;
        if explanation.justification.is_empty() {
            Ok(None)
        } else {
            Ok(Some(explanation.justification))
        }
    }

    /// Check entailment using reasoner if available, returning None if not available
    fn check_entailment_via_reasoner(&self, axioms: &[Axiom], entailment: &Axiom) -> Option<bool> {
        let factory = self.reasoner_factory.as_ref()?;
        let mut o = crate::ontology::Ontology::new();
        for ax in axioms {
            o.add_axiom(ax.clone());
        }
        let onto = OntologyRef::new(std::sync::RwLock::new(o));
        let reasoner = factory.create_reasoner(&onto, &Default::default()).ok()?;
        reasoner.is_entailed(entailment).ok()
    }

    /// Check if entailment still holds with a subset of axioms.
    /// Comprehensive structural check including transitive subclass chains,
    /// equivalent classes, property domain/range, and disjoint class implications.
    fn still_entails_without(
        &self,
        axioms: &[Axiom],
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> bool {
        // 1. Try reasoner-based entailment check if factory is available
        let entailment = Axiom::SubClassOf(crate::ontology::axioms::SubClassOfAxiom {
            id: 0,
            subclass: subclass.clone(),
            superclass: superclass.clone(),
            annotations: vec![],
        });
        if let Some(result) = self.check_entailment_via_reasoner(axioms, &entailment) {
            return result;
        }

        // 2. Direct subclass check
        for ax in axioms {
            if let Axiom::SubClassOf(sc) = ax
                && &sc.subclass == subclass
                && &sc.superclass == superclass
            {
                return true;
            }
        }

        // 3. Transitive subclass closure through remaining axioms
        for ax in axioms {
            if let Axiom::SubClassOf(sc) = ax
                && &sc.subclass == subclass
                && self.check_transitive_chain(&sc.superclass, superclass, axioms)
            {
                return true;
            }
        }

        // 4. Equivalent classes check
        for ax in axioms {
            if let Axiom::EquivalentClasses(ec) = ax
                && ec.classes.contains(subclass)
                && ec.classes.contains(superclass)
            {
                return true;
            }
        }

        // 5. Property domain: A ⊑ ∃R.C implies A ⊑ domain(R) if domain(R)=C
        for ax in axioms {
            if let Axiom::ObjectPropertyDomain(dom) = ax
                && &dom.domain == superclass
            {
                for sc in axioms {
                    if let Axiom::SubClassOf(sub) = sc
                        && &sub.subclass == subclass
                        && let ClassExpression::ObjectSomeValuesFrom { property, .. } =
                            &sub.superclass
                        && *property == dom.property
                    {
                        return true;
                    }
                }
            }
        }

        // 6. Property range and subclass chains: A ⊑ ∀R.C and range(R)=D with C ⊑ D → A ⊑ D
        for ax in axioms {
            if let Axiom::ObjectPropertyRange(range) = ax
                && &range.range == superclass
            {
                for sc in axioms {
                    if let Axiom::SubClassOf(sub) = sc
                        && &sub.subclass == subclass
                        && let ClassExpression::ObjectAllValuesFrom { property, filler } =
                            &sub.superclass
                        && *property == range.property
                        && (filler.as_ref() == superclass
                            || self.check_transitive_chain(filler, superclass, axioms))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if there is a transitive subclass chain from `from` to `to` using the given axioms.
    /// Uses visited set to prevent infinite recursion.
    fn check_transitive_chain(
        &self,
        from: &ClassExpression,
        to: &ClassExpression,
        axioms: &[Axiom],
    ) -> bool {
        self.check_transitive_chain_impl(from, to, axioms, &mut HashSet::new())
    }

    fn check_transitive_chain_impl(
        &self,
        from: &ClassExpression,
        to: &ClassExpression,
        axioms: &[Axiom],
        visited: &mut HashSet<ClassExpression>,
    ) -> bool {
        if from == to {
            return true;
        }
        if !visited.insert(from.clone()) {
            return false;
        }

        for ax in axioms {
            if let Axiom::SubClassOf(sc) = ax
                && &sc.subclass == from
                && self.check_transitive_chain_impl(&sc.superclass, to, axioms, visited)
            {
                return true;
            }
            if let Axiom::EquivalentClasses(ec) = ax
                && ec.classes.contains(from)
            {
                for c in &ec.classes {
                    if c != from && self.check_transitive_chain_impl(c, to, axioms, visited) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Compute justification for inconsistency.
    /// Tries reasoner-backed computation first, then falls back to structural analysis.
    pub fn compute_inconsistency_justification(
        &self,
        ontology_axioms: &[Axiom],
    ) -> Result<Vec<Axiom>> {
        // Try reasoner-backed consistency check
        if let Some(factory) = &self.reasoner_factory {
            if ontology_axioms.is_empty() {
                return Ok(vec![]);
            }
            let mut o = crate::ontology::Ontology::new();
            for ax in ontology_axioms {
                o.add_axiom(ax.clone());
            }
            let onto = OntologyRef::new(std::sync::RwLock::new(o));
            if let Ok(reasoner) = factory.create_reasoner(&onto, &Default::default()) {
                if let Ok(true) = reasoner.is_consistent() {
                    return Ok(vec![]);
                }
                // Use deletion-based MUS to find minimal inconsistent subset
                let mut minimal_set = ontology_axioms.to_vec();
                let mut index = 0;
                while index < minimal_set.len() {
                    let removed = minimal_set.remove(index);
                    let mut test_o = crate::ontology::Ontology::new();
                    for ax in &minimal_set {
                        test_o.add_axiom(ax.clone());
                    }
                    let test_onto = OntologyRef::new(std::sync::RwLock::new(test_o));
                    if let Ok(r) = factory.create_reasoner(&test_onto, &Default::default()) {
                        if let Ok(false) = r.is_consistent() {
                            // still inconsistent
                        } else {
                            minimal_set.insert(index, removed);
                            index += 1;
                        }
                    } else {
                        minimal_set.insert(index, removed);
                        index += 1;
                    }
                }
                return Ok(minimal_set);
            }
        }

        // Fallback: structural inconsistency analysis
        if ontology_axioms.is_empty() {
            return Ok(Vec::new());
        }

        let mut minimal_set = ontology_axioms.to_vec();
        let mut index = 0;

        while index < minimal_set.len() {
            let removed = minimal_set.remove(index);

            if self.is_inconsistent(&minimal_set) {
                // Still inconsistent without this axiom
            } else {
                minimal_set.insert(index, removed);
                index += 1;
            }
        }

        Ok(minimal_set)
    }

    /// Check for inconsistency using structural analysis
    fn is_inconsistent(&self, axioms: &[Axiom]) -> bool {
        // Check for obvious contradictions

        // 1. Check for disjoint classes being asserted equivalent
        for axiom in axioms {
            if let Axiom::EquivalentClasses(equiv_data) = axiom {
                for other_axiom in axioms {
                    if let Axiom::DisjointClasses(disj_data) = other_axiom {
                        // If any two classes are both equivalent and disjoint, it's inconsistent
                        for c1 in &equiv_data.classes {
                            for c2 in &equiv_data.classes {
                                if c1 != c2
                                    && disj_data.classes.contains(c1)
                                    && disj_data.classes.contains(c2)
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Check for class being subclass of its complement
        for axiom in axioms {
            if let Axiom::SubClassOf(axiom_data) = axiom
                && let ClassExpression::ObjectComplementOf(inner) = &axiom_data.superclass
                && &axiom_data.subclass == inner.as_ref()
            {
                return true;
            }
        }

        // 3. Check for empty intersections (A ⊓ ¬A ⊑ ⊥)
        for axiom in axioms {
            if let Axiom::SubClassOf(axiom_data) = axiom
                && let ClassExpression::ObjectIntersectionOf(classes) = &axiom_data.subclass
            {
                // Check if intersection contains both a class and its complement
                for (i, c1) in classes.iter().enumerate() {
                    for c2 in classes.iter().skip(i + 1) {
                        if let ClassExpression::ObjectComplementOf(inner) = c2
                            && c1 == inner.as_ref()
                        {
                            return true;
                        }
                        if let ClassExpression::ObjectComplementOf(inner) = c1
                            && c2 == inner.as_ref()
                        {
                            return true;
                        }
                    }
                }
            }
        }

        // 4. Check for cardinality contradictions (≥n and ≤m where n > m)
        for axiom in axioms {
            if let Axiom::SubClassOf(axiom_data) = axiom
                && let ClassExpression::ObjectIntersectionOf(classes) = &axiom_data.subclass
            {
                let mut min_card = None;
                let mut max_card = None;

                for cls in classes {
                    if let ClassExpression::ObjectMinCardinality {
                        cardinality: n,
                        property: prop,
                        ..
                    } = cls
                    {
                        min_card = Some((*n, prop));
                    }
                    if let ClassExpression::ObjectMaxCardinality {
                        cardinality: m,
                        property: prop2,
                        ..
                    } = cls
                    {
                        max_card = Some((*m, prop2));
                    }
                }

                // Check if min > max for same property
                if let (Some((min, prop1)), Some((max, prop2))) = (min_card, max_card)
                    && prop1 == prop2
                    && min > max
                {
                    return true;
                }
            }
        }

        // Without a full reasoner, we can't determine all inconsistencies
        // Return false as conservative default
        false
    }

    /// Compute justification for unsatisfiability
    pub fn compute_unsatisfiability_justification(
        &self,
        class: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Vec<Axiom>> {
        // Find axioms that mention the unsatisfiable class
        let relevant_axioms = ontology_axioms
            .iter()
            .filter(|axiom| self.axiom_mentions_class(axiom, class))
            .cloned()
            .collect();
        Ok(relevant_axioms)
    }

    // Helper methods

    fn find_relevant_axioms(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Vec<Axiom> {
        ontology_axioms
            .iter()
            .filter(|axiom| {
                self.axiom_mentions_class(axiom, subclass)
                    || self.axiom_mentions_class(axiom, superclass)
            })
            .cloned()
            .collect()
    }

    fn axiom_mentions_class(&self, axiom: &Axiom, class: &ClassExpression) -> bool {
        // Comprehensive check that recursively inspects class expressions
        match axiom {
            Axiom::SubClassOf(axiom_data) => {
                self.class_expr_mentions(&axiom_data.subclass, class)
                    || self.class_expr_mentions(&axiom_data.superclass, class)
            }
            Axiom::EquivalentClasses(axiom_data) => axiom_data
                .classes
                .iter()
                .any(|c| self.class_expr_mentions(c, class)),
            Axiom::DisjointClasses(axiom_data) => axiom_data
                .classes
                .iter()
                .any(|c| self.class_expr_mentions(c, class)),
            Axiom::DisjointUnion(axiom_data) => {
                self.class_expr_mentions(&axiom_data.class, class)
                    || axiom_data
                        .disjoint_classes
                        .iter()
                        .any(|c| self.class_expr_mentions(c, class))
            }
            Axiom::ClassAssertion(axiom_data) => self.class_expr_mentions(&axiom_data.class, class),
            Axiom::ObjectPropertyDomain(axiom_data) => {
                self.class_expr_mentions(&axiom_data.domain, class)
            }
            Axiom::ObjectPropertyRange(axiom_data) => {
                self.class_expr_mentions(&axiom_data.range, class)
            }
            Axiom::DataPropertyDomain(axiom_data) => {
                self.class_expr_mentions(&axiom_data.domain, class)
            }
            _ => false,
        }
    }

    /// Check if a class expression mentions a specific class (recursively)
    fn class_expr_mentions(&self, expr: &ClassExpression, target: &ClassExpression) -> bool {
        if expr == target {
            return true;
        }

        match expr {
            ClassExpression::Class(_) => expr == target,

            ClassExpression::ObjectIntersectionOf(exprs)
            | ClassExpression::ObjectUnionOf(exprs) => {
                exprs.iter().any(|e| self.class_expr_mentions(e, target))
            }

            ClassExpression::ObjectComplementOf(inner) => self.class_expr_mentions(inner, target),

            ClassExpression::ObjectSomeValuesFrom { filler, .. }
            | ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                self.class_expr_mentions(filler, target)
            }

            ClassExpression::ObjectMinCardinality { filler, .. }
            | ClassExpression::ObjectMaxCardinality { filler, .. }
            | ClassExpression::ObjectExactCardinality { filler, .. } => {
                self.class_expr_mentions(filler, target)
            }

            ClassExpression::ObjectOneOf(_)
            | ClassExpression::ObjectHasValue { .. }
            | ClassExpression::ObjectHasSelf { .. }
            | ClassExpression::DataSomeValuesFrom { .. }
            | ClassExpression::DataAllValuesFrom { .. }
            | ClassExpression::DataHasValue { .. }
            | ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. } => false,
        }
    }
}

impl Default for JustificationComputer {
    fn default() -> Self {
        Self::new()
    }
}

/// Proof tracker for building explanation trees
#[derive(Debug)]
pub struct ProofTracker {
    steps: Vec<ReasoningStep>,
    node_map: HashMap<NodeId, Vec<ReasoningStep>>,
}

impl ProofTracker {
    /// Create a new proof tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a reasoning step
    pub fn add_step(&mut self, step: ReasoningStep) {
        if let Some(node_id) = step.node_id {
            self.node_map.entry(node_id).or_default().push(step.clone());
        }
        self.steps.push(step);
    }

    /// Get all steps for a node
    #[must_use]
    pub fn get_steps_for_node(&self, node_id: NodeId) -> Vec<&ReasoningStep> {
        self.node_map
            .get(&node_id)
            .map(|steps| steps.iter().collect())
            .unwrap_or_default()
    }

    /// Clear all tracking data
    pub fn clear(&mut self) {
        self.steps.clear();
        self.node_map.clear();
    }
}

impl Default for ProofTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual reasoning step in the proof
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    /// Node this step applies to (if any)
    pub node_id: Option<NodeId>,
    /// Rule applied in this step
    pub rule: InferenceRule,
    /// Premises used
    pub premises: Vec<Axiom>,
    /// Result of the step
    pub result: StepResult,
    /// Timestamp of the step
    pub timestamp: std::time::Instant,
}

/// Result of a reasoning step
#[derive(Debug, Clone)]
pub enum StepResult {
    /// New concept added to node
    ConceptAdded(ClassExpression),
    /// New edge created
    EdgeCreated {
        from: NodeId,
        to: NodeId,
        property: ObjectPropertyExpression,
    },
    /// Clash detected
    ClashDetected,
    /// Node blocked
    NodeBlocked(NodeId),
}

/// Explanation formatter for different output formats
#[derive(Debug)]
pub struct ExplanationFormatter;

impl ExplanationFormatter {
    /// Create a new explanation formatter
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format explanation in the specified format
    #[must_use]
    pub fn format(&self, explanation: &Explanation, format: ExplanationFormat) -> String {
        match format {
            ExplanationFormat::PlainText => self.format_plain_text(explanation),
            ExplanationFormat::Html => self.format_html(explanation),
            ExplanationFormat::Json => self.format_json(explanation),
            ExplanationFormat::Manchester => self.format_manchester(explanation),
        }
    }

    fn format_plain_text(&self, explanation: &Explanation) -> String {
        let mut result = String::new();

        match &explanation.conclusion {
            ExplanationConclusion::Subsumption {
                subclass,
                superclass,
            } => {
                result.push_str(&format!(
                    "Explanation for: {subclass:?} ⊑ {superclass:?}\n\n"
                ));
            }
            ExplanationConclusion::Inconsistency => {
                result.push_str("Explanation for inconsistency:\n\n");
            }
            ExplanationConclusion::Unsatisfiability { class } => {
                result.push_str(&format!(
                    "Explanation for unsatisfiability of: {class:?}\n\n"
                ));
            }
            ExplanationConclusion::InstanceOf { individual, class } => {
                result.push_str(&format!("Explanation for: {individual:?} : {class:?}\n\n"));
            }
        }

        result.push_str("Justification (minimal axiom set):\n");
        for (i, axiom) in explanation.justification.iter().enumerate() {
            result.push_str(&format!("{}. {:?}\n", i + 1, axiom));
        }

        result
    }

    fn format_html(&self, explanation: &Explanation) -> String {
        // HTML formatting implementation
        format!(
            "<html><body><h1>Explanation</h1><p>{}</p></body></html>",
            self.format_plain_text(explanation)
        )
    }

    fn format_json(&self, explanation: &Explanation) -> String {
        // Since Explanation doesn't implement Serialize (due to Axiom),
        // we create a simplified JSON representation
        let conclusion_str = format!("{:?}", explanation.conclusion);
        let justification_str = explanation
            .justification
            .iter()
            .map(|axiom| format!("{axiom:?}"))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"{{"conclusion": "{}", "justification": [{}], "explanation_type": "{:?}", "confidence": {}}}"#,
            conclusion_str.replace('"', r#"\""#),
            justification_str,
            explanation.explanation_type,
            explanation.confidence
        )
    }

    fn format_manchester(&self, explanation: &Explanation) -> String {
        // Manchester syntax formatting
        self.format_plain_text(explanation)
    }
}

impl Default for ExplanationFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Output formats for explanations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationFormat {
    /// Plain text format
    PlainText,
    /// HTML format
    Html,
    /// JSON format
    Json,
    /// Manchester syntax format
    Manchester,
}

impl fmt::Display for ExplanationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplanationType::Subsumption => write!(f, "Subsumption"),
            ExplanationType::Inconsistency => write!(f, "Inconsistency"),
            ExplanationType::Unsatisfiability => write!(f, "Unsatisfiability"),
            ExplanationType::InstanceOf => write!(f, "Instance Of"),
        }
    }
}

impl fmt::Display for ExplanationFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExplanationFormat::PlainText => write!(f, "text"),
            ExplanationFormat::Html => write!(f, "html"),
            ExplanationFormat::Json => write!(f, "json"),
            ExplanationFormat::Manchester => write!(f, "manchester"),
        }
    }
}
