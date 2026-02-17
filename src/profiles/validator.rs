//! Profile Validator
//!
//! This module provides unified validation and detection across all OWL 2 profiles.
//! It can validate an ontology against a specific profile or detect which profiles
//! an ontology conforms to.

use crate::error::OxidowlError;
use crate::ontology::Ontology;
use crate::profiles::el::ELValidator;
use crate::profiles::ql::QLValidator;
use crate::profiles::rl::RLValidator;
use crate::profiles::{
    OWL2Profile, ProfileDetectionResult, ProfileValidationReport, ProfileValidator,
};

/// Main profile validator that can handle all OWL 2 profiles
pub struct OWL2ProfileValidator {
    el_validator: ELValidator,
    ql_validator: QLValidator,
    rl_validator: RLValidator,
}

impl OWL2ProfileValidator {
    /// Create a new OWL 2 profile validator
    pub fn new() -> Self {
        Self {
            el_validator: ELValidator::new(),
            ql_validator: QLValidator::new(),
            rl_validator: RLValidator::new(),
        }
    }

    /// Validate an ontology against a specific profile
    pub fn validate_profile(
        &self,
        ontology: &Ontology,
        profile: OWL2Profile,
    ) -> Result<ProfileValidationReport, OxidowlError> {
        match profile {
            OWL2Profile::EL => self.el_validator.validate(ontology),
            OWL2Profile::QL => self.ql_validator.validate(ontology),
            OWL2Profile::RL => self.rl_validator.validate(ontology),
            OWL2Profile::DL => {
                // Use the existing OWL 2 DL validator
                let mut dl_validator =
                    crate::validation::owl2_dl::OWL2DLValidator::new(ontology.clone());
                match dl_validator.validate() {
                    Ok(dl_report) => {
                        let mut report = ProfileValidationReport::new(profile);
                        if !dl_report.is_valid {
                            // Convert DL validation errors to profile violations
                            for error in &dl_report.errors {
                                report.add_violation(crate::profiles::ProfileViolation::new(
                                    crate::profiles::ProfileViolationType::DisallowedAxiom(
                                        error.error_type.to_string(),
                                    ),
                                    error.message.clone(),
                                ));
                            }
                        }
                        Ok(report)
                    }
                    Err(e) => Err(e),
                }
            }
            OWL2Profile::Full => {
                // OWL 2 Full always passes (no restrictions)
                Ok(ProfileValidationReport::new(profile))
            }
        }
    }

