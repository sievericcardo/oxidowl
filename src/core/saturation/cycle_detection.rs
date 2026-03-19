//! Cycle detection for saturation-based reasoning
//!
//! Detects cyclic dependencies in concept derivation chains to prevent infinite
//! loops during saturation. Implements the cycle detection strategy from
//! Konclude's saturation precomputation phase.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Classification of cycle types detected during saturation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleType {
    /// A cycle involving only concept (class) expressions
    ConceptCycle,
    /// A cycle involving role (property) chains
    RoleCycle,
    /// A cycle involving nominal (individual) expressions
    NominalCycle,
}

/// Information about a detected derivation cycle
#[derive(Debug, Clone)]
pub struct CycleInfo {
    /// Number of nodes in the cycle
    pub cycle_length: usize,
    /// Ordered list of concept IRIs forming the cycle path
    pub cycle_path: Vec<String>,
    /// Categorisation of the cycle
    pub cycle_type: CycleType,
}

/// Per-concept tracking state used internally by [`CycleDetector`]
#[derive(Debug)]
struct ConceptState {
    /// Current derivation depth when this concept is on the active stack
    depth: u32,
    /// Whether this concept is currently on the DFS stack (being explored)
    on_stack: bool,
    /// Unique sequence number assigned at first visit
    visit_order: u64,
    /// Lowest visit-order reachable from this node (Tarjan low-link)
    low_link: u64,
}

/// Thread-safe cycle detector for concept derivation chains.
///
/// Uses a Tarjan-style DFS tracking approach via a global visit counter and
/// per-concept state stored in a [`DashMap`]. When `detect_cycle` is called
/// with a concept and its derivation chain, the detector checks whether the
/// concept already appears within the chain (a *back edge*) and returns
/// [`CycleInfo`] if it does.
pub struct CycleDetector {
    /// Per-concept state, keyed by concept IRI string
    concept_state: DashMap<String, ConceptState>,
    /// Monotonically increasing visit counter
    visit_counter: AtomicU64,
    /// Maximum derivation chain depth before a cycle is forced (safety bound)
    max_depth: u32,
}

impl CycleDetector {
    /// Create a new `CycleDetector` with the given maximum depth bound.
    ///
    /// `max_depth` is typically set to the number of named concepts in the
    /// ontology plus a small constant. Values in the range 512–4096 are
    /// reasonable for most practical ontologies.
    pub fn new(max_depth: u32) -> Self {
        Self {
            concept_state: DashMap::new(),
            visit_counter: AtomicU64::new(1),
            max_depth,
        }
    }

    /// Check whether `concept_iri` introduces a cycle given the current
    /// `derivation_chain` (the sequence of concept IRIs leading to this
    /// point in the saturation).
    ///
    /// Returns `Some(CycleInfo)` if a cycle is detected, or `None` if the
    /// derivation is acyclic so far.
    pub fn detect_cycle(
        &self,
        concept_iri: &str,
        derivation_chain: &[String],
    ) -> Option<CycleInfo> {
        // Depth-exceeded: treat as a forced cycle.
        if derivation_chain.len() as u32 >= self.max_depth {
            let cycle_path = derivation_chain.to_vec();
            return Some(CycleInfo {
                cycle_length: cycle_path.len(),
                cycle_path,
                cycle_type: classify_cycle_type(derivation_chain),
            });
        }

        // Check whether concept_iri already appears in the chain (back edge).
        if let Some(pos) = derivation_chain.iter().position(|c| c == concept_iri) {
            let cycle_path: Vec<String> = derivation_chain[pos..].to_vec();
            let cycle_type = classify_cycle_type(&cycle_path);
            return Some(CycleInfo {
                cycle_length: cycle_path.len(),
                cycle_path,
                cycle_type,
            });
        }

        // Mark concept as visited with a new sequence number.
        let visit_order = self.visit_counter.fetch_add(1, Ordering::Relaxed);
        self.concept_state
            .entry(concept_iri.to_string())
            .and_modify(|s| {
                s.visit_order = visit_order;
                s.low_link = visit_order;
                s.on_stack = true;
                s.depth = derivation_chain.len() as u32;
            })
            .or_insert_with(|| ConceptState {
                depth: derivation_chain.len() as u32,
                on_stack: true,
                visit_order,
                low_link: visit_order,
            });

        None
    }

