//! OWL 2 Profiles
//!
//! This module implements validation and detection for the OWL 2 profiles:
//! EL, QL, RL, and DL as defined in the W3C OWL 2 Profiles specification.
//!
//! Each profile has specific restrictions on the OWL constructs that can be used,
//! providing different computational characteristics and reasoning complexity.

use crate::error::OxidowlError;
use crate::ontology::{Axiom, ClassExpression, DataRange, ObjectPropertyExpression, Ontology};

pub mod dl;
pub mod el;
pub mod ql;
pub mod rl;
pub mod validator;
pub mod el_reasoner;
pub mod rl_reasoner;

// Re-export reasoners for easier access
pub use el_reasoner::ELReasoner;
pub use rl_reasoner::RLReasoner;

/// OWL 2 Profile types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OWL2Profile {
    /// OWL 2 EL - Existential Language
    /// Optimized for classification and instance checking
    EL,
    /// OWL 2 QL - Query Language
    /// Optimized for query answering
    QL,
    /// OWL 2 RL - Rule Language  
    /// Optimized for rule-based reasoning
    RL,
    /// OWL 2 DL - Description Logic
    /// Full OWL 2 with decidability guarantees
    DL,
    /// OWL 2 Full - Complete OWL 2
    /// No computational guarantees
    Full,
}

impl OWL2Profile {
    /// Get the profile name as a string
    #[must_use] 
    pub fn name(&self) -> &'static str {
        match self {
            OWL2Profile::EL => "OWL 2 EL",
            OWL2Profile::QL => "OWL 2 QL",
            OWL2Profile::RL => "OWL 2 RL",
            OWL2Profile::DL => "OWL 2 DL",
            OWL2Profile::Full => "OWL 2 Full",
        }
    }

    /// Get the profile description
    #[must_use] 
    pub fn description(&self) -> &'static str {
        match self {
            OWL2Profile::EL => "Existential Language - optimized for classification",
            OWL2Profile::QL => "Query Language - optimized for query answering",
            OWL2Profile::RL => "Rule Language - optimized for rule-based reasoning",
            OWL2Profile::DL => "Description Logic - full OWL 2 with decidability",
            OWL2Profile::Full => "Full OWL 2 - no computational restrictions",
        }
    }
}

/// Profile validation error types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProfileViolationType {
    /// Class expression not allowed in profile
    DisallowedClassExpression(String),
    /// Property expression not allowed in profile  
    DisallowedPropertyExpression(String),
    /// Axiom type not allowed in profile
    DisallowedAxiom(String),
    /// Data range not allowed in profile
    DisallowedDataRange(String),
    /// Feature not supported in profile
    UnsupportedFeature(String),
    /// Complex construct exceeds profile limits
    ComplexityViolation(String),
    /// RDF-star: Quoted triple not supported in this profile
    QuotedTripleNotSupported(String),
    /// RDF-star: Excessive quoted triple nesting for profile
    ExcessiveNestingForProfile(String),
}

impl std::fmt::Display for ProfileViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileViolationType::DisallowedClassExpression(msg) => {
                write!(f, "Disallowed class expression: {msg}")
            }
            ProfileViolationType::DisallowedPropertyExpression(msg) => {
                write!(f, "Disallowed property expression: {msg}")
            }
            ProfileViolationType::DisallowedAxiom(msg) => {
                write!(f, "Disallowed axiom: {msg}")
            }
            ProfileViolationType::DisallowedDataRange(msg) => {
                write!(f, "Disallowed data range: {msg}")
            }
            ProfileViolationType::UnsupportedFeature(msg) => {
                write!(f, "Unsupported feature: {msg}")
            }
            ProfileViolationType::ComplexityViolation(msg) => {
                write!(f, "Complexity violation: {msg}")
            }
            ProfileViolationType::QuotedTripleNotSupported(msg) => {
                write!(f, "Quoted triple not supported in profile: {msg}")
            }
            ProfileViolationType::ExcessiveNestingForProfile(msg) => {
                write!(f, "Excessive nesting for profile: {msg}")
            }
        }
    }
}

/// Profile validation error
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileViolation {
    /// Type of violation
    pub violation_type: ProfileViolationType,
    /// Axiom ID that caused the violation (if applicable)
    pub axiom_id: Option<crate::ontology::AxiomId>,
    /// Additional context information
    pub context: String,
    /// Location information (if available)
    pub location: Option<String>,
}

impl ProfileViolation {
    /// Create a new profile violation
    pub fn new(violation_type: ProfileViolationType, context: impl Into<String>) -> Self {
        Self {
            violation_type,
            axiom_id: None,
            context: context.into(),
            location: None,
        }
    }

    /// Set the axiom ID that caused the violation
    #[must_use] 
    pub fn with_axiom_id(mut self, axiom_id: crate::ontology::AxiomId) -> Self {
        self.axiom_id = Some(axiom_id);
        self
    }

    /// Set the location information
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

impl std::fmt::Display for ProfileViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.violation_type, self.context)?;
        if let Some(axiom_id) = self.axiom_id {
            write!(f, " (axiom: {axiom_id})")?;
        }
        if let Some(location) = &self.location {
            write!(f, " at {location}")?;
        }
        Ok(())
    }
}