    /// Detect which profiles an ontology conforms to
    pub fn detect_profiles(
        &self,
        ontology: &Ontology,
    ) -> Result<ProfileDetectionResult, OxidowlError> {
        let mut result = ProfileDetectionResult::new();

        // Test profiles in order from most restrictive to least restrictive
        let profiles_to_test = vec![
            OWL2Profile::EL,
            OWL2Profile::QL,
            OWL2Profile::RL,
            OWL2Profile::DL,
            OWL2Profile::Full,
        ];

        for profile in profiles_to_test {
            let report = self.validate_profile(ontology, profile)?;
            result.add_analysis(profile, report);
        }

        // Determine the least restrictive profile needed
        if result.conforming_profiles.is_empty() {
            result.least_restrictive = OWL2Profile::Full;
        } else {
            // Find the least restrictive conforming profile
            for profile in [
                OWL2Profile::Full,
                OWL2Profile::DL,
                OWL2Profile::RL,
                OWL2Profile::QL,
                OWL2Profile::EL,
            ] {
                if result.conforming_profiles.contains(&profile) {
                    result.least_restrictive = profile;
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Get a validator for a specific profile
    pub fn get_validator(&self, profile: OWL2Profile) -> Box<dyn ProfileValidator> {
        match profile {
            OWL2Profile::EL => Box::new(ELValidator::new()),
            OWL2Profile::QL => Box::new(QLValidator::new()),
            OWL2Profile::RL => Box::new(RLValidator::new()),
            _ => Box::new(GenericValidator::new(profile)),
        }
    }

    /// Recommend the best profile for an ontology
    pub fn recommend_profile(&self, ontology: &Ontology) -> Result<OWL2Profile, OxidowlError> {
        let detection = self.detect_profiles(ontology)?;
        Ok(detection.recommended_profile())
    }

    /// Check if an ontology can be safely converted to a specific profile
    pub fn can_convert_to_profile(
        &self,
        ontology: &Ontology,
        target_profile: OWL2Profile,
    ) -> Result<bool, OxidowlError> {
        let report = self.validate_profile(ontology, target_profile)?;
        Ok(report.conforms)
    }

    /// Get optimization suggestions for a profile
    pub fn get_optimization_suggestions(
        &self,
        ontology: &Ontology,
        target_profile: OWL2Profile,
    ) -> Result<Vec<String>, OxidowlError> {
        let report = self.validate_profile(ontology, target_profile)?;
        let mut suggestions = Vec::new();

        if !report.conforms {
            suggestions.push(format!(
                "Ontology does not conform to {}. Consider the following changes:",
                target_profile.name()
            ));

            for violation in &report.violations {
                match &violation.violation_type {
                    crate::profiles::ProfileViolationType::DisallowedClassExpression(expr) => {
                        suggestions.push(format!("Remove or replace {}", expr));
                    }
                    crate::profiles::ProfileViolationType::DisallowedAxiom(axiom) => {
                        suggestions.push(format!("Remove or rewrite {} axiom", axiom));
                    }
                    crate::profiles::ProfileViolationType::ComplexityViolation(msg) => {
                        suggestions.push(format!("Simplify construct: {}", msg));
                    }
                    _ => {
                        suggestions.push(format!("Address: {}", violation.violation_type));
                    }
                }
            }
        } else {
            suggestions.push(format!(
                "Ontology conforms to {}. No changes needed.",
                target_profile.name()
            ));

            // Add performance suggestions
            match target_profile {
                OWL2Profile::EL => {
                    suggestions.push(
                        "Consider using specialized EL reasoners for optimal performance."
                            .to_string(),
                    );
                }
                OWL2Profile::QL => {
                    suggestions.push(
                        "Consider using query rewriting techniques for efficient querying."
                            .to_string(),
                    );
                }
                OWL2Profile::RL => {
                    suggestions.push(
                        "Consider using rule-based reasoners for efficient inference.".to_string(),
                    );
                }
                _ => {}
            }
        }

        Ok(suggestions)
    }
}

impl Default for OWL2ProfileValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic validator for profiles that don't have specific implementations yet
struct GenericValidator {
    profile: OWL2Profile,
}

impl GenericValidator {
    fn new(profile: OWL2Profile) -> Self {
        Self { profile }
    }
}

impl ProfileValidator for GenericValidator {
    fn validate(&self, _ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError> {
        let mut report = ProfileValidationReport::new(self.profile);

        match self.profile {
            OWL2Profile::Full => {
                // OWL 2 Full allows everything
                // report remains valid (no violations)
            }
            _ => {
                // For unimplemented profiles, add a violation
                report.add_violation(crate::profiles::ProfileViolation::new(
                    crate::profiles::ProfileViolationType::UnsupportedFeature(format!(
                        "{} validation not yet implemented",
                        self.profile.name()
                    )),
                    "Profile validation unavailable",
                ));
            }
        }

        Ok(report)
    }

    fn is_class_expression_allowed(&self, _expr: &crate::ontology::ClassExpression) -> bool {
        matches!(self.profile, OWL2Profile::Full)
    }

    fn is_property_expression_allowed(
        &self,
        _expr: &crate::ontology::ObjectPropertyExpression,
    ) -> bool {
        matches!(self.profile, OWL2Profile::Full)
    }

    fn is_axiom_allowed(&self, _axiom: &crate::ontology::Axiom) -> bool {
        matches!(self.profile, OWL2Profile::Full)
    }

    fn is_data_range_allowed(&self, _range: &crate::ontology::DataRange) -> bool {
        matches!(self.profile, OWL2Profile::Full)
    }

    fn profile(&self) -> OWL2Profile {
        self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Ontology};

    #[test]
    fn test_profile_validator_creation() {
        let validator = OWL2ProfileValidator::new();

        // Test that we can get validators for different profiles
        let _el_validator = validator.get_validator(OWL2Profile::EL);
        let _full_validator = validator.get_validator(OWL2Profile::Full);
    }

    #[test]
    fn test_owl_full_always_passes() {
        let validator = OWL2ProfileValidator::new();
        let ontology = Ontology::new();

        let report = validator
            .validate_profile(&ontology, OWL2Profile::Full)
            .expect("Failed to validate ontology against OWL 2 Full profile");
        assert!(report.conforms);
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn test_profile_detection() {
        let validator = OWL2ProfileValidator::new();
        let ontology = Ontology::new(); // Empty ontology should conform to all profiles

        let result = validator
            .detect_profiles(&ontology)
            .expect("Failed to detect compatible OWL 2 profiles for ontology");

        // Empty ontology should conform to all implemented profiles
        assert!(result.conforming_profiles.contains(&OWL2Profile::EL));
        assert!(result.conforming_profiles.contains(&OWL2Profile::Full));

        // Most restrictive should be EL for empty ontology
        assert_eq!(result.most_restrictive, Some(OWL2Profile::EL));
    }

    #[test]
    fn test_profile_recommendation() {
        let validator = OWL2ProfileValidator::new();
        let ontology = Ontology::new();

        let recommended = validator
            .recommend_profile(&ontology)
            .expect("Failed to recommend optimal OWL 2 profile for ontology");

        // For empty ontology, should recommend most restrictive (EL)
        assert_eq!(recommended, OWL2Profile::EL);
    }

    #[test]
    fn test_optimization_suggestions() {
        let validator = OWL2ProfileValidator::new();
        let ontology = Ontology::new();

        let suggestions = validator
            .get_optimization_suggestions(&ontology, OWL2Profile::EL)
            .expect("Failed to get optimization suggestions for OWL 2 EL profile");

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("conforms to"));
    }

    #[test]
    fn test_conversion_check() {
        let validator = OWL2ProfileValidator::new();
        let ontology = Ontology::new();

        // Empty ontology should be convertible to any profile
        assert!(
            validator
                .can_convert_to_profile(&ontology, OWL2Profile::EL)
                .expect("Failed to check if ontology can convert to OWL 2 EL profile")
        );
        assert!(
            validator
                .can_convert_to_profile(&ontology, OWL2Profile::Full)
                .expect("Failed to check if ontology can convert to OWL 2 Full profile")
        );
    }
}