    /// Notify the detector that we have finished exploring `concept_iri`
    /// (popping it off the DFS stack). This updates the low-link value of
    /// the parent concept if one is supplied.
    pub fn finish_concept(&self, concept_iri: &str, parent_iri: Option<&str>) {
        let low_link = {
            if let Some(mut state) = self.concept_state.get_mut(concept_iri) {
                state.on_stack = false;
                state.low_link
            } else {
                return;
            }
        };

        // Propagate low-link to parent (Tarjan relaxation step).
        if let Some(parent) = parent_iri
            && let Some(mut parent_state) = self.concept_state.get_mut(parent)
            && low_link < parent_state.low_link
        {
            parent_state.low_link = low_link;
        }
    }

    /// Reset the detector state. Call this between independent saturation runs.
    pub fn reset(&self) {
        self.concept_state.clear();
        self.visit_counter.store(1, Ordering::Relaxed);
    }

    /// Returns `true` if `concept_iri` is currently on the active DFS stack.
    pub fn is_on_stack(&self, concept_iri: &str) -> bool {
        self.concept_state
            .get(concept_iri)
            .map(|s| s.on_stack)
            .unwrap_or(false)
    }

    /// Returns the number of distinct concepts visited so far.
    pub fn visited_count(&self) -> usize {
        self.concept_state.len()
    }
}

/// Classify the type of cycle based on the IRIs in the path.
///
/// Heuristic rules:
/// - If any IRI contains a role/property indicator (`#has`, `Property`, `role`)
///   → `RoleCycle`
/// - If any IRI looks like a named individual (contains `#i_`, `Individual`,
///   `NamedIndividual`) → `NominalCycle`
/// - Otherwise → `ConceptCycle`
fn classify_cycle_type(path: &[String]) -> CycleType {
    let has_role = path.iter().any(|iri| {
        let lower = iri.to_lowercase();
        lower.contains("property") || lower.contains("#has") || lower.contains("/has") || lower.contains("role")
    });
    if has_role {
        return CycleType::RoleCycle;
    }

    let has_nominal = path.iter().any(|iri| {
        let lower = iri.to_lowercase();
        lower.contains("individual") || lower.contains("#i_") || lower.contains("nominal")
    });
    if has_nominal {
        return CycleType::NominalCycle;
    }

    CycleType::ConceptCycle
}

/// Convenience wrapper that runs cycle detection over a batch of concept IRIs
/// using the supplied `derivation_chain` as shared context. Returns all cycles
/// found.
pub fn detect_cycles_in_batch(
    detector: &CycleDetector,
    concepts: &[String],
    derivation_chain: &[String],
) -> Vec<CycleInfo> {
    concepts
        .iter()
        .filter_map(|c| detector.detect_cycle(c, derivation_chain))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cycle_in_linear_chain() {
        let detector = CycleDetector::new(64);
        let chain = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        assert!(detector.detect_cycle("D", &chain).is_none());
    }

    #[test]
    fn test_detects_direct_back_edge() {
        let detector = CycleDetector::new(64);
        let chain = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let result = detector.detect_cycle("B", &chain);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.cycle_length, 2);
        assert_eq!(info.cycle_path, vec!["B".to_string(), "C".to_string()]);
    }

    #[test]
    fn test_depth_exceeded_triggers_cycle() {
        let detector = CycleDetector::new(3);
        let chain: Vec<String> = (0..3).map(|i| format!("C{i}")).collect();
        let result = detector.detect_cycle("CX", &chain);
        assert!(result.is_some());
    }

    #[test]
    fn test_classify_concept_cycle() {
        assert_eq!(
            classify_cycle_type(&["A".to_string(), "B".to_string()]),
            CycleType::ConceptCycle
        );
    }

    #[test]
    fn test_classify_role_cycle() {
        assert_eq!(
            classify_cycle_type(&["http://example.org/hasParent".to_string(), "B".to_string()]),
            CycleType::RoleCycle
        );
    }
}
