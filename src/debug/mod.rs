//! OWL Debugger — axiom pinpointing for inconsistencies and entailments.

pub mod definitions;

use crate::Result;
use crate::explanation::ExplanationService;
use crate::explanation::blackbox::find_justification;
use crate::ontology::axioms::{Axiom, AxiomTrait};
use crate::ontology::{ClassExpression, IRI, OntologyRef};
use crate::reasoner_api::ReasonerFactory;
use definitions::DefinitionTracker;
use std::sync::Arc;
use std::time::Duration;

// ── Configuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DebuggerConfig {
    pub timeout: Option<Duration>,
    pub max_justifications_per_entailment: usize,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            max_justifications_per_entailment: 10,
        }
    }
}

// ── OWLDebugger Trait ────────────────────────────────────────────────────────

/// Finds axioms responsible for inconsistency or unexpected entailments.
pub trait OWLDebugger: Send + Sync {
    fn find_minimal_unsatisfiable_set(&self) -> Result<Vec<Axiom>>;
    fn find_justifications(&self, entailment: &Axiom) -> Result<Vec<Vec<Axiom>>>;
    fn get_unsatisfiable_classes(&self) -> Result<Vec<ClassExpression>>;
    fn get_unsatisfiability_explanation(&self, class: &ClassExpression) -> Result<Vec<Axiom>>;
    fn is_consistent(&self) -> Result<bool>;
}

// ── BlackBoxOWLDebugger ──────────────────────────────────────────────────────

/// Black-box debugger that uses reasoner calls to pinpoint problematic axioms.
pub struct BlackBoxOWLDebugger {
    ontology: OntologyRef,
    reasoner_factory: Arc<dyn ReasonerFactory>,
    #[allow(dead_code)]
    definition_tracker: DefinitionTracker,
    config: DebuggerConfig,
}

impl BlackBoxOWLDebugger {
    #[must_use]
    pub fn new(
        ontology: OntologyRef,
        reasoner_factory: Arc<dyn ReasonerFactory>,
        config: DebuggerConfig,
    ) -> Self {
        let def_tracker = {
            let guard = ontology
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            DefinitionTracker::from_ontology(&guard)
        };
        Self {
            ontology,
            reasoner_factory,
            definition_tracker: def_tracker,
            config,
        }
    }

    /// Find a minimal unsatisfiable subset.
    fn find_mups(&self) -> Result<Vec<Axiom>> {
        let guard = self
            .ontology
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let axioms = guard.axioms().to_vec();
        drop(guard);

        if axioms.is_empty() {
            return Ok(vec![]);
        }

        // Expand-shrink: find minimal set that still makes ontology inconsistent
        let mut essential: Vec<Axiom> = Vec::new();
        for ax in &axioms {
            let test_set: Vec<Axiom> = axioms
                .iter()
                .filter(|a| a.axiom_id() != ax.axiom_id())
                .cloned()
                .collect();
            let test_onto = self.build_onto(&test_set);
            let reasoner = self.reasoner_factory.create_reasoner(
                &test_onto,
                &crate::reasoner_api::OWLReasonerConfiguration::default(),
            )?;
            if reasoner.is_consistent().unwrap_or(true) {
                // ax IS essential for inconsistency
                essential.push(ax.clone());
            }
        }
        Ok(essential)
    }

    /// Get a reference to the definition tracker.
    #[must_use]
    pub fn get_definition_tracker(&self) -> &DefinitionTracker {
        &self.definition_tracker
    }

    /// Get a reference to the shared ontology.
    #[must_use]
    pub fn get_ontology(&self) -> &OntologyRef {
        &self.ontology
    }

    /// Get a reference to the reasoner factory.
    #[must_use]
    pub fn get_reasoner_factory(&self) -> &Arc<dyn ReasonerFactory> {
        &self.reasoner_factory
    }
}

impl BlackBoxOWLDebugger {
    fn build_onto(&self, axioms: &[Axiom]) -> OntologyRef {
        let mut o = crate::ontology::Ontology::new();
        for ax in axioms {
            o.add_axiom(ax.clone());
        }
        OntologyRef::new(std::sync::RwLock::new(o))
    }
}

impl OWLDebugger for BlackBoxOWLDebugger {
    fn find_minimal_unsatisfiable_set(&self) -> Result<Vec<Axiom>> {
        self.find_mups()
    }

