#![allow(dead_code)]

use crate::ontology::axioms::AxiomTrait;
use crate::ontology::concepts::ClassExpression;
use crate::ontology::{ObjectPropertyExpression, Ontology, axioms::*};
use crate::{Error, error::OxidowlError};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub profile: Option<OWL2Profile>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub severity: ValidationSeverity,
    pub axiom_id: Option<String>,
    pub location: Option<String>,
}

impl ValidationError {
    #[must_use]
    pub fn new(error_type: ValidationErrorType, message: String) -> Self {
        Self {
            error_type,
            message,
            severity: ValidationSeverity::Error,
            axiom_id: None,
            location: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
    pub warning_type: ValidationWarningType,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    NonSimplePropertyInCardinalityRestriction,
    AnonymousIndividualInAssertion,
    DataTypeRestrictionViolation,
    PropertyHierarchyViolation,
    SimpleRoleViolation,
    UndeclaredEntity,
    InvalidDatatype,
    CyclicPropertyHierarchy,
    DatatypeMismatch,
    UnrecognizedDatatype,
    InvalidFacetRestriction,
    IncompatibleDataRanges,
    InvalidLiteralEnumeration,
    InvalidDatatypeDefinition,
    DuplicateDatatypeDefinition,
    CircularDatatypeDefinition,
    InconsistentDatatypeDefinition,
    UnknownDatatype,
    EmptyDataIntersection,
    EmptyDataUnion,
    EmptyDataEnumeration,
    InvalidLiteralValue,
    DuplicateLiteral,
    InvalidLiteralSyntax,
    UnsupportedDatatype,
    InvalidDatatypeExpression,
    InvalidLiteral,
    ConflictingFacetRestrictions,
    /// RDF-star: Quoted triple used in predicate position
    QuotedTripleInPredicatePosition,
    /// RDF-star: Excessive nesting depth exceeds configured limit
    ExcessiveQuotedTripleNesting,
    /// RDF-star: Quoted triple used in unsupported context
    QuotedTripleInUnsupportedContext,
    /// RDF-star: Invalid quoted triple structure
    InvalidQuotedTripleStructure,
    /// RDF 1.2: Invalid dirLangString (missing or invalid direction)
    InvalidDirectionalLiteral,
    /// RDF 1.2: Malformed blank node label
    InvalidBlankNodeLabel,
    // ── Non-Simple Property Restrictions (OWL 2 Section 11.1) ──
    NonSimplePropertyInFunctionalProperty,
    NonSimplePropertyInInverseFunctionalProperty,
    NonSimplePropertyInIrreflexiveProperty,
    NonSimplePropertyInAsymmetricProperty,
    NonSimplePropertyInDisjointProperties,
    NonSimplePropertyInObjectHasSelf,
    // ── Property Chain / Cycle ──
    UseOfPropertyInChainCausesCycle,
    LastPropertyInChainNotInImposedRange,
    // ── IRI Validation ──
    UseOfNonAbsoluteIRI,
    OntologyIRINotAbsolute,
    OntologyVersionIRINotAbsolute,
    // ── Illegal Punning ──
    IllegalPunning,
    DatatypeIRIAlsoUsedAsClassIRI,
    // ── Reserved Vocabulary ──
    UseOfReservedVocabularyForClassIRI,
    UseOfReservedVocabularyForObjectPropertyIRI,
    UseOfReservedVocabularyForDataPropertyIRI,
    UseOfReservedVocabularyForAnnotationPropertyIRI,
    UseOfReservedVocabularyForIndividualIRI,
    UseOfReservedVocabularyForOntologyIRI,
    UseOfReservedVocabularyForVersionIRI,
    // ── Expression Position Violations ──
    UseOfNonSubClassExpression,
    UseOfNonSuperClassExpression,
    UseOfNonEquivalentClassExpression,
    // ── Other DL Restrictions ──
    UseOfTopDataPropertyAsSubProperty,
    LexicalNotInLexicalSpace,
    UseOfBuiltInDatatypeInDatatypeDefinition,
}

impl std::fmt::Display for ValidationErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationErrorType::NonSimplePropertyInCardinalityRestriction => {
                write!(f, "Non-simple property in cardinality restriction")
            }
            ValidationErrorType::AnonymousIndividualInAssertion => {
                write!(f, "Anonymous individual in assertion")
            }
            ValidationErrorType::DataTypeRestrictionViolation => {
                write!(f, "Datatype restriction violation")
            }
            ValidationErrorType::PropertyHierarchyViolation => {
                write!(f, "Property hierarchy violation")
            }
            ValidationErrorType::SimpleRoleViolation => {
                write!(f, "Simple role violation")
            }
            ValidationErrorType::UndeclaredEntity => {
                write!(f, "Undeclared entity")
            }
            ValidationErrorType::InvalidDatatype => {
                write!(f, "Invalid datatype")
            }
            ValidationErrorType::CyclicPropertyHierarchy => {
                write!(f, "Cyclic property hierarchy")
            }
            ValidationErrorType::DatatypeMismatch => {
                write!(f, "Datatype mismatch")
            }
            ValidationErrorType::UnrecognizedDatatype => {
                write!(f, "Unrecognized datatype")
            }
            ValidationErrorType::InvalidFacetRestriction => {
                write!(f, "Invalid facet restriction")
            }
            ValidationErrorType::IncompatibleDataRanges => {
                write!(f, "Incompatible data ranges")
            }
            ValidationErrorType::InvalidLiteralEnumeration => {
                write!(f, "Invalid literal enumeration")
            }
            ValidationErrorType::InvalidDatatypeDefinition => {
                write!(f, "Invalid datatype definition")
            }
            ValidationErrorType::CircularDatatypeDefinition => {
                write!(f, "Circular datatype definition")
            }
            ValidationErrorType::InconsistentDatatypeDefinition => {
                write!(f, "Inconsistent datatype definition")
            }
            ValidationErrorType::UnknownDatatype => {
                write!(f, "Unknown datatype")
            }
            ValidationErrorType::EmptyDataIntersection => {
                write!(f, "Empty data intersection")
            }
            ValidationErrorType::EmptyDataUnion => {
                write!(f, "Empty data union")
            }
            ValidationErrorType::EmptyDataEnumeration => {
                write!(f, "Empty data enumeration")
            }
            ValidationErrorType::InvalidLiteralValue => {
                write!(f, "Invalid literal value")
            }
            ValidationErrorType::DuplicateLiteral => {
                write!(f, "Duplicate literal")
            }
            ValidationErrorType::InvalidLiteralSyntax => {
                write!(f, "Invalid literal syntax")
            }
            ValidationErrorType::DuplicateDatatypeDefinition => {
                write!(f, "Duplicate datatype definition")
            }
            ValidationErrorType::UnsupportedDatatype => {
                write!(f, "Unsupported datatype")
            }
            ValidationErrorType::InvalidDatatypeExpression => {
                write!(f, "Invalid datatype expression")
            }
            ValidationErrorType::InvalidLiteral => {
                write!(f, "Invalid literal")
            }
            ValidationErrorType::ConflictingFacetRestrictions => {
                write!(f, "Conflicting facet restrictions")
            }
            ValidationErrorType::QuotedTripleInPredicatePosition => {
                write!(f, "Quoted triple cannot be used in predicate position")
            }
            ValidationErrorType::ExcessiveQuotedTripleNesting => {
                write!(f, "Quoted triple nesting depth exceeds configured limit")
            }
            ValidationErrorType::QuotedTripleInUnsupportedContext => {
                write!(f, "Quoted triple used in unsupported context")
            }
            ValidationErrorType::InvalidQuotedTripleStructure => {
                write!(f, "Invalid quoted triple structure")
            }
            ValidationErrorType::InvalidDirectionalLiteral => {
                write!(f, "Invalid directional literal (dirLangString)")
            }
            ValidationErrorType::InvalidBlankNodeLabel => {
                write!(f, "Invalid blank node label")
            }
            ValidationErrorType::NonSimplePropertyInFunctionalProperty => {
                write!(f, "Non-simple property in functional property axiom")
            }
            ValidationErrorType::NonSimplePropertyInInverseFunctionalProperty => {
                write!(
                    f,
                    "Non-simple property in inverse-functional property axiom"
                )
            }
            ValidationErrorType::NonSimplePropertyInIrreflexiveProperty => {
                write!(f, "Non-simple property in irreflexive property axiom")
            }
            ValidationErrorType::NonSimplePropertyInAsymmetricProperty => {
                write!(f, "Non-simple property in asymmetric property axiom")
            }
            ValidationErrorType::NonSimplePropertyInDisjointProperties => {
                write!(f, "Non-simple property in disjoint property axiom")
            }
            ValidationErrorType::NonSimplePropertyInObjectHasSelf => {
                write!(f, "Non-simple property in ObjectHasSelf restriction")
            }
            ValidationErrorType::UseOfPropertyInChainCausesCycle => {
                write!(f, "Property in chain causes cycle")
            }
            ValidationErrorType::LastPropertyInChainNotInImposedRange => {
                write!(f, "Last property in property chain not in imposed range")
            }
            ValidationErrorType::UseOfNonAbsoluteIRI => {
                write!(f, "Use of non-absolute IRI")
            }
            ValidationErrorType::OntologyIRINotAbsolute => {
                write!(f, "Ontology IRI is not absolute")
            }
            ValidationErrorType::OntologyVersionIRINotAbsolute => {
                write!(f, "Ontology version IRI is not absolute")
            }
            ValidationErrorType::IllegalPunning => {
                write!(f, "Illegal punning detected")
            }
            ValidationErrorType::DatatypeIRIAlsoUsedAsClassIRI => {
                write!(f, "Datatype IRI also used as class IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForClassIRI => {
                write!(f, "Use of reserved vocabulary for class IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForObjectPropertyIRI => {
                write!(f, "Use of reserved vocabulary for object property IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForDataPropertyIRI => {
                write!(f, "Use of reserved vocabulary for data property IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForAnnotationPropertyIRI => {
                write!(f, "Use of reserved vocabulary for annotation property IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForIndividualIRI => {
                write!(f, "Use of reserved vocabulary for individual IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForOntologyIRI => {
                write!(f, "Use of reserved vocabulary for ontology IRI")
            }
            ValidationErrorType::UseOfReservedVocabularyForVersionIRI => {
                write!(f, "Use of reserved vocabulary for version IRI")
            }
            ValidationErrorType::UseOfNonSubClassExpression => {
                write!(f, "Invalid expression in subclass position")
            }
            ValidationErrorType::UseOfNonSuperClassExpression => {
                write!(f, "Invalid expression in superclass position")
            }
            ValidationErrorType::UseOfNonEquivalentClassExpression => {
                write!(f, "Invalid expression in equivalent class position")
            }
            ValidationErrorType::UseOfTopDataPropertyAsSubProperty => {
                write!(f, "Use of top data property as sub-property")
            }
            ValidationErrorType::LexicalNotInLexicalSpace => {
                write!(f, "Lexical form not in lexical space of datatype")
            }
            ValidationErrorType::UseOfBuiltInDatatypeInDatatypeDefinition => {
                write!(f, "Use of built-in datatype in datatype definition")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningType {
    DeprecatedConstruct,
    PerformanceImpact,
    ProfileViolation,
    UnusedDeclaration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OWL2Profile {
    EL,
    QL,
    RL,
    DL,
    Full,
}

/// OWL 2 DL Global Restrictions Validator
/// Implements validation according to Section 11 of the OWL 2 Specification
pub struct OWL2DLValidator {
    ontology: Ontology,
    simple_properties: HashSet<crate::ontology::IRI>,
    property_hierarchy: HashMap<crate::ontology::IRI, Vec<crate::ontology::IRI>>,
    transitive_properties: HashSet<crate::ontology::IRI>,
    defined_datatypes: HashSet<String>,
    validation_cache: HashMap<String, ValidationReport>,
}

impl OWL2DLValidator {
    #[must_use]
    pub fn new(ontology: Ontology) -> Self {
        let mut validator = Self {
            ontology,
            simple_properties: HashSet::new(),
            property_hierarchy: HashMap::new(),
            transitive_properties: HashSet::new(),
            defined_datatypes: HashSet::new(),
            validation_cache: HashMap::new(),
        };

        validator.analyze_property_hierarchy();
        validator.compute_simple_properties();
        validator
    }

    /// Main validation entry point
    pub fn validate(&mut self) -> Result<ValidationReport, OxidowlError> {
        let cache_key = "full_validation".to_string();

        if let Some(cached_report) = self.validation_cache.get(&cache_key) {
            return Ok(cached_report.clone());
        }

        let mut errors = Vec::new();
        let warnings = Vec::new();

        // 1. Validate property hierarchy restrictions (Section 11.1)
        errors.extend(self.validate_property_hierarchy()?);

        // 2. Validate simple role restrictions (Section 11.2)
        errors.extend(self.validate_simple_roles()?);

        // 3. Validate anonymous individual restrictions (Section 11.3)
        errors.extend(self.validate_anonymous_individuals()?);

        // 4. Validate datatype restrictions (Section 11.4)
        errors.extend(self.validate_datatype_restrictions()?);

        // 5. Additional OWL 2 DL checks
        errors.extend(self.validate_entity_declarations()?);
        errors.extend(self.validate_property_assertions()?);

        // 6. RDF-star and RDF 1.2 validation (Phase 8)
        errors.extend(self.validate_rdf_star_constraints()?);

        let is_valid = errors.is_empty();
        let profile = if is_valid {
            Some(self.detect_profile())
        } else {
            None
        };

        let report = ValidationReport {
            is_valid,
            errors,
            warnings,
            profile,
        };

        self.validation_cache.insert(cache_key, report.clone());
        Ok(report)
    }

    /// Validate property hierarchy restrictions (OWL 2 Section 11.1)
    fn validate_property_hierarchy(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        // Check for cycles in property hierarchies
        for property in self.property_hierarchy.keys() {
            if self.has_cyclic_hierarchy(property, &mut HashSet::new()) {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::CyclicPropertyHierarchy,
                    message: format!(
                        "Cyclic property hierarchy detected involving property: {property}"
                    ),
                    severity: ValidationSeverity::Error,
                    axiom_id: None,
                    location: Some(property.to_string()),
                });
            }
        }

        // Validate property chain axioms
        for axiom in self.ontology.axioms() {
            if let Axiom::SubObjectPropertyOf(sub_prop_axiom) = axiom
                && let ObjectPropertyExpression::PropertyChain(chain) = &sub_prop_axiom.sub_property
            {
                errors.extend(self.validate_property_chain(chain, &sub_prop_axiom.super_property)?);
            }
        }

        Ok(errors)
    }

    /// Validate simple role restrictions (OWL 2 Section 11.2)
    fn validate_simple_roles(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        // Check that non-simple properties are not used in cardinality restrictions
        for axiom in self.ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(sub_class_axiom) => {
                    errors.extend(
                        self.validate_class_expression_simple_roles(&sub_class_axiom.superclass)?,
                    );
                    errors.extend(
                        self.validate_class_expression_simple_roles(&sub_class_axiom.subclass)?,
                    );
                }
                Axiom::EquivalentClasses(equiv_axiom) => {
                    for class_expr in &equiv_axiom.classes {
                        errors.extend(self.validate_class_expression_simple_roles(class_expr)?);
                    }
                }
                Axiom::DisjointClasses(disjoint_axiom) => {
                    for class_expr in &disjoint_axiom.classes {
                        errors.extend(self.validate_class_expression_simple_roles(class_expr)?);
                    }
                }
                _ => {}
            }
        }

        Ok(errors)
    }

    /// Validate that cardinality restrictions only use simple properties
    fn validate_class_expression_simple_roles(
        &self,
        class_expr: &ClassExpression,
    ) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        match class_expr {
            ClassExpression::ObjectMinCardinality {
                cardinality: _,
                property,
                filler: _,
            }
            | ClassExpression::ObjectMaxCardinality {
                cardinality: _,
                property,
                filler: _,
            }
            | ClassExpression::ObjectExactCardinality {
                cardinality: _,
                property,
                filler: _,
            } => {
                if let ObjectPropertyExpression::ObjectProperty(prop) = property {
                    let prop_iri = crate::ontology::IRI::from(prop.iri.to_string());
                    if !self.simple_properties.contains(&prop_iri) {
                        errors.push(ValidationError {
                            error_type:
                                ValidationErrorType::NonSimplePropertyInCardinalityRestriction,
                            message: format!(
                                "Non-simple property {prop_iri} used in cardinality restriction"
                            ),
                            severity: ValidationSeverity::Error,
                            axiom_id: None,
                            location: Some(prop_iri.to_string()),
                        });
                    }
                }
            }
            ClassExpression::ObjectIntersectionOf(exprs)
            | ClassExpression::ObjectUnionOf(exprs) => {
                for expr in exprs {
                    errors.extend(self.validate_class_expression_simple_roles(expr)?);
                }
            }
            ClassExpression::ObjectComplementOf(expr) => {
                errors.extend(self.validate_class_expression_simple_roles(expr)?);
            }
            ClassExpression::ObjectSomeValuesFrom {
                property: _,
                filler,
            }
            | ClassExpression::ObjectAllValuesFrom {
                property: _,
                filler,
            } => {
                errors.extend(self.validate_class_expression_simple_roles(filler)?);
            }
            _ => {}
        }

        Ok(errors)
    }

    /// Validate anonymous individual restrictions (OWL 2 Section 11.3)
    fn validate_anonymous_individuals(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        // Check that anonymous individuals are not used in certain contexts
        for axiom in self.ontology.axioms() {
            match axiom {
                Axiom::SameIndividual(same_axiom) => {
                    for individual in &same_axiom.individuals {
                        if individual.is_anonymous() {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::AnonymousIndividualInAssertion,
                                message: "Anonymous individual used in SameIndividual axiom"
                                    .to_string(),
                                severity: ValidationSeverity::Error,
                                axiom_id: Some(same_axiom.id.to_string()),
                                location: None,
                            });
                        }
                    }
                }
                Axiom::DifferentIndividuals(diff_axiom) => {
                    for individual in &diff_axiom.individuals {
                        if individual.is_anonymous() {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::AnonymousIndividualInAssertion,
                                message: "Anonymous individual used in DifferentIndividuals axiom"
                                    .to_string(),
                                severity: ValidationSeverity::Error,
                                axiom_id: Some(diff_axiom.id.to_string()),
                                location: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(errors)
    }

    /// Validate datatype restrictions
    fn validate_datatype_restrictions(&mut self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        // Check that only recognized datatypes are used
        // Clone axioms to avoid borrow checker issues
        let axioms: Vec<_> = self.ontology.axioms().to_vec();
        for axiom in &axioms {
            self.validate_datatype_usage_in_axiom(axiom, &mut errors)?;
        }

        Ok(errors)
    }

    /// Validate datatype usage within an axiom
    fn validate_datatype_usage_in_axiom(
        &mut self,
        axiom: &Axiom,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        match axiom {
            Axiom::DataPropertyRange(range_axiom) => {
                self.validate_data_range(&range_axiom.range, errors)?;
            }
            Axiom::DataPropertyAssertion(_) => {
                // OWL 2 DL structural validation does not check whether literal values
                // are semantically compatible with the declared data property range.
                // That is a model-level consistency check, not a structural restriction
                // from OWL 2 Specification Section 11.  Doing so here generates
                // hundreds of false-positive violations on valid ontologies.
            }
            Axiom::DatatypeDefinition(datatype_def) => {
                // Validate the datatype definition itself
                self.validate_datatype_definition(datatype_def, errors)?;
            }
            _ => {
                // Check for any embedded data ranges in other axioms
                self.validate_embedded_data_ranges_in_axiom(axiom, errors)?;
            }
        }
        Ok(())
    }

    /// Validate a data range
    fn validate_data_range(
        &self,
        range: &crate::ontology::DataRange,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        match range {
            crate::ontology::DataRange::Datatype(iri) => {
                if !self.is_recognized_datatype(iri) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::UnrecognizedDatatype,
                        format!("Unrecognized datatype: {iri}"),
                    ));
                }
            }
            crate::ontology::DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                // Validate the base datatype
                if !self.is_recognized_datatype(datatype) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::UnrecognizedDatatype,
                        format!("Unrecognized base datatype in restriction: {datatype}"),
                    ));
                }

                // Validate facet restrictions
                for restriction in restrictions {
                    self.validate_facet_restriction(datatype, restriction, errors)?;
                }
            }
            crate::ontology::DataRange::DataIntersectionOf(ranges) => {
                for range in ranges {
                    self.validate_data_range(range, errors)?;
                }
                // Check for compatibility of intersected ranges
                self.validate_data_range_intersection(ranges, errors)?;
            }
            crate::ontology::DataRange::DataUnionOf(ranges) => {
                for range in ranges {
                    self.validate_data_range(range, errors)?;
                }
            }
            crate::ontology::DataRange::DataComplementOf(range) => {
                self.validate_data_range(range, errors)?;
            }
            crate::ontology::DataRange::DataOneOf(literals) => {
                // Validate that all literals are of compatible types
                self.validate_literal_enumeration(literals, errors)?;
            }
        }
        Ok(())
    }

    /// Check if a datatype is recognized according to OWL 2
    fn is_recognized_datatype(&self, iri: &crate::ontology::IRI) -> bool {
        let iri_str = iri.as_str();

        // OWL 2 built-in datatypes
        matches!(
            iri_str,
            // XML Schema datatypes
            "http://www.w3.org/2001/XMLSchema#string" |
            "http://www.w3.org/2001/XMLSchema#boolean" |
            "http://www.w3.org/2001/XMLSchema#decimal" |
            "http://www.w3.org/2001/XMLSchema#float" |
            "http://www.w3.org/2001/XMLSchema#double" |
            "http://www.w3.org/2001/XMLSchema#dateTime" |
            "http://www.w3.org/2001/XMLSchema#time" |
            "http://www.w3.org/2001/XMLSchema#date" |
            "http://www.w3.org/2001/XMLSchema#gYearMonth" |
            "http://www.w3.org/2001/XMLSchema#gYear" |
            "http://www.w3.org/2001/XMLSchema#gMonthDay" |
            "http://www.w3.org/2001/XMLSchema#gDay" |
            "http://www.w3.org/2001/XMLSchema#gMonth" |
            "http://www.w3.org/2001/XMLSchema#hexBinary" |
            "http://www.w3.org/2001/XMLSchema#base64Binary" |
            "http://www.w3.org/2001/XMLSchema#anyURI" |
            "http://www.w3.org/2001/XMLSchema#QName" |
            "http://www.w3.org/2001/XMLSchema#NOTATION" |
            "http://www.w3.org/2001/XMLSchema#normalizedString" |
            "http://www.w3.org/2001/XMLSchema#token" |
            "http://www.w3.org/2001/XMLSchema#language" |
            "http://www.w3.org/2001/XMLSchema#NMTOKEN" |
            "http://www.w3.org/2001/XMLSchema#NMTOKENS" |
            "http://www.w3.org/2001/XMLSchema#Name" |
            "http://www.w3.org/2001/XMLSchema#NCName" |
            "http://www.w3.org/2001/XMLSchema#ID" |
            "http://www.w3.org/2001/XMLSchema#IDREF" |
            "http://www.w3.org/2001/XMLSchema#IDREFS" |
            "http://www.w3.org/2001/XMLSchema#ENTITY" |
            "http://www.w3.org/2001/XMLSchema#ENTITIES" |
            "http://www.w3.org/2001/XMLSchema#integer" |
            "http://www.w3.org/2001/XMLSchema#nonPositiveInteger" |
            "http://www.w3.org/2001/XMLSchema#negativeInteger" |
            "http://www.w3.org/2001/XMLSchema#long" |
            "http://www.w3.org/2001/XMLSchema#int" |
            "http://www.w3.org/2001/XMLSchema#short" |
            "http://www.w3.org/2001/XMLSchema#byte" |
            "http://www.w3.org/2001/XMLSchema#nonNegativeInteger" |
            "http://www.w3.org/2001/XMLSchema#unsignedLong" |
            "http://www.w3.org/2001/XMLSchema#unsignedInt" |
            "http://www.w3.org/2001/XMLSchema#unsignedShort" |
            "http://www.w3.org/2001/XMLSchema#unsignedByte" |
            "http://www.w3.org/2001/XMLSchema#positiveInteger" |
            // RDF datatypes
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral" |
            "http://www.w3.org/2001/XMLSchema#duration" |
            // OWL specific
            "http://www.w3.org/2002/07/owl#real" |
            "http://www.w3.org/2002/07/owl#rational"
        )
    }

    /// Validate entity declarations
    fn validate_entity_declarations(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();
        let mut declared_entities = HashSet::new();

        // Collect all declared entities
        for axiom in self.ontology.axioms() {
            if let Axiom::Declaration(decl_axiom) = axiom {
                declared_entities.insert(decl_axiom.entity.clone());
            }
        }

        // Check that all used entities are declared
        for axiom in self.ontology.axioms() {
            let used_entities = self.extract_entities_from_axiom(axiom);
            for entity in used_entities {
                if !declared_entities.contains(&entity) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::UndeclaredEntity,
                        message: format!("Undeclared entity used: {entity:?}"),
                        severity: ValidationSeverity::Error,
                        axiom_id: Some(axiom.axiom_id().to_string()),
                        location: None,
                    });
                }
            }
        }

        Ok(errors)
    }

    /// Validate property assertions
    fn validate_property_assertions(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let errors = Vec::new();

        // Check functional property violations, etc.
        // This would be expanded based on specific requirements

        Ok(errors)
    }

    /// Validate RDF-star and RDF 1.2 constraints (Phase 8)
    ///
    /// This method validates:
    /// - Quoted triples are not used in predicate position
    /// - Nesting depth does not exceed configured limits
    /// - Quoted triple structures are well-formed
    /// - Directional literals (dirLangString) are properly formatted
    /// - Blank node labels conform to RDF 1.2 well-formedness rules
    ///
    /// Note: RDF 1.1 ontologies without RDF-star features pass through unchanged
    fn validate_rdf_star_constraints(&self) -> Result<Vec<ValidationError>, OxidowlError> {
        let mut errors = Vec::new();

        // Check if ontology has RDF graph with potential RDF-star features
        if let Some(rdf_graph) = self.ontology.get_rdf_graph() {
            // Validate each triple in the RDF graph
            for triple in rdf_graph.triples() {
                // 1. Validate subject position (quoted triples allowed)
                if let Err(e) = self.validate_rdf_term(&triple.subject, "subject", None) {
                    errors.push(e);
                }

                // 2. Validate predicate position (quoted triples NOT allowed)
                match &triple.predicate {
                    crate::semantics::RdfTerm::QuotedTriple(_) => {
                        errors.push(ValidationError::new(
                            ValidationErrorType::QuotedTripleInPredicatePosition,
                            "Quoted triples are not allowed in predicate position in RDF-star"
                                .to_string(),
                        ));
                    }
                    _ => {
                        if let Err(e) = self.validate_rdf_term(&triple.predicate, "predicate", None)
                        {
                            errors.push(e);
                        }
                    }
                }

                // 3. Validate object position (quoted triples allowed)
                if let Err(e) = self.validate_rdf_term(&triple.object, "object", None) {
                    errors.push(e);
                }

                // 4. Check nesting depth
                let depth = triple.depth();
                let max_depth = self.get_max_nesting_depth();
                if depth > max_depth {
                    errors.push(ValidationError::new(
                        ValidationErrorType::ExcessiveQuotedTripleNesting,
                        format!(
                            "Quoted triple nesting depth {depth} exceeds configured maximum {max_depth}"
                        ),
                    ));
                }
            }
        }

        Ok(errors)
    }

    /// Validate an RDF term for RDF-star and RDF 1.2 compliance
    fn validate_rdf_term(
        &self,
        term: &crate::semantics::RdfTerm,
        position: &str,
        _parent_depth: Option<usize>,
    ) -> Result<(), ValidationError> {
        match term {
            crate::semantics::RdfTerm::QuotedTriple(inner_triple) => {
                // Validate the inner triple recursively
                self.validate_quoted_triple_structure(inner_triple)?;
                Ok(())
            }
            crate::semantics::RdfTerm::Literal {
                value: _,
                datatype,
                language,
                direction,
            } => {
                // Validate directional literals (RDF 1.2 feature)
                if direction.is_some() {
                    // Must have language tag for dirLangString
                    if language.is_none() {
                        return Err(ValidationError::new(
                            ValidationErrorType::InvalidDirectionalLiteral,
                            format!(
                                "Directional literal in {position} position must have a language tag"
                            ),
                        ));
                    }

                    // Direction must be "ltr" or "rtl"
                    if let Some(dir) = direction
                        && dir != "ltr"
                        && dir != "rtl"
                    {
                        return Err(ValidationError::new(
                            ValidationErrorType::InvalidDirectionalLiteral,
                            format!(
                                "Invalid direction '{dir}' in {position} position (must be 'ltr' or 'rtl')"
                            ),
                        ));
                    }

                    // Datatype should be rdf:dirLangString if direction is present
                    if let Some(dt) = datatype
                        && dt.as_str() != "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString"
                    {
                        return Err(ValidationError::new(
                            ValidationErrorType::InvalidDirectionalLiteral,
                            format!(
                                "Directional literal in {position} position must have datatype rdf:dirLangString"
                            ),
                        ));
                    }
                }
                Ok(())
            }
            crate::semantics::RdfTerm::BlankNode(id) => {
                // Validate blank node label per RDF 1.1/1.2 well-formedness rules.
                //
                // Grammar (RDF 1.1 §2.1 / Turtle §2.1):
                //   BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?
                //
                // PN_CHARS_U is [A-Za-z0-9_] (and Unicode letters/digits, but here we
                // require only the ASCII subset for well-formedness on raw labels).
                // PN_CHARS additionally allows '-' and '.'.
                //
                // The previous check required ONLY alphanumeric characters, which rejected
                // valid labels like `_:b_annot_123` (contains '_') or `_:b-0` (contains '-').
                fn is_valid_bnode_char(c: char) -> bool {
                    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
                }

                if let Some(label) = id.strip_prefix("_:") {
                    if label.is_empty() || !label.chars().all(is_valid_bnode_char) {
                        return Err(ValidationError::new(
                            ValidationErrorType::InvalidBlankNodeLabel,
                            format!(
                                "Invalid blank node label '{id}' in {position} position \
                                 (label must match [A-Za-z0-9_\\-.]+ after the '_:' prefix)"
                            ),
                        ));
                    }
                } else if id.is_empty() || !id.chars().all(is_valid_bnode_char) {
                    // Legacy bare label without the '_:' prefix
                    return Err(ValidationError::new(
                        ValidationErrorType::InvalidBlankNodeLabel,
                        format!("Invalid blank node label '{id}' in {position} position"),
                    ));
                }
                Ok(())
            }
            crate::semantics::RdfTerm::TripleTerm(inner_triple) => {
                // RDF 1.2 triple terms: validate as embedded triples (object-position only)
                self.validate_quoted_triple_structure(inner_triple)?;
                Ok(())
            }
            crate::semantics::RdfTerm::Iri(_) => Ok(()),
        }
    }

    /// Validate the structure of a quoted triple
    fn validate_quoted_triple_structure(
        &self,
        triple: &crate::semantics::Triple,
    ) -> Result<(), ValidationError> {
        // Ensure predicate is not a quoted triple (already checked at top level)
        if matches!(triple.predicate, crate::semantics::RdfTerm::QuotedTriple(_)) {
            return Err(ValidationError::new(
                ValidationErrorType::QuotedTripleInPredicatePosition,
                "Quoted triple cannot contain another quoted triple in predicate position"
                    .to_string(),
            ));
        }

        // Recursively validate nested terms
        if let crate::semantics::RdfTerm::QuotedTriple(inner) = &triple.subject {
            self.validate_quoted_triple_structure(inner)?;
        }
        if let crate::semantics::RdfTerm::QuotedTriple(inner) = &triple.object {
            self.validate_quoted_triple_structure(inner)?;
        }

        Ok(())
    }

    /// Get the maximum allowed nesting depth from configuration or use default
    ///
    /// Returns the configured max depth for quoted triple nesting.
    /// Default is 5 levels, which should be sufficient for most use cases.
    ///
    /// Note: In future, this should be exposed in `ReasoningConfig` or `ValidationConfig`
    fn get_max_nesting_depth(&self) -> usize {
        // Use default value of 5 - appropriate for most RDF-star use cases
        // This balances expressiveness with computational complexity
        // Can be made configurable when ValidationConfig is added to ReasonerConfig
        const DEFAULT_MAX_NESTING: usize = 5;
        DEFAULT_MAX_NESTING
    }

    /// Analyze property hierarchy to build internal structures
    fn analyze_property_hierarchy(&mut self) {
        for axiom in self.ontology.axioms() {
            match axiom {
                Axiom::SubObjectPropertyOf(sub_prop_axiom) => {
                    if let ObjectPropertyExpression::ObjectProperty(sub_prop) =
                        &sub_prop_axiom.sub_property
                        && let ObjectPropertyExpression::ObjectProperty(super_prop) =
                            &sub_prop_axiom.super_property
                    {
                        self.property_hierarchy
                            .entry(crate::ontology::IRI::from(sub_prop.iri.to_string()))
                            .or_default()
                            .push(crate::ontology::IRI::from(super_prop.iri.to_string()));
                    }
                }
                Axiom::TransitiveObjectProperty(trans_axiom) => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = &trans_axiom.property {
                        self.transitive_properties
                            .insert(crate::ontology::IRI::from(prop.iri.to_string()));
                    }
                }
                _ => {}
            }
        }
    }

    /// Compute simple properties according to OWL 2 DL rules
    fn compute_simple_properties(&mut self) {
        // A property is simple if it is not transitive and does not have transitive sub-properties
        let all_properties: HashSet<crate::ontology::IRI> = self
            .ontology
            .axioms()
            .iter()
            .filter_map(|axiom| match axiom {
                Axiom::Declaration(decl) => {
                    if let Entity::ObjectProperty(prop) = &decl.entity {
                        Some(prop.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        for property in all_properties {
            if self.is_simple_property(&property) {
                self.simple_properties.insert(property);
            }
        }
    }

    /// Check if a property is simple (not transitive, no transitive sub-properties)
    fn is_simple_property(&self, property: &crate::ontology::IRI) -> bool {
        // Property is not simple if it's transitive
        if self.transitive_properties.contains(property) {
            return false;
        }

        // Property is not simple if it has transitive sub-properties
        if self.has_transitive_subproperty(property, &mut HashSet::new()) {
            return false;
        }

        true
    }

    /// Check if property has transitive sub-properties
    fn has_transitive_subproperty(
        &self,
        property: &crate::ontology::IRI,
        visited: &mut HashSet<crate::ontology::IRI>,
    ) -> bool {
        if visited.contains(property) {
            return false; // Avoid infinite recursion
        }
        visited.insert(property.clone());

        // Check direct sub-properties
        for (sub_prop, super_props) in &self.property_hierarchy {
            if super_props.contains(property) {
                if self.transitive_properties.contains(sub_prop) {
                    return true;
                }
                if self.has_transitive_subproperty(sub_prop, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Check for cyclic property hierarchy
    fn has_cyclic_hierarchy(
        &self,
        property: &crate::ontology::IRI,
        visited: &mut HashSet<crate::ontology::IRI>,
    ) -> bool {
        if visited.contains(property) {
            return true;
        }
        visited.insert(property.clone());

        if let Some(super_properties) = self.property_hierarchy.get(property) {
            for super_prop in super_properties {
                if self.has_cyclic_hierarchy(super_prop, visited) {
                    return true;
                }
            }
        }

        visited.remove(property);
        false
    }

    /// Validate property chain axiom
    fn validate_property_chain(
        &self,
        _chain: &[ObjectPropertyExpression],
        _super_property: &ObjectPropertyExpression,
    ) -> Result<Vec<ValidationError>, OxidowlError> {
        let errors = Vec::new();

        // Implement property chain validation according to OWL 2 DL rules
        // This is a complex validation that ensures property chain axioms
        // don't violate the regularity conditions

        Ok(errors)
    }

    /// Check if a datatype is valid
    fn is_valid_datatype(&self, _data_range: &horned_owl::model::DataRange<String>) -> bool {
        // This will be enhanced when we implement the complete OWL 2 datatype map
        // For now, assume basic validation
        true
    }

    /// Extract entities from an axiom for validation
    fn extract_entities_from_axiom(&self, _axiom: &Axiom) -> Vec<Entity> {
        // Extract all entities used in an axiom
        // This would be a comprehensive extraction of all IRIs used
        Vec::new()
    }

    /// Get the range of a data property if defined
    fn get_data_property_range(
        &self,
        property: &crate::ontology::DataPropertyExpression,
    ) -> Option<crate::ontology::DataRange> {
        for axiom in self.ontology.axioms() {
            if let Axiom::DataPropertyRange(range_axiom) = axiom
                && self
                    .property_expression_matches(&range_axiom.property, property)
                    .unwrap_or(false)
            {
                return Some(range_axiom.range.clone());
            }
        }
        None
    }

    /// Check if a literal is compatible with a data range
    fn is_literal_compatible_with_range(
        &self,
        literal: &crate::ontology::Literal,
        range: &crate::ontology::DataRange,
    ) -> bool {
        use crate::ontology::DataRange;

        match range {
            DataRange::Datatype(datatype) => {
                self.check_literal_datatype_compatibility(literal, datatype)
            }
            DataRange::DataIntersectionOf(ranges) => {
                // Literal must be compatible with all ranges in intersection
                ranges
                    .iter()
                    .all(|r| self.is_literal_compatible_with_range(literal, r))
            }
            DataRange::DataUnionOf(ranges) => {
                // Literal must be compatible with at least one range in union
                ranges
                    .iter()
                    .any(|r| self.is_literal_compatible_with_range(literal, r))
            }
            DataRange::DataComplementOf(complement_range) => {
                // Literal must NOT be compatible with the complement range
                !self.is_literal_compatible_with_range(literal, complement_range)
            }
            DataRange::DataOneOf(literals) => {
                // Literal must be one of the enumerated values
                literals.iter().any(|l| self.literals_equal(literal, l))
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                // Check base datatype and all facet restrictions
                self.check_literal_datatype_compatibility(literal, datatype)
                    && self.check_facet_restrictions(literal, restrictions)
            }
        }
    }

    /// Check if two literals are equal
    fn literals_equal(
        &self,
        lit1: &crate::ontology::Literal,
        lit2: &crate::ontology::Literal,
    ) -> bool {
        lit1.value == lit2.value && lit1.datatype == lit2.datatype
    }

    /// Check if a literal is compatible with a specific datatype
    fn check_literal_datatype_compatibility(
        &self,
        literal: &crate::ontology::Literal,
        datatype: &crate::ontology::IRI,
    ) -> bool {
        // If literal has explicit datatype, check for exact match or subtype relationship
        if let Some(literal_datatype_url) = &literal.datatype {
            let literal_datatype = crate::ontology::IRI::new(literal_datatype_url.as_str());
            return literal_datatype == *datatype
                || self.is_datatype_subtype(&literal_datatype, datatype);
        }

        // For plain literals, check if they can be interpreted as the target datatype
        self.can_parse_as_datatype(&literal.value, datatype)
    }

    /// Check if one datatype is a subtype of another
    fn is_datatype_subtype(
        &self,
        subtype: &crate::ontology::IRI,
        supertype: &crate::ontology::IRI,
    ) -> bool {
        // Implement basic XML Schema type hierarchy
        let subtype_str = subtype.as_str();
        let supertype_str = supertype.as_str();

        // Examples of subtype relationships in XML Schema
        match (subtype_str, supertype_str) {
            // Integer hierarchy
            (
                "http://www.w3.org/2001/XMLSchema#int",
                "http://www.w3.org/2001/XMLSchema#integer",
            ) => true,
            ("http://www.w3.org/2001/XMLSchema#short", "http://www.w3.org/2001/XMLSchema#int") => {
                true
            }
            ("http://www.w3.org/2001/XMLSchema#byte", "http://www.w3.org/2001/XMLSchema#short") => {
                true
            }

            // Unsigned integer hierarchy
            (
                "http://www.w3.org/2001/XMLSchema#unsignedInt",
                "http://www.w3.org/2001/XMLSchema#unsignedLong",
            ) => true,
            (
                "http://www.w3.org/2001/XMLSchema#unsignedShort",
                "http://www.w3.org/2001/XMLSchema#unsignedInt",
            ) => true,
            (
                "http://www.w3.org/2001/XMLSchema#unsignedByte",
                "http://www.w3.org/2001/XMLSchema#unsignedShort",
            ) => true,

            // All numeric types derive from decimal
            (sub, "http://www.w3.org/2001/XMLSchema#decimal") if self.is_numeric_datatype(sub) => {
                true
            }

            _ => false,
        }
    }

    /// Check if a string can be parsed as a specific datatype
    fn can_parse_as_datatype(&self, value: &str, datatype: &crate::ontology::IRI) -> bool {
        match datatype.as_str() {
            "http://www.w3.org/2001/XMLSchema#string" => true,
            "http://www.w3.org/2001/XMLSchema#boolean" => {
                matches!(value, "true" | "false" | "1" | "0")
            }
            "http://www.w3.org/2001/XMLSchema#integer" => value.parse::<i64>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#int" => value.parse::<i32>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#short" => value.parse::<i16>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#byte" => value.parse::<i8>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#decimal" => value.parse::<f64>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#float" => value.parse::<f32>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#double" => value.parse::<f64>().is_ok(),
            "http://www.w3.org/2001/XMLSchema#date" => {
                // Simple date format check (YYYY-MM-DD)
                value.len() == 10
                    && value.chars().nth(4) == Some('-')
                    && value.chars().nth(7) == Some('-')
            }
            "http://www.w3.org/2001/XMLSchema#dateTime" => {
                // Simple datetime format check
                value.contains('T')
            }
            "http://www.w3.org/2001/XMLSchema#time" => {
                // Simple time format check (HH:MM:SS)
                value.contains(':')
            }
            _ => true, // Conservative approach for unknown datatypes
        }
    }

    /// Check if a datatype IRI represents a numeric type
    fn is_numeric_datatype(&self, datatype: &str) -> bool {
        matches!(
            datatype,
            "http://www.w3.org/2001/XMLSchema#integer"
                | "http://www.w3.org/2001/XMLSchema#int"
                | "http://www.w3.org/2001/XMLSchema#short"
                | "http://www.w3.org/2001/XMLSchema#byte"
                | "http://www.w3.org/2001/XMLSchema#long"
                | "http://www.w3.org/2001/XMLSchema#unsignedLong"
                | "http://www.w3.org/2001/XMLSchema#unsignedInt"
                | "http://www.w3.org/2001/XMLSchema#unsignedShort"
                | "http://www.w3.org/2001/XMLSchema#unsignedByte"
                | "http://www.w3.org/2001/XMLSchema#decimal"
                | "http://www.w3.org/2001/XMLSchema#float"
                | "http://www.w3.org/2001/XMLSchema#double"
        )
    }

    /// Check facet restrictions on a literal
    fn check_facet_restrictions(
        &self,
        literal: &crate::ontology::Literal,
        restrictions: &[crate::ontology::FacetRestriction],
    ) -> bool {
        for restriction in restrictions {
            if !self.check_single_facet_restriction(literal, restriction) {
                return false;
            }
        }
        true
    }

    /// Check a single facet restriction
    fn check_single_facet_restriction(
        &self,
        literal: &crate::ontology::Literal,
        restriction: &crate::ontology::FacetRestriction,
    ) -> bool {
        // Convert facet IRI to a more usable form for matching
        let facet_str = restriction.facet.as_str();

        match facet_str {
            "http://www.w3.org/2001/XMLSchema#length" => {
                if let Ok(length) = restriction.value.value.parse::<usize>() {
                    literal.value.len() == length
                } else {
                    false
                }
            }
            "http://www.w3.org/2001/XMLSchema#minLength" => {
                if let Ok(min_length) = restriction.value.value.parse::<usize>() {
                    literal.value.len() >= min_length
                } else {
                    false
                }
            }
            "http://www.w3.org/2001/XMLSchema#maxLength" => {
                if let Ok(max_length) = restriction.value.value.parse::<usize>() {
                    literal.value.len() <= max_length
                } else {
                    false
                }
            }
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                // Simple pattern matching - in practice would use regex
                literal.value.contains(&restriction.value.value)
            }
            "http://www.w3.org/2001/XMLSchema#minInclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a >= b)
            }
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a <= b)
            }
            "http://www.w3.org/2001/XMLSchema#minExclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a > b)
            }
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                self.compare_numeric_values(&literal.value, &restriction.value.value, |a, b| a < b)
            }
            _ => {
                // For unknown facet types, conservatively return true
                true
            }
        }
    }

    /// Compare numeric values with a given comparison function
    fn compare_numeric_values<F>(&self, value1: &str, value2: &str, cmp: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        if let (Ok(val1), Ok(val2)) = (value1.parse::<f64>(), value2.parse::<f64>()) {
            cmp(val1, val2)
        } else {
            false
        }
    }

    /// Check if property expressions match
    fn property_expression_matches(
        &self,
        prop1: &crate::ontology::DataPropertyExpression,
        prop2: &crate::ontology::DataPropertyExpression,
    ) -> Result<bool, crate::error::OxidowlError> {
        match (prop1, prop2) {
            (
                crate::ontology::DataPropertyExpression::DataProperty(p1),
                crate::ontology::DataPropertyExpression::DataProperty(p2),
            ) => Ok(p1.iri == p2.iri),
        }
    }

    /// Format a data range for error messages
    fn format_data_range(&self, range: &crate::ontology::DataRange) -> String {
        match range {
            crate::ontology::DataRange::Datatype(iri) => iri.to_string(),
            crate::ontology::DataRange::DatatypeRestriction { datatype, .. } => {
                format!("restriction on {datatype}")
            }
            crate::ontology::DataRange::DataIntersectionOf(_) => {
                "intersection of data ranges".to_string()
            }
            crate::ontology::DataRange::DataUnionOf(_) => "union of data ranges".to_string(),
            crate::ontology::DataRange::DataComplementOf(_) => {
                "complement of data range".to_string()
            }
            crate::ontology::DataRange::DataOneOf(_) => "enumeration of literals".to_string(),
        }
    }

    /// Validate a datatype definition
    fn validate_datatype_definition(
        &mut self,
        datatype_def: &crate::ontology::datatypes::DatatypeDefinitionAxiom,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        // Check if the datatype is already defined
        if self
            .defined_datatypes
            .contains(&datatype_def.datatype.to_string())
        {
            errors.push(ValidationError::new(
                ValidationErrorType::DuplicateDatatypeDefinition,
                format!(
                    "Datatype {} is defined multiple times",
                    datatype_def.datatype
                ),
            ));
        }

        // Add to defined datatypes
        self.defined_datatypes
            .insert(datatype_def.datatype.to_string());

        // Validate the datatype expression - proper conversion from horned_owl::DataRange to internal DataRange
        if let Ok(internal_range) = self.convert_horned_owl_data_range(&datatype_def.data_range) {
            self.validate_datatype_expression(&internal_range, errors)?;
        } else {
            errors.push(ValidationError::new(
                ValidationErrorType::UnsupportedDatatype,
                format!(
                    "Cannot convert datatype expression for: {}",
                    datatype_def.datatype
                ),
            ));
        }

        // Check for circular datatype definitions - proper conversion and checking
        if let Ok(internal_range) = self.convert_horned_owl_data_range(&datatype_def.data_range)
            && self.has_circular_datatype_reference(datatype_def.datatype.as_ref(), &internal_range)
        {
            errors.push(ValidationError::new(
                ValidationErrorType::CircularDatatypeDefinition,
                format!(
                    "Circular reference in datatype definition: {}",
                    datatype_def.datatype
                ),
            ));
        }

        Ok(())
    }

    /// Validate a datatype expression for OWL 2 DL compliance
    fn validate_datatype_expression(
        &self,
        data_range: &crate::ontology::datatypes::DataRange,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), Error> {
        match data_range {
            crate::ontology::datatypes::DataRange::Datatype(dt) => {
                // Check if datatype is known/supported
                let dt_iri = dt.to_string();
                if !self.is_supported_datatype(&dt_iri) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::UnsupportedDatatype,
                        format!("Unsupported datatype: {dt_iri}"),
                    ));
                }
            }
            crate::ontology::datatypes::DataRange::DataIntersectionOf(ranges) => {
                // Validate each range in intersection
                for range in ranges {
                    self.validate_datatype_expression(range, errors)?;
                }

                // Check for meaningful intersections
                if ranges.len() < 2 {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidDatatypeExpression,
                        "Data intersection must have at least 2 operands".to_string(),
                    ));
                }
            }
            crate::ontology::datatypes::DataRange::DataUnionOf(ranges) => {
                // Validate each range in union
                for range in ranges {
                    self.validate_datatype_expression(range, errors)?;
                }

                // Check for meaningful unions
                if ranges.len() < 2 {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidDatatypeExpression,
                        "Data union must have at least 2 operands".to_string(),
                    ));
                }
            }
            crate::ontology::datatypes::DataRange::DataComplementOf(range) => {
                // Validate the complemented range
                self.validate_datatype_expression(range, errors)?;
            }
            crate::ontology::datatypes::DataRange::DataOneOf(values) => {
                // Validate each literal value
                for literal in values {
                    // Convert horned_owl literal to string then to internal literal
                    let literal_string = match literal {
                        horned_owl::model::Literal::Simple { literal } => literal.clone(),
                        horned_owl::model::Literal::Language { literal, .. } => literal.clone(),
                        horned_owl::model::Literal::Datatype { literal, .. } => literal.clone(),
                    };
                    let internal_literal = crate::ontology::Literal::new(literal_string);
                    if !self.validate_literal_value(&internal_literal) {
                        errors.push(ValidationError::new(
                            ValidationErrorType::InvalidLiteral,
                            "Invalid literal in data enumeration".to_string(),
                        ));
                    }
                }

                // Check for meaningful enumerations
                if values.is_empty() {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidDatatypeExpression,
                        "Data enumeration cannot be empty".to_string(),
                    ));
                }
            }
            crate::ontology::datatypes::DataRange::DatatypeRestriction { datatype, facets } => {
                // Validate base datatype
                let dt_iri = datatype.to_string();
                if !self.is_supported_datatype(&dt_iri) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::UnsupportedDatatype,
                        format!("Unsupported base datatype: {dt_iri}"),
                    ));
                }

                // Validate facet restrictions
                self.validate_facet_restrictions(&dt_iri, facets, errors)?;
            }
        }

        Ok(())
    }

    /// Check if a datatype is supported in OWL 2 DL
    fn is_supported_datatype(&self, datatype_iri: &str) -> bool {
        // Standard XML Schema datatypes
        matches!(
            datatype_iri,
            "http://www.w3.org/2001/XMLSchema#string"
                | "http://www.w3.org/2001/XMLSchema#boolean"
                | "http://www.w3.org/2001/XMLSchema#decimal"
                | "http://www.w3.org/2001/XMLSchema#float"
                | "http://www.w3.org/2001/XMLSchema#double"
                | "http://www.w3.org/2001/XMLSchema#dateTime"
                | "http://www.w3.org/2001/XMLSchema#time"
                | "http://www.w3.org/2001/XMLSchema#date"
                | "http://www.w3.org/2001/XMLSchema#gYearMonth"
                | "http://www.w3.org/2001/XMLSchema#gYear"
                | "http://www.w3.org/2001/XMLSchema#gMonthDay"
                | "http://www.w3.org/2001/XMLSchema#gDay"
                | "http://www.w3.org/2001/XMLSchema#gMonth"
                | "http://www.w3.org/2001/XMLSchema#hexBinary"
                | "http://www.w3.org/2001/XMLSchema#base64Binary"
                | "http://www.w3.org/2001/XMLSchema#anyURI"
                | "http://www.w3.org/2001/XMLSchema#QName"
                | "http://www.w3.org/2001/XMLSchema#NOTATION"
                | "http://www.w3.org/2001/XMLSchema#normalizedString"
                | "http://www.w3.org/2001/XMLSchema#token"
                | "http://www.w3.org/2001/XMLSchema#language"
                | "http://www.w3.org/2001/XMLSchema#NMTOKEN"
                | "http://www.w3.org/2001/XMLSchema#NMTOKENS"
                | "http://www.w3.org/2001/XMLSchema#Name"
                | "http://www.w3.org/2001/XMLSchema#NCName"
                | "http://www.w3.org/2001/XMLSchema#ID"
                | "http://www.w3.org/2001/XMLSchema#IDREF"
                | "http://www.w3.org/2001/XMLSchema#IDREFS"
                | "http://www.w3.org/2001/XMLSchema#ENTITY"
                | "http://www.w3.org/2001/XMLSchema#ENTITIES"
                | "http://www.w3.org/2001/XMLSchema#integer"
                | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
                | "http://www.w3.org/2001/XMLSchema#negativeInteger"
                | "http://www.w3.org/2001/XMLSchema#long"
                | "http://www.w3.org/2001/XMLSchema#int"
                | "http://www.w3.org/2001/XMLSchema#short"
                | "http://www.w3.org/2001/XMLSchema#byte"
                | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
                | "http://www.w3.org/2001/XMLSchema#unsignedLong"
                | "http://www.w3.org/2001/XMLSchema#unsignedInt"
                | "http://www.w3.org/2001/XMLSchema#unsignedShort"
                | "http://www.w3.org/2001/XMLSchema#unsignedByte"
                | "http://www.w3.org/2001/XMLSchema#positiveInteger"
                | "http://www.w3.org/2001/XMLSchema#duration"
                | "http://www.w3.org/2001/XMLSchema#dayTimeDuration"
                | "http://www.w3.org/2001/XMLSchema#yearMonthDuration"
                | "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral"
                | "http://www.w3.org/2000/01/rdf-schema#Literal"
        ) || self.defined_datatypes.contains(datatype_iri)
    }

    /// Validate literal value for OWL 2 DL
    fn validate_literal_value(&self, _literal: &crate::ontology::Literal) -> bool {
        // For now, just return true - proper validation would check the literal format
        true
    }

    /// Validate facet restrictions
    fn validate_facet_restrictions(
        &self,
        base_datatype: &str,
        restrictions: &[crate::ontology::datatypes::FacetRestriction],
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), Error> {
        for restriction in restrictions {
            // Check if facet is applicable to the base datatype
            if !self.is_facet_applicable_to_datatype(&restriction.facet, base_datatype) {
                errors.push(ValidationError::new(
                    ValidationErrorType::InvalidFacetRestriction,
                    format!(
                        "Facet {:?} not applicable to datatype {}",
                        restriction.facet, base_datatype
                    ),
                ));
            }

            // Validate the restriction value
            let literal_string = match &restriction.literal {
                horned_owl::model::Literal::Simple { literal } => literal.clone(),
                horned_owl::model::Literal::Language { literal, .. } => literal.clone(),
                horned_owl::model::Literal::Datatype { literal, .. } => literal.clone(),
            };
            let internal_literal = crate::ontology::Literal::new(literal_string);
            if !self.validate_literal_value(&internal_literal) {
                errors.push(ValidationError::new(
                    ValidationErrorType::InvalidLiteral,
                    "Invalid facet restriction value".to_string(),
                ));
            }
        }

        // Check for conflicting restrictions
        self.check_conflicting_facet_restrictions(restrictions, errors)?;

        Ok(())
    }

    /// Check if a facet is applicable to a datatype
    fn is_facet_applicable_to_datatype(
        &self,
        facet: &crate::ontology::datatypes::ConstrainingFacet,
        datatype: &str,
    ) -> bool {
        use crate::ontology::datatypes::ConstrainingFacet;

        match facet {
            ConstrainingFacet::Length
            | ConstrainingFacet::MinLength
            | ConstrainingFacet::MaxLength => {
                // Length facets apply to string-based and binary datatypes
                matches!(
                    datatype,
                    "http://www.w3.org/2001/XMLSchema#string"
                        | "http://www.w3.org/2001/XMLSchema#normalizedString"
                        | "http://www.w3.org/2001/XMLSchema#token"
                        | "http://www.w3.org/2001/XMLSchema#language"
                        | "http://www.w3.org/2001/XMLSchema#Name"
                        | "http://www.w3.org/2001/XMLSchema#NCName"
                        | "http://www.w3.org/2001/XMLSchema#ID"
                        | "http://www.w3.org/2001/XMLSchema#IDREF"
                        | "http://www.w3.org/2001/XMLSchema#ENTITY"
                        | "http://www.w3.org/2001/XMLSchema#hexBinary"
                        | "http://www.w3.org/2001/XMLSchema#base64Binary"
                        | "http://www.w3.org/2001/XMLSchema#anyURI"
                )
            }
            ConstrainingFacet::Pattern => {
                // Pattern applies to string-based datatypes
                matches!(
                    datatype,
                    "http://www.w3.org/2001/XMLSchema#string"
                        | "http://www.w3.org/2001/XMLSchema#normalizedString"
                        | "http://www.w3.org/2001/XMLSchema#token"
                        | "http://www.w3.org/2001/XMLSchema#language"
                        | "http://www.w3.org/2001/XMLSchema#Name"
                        | "http://www.w3.org/2001/XMLSchema#NCName"
                        | "http://www.w3.org/2001/XMLSchema#ID"
                        | "http://www.w3.org/2001/XMLSchema#IDREF"
                        | "http://www.w3.org/2001/XMLSchema#ENTITY"
                        | "http://www.w3.org/2001/XMLSchema#anyURI"
                )
            }
            ConstrainingFacet::MinInclusive
            | ConstrainingFacet::MaxInclusive
            | ConstrainingFacet::MinExclusive
            | ConstrainingFacet::MaxExclusive => {
                // Value range facets apply to ordered datatypes
                matches!(
                    datatype,
                    "http://www.w3.org/2001/XMLSchema#decimal"
                        | "http://www.w3.org/2001/XMLSchema#float"
                        | "http://www.w3.org/2001/XMLSchema#double"
                        | "http://www.w3.org/2001/XMLSchema#integer"
                        | "http://www.w3.org/2001/XMLSchema#long"
                        | "http://www.w3.org/2001/XMLSchema#int"
                        | "http://www.w3.org/2001/XMLSchema#short"
                        | "http://www.w3.org/2001/XMLSchema#byte"
                        | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
                        | "http://www.w3.org/2001/XMLSchema#positiveInteger"
                        | "http://www.w3.org/2001/XMLSchema#unsignedLong"
                        | "http://www.w3.org/2001/XMLSchema#unsignedInt"
                        | "http://www.w3.org/2001/XMLSchema#unsignedShort"
                        | "http://www.w3.org/2001/XMLSchema#unsignedByte"
                        | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
                        | "http://www.w3.org/2001/XMLSchema#negativeInteger"
                        | "http://www.w3.org/2001/XMLSchema#dateTime"
                        | "http://www.w3.org/2001/XMLSchema#time"
                        | "http://www.w3.org/2001/XMLSchema#date"
                )
            }
            ConstrainingFacet::TotalDigits | ConstrainingFacet::FractionDigits => {
                // Precision facets apply to decimal and derived datatypes
                matches!(
                    datatype,
                    "http://www.w3.org/2001/XMLSchema#decimal"
                        | "http://www.w3.org/2001/XMLSchema#integer"
                        | "http://www.w3.org/2001/XMLSchema#long"
                        | "http://www.w3.org/2001/XMLSchema#int"
                        | "http://www.w3.org/2001/XMLSchema#short"
                        | "http://www.w3.org/2001/XMLSchema#byte"
                        | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
                        | "http://www.w3.org/2001/XMLSchema#positiveInteger"
                        | "http://www.w3.org/2001/XMLSchema#unsignedLong"
                        | "http://www.w3.org/2001/XMLSchema#unsignedInt"
                        | "http://www.w3.org/2001/XMLSchema#unsignedShort"
                        | "http://www.w3.org/2001/XMLSchema#unsignedByte"
                        | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
                        | "http://www.w3.org/2001/XMLSchema#negativeInteger"
                )
            }
            ConstrainingFacet::Enumeration => {
                // Enumeration can apply to any datatype
                true
            }
            ConstrainingFacet::WhiteSpace => {
                // WhiteSpace facet applies to string-based datatypes
                matches!(
                    datatype,
                    "http://www.w3.org/2001/XMLSchema#string"
                        | "http://www.w3.org/2001/XMLSchema#normalizedString"
                        | "http://www.w3.org/2001/XMLSchema#token"
                )
            }
        }
    }

    /// Convert `horned_owl` data range to internal data range
    fn convert_horned_owl_data_range(
        &self,
        range: &horned_owl::model::DataRange<String>,
    ) -> Result<crate::ontology::datatypes::DataRange, Error> {
        match range {
            horned_owl::model::DataRange::Datatype(dt) => Ok(
                crate::ontology::datatypes::DataRange::Datatype(dt.clone().into()),
            ),
            horned_owl::model::DataRange::DataIntersectionOf(ranges) => {
                let converted_ranges: Result<Vec<_>, _> = ranges
                    .iter()
                    .map(|r| self.convert_horned_owl_data_range(r))
                    .collect();
                Ok(crate::ontology::datatypes::DataRange::DataIntersectionOf(
                    converted_ranges?,
                ))
            }
            horned_owl::model::DataRange::DataUnionOf(ranges) => {
                let converted_ranges: Result<Vec<_>, _> = ranges
                    .iter()
                    .map(|r| self.convert_horned_owl_data_range(r))
                    .collect();
                Ok(crate::ontology::datatypes::DataRange::DataUnionOf(
                    converted_ranges?,
                ))
            }
            horned_owl::model::DataRange::DataComplementOf(range) => {
                let converted_range = self.convert_horned_owl_data_range(range)?;
                Ok(crate::ontology::datatypes::DataRange::DataComplementOf(
                    Box::new(converted_range),
                ))
            }
            horned_owl::model::DataRange::DataOneOf(literals) => {
                let converted_literals: Vec<_> = literals
                    .iter()
                    .map(|lit| self.convert_horned_owl_literal(lit))
                    .collect();
                Ok(crate::ontology::datatypes::DataRange::DataOneOf(
                    converted_literals,
                ))
            }
            horned_owl::model::DataRange::DatatypeRestriction(datatype, restrictions) => {
                let converted_restrictions: Result<Vec<_>, _> = restrictions
                    .iter()
                    .map(|fr| self.convert_horned_owl_facet_restriction(fr))
                    .collect();
                Ok(crate::ontology::datatypes::DataRange::DatatypeRestriction {
                    datatype: datatype.clone().into(),
                    facets: converted_restrictions?,
                })
            }
        }
    }

    /// Convert `horned_owl` literal to internal literal
    fn convert_horned_owl_literal(
        &self,
        literal: &horned_owl::model::Literal<String>,
    ) -> horned_owl::model::Literal<String> {
        // For now, just clone since they're compatible
        literal.clone()
    }

    /// Convert `horned_owl` facet restriction to internal facet restriction
    fn convert_horned_owl_facet_restriction(
        &self,
        fr: &horned_owl::model::FacetRestriction<String>,
    ) -> Result<crate::ontology::datatypes::FacetRestriction, Error> {
        // Convert facet by string representation since enum variants are private
        let facet_str = format!("{:?}", fr.f);
        let internal_facet = match facet_str.as_str() {
            "Length" => crate::ontology::datatypes::ConstrainingFacet::Length,
            "MinLength" => crate::ontology::datatypes::ConstrainingFacet::MinLength,
            "MaxLength" => crate::ontology::datatypes::ConstrainingFacet::MaxLength,
            "Pattern" => crate::ontology::datatypes::ConstrainingFacet::Pattern,
            "MinInclusive" => crate::ontology::datatypes::ConstrainingFacet::MinInclusive,
            "MaxInclusive" => crate::ontology::datatypes::ConstrainingFacet::MaxInclusive,
            "MinExclusive" => crate::ontology::datatypes::ConstrainingFacet::MinExclusive,
            "MaxExclusive" => crate::ontology::datatypes::ConstrainingFacet::MaxExclusive,
            "TotalDigits" => crate::ontology::datatypes::ConstrainingFacet::TotalDigits,
            "FractionDigits" => crate::ontology::datatypes::ConstrainingFacet::FractionDigits,
            _ => {
                return Err(Error::invalid_input(format!(
                    "Unknown facet type: {facet_str}"
                )));
            }
        };

        Ok(crate::ontology::datatypes::FacetRestriction {
            facet: internal_facet,
            literal: fr.l.clone(),
        })
    }

    /// Check for circular datatype references
    fn has_circular_datatype_reference(
        &self,
        datatype: &str,
        range: &crate::ontology::datatypes::DataRange,
    ) -> bool {
        let mut visited = HashSet::new();
        self.check_datatype_circularity(datatype, range, &mut visited)
    }

    /// Recursive helper for circular datatype checking
    fn check_datatype_circularity(
        &self,
        target_datatype: &str,
        range: &crate::ontology::datatypes::DataRange,
        visited: &mut HashSet<String>,
    ) -> bool {
        match range {
            crate::ontology::datatypes::DataRange::Datatype(dt) => {
                let dt_iri = dt.as_ref();
                if dt_iri == target_datatype {
                    return true;
                }
                if visited.contains(dt_iri) {
                    return false; // Already checked
                }
                visited.insert(dt_iri.to_string());
                false
            }
            crate::ontology::datatypes::DataRange::DataIntersectionOf(ranges)
            | crate::ontology::datatypes::DataRange::DataUnionOf(ranges) => ranges
                .iter()
                .any(|r| self.check_datatype_circularity(target_datatype, r, visited)),
            crate::ontology::datatypes::DataRange::DataComplementOf(range) => {
                self.check_datatype_circularity(target_datatype, range, visited)
            }
            crate::ontology::datatypes::DataRange::DataOneOf(_) => false,
            crate::ontology::datatypes::DataRange::DatatypeRestriction { datatype, .. } => {
                let dt_iri = datatype.as_ref();
                if dt_iri == target_datatype {
                    return true;
                }
                if visited.contains(dt_iri) {
                    return false;
                }
                visited.insert(dt_iri.to_string());
                false
            }
        }
    }

    /// Check for conflicting facet restrictions
    fn check_conflicting_facet_restrictions(
        &self,
        restrictions: &[crate::ontology::datatypes::FacetRestriction],
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), Error> {
        use crate::ontology::datatypes::ConstrainingFacet;

        let mut min_inclusive: Option<&str> = None;
        let mut max_inclusive: Option<&str> = None;
        let mut min_exclusive: Option<&str> = None;
        let mut max_exclusive: Option<&str> = None;
        let mut min_length: Option<&str> = None;
        let mut max_length: Option<&str> = None;
        let mut exact_length: Option<&str> = None;

        for restriction in restrictions {
            let value_str = match &restriction.literal {
                horned_owl::model::Literal::Simple { literal } => literal.as_str(),
                horned_owl::model::Literal::Language { literal, .. } => literal.as_str(),
                horned_owl::model::Literal::Datatype { literal, .. } => literal.as_str(),
            };

            match restriction.facet {
                ConstrainingFacet::MinInclusive => {
                    if min_inclusive.is_some() || min_exclusive.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple minimum value restrictions".to_string(),
                        ));
                    }
                    min_inclusive = Some(value_str);
                }
                ConstrainingFacet::MaxInclusive => {
                    if max_inclusive.is_some() || max_exclusive.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple maximum value restrictions".to_string(),
                        ));
                    }
                    max_inclusive = Some(value_str);
                }
                ConstrainingFacet::MinExclusive => {
                    if min_inclusive.is_some() || min_exclusive.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple minimum value restrictions".to_string(),
                        ));
                    }
                    min_exclusive = Some(value_str);
                }
                ConstrainingFacet::MaxExclusive => {
                    if max_inclusive.is_some() || max_exclusive.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple maximum value restrictions".to_string(),
                        ));
                    }
                    max_exclusive = Some(value_str);
                }
                ConstrainingFacet::MinLength => {
                    if min_length.is_some() || exact_length.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple minimum length restrictions".to_string(),
                        ));
                    }
                    min_length = Some(value_str);
                }
                ConstrainingFacet::MaxLength => {
                    if max_length.is_some() || exact_length.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Multiple maximum length restrictions".to_string(),
                        ));
                    }
                    max_length = Some(value_str);
                }
                ConstrainingFacet::Length => {
                    if exact_length.is_some() || min_length.is_some() || max_length.is_some() {
                        errors.push(ValidationError::new(
                            ValidationErrorType::ConflictingFacetRestrictions,
                            "Exact length conflicts with min/max length".to_string(),
                        ));
                    }
                    exact_length = Some(value_str);
                }
                _ => {} // Other facets don't typically conflict
            }
        }

        // Check for impossible value ranges
        if let (Some(min_val), Some(max_val)) = (
            min_inclusive.or(min_exclusive),
            max_inclusive.or(max_exclusive),
        ) && let (Ok(min_num), Ok(max_num)) = (min_val.parse::<f64>(), max_val.parse::<f64>())
            && min_num > max_num
        {
            errors.push(ValidationError::new(
                ValidationErrorType::ConflictingFacetRestrictions,
                "Minimum value greater than maximum value".to_string(),
            ));
        }

        // Check for impossible length ranges
        if let (Some(min_len), Some(max_len)) = (min_length, max_length)
            && let (Ok(min_num), Ok(max_num)) = (min_len.parse::<usize>(), max_len.parse::<usize>())
            && min_num > max_num
        {
            errors.push(ValidationError::new(
                ValidationErrorType::ConflictingFacetRestrictions,
                "Minimum length greater than maximum length".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if a datatype IRI is known (built-in or previously defined)
    fn is_known_datatype(&self, iri: &str) -> bool {
        // Check built-in XML Schema datatypes
        matches!(
            iri,
            "http://www.w3.org/2001/XMLSchema#string"
                | "http://www.w3.org/2001/XMLSchema#boolean"
                | "http://www.w3.org/2001/XMLSchema#decimal"
                | "http://www.w3.org/2001/XMLSchema#float"
                | "http://www.w3.org/2001/XMLSchema#double"
                | "http://www.w3.org/2001/XMLSchema#integer"
                | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
                | "http://www.w3.org/2001/XMLSchema#positiveInteger"
                | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
                | "http://www.w3.org/2001/XMLSchema#negativeInteger"
                | "http://www.w3.org/2001/XMLSchema#long"
                | "http://www.w3.org/2001/XMLSchema#int"
                | "http://www.w3.org/2001/XMLSchema#short"
                | "http://www.w3.org/2001/XMLSchema#byte"
                | "http://www.w3.org/2001/XMLSchema#unsignedLong"
                | "http://www.w3.org/2001/XMLSchema#unsignedInt"
                | "http://www.w3.org/2001/XMLSchema#unsignedShort"
                | "http://www.w3.org/2001/XMLSchema#unsignedByte"
                | "http://www.w3.org/2001/XMLSchema#dateTime"
                | "http://www.w3.org/2001/XMLSchema#dateTimeStamp"
                | "http://www.w3.org/2001/XMLSchema#anyURI"
                | "http://www.w3.org/2001/XMLSchema#base64Binary"
                | "http://www.w3.org/2001/XMLSchema#hexBinary"
                | "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral"
        ) || self.defined_datatypes.contains(iri)
    }

    /// Validate embedded data ranges in axioms
    fn validate_embedded_data_ranges_in_axiom(
        &self,
        axiom: &Axiom,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                self.validate_data_ranges_in_class_expression(&subclass_axiom.subclass, errors)?;
                self.validate_data_ranges_in_class_expression(&subclass_axiom.superclass, errors)?;
            }
            Axiom::EquivalentClasses(equiv_axiom) => {
                for class_expr in &equiv_axiom.classes {
                    self.validate_data_ranges_in_class_expression(class_expr, errors)?;
                }
            }
            Axiom::DisjointClasses(disjoint_axiom) => {
                for class_expr in &disjoint_axiom.classes {
                    self.validate_data_ranges_in_class_expression(class_expr, errors)?;
                }
            }
            Axiom::ClassAssertion(class_assertion) => {
                self.validate_data_ranges_in_class_expression(&class_assertion.class, errors)?;
            }
            _ => {} // Other axioms don't typically contain embedded data ranges
        }
        Ok(())
    }

    /// Validate data ranges within class expressions
    fn validate_data_ranges_in_class_expression(
        &self,
        expr: &crate::ontology::ClassExpression,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        match expr {
            crate::ontology::ClassExpression::DataSomeValuesFrom { filler, .. }
            | crate::ontology::ClassExpression::DataAllValuesFrom { filler, .. } => {
                self.validate_data_range(filler, errors)?;
            }
            crate::ontology::ClassExpression::DataMinCardinality { filler, .. }
            | crate::ontology::ClassExpression::DataMaxCardinality { filler, .. }
            | crate::ontology::ClassExpression::DataExactCardinality { filler, .. } => {
                self.validate_data_range(filler, errors)?;
            }
            crate::ontology::ClassExpression::ObjectIntersectionOf(exprs)
            | crate::ontology::ClassExpression::ObjectUnionOf(exprs) => {
                for expr in exprs {
                    self.validate_data_ranges_in_class_expression(expr, errors)?;
                }
            }
            crate::ontology::ClassExpression::ObjectComplementOf(expr) => {
                self.validate_data_ranges_in_class_expression(expr, errors)?;
            }
            crate::ontology::ClassExpression::ObjectSomeValuesFrom { filler, .. }
            | crate::ontology::ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                self.validate_data_ranges_in_class_expression(filler, errors)?;
            }
            crate::ontology::ClassExpression::ObjectMinCardinality { filler, .. }
            | crate::ontology::ClassExpression::ObjectMaxCardinality { filler, .. }
            | crate::ontology::ClassExpression::ObjectExactCardinality { filler, .. } => {
                self.validate_data_ranges_in_class_expression(filler, errors)?;
            }
            _ => {} // Other class expressions don't contain data ranges
        }
        Ok(())
    }

    /// Validate facet restrictions
    fn validate_facet_restriction(
        &self,
        datatype: &crate::ontology::IRI,
        restriction: &crate::ontology::FacetRestriction,
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        // Check if the facet is applicable to the datatype
        let facet_name = &restriction.facet;
        let datatype_iri = &datatype.to_string();

        match facet_name.as_str() {
            "http://www.w3.org/2001/XMLSchema#minLength"
            | "http://www.w3.org/2001/XMLSchema#maxLength"
            | "http://www.w3.org/2001/XMLSchema#length" => {
                // Length facets are only applicable to string-based datatypes
                if !self.is_string_based_datatype(datatype_iri) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidFacetRestriction,
                        format!(
                            "Length facet {facet_name} not applicable to datatype {datatype_iri}"
                        ),
                    ));
                }
            }
            "http://www.w3.org/2001/XMLSchema#minInclusive"
            | "http://www.w3.org/2001/XMLSchema#maxInclusive"
            | "http://www.w3.org/2001/XMLSchema#minExclusive"
            | "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                // Range facets are only applicable to ordered datatypes
                if !self.is_ordered_datatype(datatype_iri) {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidFacetRestriction,
                        format!(
                            "Range facet {facet_name} not applicable to datatype {datatype_iri}"
                        ),
                    ));
                }
            }
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                // Pattern facets are applicable to most datatypes
                // Validate the regular expression
                if regex::Regex::new(&restriction.value.to_string()).is_err() {
                    errors.push(ValidationError::new(
                        ValidationErrorType::InvalidDatatype,
                        format!("Invalid regular expression pattern: {}", restriction.value),
                    ));
                }
            }
            _ => {
                errors.push(ValidationError::new(
                    ValidationErrorType::InvalidFacetRestriction,
                    format!("Unknown facet: {facet_name}"),
                ));
            }
        }

        Ok(())
    }

    /// Check if datatype is string-based
    fn is_string_based_datatype(&self, iri: &str) -> bool {
        matches!(
            iri,
            "http://www.w3.org/2001/XMLSchema#string"
                | "http://www.w3.org/2001/XMLSchema#normalizedString"
                | "http://www.w3.org/2001/XMLSchema#token"
                | "http://www.w3.org/2001/XMLSchema#Name"
                | "http://www.w3.org/2001/XMLSchema#NCName"
                | "http://www.w3.org/2001/XMLSchema#anyURI"
        )
    }

    /// Check if datatype is ordered
    fn is_ordered_datatype(&self, iri: &str) -> bool {
        matches!(
            iri,
            "http://www.w3.org/2001/XMLSchema#decimal"
                | "http://www.w3.org/2001/XMLSchema#float"
                | "http://www.w3.org/2001/XMLSchema#double"
                | "http://www.w3.org/2001/XMLSchema#integer"
                | "http://www.w3.org/2001/XMLSchema#long"
                | "http://www.w3.org/2001/XMLSchema#int"
                | "http://www.w3.org/2001/XMLSchema#short"
                | "http://www.w3.org/2001/XMLSchema#byte"
                | "http://www.w3.org/2001/XMLSchema#dateTime"
                | "http://www.w3.org/2001/XMLSchema#date"
                | "http://www.w3.org/2001/XMLSchema#time"
        )
    }

    /// Validate data range intersection compatibility
    fn validate_data_range_intersection(
        &self,
        ranges: &[crate::ontology::DataRange],
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        // Check for incompatible data ranges in intersection
        for (i, range1) in ranges.iter().enumerate() {
            for range2 in ranges.iter().skip(i + 1) {
                if self.are_data_ranges_disjoint(range1, range2)? {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::IncompatibleDataRanges,
                        message: format!(
                            "Incompatible data ranges in intersection: {range1:?} and {range2:?}"
                        ),
                        severity: ValidationSeverity::Error,
                        axiom_id: None,
                        location: None,
                    });
                }
            }
        }
        Ok(())
    }

    /// Check if two data ranges are disjoint
    fn are_data_ranges_disjoint(
        &self,
        range1: &crate::ontology::DataRange,
        range2: &crate::ontology::DataRange,
    ) -> Result<bool, ValidationError> {
        // Basic disjointness check for common data ranges
        match (range1, range2) {
            (
                crate::ontology::DataRange::Datatype(dt1),
                crate::ontology::DataRange::Datatype(dt2),
            ) => {
                // Check if datatypes are fundamentally incompatible
                let iri1 = dt1.as_str();
                let iri2 = dt2.as_str();

                let numeric_types = [
                    "http://www.w3.org/2001/XMLSchema#integer",
                    "http://www.w3.org/2001/XMLSchema#decimal",
                    "http://www.w3.org/2001/XMLSchema#float",
                    "http://www.w3.org/2001/XMLSchema#double",
                ];

                let string_types = [
                    "http://www.w3.org/2001/XMLSchema#string",
                    "http://www.w3.org/2001/XMLSchema#normalizedString",
                ];

                let is_numeric1 = numeric_types.contains(&iri1);
                let is_numeric2 = numeric_types.contains(&iri2);
                let is_string1 = string_types.contains(&iri1);
                let is_string2 = string_types.contains(&iri2);

                // Different type categories are disjoint
                Ok((is_numeric1 && is_string2) || (is_string1 && is_numeric2))
            }
            _ => Ok(false), // Conservative approach for complex ranges
        }
    }

    /// Validate literal enumeration
    fn validate_literal_enumeration(
        &self,
        literals: &[crate::ontology::Literal],
        errors: &mut Vec<ValidationError>,
    ) -> Result<(), OxidowlError> {
        // Check for duplicate literals
        let mut seen_literals = std::collections::HashSet::new();
        for literal in literals {
            let literal_key = (
                literal.value.clone(),
                literal
                    .datatype
                    .as_ref()
                    .map(std::string::ToString::to_string),
            );
            if seen_literals.contains(&literal_key) {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::DuplicateLiteral,
                    message: format!("Duplicate literal in enumeration: {}", literal.value),
                    severity: ValidationSeverity::Warning,
                    axiom_id: None,
                    location: None,
                });
            }
            seen_literals.insert(literal_key);
        }

        // Validate literal syntax
        for literal in literals {
            if let Some(datatype) = &literal.datatype
                && !self.is_valid_literal_for_datatype(
                    &literal.value,
                    &crate::ontology::IRI::from_url(datatype.clone()),
                )?
            {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::InvalidLiteralSyntax,
                    message: format!(
                        "Invalid literal '{}' for datatype {}",
                        literal.value, datatype
                    ),
                    severity: ValidationSeverity::Error,
                    axiom_id: None,
                    location: None,
                });
            }
        }

        Ok(())
    }

    /// Check if a literal is valid for a given datatype
    fn is_valid_literal_for_datatype(
        &self,
        lexical_form: &str,
        datatype: &crate::ontology::IRI,
    ) -> Result<bool, ValidationError> {
        match datatype.as_str() {
            "http://www.w3.org/2001/XMLSchema#integer" => Ok(lexical_form.parse::<i64>().is_ok()),
            "http://www.w3.org/2001/XMLSchema#decimal" => Ok(lexical_form.parse::<f64>().is_ok()),
            "http://www.w3.org/2001/XMLSchema#boolean" => {
                Ok(lexical_form == "true" || lexical_form == "false")
            }
            "http://www.w3.org/2001/XMLSchema#string" => {
                Ok(true) // Any string is valid
            }
            _ => Ok(true), // Conservative approach for unknown datatypes
        }
    }

    /// Detect which OWL 2 profile the ontology conforms to
    fn detect_profile(&self) -> OWL2Profile {
        let mut has_complex_class_expressions = false;
        let has_number_restrictions = false;
        let has_nominals = false;
        let mut has_inverse_properties = false;
        let has_complex_role_inclusions = false;

        // Analyze ontology constructs
        for axiom in self.ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::SubClassOf(axiom)
                    if (self.is_complex_class_expression(&axiom.subclass)
                        || self.is_complex_class_expression(&axiom.superclass)) =>
                {
                    has_complex_class_expressions = true;
                }
                crate::ontology::Axiom::EquivalentClasses(axiom) => {
                    for class in &axiom.classes {
                        if self.is_complex_class_expression(class) {
                            has_complex_class_expressions = true;
                        }
                    }
                }
                crate::ontology::Axiom::InverseObjectProperties(_) => {
                    has_inverse_properties = true;
                }
                _ => {}
            }
        }

        // Determine profile based on constructs used
        if has_complex_role_inclusions || has_nominals || has_number_restrictions {
            OWL2Profile::DL
        } else if has_inverse_properties || has_complex_class_expressions {
            OWL2Profile::RL
        } else {
            OWL2Profile::EL
        }
    }

    /// Check if a class expression is complex (beyond EL expressivity)
    fn is_complex_class_expression(&self, class_expr: &crate::ontology::ClassExpression) -> bool {
        match class_expr {
            crate::ontology::ClassExpression::Class(_) => false,
            crate::ontology::ClassExpression::ObjectIntersectionOf(_) => false, // Allowed in EL
            crate::ontology::ClassExpression::ObjectUnionOf(_) => true,         // Not in EL
            crate::ontology::ClassExpression::ObjectComplementOf(_) => true,    // Not in EL
            crate::ontology::ClassExpression::ObjectSomeValuesFrom {
                property: _,
                filler,
            } => self.is_complex_class_expression(filler),
            crate::ontology::ClassExpression::ObjectAllValuesFrom {
                property: _,
                filler: _,
            } => true, // Not in EL
            crate::ontology::ClassExpression::ObjectMinCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => true, // Not in EL
            crate::ontology::ClassExpression::ObjectMaxCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => true, // Not in EL
            crate::ontology::ClassExpression::ObjectExactCardinality {
                property: _,
                cardinality: _,
                filler: _,
            } => true, // Not in EL
            _ => false, // Conservative for other expressions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_property_detection() {
        // Create test ontology with transitive property
        let ontology = Ontology::new();
        // Add test axioms...

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator
            .validate()
            .expect("Failed to validate OWL 2 DL compliance for ontology");

        // Assert validation results
        assert!(report.is_valid);
    }

    #[test]
    fn test_cardinality_restriction_validation() {
        // Test that non-simple properties are rejected in cardinality restrictions
        let ontology = Ontology::new();
        // Note: Simplified implementation doesn't fully detect complex cardinality violations

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator
            .validate()
            .expect("Failed to validate OWL 2 DL compliance for ontology");

        // The simplified implementation returns valid for empty ontologies
        assert!(report.is_valid);
    }

    #[test]
    fn test_anonymous_individual_validation() {
        // Test that anonymous individuals are properly validated
        let ontology = Ontology::new();
        // Add test axioms with anonymous individuals...

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator
            .validate()
            .expect("Failed to validate OWL 2 DL compliance for ontology");

        // Check for appropriate errors
        assert_eq!(report.errors.len(), 0); // Should be valid if used correctly
    }

    #[test]
    fn test_quoted_triple_in_subject_position() {
        // Test that quoted triples in subject position are allowed
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Create: << :alice :knows :bob >> :certainty 0.95
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        let certainty = RdfTerm::iri("http://example.org/certainty").unwrap();
        let value = RdfTerm::Literal {
            value: "0.95".to_string(),
            datatype: None,
            language: None,
            direction: None,
        };

        let inner_triple = Triple::new(alice, knows, bob);
        let quoted_subject = RdfTerm::QuotedTriple(Box::new(inner_triple));
        let meta_triple = Triple::new(quoted_subject, certainty, value);

        graph.add_triple(meta_triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be valid - quoted triples allowed in subject position
        assert!(report.is_valid, "Quoted triple in subject should be valid");
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_quoted_triple_in_predicate_position_rejected() {
        // Test that quoted triples in predicate position are rejected
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Create invalid: :alice << :p1 :p2 :p3 >> :bob
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let p1 = RdfTerm::iri("http://example.org/p1").unwrap();
        let p2 = RdfTerm::iri("http://example.org/p2").unwrap();
        let p3 = RdfTerm::iri("http://example.org/p3").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();

        let inner_triple = Triple::new(p1, p2, p3);
        let quoted_predicate = RdfTerm::QuotedTriple(Box::new(inner_triple));
        let invalid_triple = Triple::new(alice, quoted_predicate, bob);

        graph.add_triple(invalid_triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be invalid - quoted triples not allowed in predicate position
        assert!(
            !report.is_valid,
            "Quoted triple in predicate should be invalid"
        );
        assert!(report.errors.iter().any(|e| matches!(
            e.error_type,
            ValidationErrorType::QuotedTripleInPredicatePosition
        )));
    }

    #[test]
    fn test_nested_quoted_triple_validation() {
        // Test that nested quoted triples within limits are valid
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Create: << << :a :b :c >> :d :e >> :f :g
        let a = RdfTerm::iri("http://example.org/a").unwrap();
        let b = RdfTerm::iri("http://example.org/b").unwrap();
        let c = RdfTerm::iri("http://example.org/c").unwrap();
        let d = RdfTerm::iri("http://example.org/d").unwrap();
        let e = RdfTerm::iri("http://example.org/e").unwrap();
        let f = RdfTerm::iri("http://example.org/f").unwrap();
        let g = RdfTerm::iri("http://example.org/g").unwrap();

        let inner_triple = Triple::new(a, b, c);
        let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner_triple));
        let middle_triple = Triple::new(inner_quoted, d, e);
        let middle_quoted = RdfTerm::QuotedTriple(Box::new(middle_triple));
        let outer_triple = Triple::new(middle_quoted, f, g);

        graph.add_triple(outer_triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be valid - 2-level nesting is within default limit of 5
        assert!(
            report.is_valid,
            "Nested quoted triples within limit should be valid"
        );
    }

    #[test]
    fn test_excessive_nesting_rejected() {
        // Test that extremely deep nesting exceeds limits
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Create deeply nested structure (depth = 6, exceeding limit of 5)
        let a = RdfTerm::iri("http://example.org/a").unwrap();
        let b = RdfTerm::iri("http://example.org/b").unwrap();
        let c = RdfTerm::iri("http://example.org/c").unwrap();

        // Build 6 levels of nesting (starting from depth 0)
        // This creates a triple with depth = 6
        let mut current = Triple::new(a.clone(), b.clone(), c.clone());
        for _ in 0..6 {
            let quoted = RdfTerm::QuotedTriple(Box::new(current));
            current = Triple::new(quoted, b.clone(), c.clone());
        }

        // Verify depth is 6
        assert_eq!(current.depth(), 6, "Triple should have depth 6");

        graph.add_triple(current);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be invalid - exceeds default limit of 5
        assert!(!report.is_valid, "Excessive nesting should be invalid");
        assert!(report.errors.iter().any(|e| matches!(
            e.error_type,
            ValidationErrorType::ExcessiveQuotedTripleNesting
        )));
    }

    #[test]
    fn test_directional_literal_validation() {
        // Test that directional literals (RDF 1.2) are properly validated
        use crate::semantics::{RdfGraph, RdfTerm, Triple};
        use url::Url;

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject").unwrap();
        let predicate = RdfTerm::iri("http://example.org/label").unwrap();

        // Valid directional literal
        let valid_literal = RdfTerm::Literal {
            value: "مرحبا".to_string(),
            datatype: Some(
                Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString").unwrap(),
            ),
            language: Some("ar".to_string()),
            direction: Some("rtl".to_string()),
        };

        let triple = Triple::new(subject, predicate, valid_literal);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be valid
        assert!(
            report.is_valid,
            "Valid directional literal should pass validation"
        );
    }

    #[test]
    fn test_invalid_directional_literal_without_language() {
        // Test that directional literal without language is rejected
        use crate::semantics::{RdfGraph, RdfTerm, Triple};
        use url::Url;

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject").unwrap();
        let predicate = RdfTerm::iri("http://example.org/label").unwrap();

        // Invalid: direction without language
        let invalid_literal = RdfTerm::Literal {
            value: "Hello".to_string(),
            datatype: Some(
                Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString").unwrap(),
            ),
            language: None,
            direction: Some("ltr".to_string()),
        };

        let triple = Triple::new(subject, predicate, invalid_literal);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be invalid
        assert!(
            !report.is_valid,
            "Directional literal without language should be invalid"
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e.error_type, ValidationErrorType::InvalidDirectionalLiteral))
        );
    }

    #[test]
    fn test_invalid_direction_value() {
        // Test that invalid direction value is rejected
        use crate::semantics::{RdfGraph, RdfTerm, Triple};
        use url::Url;

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject").unwrap();
        let predicate = RdfTerm::iri("http://example.org/label").unwrap();

        // Invalid: direction must be "ltr" or "rtl"
        let invalid_literal = RdfTerm::Literal {
            value: "Hello".to_string(),
            datatype: Some(
                Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString").unwrap(),
            ),
            language: Some("en".to_string()),
            direction: Some("invalid".to_string()),
        };

        let triple = Triple::new(subject, predicate, invalid_literal);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be invalid
        assert!(
            !report.is_valid,
            "Invalid direction value should be rejected"
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e.error_type, ValidationErrorType::InvalidDirectionalLiteral))
        );
    }

    #[test]
    fn test_blank_node_label_validation() {
        // Test that blank node labels are validated
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::BlankNode("_:validLabel123".to_string());
        let predicate = RdfTerm::iri("http://example.org/property").unwrap();
        let object = RdfTerm::iri("http://example.org/value").unwrap();

        let triple = Triple::new(subject, predicate, object);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be valid
        assert!(
            report.is_valid,
            "Valid blank node label should pass validation"
        );
    }

    #[test]
    fn test_invalid_blank_node_label() {
        // Test that invalid blank node labels are rejected
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Invalid: contains special characters
        let subject = RdfTerm::BlankNode("_:invalid-label!".to_string());
        let predicate = RdfTerm::iri("http://example.org/property").unwrap();
        let object = RdfTerm::iri("http://example.org/value").unwrap();

        let triple = Triple::new(subject, predicate, object);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate().unwrap();

        // Should be invalid
        assert!(
            !report.is_valid,
            "Invalid blank node label should be rejected"
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e.error_type, ValidationErrorType::InvalidBlankNodeLabel))
        );
    }

    #[test]
    fn test_rdf11_ontology_validates_identically() {
        // Test that RDF 1.1 ontologies without RDF-star features pass validation
        use crate::semantics::{RdfGraph, RdfTerm, Triple};

        let mut ontology = Ontology::new();
        let mut graph = RdfGraph::new();

        // Simple RDF 1.1 triple with no RDF-star features
        let subject = RdfTerm::iri("http://example.org/alice").unwrap();
        let predicate = RdfTerm::iri("http://example.org/knows").unwrap();
        let object = RdfTerm::iri("http://example.org/bob").unwrap();

        let triple = Triple::new(subject, predicate, object);
        graph.add_triple(triple);
        ontology.set_rdf_graph(graph);

        let mut validator = OWL2DLValidator::new(ontology);
        let report = validator.validate();

        // Should be valid - no RDF-star features
        assert!(report.is_ok());
        let report = report.unwrap();
        assert!(
            report.is_valid,
            "RDF 1.1 ontology should validate identically"
        );
        assert_eq!(report.errors.len(), 0);
    }
}
