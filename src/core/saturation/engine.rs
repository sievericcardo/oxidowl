//! Core saturation engine implementation

use crate::{
    Error, Result,
    ontology::{ClassExpression, Ontology},
};
use super::{
    config::SaturationConfig,
    node::{SaturationNode, SaturationStatus},
    rules::SaturationRuleSet,
};
use crate::core::persistent_collections::ConceptSet;
use log::{debug, info, warn};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Result of saturation operation
#[derive(Debug, Clone)]
pub struct SaturationResult {
    /// Map from concepts to their saturation nodes
    pub nodes: HashMap<ClassExpression, SaturationNode>,

    /// Statistics about the saturation process
    pub statistics: SaturationStatistics,

    /// Computed subsumption relationships
    pub subsumptions: HashMap<ClassExpression, ConceptSet>,
}

/// Statistics collected during saturation
#[derive(Debug, Clone, Default)]
pub struct SaturationStatistics {
    /// Number of concepts processed
    pub concepts_processed: usize,

    /// Number of concepts fully saturated
    pub concepts_complete: usize,

    /// Number of concepts requiring tableau
    pub concepts_requiring_tableau: usize,

    /// Total number of rule applications
    pub rule_applications: usize,

    /// Total saturation time
    pub saturation_time: std::time::Duration,

    /// Number of iterations performed
    pub iterations: usize,

    /// Number of concepts marked as inconsistent
    pub inconsistent_concepts: usize,
}

impl SaturationResult {
    /// Create a new saturation result
    pub fn new(nodes: HashMap<ClassExpression, SaturationNode>) -> Self {
        let mut statistics = SaturationStatistics::default();
        let mut subsumptions = HashMap::new();

        for (concept, node) in &nodes {
            statistics.concepts_processed += 1;

            if node.is_complete() {
                statistics.concepts_complete += 1;
            }

            if node.requires_full_tableau() {
                statistics.concepts_requiring_tableau += 1;
            }

            if node.is_inconsistent {
                statistics.inconsistent_concepts += 1;
            }

            // Build subsumption map
            subsumptions.insert(concept.clone(), node.all_subsumers.clone());
        }

        Self {
            nodes,
            statistics,
            subsumptions,
        }
    }

    /// Get the saturation node for a concept
    pub fn get_node(&self, concept: &ClassExpression) -> Option<&SaturationNode> {
        self.nodes.get(concept)
    }

    /// Get direct subsumers for a concept
    pub fn get_direct_subsumers(&self, concept: &ClassExpression) -> ConceptSet {
        self.nodes
            .get(concept)
            .map(|node| node.direct_subsumers.clone())
            .unwrap_or_default()
    }

    /// Check if one concept subsumes another based on saturation
    pub fn subsumes(&self, subsumer: &ClassExpression, subsumed: &ClassExpression) -> bool {
        if let Some(node) = self.nodes.get(subsumed) {
            node.all_subsumers.contains(subsumer) || node.saturated_concepts.contains(subsumer)
        } else {
            false
        }
    }
}

/// Main saturation engine
#[derive(Debug)]
pub struct SaturationEngine {
    /// Configuration for saturation
    config: SaturationConfig,

    /// Rule set for saturation
    rule_set: Arc<SaturationRuleSet>,

    /// Cache of saturation results
    cache: Arc<RwLock<HashMap<ClassExpression, SaturationNode>>>,
}

