//! Inverted indices for fast concept and role lookups
//!
//! This module provides O(1) lookups for questions like:
//! - "Which concepts reference this IRI?"
//! - "Which nodes have existential restrictions on this property?"
//! - "What are all the subclass axioms involving this concept?"

use crate::ontology::{ClassExpression, ObjectPropertyExpression};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Index for fast concept lookups by IRI
#[derive(Debug, Clone)]
pub struct ConceptIndex {
    /// Map from IRI to all concepts that reference it
    iri_to_concepts: Arc<RwLock<HashMap<String, HashSet<u64>>>>,
    
    /// Map from concept hash to the actual concept
    hash_to_concept: Arc<RwLock<HashMap<u64, ClassExpression>>>,
    
    /// Statistics
    stats: Arc<RwLock<IndexStatistics>>,
}

/// Index statistics for monitoring effectiveness
#[derive(Debug, Clone, Default)]
pub struct IndexStatistics {
    pub total_concepts: usize,
    pub total_iris: usize,
    pub total_lookups: usize,
    pub cache_hits: usize,
}

impl IndexStatistics {
    /// Get cache hit rate
    #[must_use] 
    pub fn hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_lookups as f64
        }
    }
}

impl ConceptIndex {
    /// Create a new concept index
    #[must_use] 
    pub fn new() -> Self {
        Self {
            iri_to_concepts: Arc::new(RwLock::new(HashMap::new())),
            hash_to_concept: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IndexStatistics::default())),
        }
    }

    /// Index a concept and all IRIs it references
    pub fn index_concept(&self, concept: &ClassExpression) -> crate::Result<()> {
        let hash = crate::core::fast_hashing::hash_concept(concept);
        
        // Store concept by hash
        {
            let mut hash_map = self.hash_to_concept.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex hash map lock poisoned: {e}") 
                })?;
            hash_map.insert(hash, concept.clone());
        }

        // Extract and index all IRIs referenced by this concept
        let iris = self.extract_iris(concept);
        
        {
            let mut iri_map = self.iri_to_concepts.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex IRI map lock poisoned: {e}") 
                })?;
            
            for iri in iris {
                iri_map.entry(iri).or_insert_with(HashSet::new).insert(hash);
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex stats lock poisoned: {e}") 
                })?;
            stats.total_concepts += 1;
        }

        Ok(())
    }

    /// Find all concepts that reference a given IRI (O(1) lookup)
    pub fn find_concepts_by_iri(&self, iri: &str) -> crate::Result<Vec<ClassExpression>> {
        // Update lookup statistics
        {
            let mut stats = self.stats.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex stats lock poisoned: {e}") 
                })?;
            stats.total_lookups += 1;
        }

        let iri_map = self.iri_to_concepts.read()
            .map_err(|e| crate::Error::Cache { 
                message: format!("ConceptIndex IRI map lock poisoned: {e}") 
            })?;
        
        let concept_hashes = match iri_map.get(iri) {
            Some(hashes) => hashes.clone(),
            None => return Ok(Vec::new()),
        };

        // Update cache hit statistics
        if !concept_hashes.is_empty() {
            let mut stats = self.stats.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex stats lock poisoned: {e}") 
                })?;
            stats.cache_hits += 1;
        }

        // Retrieve concepts by hash
        let hash_map = self.hash_to_concept.read()
            .map_err(|e| crate::Error::Cache { 
                message: format!("ConceptIndex hash map lock poisoned: {e}") 
            })?;
        
        Ok(concept_hashes
            .iter()
            .filter_map(|hash| hash_map.get(hash).cloned())
            .collect())
    }

    /// Extract all IRIs referenced by a concept
    fn extract_iris(&self, concept: &ClassExpression) -> Vec<String> {
        let mut iris = Vec::new();
        self.extract_iris_recursive(concept, &mut iris, 0);
        iris.sort();
        iris.dedup();
        iris
    }

    /// Maximum recursion depth for extraction to prevent stack overflow
    const MAX_EXTRACTION_DEPTH: usize = 500;

    /// Recursively extract IRIs from a concept expression
    fn extract_iris_recursive(&self, concept: &ClassExpression, iris: &mut Vec<String>, depth: usize) {
        // Prevent stack overflow on deeply nested expressions
        if depth > Self::MAX_EXTRACTION_DEPTH {
            return;
        }
        
        match concept {
            ClassExpression::Class(class) => {
                iris.push(class.iri.as_str().to_string());
            }
            ClassExpression::ObjectIntersectionOf(exprs) 
            | ClassExpression::ObjectUnionOf(exprs) => {
                for expr in exprs {
                    self.extract_iris_recursive(expr, iris, depth + 1);
                }
            }
            ClassExpression::ObjectOneOf(individuals) => {
                for ind in individuals {
                    if let crate::ontology::Individual::Named(named) = ind {
                        iris.push(named.iri.as_str().to_string());
                    }
                }
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler }
            | ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.extract_property_iris(property, iris, depth + 1);
                self.extract_iris_recursive(filler, iris, depth + 1);
            }
            ClassExpression::ObjectHasValue { property, value } => {
                self.extract_property_iris(property, iris, depth + 1);
                if let crate::ontology::Individual::Named(named) = value {
                    iris.push(named.iri.as_str().to_string());
                }
            }
            ClassExpression::ObjectHasSelf { property } => {
                self.extract_property_iris(property, iris, depth + 1);
            }
            ClassExpression::ObjectMinCardinality { property, filler, .. }
            | ClassExpression::ObjectMaxCardinality { property, filler, .. }
            | ClassExpression::ObjectExactCardinality { property, filler, .. } => {
                self.extract_property_iris(property, iris, depth + 1);
                self.extract_iris_recursive(filler, iris, depth + 1);
            }
            ClassExpression::DataSomeValuesFrom { property, .. }
            | ClassExpression::DataAllValuesFrom { property, .. }
            | ClassExpression::DataHasValue { property, .. }
            | ClassExpression::DataMinCardinality { property, .. }
            | ClassExpression::DataMaxCardinality { property, .. }
            | ClassExpression::DataExactCardinality { property, .. } => {
                // Extract data property IRI
                iris.push(property.to_string());
            }
            ClassExpression::ObjectComplementOf(expr) => {
                self.extract_iris_recursive(expr, iris, depth + 1);
            }
        }
    }

    fn extract_property_iris(&self, prop: &ObjectPropertyExpression, iris: &mut Vec<String>, depth: usize) {
        // Prevent stack overflow on deeply nested property chains
        if depth > Self::MAX_EXTRACTION_DEPTH {
            return;
        }
        
        match prop {
            ObjectPropertyExpression::ObjectProperty(p) => {
                iris.push(p.iri.as_str().to_string());
            }
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                iris.push(p.iri.as_str().to_string());
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                for p in chain {
                    self.extract_property_iris(p, iris, depth + 1);
                }
            }
        }
    }

    /// Get index statistics
    pub fn get_statistics(&self) -> crate::Result<IndexStatistics> {
        let stats = self.stats.read()
            .map_err(|e| crate::Error::Cache { 
                message: format!("ConceptIndex stats lock poisoned: {e}") 
            })?;
        
        let mut result = stats.clone();
        
        // Update total IRIs count
        let iri_map = self.iri_to_concepts.read()
            .map_err(|e| crate::Error::Cache { 
                message: format!("ConceptIndex IRI map lock poisoned: {e}") 
            })?;
        result.total_iris = iri_map.len();
        
        Ok(result)
    }
    
    /// Get the number of indexed concepts
    #[must_use] 
    pub fn size(&self) -> usize {
        self.hash_to_concept.read()
            .map(|map| map.len())
            .unwrap_or(0)
    }

    /// Clear the index
    pub fn clear(&self) -> crate::Result<()> {
        {
            let mut iri_map = self.iri_to_concepts.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex IRI map lock poisoned: {e}") 
                })?;
            iri_map.clear();
        }

        {
            let mut hash_map = self.hash_to_concept.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex hash map lock poisoned: {e}") 
                })?;
            hash_map.clear();
        }

        {
            let mut stats = self.stats.write()
                .map_err(|e| crate::Error::Cache { 
                    message: format!("ConceptIndex stats lock poisoned: {e}") 
                })?;
            *stats = IndexStatistics::default();
        }

        Ok(())
    }
}

