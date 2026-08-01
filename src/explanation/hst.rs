//! Hitting Set Tree (HST) explanation generator.
//! Computes ALL minimal justifications for an entailment using Reiter's algorithm.

use super::generator::{Explanation, ExplanationGenerator};
use crate::Result;
use crate::ontology::OntologyRef;
use crate::ontology::axioms::{Axiom, AxiomId, AxiomTrait};
use crate::reasoner_api::ReasonerFactory;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for HST-based explanation generation.
#[derive(Debug, Clone)]
pub struct HSTConfig {
    pub max_depth: usize,
    pub max_justifications: usize,
}

impl Default for HSTConfig {
    fn default() -> Self {
        Self {
            max_depth: 50,
            max_justifications: 50,
        }
    }
}

/// A node in the Hitting Set Tree.
#[derive(Debug, Clone)]
struct HSTNode {
    path_constraints: HashSet<AxiomId>,
    explored: bool,
    justification: Option<Vec<Axiom>>,
    children: Vec<(AxiomId, usize)>,
}

/// Internal state for the HST breadth-first traversal.
struct HSTState {
    nodes: Vec<HSTNode>,
    justifications: Vec<Vec<Axiom>>,
    queue: VecDeque<usize>,
    visited_paths: HashSet<Vec<AxiomId>>,
}

impl HSTState {
    fn new() -> Self {
        let root = HSTNode {
            path_constraints: HashSet::new(),
            explored: false,
            justification: None,
            children: Vec::new(),
        };
        let mut queue = VecDeque::new();
        queue.push_back(0);
        Self {
            nodes: vec![root],
            justifications: Vec::new(),
            queue,
            visited_paths: HashSet::new(),
        }
    }

    fn is_path_closed(&self, path_constraints: &HashSet<AxiomId>, justifications: &[Vec<Axiom>]) -> bool {
        if justifications.is_empty() {
            return false;
        }
        justifications.iter().all(|j| {
            j.iter().any(|ax| path_constraints.contains(&ax.axiom_id()))
        })
    }

    fn is_path_covered(&self, path_constraints: &HashSet<AxiomId>) -> bool {
        if path_constraints.is_empty() {
            return false;
        }
        let mut sorted: Vec<AxiomId> = path_constraints.iter().copied().collect();
        sorted.sort_unstable();
        self.visited_paths.contains(&sorted)
    }

    fn mark_path_visited(&mut self, path_constraints: &HashSet<AxiomId>) {
        let mut sorted: Vec<AxiomId> = path_constraints.iter().copied().collect();
        sorted.sort_unstable();
        self.visited_paths.insert(sorted);
    }
}

/// Hitting Set Tree generator — finds multiple minimal justifications.
pub struct HSTExplanationGenerator {
    ontology: Option<OntologyRef>,
    factory: Arc<dyn ReasonerFactory>,
    config: HSTConfig,
}

impl HSTExplanationGenerator {
    #[must_use]
    pub fn new(factory: Arc<dyn ReasonerFactory>, config: HSTConfig) -> Self {
        Self {
            ontology: None,
            factory,
            config,
        }
    }

    #[must_use]
    pub fn new_with_ontology(
        ontology: OntologyRef,
        factory: Arc<dyn ReasonerFactory>,
        config: HSTConfig,
    ) -> Self {
        Self {
            ontology: Some(ontology),
            factory,
            config,
        }
    }

    /// Find up to `limit` minimal justifications using Reiter's HST algorithm.
    ///
    /// Algorithm:
    /// 1. Start with an empty set of justifications F.
    /// 2. Create an HST with root node labeled by empty path constraints.
    /// 3. Breadth-first traversal: for each node n with path constraints h(n):
    ///    a. Find a justification j whose axioms are disjoint from h(n).
    ///    b. If none exists, mark n as closed.
    ///    c. If j is already known, mark n as duplicate.
    ///    d. Otherwise add j to F, and for each axiom a in j, create child with
    ///       path constraints h(n) ∪ {a} if not already covered.
    /// 4. Return F when done or limit reached.
    pub fn find_justifications(
        &self,
        ontology: &OntologyRef,
        entailment: &Axiom,
        limit: usize,
    ) -> Result<Vec<Vec<Axiom>>> {
        let axioms: Vec<Axiom> = {
            let guard = ontology.read().map_err(|e| crate::Error::Internal {
                message: format!("{e}"),
            })?;
            guard.axioms().to_vec()
        };

        if axioms.is_empty() {
            return Ok(vec![]);
        }

        let max_depth = self.config.max_depth;
        let max_justifications = self.config.max_justifications.min(limit);

        let mut state = HSTState::new();

        // Breadth-first HST traversal
        while let Some(node_idx) = state.queue.pop_front() {
            if state.justifications.len() >= max_justifications {
                break;
            }

            // Mark this node as explored
            state.nodes[node_idx].explored = true;

            let path_constraints = state.nodes[node_idx].path_constraints.clone();

            // Check if this path is closed given current justifications
            if state.is_path_closed(&path_constraints, &state.justifications) {
                continue;
            }

            // Find a justification whose axioms are disjoint from path constraints
            let j = self.find_justification_avoiding(&axioms, entailment, &path_constraints)?;

            if j.is_empty() {
                // No justification exists — node is closed
                continue;
            }

            // Check if this justification is already known
            if state.justifications.iter().any(|existing| {
                existing.len() == j.len()
                    && existing.iter().all(|ax| j.contains(ax))
            }) {
                continue;
            }

            // New justification found — add to F
            state.justifications.push(j.clone());

            if state.justifications.len() >= max_justifications {
                break;
            }

            // Create children: for each axiom in the justification, add edge to new node
            for ax in &j {
                let mut child_constraints = path_constraints.clone();
                child_constraints.insert(ax.axiom_id());

                // Skip if this path is already covered or exceeds max depth
                let depth = child_constraints.len();
                if depth > max_depth || state.is_path_covered(&child_constraints) {
                    continue;
                }

                state.mark_path_visited(&child_constraints);

                let child_idx = state.nodes.len();
                state.nodes.push(HSTNode {
                    path_constraints: child_constraints,
                    explored: false,
                    justification: None,
                    children: Vec::new(),
                });
                state.nodes[node_idx].children.push((ax.axiom_id(), child_idx));
                state.queue.push_back(child_idx);
            }

            // Also process: if this node was the root and we found a new justification,
            // we still continue exploring deeper paths
        }

        Ok(state.justifications)
    }

