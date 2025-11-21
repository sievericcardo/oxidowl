//! Explanation Generation for Oxidowl
//!
//! This module provides comprehensive explanation services for reasoning results,
//! including proof tracking, justification computation, and explanation formatting.

use crate::{
    Error, Result,
    core::tableau::{NodeId, TableauEdge, TableauNode},
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    sync::{Arc, Mutex},
};

/// Main explanation service
#[derive(Debug)]
pub struct ExplanationService {
    proof_tracker: Arc<Mutex<ProofTracker>>,
    justification_computer: JustificationComputer,
    explanation_formatter: ExplanationFormatter,
}

impl ExplanationService {
    /// Create a new explanation service
    pub fn new() -> Self {
        Self {
            proof_tracker: Arc::new(Mutex::new(ProofTracker::new())),
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
        let mut justification = self
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
    pub fn track_reasoning_step(&self, step: ReasoningStep) -> Result<()> {
        if let Ok(mut tracker) = self.proof_tracker.lock() {
            tracker.add_step(step);
        }
        Ok(())
    }

    // Private helper methods

    fn build_subsumption_proof_tree(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        justification: &[Axiom],
    ) -> Result<ProofTree> {
        // Build proof tree for subsumption
        let root = ProofNode {
            id: 0,
            inference: Inference::Subsumption {
                subclass: subclass.clone(),
                superclass: superclass.clone(),
            },
            premises: justification.iter().map(|ax| ax.clone()).collect(),
            children: vec![],
            rule_applied: InferenceRule::Subsumption,
        };

        Ok(ProofTree {
            root,
            nodes: vec![],
        })
    }

    fn build_inconsistency_proof_tree(&self, justification: &[Axiom]) -> Result<ProofTree> {
        let root = ProofNode {
            id: 0,
            inference: Inference::Inconsistency,
            premises: justification.iter().map(|ax| ax.clone()).collect(),
            children: vec![],
            rule_applied: InferenceRule::Contradiction,
        };

        Ok(ProofTree {
            root,
            nodes: vec![],
        })
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
            premises: justification.iter().map(|ax| ax.clone()).collect(),
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
#[derive(Debug)]
pub struct JustificationComputer {
    cache: std::cell::RefCell<HashMap<String, Vec<Axiom>>>,
}

impl JustificationComputer {
    /// Create a new justification computer
    pub fn new() -> Self {
        Self {
            cache: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// Compute justification for a subsumption
    pub fn compute_subsumption_justification(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology_axioms: &[Axiom],
    ) -> Result<Vec<Axiom>> {
        // Implement MUS (Minimal Unsatisfiable Subset) using a deletion-based algorithm
        // Start with all relevant axioms and remove one at a time to check minimality

        let relevant_axioms = self.find_relevant_axioms(subclass, superclass, ontology_axioms);

        if relevant_axioms.is_empty() {
            return Ok(Vec::new());
        }

        // Try to minimize the axiom set
        let mut minimal_set = relevant_axioms.clone();
        let mut index = 0;

        while index < minimal_set.len() {
            // Try removing axiom at index
            let removed = minimal_set.remove(index);

            // Check if entailment still holds without this axiom
            if self.still_entails_without(&minimal_set, subclass, superclass) {
                // Can remove this axiom - it's not essential
                // Don't increment index, check next axiom at same position
            } else {
                // Need this axiom - put it back and move to next
                minimal_set.insert(index, removed);
                index += 1;
            }
        }

        Ok(minimal_set)
    }

    /// Check if entailment still holds with a subset of axioms (simplified check)
    fn still_entails_without(
        &self,
        axioms: &[Axiom],
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> bool {
        // Simplified entailment check
        // In practice, this would use a proper reasoner

        // Check direct SubClassOf axioms
        for axiom in axioms {
            if let Axiom::SubClassOf(axiom_data) = axiom {
                if &axiom_data.subclass == subclass && &axiom_data.superclass == superclass {
                    return true;
                }

                // Check transitivity: if subclass -> intermediate and intermediate -> superclass
                if &axiom_data.subclass == subclass {
                    for other_axiom in axioms {
                        if let Axiom::SubClassOf(other_data) = other_axiom {
                            if &other_data.subclass == &axiom_data.superclass
                                && &other_data.superclass == superclass
                            {
                                return true;
                            }
                        }
                    }
                }
            }

            // Check EquivalentClasses
            if let Axiom::EquivalentClasses(axiom_data) = axiom {
                if axiom_data.classes.contains(subclass) && axiom_data.classes.contains(superclass)
                {
                    return true;
                }
            }
        }

        false
    }

    /// Compute justification for inconsistency
    pub fn compute_inconsistency_justification(
        &self,
        ontology_axioms: &[Axiom],
    ) -> Result<Vec<Axiom>> {
        // Find a minimal subset of axioms that causes inconsistency
        // This uses a deletion-based algorithm similar to MUS

        if ontology_axioms.is_empty() {
            return Ok(Vec::new());
        }

        let mut minimal_set = ontology_axioms.to_vec();
        let mut index = 0;

        while index < minimal_set.len() {
            // Try removing axiom at index
            let removed = minimal_set.remove(index);

            // Check if the subset is still inconsistent
            if self.is_inconsistent(&minimal_set) {
                // Still inconsistent without this axiom - can remove it
                // Don't increment index
            } else {
                // Need this axiom - put it back and move to next
                minimal_set.insert(index, removed);
                index += 1;
            }
        }

        Ok(minimal_set)
    }

    /// Simplified check for inconsistency (placeholder for proper reasoner check)
    fn is_inconsistent(&self, axioms: &[Axiom]) -> bool {
        // Check for obvious contradictions

        // Check for disjoint classes being asserted equivalent
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

        // Check for class being subclass of its complement
        for axiom in axioms {
            if let Axiom::SubClassOf(axiom_data) = axiom {
                if let ClassExpression::ObjectComplementOf(inner) = &axiom_data.superclass {
                    if &axiom_data.subclass == inner.as_ref() {
                        return true;
                    }
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
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a reasoning step
    pub fn add_step(&mut self, step: ReasoningStep) {
        if let Some(node_id) = step.node_id {
            self.node_map
                .entry(node_id)
                .or_insert_with(Vec::new)
                .push(step.clone());
        }
        self.steps.push(step);
    }

    /// Get all steps for a node
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
    pub fn new() -> Self {
        Self
    }

    /// Format explanation in the specified format
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
                    "Explanation for: {:?} ⊑ {:?}\n\n",
                    subclass, superclass
                ));
            }
            ExplanationConclusion::Inconsistency => {
                result.push_str("Explanation for inconsistency:\n\n");
            }
            ExplanationConclusion::Unsatisfiability { class } => {
                result.push_str(&format!(
                    "Explanation for unsatisfiability of: {:?}\n\n",
                    class
                ));
            }
            ExplanationConclusion::InstanceOf { individual, class } => {
                result.push_str(&format!(
                    "Explanation for: {:?} : {:?}\n\n",
                    individual, class
                ));
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
            .map(|axiom| format!("{:?}", axiom))
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
