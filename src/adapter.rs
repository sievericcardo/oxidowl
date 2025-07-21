//! Horned-OWL Adapter Module
//!
//! This module provides conversion functions between horned-owl types and oxidowl types,
//! allowing us to use horned-owl's robust parsing and ontology model while maintaining
//! oxidowl's specialized reasoning algorithms.

use crate::{
    Result, Error,
    ontology::{
        IRI, ClassExpression, ObjectPropertyExpression, DataPropertyExpression, 
        AnnotationPropertyExpression, Individual, Class, ObjectProperty, DataProperty,
        AnnotationProperty, Literal, Annotation, DataRange,
        axioms::{Axiom, AxiomId, Entity, DeclarationAxiom, SubClassOfAxiom, EquivalentClassesAxiom, 
                DisjointClassesAxiom, DisjointUnionAxiom}
    }
};

use horned_owl::model::*;
use std::collections::HashMap;

/// Adapter for converting between horned-owl and oxidowl ontology models
pub struct HornedOwlAdapter {
    /// Counter for generating unique axiom IDs
    next_axiom_id: u64,
    /// Cache for IRI conversions
    iri_cache: HashMap<horned_owl::model::IRI<String>, crate::ontology::IRI>,
}

impl HornedOwlAdapter {
    /// Create a new adapter
    pub fn new() -> Self {
        Self {
            next_axiom_id: 1,
            iri_cache: HashMap::new(),
        }
    }

    /// Generate next axiom ID
    fn next_axiom_id(&mut self) -> AxiomId {
        let id = self.next_axiom_id;
        self.next_axiom_id += 1;
        id
    }

    /// Convert horned-owl IRI to oxidowl IRI
    pub fn convert_iri(&mut self, horned_iri: &horned_owl::model::IRI<String>) -> Result<crate::ontology::IRI> {
        if let Some(cached) = self.iri_cache.get(horned_iri) {
            return Ok(cached.clone());
        }

        let oxidowl_iri = crate::ontology::IRI::new(&format!("{}", horned_iri));
        self.iri_cache.insert(horned_iri.clone(), oxidowl_iri.clone());
        Ok(oxidowl_iri)
    }

    /// Convert horned-owl Class to oxidowl Class
    pub fn convert_class(&mut self, horned_class: &horned_owl::model::Class<String>) -> Result<Class> {
        let iri = self.convert_iri(&horned_class.0)?;
        Ok(Class::new(iri))
    }

    /// Convert horned-owl ObjectProperty to oxidowl ObjectProperty
    pub fn convert_object_property(&mut self, horned_prop: &horned_owl::model::ObjectProperty<String>) -> Result<ObjectProperty> {
        let iri = self.convert_iri(&horned_prop.0)?;
        let url = iri.to_url().map_err(|e| Error::reasoning(&format!("Failed to convert IRI to URL: {}", e)))?;
        Ok(ObjectProperty { iri: url })
    }

    /// Convert horned-owl DataProperty to oxidowl DataProperty  
    pub fn convert_data_property(&mut self, horned_prop: &horned_owl::model::DataProperty<String>) -> Result<DataProperty> {
        let iri = self.convert_iri(&horned_prop.0)?;
        Ok(DataProperty { iri })
    }

    /// Convert horned-owl AnnotationProperty to oxidowl AnnotationProperty
    pub fn convert_annotation_property(&mut self, horned_prop: &horned_owl::model::AnnotationProperty<String>) -> Result<AnnotationProperty> {
        let iri = self.convert_iri(&horned_prop.0)?;
        Ok(AnnotationProperty { iri })
    }

    /// Convert horned-owl NamedIndividual to oxidowl Individual
    pub fn convert_individual(&mut self, horned_ind: &horned_owl::model::NamedIndividual<String>) -> Result<Individual> {
        let iri = self.convert_iri(&horned_ind.0)?;
        Ok(Individual::named(iri))
    }

