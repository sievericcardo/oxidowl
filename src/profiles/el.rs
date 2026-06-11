//! OWL 2 EL Profile Validator
//!
//! OWL 2 EL is the existential language profile, optimized for polynomial-time
//! classification and instance checking. It allows:
//!
//! - Existential quantification (`ObjectSomeValuesFrom`, `DataSomeValuesFrom`)
//! - Intersection (`ObjectIntersectionOf`)  
//! - Nominals (`ObjectOneOf`) with some restrictions
//! - Self restrictions (`ObjectHasSelf`)
//! - Basic property axioms
//!
//! Disallowed constructs:
//! - Universal quantification (`ObjectAllValuesFrom`, `DataAllValuesFrom`)
//! - Complement (`ObjectComplementOf`)
//! - Union (`ObjectUnionOf`)
//! - Cardinality restrictions
//! - Disjoint classes/properties

use crate::error::OxidowlError;
use crate::ontology::axioms::AxiomTrait;
use crate::ontology::{Axiom, ClassExpression, DataRange, ObjectPropertyExpression, Ontology};
use crate::profiles::{
    OWL2Profile, ProfileValidationReport, ProfileValidator, ProfileViolation, ProfileViolationType,
    ValidationStatistics,
};
use std::time::Instant;

/// OWL 2 EL Profile Validator
pub struct ELValidator;

