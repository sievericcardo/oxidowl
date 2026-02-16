//! OWL 2 QL Profile Validator
//!
//! OWL 2 QL (Query Language) is optimized for efficient query answering and supports
//! conjunctive query answering in polynomial time. It allows:
//!
//! - Basic class and property hierarchies
//! - Simple existential quantification
//! - Limited intersection
//! - Domain and range restrictions
//!
//! This implementation provides full OWL 2 QL profile validation according to the W3C specification.

use crate::error::OxidowlError;
use crate::ontology::{Axiom, ClassExpression, DataRange, ObjectPropertyExpression, Ontology};
use crate::profiles::{
    OWL2Profile, ProfileValidationReport, ProfileValidator, ProfileViolation, ProfileViolationType,
};
use std::collections::HashSet;

/// OWL 2 QL Profile Validator
///
/// Implements complete OWL 2 QL profile validation according to W3C specification.
/// OWL 2 QL restricts the language to ensure polynomial-time query answering.
pub struct QLValidator {
    /// Prohibited constructs in QL
    prohibited_constructs: HashSet<String>,
}

impl QLValidator {
    /// Create a new QL profile validator
    pub fn new() -> Self {
        let mut prohibited_constructs = HashSet::new();

        // Add constructs prohibited in OWL 2 QL
        prohibited_constructs.insert("ObjectUnionOf".to_string());
        prohibited_constructs.insert("ObjectComplementOf".to_string());
        prohibited_constructs.insert("ObjectOneOf".to_string());
        prohibited_constructs.insert("ObjectHasSelf".to_string());
        prohibited_constructs.insert("ObjectMinCardinality".to_string());
        prohibited_constructs.insert("ObjectMaxCardinality".to_string());
        prohibited_constructs.insert("ObjectExactCardinality".to_string());
        prohibited_constructs.insert("DataUnionOf".to_string());
        prohibited_constructs.insert("DataComplementOf".to_string());
        prohibited_constructs.insert("DataOneOf".to_string());
        prohibited_constructs.insert("DataMinCardinality".to_string());
        prohibited_constructs.insert("DataMaxCardinality".to_string());
        prohibited_constructs.insert("DataExactCardinality".to_string());

        Self {
            prohibited_constructs,
        }
    }

