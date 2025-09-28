//! Advanced query processing module
//! 
//! This module implements high-performance conjunctive query answering with:
//! - SPARQL-like query capabilities
//! - OWL 2 QL query rewriting optimization
//! - Efficient query execution strategies

pub mod conjunctive;
pub mod rewriting;
pub mod optimization;
pub mod execution;

pub use conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
pub use rewriting::QueryRewriter;
pub use optimization::QueryOptimizer;
pub use execution::{QueryEngine, ConjunctiveQueryResult, AdvancedQueryError};