    /// Convert horned-owl ClassExpression to oxidowl ClassExpression
    pub fn convert_class_expression(&mut self, horned_expr: &horned_owl::model::ClassExpression<String>) -> Result<ClassExpression> {
        match horned_expr {
            horned_owl::model::ClassExpression::Class(class) => {
                let oxidowl_class = self.convert_class(class)?;
                Ok(ClassExpression::Class(oxidowl_class))
            }
            horned_owl::model::ClassExpression::ObjectIntersectionOf(expressions) => {
                let converted_exprs: Result<Vec<_>> = expressions.iter()
                    .map(|expr| self.convert_class_expression(expr))
                    .collect();
                Ok(ClassExpression::ObjectIntersectionOf(converted_exprs?))
            }
            horned_owl::model::ClassExpression::ObjectUnionOf(expressions) => {
                let converted_exprs: Result<Vec<_>> = expressions.iter()
                    .map(|expr| self.convert_class_expression(expr))
                    .collect();
                Ok(ClassExpression::ObjectUnionOf(converted_exprs?))
            }
            horned_owl::model::ClassExpression::ObjectComplementOf(expression) => {
                let converted_expr = self.convert_class_expression(expression)?;
                Ok(ClassExpression::ObjectComplementOf(Box::new(converted_expr)))
            }
            horned_owl::model::ClassExpression::ObjectOneOf(individuals) => {
                let converted_individuals: Result<Vec<_>> = individuals.iter()
                    .map(|ind| {
                        match ind {
                            horned_owl::model::Individual::Named(named) => self.convert_individual(named),
                            horned_owl::model::Individual::Anonymous(_) => {
                                // For now, skip anonymous individuals
                                Err(Error::reasoning("Anonymous individuals not yet supported"))
                            }
                        }
                    })
                    .collect();
                Ok(ClassExpression::ObjectOneOf(converted_individuals?))
            }
            horned_owl::model::ClassExpression::ObjectSomeValuesFrom { ope, bce } => {
                let property = self.convert_object_property_expression(ope)?;
                let filler = self.convert_class_expression(bce)?;
                Ok(ClassExpression::ObjectSomeValuesFrom {
                    property,
                    filler: Box::new(filler),
                })
            }
            horned_owl::model::ClassExpression::ObjectAllValuesFrom { ope, bce } => {
                let property = self.convert_object_property_expression(ope)?;
                let filler = self.convert_class_expression(bce)?;
                Ok(ClassExpression::ObjectAllValuesFrom {
                    property,
                    filler: Box::new(filler),
                })
            }
            horned_owl::model::ClassExpression::ObjectHasValue { ope, i } => {
                let property = self.convert_object_property_expression(ope)?;
                let individual = match i {
                    horned_owl::model::Individual::Named(named) => self.convert_individual(named)?,
                    horned_owl::model::Individual::Anonymous(_) => {
                        return Err(Error::reasoning("Anonymous individuals not yet supported"));
                    }
                };
                Ok(ClassExpression::ObjectHasValue {
                    property,
                    value: individual,
                })
            }
            horned_owl::model::ClassExpression::ObjectHasSelf(ope) => {
                let property = self.convert_object_property_expression(ope)?;
                Ok(ClassExpression::ObjectHasSelf { property })
            }
            horned_owl::model::ClassExpression::ObjectMinCardinality { n, ope, bce } => {
                let property = self.convert_object_property_expression(ope)?;
                let filler = self.convert_class_expression(bce)?;
                Ok(ClassExpression::ObjectMinCardinality {
                    property,
                    cardinality: *n,
                    filler: Box::new(filler),
                })
            }
            horned_owl::model::ClassExpression::ObjectMaxCardinality { n, ope, bce } => {
                let property = self.convert_object_property_expression(ope)?;
                let filler = self.convert_class_expression(bce)?;
                Ok(ClassExpression::ObjectMaxCardinality {
                    property,
                    cardinality: *n,
                    filler: Box::new(filler),
                })
            }
            horned_owl::model::ClassExpression::ObjectExactCardinality { n, ope, bce } => {
                let property = self.convert_object_property_expression(ope)?;
                let filler = self.convert_class_expression(bce)?;
                Ok(ClassExpression::ObjectExactCardinality {
                    property,
                    cardinality: *n,
                    filler: Box::new(filler),
                })
            }
            horned_owl::model::ClassExpression::DataSomeValuesFrom { dp, dr } => {
                let property = self.convert_data_property_expression(dp)?;
                let range = self.convert_data_range(dr)?;
                Ok(ClassExpression::DataSomeValuesFrom {
                    property,
                    filler: range,
                })
            }
            horned_owl::model::ClassExpression::DataAllValuesFrom { dp, dr } => {
                let property = self.convert_data_property_expression(dp)?;
                let range = self.convert_data_range(dr)?;
                Ok(ClassExpression::DataAllValuesFrom {
                    property,
                    filler: range,
                })
            }
            horned_owl::model::ClassExpression::DataHasValue { dp, l } => {
                let property = self.convert_data_property_expression(dp)?;
                let literal = self.convert_literal(l)?;
                Ok(ClassExpression::DataHasValue {
                    property,
                    value: literal,
                })
            }
            horned_owl::model::ClassExpression::DataMinCardinality { n, dp, dr } => {
                let property = self.convert_data_property_expression(dp)?;
                let range = self.convert_data_range(dr)?;
                Ok(ClassExpression::DataMinCardinality {
                    property,
                    cardinality: *n,
                    filler: range,
                })
            }
            horned_owl::model::ClassExpression::DataMaxCardinality { n, dp, dr } => {
                let property = self.convert_data_property_expression(dp)?;
                let range = self.convert_data_range(dr)?;
                Ok(ClassExpression::DataMaxCardinality {
                    property,
                    cardinality: *n,
                    filler: range,
                })
            }
            horned_owl::model::ClassExpression::DataExactCardinality { n, dp, dr } => {
                let property = self.convert_data_property_expression(dp)?;
                let range = self.convert_data_range(dr)?;
                Ok(ClassExpression::DataExactCardinality {
                    property,
                    cardinality: *n,
                    filler: range,
                })
            }
        }
    }

