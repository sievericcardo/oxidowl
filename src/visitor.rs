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
    InverseFunctionalObjectPropertyAxiom, InverseObjectPropertiesAxiom,
    IrreflexiveObjectPropertyAxiom, NegativeDataPropertyAssertionAxiom,
    NegativeObjectPropertyAssertionAxiom, ObjectPropertyAssertionAxiom, ObjectPropertyDomainAxiom,
    ObjectPropertyRangeAxiom, ReflexiveObjectPropertyAxiom, SWRLAtom, SWRLDArgument, SWRLIArgument,
    SWRLRule, SWRLRuleAxiom, SWRLVariable, SameIndividualAxiom, SubAnnotationPropertyOfAxiom,
    SubClassOfAxiom, SubDataPropertyOfAxiom, SubObjectPropertyOfAxiom,
    SymmetricObjectPropertyAxiom, TransitiveObjectPropertyAxiom,
};
use crate::ontology::{
    Annotation, ClassExpression, DataPropertyExpression, DataRange, Individual, Literal,
    ObjectPropertyExpression, Ontology,
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
            // Handle annotation axiom class expressions (unusual but possible)
            ClassExpression::AnnotationAssertion { .. }
            | ClassExpression::SubAnnotationPropertyOf { .. }
            | ClassExpression::AnnotationPropertyDomain { .. }
            | ClassExpression::AnnotationPropertyRange { .. } => {
                // These are typically axioms, not class expressions
                // But we handle them gracefully by doing nothing
                Ok(())
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
    fn visit_sub_object_property_axiom(&mut self, _axiom: &SubObjectPropertyOfAxiom) -> Result<()> {
        Ok(())
    }
    fn visit_equivalent_object_properties_axiom(
        &mut self,
        _axiom: &EquivalentObjectPropertiesAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_disjoint_object_properties_axiom(
        &mut self,
        _axiom: &DisjointObjectPropertiesAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_inverse_object_properties_axiom(
        &mut self,
        _axiom: &InverseObjectPropertiesAxiom,
    ) -> Result<()> {
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
        _axiom: &FunctionalObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_inverse_functional_object_property_axiom(
        &mut self,
        _axiom: &InverseFunctionalObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_reflexive_object_property_axiom(
        &mut self,
        _axiom: &ReflexiveObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_irreflexive_object_property_axiom(
        &mut self,
        _axiom: &IrreflexiveObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_symmetric_object_property_axiom(
        &mut self,
        _axiom: &SymmetricObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_asymmetric_object_property_axiom(
        &mut self,
        _axiom: &AsymmetricObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_transitive_object_property_axiom(
        &mut self,
        _axiom: &TransitiveObjectPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }

    // Data property axioms
    fn visit_sub_data_property_axiom(&mut self, _axiom: &SubDataPropertyOfAxiom) -> Result<()> {
        Ok(())
    }
    fn visit_equivalent_data_properties_axiom(
        &mut self,
        _axiom: &EquivalentDataPropertiesAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_disjoint_data_properties_axiom(
        &mut self,
        _axiom: &DisjointDataPropertiesAxiom,
    ) -> Result<()> {
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
        _axiom: &FunctionalDataPropertyAxiom,
    ) -> Result<()> {
        Ok(())
    }

    // Individual axioms
    fn visit_same_individual_axiom(&mut self, _axiom: &SameIndividualAxiom) -> Result<()> {
        Ok(())
    }
    fn visit_different_individuals_axiom(
        &mut self,
        _axiom: &DifferentIndividualsAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_class_assertion_axiom(&mut self, axiom: &ClassAssertionAxiom) -> Result<()> {
        self.visit_class_expression(&axiom.class)?;
        self.visit_individual(&axiom.individual)
    }
    fn visit_object_property_assertion_axiom(
        &mut self,
        _axiom: &ObjectPropertyAssertionAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_data_property_assertion_axiom(
        &mut self,
        _axiom: &DataPropertyAssertionAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_negative_object_property_assertion_axiom(
        &mut self,
        _axiom: &NegativeObjectPropertyAssertionAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_negative_data_property_assertion_axiom(
        &mut self,
        _axiom: &NegativeDataPropertyAssertionAxiom,
    ) -> Result<()> {
        Ok(())
    }

    // Annotation axioms
    fn visit_annotation_assertion_axiom(
        &mut self,
        _axiom: &AnnotationAssertionAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_sub_annotation_property_axiom(
        &mut self,
        _axiom: &SubAnnotationPropertyOfAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_annotation_property_domain_axiom(
        &mut self,
        _axiom: &AnnotationPropertyDomainAxiom,
    ) -> Result<()> {
        Ok(())
    }
    fn visit_annotation_property_range_axiom(
        &mut self,
        _axiom: &AnnotationPropertyRangeAxiom,
    ) -> Result<()> {
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