/// Profile validation report
#[derive(Debug, Clone)]
pub struct ProfileValidationReport {
    /// The profile that was validated against
    pub profile: OWL2Profile,
    /// Whether the ontology conforms to the profile
    pub conforms: bool,
    /// List of violations found
    pub violations: Vec<ProfileViolation>,
    /// Statistics about the validation
    pub stats: ValidationStatistics,
}

impl ProfileValidationReport {
    /// Create a new profile validation report
    #[must_use] 
    pub fn new(profile: OWL2Profile) -> Self {
        Self {
            profile,
            conforms: true,
            violations: Vec::new(),
            stats: ValidationStatistics::default(),
        }
    }

    /// Add a violation to the report
    pub fn add_violation(&mut self, violation: ProfileViolation) {
        self.conforms = false;
        self.violations.push(violation);
    }

    /// Check if the ontology conforms to the profile
    #[must_use] 
    pub fn is_valid(&self) -> bool {
        self.conforms
    }

    /// Get the number of violations
    #[must_use] 
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

/// Validation statistics
#[derive(Debug, Clone, Default)]
pub struct ValidationStatistics {
    /// Number of axioms checked
    pub axioms_checked: usize,
    /// Number of class expressions checked
    pub class_expressions_checked: usize,
    /// Number of property expressions checked  
    pub property_expressions_checked: usize,
    /// Validation duration in milliseconds
    pub duration_ms: u64,
}

/// Profile detection result
#[derive(Debug, Clone)]
pub struct ProfileDetectionResult {
    /// Profiles that the ontology conforms to
    pub conforming_profiles: Vec<OWL2Profile>,
    /// The most restrictive conforming profile
    pub most_restrictive: Option<OWL2Profile>,
    /// The least restrictive profile needed
    pub least_restrictive: OWL2Profile,
    /// Detailed analysis for each profile
    pub profile_analysis: std::collections::HashMap<OWL2Profile, ProfileValidationReport>,
}

impl ProfileDetectionResult {
    /// Create a new profile detection result
    #[must_use] 
    pub fn new() -> Self {
        Self {
            conforming_profiles: Vec::new(),
            most_restrictive: None,
            least_restrictive: OWL2Profile::Full,
            profile_analysis: std::collections::HashMap::new(),
        }
    }

    /// Add a profile analysis result
    pub fn add_analysis(&mut self, profile: OWL2Profile, report: ProfileValidationReport) {
        if report.conforms {
            self.conforming_profiles.push(profile);
            if self.most_restrictive.is_none() {
                self.most_restrictive = Some(profile);
            }
        }
        self.profile_analysis.insert(profile, report);
    }

    /// Get the recommended profile for the ontology
    #[must_use] 
    pub fn recommended_profile(&self) -> OWL2Profile {
        self.most_restrictive.unwrap_or(self.least_restrictive)
    }
}

impl Default for ProfileDetectionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for profile-specific validation
pub trait ProfileValidator {
    /// Validate an ontology against this profile
    fn validate(&self, ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError>;

    /// Check if a class expression is allowed in this profile
    fn is_class_expression_allowed(&self, expr: &ClassExpression) -> bool;

    /// Check if a property expression is allowed in this profile
    fn is_property_expression_allowed(&self, expr: &ObjectPropertyExpression) -> bool;

    /// Check if an axiom is allowed in this profile
    fn is_axiom_allowed(&self, axiom: &Axiom) -> bool;

    /// Check if a data range is allowed in this profile
    fn is_data_range_allowed(&self, range: &DataRange) -> bool;

    /// Get the profile this validator handles
    fn profile(&self) -> OWL2Profile;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_names() {
        assert_eq!(OWL2Profile::EL.name(), "OWL 2 EL");
        assert_eq!(OWL2Profile::QL.name(), "OWL 2 QL");
        assert_eq!(OWL2Profile::RL.name(), "OWL 2 RL");
        assert_eq!(OWL2Profile::DL.name(), "OWL 2 DL");
        assert_eq!(OWL2Profile::Full.name(), "OWL 2 Full");
    }

    #[test]
    fn test_profile_violation_display() {
        let violation = ProfileViolation::new(
            ProfileViolationType::DisallowedClassExpression("ObjectComplementOf".to_string()),
            "Class complement not allowed in OWL 2 EL",
        )
        .with_axiom_id(123);

        let display = violation.to_string();
        assert!(display.contains("Disallowed class expression"));
        assert!(display.contains("ObjectComplementOf"));
        assert!(display.contains("axiom: 123"));
    }

    #[test]
    fn test_validation_report() {
        let mut report = ProfileValidationReport::new(OWL2Profile::EL);
        assert!(report.is_valid());
        assert_eq!(report.violation_count(), 0);

        let violation = ProfileViolation::new(
            ProfileViolationType::UnsupportedFeature("Universal restriction".to_string()),
            "ObjectAllValuesFrom not supported",
        );
        report.add_violation(violation);

        assert!(!report.is_valid());
        assert_eq!(report.violation_count(), 1);
    }

    #[test]
    fn test_profile_detection_result() {
        let mut result = ProfileDetectionResult::new();
        assert_eq!(result.conforming_profiles.len(), 0);
        assert!(result.most_restrictive.is_none());

        let el_report = ProfileValidationReport::new(OWL2Profile::EL);
        result.add_analysis(OWL2Profile::EL, el_report);

        assert_eq!(result.conforming_profiles.len(), 1);
        assert_eq!(result.most_restrictive, Some(OWL2Profile::EL));
        assert_eq!(result.recommended_profile(), OWL2Profile::EL);
    }
}