    /// Convert horned-owl ObjectPropertyExpression to oxidowl ObjectPropertyExpression
    pub fn convert_object_property_expression(&mut self, horned_expr: &horned_owl::model::ObjectPropertyExpression<String>) -> Result<ObjectPropertyExpression> {
        match horned_expr {
            horned_owl::model::ObjectPropertyExpression::ObjectProperty(prop) => {
                let oxidowl_prop = self.convert_object_property(prop)?;
                Ok(ObjectPropertyExpression::ObjectProperty(oxidowl_prop))
            }
            horned_owl::model::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                let oxidowl_prop = self.convert_object_property(prop)?;
                Ok(ObjectPropertyExpression::InverseObjectProperty(oxidowl_prop))
            }
        }
    }

    /// Convert horned-owl DataPropertyExpression to oxidowl DataPropertyExpression  
    pub fn convert_data_property_expression(&mut self, horned_expr: &horned_owl::model::DataProperty<String>) -> Result<DataPropertyExpression> {
        let oxidowl_prop = self.convert_data_property(horned_expr)?;
        Ok(DataPropertyExpression::DataProperty(oxidowl_prop))
    }

    /// Convert horned-owl AnnotationPropertyExpression to oxidowl AnnotationPropertyExpression
    pub fn convert_annotation_property_expression(&mut self, horned_expr: &horned_owl::model::AnnotationProperty<String>) -> Result<AnnotationPropertyExpression> {
        let oxidowl_prop = self.convert_annotation_property(horned_expr)?;
        Ok(AnnotationPropertyExpression::AnnotationProperty(oxidowl_prop))
    }

    /// Convert horned-owl Literal to oxidowl Literal
    pub fn convert_literal(&mut self, horned_literal: &horned_owl::model::Literal<String>) -> Result<Literal> {
        match horned_literal {
            horned_owl::model::Literal::Simple { literal } => {
                Ok(Literal::new(literal.clone()))
            }
            horned_owl::model::Literal::Language { literal, lang } => {
                Ok(Literal::with_language(literal.clone(), lang.clone()))
            }
            horned_owl::model::Literal::Datatype { literal, datatype_iri } => {
                let dt_iri = self.convert_iri(datatype_iri)?;
                Ok(Literal::with_datatype(literal.clone(), dt_iri))
            }
        }
    }

    /// Convert horned-owl DataRange to oxidowl DataRange
    pub fn convert_data_range(&mut self, horned_range: &horned_owl::model::DataRange<String>) -> Result<DataRange> {
        match horned_range {
            horned_owl::model::DataRange::Datatype(dt) => {
                let iri = self.convert_iri(&dt.0)?;
                Ok(DataRange::Datatype(iri))
            }
            horned_owl::model::DataRange::DataIntersectionOf(ranges) => {
                let converted_ranges: Result<Vec<_>> = ranges.iter()
                    .map(|range| self.convert_data_range(range))
                    .collect();
                Ok(DataRange::DataIntersectionOf(converted_ranges?))
            }
            horned_owl::model::DataRange::DataUnionOf(ranges) => {
                let converted_ranges: Result<Vec<_>> = ranges.iter()
                    .map(|range| self.convert_data_range(range))
                    .collect();
                Ok(DataRange::DataUnionOf(converted_ranges?))
            }
            horned_owl::model::DataRange::DataComplementOf(range) => {
                let converted_range = self.convert_data_range(range)?;
                Ok(DataRange::DataComplementOf(Box::new(converted_range)))
            }
            horned_owl::model::DataRange::DataOneOf(literals) => {
                let converted_literals: Result<Vec<_>> = literals.iter()
                    .map(|lit| self.convert_literal(lit))
                    .collect();
                Ok(DataRange::DataOneOf(converted_literals?))
            }
            horned_owl::model::DataRange::DatatypeRestriction(dt, restrictions) => {
                let iri = self.convert_iri(&dt.0)?;
                // For now, we'll just use the base datatype and ignore facet restrictions
                // This could be enhanced to support full datatype restrictions
                Ok(DataRange::Datatype(iri))
            }
        }
    }

    /// Convert horned-owl Annotation to oxidowl Annotation
    pub fn convert_annotation(&mut self, horned_ann: &horned_owl::model::Annotation<String>) -> Result<Annotation> {
        let property = self.convert_annotation_property(&horned_ann.ap)?;
        let value = match &horned_ann.av {
            horned_owl::model::AnnotationValue::Literal(lit) => {
                crate::ontology::AnnotationValue::Literal(self.convert_literal(lit)?)
            }
            horned_owl::model::AnnotationValue::IRI(iri) => {
                crate::ontology::AnnotationValue::IRI(self.convert_iri(iri)?)
            }
            horned_owl::model::AnnotationValue::AnonymousIndividual(_) => {
                // For now, we'll skip anonymous individuals in annotations
                return Err(Error::reasoning("Anonymous individuals in annotations not yet supported"));
            }
        };
        Ok(Annotation { property, value })
    }

    /// Convert horned-owl axiom to oxidowl axiom (simplified implementation)
    pub fn convert_axiom(&mut self, _horned_axiom: &str) -> Result<Axiom> {
        // For now, we'll implement a minimal conversion
        // This would need to be expanded based on actual horned-owl axiom API
        Err(Error::reasoning("Axiom conversion not yet implemented"))
    }

    /// Convert a complete horned-owl ontology to oxidowl ontology (simplified)
    pub fn convert_ontology_simple(&mut self, ontology_text: &str) -> Result<crate::ontology::Ontology> {
        // For now, we'll use the existing parsers but with enhanced error handling
        // This is a placeholder that would be expanded with proper horned-owl integration
        crate::parsers::turtle::parse(ontology_text)
    }
}

impl Default for HornedOwlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iri_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        let horned_iri = horned_owl::model::IRI("http://example.org/test".to_string());
        let oxidowl_iri = adapter.convert_iri(&horned_iri).unwrap();
        assert_eq!(oxidowl_iri.as_str(), "http://example.org/test");
    }

    #[test]
    fn test_class_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        let horned_class = horned_owl::model::Class(horned_owl::model::IRI("http://example.org/Person".to_string()));
        let oxidowl_class = adapter.convert_class(&horned_class).unwrap();
        assert_eq!(oxidowl_class.iri.as_str(), "http://example.org/Person");
    }
}