impl Default for ConceptIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_concept_index_basic() {
        let index = ConceptIndex::new();

        let concept = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Person"),
        });

        index.index_concept(&concept).unwrap();

        // Should find concept by IRI
        let results = index.find_concepts_by_iri("http://example.org/Person").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], concept);
    }

    #[test]
    fn test_concept_index_complex() {
        let index = ConceptIndex::new();

        let person = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Person"),
        });
        let student = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Student"),
        });

        let intersection = ClassExpression::ObjectIntersectionOf(vec![
            person.clone(),
            student.clone(),
        ]);

        index.index_concept(&intersection).unwrap();

        // Should find intersection when looking up either IRI
        let results1 = index.find_concepts_by_iri("http://example.org/Person").unwrap();
        assert_eq!(results1.len(), 1);

        let results2 = index.find_concepts_by_iri("http://example.org/Student").unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_index_statistics() {
        let index = ConceptIndex::new();

        let concept = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Person"),
        });

        index.index_concept(&concept).unwrap();

        let stats = index.get_statistics().unwrap();
        assert_eq!(stats.total_concepts, 1);
        assert_eq!(stats.total_iris, 1);

        // Perform lookups
        index.find_concepts_by_iri("http://example.org/Person").unwrap();
        index.find_concepts_by_iri("http://example.org/Unknown").unwrap();

        let stats = index.get_statistics().unwrap();
        assert_eq!(stats.total_lookups, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }
}
