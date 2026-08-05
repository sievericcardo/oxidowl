//! Inverted indices for fast concept and role lookups
//!
//! This module provides O(1) lookups for questions like:
//! - "Which concepts reference this IRI?"
//! - "Which nodes have existential restrictions on this property?"
//! - "What are all the subclass axioms involving this concept?"
//!
//! The index uses a single `DashMap` with fine-grained shard locking instead
//! of the previous triple-`RwLock` design, eliminating sequential lock acquisition.

use crate::ontology::{ClassExpression, ObjectPropertyExpression};
#[cfg(feature = "cache")]
use dashmap::DashMap;
#[cfg(not(feature = "cache"))]
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Index for fast concept lookups by IRI.
///
/// Uses a single `DashMap` (shard-locked) instead of multiple `RwLock`-guarded
/// `HashMap`s, reducing lock contention from 3 sequential acquisitions to 1
/// shard-level lock per operation.
pub struct ConceptIndex {
    /// Map from IRI (interned `Arc<str>`) → concepts that reference that IRI.
    /// Each entry also doubles as deduplication: the u64 hash is stored inline
    /// so we avoid a separate hash→concept map.
    #[cfg(feature = "cache")]
    iri_to_concepts: DashMap<Arc<str>, Vec<(u64, ClassExpression)>>,

    /// Fallback for when the `cache` feature is disabled.
    #[cfg(not(feature = "cache"))]
    iri_to_concepts: std::sync::RwLock<HashMap<String, Vec<(u64, ClassExpression)>>>,

    /// Statistics — atomic counters avoid any lock.
    total_concepts: AtomicU64,
    total_lookups: AtomicU64,
    cache_hits: AtomicU64,
}

