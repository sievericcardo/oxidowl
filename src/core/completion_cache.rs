//! Completion graph caching for incremental reasoning
//!
//! This module implements caching of tableau completion graphs to avoid recomputing
//! subsumption tests.

use crate::{
    Error, Result,
    ontology::ClassExpression,
    core::hash_concept,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cached completion graph for a concept
#[derive(Debug, Clone)]
pub struct CompletionGraph {
    /// The concept this graph represents
    pub concept: ClassExpression,
    
    /// Hash of the concept for quick lookups
    pub concept_hash: u64,
    
    /// Whether the concept is satisfiable
    pub is_satisfiable: bool,
    
    /// Cached subsumption relationships (`concept_hash` -> subsumes)
    pub subsumptions: HashMap<u64, bool>,
    
    /// Generation/version for cache invalidation
    pub generation: u64,
}

impl CompletionGraph {
    /// Create a new completion graph
    #[must_use] 
    pub fn new(concept: ClassExpression, is_satisfiable: bool, generation: u64) -> Self {
        let concept_hash = hash_concept(&concept);
        Self {
            concept,
            concept_hash,
            is_satisfiable,
            subsumptions: HashMap::new(),
            generation,
        }
    }
    
    /// Check if this graph has cached subsumption for another concept
    #[must_use] 
    pub fn has_subsumption(&self, other_hash: u64) -> Option<bool> {
        self.subsumptions.get(&other_hash).copied()
    }
    
    /// Cache a subsumption result
    pub fn cache_subsumption(&mut self, other_hash: u64, subsumes: bool) {
        self.subsumptions.insert(other_hash, subsumes);
    }
}

/// Cache for completion graphs with generation-based invalidation
#[derive(Debug)]
pub struct CompletionGraphCache {
    /// Map from concept hash to completion graph
    graphs: Arc<RwLock<HashMap<u64, CompletionGraph>>>,
    
    /// Current generation for cache invalidation
    current_generation: Arc<RwLock<u64>>,
    
    /// Statistics
    stats: Arc<RwLock<CacheStatistics>>,
}

/// Statistics for completion graph cache
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    pub total_queries: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub subsumption_hits: usize,
    pub subsumption_misses: usize,
    pub invalidations: usize,
}

impl CacheStatistics {
    /// Get overall cache hit rate
    #[must_use] 
    pub fn hit_rate(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_queries as f64
        }
    }
    
    /// Get subsumption cache hit rate
    #[must_use] 
    pub fn subsumption_hit_rate(&self) -> f64 {
        let total = self.subsumption_hits + self.subsumption_misses;
        if total == 0 {
            0.0
        } else {
            self.subsumption_hits as f64 / total as f64
        }
    }
}

impl CompletionGraphCache {
    /// Create a new completion graph cache
    #[must_use] 
    pub fn new() -> Self {
        Self {
            graphs: Arc::new(RwLock::new(HashMap::new())),
            current_generation: Arc::new(RwLock::new(0)),
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
        }
    }
    
    /// Get a cached completion graph
    pub fn get(&self, concept: &ClassExpression) -> Result<Option<CompletionGraph>> {
        let concept_hash = hash_concept(concept);
        
        // Update stats
        {
            let mut stats = self.stats.write()
                .map_err(|e| Error::Cache { 
                    message: format!("CompletionGraphCache stats lock poisoned: {e}") 
                })?;
            stats.total_queries += 1;
        }
        
        let graphs = self.graphs.read()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache read lock poisoned: {e}") 
            })?;
        
        if let Some(graph) = graphs.get(&concept_hash) {
            // Check if graph is still valid (generation matches)
            let current_gen = *self.current_generation.read()
                .map_err(|e| Error::Cache { 
                    message: format!("CompletionGraphCache generation lock poisoned: {e}") 
                })?;
            
            if graph.generation == current_gen {
                // Update hit stats
                let mut stats = self.stats.write()
                    .map_err(|e| Error::Cache { 
                        message: format!("CompletionGraphCache stats lock poisoned: {e}") 
                    })?;
                stats.cache_hits += 1;
                
                return Ok(Some(graph.clone()));
            }
        }
        
        // Update miss stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache stats lock poisoned: {e}") 
            })?;
        stats.cache_misses += 1;
        
        Ok(None)
    }
    
    /// Store a completion graph
    pub fn put(&self, graph: CompletionGraph) -> Result<()> {
        let mut graphs = self.graphs.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache write lock poisoned: {e}") 
            })?;
        
        graphs.insert(graph.concept_hash, graph);
        Ok(())
    }
    
    /// Check cached subsumption (concept1 subsumes concept2)
    pub fn check_subsumption(
        &self,
        concept1: &ClassExpression,
        concept2: &ClassExpression,
    ) -> Result<Option<bool>> {
        let hash1 = hash_concept(concept1);
        let hash2 = hash_concept(concept2);
        
        let graphs = self.graphs.read()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache read lock poisoned: {e}") 
            })?;
        
        if let Some(graph) = graphs.get(&hash1) {
            // Check generation
            let current_gen = *self.current_generation.read()
                .map_err(|e| Error::Cache { 
                    message: format!("CompletionGraphCache generation lock poisoned: {e}") 
                })?;
            
            if graph.generation == current_gen
                && let Some(result) = graph.has_subsumption(hash2) {
                    // Update subsumption hit stats
                    let mut stats = self.stats.write()
                        .map_err(|e| Error::Cache { 
                            message: format!("CompletionGraphCache stats lock poisoned: {e}") 
                        })?;
                    stats.subsumption_hits += 1;
                    
                    return Ok(Some(result));
                }
        }
        
        // Update subsumption miss stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache stats lock poisoned: {e}") 
            })?;
        stats.subsumption_misses += 1;
        
        Ok(None)
    }
    
    /// Cache a subsumption result
    pub fn cache_subsumption(
        &self,
        concept1: &ClassExpression,
        concept2: &ClassExpression,
        subsumes: bool,
    ) -> Result<()> {
        let hash1 = hash_concept(concept1);
        let hash2 = hash_concept(concept2);
        
        let mut graphs = self.graphs.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache write lock poisoned: {e}") 
            })?;
        
        if let Some(graph) = graphs.get_mut(&hash1) {
            graph.cache_subsumption(hash2, subsumes);
        }
        
        Ok(())
    }
    
    /// Invalidate cache (increment generation)
    pub fn invalidate(&self) -> Result<()> {
        let mut generation_guard = self.current_generation.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache generation lock poisoned: {e}") 
            })?;
        *generation_guard += 1;
        
        // Update invalidation stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache stats lock poisoned: {e}") 
            })?;
        stats.invalidations += 1;
        
        Ok(())
    }
    
    /// Selective invalidation - only invalidate affected concepts
    pub fn invalidate_concepts(
        &self,
        affected_concepts: &[u64],
    ) -> Result<()> {
        let mut graphs = self.graphs.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache write lock poisoned: {e}") 
            })?;
        
        for hash in affected_concepts {
            graphs.remove(hash);
        }
        
        // Update invalidation stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache stats lock poisoned: {e}") 
            })?;
        stats.invalidations += 1;
        
        Ok(())
    }
    
    /// Get current generation
    pub fn current_generation(&self) -> Result<u64> {
        let generation_guard = self.current_generation.read()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache generation lock poisoned: {e}") 
            })?;
        Ok(*generation_guard)
    }
    
    /// Get cache statistics
    pub fn statistics(&self) -> Result<CacheStatistics> {
        let stats = self.stats.read()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache stats lock poisoned: {e}") 
            })?;
        Ok(stats.clone())
    }
    
    /// Clear all cached graphs
    pub fn clear(&self) -> Result<()> {
        let mut graphs = self.graphs.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache write lock poisoned: {e}") 
            })?;
        graphs.clear();
        
        let mut generation_guard = self.current_generation.write()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache generation lock poisoned: {e}") 
            })?;
        *generation_guard += 1;
        
        Ok(())
    }
    
    /// Get cache size
    pub fn size(&self) -> Result<usize> {
        let graphs = self.graphs.read()
            .map_err(|e| Error::Cache { 
                message: format!("CompletionGraphCache read lock poisoned: {e}") 
            })?;
        Ok(graphs.len())
    }
}