    /// Validate class expressions in the ontology
    fn validate_class_expressions(
        &self,
        ontology: &Ontology,
        report: &mut ProfileValidationReport,
    ) -> Result<(), OxidowlError> {
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(sub_axiom) => {
                    // Check subclass (must be basic class expression)
                    if !self.is_ql_subclass_expression(&sub_axiom.subclass) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                sub_axiom.subclass
                            )),
                            format!(
                                "Invalid subclass expression in QL profile: {:?}",
                                sub_axiom.subclass
                            ),
                        ));
                    }

                    // Check superclass (must be basic or simple existential)
                    if !self.is_ql_superclass_expression(&sub_axiom.superclass) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                sub_axiom.superclass
                            )),
                            format!(
                                "Invalid superclass expression in QL profile: {:?}",
                                sub_axiom.superclass
                            ),
                        ));
                    }
                }
                Axiom::EquivalentClasses(equiv_axiom) => {
                    // All classes must be basic
                    for class_expr in &equiv_axiom.classes {
                        if !self.is_ql_basic_class_expression(class_expr) {
                            report.add_violation(ProfileViolation::new(
                                ProfileViolationType::DisallowedClassExpression(format!(
                                    "{:?}",
                                    class_expr
                                )),
                                format!("Non-basic class in equivalence: {:?}", class_expr),
                            ));
                        }
                    }
                }
                Axiom::DisjointClasses(disjoint_axiom) => {
                    // All classes must be basic
                    for class_expr in &disjoint_axiom.classes {
                        if !self.is_ql_basic_class_expression(class_expr) {
                            report.add_violation(ProfileViolation::new(
                                ProfileViolationType::DisallowedClassExpression(format!(
                                    "{:?}",
                                    class_expr
                                )),
                                format!("Non-basic class in disjointness: {:?}", class_expr),
                            ));
                        }
                    }
                }
                Axiom::ClassAssertion(class_assertion) => {
                    // Class must be basic
                    if !self.is_ql_basic_class_expression(&class_assertion.class) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                class_assertion.class
                            )),
                            format!("Non-basic class in assertion: {:?}", class_assertion.class),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Validate property expressions in the ontology
    fn validate_property_expressions(
        &self,
        ontology: &Ontology,
        report: &mut ProfileValidationReport,
    ) -> Result<(), OxidowlError> {
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubObjectPropertyOf(sub_prop_axiom) => {
                    // Check if property expressions are valid for QL
                    if !self.is_ql_property_expression(&sub_prop_axiom.sub_property) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedPropertyExpression(format!(
                                "{:?}",
                                sub_prop_axiom.sub_property
                            )),
                            format!(
                                "Invalid sub-property expression in QL: {:?}",
                                sub_prop_axiom.sub_property
                            ),
                        ));
                    }

                    if !self.is_ql_property_expression(&sub_prop_axiom.super_property) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedPropertyExpression(format!(
                                "{:?}",
                                sub_prop_axiom.super_property
                            )),
                            format!(
                                "Invalid super-property expression in QL: {:?}",
                                sub_prop_axiom.super_property
                            ),
                        ));
                    }
                }
                Axiom::ObjectPropertyDomain(domain_axiom) => {
                    // Domain must be basic class expression
                    if !self.is_ql_basic_class_expression(&domain_axiom.domain) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                domain_axiom.domain
                            )),
                            format!("Non-basic domain in QL: {:?}", domain_axiom.domain),
                        ));
                    }
                }
                Axiom::ObjectPropertyRange(range_axiom) => {
                    // Range must be basic class expression
                    if !self.is_ql_basic_class_expression(&range_axiom.range) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                range_axiom.range
                            )),
                            format!("Non-basic range in QL: {:?}", range_axiom.range),
                        ));
                    }
                }
                Axiom::DataPropertyDomain(domain_axiom) => {
                    // Domain must be basic class expression
                    if !self.is_ql_basic_class_expression(&domain_axiom.domain) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                domain_axiom.domain
                            )),
                            format!("Non-basic domain in QL: {:?}", domain_axiom.domain),
                        ));
                    }
                }
                Axiom::ClassAssertion(class_axiom) => {
                    // Class must be basic
                    if !self.is_ql_basic_class_expression(&class_axiom.class) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedClassExpression(format!(
                                "{:?}",
                                class_axiom.class
                            )),
                            format!("Non-basic class in assertion: {:?}", class_axiom.class),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Validate data ranges in the ontology
    fn validate_data_ranges(
        &self,
        ontology: &Ontology,
        report: &mut ProfileValidationReport,
    ) -> Result<(), OxidowlError> {
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::DataPropertyRange(range_axiom) => {
                    // Data range must be basic datatype
                    if !self.is_ql_basic_datatype(&range_axiom.range) {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedDataRange(format!(
                                "{:?}",
                                range_axiom.range
                            )),
                            format!("Non-basic datatype in QL range: {:?}", range_axiom.range),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Check for prohibited axiom types in QL
    fn validate_axiom_types(
        &self,
        ontology: &Ontology,
        report: &mut ProfileValidationReport,
    ) -> Result<(), OxidowlError> {
        for axiom in ontology.axioms() {
            match axiom {
                // Property chain axioms are not allowed in QL
                Axiom::SubObjectPropertyOf(sub_prop) => {
                    if let ObjectPropertyExpression::PropertyChain(_) = &sub_prop.sub_property {
                        report.add_violation(ProfileViolation::new(
                            ProfileViolationType::DisallowedAxiom(format!("{:?}", sub_prop)),
                            "Property chains are not allowed in OWL 2 QL".to_string(),
                        ));
                    }
                }

                // Asymmetric property axioms are not allowed
                Axiom::AsymmetricObjectProperty(_) => {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                        "Asymmetric property axioms are not allowed in OWL 2 QL".to_string(),
                    ));
                }

                // Irreflexive property axioms are not allowed
                Axiom::IrreflexiveObjectProperty(_) => {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                        "Irreflexive property axioms are not allowed in OWL 2 QL".to_string(),
                    ));
                }

                // Transitive property axioms are not allowed
                Axiom::TransitiveObjectProperty(_) => {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                        "Transitive property axioms are not allowed in OWL 2 QL".to_string(),
                    ));
                }

                // Has key axioms are not allowed
                Axiom::HasKey(_) => {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                        "HasKey axioms are not allowed in OWL 2 QL".to_string(),
                    ));
                }

                _ => {}
            }
        }
        Ok(())
    }

    /// Check if class expression is valid QL subclass expression
    fn is_ql_subclass_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectIntersectionOf(exprs) => {
                // Intersection of basic class expressions is allowed
                exprs.iter().all(|e| self.is_ql_basic_class_expression(e))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // Simple existential quantification over basic property and owl:Thing
                self.is_ql_basic_property_expression(property)
                    && matches!(filler.as_ref(), ClassExpression::Class(c) if c.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing")
            }
            _ => false,
        }
    }

    /// Check if class expression is valid QL superclass expression  
    fn is_ql_superclass_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // Existential quantification with basic property and basic class
                self.is_ql_basic_property_expression(property)
                    && self.is_ql_basic_class_expression(filler)
            }
            ClassExpression::ObjectComplementOf(inner) => {
                // Complement of basic class or simple existential
                self.is_ql_basic_class_expression(inner)
                    || matches!(inner.as_ref(),
                        ClassExpression::ObjectSomeValuesFrom { property, filler }
                        if self.is_ql_basic_property_expression(property) &&
                           matches!(filler.as_ref(), ClassExpression::Class(c) if c.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing")
                    )
            }
            _ => false,
        }
    }

    /// Check if property expression is valid in QL
    fn is_ql_property_expression(&self, expr: &ObjectPropertyExpression) -> bool {
        match expr {
            ObjectPropertyExpression::ObjectProperty(_) => true,
            ObjectPropertyExpression::InverseObjectProperty(_prop) => {
                // Inverse of basic property is allowed
                true // prop is already an ObjectProperty, so it's basic
            }
            ObjectPropertyExpression::PropertyChain(_) => false, // Not allowed in QL
        }
    }

    /// Check if property expression is basic (atomic property or its inverse)
    fn is_ql_basic_property_expression(&self, expr: &ObjectPropertyExpression) -> bool {
        match expr {
            ObjectPropertyExpression::ObjectProperty(_) => true,
            ObjectPropertyExpression::InverseObjectProperty(_prop) => {
                true // prop is already an ObjectProperty, so it's basic
            }
            _ => false,
        }
    }

    /// Check if datatype is basic (atomic datatype)
    fn is_ql_basic_datatype(&self, data_range: &DataRange) -> bool {
        match data_range {
            DataRange::Datatype(_) => true,
            _ => false, // Datatype restrictions, unions, etc. not allowed in QL
        }
    }

    /// Check if a class expression is a valid QL sub-class expression
    fn is_ql_sub_class_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property)
                    && matches!(filler.as_ref(), ClassExpression::Class(_))
            }
            ClassExpression::DataSomeValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            }
            ClassExpression::ObjectIntersectionOf(classes) => {
                classes.iter().all(|c| self.is_ql_sub_class_expression(c))
            }
            _ => false,
        }
    }

    /// Check if a class expression is a valid QL super-class expression
    fn is_ql_super_class_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.is_property_expression_allowed(property)
                    && matches!(filler.as_ref(), ClassExpression::Class(_))
            }
            ClassExpression::DataSomeValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            }
            _ => false,
        }
    }

    /// Check if a class expression is a basic QL class expression (atomic class)
    fn is_ql_basic_class_expression(&self, expr: &ClassExpression) -> bool {
        matches!(expr, ClassExpression::Class(_))
    }

    /// Check class expressions within an axiom
    fn check_class_expressions_in_axiom(
        &self,
        axiom: &Axiom,
        report: &mut ProfileValidationReport,
    ) {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                if !self.is_ql_sub_class_expression(&subclass_axiom.subclass) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!(
                            "{:?}",
                            subclass_axiom.subclass
                        )),
                        "Sub-class expression not allowed in OWL 2 QL profile".to_string(),
                    ));
                }
                if !self.is_ql_super_class_expression(&subclass_axiom.superclass) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!(
                            "{:?}",
                            subclass_axiom.superclass
                        )),
                        "Super-class expression not allowed in OWL 2 QL profile".to_string(),
                    ));
                }
            }
            Axiom::ClassAssertion(class_axiom) => {
                if !self.is_ql_basic_class_expression(&class_axiom.class) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedClassExpression(format!(
                            "{:?}",
                            class_axiom.class
                        )),
                        "Class expression in assertion not allowed in OWL 2 QL profile".to_string(),
                    ));
                }
            }
            _ => {} // Other axioms checked elsewhere
        }
    }

    /// Check property expressions within an axiom
    fn check_property_expressions_in_axiom(
        &self,
        axiom: &Axiom,
        report: &mut ProfileValidationReport,
    ) {
        match axiom {
            Axiom::ObjectPropertyAssertion(prop_axiom) => {
                if !self.is_property_expression_allowed(&prop_axiom.property) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedPropertyExpression(format!(
                            "{:?}",
                            prop_axiom.property
                        )),
                        "Property expression not allowed in OWL 2 QL profile".to_string(),
                    ));
                }
            }
            _ => {} // Other axioms checked elsewhere
        }
    }

    /// Check data ranges within an axiom
    fn check_data_ranges_in_axiom(&self, axiom: &Axiom, report: &mut ProfileValidationReport) {
        match axiom {
            Axiom::DataPropertyRange(range_axiom) => {
                if !self.is_data_range_allowed(&range_axiom.range) {
                    report.add_violation(ProfileViolation::new(
                        ProfileViolationType::DisallowedDataRange(format!(
                            "{:?}",
                            range_axiom.range
                        )),
                        "Data range not allowed in OWL 2 QL profile".to_string(),
                    ));
                }
            }
            _ => {} // Other axioms checked elsewhere
        }
    }
}