    /// Find a single minimal justification whose axioms are disjoint from `avoid`.
    fn find_justification_avoiding(
        &self,
        axioms: &[Axiom],
        entailment: &Axiom,
        avoid: &HashSet<AxiomId>,
    ) -> Result<Vec<Axiom>> {
        // Filter out axioms that are in the avoid set
        let filtered: Vec<Axiom> = axioms
            .iter()
            .filter(|a| !avoid.contains(&a.axiom_id()))
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Ok(vec![]);
        }

        self.expand_shrink(&filtered, entailment)
    }

    fn expand_shrink(&self, axioms: &[Axiom], entailment: &Axiom) -> Result<Vec<Axiom>> {
        if axioms.is_empty() {
            return Ok(vec![]);
        }

        // Expand: start with all, remove each to see if still entailed
        let mut essential: HashSet<AxiomId> = HashSet::new();
        for ax in axioms {
            let test_set: Vec<Axiom> = axioms
                .iter()
                .filter(|a| a.axiom_id() != ax.axiom_id())
                .cloned()
                .collect();
            if test_set.is_empty() {
                continue;
            }
            let onto = Self::build_onto(&test_set);
            if let Ok(reasoner) = self.factory.create_reasoner(&onto, &Default::default()) {
                if reasoner.is_entailed(entailment).unwrap_or(false) {
                    // ax is NOT essential
                } else {
                    essential.insert(ax.axiom_id());
                }
            }
        }

        // Shrink: try removing each essential
        let mut minimal: Vec<Axiom> = Vec::new();
        for ax in axioms.iter().filter(|a| essential.contains(&a.axiom_id())) {
            let test_set: Vec<Axiom> = minimal.iter().chain(std::iter::once(ax)).cloned().collect();
            let onto = Self::build_onto(&test_set);
            if let Ok(reasoner) = self.factory.create_reasoner(&onto, &Default::default())
                && reasoner.is_entailed(entailment).unwrap_or(false) {
                    minimal.push(ax.clone());
                }
        }

        Ok(minimal)
    }

    fn build_onto(axioms: &[Axiom]) -> OntologyRef {
        let mut o = crate::ontology::Ontology::new();
        for ax in axioms {
            o.add_axiom(ax.clone());
        }
        OntologyRef::new(std::sync::RwLock::new(o))
    }
}

impl ExplanationGenerator for HSTExplanationGenerator {
    fn get_explanation(&self, entailment: &Axiom) -> Result<Explanation> {
        if let Some(ref ontology) = self.ontology {
            let justifications = self.find_justifications(ontology, entailment, 1)?;
            if let Some(first_just) = justifications.into_iter().next() {
                Ok(Explanation {
                    entailment: entailment.clone(),
                    justification: first_just,
                    is_minimal: true,
                    computation_time: Duration::default(),
                })
            } else {
                Ok(Explanation {
                    entailment: entailment.clone(),
                    justification: vec![],
                    is_minimal: true,
                    computation_time: Duration::default(),
                })
            }
        } else {
            Err(crate::Error::Unsupported {
                message: "HST requires explicit ontology".into(),
            })
        }
    }

    fn get_explanations(&self, entailment: &Axiom, limit: usize) -> Result<Vec<Explanation>> {
        if let Some(ref ontology) = self.ontology {
            let justifications = self.find_justifications(ontology, entailment, limit)?;
            Ok(justifications
                .into_iter()
                .map(|j| Explanation {
                    entailment: entailment.clone(),
                    justification: j,
                    is_minimal: true,
                    computation_time: Duration::default(),
                })
                .collect())
        } else {
            Err(crate::Error::Unsupported {
                message: "HST requires explicit ontology".into(),
            })
        }
    }
}
