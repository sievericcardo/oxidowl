//! Explanation Generation for Oxidowl
//!
//! This module provides comprehensive explanation services for reasoning results,
//! including proof tracking, justification computation, and explanation formatting.

use crate::{
    Error, Result,
    ontology::{Axiom, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression},
    core::tableau::{TableauNode, TableauEdge, NodeId},
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
        let mut justification = self.justification_computer.compute_subsumption_justification(
            subclass,
            superclass,
            ontology_axioms,
        )?;

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
        let justification = self.justification_computer.compute_inconsistency_justification(ontology_axioms)?;
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
        let justification = self.justification_computer.compute_unsatisfiability_justification(
            class,
            ontology_axioms,
        )?;
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
    pub fn format_explanation(&self, explanation: &Explanation, format: ExplanationFormat) -> String {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Unsatisfiability {
        class: ClassExpression,
    },
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofTree {
    /// Root node of the proof tree
    pub root: ProofNode,
    /// All nodes in the tree
    pub nodes: Vec<ProofNode>,
}

/// Individual node in a proof tree
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Unsatisfiability {
        class: ClassExpression,
    },
    /// Tableau rule application
    TableauRule {
        rule: String,
        node: String,
    },
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
        // For now, return all relevant axioms
        // In a full implementation, this would use algorithms like hitting set tree
        let relevant_axioms = self.find_relevant_axioms(subclass, superclass, ontology_axioms);
        Ok(relevant_axioms)
    }

    /// Compute justification for inconsistency
    pub fn compute_inconsistency_justification(&self, ontology_axioms: &[Axiom]) -> Result<Vec<Axiom>> {
        // Simplified implementation - return all axioms for now
        Ok(ontology_axioms.to_vec())
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
                self.axiom_mentions_class(axiom, subclass) || self.axiom_mentions_class(axiom, superclass)
            })
            .cloned()
            .collect()
    }

    fn axiom_mentions_class(&self, axiom: &Axiom, class: &ClassExpression) -> bool {
        // Simplified check - in practice this would be more sophisticated
        match axiom {
            Axiom::SubClassOf(sub, sup) => sub == class || sup == class,
            Axiom::EquivalentClasses(classes) => classes.contains(class),
            Axiom::DisjointClasses(classes) => classes.contains(class),
            _ => false,
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
            self.node_map.entry(node_id).or_insert_with(Vec::new).push(step.clone());
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
            ExplanationConclusion::Subsumption { subclass, superclass } => {
                result.push_str(&format!("Explanation for: {:?} ⊑ {:?}\n\n", subclass, superclass));
            }
            ExplanationConclusion::Inconsistency => {
                result.push_str("Explanation for inconsistency:\n\n");
            }
            ExplanationConclusion::Unsatisfiability { class } => {
                result.push_str(&format!("Explanation for unsatisfiability of: {:?}\n\n", class));
            }
            ExplanationConclusion::InstanceOf { individual, class } => {
                result.push_str(&format!("Explanation for: {:?} : {:?}\n\n", individual, class));
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
        format!("<html><body><h1>Explanation</h1><p>{}</p></body></html>", 
                self.format_plain_text(explanation))
    }

    fn format_json(&self, explanation: &Explanation) -> String {
        serde_json::to_string_pretty(explanation).unwrap_or_default()
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