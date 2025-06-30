//! Error types for the Oxidowl reasoner
//!
//! This module defines all error types that can occur during reasoning operations,
//! parsing, network operations, and configuration.

use std::fmt;

/// Main error type for Oxidowl
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Ontology parsing error
    #[error("Ontology parsing error: {message}")]
    OntologyParsing { message: String },

    /// Reasoning error during tableau processing
    #[error("Reasoning error: {message}")]
    Reasoning { message: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Network error
    #[error("Network error: {message}")]
    Network { message: String },

    /// File I/O error
    #[error("File I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// XML parsing error
    #[error("XML parsing error: {message}")]
    XmlParsing { message: String },

    /// Sparql error
    #[error("SPARQL error: {message}")]
    Sparql { message: String },

    /// Cache error
    #[error("Cache error: {message}")]
    Cache { message: String },

    /// Memory/resource limit exceeded
    #[error("Resource exhaustion: {message}")]
    ResourceExhaustion { message: String },

    /// Timeout error
    #[error("Timeout error: {message}")]
    Timeout { message: String },

    /// Unsupported operation
    #[error("Unsupported operation: {message}")]
    Unsupported { message: String },

    /// Internal logic error
    #[error("Internal logic error: {message}")]
    Internal { message: String },
}

/// Specialized error for reasoner operations
pub type Result<T> = std::result::Result<T, Error>;
