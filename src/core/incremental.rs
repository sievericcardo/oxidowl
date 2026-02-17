//! Incremental reasoning for efficient ontology updates
//!
//! This module implements incremental classification - only reclassifying concepts
//! that are affected by ontology changes.

use crate::{
    Error, Result,
    ontology::{ClassExpression, Ontology},
    core::{ConceptIndex, hash_concept},
};
use std::collections::{HashSet, HashMap};
use std::sync::{Arc, RwLock};

/// Tracks dependencies between concepts for incremental reasoning
#[derive(Debug, Clone)]
pub struct DependencyTracker {
    /// Map from concept hash to concepts that depend on it
    /// If A depends on B, then B -> {A, ...}
    dependencies: HashMap<u64, HashSet<u64>>,
    
    /// Reverse map: concept hash to concepts it depends on
    /// If A depends on B, then A -> {B, ...}
    reverse_dependencies: HashMap<u64, HashSet<u64>>,
}

impl DependencyTracker {
    /// Create a new dependency tracker
    #[must_use] 
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        }
    }
    
    /// Add a dependency: `dependent` depends on `dependency`
    pub fn add_dependency(&mut self, dependent: u64, dependency: u64) {
        // Forward: dependency -> {dependent}
        self.dependencies
            .entry(dependency)
            .or_default()
            .insert(dependent);
        
        // Reverse: dependent -> {dependency}
        self.reverse_dependencies
            .entry(dependent)
            .or_default()
            .insert(dependency);
    }
    
    /// Get all concepts that depend on the given concept
    #[must_use] 
    pub fn get_dependents(&self, concept_hash: u64) -> HashSet<u64> {
        self.dependencies
            .get(&concept_hash)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get all concepts that the given concept depends on
    #[must_use] 
    pub fn get_dependencies(&self, concept_hash: u64) -> HashSet<u64> {
        self.reverse_dependencies
            .get(&concept_hash)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Compute transitive closure of dependents
    /// Returns all concepts that transitively depend on the given concept
    #[must_use] 
    pub fn transitive_dependents(&self, concept_hash: u64) -> HashSet<u64> {
        let mut result = HashSet::new();
        let mut queue = vec![concept_hash];
        let mut visited = HashSet::new();
        
        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue; // Already visited
            }
            
            if let Some(dependents) = self.dependencies.get(&current) {
                for &dep in dependents {
                    result.insert(dep);
                    queue.push(dep);
                }
            }
        }
        
        result
    }
    
    /// Clear all dependencies
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.reverse_dependencies.clear();
    }
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages incremental classification
#[derive(Debug)]
pub struct IncrementalClassifier {
    /// Dependency tracker for concepts
    dependencies: Arc<RwLock<DependencyTracker>>,
    
    /// Concept index for IRI-based lookups
    concept_index: Arc<RwLock<ConceptIndex>>,
    
    /// Current classification results (`concept_hash` -> `super_classes`)
    classification: Arc<RwLock<HashMap<u64, HashSet<u64>>>>,
    
    /// Statistics
    stats: Arc<RwLock<IncrementalStatistics>>,
}

/// Statistics for incremental classification
#[derive(Debug, Clone, Default)]
pub struct IncrementalStatistics {
    pub total_concepts: usize,
    pub total_updates: usize,
    pub concepts_reclassified: usize,
    pub concepts_skipped: usize,
}

impl IncrementalStatistics {
    /// Get percentage of concepts that were reclassified vs skipped
    #[must_use] 
    pub fn reclassification_rate(&self) -> f64 {
        let total = self.concepts_reclassified + self.concepts_skipped;
        if total == 0 {
            0.0
        } else {
            self.concepts_reclassified as f64 / total as f64
        }
    }
}

