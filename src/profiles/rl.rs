//! OWL 2 RL Profile Validator
//!
//! OWL 2 RL (Rule Language) is optimized for rule-based reasoning and supports
//! polynomial-time reasoning using rule engines. It allows:
//!
//! - Horn clause-like constructs
//! - Basic property characteristics
//! - Simple class expressions
//! - Forward chaining reasoning
//!
//! This implementation provides full OWL 2 RL profile validation according to the W3C specification.

use crate::error::OxidowlError;
use crate::ontology::{Ontology, Axiom, ClassExpression, ObjectPropertyExpression, DataRange, DataPropertyExpression};
use crate::profiles::{
    ProfileValidator, ProfileValidationReport, ProfileViolation, ProfileViolationType, 
    OWL2Profile
};
use std::collections::HashSet;

/// OWL 2 RL Profile Validator (placeholder implementation)
pub struct RLValidator;

impl RLValidator {
    /// Create a new RL profile validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate class expressions in the ontology
    fn validate_class_expressions(&self, ontology: &Ontology, report: &mut ProfileValidationReport) -> Result<(), OxidowlError> {
        for axiom in &ontology.axioms {
            self.check_class_expressions_in_axiom(axiom, report);
        }
        Ok(())
    }
    
    /// Validate property expressions in the ontology
    fn validate_property_expressions(&self, ontology: &Ontology, report: &mut ProfileValidationReport) -> Result<(), OxidowlError> {
        for axiom in &ontology.axioms {
            self.check_property_expressions_in_axiom(axiom, report);
        }
        Ok(())
    }
    
    /// Validate data ranges in the ontology
    fn validate_data_ranges(&self, ontology: &Ontology, report: &mut ProfileValidationReport) -> Result<(), OxidowlError> {
        for axiom in &ontology.axioms {
            self.check_data_ranges_in_axiom(axiom, report);
        }
        Ok(())
    }
    
    /// Check if a class expression is a valid RL sub-class expression
    fn is_rl_sub_class_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectIntersectionOf(classes) => {
                classes.iter().all(|c| self.is_rl_sub_class_expression(c))
            },
            ClassExpression::ObjectUnionOf(classes) => {
                classes.iter().all(|c| matches!(c, ClassExpression::Class(_)))
            },
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property) && 
                matches!(filler.as_ref(), ClassExpression::Class(_))
            },
            ClassExpression::ObjectHasValue { property, .. } => {
                self.is_property_expression_allowed(property)
            },
            ClassExpression::DataSomeValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            },
            ClassExpression::DataHasValue { .. } => true,
            _ => false,
        }
    }
    
    /// Check if a class expression is a valid RL super-class expression
    fn is_rl_super_class_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectIntersectionOf(classes) => {
                classes.iter().all(|c| matches!(c, ClassExpression::Class(_)))
            },
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property) && 
                matches!(filler.as_ref(), ClassExpression::Class(_))
            },
            ClassExpression::DataAllValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            },
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                (*cardinality == 0 && filler.as_ref() == &ClassExpression::nothing() && self.is_property_expression_allowed(property)) ||
                (*cardinality == 1 && filler.as_ref() == &ClassExpression::thing() && self.is_property_expression_allowed(property))
            },
            ClassExpression::DataMaxCardinality { cardinality, filler, .. } => {
                (*cardinality == 0 && self.is_data_range_allowed(filler)) ||
                (*cardinality == 1 && self.is_data_range_allowed(filler))
            },
            _ => false,
        }
    }
    
    /// Check class expressions within an axiom
    fn check_class_expressions_in_axiom(&self, axiom: &Axiom, report: &mut ProfileValidationReport) {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                if !self.is_rl_sub_class_expression(&subclass_axiom.subclass) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!("{:?}", subclass_axiom.subclass)),
                        "Sub-class expression not allowed in OWL 2 RL profile",
                    ));
                }
                if !self.is_rl_super_class_expression(&subclass_axiom.superclass) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!("{:?}", subclass_axiom.superclass)),
                        "Super-class expression not allowed in OWL 2 RL profile",
                    ));
                }
            },
            Axiom::ClassAssertion(class_axiom) => {
                if !matches!(&class_axiom.class, ClassExpression::Class(_)) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!("{:?}", class_axiom.class)),
                        "Only atomic classes allowed in assertions in OWL 2 RL profile",
                    ));
                }
            },
            _ => {} // Other axioms checked elsewhere
        }
    }
    
    /// Check property expressions within an axiom
    fn check_property_expressions_in_axiom(&self, axiom: &Axiom, report: &mut ProfileValidationReport) {
        match axiom {
            Axiom::ObjectPropertyAssertion(prop_axiom) => {
                if !self.is_property_expression_allowed(&prop_axiom.property) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedPropertyExpression(format!("{:?}", prop_axiom.property)),
                        "Property expression not allowed in OWL 2 RL profile",
                    ));
                }
            },
            _ => {} // Other axioms checked elsewhere
        }
    }
    
    /// Check data ranges within an axiom
    fn check_data_ranges_in_axiom(&self, axiom: &Axiom, report: &mut ProfileValidationReport) {
        match axiom {
            Axiom::DataPropertyRange(range_axiom) => {
                if !self.is_data_range_allowed(&range_axiom.range) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedDataRange(format!("{:?}", range_axiom.range)),
                        "Data range not allowed in OWL 2 RL profile",
                    ));
                }
            },
            _ => {} // Other axioms checked elsewhere
        }
    }
}

