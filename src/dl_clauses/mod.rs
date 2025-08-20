//! DL Clause generation and representation
//! 
//! This module provides functionality for converting OWL axioms into Description Logic (DL) clauses,
//! following the style of tableau-based reasoners like HermiT, Konclude, and Pellet.

mod types;
mod generator;
mod axiom_compilers;
mod union_disjunctive;
mod helpers;
mod formatting;

// Re-export public types and functions
pub use types::{DLAtom, DLClause, DLClauseSet, DLClauseStatistics};
pub use generator::DLClauseGenerator;
