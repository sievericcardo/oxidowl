//! Query processing module for OWL 2 DL ontologies
//!
//! This module provides comprehensive query capabilities including:
//! - DL queries with Manchester syntax
//! - Advanced conjunctive queries with SPARQL-like capabilities
//! - OWL 2 QL query rewriting and optimization
//! - High-performance query execution
//! - In-process SPARQL queries via [`SparqlStore`] (requires `sparql-store` feature)

pub mod advanced;
pub mod dl_query;

#[cfg(feature = "sparql-store")]
pub mod sparql_store;

// Re-export main query interfaces for backward compatibility
pub use dl_query::{DLQuery, DLQueryEngine, DLQueryParser, QueryError, QueryResult, QueryType};

// Export new advanced query capabilities
pub use advanced::{
    AdvancedQueryError, AdvancedQueryOptimizer, ConjunctiveQuery, ConjunctiveQueryResult,
    DLQueryFeatureExtractor, IntelligentIndexingSystem, PerformanceMonitor, PerformancePredictor,
    QueryAtom, QueryEngine, QueryOptimizer, QueryVariable,
};

// Re-export SparqlStore for consumers using the sparql-store feature
#[cfg(feature = "sparql-store")]
pub use sparql_store::SparqlStore;

use crate::ontology::Ontology;
use crate::reasoning::ReasoningService;
use std::sync::Arc;

/// Unified query service that provides DL queries, advanced conjunctive queries,
/// and (when the `sparql-store` feature is enabled) in-process SPARQL queries
/// via an embedded [`SparqlStore`].
pub struct QueryService {
    dl_engine: DLQueryEngine,
    advanced_engine: QueryEngine,
    #[cfg(feature = "sparql-store")]
    sparql_store: Option<SparqlStore>,
}

impl QueryService {
    /// Create a new query service without a pre-populated SPARQL store.
    pub fn new(
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
    ) -> Result<Self, AdvancedQueryError> {
        Ok(Self {
            dl_engine: DLQueryEngine::new(reasoning_service.clone()),
            advanced_engine: QueryEngine::new(ontology, reasoning_service.clone())?,
            #[cfg(feature = "sparql-store")]
            sparql_store: None,
        })
    }

    /// Create a new query service with an already-initialised [`SparqlStore`].
    ///
    /// This variant is used by the SMOL interpreter to serve both DL and SPARQL
    /// queries from a single service without starting an HTTP server.
    #[cfg(feature = "sparql-store")]
    pub fn with_sparql_store(
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
        sparql_store: SparqlStore,
    ) -> Result<Self, AdvancedQueryError> {
        Ok(Self {
            dl_engine: DLQueryEngine::new(reasoning_service.clone()),
            advanced_engine: QueryEngine::new(ontology, reasoning_service.clone())?,
            sparql_store: Some(sparql_store),
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

    /// Execute an in-process SPARQL SELECT query.
    ///
    /// # Errors
    /// Returns `Err` if no store is attached or if query execution fails.
    #[cfg(feature = "sparql-store")]
    pub fn execute_sparql_select(
        &self,
        query: &str,
    ) -> crate::Result<Vec<std::collections::HashMap<String, crate::semantics::RdfTerm>>> {
        self.sparql_store
            .as_ref()
            .ok_or_else(|| crate::Error::Sparql {
                message: "No SparqlStore attached to QueryService".to_string(),
            })?
            .execute_select(query)
    }

    /// Execute an in-process SPARQL ASK query.
    #[cfg(feature = "sparql-store")]
    pub fn execute_sparql_ask(&self, query: &str) -> crate::Result<bool> {
        self.sparql_store
            .as_ref()
            .ok_or_else(|| crate::Error::Sparql {
                message: "No SparqlStore attached to QueryService".to_string(),
            })?
            .execute_ask(query)
    }

    /// Execute an in-process SPARQL CONSTRUCT query.
    #[cfg(feature = "sparql-store")]
    pub fn execute_sparql_construct(
        &self,
        query: &str,
    ) -> crate::Result<Vec<crate::semantics::Triple>> {
        self.sparql_store
            .as_ref()
            .ok_or_else(|| crate::Error::Sparql {
                message: "No SparqlStore attached to QueryService".to_string(),
            })?
            .execute_construct(query)
    }

    /// Execute an in-process SPARQL UPDATE.
    #[cfg(feature = "sparql-store")]
    pub fn execute_sparql_update(&mut self, update: &str) -> crate::Result<()> {
        self.sparql_store
            .as_mut()
            .ok_or_else(|| crate::Error::Sparql {
                message: "No SparqlStore attached to QueryService".to_string(),
            })?
            .execute_update(update)
    }

    /// Attach (or replace) the in-process SPARQL store.
    #[cfg(feature = "sparql-store")]
    pub fn attach_sparql_store(&mut self, store: SparqlStore) {
        self.sparql_store = Some(store);
    }

    /// Borrow the attached SPARQL store, if any.
    #[cfg(feature = "sparql-store")]
    #[must_use]
    pub fn sparql_store(&self) -> Option<&SparqlStore> {
        self.sparql_store.as_ref()
    }

    /// Mutably borrow the attached SPARQL store, if any.
    #[cfg(feature = "sparql-store")]
    #[must_use]
    pub fn sparql_store_mut(&mut self) -> Option<&mut SparqlStore> {
        self.sparql_store.as_mut()
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
