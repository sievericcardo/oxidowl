//! Visitor Pattern for Ontology Traversal
//!
//! This module implements the visitor pattern for traversing and manipulating
//! OWL ontologies, providing functionality similar to horned-owl's visitor traits.

use crate::Result;
use crate::ontology::axioms::{
    AnnotationAssertionAxiom, AnnotationPropertyDomainAxiom, AnnotationPropertyRangeAxiom,
    AsymmetricObjectPropertyAxiom, Axiom, ClassAssertionAxiom, DataPropertyAssertionAxiom,
    DataPropertyDomainAxiom, DataPropertyRangeAxiom, DeclarationAxiom, DifferentIndividualsAxiom,
    DisjointClassesAxiom, DisjointDataPropertiesAxiom, DisjointObjectPropertiesAxiom,
    DisjointUnionAxiom, Entity, EquivalentClassesAxiom, EquivalentDataPropertiesAxiom,
    EquivalentObjectPropertiesAxiom, FunctionalDataPropertyAxiom, FunctionalObjectPropertyAxiom,
    HasKeyAxiom, InverseFunctionalObjectPropertyAxiom, InverseObjectPropertiesAxiom,
    IrreflexiveObjectPropertyAxiom, NegativeDataPropertyAssertionAxiom,
    NegativeObjectPropertyAssertionAxiom, ObjectPropertyAssertionAxiom, ObjectPropertyDomainAxiom,
    ObjectPropertyRangeAxiom, ReflexiveObjectPropertyAxiom, SWRLAtom, SWRLDArgument, SWRLIArgument,
    SWRLRule, SWRLRuleAxiom, SWRLVariable, SameIndividualAxiom, SubAnnotationPropertyOfAxiom,
    SubClassOfAxiom, SubDataPropertyOfAxiom, SubObjectPropertyOfAxiom,
    SymmetricObjectPropertyAxiom, TransitiveObjectPropertyAxiom,
};
use crate::ontology::{
    Annotation, AnnotationProperty, AnnotationSubject, AnnotationValue, ClassExpression,
    DataPropertyExpression, DataRange, Individual, Literal, ObjectPropertyExpression, Ontology,
};

/// Visitor trait for traversing ontology components
pub trait OntologyVisitor<R = ()> {
    /// Visit an ontology
    fn visit_ontology(&mut self, ontology: &Ontology) -> Result<R> {
        self.visit_ontology_default(ontology)
    }

    /// Default implementation for visiting an ontology
    fn visit_ontology_default(&mut self, ontology: &Ontology) -> Result<R> {
        for axiom in ontology.axioms() {
            self.visit_axiom(axiom)?;
        }
        Ok(self.default_result())
    }