impl IncrementalClassifier {
    /// Create a new incremental classifier
    #[must_use] 
    pub fn new() -> Self {
        Self {
            dependencies: Arc::new(RwLock::new(DependencyTracker::new())),
            concept_index: Arc::new(RwLock::new(ConceptIndex::new())),
            classification: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IncrementalStatistics::default())),
        }
    }
    
    /// Initialize with an ontology (full classification)
    pub fn initialize(&self, ontology: &Ontology) -> Result<()> {
        // Extract all concepts and build index
        let index = self.concept_index.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier index lock poisoned: {e}") 
            })?;
        
        // Index all axioms - extract concepts and index them
        use crate::ontology::Axiom;
        
        for axiom in ontology.axioms() {
            match axiom {
                // Class axioms
                Axiom::SubClassOf(ax) => {
                    index.index_concept(&ax.subclass)?;
                    index.index_concept(&ax.superclass)?;
                }
                Axiom::EquivalentClasses(ax) => {
                    for concept in &ax.classes {
                        index.index_concept(concept)?;
                    }
                }
                Axiom::DisjointClasses(ax) => {
                    for concept in &ax.classes {
                        index.index_concept(concept)?;
                    }
                }
                Axiom::DisjointUnion(ax) => {
                    index.index_concept(&ax.class)?;
                    for concept in &ax.disjoint_classes {
                        index.index_concept(concept)?;
                    }
                }
                
                // Property domain/range axioms
                Axiom::ObjectPropertyDomain(ax) => {
                    index.index_concept(&ax.domain)?;
                }
                Axiom::ObjectPropertyRange(ax) => {
                    index.index_concept(&ax.range)?;
                }
                Axiom::DataPropertyDomain(ax) => {
                    index.index_concept(&ax.domain)?;
                }
                
                // Individual axioms
                Axiom::ClassAssertion(ax) => {
                    index.index_concept(&ax.class)?;
                }
                
                // Annotation property domain
                Axiom::AnnotationPropertyDomain(ax) => {
                    index.index_concept(&ax.domain)?;
                }
                
                // Other axioms don't contain concepts
                _ => {}
            }
        }
        
        // Update stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier stats lock poisoned: {e}") 
            })?;
        stats.total_concepts = index.size();
        
        Ok(())
    }
    
    /// Add a dependency between concepts
    pub fn add_dependency(
        &self,
        dependent: &ClassExpression,
        dependency: &ClassExpression,
    ) -> Result<()> {
        let dep_hash = hash_concept(dependent);
        let dep_on_hash = hash_concept(dependency);
        
        let mut deps = self.dependencies.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier dependencies lock poisoned: {e}") 
            })?;
        
        deps.add_dependency(dep_hash, dep_on_hash);
        Ok(())
    }
    
    /// Get concepts affected by changes to the given IRIs
    pub fn get_affected_concepts(&self, changed_iris: &[String]) -> Result<HashSet<u64>> {
        let index = self.concept_index.read()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier index lock poisoned: {e}") 
            })?;
        
        let mut affected = HashSet::new();
        
        // Find all concepts that reference the changed IRIs
        for iri in changed_iris {
            if let Ok(concepts) = index.find_concepts_by_iri(iri) {
                for concept in concepts {
                    let hash = hash_concept(&concept);
                    affected.insert(hash);
                }
            }
        }
        
        // Get transitive dependents
        let deps = self.dependencies.read()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier dependencies lock poisoned: {e}") 
            })?;
        
        let mut transitive_affected = HashSet::new();
        for &hash in &affected {
            transitive_affected.insert(hash);
            transitive_affected.extend(deps.transitive_dependents(hash));
        }
        
        Ok(transitive_affected)
    }
    
    /// Update classification for a concept
    pub fn update_classification(
        &self,
        concept_hash: u64,
        super_classes: HashSet<u64>,
    ) -> Result<()> {
        let mut classification = self.classification.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier classification lock poisoned: {e}") 
            })?;
        
        classification.insert(concept_hash, super_classes);
        
        // Update stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier stats lock poisoned: {e}") 
            })?;
        stats.concepts_reclassified += 1;
        
        Ok(())
    }
    
    /// Get classification for a concept
    pub fn get_classification(&self, concept_hash: u64) -> Result<Option<HashSet<u64>>> {
        let classification = self.classification.read()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier classification lock poisoned: {e}") 
            })?;
        
        Ok(classification.get(&concept_hash).cloned())
    }
    
    /// Perform incremental update
    pub fn incremental_update(&self, changed_iris: &[String]) -> Result<HashSet<u64>> {
        // Get affected concepts
        let affected = self.get_affected_concepts(changed_iris)?;
        
        // Update stats
        let mut stats = self.stats.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier stats lock poisoned: {e}") 
            })?;
        stats.total_updates += 1;
        
        // Return concepts that need reclassification
        Ok(affected)
    }
    
    /// Mark a concept as skipped (didn't need reclassification)
    pub fn mark_skipped(&self) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier stats lock poisoned: {e}") 
            })?;
        stats.concepts_skipped += 1;
        Ok(())
    }
    
    /// Get statistics
    pub fn statistics(&self) -> Result<IncrementalStatistics> {
        let stats = self.stats.read()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier stats lock poisoned: {e}") 
            })?;
        Ok(stats.clone())
    }
    
    /// Clear all classification data
    pub fn clear(&self) -> Result<()> {
        let mut deps = self.dependencies.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier dependencies lock poisoned: {e}") 
            })?;
        deps.clear();
        
        let mut classification = self.classification.write()
            .map_err(|e| Error::Internal { 
                message: format!("IncrementalClassifier classification lock poisoned: {e}") 
            })?;
        classification.clear();
        
        Ok(())
    }
}