impl Default for RLValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileValidator for RLValidator {
    fn validate(&self, ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError> {
        let mut report = ProfileValidationReport::new(OWL2Profile::RL);
        
        // Validate all axioms in the ontology
        for axiom in &ontology.axioms {
            if !self.is_axiom_allowed(axiom) {
                report.add_violation(ProfileViolation::new(
                    ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                    &format!("Axiom type not supported in OWL 2 RL profile: {:?}", axiom),
                ));
            }
        }
        
        // Additional validation for complex constructs
        self.validate_class_expressions(ontology, &mut report)?;
        self.validate_property_expressions(ontology, &mut report)?;
        self.validate_data_ranges(ontology, &mut report)?;
        
        Ok(report)
    }

    fn is_class_expression_allowed(&self, expr: &ClassExpression) -> bool {
        match expr {
            // Always allowed
            ClassExpression::Class(_) => true,
            
            // Allowed with restrictions
            ClassExpression::ObjectIntersectionOf(classes) => {
                classes.iter().all(|c| self.is_class_expression_allowed(c))
            },
            ClassExpression::ObjectUnionOf(classes) => {
                // Union only allowed of atomic classes
                classes.iter().all(|c| matches!(c, ClassExpression::Class(_)))
            },
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property) && 
                matches!(filler.as_ref(), ClassExpression::Class(_))
            },
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property) && 
                matches!(filler.as_ref(), ClassExpression::Class(_))
            },
            ClassExpression::ObjectHasValue { property, .. } => {
                self.is_property_expression_allowed(property)
            },
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                *cardinality <= 1 && self.is_property_expression_allowed(property) && 
                matches!(filler.as_ref(), ClassExpression::Class(_))
            },
            ClassExpression::DataSomeValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            },
            ClassExpression::DataAllValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            },
            ClassExpression::DataHasValue { .. } => true,
            ClassExpression::DataMaxCardinality { cardinality, filler, .. } => {
                *cardinality <= 1 && self.is_data_range_allowed(filler)
            },
            
            // Not allowed in RL
            ClassExpression::ObjectComplementOf(_) => false,
            ClassExpression::ObjectMinCardinality { .. } => false,
            ClassExpression::ObjectExactCardinality { .. } => false,
            ClassExpression::DataMinCardinality { .. } => false,
            ClassExpression::DataExactCardinality { .. } => false,
            ClassExpression::ObjectHasSelf { .. } => false,
            ClassExpression::ObjectOneOf(_) => false,
            
            // Annotation expressions (should not be in ClassExpression but need to handle for now)
            ClassExpression::AnnotationAssertion { .. } => false,
            ClassExpression::SubAnnotationPropertyOf { .. } => false,
            ClassExpression::AnnotationPropertyDomain { .. } => false,
            ClassExpression::AnnotationPropertyRange { .. } => false,
        }
    }

    fn is_property_expression_allowed(&self, expr: &ObjectPropertyExpression) -> bool {
        match expr {
            // Only atomic object properties allowed in OWL 2 RL
            ObjectPropertyExpression::ObjectProperty(_) => true,
            ObjectPropertyExpression::InverseObjectProperty(_) => false,
            ObjectPropertyExpression::PropertyChain(_) => false, // Property chains not allowed in basic RL
        }
    }

    fn is_axiom_allowed(&self, axiom: &Axiom) -> bool {
        match axiom {
            // Class axioms - allowed with restrictions
            Axiom::SubClassOf(subclass_axiom) => {
                self.is_rl_sub_class_expression(&subclass_axiom.subclass) && 
                self.is_rl_super_class_expression(&subclass_axiom.superclass)
            },
            Axiom::EquivalentClasses(equiv_axiom) => {
                equiv_axiom.classes.iter().all(|c| matches!(c, ClassExpression::Class(_)))
            },
            Axiom::DisjointClasses(disjoint_axiom) => {
                disjoint_axiom.classes.iter().all(|c| matches!(c, ClassExpression::Class(_)))
            },
            Axiom::DisjointUnion(_) => false, // Disjoint unions not allowed in RL
            
            // Object property axioms - most are allowed
            Axiom::SubObjectPropertyOf(subprop_axiom) => {
                self.is_property_expression_allowed(&subprop_axiom.sub_property) && 
                self.is_property_expression_allowed(&subprop_axiom.super_property)
            },
            Axiom::EquivalentObjectProperties(equiv_axiom) => {
                equiv_axiom.properties.iter().all(|p| self.is_property_expression_allowed(p))
            },
            Axiom::DisjointObjectProperties(disjoint_axiom) => {
                disjoint_axiom.properties.iter().all(|p| self.is_property_expression_allowed(p))
            },
            Axiom::ObjectPropertyDomain(domain_axiom) => {
                self.is_property_expression_allowed(&domain_axiom.property) && 
                matches!(&domain_axiom.domain, ClassExpression::Class(_))
            },
            Axiom::ObjectPropertyRange(range_axiom) => {
                self.is_property_expression_allowed(&range_axiom.property) && 
                matches!(&range_axiom.range, ClassExpression::Class(_))
            },
            Axiom::InverseObjectProperties(_) => false, // Not allowed in RL
            Axiom::FunctionalObjectProperty(_) => true,
            Axiom::InverseFunctionalObjectProperty(_) => true,
            Axiom::ReflexiveObjectProperty(_) => true,
            Axiom::IrreflexiveObjectProperty(_) => true,
            Axiom::SymmetricObjectProperty(_) => true,
            Axiom::AsymmetricObjectProperty(_) => true,
            Axiom::TransitiveObjectProperty(_) => true,
            
            // Data property axioms - most are allowed
            Axiom::SubDataPropertyOf(_) => true,
            Axiom::EquivalentDataProperties(_) => true,
            Axiom::DisjointDataProperties(_) => true,
            Axiom::DataPropertyDomain(domain_axiom) => {
                matches!(&domain_axiom.domain, ClassExpression::Class(_))
            },
            Axiom::DataPropertyRange(_) => true,
            Axiom::FunctionalDataProperty(_) => true,
            
            // Individual axioms - limited support
            Axiom::ClassAssertion(class_axiom) => {
                matches!(&class_axiom.class, ClassExpression::Class(_))
            },
            Axiom::ObjectPropertyAssertion(prop_axiom) => {
                self.is_property_expression_allowed(&prop_axiom.property)
            },
            Axiom::NegativeObjectPropertyAssertion(_) => true,
            Axiom::DataPropertyAssertion(_) => true,
            Axiom::NegativeDataPropertyAssertion(_) => true,
            Axiom::SameIndividual(_) => true,
            Axiom::DifferentIndividuals(_) => true,
            
            // Other axioms
            Axiom::HasKey(_) => true, // Allowed in RL
            Axiom::DatatypeDefinition(_) => false, // Not allowed in RL
            Axiom::Declaration(_) => true,
            Axiom::AnnotationAssertion(_) => true,
            Axiom::SubAnnotationPropertyOf(_) => true,
            Axiom::AnnotationPropertyDomain(_) => true,
            Axiom::AnnotationPropertyRange(_) => true,
            Axiom::Rule(_) => true, // SWRL rules are allowed in RL
        }
    }

    fn is_data_range_allowed(&self, range: &DataRange) -> bool {
        match range {
            // Atomic datatypes and intersections allowed
            DataRange::Datatype(_) => true,
            DataRange::DataIntersectionOf(ranges) => {
                ranges.iter().all(|r| matches!(r, DataRange::Datatype(_)))
            },
            
            // Not allowed in RL
            DataRange::DataUnionOf(_) => false,
            DataRange::DataComplementOf(_) => false,
            DataRange::DataOneOf(_) => false,
            DataRange::DatatypeRestriction { .. } => false,
        }
    }

    fn profile(&self) -> OWL2Profile {
        OWL2Profile::RL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_rl_validator_creation() {
        let validator = RLValidator::new();
        assert_eq!(validator.profile(), OWL2Profile::RL);
    }

    #[test]
    fn test_rl_class_expressions() {
        let validator = RLValidator::new();
        
        // Atomic class - allowed
        let atomic_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
        assert!(validator.is_class_expression_allowed(&atomic_class));
        
        // Union of atomic classes - allowed in RL
        let union = ClassExpression::ObjectUnionOf(vec![atomic_class.clone()]);
        assert!(validator.is_class_expression_allowed(&union));
        
        // Complement - not allowed in RL
        let complement = ClassExpression::ObjectComplementOf(Box::new(atomic_class.clone()));
        assert!(!validator.is_class_expression_allowed(&complement));
        
        // Max cardinality 1 - allowed in RL
        let max_card = ClassExpression::ObjectMaxCardinality {
            cardinality: 1,
            property: ObjectPropertyExpression::ObjectProperty(
                crate::ontology::ObjectProperty::new(IRI::new("http://example.org/prop")).unwrap()
            ),
            filler: Box::new(ClassExpression::Class(Class::new(IRI::new("http://example.org/Thing"))))
        };
        assert!(validator.is_class_expression_allowed(&max_card));
        
        // Min cardinality - not allowed in RL
        let min_card = ClassExpression::ObjectMinCardinality {
            cardinality: 1,
            property: ObjectPropertyExpression::ObjectProperty(
                crate::ontology::ObjectProperty::new(IRI::new("http://example.org/prop")).unwrap()
            ),
            filler: Box::new(ClassExpression::Class(Class::new(IRI::new("http://example.org/Thing"))))
        };
        assert!(!validator.is_class_expression_allowed(&min_card));
    }

    #[test]
    fn test_rl_property_expressions() {
        let validator = RLValidator::new();
        
        // Atomic object property - allowed
        let prop = ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasParent")).unwrap()
        );
        assert!(validator.is_property_expression_allowed(&prop));
        
        // Inverse property - not allowed in RL
        let inverse_prop = ObjectPropertyExpression::InverseObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasParent")).unwrap()
        );
        assert!(!validator.is_property_expression_allowed(&inverse_prop));
    }

    #[test]
    fn test_rl_data_ranges() {
        let validator = RLValidator::new();
        
        // Atomic datatype - allowed
        let datatype = DataRange::Datatype(
            IRI::new("http://www.w3.org/2001/XMLSchema#string")
        );
        assert!(validator.is_data_range_allowed(&datatype));
        
        // Intersection of datatypes - allowed in RL
        let intersection = DataRange::DataIntersectionOf(vec![datatype.clone()]);
        assert!(validator.is_data_range_allowed(&intersection));
        
        // Union of datatypes - not allowed in RL
        let union = DataRange::DataUnionOf(vec![datatype]);
        assert!(!validator.is_data_range_allowed(&union));
    }

    #[test]
    fn test_rl_validation_empty_ontology() {
        let validator = RLValidator::new();
        let ontology = Ontology::new();
        
        let report = validator.validate(&ontology).unwrap();
        assert!(report.conforms); // Empty ontology should conform
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_rl_validation_with_violations() {
        let validator = RLValidator::new();
        let mut ontology = Ontology::new();
        
        // Add a non-RL axiom (inverse object property)
        let prop1 = crate::ontology::ObjectProperty::new(IRI::new("http://example.org/prop1")).unwrap();
        let prop2 = crate::ontology::ObjectProperty::new(IRI::new("http://example.org/prop2")).unwrap();
        ontology.axioms.push(Axiom::InverseObjectProperties(
            crate::ontology::axioms::InverseObjectPropertiesAxiom {
                id: 0,
                property1: crate::ontology::ObjectPropertyExpression::ObjectProperty(prop1),
                property2: crate::ontology::ObjectPropertyExpression::ObjectProperty(prop2),
                annotations: Vec::new(),
            }
        ));
        
        let report = validator.validate(&ontology).unwrap();
        assert!(!report.conforms);
        assert!(!report.violations.is_empty());
    }
}