    /// Visit an axiom
    fn visit_axiom(&mut self, axiom: &Axiom) -> Result<()> {
        match axiom {
            Axiom::Declaration(decl) => self.visit_declaration_axiom(decl),
            Axiom::SubClassOf(subclass) => self.visit_subclass_axiom(subclass),
            Axiom::EquivalentClasses(equiv) => self.visit_equivalent_classes_axiom(equiv),
            Axiom::DisjointClasses(disjoint) => self.visit_disjoint_classes_axiom(disjoint),
            Axiom::DisjointUnion(union) => self.visit_disjoint_union_axiom(union),
            Axiom::SubObjectPropertyOf(subprop) => self.visit_sub_object_property_axiom(subprop),
            Axiom::EquivalentObjectProperties(equiv) => {
                self.visit_equivalent_object_properties_axiom(equiv)
            }
            Axiom::DisjointObjectProperties(disjoint) => {
                self.visit_disjoint_object_properties_axiom(disjoint)
            }
            Axiom::InverseObjectProperties(inverse) => {
                self.visit_inverse_object_properties_axiom(inverse)
            }
            Axiom::ObjectPropertyDomain(domain) => self.visit_object_property_domain_axiom(domain),
            Axiom::ObjectPropertyRange(range) => self.visit_object_property_range_axiom(range),
            Axiom::FunctionalObjectProperty(func) => {
                self.visit_functional_object_property_axiom(func)
            }
            Axiom::InverseFunctionalObjectProperty(inv_func) => {
                self.visit_inverse_functional_object_property_axiom(inv_func)
            }
            Axiom::ReflexiveObjectProperty(refl) => {
                self.visit_reflexive_object_property_axiom(refl)
            }
            Axiom::IrreflexiveObjectProperty(irrefl) => {
                self.visit_irreflexive_object_property_axiom(irrefl)
            }
            Axiom::SymmetricObjectProperty(sym) => self.visit_symmetric_object_property_axiom(sym),
            Axiom::AsymmetricObjectProperty(asym) => {
                self.visit_asymmetric_object_property_axiom(asym)
            }
            Axiom::TransitiveObjectProperty(trans) => {
                self.visit_transitive_object_property_axiom(trans)
            }
            Axiom::SubDataPropertyOf(subdata) => self.visit_sub_data_property_axiom(subdata),
            Axiom::EquivalentDataProperties(equiv_data) => {
                self.visit_equivalent_data_properties_axiom(equiv_data)
            }
            Axiom::DisjointDataProperties(disjoint_data) => {
                self.visit_disjoint_data_properties_axiom(disjoint_data)
            }
            Axiom::DataPropertyDomain(data_domain) => {
                self.visit_data_property_domain_axiom(data_domain)
            }
            Axiom::DataPropertyRange(data_range) => {
                self.visit_data_property_range_axiom(data_range)
            }
            Axiom::FunctionalDataProperty(func_data) => {
                self.visit_functional_data_property_axiom(func_data)
            }
            Axiom::SameIndividual(same) => self.visit_same_individual_axiom(same),
            Axiom::DifferentIndividuals(diff) => self.visit_different_individuals_axiom(diff),
            Axiom::ClassAssertion(axiom) => {
                self.visit_class_expression(&axiom.class)?;
                Ok(())
            }
            Axiom::ObjectPropertyAssertion(obj_assert) => {
                self.visit_object_property_assertion_axiom(obj_assert)
            }
            Axiom::DataPropertyAssertion(data_assert) => {
                self.visit_data_property_assertion_axiom(data_assert)
            }
            Axiom::NegativeObjectPropertyAssertion(neg_obj) => {
                self.visit_negative_object_property_assertion_axiom(neg_obj)
            }
            Axiom::NegativeDataPropertyAssertion(neg_data) => {
                self.visit_negative_data_property_assertion_axiom(neg_data)
            }
            Axiom::AnnotationAssertion(ann_assert) => {
                self.visit_annotation_assertion_axiom(ann_assert)
            }
            Axiom::SubAnnotationPropertyOf(sub_ann) => {
                self.visit_sub_annotation_property_axiom(sub_ann)
            }
            Axiom::AnnotationPropertyDomain(ann_domain) => {
                self.visit_annotation_property_domain_axiom(ann_domain)
            }
            Axiom::AnnotationPropertyRange(ann_range) => {
                self.visit_annotation_property_range_axiom(ann_range)
            }
            Axiom::Rule(rule) => self.visit_swrl_rule_axiom(rule),
            Axiom::HasKey(haskey) => self.visit_haskey_axiom(haskey),
            Axiom::DatatypeDefinition(dt_def) => self.visit_datatype_definition_axiom(dt_def),
        }
    }