impl Default for IncrementalClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_dependency_tracker_basic() {
        let mut tracker = DependencyTracker::new();
        
        // A depends on B, C depends on A
        tracker.add_dependency(1, 2); // A depends on B
        tracker.add_dependency(3, 1); // C depends on A
        
        // B has A as dependent
        assert_eq!(tracker.get_dependents(2), HashSet::from([1]));
        
        // A has C as dependent
        assert_eq!(tracker.get_dependents(1), HashSet::from([3]));
        
        // A depends on B
        assert_eq!(tracker.get_dependencies(1), HashSet::from([2]));
    }

    #[test]
    fn test_transitive_dependents() {
        let mut tracker = DependencyTracker::new();
        
        // Chain: D -> C -> B -> A
        // Means: B depends on A, C depends on B, D depends on C
        tracker.add_dependency(2, 1); // B depends on A
        tracker.add_dependency(3, 2); // C depends on B
        tracker.add_dependency(4, 3); // D depends on C
        
        // Changing A should affect B, C, D
        let affected = tracker.transitive_dependents(1);
        assert_eq!(affected.len(), 3);
        assert!(affected.contains(&2)); // B
        assert!(affected.contains(&3)); // C
        assert!(affected.contains(&4)); // D
    }

    #[test]
    fn test_incremental_classifier_creation() {
        let classifier = IncrementalClassifier::new();
        let stats = classifier.statistics().unwrap();
        
        assert_eq!(stats.total_concepts, 0);
        assert_eq!(stats.total_updates, 0);
        assert_eq!(stats.concepts_reclassified, 0);
    }

    #[test]
    fn test_add_and_retrieve_dependency() {
        let classifier = IncrementalClassifier::new();
        let student = ClassExpression::Class(Class::new(IRI::new("http://example.org/Student")));
        let person = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        // Student depends on Person
        classifier.add_dependency(&student, &person).unwrap();
        
        // Verify dependency was added (indirect check via affected concepts)
        // This would need more integration to fully test
    }

    #[test]
    fn test_classification_storage() {
        let classifier = IncrementalClassifier::new();
        let person_hash = 12345u64;
        let thing_hash = 67890u64;
        
        // Store classification: Person is subclass of Thing
        let super_classes = HashSet::from([thing_hash]);
        classifier.update_classification(person_hash, super_classes.clone()).unwrap();
        
        // Retrieve it
        let retrieved = classifier.get_classification(person_hash).unwrap();
        assert_eq!(retrieved, Some(super_classes));
        
        // Stats should show 1 reclassification
        let stats = classifier.statistics().unwrap();
        assert_eq!(stats.concepts_reclassified, 1);
    }

    #[test]
    fn test_incremental_statistics() {
        let classifier = IncrementalClassifier::new();
        
        // Simulate some updates
        classifier.update_classification(1, HashSet::from([2])).unwrap();
        classifier.mark_skipped().unwrap();
        classifier.mark_skipped().unwrap();
        
        let stats = classifier.statistics().unwrap();
        assert_eq!(stats.concepts_reclassified, 1);
        assert_eq!(stats.concepts_skipped, 2);
        assert!((stats.reclassification_rate() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_clear_classifier() {
        let classifier = IncrementalClassifier::new();
        
        // Add some data
        classifier.update_classification(1, HashSet::from([2])).unwrap();
        
        // Clear
        classifier.clear().unwrap();
        
        // Should be gone
        assert!(classifier.get_classification(1).unwrap().is_none());
    }
}
