//! Entailment Relations Implementation
//!
//! This module implements various entailment relations for RDF, RDFS, and OWL
//! according to the W3C specifications. The module is split into focused components:
//!
//! - [`checker`] - Main entailment checker with support for different regimes  
//! - [`owl2_rl`] - OWL 2 RL rule engine implementation

pub mod checker;
pub mod owl2_rl;

// Re-export main types for convenience
pub use checker::{EntailmentChecker, EntailmentRegime};
pub use owl2_rl::Owl2RlEngine;

// Include tests when testing
#[cfg(test)]
mod tests {
    include!("tests.rs");
}