    /// Visit a class expression
    fn visit_class_expression(&mut self, expr: &ClassExpression) -> Result<()> {
        match expr {
            ClassExpression::Class(class) => self.visit_class(class),
            ClassExpression::ObjectIntersectionOf(expressions) => {
                for expr in expressions {
                    self.visit_class_expression(expr)?;
                }
                Ok(())
            }
            ClassExpression::ObjectUnionOf(expressions) => {
                for expr in expressions {
                    self.visit_class_expression(expr)?;
                }
                Ok(())
            }
            ClassExpression::ObjectComplementOf(expr) => self.visit_class_expression(expr),
            ClassExpression::ObjectOneOf(individuals) => {
                for individual in individuals {
                    self.visit_individual(individual)?;
                }
                Ok(())
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.visit_object_property_expression(property)?;
                self.visit_class_expression(filler)
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.visit_object_property_expression(property)?;
                self.visit_class_expression(filler)
            }
            ClassExpression::ObjectHasValue { property, value } => {
                self.visit_object_property_expression(property)?;
                self.visit_individual(value)
            }
            ClassExpression::ObjectHasSelf { property } => {
                self.visit_object_property_expression(property)
            }
            ClassExpression::ObjectMinCardinality {
                property, filler, ..
            }
            | ClassExpression::ObjectMaxCardinality {
                property, filler, ..
            }
            | ClassExpression::ObjectExactCardinality {
                property, filler, ..
            } => {
                self.visit_object_property_expression(property)?;
                self.visit_class_expression(filler)
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                self.visit_data_property_expression(property)?;
                self.visit_data_range(filler)
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                self.visit_data_property_expression(property)?;
                self.visit_data_range(filler)
            }
            ClassExpression::DataHasValue { property, value } => {
                self.visit_data_property_expression(property)?;
                self.visit_literal(value)
            }
            ClassExpression::DataMinCardinality {
                property, filler, ..
            }
            | ClassExpression::DataMaxCardinality {
                property, filler, ..
            }
            | ClassExpression::DataExactCardinality {
                property, filler, ..
            } => {
                self.visit_data_property_expression(property)?;
                self.visit_data_range(filler)
            }
        }
    }

    /// Visit SWRL rule axiom
    fn visit_swrl_rule_axiom(&mut self, rule_axiom: &SWRLRuleAxiom) -> Result<()> {
        self.visit_swrl_rule(&rule_axiom.rule)
    }

    /// Visit SWRL rule
    fn visit_swrl_rule(&mut self, rule: &SWRLRule) -> Result<()> {
        for atom in &rule.head {
            self.visit_swrl_atom(atom)?;
        }
        for atom in &rule.body {
            self.visit_swrl_atom(atom)?;
        }
        Ok(())
    }

