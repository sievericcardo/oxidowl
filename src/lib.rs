//! Oxidowl: A Rust OWL 2 DL reasoner
//!
//! This crate provides a complete Description Logic reasoner for SROIQV(D),
//! supporting nearly all features of OWL 2 DL. It maintains the architecture
//! and behavior of the original C++ implementation while leveraging Rust's
//! memory safety and performance characteristics.
//!
//! # Main Components
//!
//! - [`core`] - Core reasoning engine with tableau algorithms
//! - [`ontology`] - Ontology representation and management
//! - [`parsers`] - Input format parsers (OWL XML, Functional, RDF)
//! - [`reasoning`] - High-level reasoning tasks and coordination
//! - [`network`] - HTTP servers for OWLlink and SPARQL
//! - [`config`] - Configuration management
//!
//! # Example Usage
//!
//! ```rust
//! use oxidowl::{Reasoner, ReasonerConfig, OntologyFormat};
//!
//! // Create a reasoner with default configuration
//! let config = ReasonerConfig::default();
//! let mut reasoner = Reasoner::new(config)?;
//!
//! // Load an ontology
//! reasoner.load_ontology_from_file("example.owl", OntologyFormat::OwlXml)?;
//!
//! // Check consistency
//! let is_consistent = reasoner.is_consistent()?;
//! println!("Ontology is consistent: {}", is_consistent);
//!
//! // Perform classification
//! let class_hierarchy = reasoner.classify()?;
//! 
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cache;
pub mod config;
pub mod core;
pub mod error;
pub mod network;
pub mod ontology;
pub mod parsers;
pub mod reasoning;
pub mod utils;

// Re-export main types for convenience
pub use crate::core::reasoner::{Reasoner};
pub use crate::config::{ReasonerConfig, TableauAlgorithm};
pub use crate::error::{Error, Result};
pub use crate::ontology::{Ontology, OntologyFormat};
pub use crate::core::reasoner::{ReasoningTask, ReasoningResult};

/// Version information matching the original Konclude
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "Konclude-rs";
pub const DESCRIPTION: &str = "Rust port of the Konclude OWL 2 DL reasoner";

/// Build information
pub fn version_info() -> String {
    format!(
        "{} - {}, Version {} (Rust port)",
        NAME,
        DESCRIPTION,
        VERSION
    )
}

/// Get supported description logic expressivities
pub fn supported_expressivities() -> Vec<&'static str> {
    vec![
        "ALC", "ALCH", "ALCHI", "ALCHIQ", "ALCHIF", "ALCHIQ", 
        "SHIQ", "SHIF", "SHIN", "SHOIN", "SROIQ", "SROIQV"
    ]
}

/// Check if a specific expressivity is supported
pub fn supports_expressivity(expressivity: &str) -> bool {
    supported_expressivities().contains(&expressivity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains("Konclude-rs"));
        assert!(info.contains(VERSION));
    }

    #[test]
    fn test_supported_expressivities() {
        let expressivities = supported_expressivities();
        assert!(expressivities.contains(&"SROIQ"));
        assert!(expressivities.contains(&"ALC"));
    }

    #[test]
    fn test_supports_expressivity() {
        assert!(supports_expressivity("SROIQ"));
        assert!(supports_expressivity("ALC"));
        assert!(!supports_expressivity("UNKNOWN"));
    }
}
