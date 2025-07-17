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

    /// Invalid input errors
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Invalid disjunct index
    #[error("Invalid disjunct index: {index}")]
    InvalidDisjunctIndex { index: usize },

    /// Invalid branching choice
    #[error("Invalid branching choice: {index}")]
    InvalidBranchingChoice { index: usize },

    /// Maximum depth exceeded
    #[error("Maximum depth exceeded: {depth}")]
    MaxDepthExceeded { depth: usize },

    /// Branching point not found
    #[error("Branching point not found: {id}")]
    BranchingPointNotFound { id: String },

    /// No branching choices available
    #[error("No branching choices available")]
    NoBranchingChoicesAvailable,

    /// Resource exhausted (alternative form)
    #[error("Resource exhausted: {message}")]
    ResourceExhausted { message: String },

    /// Invalid property chain
    #[error("Invalid property chain: {message}")]
    InvalidPropertyChain { message: String },

    /// Invalid assertion
    #[error("Invalid assertion: {message}")]
    InvalidAssertion { message: String },
}

/// Specialized error for reasoner operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Ontology parsing error constructor
    pub fn ontology_parsing<S: Into<String>>(message: S) -> Self {
        Self::OntologyParsing {
            message: message.into(),
        }
    }

    /// Reasoning error constructor
    pub fn reasoning<S: Into<String>>(message: S) -> Self {
        Self::Reasoning {
            message: message.into(),
        }
    }

    /// Configuration error constructor
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Network error constructor
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// File I/O error constructor
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io {
            source: std::io::Error::new(std::io::ErrorKind::Other, message.into()),
        }
    }

    /// XML parsing error constructor
    pub fn xml_parsing<S: Into<String>>(message: S) -> Self {
        Self::XmlParsing {
            message: message.into(),
        }
    }

    /// SPARQL error constructor
    pub fn sparql<S: Into<String>>(message: S) -> Self {
        Self::Sparql {
            message: message.into(),
        }
    }

    /// Cache error constructor
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Resource exhaustion error constructor
    pub fn resource_exhaustion<S: Into<String>>(message: S) -> Self {
        Self::ResourceExhaustion {
            message: message.into(),
        }
    }

    /// Timeout error constructor
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    /// Unsupported operation error constructor
    pub fn unsupported<S: Into<String>>(message: S) -> Self {
        Self::Unsupported {
            message: message.into(),
        }
    }

    /// Internal logic error constructor
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input<S: Into<String>>(message: S) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    /// Create an invalid disjunct index error
    pub fn invalid_disjunct_index(index: usize) -> Self {
        Self::InvalidDisjunctIndex { index }
    }

    /// Create an invalid branching choice error
    pub fn invalid_branching_choice(index: usize) -> Self {
        Self::InvalidBranchingChoice { index }
    }

    /// Create a max depth exceeded error
    pub fn max_depth_exceeded(depth: usize) -> Self {
        Self::MaxDepthExceeded { depth }
    }

    /// Create a branching point not found error
    pub fn branching_point_not_found<S: Into<String>>(id: S) -> Self {
        Self::BranchingPointNotFound { id: id.into() }
    }

    /// Create a no branching choices available error
    pub fn no_branching_choices_available() -> Self {
        Self::NoBranchingChoicesAvailable
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted<S: Into<String>>(message: S) -> Self {
        Self::ResourceExhausted {
            message: message.into(),
        }
    }
}

/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Input/parsing errors
    Input,
    /// Reasoning errors
    Reasoning,
    /// Configuration errors
    Config,
    /// Network errors
    Network,
    /// Resource errors
    Resource,
    /// Internal logic errors
    Internal,
}

impl Error {
    /// Get the category of the error
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::OntologyParsing { .. } 
                | Error::XmlParsing { .. } 
                | Error::Io { .. } => ErrorCategory::Input,
            Error::Reasoning { .. }
                | Error::Sparql { .. } => ErrorCategory::Reasoning,
            Error::Config { .. } => ErrorCategory::Config,
            Error::Network { .. } => ErrorCategory::Network,
            Error::Cache { .. }
                | Error::ResourceExhaustion { .. }
                | Error::ResourceExhausted { .. }
                | Error::Timeout { .. } => ErrorCategory::Resource,
            Error::Unsupported { .. }
                | Error::Internal { .. } => ErrorCategory::Internal,
            Error::InvalidInput { .. }
                | Error::InvalidDisjunctIndex { .. }
                | Error::InvalidBranchingChoice { .. }
                | Error::MaxDepthExceeded { .. }
                | Error::BranchingPointNotFound { .. }
                | Error::NoBranchingChoicesAvailable 
                | Error::InvalidPropertyChain { .. }
                | Error::InvalidAssertion { .. } => ErrorCategory::Internal,
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Error::OntologyParsing { .. }
            | Error::XmlParsing { .. }
            | Error::Config { .. }
            | Error::Sparql { .. }
            | Error::Unsupported { .. } => false,
            Error::Reasoning { .. }
            | Error::Cache { .. }
            | Error::ResourceExhaustion { .. }
            | Error::ResourceExhausted { .. }
            | Error::Timeout { .. }
            | Error::Network { .. } => true,
            Error::Io { .. } 
            | Error::Internal { .. } => false,
            Error::InvalidInput { .. }
            | Error::InvalidDisjunctIndex { .. }
            | Error::InvalidBranchingChoice { .. }
            | Error::MaxDepthExceeded { .. }
            | Error::BranchingPointNotFound { .. }
            | Error::NoBranchingChoicesAvailable 
            | Error::InvalidPropertyChain { .. }
            | Error::InvalidAssertion { .. } => false,
        }
    }
}