    /// Visit SWRL atom
    fn visit_swrl_atom(&mut self, atom: &SWRLAtom) -> Result<()> {
        match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => {
                self.visit_class_expression(predicate)?;
                self.visit_swrl_iargument(argument)
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                self.visit_object_property_expression(predicate)?;
                self.visit_swrl_iargument(first_argument)?;
                self.visit_swrl_iargument(second_argument)
            }
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                self.visit_data_property_expression(predicate)?;
                self.visit_swrl_iargument(first_argument)?;
                self.visit_swrl_dargument(second_argument)
            }
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => {
                self.visit_data_range(predicate)?;
                self.visit_swrl_dargument(argument)
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => {
                self.visit_swrl_iargument(first_argument)?;
                self.visit_swrl_iargument(second_argument)
            }
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                self.visit_swrl_iargument(first_argument)?;
                self.visit_swrl_iargument(second_argument)
            }
            SWRLAtom::BuiltInAtom {
                predicate: _,
                arguments,
            } => {
                for arg in arguments {
                    self.visit_swrl_dargument(arg)?;
                }
                Ok(())
            }
        }
    }

    /// Visit SWRL individual argument
    fn visit_swrl_iargument(&mut self, arg: &SWRLIArgument) -> Result<()> {
        match arg {
            SWRLIArgument::Individual(individual) => self.visit_individual(individual),
            SWRLIArgument::Variable(variable) => self.visit_swrl_variable(variable),
        }
    }

    /// Visit SWRL data argument
    fn visit_swrl_dargument(&mut self, arg: &SWRLDArgument) -> Result<()> {
        match arg {
            SWRLDArgument::Literal(literal) => self.visit_literal(literal),
            SWRLDArgument::Variable(variable) => self.visit_swrl_variable(variable),
        }
    }

    /// Visit SWRL variable
    fn visit_swrl_variable(&mut self, _variable: &SWRLVariable) -> Result<()> {
        Ok(())
    }

    // Default implementations for visiting specific axiom types
    fn visit_declaration_axiom(&mut self, _axiom: &DeclarationAxiom) -> Result<()> {
        Ok(())
    }
    fn visit_subclass_axiom(&mut self, axiom: &SubClassOfAxiom) -> Result<()> {
        self.visit_class_expression(&axiom.subclass)?;
        self.visit_class_expression(&axiom.superclass)
    }
    fn visit_equivalent_classes_axiom(&mut self, axiom: &EquivalentClassesAxiom) -> Result<()> {
        for class in &axiom.classes {
            self.visit_class_expression(class)?;
        }
        Ok(())
    }
    fn visit_disjoint_classes_axiom(&mut self, axiom: &DisjointClassesAxiom) -> Result<()> {
        for class in &axiom.classes {
            self.visit_class_expression(class)?;
        }
        Ok(())
    }
    fn visit_disjoint_union_axiom(&mut self, axiom: &DisjointUnionAxiom) -> Result<()> {
        self.visit_class_expression(&axiom.class)?;
        for class in &axiom.disjoint_classes {
            self.visit_class_expression(class)?;
        }
        Ok(())
    }

    // Object property axioms
    fn visit_sub_object_property_axiom(&mut self, axiom: &SubObjectPropertyOfAxiom) -> Result<()> {
        self.visit_object_property_expression(&axiom.sub_property)?;
        self.visit_object_property_expression(&axiom.super_property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_equivalent_object_properties_axiom(
        &mut self,
        axiom: &EquivalentObjectPropertiesAxiom,
    ) -> Result<()> {
        for property in &axiom.properties {
            self.visit_object_property_expression(property)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_disjoint_object_properties_axiom(
        &mut self,
        axiom: &DisjointObjectPropertiesAxiom,
    ) -> Result<()> {
        for property in &axiom.properties {
            self.visit_object_property_expression(property)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_inverse_object_properties_axiom(
        &mut self,
        axiom: &InverseObjectPropertiesAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property1)?;
        self.visit_object_property_expression(&axiom.property2)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_object_property_domain_axiom(
        &mut self,
        axiom: &ObjectPropertyDomainAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        self.visit_class_expression(&axiom.domain)
    }
    fn visit_object_property_range_axiom(
        &mut self,
        axiom: &ObjectPropertyRangeAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        self.visit_class_expression(&axiom.range)
    }
    fn visit_functional_object_property_axiom(
        &mut self,
        axiom: &FunctionalObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_inverse_functional_object_property_axiom(
        &mut self,
        axiom: &InverseFunctionalObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_reflexive_object_property_axiom(
        &mut self,
        axiom: &ReflexiveObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_irreflexive_object_property_axiom(
        &mut self,
        axiom: &IrreflexiveObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_symmetric_object_property_axiom(
        &mut self,
        axiom: &SymmetricObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_asymmetric_object_property_axiom(
        &mut self,
        axiom: &AsymmetricObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_transitive_object_property_axiom(
        &mut self,
        axiom: &TransitiveObjectPropertyAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }

    // Data property axioms
    fn visit_sub_data_property_axiom(&mut self, axiom: &SubDataPropertyOfAxiom) -> Result<()> {
        self.visit_data_property_expression(&axiom.sub_property)?;
        self.visit_data_property_expression(&axiom.super_property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_equivalent_data_properties_axiom(
        &mut self,
        axiom: &EquivalentDataPropertiesAxiom,
    ) -> Result<()> {
        for property in &axiom.properties {
            self.visit_data_property_expression(property)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_disjoint_data_properties_axiom(
        &mut self,
        axiom: &DisjointDataPropertiesAxiom,
    ) -> Result<()> {
        for property in &axiom.properties {
            self.visit_data_property_expression(property)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_data_property_domain_axiom(&mut self, axiom: &DataPropertyDomainAxiom) -> Result<()> {
        self.visit_data_property_expression(&axiom.property)?;
        self.visit_class_expression(&axiom.domain)
    }
    fn visit_data_property_range_axiom(&mut self, axiom: &DataPropertyRangeAxiom) -> Result<()> {
        self.visit_data_property_expression(&axiom.property)?;
        self.visit_data_range(&axiom.range)
    }
    fn visit_functional_data_property_axiom(
        &mut self,
        axiom: &FunctionalDataPropertyAxiom,
    ) -> Result<()> {
        self.visit_data_property_expression(&axiom.property)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }

    // Individual axioms
    fn visit_same_individual_axiom(&mut self, axiom: &SameIndividualAxiom) -> Result<()> {
        for individual in &axiom.individuals {
            self.visit_individual(individual)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_different_individuals_axiom(
        &mut self,
        axiom: &DifferentIndividualsAxiom,
    ) -> Result<()> {
        for individual in &axiom.individuals {
            self.visit_individual(individual)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_class_assertion_axiom(&mut self, axiom: &ClassAssertionAxiom) -> Result<()> {
        self.visit_class_expression(&axiom.class)?;
        self.visit_individual(&axiom.individual)
    }
    fn visit_object_property_assertion_axiom(
        &mut self,
        axiom: &ObjectPropertyAssertionAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        self.visit_individual(&axiom.source)?;
        self.visit_individual(&axiom.target)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_data_property_assertion_axiom(
        &mut self,
        axiom: &DataPropertyAssertionAxiom,
    ) -> Result<()> {
        self.visit_data_property_expression(&axiom.property)?;
        self.visit_individual(&axiom.individual)?;
        self.visit_literal(&axiom.value)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_negative_object_property_assertion_axiom(
        &mut self,
        axiom: &NegativeObjectPropertyAssertionAxiom,
    ) -> Result<()> {
        self.visit_object_property_expression(&axiom.property)?;
        self.visit_individual(&axiom.source)?;
        self.visit_individual(&axiom.target)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_negative_data_property_assertion_axiom(
        &mut self,
        axiom: &NegativeDataPropertyAssertionAxiom,
    ) -> Result<()> {
        self.visit_data_property_expression(&axiom.property)?;
        self.visit_individual(&axiom.individual)?;
        self.visit_literal(&axiom.value)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }

    // Annotation axioms
    fn visit_annotation_assertion_axiom(&mut self, axiom: &AnnotationAssertionAxiom) -> Result<()> {
        // Visit the subject, property, and value
        self.visit_annotation_subject(&axiom.subject)?;
        self.visit_annotation_property(&axiom.property)?;
        self.visit_annotation_value(&axiom.value)?;
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_sub_annotation_property_axiom(
        &mut self,
        axiom: &SubAnnotationPropertyOfAxiom,
    ) -> Result<()> {
        // Visit the property IRIs
        // Note: In a real implementation, you might want to visit the properties
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_annotation_property_domain_axiom(
        &mut self,
        axiom: &AnnotationPropertyDomainAxiom,
    ) -> Result<()> {
        // Visit the domain IRI
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }
    fn visit_annotation_property_range_axiom(
        &mut self,
        axiom: &AnnotationPropertyRangeAxiom,
    ) -> Result<()> {
        // Visit the range IRI
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }

    fn visit_haskey_axiom(&mut self, axiom: &HasKeyAxiom) -> Result<()> {
        self.visit_class_expression(&axiom.class)?;
        for obj_prop in &axiom.object_properties {
            self.visit_object_property_expression(obj_prop)?;
        }
        for data_prop in &axiom.data_properties {
            self.visit_data_property_expression(data_prop)?;
        }
        for annotation in &axiom.annotations {
            self.visit_annotation(annotation)?;
        }
        Ok(())
    }

    fn visit_datatype_definition_axiom(
        &mut self,
        axiom: &crate::ontology::datatypes::DatatypeDefinitionAxiom,
    ) -> Result<()> {
        // Visit the datatype IRI
        // (datatype field is IRI<String>, which we can't directly visit but can process)

        // Visit the horned_owl data_range by converting it to oxidowl DataRange
        self.visit_horned_owl_data_range(&axiom.data_range)?;

        // Visit annotations
        for annotation in &axiom.annotations {
            self.visit_horned_owl_annotation(annotation)?;
        }

        Ok(())
    }

    /// Visit a horned_owl DataRange structure
    ///
    /// This helper method traverses horned_owl::model::DataRange types and
    /// recursively processes their components. It enables the visitor pattern
    /// to work with foreign horned_owl types embedded in oxidowl ontologies.
    fn visit_horned_owl_data_range(
        &mut self,
        data_range: &horned_owl::model::DataRange<String>,
    ) -> Result<()> {
        use horned_owl::model::DataRange;

        // Recursively traverse the data range structure
        // This provides a hook for visitors to process horned_owl DataRange types
        match data_range {
            DataRange::Datatype(_dt) => {
                // Visit datatype IRI - implementers can override to track
                Ok(())
            }
            DataRange::DataIntersectionOf(ranges) => {
                // Recursively visit all ranges in the intersection
                for range in ranges {
                    self.visit_horned_owl_data_range(range)?;
                }
                Ok(())
            }
            DataRange::DataUnionOf(ranges) => {
                // Recursively visit all ranges in the union
                for range in ranges {
                    self.visit_horned_owl_data_range(range)?;
                }
                Ok(())
            }
            DataRange::DataComplementOf(range) => {
                // Recursively visit the complemented range
                self.visit_horned_owl_data_range(range)?;
                Ok(())
            }
            DataRange::DataOneOf(_literals) => {
                // Visit enumerated literals - implementers can override to count
                Ok(())
            }
            DataRange::DatatypeRestriction(_dt, _facet_restrictions) => {
                // Visit datatype restrictions - implementers can override to count
                Ok(())
            }
        }
    }

    /// Visit a horned_owl Annotation structure
    fn visit_horned_owl_annotation(
        &mut self,
        _annotation: &horned_owl::model::Annotation<String>,
    ) -> Result<()> {
        // horned_owl Annotation has fields: ap (annotation property) and av (annotation value)
        // We provide a default no-op implementation
        // Specific visitors can override to extract property/value information
        Ok(())
    }

    // Visit component types
    fn visit_class(&mut self, _class: &crate::ontology::Class) -> Result<()> {
        Ok(())
    }
    fn visit_object_property_expression(&mut self, _prop: &ObjectPropertyExpression) -> Result<()> {
        Ok(())
    }
    fn visit_data_property_expression(&mut self, _prop: &DataPropertyExpression) -> Result<()> {
        Ok(())
    }
    fn visit_individual(&mut self, _individual: &Individual) -> Result<()> {
        Ok(())
    }
    fn visit_literal(&mut self, _literal: &Literal) -> Result<()> {
        Ok(())
    }
    fn visit_data_range(&mut self, _range: &DataRange) -> Result<()> {
        Ok(())
    }
    fn visit_annotation(&mut self, _annotation: &Annotation) -> Result<()> {
        Ok(())
    }
    fn visit_annotation_subject(&mut self, _subject: &AnnotationSubject) -> Result<()> {
        Ok(())
    }
    fn visit_annotation_value(&mut self, _value: &AnnotationValue) -> Result<()> {
        Ok(())
    }
    fn visit_annotation_property(&mut self, _property: &AnnotationProperty) -> Result<()> {
        Ok(())
    }

    /// Default result value
    fn default_result(&self) -> R;
}

/// Statistics visitor for gathering ontology metrics
#[derive(Debug, Default)]
pub struct StatisticsVisitor {
    pub class_count: usize,
    pub object_property_count: usize,
    pub data_property_count: usize,
    pub individual_count: usize,
    pub axiom_count: usize,
    pub swrl_rule_count: usize,
    pub logical_axiom_count: usize,
    pub annotation_axiom_count: usize,
}

impl OntologyVisitor<StatisticsVisitor> for StatisticsVisitor {
    fn visit_ontology(&mut self, ontology: &Ontology) -> Result<StatisticsVisitor> {
        self.axiom_count = ontology.axioms().len();
        // Reset counts for new statistics gathering
        self.class_count = 0;
        self.object_property_count = 0;

        // Visit each axiom to count specific types
        for axiom in ontology.axioms() {
            self.visit_axiom(axiom)?;
        }

        Ok(StatisticsVisitor {
            class_count: self.class_count,
            object_property_count: self.object_property_count,
            data_property_count: self.data_property_count,
            individual_count: self.individual_count,
            axiom_count: self.axiom_count,
            swrl_rule_count: self.swrl_rule_count,
            logical_axiom_count: self.logical_axiom_count,
            annotation_axiom_count: self.annotation_axiom_count,
        })
    }

    fn visit_declaration_axiom(&mut self, axiom: &DeclarationAxiom) -> Result<()> {
        match &axiom.entity {
            Entity::Class(_) => self.class_count += 1,
            Entity::ObjectProperty(_) => self.object_property_count += 1,
            Entity::DataProperty(_) => self.data_property_count += 1,
            Entity::NamedIndividual(_) => self.individual_count += 1,
            _ => {}
        }
        Ok(())
    }

    fn visit_swrl_rule_axiom(&mut self, rule_axiom: &SWRLRuleAxiom) -> Result<()> {
        self.swrl_rule_count += 1;
        self.visit_swrl_rule(&rule_axiom.rule)
    }

    fn default_result(&self) -> StatisticsVisitor {
        StatisticsVisitor::default()
    }
}

/// Class collector visitor for finding all classes in an ontology
#[derive(Debug, Default)]
pub struct ClassCollector {
    pub classes: std::collections::HashSet<crate::ontology::Class>,
}

impl OntologyVisitor<std::collections::HashSet<crate::ontology::Class>> for ClassCollector {
    fn visit_class(&mut self, class: &crate::ontology::Class) -> Result<()> {
        self.classes.insert(class.clone());
        Ok(())
    }

    fn default_result(&self) -> std::collections::HashSet<crate::ontology::Class> {
        self.classes.clone()
    }
}

/// Visitor interface for OntologyChange variants.
pub trait OntologyChangeVisitor {
    fn visit_add_axiom(&mut self, _ontology_iri: &crate::ontology::IRI, _axiom: &Axiom) {}
    fn visit_remove_axiom(&mut self, _ontology_iri: &crate::ontology::IRI, _axiom: &Axiom) {}
    fn visit_add_import(&mut self, _ontology_iri: &crate::ontology::IRI, _import: &crate::import::ImportDeclaration) {}
    fn visit_remove_import(&mut self, _ontology_iri: &crate::ontology::IRI, _import: &crate::import::ImportDeclaration) {}
    fn visit_add_ontology_annotation(
        &mut self,
        _ontology_iri: &crate::ontology::IRI,
        _annotation: &Annotation,
    ) {
    }
    fn visit_remove_ontology_annotation(
        &mut self,
        _ontology_iri: &crate::ontology::IRI,
        _annotation: &Annotation,
    ) {
    }
    fn visit_set_ontology_id(
        &mut self,
        _ontology_iri: &crate::ontology::IRI,
        _new_iri: &crate::ontology::IRI,
        _new_version_iri: &Option<crate::ontology::IRI>,
    ) {
    }
}

/// Dispatch an OntologyChange to the appropriate visitor method.
pub fn dispatch_change(change: &crate::manager::changes::OntologyChange, visitor: &mut dyn OntologyChangeVisitor) {
    match change {
        crate::manager::changes::OntologyChange::AddAxiom {
            ontology_iri,
            axiom,
        } => visitor.visit_add_axiom(ontology_iri, axiom),
        crate::manager::changes::OntologyChange::RemoveAxiom {
            ontology_iri,
            axiom,
        } => visitor.visit_remove_axiom(ontology_iri, axiom),
        crate::manager::changes::OntologyChange::AddImport {
            ontology_iri,
            import,
        } => visitor.visit_add_import(ontology_iri, import),
        crate::manager::changes::OntologyChange::RemoveImport {
            ontology_iri,
            import,
        } => visitor.visit_remove_import(ontology_iri, import),
        crate::manager::changes::OntologyChange::AddOntologyAnnotation {
            ontology_iri,
            annotation,
        } => visitor.visit_add_ontology_annotation(ontology_iri, annotation),
        crate::manager::changes::OntologyChange::RemoveOntologyAnnotation {
            ontology_iri,
            annotation,
        } => visitor.visit_remove_ontology_annotation(ontology_iri, annotation),
        crate::manager::changes::OntologyChange::SetOntologyId {
            ontology_iri,
            new_iri,
            new_version_iri,
        } => visitor.visit_set_ontology_id(ontology_iri, new_iri, new_version_iri),
    }
}