impl SaturationEngine {
    /// Create a new saturation engine
    pub fn new(config: SaturationConfig) -> Self {
        Self {
            config,
            rule_set: Arc::new(SaturationRuleSet::new_owl2_dl()),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create an engine with default configuration
    pub fn default() -> Self {
        Self::new(SaturationConfig::default())
    }

    /// Saturate an entire ontology
    pub fn saturate_ontology(&self, ontology: &Ontology) -> Result<SaturationResult> {
        let start_time = Instant::now();
        info!("Starting ontology saturation");

        // Extract all named classes
        let signature = ontology.signature()?;
        let concepts: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        info!("Saturating {} concepts", concepts.len());

        // Saturate each concept
        let nodes = if self.config.enable_parallel && concepts.len() > 10 {
            #[cfg(feature = "parallel")]
            {
                self.saturate_concepts_parallel(&concepts, ontology)?
            }
            #[cfg(not(feature = "parallel"))]
            {
                self.saturate_concepts_sequential(&concepts, ontology)?
            }
        } else {
            self.saturate_concepts_sequential(&concepts, ontology)?
        };

        // Compute transitive closure of subsumers
        let nodes = self.compute_transitive_subsumers(nodes);

        let saturation_time = start_time.elapsed();
        info!("Saturation completed in {:?}", saturation_time);

        let mut result = SaturationResult::new(nodes);
        result.statistics.saturation_time = saturation_time;

        Ok(result)
    }

    /// Saturate a single concept
    pub fn saturate_concept(
        &self,
        concept: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<SaturationNode> {
        // Check cache first
        if self.config.enable_caching {
            if let Ok(cache) = self.cache.read() {
                if let Some(cached_node) = cache.get(concept) {
                    debug!("Found cached saturation for {:?}", concept);
                    return Ok(cached_node.clone());
                }
            }
        }

        let mut node = SaturationNode::new(concept.clone());
        let mut iteration = 0;

        debug!("Saturating concept: {:?}", concept);

        loop {
            iteration += 1;
            node.iteration_count = iteration;

            if iteration > self.config.max_iterations {
                warn!(
                    "Reached max iterations ({}) for concept {:?}",
                    self.config.max_iterations, concept
                );
                node.status = SaturationStatus::Partial;
                break;
            }

            // Apply all saturation rules
            let changed = self.rule_set.apply_all(&mut node, ontology)?;

            // Check for inconsistency
            if self.check_inconsistency(&node) {
                node.mark_inconsistent();
                debug!("Concept {:?} is inconsistent", concept);
                break;
            }

            // Update status based on branch count
            node.update_status(self.config.max_branches);

            // If no changes or marked as requiring tableau, stop
            if !changed || node.requires_full_tableau() {
                if node.status == SaturationStatus::InProgress
                    || node.status == SaturationStatus::Unprocessed
                {
                    node.status = SaturationStatus::Complete;
                }
                break;
            }
        }

        debug!(
            "Saturated {:?} in {} iterations, status: {:?}",
            concept, iteration, node.status
        );

        // Cache the result
        if self.config.enable_caching {
            if let Ok(mut cache) = self.cache.write() {
                cache.insert(concept.clone(), node.clone());
            }
        }

        Ok(node)
    }

    /// Saturate multiple concepts sequentially
    fn saturate_concepts_sequential(
        &self,
        concepts: &[ClassExpression],
        ontology: &Ontology,
    ) -> Result<HashMap<ClassExpression, SaturationNode>> {
        let mut nodes = HashMap::new();

        for (i, concept) in concepts.iter().enumerate() {
            if i % 100 == 0 {
                debug!("Saturation progress: {}/{}", i, concepts.len());
            }

            let node = self.saturate_concept(concept, ontology)?;
            nodes.insert(concept.clone(), node);
        }

        Ok(nodes)
    }

    /// Saturate multiple concepts in parallel
    #[cfg(feature = "parallel")]
    fn saturate_concepts_parallel(
        &self,
        concepts: &[ClassExpression],
        ontology: &Ontology,
    ) -> Result<HashMap<ClassExpression, SaturationNode>> {
        use std::sync::Mutex;

        let nodes = Arc::new(Mutex::new(HashMap::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        concepts.par_iter().enumerate().for_each(|(i, concept)| {
            if i % 100 == 0 {
                debug!("Saturation progress: {}/{}", i, concepts.len());
            }

            match self.saturate_concept(concept, ontology) {
                Ok(node) => {
                    if let Ok(mut nodes_lock) = nodes.lock() {
                        nodes_lock.insert(concept.clone(), node);
                    }
                }
                Err(e) => {
                    if let Ok(mut errors_lock) = errors.lock() {
                        errors_lock.push(e);
                    }
                }
            }
        });

        // Check for errors
        if let Ok(errors_lock) = errors.lock() {
            if !errors_lock.is_empty() {
                return Err(errors_lock[0].clone());
            }
        }

        // Extract results
        let nodes = Arc::try_unwrap(nodes)
            .map_err(|_| Error::reasoning("Failed to unwrap nodes Arc"))?
            .into_inner()
            .map_err(|_| Error::reasoning("Failed to acquire nodes lock"))?;

        Ok(nodes)
    }

    /// Compute transitive closure of subsumers
    fn compute_transitive_subsumers(
        &self,
        mut nodes: HashMap<ClassExpression, SaturationNode>,
    ) -> HashMap<ClassExpression, SaturationNode> {
        debug!("Computing transitive closure of subsumers");

        // Build adjacency list
        let mut subsumption_graph: HashMap<ClassExpression, ConceptSet> =
            HashMap::new();

        for (concept, node) in &nodes {
            subsumption_graph.insert(concept.clone(), node.direct_subsumers.clone());
        }

        // Compute transitive closure using Warshall's algorithm
        let concepts: Vec<_> = nodes.keys().cloned().collect();

        for concept in &concepts {
            if let Some(node) = nodes.get_mut(concept) {
                let mut all_subsumers = node.direct_subsumers.clone();
                let mut to_process: Vec<_> = node.direct_subsumers.iter().cloned().collect();
                let mut visited = ConceptSet::new();

                while let Some(subsumer) = to_process.pop() {
                    if visited.contains(&subsumer) {
                        continue;
                    }
                    visited = visited.update(subsumer.clone());

                    if let Some(indirect_subsumers) = subsumption_graph.get(&subsumer) {
                        for indirect in indirect_subsumers {
                            if !all_subsumers.contains(indirect) {
                                all_subsumers = all_subsumers.update(indirect.clone());
                                to_process.push(indirect.clone());
                            }
                        }
                    }
                }

                node.all_subsumers = all_subsumers;
            }
        }

        nodes
    }

    /// Check if a node represents an inconsistent concept
    fn check_inconsistency(&self, node: &SaturationNode) -> bool {
        // Check for Nothing (bottom concept)
        if node.saturated_concepts.iter().any(|c| {
            matches!(c, ClassExpression::Class(cls) if cls.iri.to_string().ends_with("Nothing"))
        }) {
            return true;
        }

        // Check for complementary concepts
        for concept in &node.saturated_concepts {
            if let ClassExpression::ObjectComplementOf(complement) = concept {
                if node.saturated_concepts.contains(complement.as_ref()) {
                    return true;
                }
            }
        }

        false
    }

    /// Get direct subsumers for a concept
    pub fn get_direct_subsumers(
        &self,
        concept: &ClassExpression,
        result: &SaturationResult,
    ) -> ConceptSet {
        result.get_direct_subsumers(concept)
    }

    /// Update saturation incrementally based on a set of changed concepts
    pub fn update_incremental(
        &self,
        changed_concepts: &ConceptSet,
        ontology: &Ontology,
        previous_result: &SaturationResult,
    ) -> Result<SaturationResult> {
        info!(
            "Performing incremental saturation update for {} changed concepts",
            changed_concepts.len()
        );

        // Start with previous nodes
        let mut nodes = previous_result.nodes.clone();

        // Compute affected concepts (transitive dependencies)
        let affected = self.compute_affected_concepts(changed_concepts, &nodes);

        info!("Re-saturating {} affected concepts", affected.len());

        // Re-saturate affected concepts
        for concept in &affected {
            let node = self.saturate_concept(concept, ontology)?;
            nodes.insert(concept.clone(), node);
        }

        // Recompute transitive closure
        let nodes = self.compute_transitive_subsumers(nodes);

        Ok(SaturationResult::new(nodes))
    }

    /// Compute concepts affected by changes (transitively)
    fn compute_affected_concepts(
        &self,
        changed: &ConceptSet,
        nodes: &HashMap<ClassExpression, SaturationNode>,
    ) -> ConceptSet {
        let mut affected = changed.clone();
        let mut to_process: Vec<_> = changed.iter().cloned().collect();

        while let Some(concept) = to_process.pop() {
            // Find all concepts that depend on this concept
            for (other_concept, node) in nodes {
                if node.saturated_concepts.contains(&concept)
                    || node.direct_subsumers.contains(&concept)
                {
                    if !affected.contains(other_concept) {
                        affected = affected.update(other_concept.clone());
                        to_process.push(other_concept.clone());
                    }
                }
            }
        }

        affected
    }

    /// Clear the saturation cache
    pub fn clear_cache(&self) -> Result<()> {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        Ok(())
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }
}

impl Default for SaturationEngine {
    fn default() -> Self {
        Self::new(SaturationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturation_engine_creation() {
        let engine = SaturationEngine::default();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_saturation_config() {
        let config = SaturationConfig::default()
            .with_max_branches(10)
            .with_aggressive_saturation(true);

        assert_eq!(config.max_branches, 10);
        assert!(config.aggressive_saturation);
    }
}
