//! Error types for the Oxidowl reasoner
//!
//! This module defines all error types that can occur during reasoning operations,
//! parsing, network operations, and configuration.

/// Main error type for Oxidowl
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Ontology parsing error with optional context information
    #[error("{}", format_ontology_parsing_error(.message, .line, .column, .context, .token))]
    OntologyParsing {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
        context: Option<String>,
        token: Option<String>,
    },

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
    #[error("File I/O error: {message}")]
    Io { message: String },

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

    /// Invalid datatype error
    #[error("Invalid datatype: {0}")]
    InvalidDatatype(String),

    /// Invalid literal error
    #[error("Invalid literal: {0}")]
    InvalidLiteral(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

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

    /// Queue full error
    #[error("Queue full")]
    QueueFull,

    /// DL Query parsing error
    #[error("DL Query error: {message}")]
    DLQuery { message: String },

    /// Axiom already exists error
    #[error("Axiom already exists")]
    AxiomAlreadyExists,

    /// Axiom not found error
    #[error("Axiom not found")]
    AxiomNotFound,

    /// Import error
    #[error("Import error: {message}")]
    ImportError { message: String },

    /// Reasoning error (alternative variant)
    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    /// Configuration error (alternative variant)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Lock acquisition failed (poisoned lock)
    #[error("Lock poisoned: {message}")]
    LockPoisoned { message: String },

    /// Data structure in unexpected state
    #[error("Data structure error: {message}")]
    DataStructure { message: String },

    /// Collection operation failed
    #[error("Collection error: {message}")]
    CollectionError { message: String },

    /// System operation failed
    #[error("System error: {message}")]
    SystemError { message: String },

    /// RDF-star syntax error with optional position information
    #[error("{}", format_rdf_star_syntax_error(.message, .line, .column, .context))]
    RdfStarSyntax {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
        context: Option<String>,
    },

    /// Quoted triple in predicate position (forbidden by RDF-star spec)
    #[error("RDF-star error: Quoted triples are not allowed in predicate position")]
    QuotedTripleInPredicatePosition,

    /// Excessive quoted triple nesting
    #[error("RDF-star error: Quoted triple nesting depth {depth} exceeds maximum {max}")]
    ExcessiveQuotedTripleNesting { depth: usize, max: usize },

    /// Invalid quoted triple structure
    #[error("RDF-star error: Invalid quoted triple structure: {message}")]
    InvalidQuotedTripleStructure { message: String },

    /// RDF 1.2 directional literal error
    #[error("RDF 1.2 error: Invalid directional literal: {message}")]
    InvalidDirectionalLiteral { message: String },

    /// RDF version incompatibility
    #[error(
        "RDF version error: {feature} requires {required_version}, but graph is in {current_version} mode"
    )]
    RdfVersionIncompatibility {
        feature: String,
        required_version: String,
        current_version: String,
    },

    /// Reification conversion error
    #[error("Reification error: {message}")]
    ReificationError { message: String },
}