impl ELValidator {
    /// Create a new EL profile validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a class expression is valid in OWL 2 EL
    fn validate_class_expression(&self, expr: &ClassExpression) -> Result<(), ProfileViolation> {
        match expr {
            // Allowed constructs
            ClassExpression::Class(_) => Ok(()),
            ClassExpression::ObjectIntersectionOf(operands) => {
                // All operands must be valid EL class expressions
                for operand in operands {
                    self.validate_class_expression(operand)?;
                }
                Ok(())
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // Property must be simple (no property chains in EL)
                if !self.is_simple_property_expression(property) {
                    return Err(ProfileViolation::new(
                        ProfileViolationType::DisallowedPropertyExpression(
                            "Property chains not allowed in OWL 2 EL".to_string(),
                        ),
                        "ObjectSomeValuesFrom with complex property",
                    ));
                }
                // Filler must be valid EL class expression
                self.validate_class_expression(filler)?;
                Ok(())
            }
            ClassExpression::ObjectHasSelf { property } => {
                // Self restrictions allowed with simple properties
                if !self.is_simple_property_expression(property) {
                    return Err(ProfileViolation::new(
                        ProfileViolationType::DisallowedPropertyExpression(
                            "Property chains not allowed in OWL 2 EL".to_string(),
                        ),
                        "ObjectHasSelf with complex property",
                    ));
                }
                Ok(())
            }
            ClassExpression::ObjectOneOf(individuals) => {
                // Nominals allowed but with restrictions on usage
                if individuals.len() > 1 {
                    return Err(ProfileViolation::new(
                        ProfileViolationType::ComplexityViolation(
                            "Multiple individuals in ObjectOneOf not recommended in OWL 2 EL"
                                .to_string(),
                        ),
                        "Complex nominal",
                    ));
                }
                Ok(())
            }
            ClassExpression::DataSomeValuesFrom {
                property: _,
                filler,
            } => {
                // Data some values from allowed with simple data ranges
                self.validate_data_range(filler)?;
                Ok(())
            }

            // Disallowed constructs
            ClassExpression::ObjectUnionOf { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("ObjectUnionOf".to_string()),
                "Union not allowed in OWL 2 EL",
            )),
            ClassExpression::ObjectComplementOf { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("ObjectComplementOf".to_string()),
                "Complement not allowed in OWL 2 EL",
            )),
            ClassExpression::ObjectAllValuesFrom { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("ObjectAllValuesFrom".to_string()),
                "Universal quantification not allowed in OWL 2 EL",
            )),
            ClassExpression::ObjectHasValue { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("ObjectHasValue".to_string()),
                "HasValue not allowed in OWL 2 EL",
            )),
            ClassExpression::ObjectMinCardinality { .. }
            | ClassExpression::ObjectMaxCardinality { .. }
            | ClassExpression::ObjectExactCardinality { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression(
                    "Cardinality restrictions".to_string(),
                ),
                "Cardinality restrictions not allowed in OWL 2 EL",
            )),
            ClassExpression::DataAllValuesFrom { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("DataAllValuesFrom".to_string()),
                "Universal data quantification not allowed in OWL 2 EL",
            )),
            ClassExpression::DataHasValue { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression("DataHasValue".to_string()),
                "Data HasValue not allowed in OWL 2 EL",
            )),
            ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedClassExpression(
                    "Data cardinality restrictions".to_string(),
                ),
                "Data cardinality restrictions not allowed in OWL 2 EL",
            )),
        }
    }

    /// Check if a property expression is simple (no property chains)
    fn is_simple_property_expression(&self, expr: &ObjectPropertyExpression) -> bool {
        matches!(
            expr,
            ObjectPropertyExpression::ObjectProperty(_)
                | ObjectPropertyExpression::InverseObjectProperty(_)
        )
    }

    /// Validate a data range for OWL 2 EL
    fn validate_data_range(&self, range: &DataRange) -> Result<(), ProfileViolation> {
        match range {
            // Only basic datatypes are allowed in OWL 2 EL
            DataRange::Datatype(_) => Ok(()),

            // Complex data ranges are not allowed
            DataRange::DataIntersectionOf(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedDataRange("DataIntersectionOf".to_string()),
                "Data intersections not allowed in OWL 2 EL",
            )),
            DataRange::DataUnionOf(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedDataRange("DataUnionOf".to_string()),
                "Data unions not allowed in OWL 2 EL",
            )),
            DataRange::DataComplementOf(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedDataRange("DataComplementOf".to_string()),
                "Data complements not allowed in OWL 2 EL",
            )),
            DataRange::DataOneOf(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedDataRange("DataOneOf".to_string()),
                "Data enumerations not allowed in OWL 2 EL",
            )),
            DataRange::DatatypeRestriction { .. } => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedDataRange("DatatypeRestriction".to_string()),
                "Datatype restrictions not allowed in OWL 2 EL",
            )),
        }
    }

    /// Check if an axiom is allowed in OWL 2 EL
    fn validate_axiom(&self, axiom: &Axiom) -> Result<(), ProfileViolation> {
        match axiom {
            // Allowed axioms
            Axiom::Declaration(_) => Ok(()),
            Axiom::SubClassOf(axiom) => {
                self.validate_class_expression(&axiom.subclass)?;
                self.validate_class_expression(&axiom.superclass)?;
                Ok(())
            }
            Axiom::EquivalentClasses(axiom) => {
                for expr in &axiom.classes {
                    self.validate_class_expression(expr)?;
                }
                Ok(())
            }
            Axiom::ClassAssertion(axiom) => {
                self.validate_class_expression(&axiom.class)?;
                Ok(())
            }
            Axiom::ObjectPropertyAssertion(_) => Ok(()),
            Axiom::DataPropertyAssertion(_) => Ok(()),
            Axiom::SubObjectPropertyOf(_) => Ok(()), // Simple property hierarchies allowed
            Axiom::ObjectPropertyDomain(axiom) => {
                self.validate_class_expression(&axiom.domain)?;
                Ok(())
            }
            Axiom::ObjectPropertyRange(axiom) => {
                self.validate_class_expression(&axiom.range)?;
                Ok(())
            }
            Axiom::SubDataPropertyOf(_) => Ok(()),
            Axiom::DataPropertyDomain(axiom) => {
                self.validate_class_expression(&axiom.domain)?;
                Ok(())
            }
            Axiom::DataPropertyRange(_) => Ok(()),

            // Restricted or disallowed axioms
            Axiom::DisjointClasses(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("DisjointClasses".to_string()),
                "Disjoint classes not allowed in OWL 2 EL",
            )),
            Axiom::DisjointUnion(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("DisjointUnion".to_string()),
                "Disjoint union not allowed in OWL 2 EL",
            )),
            Axiom::DisjointObjectProperties(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("DisjointObjectProperties".to_string()),
                "Disjoint object properties not allowed in OWL 2 EL",
            )),
            Axiom::DisjointDataProperties(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("DisjointDataProperties".to_string()),
                "Disjoint data properties not allowed in OWL 2 EL",
            )),
            Axiom::FunctionalObjectProperty(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("FunctionalObjectProperty".to_string()),
                "Functional object properties not allowed in OWL 2 EL",
            )),
            Axiom::InverseFunctionalObjectProperty(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom(
                    "InverseFunctionalObjectProperty".to_string(),
                ),
                "Inverse functional object properties not allowed in OWL 2 EL",
            )),
            Axiom::FunctionalDataProperty(_) => Err(ProfileViolation::new(
                ProfileViolationType::DisallowedAxiom("FunctionalDataProperty".to_string()),
                "Functional data properties not allowed in OWL 2 EL",
            )),

            // Other axioms - allow for now but could be restricted
            _ => Ok(()),
        }
    }
}