impl Default for QLValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileValidator for QLValidator {
    fn validate(&self, ontology: &Ontology) -> Result<ProfileValidationReport, OxidowlError> {
        let mut report = ProfileValidationReport::new(OWL2Profile::QL);

        // Validate all axioms in the ontology
        for axiom in ontology.axioms() {
            if !self.is_axiom_allowed(axiom) {
                report.add_violation(ProfileViolation::new(
                    ProfileViolationType::DisallowedAxiom(format!("{:?}", axiom)),
                    format!("Axiom type not supported in OWL 2 QL profile: {:?}", axiom),
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
            // Allowed in OWL 2 QL
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // Only atomic properties and classes allowed
                self.is_property_expression_allowed(property)
                    && matches!(filler.as_ref(), ClassExpression::Class(_))
            }
            ClassExpression::DataSomeValuesFrom { filler, .. } => {
                self.is_data_range_allowed(filler)
            }
            ClassExpression::ObjectIntersectionOf(classes) => {
                // Only intersection of atomic classes
                classes
                    .iter()
                    .all(|c| matches!(c, ClassExpression::Class(_)))
            }

            // Not allowed in OWL 2 QL
            ClassExpression::ObjectUnionOf(_) => false,
            ClassExpression::ObjectComplementOf(_) => false,
            ClassExpression::ObjectAllValuesFrom {
                property: _,
                filler: _,
            } => false,
            ClassExpression::ObjectHasValue {
                property: _,
                value: _,
            } => false,
            ClassExpression::ObjectMinCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::ObjectMaxCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::ObjectExactCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::DataAllValuesFrom {
                property: _,
                filler: _,
            } => false,
            ClassExpression::DataHasValue {
                property: _,
                value: _,
            } => false,
            ClassExpression::DataMinCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::DataMaxCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::DataExactCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => false,
            ClassExpression::ObjectHasSelf { property: _ } => false,
            ClassExpression::ObjectOneOf(_) => false,
        }
    }

    fn is_property_expression_allowed(&self, expr: &ObjectPropertyExpression) -> bool {
        match expr {
            // Only atomic object properties allowed in OWL 2 QL
            ObjectPropertyExpression::ObjectProperty(_) => true,
            ObjectPropertyExpression::InverseObjectProperty(_) => false,
            ObjectPropertyExpression::PropertyChain(_) => false, // Property chains not allowed in QL
        }
    }

    fn is_axiom_allowed(&self, axiom: &Axiom) -> bool {
        match axiom {
            // Class axioms
            Axiom::SubClassOf(subclass_axiom) => {
                self.is_ql_sub_class_expression(&subclass_axiom.subclass)
                    && self.is_ql_super_class_expression(&subclass_axiom.superclass)
            }
            Axiom::EquivalentClasses(equiv_axiom) => equiv_axiom
                .classes
                .iter()
                .all(|c| self.is_ql_basic_class_expression(c)),
            Axiom::DisjointClasses(disjoint_axiom) => disjoint_axiom
                .classes
                .iter()
                .all(|c| self.is_ql_basic_class_expression(c)),
            Axiom::DisjointUnion(_) => false, // Disjoint unions not allowed in QL

            // Object property axioms
            Axiom::SubObjectPropertyOf(subprop_axiom) => {
                self.is_property_expression_allowed(&subprop_axiom.sub_property)
                    && self.is_property_expression_allowed(&subprop_axiom.super_property)
            }
            Axiom::EquivalentObjectProperties(equiv_axiom) => equiv_axiom
                .properties
                .iter()
                .all(|p| self.is_property_expression_allowed(p)),
            Axiom::DisjointObjectProperties(disjoint_axiom) => disjoint_axiom
                .properties
                .iter()
                .all(|p| self.is_property_expression_allowed(p)),
            Axiom::ObjectPropertyDomain(domain_axiom) => {
                self.is_property_expression_allowed(&domain_axiom.property)
                    && self.is_ql_basic_class_expression(&domain_axiom.domain)
            }
            Axiom::ObjectPropertyRange(range_axiom) => {
                self.is_property_expression_allowed(&range_axiom.property)
                    && self.is_ql_basic_class_expression(&range_axiom.range)
            }
            Axiom::InverseObjectProperties(_) => false, // Not allowed in QL
            Axiom::FunctionalObjectProperty(_) => false, // Not allowed in QL
            Axiom::InverseFunctionalObjectProperty(_) => false, // Not allowed in QL
            Axiom::ReflexiveObjectProperty(_) => false, // Not allowed in QL
            Axiom::IrreflexiveObjectProperty(_) => false, // Not allowed in QL
            Axiom::SymmetricObjectProperty(_) => false, // Not allowed in QL
            Axiom::AsymmetricObjectProperty(_) => false, // Not allowed in QL
            Axiom::TransitiveObjectProperty(_) => false, // Not allowed in QL

            // Data property axioms
            Axiom::SubDataPropertyOf(_) => true,
            Axiom::EquivalentDataProperties(_) => true,
            Axiom::DisjointDataProperties(_) => true,
            Axiom::DataPropertyDomain(domain_axiom) => {
                self.is_ql_basic_class_expression(&domain_axiom.domain)
            }
            Axiom::DataPropertyRange(_) => true,
            Axiom::FunctionalDataProperty(_) => false, // Not allowed in QL

            // Individual axioms
            Axiom::ClassAssertion(class_axiom) => {
                self.is_ql_basic_class_expression(&class_axiom.class)
            }
            Axiom::ObjectPropertyAssertion(prop_axiom) => {
                self.is_property_expression_allowed(&prop_axiom.property)
            }
            Axiom::NegativeObjectPropertyAssertion(_) => false, // Not allowed in QL
            Axiom::DataPropertyAssertion(_) => true,
            Axiom::NegativeDataPropertyAssertion(_) => false, // Not allowed in QL
            Axiom::SameIndividual(_) => true,
            Axiom::DifferentIndividuals(_) => false, // Not allowed in QL

            // Other axioms
            Axiom::HasKey(_) => false,             // Not allowed in QL
            Axiom::DatatypeDefinition(_) => false, // Not allowed in QL
            Axiom::Declaration(_) => true,
            Axiom::AnnotationAssertion(_) => true,
            Axiom::SubAnnotationPropertyOf(_) => true,
            Axiom::AnnotationPropertyDomain(_) => true,
            Axiom::AnnotationPropertyRange(_) => true,
            Axiom::Rule(_) => false, // SWRL rules not allowed in QL
        }
    }

    fn is_data_range_allowed(&self, data_range: &DataRange) -> bool {
        match data_range {
            DataRange::Datatype(_) => true,
            DataRange::DataIntersectionOf { .. } => false,
            DataRange::DataUnionOf { .. } => false,
            DataRange::DataComplementOf { .. } => false,
            DataRange::DataOneOf { .. } => false,
            DataRange::DatatypeRestriction {
                datatype: _,
                restrictions: _,
            } => false,
        }
    }

    fn profile(&self) -> OWL2Profile {
        OWL2Profile::QL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_ql_validator_creation() {
        let validator = QLValidator::new();
        assert_eq!(validator.profile(), OWL2Profile::QL);
    }

    #[test]
    fn test_ql_basic_class_expressions() {
        let validator = QLValidator::new();

        // Atomic class - allowed
        let atomic_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
        assert!(validator.is_class_expression_allowed(&atomic_class));

        // Union - not allowed in QL
        let union = ClassExpression::ObjectUnionOf(vec![atomic_class.clone()]);
        assert!(!validator.is_class_expression_allowed(&union));

        // Complement - not allowed in QL
        let complement = ClassExpression::ObjectComplementOf(Box::new(atomic_class.clone()));
        assert!(!validator.is_class_expression_allowed(&complement));
    }

    #[test]
    fn test_ql_property_expressions() {
        let validator = QLValidator::new();

        // Atomic object property - allowed
        let prop = ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasParent"))
                .expect("Failed to create ObjectProperty for test: hasParent"),
        );
        assert!(validator.is_property_expression_allowed(&prop));

        // Inverse property - not allowed in QL
        let inverse_prop = ObjectPropertyExpression::InverseObjectProperty(
            crate::ontology::ObjectProperty::new(IRI::new("http://example.org/hasParent"))
                .expect("Failed to create ObjectProperty for test: hasParent"),
        );
        assert!(!validator.is_property_expression_allowed(&inverse_prop));
    }

    #[test]
    fn test_ql_data_ranges() {
        let validator = QLValidator::new();

        // Atomic datatype - allowed
        let datatype = DataRange::Datatype(IRI::new("http://www.w3.org/2001/XMLSchema#string"));
        assert!(validator.is_data_range_allowed(&datatype));

        // Union of datatypes - not allowed in QL
        let union = DataRange::DataUnionOf(vec![datatype]);
        assert!(!validator.is_data_range_allowed(&union));
    }

    #[test]
    fn test_ql_validation_empty_ontology() {
        let validator = QLValidator::new();
        let ontology = Ontology::new();

        let report = validator
            .validate(&ontology)
            .expect("Failed to validate ontology against OWL 2 profile");
        assert!(report.conforms); // Empty ontology should conform
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_ql_validation_with_violations() {
        let validator = QLValidator::new();
        let mut ontology = Ontology::new();

        // Add a non-QL axiom (functional property)
        let prop = crate::ontology::ObjectProperty::new(IRI::new("http://example.org/prop"))
            .expect("Failed to create ObjectProperty for test: prop");
        ontology.axioms.push(Axiom::FunctionalObjectProperty(
            crate::ontology::axioms::FunctionalObjectPropertyAxiom {
                id: 0,
                property: crate::ontology::ObjectPropertyExpression::ObjectProperty(prop),
                annotations: Vec::new(),
            },
        ));

        let report = validator
            .validate(&ontology)
            .expect("Failed to validate ontology against OWL 2 profile");
        assert!(!report.conforms);
        assert!(!report.violations.is_empty());
    }
}