// Manual Clone implementation because Backtrace doesn't implement Clone
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::OntologyParsing {
                message,
                line,
                column,
                context,
                token,
            } => Error::OntologyParsing {
                message: message.clone(),
                line: *line,
                column: *column,
                context: context.clone(),
                token: token.clone(),
            },
            Error::Reasoning { message } => Error::Reasoning {
                message: message.clone(),
            },
            Error::Config { message } => Error::Config {
                message: message.clone(),
            },
            Error::Network { message } => Error::Network {
                message: message.clone(),
            },
            Error::Io { message } => Error::Io {
                message: message.clone(),
            },
            Error::XmlParsing { message } => Error::XmlParsing {
                message: message.clone(),
            },
            Error::Sparql { message } => Error::Sparql {
                message: message.clone(),
            },
            Error::Cache { message } => Error::Cache {
                message: message.clone(),
            },
            Error::ResourceExhaustion { message } => Error::ResourceExhaustion {
                message: message.clone(),
            },
            Error::Timeout { message } => Error::Timeout {
                message: message.clone(),
            },
            Error::Unsupported { message } => Error::Unsupported {
                message: message.clone(),
            },
            Error::InvalidDatatype(s) => Error::InvalidDatatype(s.clone()),
            Error::InvalidLiteral(s) => Error::InvalidLiteral(s.clone()),
            Error::ParseError(s) => Error::ParseError(s.clone()),
            Error::Internal { message } => Error::Internal {
                message: message.clone(),
            },
            Error::InvalidInput { message } => Error::InvalidInput {
                message: message.clone(),
            },
            Error::InvalidDisjunctIndex { index } => Error::InvalidDisjunctIndex { index: *index },
            Error::InvalidBranchingChoice { index } => {
                Error::InvalidBranchingChoice { index: *index }
            }
            Error::MaxDepthExceeded { depth } => Error::MaxDepthExceeded { depth: *depth },
            Error::BranchingPointNotFound { id } => {
                Error::BranchingPointNotFound { id: id.clone() }
            }
            Error::NoBranchingChoicesAvailable => Error::NoBranchingChoicesAvailable,
            Error::ResourceExhausted { message } => Error::ResourceExhausted {
                message: message.clone(),
            },
            Error::InvalidPropertyChain { message } => Error::InvalidPropertyChain {
                message: message.clone(),
            },
            Error::InvalidAssertion { message } => Error::InvalidAssertion {
                message: message.clone(),
            },
            Error::QueueFull => Error::QueueFull,
            Error::DLQuery { message } => Error::DLQuery {
                message: message.clone(),
            },
            Error::AxiomAlreadyExists => Error::AxiomAlreadyExists,
            Error::AxiomNotFound => Error::AxiomNotFound,
            Error::ImportError { message } => Error::ImportError {
                message: message.clone(),
            },
            Error::ReasoningError(s) => Error::ReasoningError(s.clone()),
            Error::ConfigurationError(s) => Error::ConfigurationError(s.clone()),
            Error::LockPoisoned { message } => Error::LockPoisoned {
                message: message.clone(),
            },
            Error::DataStructure { message } => Error::DataStructure {
                message: message.clone(),
            },
            Error::CollectionError { message } => Error::CollectionError {
                message: message.clone(),
            },
            Error::SystemError { message } => Error::SystemError {
                message: message.clone(),
            },
            Error::RdfStarSyntax {
                message,
                line,
                column,
                context,
            } => Error::RdfStarSyntax {
                message: message.clone(),
                line: *line,
                column: *column,
                context: context.clone(),
            },
            Error::QuotedTripleInPredicatePosition => Error::QuotedTripleInPredicatePosition,
            Error::ExcessiveQuotedTripleNesting { depth, max } => {
                Error::ExcessiveQuotedTripleNesting {
                    depth: *depth,
                    max: *max,
                }
            }
            Error::InvalidQuotedTripleStructure { message } => {
                Error::InvalidQuotedTripleStructure {
                    message: message.clone(),
                }
            }
            Error::InvalidDirectionalLiteral { message } => Error::InvalidDirectionalLiteral {
                message: message.clone(),
            },
            Error::RdfVersionIncompatibility {
                feature,
                required_version,
                current_version,
            } => Error::RdfVersionIncompatibility {
                feature: feature.clone(),
                required_version: required_version.clone(),
                current_version: current_version.clone(),
            },
            Error::ReificationError { message } => Error::ReificationError {
                message: message.clone(),
            },
        }
    }
}

/// Format ontology parsing error based on verbosity level
fn format_ontology_parsing_error(
    message: &str,
    line: &Option<usize>,
    column: &Option<usize>,
    context: &Option<String>,
    token: &Option<String>,
) -> String {
    let mut result = format!("Ontology parsing error: {message}");

    if let Some(l) = line {
        if let Some(c) = column {
            result.push_str(&format!(" at line {l}, column {c}"));
        } else {
            result.push_str(&format!(" at line {l}"));
        }
    }

    if let Some(t) = token {
        result.push_str(&format!(" (token: '{t}')"));
    }

    if let Some(ctx) = context {
        result.push_str(&format!("\nContext: {ctx}"));
    }

    result
}

/// Format RDF-star syntax error with position information
fn format_rdf_star_syntax_error(
    message: &str,
    line: &Option<usize>,
    column: &Option<usize>,
    context: &Option<String>,
) -> String {
    let mut result = format!("RDF-star syntax error: {message}");

    if let Some(l) = line {
        if let Some(c) = column {
            result.push_str(&format!(" at line {l}, column {c}"));
        } else {
            result.push_str(&format!(" at line {l}"));
        }
    }

    if let Some(ctx) = context {
        result.push_str(&format!("\nContext: {ctx}"));
    }

    // Add helpful hint for common RDF-star errors
    if message.contains("<<") || message.contains(">>") {
        result.push_str("\nHint: Quoted triples use << >> syntax and require RDF-star mode");
    }

    result
}

