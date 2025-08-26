//! OWL 2 DL Profile Validator
//!
//! OWL 2 DL (Description Logic) provides the full expressivity of OWL 2 while
//! maintaining decidability. This validator wraps the existing OWL 2 DL validation
//! functionality to integrate with the profile system.

use crate::error::OxidowlError;
use crate::ontology::{Ontology, Axiom, ClassExpression, ObjectPropertyExpression, DataRange};
use crate::profiles::{
    ProfileValidator, ProfileValidationReport, ProfileViolation, ProfileViolationType, 
    OWL2Profile
};
use crate::validation::owl2_dl::OWL2DLValidator as CoreDLValidator;

/// OWL 2 DL Profile Validator
pub struct DLValidator;

impl DLValidator {
    /// Create a new DL profile validator
    pub fn new() -> Self {
        Self
    }
}

impl Default for DLValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileValidator for DLValidator {
    fn validate(&self, ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError> {
        let mut report = ProfileValidationReport::new(OWL2Profile::DL);
        
        // Use the existing OWL 2 DL validator
        let mut dl_validator = CoreDLValidator::new(ontology.clone());
        match dl_validator.validate() {
            Ok(dl_report) => {
                if !dl_report.is_valid {
                    // Convert DL validation errors to profile violations
                    for error in &dl_report.errors {
                        let violation = ProfileViolation::new(
                            ProfileViolationType::DisallowedAxiom(
                                error.error_type.to_string()
                            ),
                            error.message.clone(),
                        );
                        report.add_violation(violation);
                    }
                }
                
                // Convert warnings if any
                for warning in &dl_report.warnings {
                    let violation = ProfileViolation::new(
                        ProfileViolationType::ComplexityViolation(
                            warning.message.clone()
                        ),
                        "OWL 2 DL complexity warning",
                    );
                    report.add_violation(violation);
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
        
        Ok(report)
    }

    fn is_class_expression_allowed(&self, _expr: &ClassExpression) -> bool {
        // All class expressions are allowed in OWL 2 DL
        // The specific restrictions are checked by the global restrictions validator
        true
    }

    fn is_property_expression_allowed(&self, _expr: &ObjectPropertyExpression) -> bool {
        // All property expressions are allowed in OWL 2 DL
        true
    }

    fn is_axiom_allowed(&self, _axiom: &Axiom) -> bool {
        // All axiom types are allowed in OWL 2 DL
        // The specific restrictions are enforced by global restrictions
        true
    }

    fn is_data_range_allowed(&self, _range: &DataRange) -> bool {
        // All data ranges are allowed in OWL 2 DL
        true
    }

    fn profile(&self) -> OWL2Profile {
        OWL2Profile::DL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dl_validator_creation() {
        let validator = DLValidator::new();
        assert_eq!(validator.profile(), OWL2Profile::DL);
    }

    #[test]
    fn test_dl_validation_with_empty_ontology() {
        let validator = DLValidator::new();
        let ontology = Ontology::new();
        
        let report = validator.validate(&ontology).unwrap();
        // Empty ontology should be valid for OWL 2 DL
        assert!(report.conforms);
    }

    #[test]
    fn test_dl_allows_all_constructs() {
        let validator = DLValidator::new();
        
        // Test that basic constructs are allowed
        let class_expr = crate::ontology::ClassExpression::class(
            crate::ontology::IRI::new("http://example.org/Test")
        );
        assert!(validator.is_class_expression_allowed(&class_expr));
        
        let prop_expr = crate::ontology::ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(
                crate::ontology::IRI::new("http://example.org/prop")
            ).unwrap()
        );
        assert!(validator.is_property_expression_allowed(&prop_expr));
    }
}
