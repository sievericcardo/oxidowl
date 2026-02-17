//! Query processing module for OWL 2 DL ontologies
//!
//! This module provides comprehensive query capabilities including:
//! - DL queries with Manchester syntax
//! - Advanced conjunctive queries with SPARQL-like capabilities  
//! - OWL 2 QL query rewriting and optimization
//! - High-performance query execution

pub mod advanced;
pub mod dl_query;

// Re-export main query interfaces for backward compatibility
pub use dl_query::{DLQuery, DLQueryEngine, DLQueryParser, QueryError, QueryResult, QueryType};

// Export new advanced query capabilities
pub use advanced::{
    AdvancedQueryError, AdvancedQueryOptimizer, ConjunctiveQuery, ConjunctiveQueryResult,
    DLQueryFeatureExtractor, IntelligentIndexingSystem, PerformanceMonitor, PerformancePredictor,
    QueryAtom, QueryEngine, QueryOptimizer, QueryVariable,
};

use crate::ontology::Ontology;
use crate::reasoning::ReasoningService;
use std::sync::Arc;

/// Unified query service that provides both DL queries and advanced conjunctive queries
pub struct QueryService {
    dl_engine: DLQueryEngine,
    advanced_engine: QueryEngine,
}

impl QueryService {
    /// Create a new query service
    pub fn new(
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
    ) -> Result<Self, AdvancedQueryError> {
        Ok(Self {
            dl_engine: DLQueryEngine::new(reasoning_service.clone()),
            advanced_engine: QueryEngine::new(ontology, reasoning_service.clone())?,
        })
    }

    /// Execute a DL query with Manchester syntax
    pub async fn execute_dl_query(
        &mut self,
        query: &str,
    ) -> std::result::Result<QueryResult, crate::Error> {
        self.dl_engine.execute_query(query).await
    }

    /// Execute an advanced conjunctive query
    pub fn execute_conjunctive_query(
        &mut self,
        query: &ConjunctiveQuery,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        self.advanced_engine.execute_query(query)
    }

    /// Get the DL query engine for direct access
    #[must_use]
    pub fn dl_engine(&self) -> &DLQueryEngine {
        &self.dl_engine
    }

    /// Get the advanced query engine for direct access
    #[must_use]
    pub fn advanced_engine(&self) -> &QueryEngine {
        &self.advanced_engine
    }
}