/// Specialized error for reasoner operations
/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Type alias for backwards compatibility
pub type OxidowlError = Error;

impl Error {
    /// Ontology parsing error constructor (minimal - backward compatible)
    pub fn ontology_parsing<S: Into<String>>(message: S) -> Self {
        Self::OntologyParsing {
            message: message.into(),
            line: None,
            column: None,
            context: None,
            token: None,
        }
    }

    /// Ontology parsing error constructor with detailed context
    pub fn ontology_parsing_detailed<S: Into<String>>(
        message: S,
        line: Option<usize>,
        column: Option<usize>,
        context: Option<String>,
        token: Option<String>,
    ) -> Self {
        Self::OntologyParsing {
            message: message.into(),
            line,
            column,
            context,
            token,
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
            message: message.into(),
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

    /// DL Query error constructor
    pub fn dl_query<S: Into<String>>(message: S) -> Self {
        Self::DLQuery {
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

    /// Lock poisoned error constructor
    pub fn lock_poisoned<S: Into<String>>(message: S) -> Self {
        Self::LockPoisoned {
            message: message.into(),
        }
    }

    /// Data structure error constructor
    pub fn data_structure<S: Into<String>>(message: S) -> Self {
        Self::DataStructure {
            message: message.into(),
        }
    }

    /// Collection error constructor
    pub fn collection_error<S: Into<String>>(message: S) -> Self {
        Self::CollectionError {
            message: message.into(),
        }
    }

    /// System error constructor
    pub fn system_error<S: Into<String>>(message: S) -> Self {
        Self::SystemError {
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
    #[must_use]
    pub fn invalid_disjunct_index(index: usize) -> Self {
        Self::InvalidDisjunctIndex { index }
    }

    /// Create an invalid branching choice error
    #[must_use]
    pub fn invalid_branching_choice(index: usize) -> Self {
        Self::InvalidBranchingChoice { index }
    }

    /// Create a max depth exceeded error
    #[must_use]
    pub fn max_depth_exceeded(depth: usize) -> Self {
        Self::MaxDepthExceeded { depth }
    }

    /// Create a branching point not found error
    pub fn branching_point_not_found<S: Into<String>>(id: S) -> Self {
        Self::BranchingPointNotFound { id: id.into() }
    }

    /// Create a no branching choices available error
    #[must_use]
    pub fn no_branching_choices_available() -> Self {
        Self::NoBranchingChoicesAvailable
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted<S: Into<String>>(message: S) -> Self {
        Self::ResourceExhausted {
            message: message.into(),
        }
    }

    /// Create an import error
    pub fn import_error<S: Into<String>>(message: S) -> Self {
        Self::ImportError {
            message: message.into(),
        }
    }

    /// Create a parse error
    pub fn parse_error<S: Into<String>>(message: S) -> Self {
        Self::ParseError(message.into())
    }

    /// RDF-star syntax error constructor (minimal)
    pub fn rdf_star_syntax<S: Into<String>>(message: S) -> Self {
        Self::RdfStarSyntax {
            message: message.into(),
            line: None,
            column: None,
            context: None,
        }
    }

    /// RDF-star syntax error constructor with detailed context
    pub fn rdf_star_syntax_detailed<S: Into<String>>(
        message: S,
        line: Option<usize>,
        column: Option<usize>,
        context: Option<String>,
    ) -> Self {
        Self::RdfStarSyntax {
            message: message.into(),
            line,
            column,
            context,
        }
    }

    /// Quoted triple in predicate position error
    #[must_use]
    pub fn quoted_triple_in_predicate_position() -> Self {
        Self::QuotedTripleInPredicatePosition
    }

    /// Excessive quoted triple nesting error
    #[must_use]
    pub fn excessive_quoted_triple_nesting(depth: usize, max: usize) -> Self {
        Self::ExcessiveQuotedTripleNesting { depth, max }
    }

    /// Invalid quoted triple structure error
    pub fn invalid_quoted_triple_structure<S: Into<String>>(message: S) -> Self {
        Self::InvalidQuotedTripleStructure {
            message: message.into(),
        }
    }

    /// Invalid directional literal error
    pub fn invalid_directional_literal<S: Into<String>>(message: S) -> Self {
        Self::InvalidDirectionalLiteral {
            message: message.into(),
        }
    }

    /// RDF version incompatibility error
    pub fn rdf_version_incompatibility<S: Into<String>>(
        feature: S,
        required_version: S,
        current_version: S,
    ) -> Self {
        Self::RdfVersionIncompatibility {
            feature: feature.into(),
            required_version: required_version.into(),
            current_version: current_version.into(),
        }
    }

    /// Reification error constructor
    pub fn reification_error<S: Into<String>>(message: S) -> Self {
        Self::ReificationError {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
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
    #[must_use]
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::OntologyParsing { .. } | Error::XmlParsing { .. } | Error::Io { .. } => {
                ErrorCategory::Input
            }
            Error::Reasoning { .. } | Error::Sparql { .. } => ErrorCategory::Reasoning,
            Error::Config { .. } => ErrorCategory::Config,
            Error::Network { .. } => ErrorCategory::Network,
            Error::Cache { .. }
            | Error::ResourceExhaustion { .. }
            | Error::ResourceExhausted { .. }
            | Error::Timeout { .. } => ErrorCategory::Resource,
            Error::Unsupported { .. } | Error::Internal { .. } | Error::LockPoisoned { .. } => {
                ErrorCategory::Internal
            }
            Error::InvalidInput { .. }
            | Error::InvalidDisjunctIndex { .. }
            | Error::InvalidBranchingChoice { .. }
            | Error::MaxDepthExceeded { .. }
            | Error::BranchingPointNotFound { .. }
            | Error::NoBranchingChoicesAvailable
            | Error::InvalidPropertyChain { .. }
            | Error::InvalidAssertion { .. }
            | Error::QueueFull
            | Error::AxiomAlreadyExists
            | Error::AxiomNotFound
            | Error::DataStructure { .. }
            | Error::CollectionError { .. }
            | Error::SystemError { .. } => ErrorCategory::Internal,
            Error::DLQuery { .. } => ErrorCategory::Input,
            Error::InvalidDatatype(_) => ErrorCategory::Input,
            Error::InvalidLiteral(_) => ErrorCategory::Input,
            Error::ParseError(_) => ErrorCategory::Input,
            Error::ImportError { .. } => ErrorCategory::Input,
            Error::ReasoningError(_) => ErrorCategory::Reasoning,
            Error::ConfigurationError(_) => ErrorCategory::Config,
            Error::RdfStarSyntax { .. }
            | Error::QuotedTripleInPredicatePosition
            | Error::ExcessiveQuotedTripleNesting { .. }
            | Error::InvalidQuotedTripleStructure { .. }
            | Error::InvalidDirectionalLiteral { .. }
            | Error::RdfVersionIncompatibility { .. } => ErrorCategory::Input,
            Error::ReificationError { .. } => ErrorCategory::Internal,
        }
    }

    /// Check if the error is recoverable
    #[must_use]
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
            Error::Io { .. } | Error::Internal { .. } | Error::LockPoisoned { .. } => false,
            Error::InvalidInput { .. }
            | Error::InvalidDisjunctIndex { .. }
            | Error::InvalidBranchingChoice { .. }
            | Error::MaxDepthExceeded { .. }
            | Error::BranchingPointNotFound { .. }
            | Error::NoBranchingChoicesAvailable
            | Error::InvalidPropertyChain { .. }
            | Error::InvalidAssertion { .. }
            | Error::QueueFull
            | Error::AxiomAlreadyExists
            | Error::AxiomNotFound
            | Error::DataStructure { .. }
            | Error::CollectionError { .. }
            | Error::SystemError { .. } => false,
            Error::DLQuery { .. } => false,
            Error::InvalidDatatype(_) => false,
            Error::InvalidLiteral(_) => false,
            Error::ParseError(_) => false,
            Error::ImportError { .. } => false,
            Error::ReasoningError(_) => true,
            Error::ConfigurationError(_) => false,
            Error::RdfStarSyntax { .. }
            | Error::QuotedTripleInPredicatePosition
            | Error::ExcessiveQuotedTripleNesting { .. }
            | Error::InvalidQuotedTripleStructure { .. }
            | Error::InvalidDirectionalLiteral { .. }
            | Error::RdfVersionIncompatibility { .. }
            | Error::ReificationError { .. } => false,
        }
    }
}

impl From<crate::validation::owl2_dl::ValidationError> for Error {
    fn from(err: crate::validation::owl2_dl::ValidationError) -> Self {
        Error::Config {
            message: err.message,
        }
    }
}

impl From<crate::query::advanced::execution::AdvancedQueryError> for Error {
    fn from(err: crate::query::advanced::execution::AdvancedQueryError) -> Self {
        Error::Reasoning {
            message: err.to_string(),
        }
    }
}
