//! DL Clause generation and representation
//!
//! This module provides functionality for converting OWL axioms into Description Logic (DL) clauses,
//! following the style of tableau-based reasoners like HermiT, Konclude, and Pellet.

mod axiom_compilers;
mod formatting;
mod generator;
mod helpers;
mod types;
mod union_disjunctive;

#[cfg(test)]
mod enhanced_test;

// Re-export public types and functions
pub use generator::DLClauseGenerator;
pub use types::{DLAtom, DLClause, DLClauseSet, DLClauseStatistics, Individual, Variable};