impl std::fmt::Debug for ConceptIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConceptIndex")
            .field(
                "total_concepts",
                &self.total_concepts.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl Clone for ConceptIndex {
    fn clone(&self) -> Self {
        #[cfg(feature = "cache")]
        {
            let new_map = DashMap::new();
            for entry in &self.iri_to_concepts {
                new_map.insert(Arc::clone(entry.key()), entry.value().clone());
            }
            Self {
                iri_to_concepts: new_map,
                total_concepts: AtomicU64::new(self.total_concepts.load(Ordering::Relaxed)),
                total_lookups: AtomicU64::new(self.total_lookups.load(Ordering::Relaxed)),
                cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            }
        }
        #[cfg(not(feature = "cache"))]
        {
            let guard = self
                .iri_to_concepts
                .read()
                .unwrap_or_else(|e| e.into_inner());
            Self {
                iri_to_concepts: std::sync::RwLock::new(guard.clone()),
                total_concepts: AtomicU64::new(self.total_concepts.load(Ordering::Relaxed)),
                total_lookups: AtomicU64::new(self.total_lookups.load(Ordering::Relaxed)),
                cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            }
        }
    }
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
            #[cfg(feature = "cache")]
            iri_to_concepts: DashMap::new(),
            #[cfg(not(feature = "cache"))]
            iri_to_concepts: std::sync::RwLock::new(HashMap::new()),
            total_concepts: AtomicU64::new(0),
            total_lookups: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
        }
    }

    /// Index a concept and all IRIs it references
    pub fn index_concept(&self, concept: &ClassExpression) -> crate::Result<()> {
        let hash = crate::core::fast_hashing::hash_concept(concept);
        let iris = self.extract_iris(concept);

        #[cfg(feature = "cache")]
        {
            for iri in iris {
                let key: Arc<str> = Arc::from(iri.as_str());
                let mut entry = self.iri_to_concepts.entry(key).or_default();
                // Dedup by hash
                if !entry.iter().any(|(h, _)| *h == hash) {
                    entry.push((hash, concept.clone()));
                }
            }
        }
        #[cfg(not(feature = "cache"))]
        {
            let mut map = self
                .iri_to_concepts
                .write()
                .map_err(|e| crate::Error::Cache {
                    message: format!("ConceptIndex lock poisoned: {e}"),
                })?;
            for iri in iris {
                let entry = map.entry(iri).or_default();
                if !entry.iter().any(|(h, _)| *h == hash) {
                    entry.push((hash, concept.clone()));
                }
            }
        }

        self.total_concepts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Find all concepts that reference a given IRI (O(1) shard lookup)
    pub fn find_concepts_by_iri(&self, iri: &str) -> crate::Result<Vec<ClassExpression>> {
        self.total_lookups.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "cache")]
        {
            if let Some(entry) = self.iri_to_concepts.get(iri) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(entry.iter().map(|(_, c)| c.clone()).collect());
            }
            Ok(Vec::new())
        }
        #[cfg(not(feature = "cache"))]
        {
            let map = self
                .iri_to_concepts
                .read()
                .map_err(|e| crate::Error::Cache {
                    message: format!("ConceptIndex lock poisoned: {e}"),
                })?;
            if let Some(entry) = map.get(iri) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(entry.iter().map(|(_, c)| c.clone()).collect());
            }
            Ok(Vec::new())
        }
    }

    /// Extract all IRIs referenced by a concept
    fn extract_iris(&self, concept: &ClassExpression) -> Vec<String> {
        let mut iris = Vec::new();
        self.extract_iris_recursive(concept, &mut iris, 0);
        iris.sort_unstable();
        iris.dedup();
        iris
    }

    const MAX_EXTRACTION_DEPTH: usize = 500;

    fn extract_iris_recursive(
        &self,
        concept: &ClassExpression,
        iris: &mut Vec<String>,
        depth: usize,
    ) {
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
            ClassExpression::ObjectMinCardinality {
                property, filler, ..
            }
            | ClassExpression::ObjectMaxCardinality {
                property, filler, ..
            }
            | ClassExpression::ObjectExactCardinality {
                property, filler, ..
            } => {
                self.extract_property_iris(property, iris, depth + 1);
                self.extract_iris_recursive(filler, iris, depth + 1);
            }
            ClassExpression::DataSomeValuesFrom { property, .. }
            | ClassExpression::DataAllValuesFrom { property, .. }
            | ClassExpression::DataHasValue { property, .. }
            | ClassExpression::DataMinCardinality { property, .. }
            | ClassExpression::DataMaxCardinality { property, .. }
            | ClassExpression::DataExactCardinality { property, .. } => {
                iris.push(property.to_string());
            }
            ClassExpression::ObjectComplementOf(expr) => {
                self.extract_iris_recursive(expr, iris, depth + 1);
            }
        }
    }

    fn extract_property_iris(
        &self,
        prop: &ObjectPropertyExpression,
        iris: &mut Vec<String>,
        depth: usize,
    ) {
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
        #[cfg(feature = "cache")]
        let total_iris = self.iri_to_concepts.len();
        #[cfg(not(feature = "cache"))]
        let total_iris = self.iri_to_concepts.read().map(|m| m.len()).unwrap_or(0);

        Ok(IndexStatistics {
            total_concepts: self.total_concepts.load(Ordering::Relaxed) as usize,
            total_iris,
            total_lookups: self.total_lookups.load(Ordering::Relaxed) as usize,
            cache_hits: self.cache_hits.load(Ordering::Relaxed) as usize,
        })
    }

    /// Get the number of indexed concepts
    #[must_use]
    pub fn size(&self) -> usize {
        self.total_concepts.load(Ordering::Relaxed) as usize
    }

    /// Clear the index
    pub fn clear(&self) -> crate::Result<()> {
        #[cfg(feature = "cache")]
        self.iri_to_concepts.clear();
        #[cfg(not(feature = "cache"))]
        self.iri_to_concepts
            .write()
            .map_err(|e| crate::Error::Cache {
                message: format!("ConceptIndex lock poisoned: {e}"),
            })?
            .clear();

        self.total_concepts.store(0, Ordering::Relaxed);
        self.total_lookups.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
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
        let results = index
            .find_concepts_by_iri("http://example.org/Person")
            .unwrap();
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
        let intersection =
            ClassExpression::ObjectIntersectionOf(vec![person.clone(), student.clone()]);
        index.index_concept(&intersection).unwrap();
        let results1 = index
            .find_concepts_by_iri("http://example.org/Person")
            .unwrap();
        assert_eq!(results1.len(), 1);
        let results2 = index
            .find_concepts_by_iri("http://example.org/Student")
            .unwrap();
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
        index
            .find_concepts_by_iri("http://example.org/Person")
            .unwrap();
        index
            .find_concepts_by_iri("http://example.org/Unknown")
            .unwrap();
        let stats = index.get_statistics().unwrap();
        assert_eq!(stats.total_lookups, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }
}