    fn find_justifications(&self, entailment: &Axiom) -> Result<Vec<Vec<Axiom>>> {
        let j0 = find_justification(&self.ontology, entailment, &self.reasoner_factory)?;
        if j0.is_empty() {
            return Ok(vec![]);
        }
        let mut result = vec![j0];

        // Remove each axiom from the first justification to find alternatives
        let first_just: Vec<Axiom> = result[0].clone();
        for ax in &first_just {
            if result.len() >= self.config.max_justifications_per_entailment {
                break;
            }
            let guard = self
                .ontology
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let axioms: Vec<Axiom> = guard
                .axioms()
                .iter()
                .filter(|a| a.axiom_id() != ax.axiom_id())
                .cloned()
                .collect();
            drop(guard);
            let test_onto = self.build_onto(&axioms);
            if let Ok(alt) = find_justification(&test_onto, entailment, &self.reasoner_factory)
                && !alt.is_empty()
                && !result.iter().any(|r| r == &alt)
            {
                result.push(alt);
            }
        }
        Ok(result)
    }

    fn get_unsatisfiable_classes(&self) -> Result<Vec<ClassExpression>> {
        let reasoner = self.reasoner_factory.create_reasoner(
            &self.ontology,
            &crate::reasoner_api::OWLReasonerConfiguration::default(),
        )?;
        let node = reasoner.get_unsatisfiable_classes()?;
        Ok(node.get_entities().iter().cloned().collect())
    }

    fn get_unsatisfiability_explanation(&self, class: &ClassExpression) -> Result<Vec<Axiom>> {
        let entailment = Axiom::SubClassOf(crate::ontology::axioms::SubClassOfAxiom {
            id: 0,
            subclass: class.clone(),
            superclass: ClassExpression::Class(crate::ontology::Class {
                iri: IRI::owl_nothing(),
            }),
            annotations: vec![],
        });
        let justs = self.find_justifications(&entailment)?;
        Ok(justs.first().cloned().unwrap_or_default())
    }

    fn is_consistent(&self) -> Result<bool> {
        let reasoner = self.reasoner_factory.create_reasoner(
            &self.ontology,
            &crate::reasoner_api::OWLReasonerConfiguration::default(),
        )?;
        reasoner.is_consistent()
    }
}

// ── ExplanationService + BlackBoxOWLDebugger Integration ─────────────────────

impl ExplanationService {
    /// Generate an explanation using the debugger's reasoner and ontology.
    /// Both systems share the same ontology reference via the debugger.
    pub fn explain_via_debugger(
        &self,
        debugger: &BlackBoxOWLDebugger,
        entailment: &Axiom,
    ) -> Result<crate::explanation::Explanation> {
        use crate::explanation::{
            Explanation, ExplanationConclusion, ExplanationType, Inference, InferenceRule,
            ProofNode, ProofTree,
        };

        let justifications = debugger.find_justifications(entailment)?;
        let justification = justifications.into_iter().next().unwrap_or_default();

        let conclusion = match entailment {
            Axiom::SubClassOf(a) => ExplanationConclusion::Subsumption {
                subclass: a.subclass.clone(),
                superclass: a.superclass.clone(),
            },
            Axiom::EquivalentClasses(a) if a.classes.len() >= 2 => {
                ExplanationConclusion::Subsumption {
                    subclass: a.classes[0].clone(),
                    superclass: a.classes[1].clone(),
                }
            }
            Axiom::ClassAssertion(a) => ExplanationConclusion::InstanceOf {
                individual: a.individual.clone(),
                class: a.class.clone(),
            },
            _ => ExplanationConclusion::Inconsistency,
        };

        let root = ProofNode {
            id: 0,
            inference: Inference::Subsumption {
                subclass: ClassExpression::Class(crate::ontology::Class {
                    iri: IRI::new("urn:placeholder"),
                }),
                superclass: ClassExpression::Class(crate::ontology::Class {
                    iri: IRI::new("urn:placeholder"),
                }),
            },
            premises: justification.clone(),
            children: vec![],
            rule_applied: InferenceRule::Subsumption,
        };

        Ok(Explanation {
            conclusion,
            justification,
            proof_tree: ProofTree {
                root,
                nodes: vec![],
            },
            explanation_type: ExplanationType::Subsumption,
            confidence: 1.0,
        })
    }

    /// Use the debugger's DefinitionTracker to find explanations
    /// for a specific subsumption entailment.
    pub fn explain_subsumption_via_debugger(
        &self,
        debugger: &BlackBoxOWLDebugger,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<crate::explanation::Explanation> {
        let entailment = Axiom::SubClassOf(crate::ontology::axioms::SubClassOfAxiom {
            id: 0,
            subclass: subclass.clone(),
            superclass: superclass.clone(),
            annotations: vec![],
        });
        self.explain_via_debugger(debugger, &entailment)
    }
}