impl Default for CompletionGraphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_completion_graph_creation() {
        let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let graph = CompletionGraph::new(concept, true, 0);
        
        assert!(graph.is_satisfiable);
        assert_eq!(graph.generation, 0);
        assert_eq!(graph.subsumptions.len(), 0);
    }

    #[test]
    fn test_cache_basic_operations() {
        let cache = CompletionGraphCache::new();
        let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        // Initially empty
        assert!(cache.get(&concept).unwrap().is_none());
        
        // Store graph
        let graph = CompletionGraph::new(concept.clone(), true, 0);
        cache.put(graph).unwrap();
        
        // Should retrieve it
        let retrieved = cache.get(&concept).unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_satisfiable);
    }

    #[test]
    fn test_generation_invalidation() {
        let cache = CompletionGraphCache::new();
        let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        // Store graph at generation 0
        let graph = CompletionGraph::new(concept.clone(), true, 0);
        cache.put(graph).unwrap();
        
        // Should retrieve it
        assert!(cache.get(&concept).unwrap().is_some());
        
        // Invalidate cache (increment generation)
        cache.invalidate().unwrap();
        
        // Should no longer retrieve it (generation mismatch)
        assert!(cache.get(&concept).unwrap().is_none());
    }

    #[test]
    fn test_subsumption_caching() {
        let cache = CompletionGraphCache::new();
        let person = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let student = ClassExpression::Class(Class::new(IRI::new("http://example.org/Student")));
        
        // Store graph for person
        let graph = CompletionGraph::new(person.clone(), true, 0);
        cache.put(graph).unwrap();
        
        // Initially no subsumption cached
        assert!(cache.check_subsumption(&person, &student).unwrap().is_none());
        
        // Cache subsumption
        cache.cache_subsumption(&person, &student, true).unwrap();
        
        // Should retrieve cached result
        assert_eq!(cache.check_subsumption(&person, &student).unwrap(), Some(true));
    }

    #[test]
    fn test_statistics() {
        let cache = CompletionGraphCache::new();
        let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        // Perform operations
        cache.get(&concept).unwrap(); // Miss
        let graph = CompletionGraph::new(concept.clone(), true, 0);
        cache.put(graph).unwrap();
        cache.get(&concept).unwrap(); // Hit
        cache.invalidate().unwrap();
        
        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_queries, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.invalidations, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_selective_invalidation() {
        let cache = CompletionGraphCache::new();
        let person = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let student = ClassExpression::Class(Class::new(IRI::new("http://example.org/Student")));
        
        // Store both graphs
        let graph1 = CompletionGraph::new(person.clone(), true, 0);
        let graph2 = CompletionGraph::new(student.clone(), true, 0);
        let person_hash = graph1.concept_hash;
        cache.put(graph1).unwrap();
        cache.put(graph2).unwrap();
        
        // Both should be retrievable
        assert!(cache.get(&person).unwrap().is_some());
        assert!(cache.get(&student).unwrap().is_some());
        
        // Selectively invalidate only person
        cache.invalidate_concepts(&[person_hash]).unwrap();
        
        // Person should be gone, student should remain
        assert!(cache.get(&person).unwrap().is_none());
        assert!(cache.get(&student).unwrap().is_some());
    }
}