impl Default for ELValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileValidator for ELValidator {
    fn validate(&self, ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError> {
        let start_time = Instant::now();
        let mut report = ProfileValidationReport::new(OWL2Profile::EL);
        let mut stats = ValidationStatistics::default();

        // Validate all axioms
        for axiom in ontology.axioms() {
            stats.axioms_checked += 1;

            if let Err(violation) = self.validate_axiom(axiom) {
                let violation = violation.with_axiom_id(axiom.axiom_id());
                report.add_violation(violation);
            }
        }

        stats.duration_ms = start_time.elapsed().as_millis() as u64;
        report.stats = stats;

        Ok(report)
    }

    fn is_class_expression_allowed(&self, expr: &ClassExpression) -> bool {
        self.validate_class_expression(expr).is_ok()
    }

    fn is_property_expression_allowed(&self, expr: &ObjectPropertyExpression) -> bool {
        self.is_simple_property_expression(expr)
    }

    fn is_axiom_allowed(&self, axiom: &Axiom) -> bool {
        self.validate_axiom(axiom).is_ok()
    }

    fn is_data_range_allowed(&self, range: &DataRange) -> bool {
        self.validate_data_range(range).is_ok()
    }

    fn profile(&self) -> OWL2Profile {
        OWL2Profile::EL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{ClassExpression, IRI, Individual};

    #[test]
    fn test_el_allows_intersection() {
        let validator = ELValidator::new();

        let class_a = ClassExpression::class(IRI::new("http://example.org/A"));
        let class_b = ClassExpression::class(IRI::new("http://example.org/B"));
        let intersection = ClassExpression::ObjectIntersectionOf(vec![class_a, class_b]);

        assert!(validator.is_class_expression_allowed(&intersection));
    }

    #[test]
    fn test_el_disallows_union() {
        let validator = ELValidator::new();

        let class_a = ClassExpression::class(IRI::new("http://example.org/A"));
        let class_b = ClassExpression::class(IRI::new("http://example.org/B"));
        let union = ClassExpression::ObjectUnionOf(vec![class_a, class_b]);

        assert!(!validator.is_class_expression_allowed(&union));
    }

    #[test]
    fn test_el_disallows_complement() {
        let validator = ELValidator::new();

        let class_a = ClassExpression::class(IRI::new("http://example.org/A"));
        let complement = ClassExpression::ObjectComplementOf(Box::new(class_a));

        assert!(!validator.is_class_expression_allowed(&complement));
    }

    #[test]
    fn test_el_allows_existential() {
        let validator = ELValidator::new();

        let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasChild"))
                .expect("Failed to create ObjectProperty for test: hasChild"),
        );
        let filler = ClassExpression::class(IRI::new("http://example.org/Person"));
        let existential = ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        };

        assert!(validator.is_class_expression_allowed(&existential));
    }

    #[test]
    fn test_el_disallows_universal() {
        let validator = ELValidator::new();

        let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasChild"))
                .expect("Failed to create ObjectProperty for test: hasChild"),
        );
        let filler = ClassExpression::class(IRI::new("http://example.org/Person"));
        let universal = ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        };

        assert!(!validator.is_class_expression_allowed(&universal));
    }

    #[test]
    fn test_el_allows_self_restrictions() {
        let validator = ELValidator::new();

        let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/knows"))
                .expect("Failed to create ObjectProperty for test: knows"),
        );
        let self_restriction = ClassExpression::ObjectHasSelf { property };

        assert!(validator.is_class_expression_allowed(&self_restriction));
    }

    #[test]
    fn test_el_allows_simple_nominals() {
        let validator = ELValidator::new();

        let individual = Individual::named(IRI::new("http://example.org/john"));
        let nominal = ClassExpression::ObjectOneOf(vec![individual]);

        assert!(validator.is_class_expression_allowed(&nominal));
    }
}
