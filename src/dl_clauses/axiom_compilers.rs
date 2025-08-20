//! Axiom compilation methods for converting OWL axioms to DL clauses

use crate::{
    error::Result,
    ontology::{Axiom, ClassExpression, ObjectPropertyExpression, DataPropertyExpression},
};
use log::debug;

use super::{
    types::{DLClause, DLAtom},
    helpers::HelperMethods,
    union_disjunctive::UnionDisjunctiveCompiler,
};

/// Axiom compilation trait that extends DLClauseGenerator
pub trait AxiomCompiler: HelperMethods + UnionDisjunctiveCompiler {
    /// Compile a single axiom to DL clauses
    fn compile_axiom(&mut self, axiom: &Axiom) -> Result<Vec<DLClause>> {
        match axiom {
            Axiom::SubClassOf(axiom) => self.compile_subclass_axiom(axiom),
            Axiom::EquivalentClasses(axiom) => self.compile_equivalent_classes_axiom(axiom),
            Axiom::DisjointClasses(axiom) => self.compile_disjoint_classes_axiom(axiom),
            Axiom::DisjointUnion(axiom) => self.compile_disjoint_union_axiom(axiom),
            Axiom::ClassAssertion(axiom) => self.compile_class_assertion_axiom(axiom),
            Axiom::ObjectPropertyAssertion(axiom) => self.compile_object_property_assertion_axiom(axiom),
            Axiom::DataPropertyAssertion(axiom) => self.compile_data_property_assertion_axiom(axiom),
            Axiom::SubObjectPropertyOf(axiom) => self.compile_sub_object_property_axiom(axiom),
            Axiom::SubDataPropertyOf(axiom) => self.compile_sub_data_property_axiom(axiom),
            Axiom::ObjectPropertyDomain(axiom) => self.compile_object_property_domain_axiom(axiom),
            Axiom::ObjectPropertyRange(axiom) => self.compile_object_property_range_axiom(axiom),
            Axiom::DataPropertyDomain(axiom) => self.compile_data_property_domain_axiom(axiom),
            Axiom::DataPropertyRange(axiom) => self.compile_data_property_range_axiom(axiom),
            Axiom::FunctionalObjectProperty(axiom) => self.compile_functional_object_property_axiom(axiom),
            Axiom::FunctionalDataProperty(axiom) => self.compile_functional_data_property_axiom(axiom),
            Axiom::InverseFunctionalObjectProperty(axiom) => self.compile_inverse_functional_object_property_axiom(axiom),
            Axiom::ReflexiveObjectProperty(axiom) => {
                debug!("Found ReflexiveObjectProperty axiom");
                self.compile_reflexive_object_property_axiom(axiom)
            },
            Axiom::IrreflexiveObjectProperty(axiom) => {
                debug!("Found IrreflexiveObjectProperty axiom");
                self.compile_irreflexive_object_property_axiom(axiom)
            },
            Axiom::SymmetricObjectProperty(axiom) => {
                debug!("Found SymmetricObjectProperty axiom");
                self.compile_symmetric_object_property_axiom(axiom)
            },
            Axiom::AsymmetricObjectProperty(axiom) => {
                debug!("Found AsymmetricObjectProperty axiom");
                self.compile_asymmetric_object_property_axiom(axiom)
            },
            Axiom::TransitiveObjectProperty(axiom) => {
                debug!("Found TransitiveObjectProperty axiom");
                self.compile_transitive_object_property_axiom(axiom)
            },
            Axiom::InverseObjectProperties(axiom) => {
                debug!("Found InverseObjectProperties axiom");
                self.compile_inverse_object_properties_axiom(axiom)
            },
            Axiom::EquivalentObjectProperties(axiom) => self.compile_equivalent_object_properties_axiom(axiom),
            Axiom::EquivalentDataProperties(axiom) => self.compile_equivalent_data_properties_axiom(axiom),
            Axiom::DisjointObjectProperties(axiom) => self.compile_disjoint_object_properties_axiom(axiom),
            Axiom::DisjointDataProperties(axiom) => self.compile_disjoint_data_properties_axiom(axiom),
            Axiom::SameIndividual(axiom) => {
                debug!("Found SameIndividual axiom");
                self.compile_same_individual_axiom(axiom)
            },
            Axiom::DifferentIndividuals(axiom) => {
                debug!("Found DifferentIndividuals axiom");
                self.compile_different_individuals_axiom(axiom)
            },
            Axiom::NegativeObjectPropertyAssertion(axiom) => {
                debug!("Found NegativeObjectPropertyAssertion axiom");
                self.compile_negative_object_property_assertion_axiom(axiom)
            },
            Axiom::NegativeDataPropertyAssertion(axiom) => {
                debug!("Found NegativeDataPropertyAssertion axiom");
                self.compile_negative_data_property_assertion_axiom(axiom)
            },
            Axiom::HasKey(axiom) => {
                debug!("Found HasKey axiom");
                self.compile_has_key_axiom(axiom)
            },
            Axiom::Rule(axiom) => {
                debug!("Found SWRL Rule axiom");
                self.compile_swrl_rule_axiom(axiom)
            },
            _ => {
                // For remaining unsupported axiom types, return empty clause set for now
                debug!("Unsupported axiom type: {:?}", std::mem::discriminant(axiom));
                Ok(Vec::new())
            }
        }
    }

    /// Compile SubClassOf axiom
    fn compile_subclass_axiom(&mut self, axiom: &crate::ontology::SubClassOfAxiom) -> Result<Vec<DLClause>>;

    /// Compile EquivalentClasses axiom
    fn compile_equivalent_classes_axiom(&mut self, axiom: &crate::ontology::EquivalentClassesAxiom) -> Result<Vec<DLClause>>;

    /// Compile DisjointClasses axiom
    fn compile_disjoint_classes_axiom(&mut self, axiom: &crate::ontology::DisjointClassesAxiom) -> Result<Vec<DLClause>>;

    /// Compile DisjointUnion axiom
    fn compile_disjoint_union_axiom(&mut self, axiom: &crate::ontology::DisjointUnionAxiom) -> Result<Vec<DLClause>>;

    /// Compile ClassAssertion axiom
    fn compile_class_assertion_axiom(&mut self, axiom: &crate::ontology::ClassAssertionAxiom) -> Result<Vec<DLClause>>;

    /// Compile ObjectPropertyAssertion axiom
    fn compile_object_property_assertion_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>>;

    /// Compile DataPropertyAssertion axiom
    fn compile_data_property_assertion_axiom(&mut self, axiom: &crate::ontology::DataPropertyAssertionAxiom) -> Result<Vec<DLClause>>;

    /// Compile SubObjectPropertyOf axiom
    fn compile_sub_object_property_axiom(&mut self, axiom: &crate::ontology::SubObjectPropertyOfAxiom) -> Result<Vec<DLClause>>;

    /// Compile SubDataPropertyOf axiom
    fn compile_sub_data_property_axiom(&mut self, axiom: &crate::ontology::SubDataPropertyOfAxiom) -> Result<Vec<DLClause>>;

    /// Compile ObjectPropertyDomain axiom
    fn compile_object_property_domain_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyDomainAxiom) -> Result<Vec<DLClause>>;

    /// Compile ObjectPropertyRange axiom
    fn compile_object_property_range_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyRangeAxiom) -> Result<Vec<DLClause>>;

    /// Compile DataPropertyDomain axiom
    fn compile_data_property_domain_axiom(&mut self, axiom: &crate::ontology::DataPropertyDomainAxiom) -> Result<Vec<DLClause>>;

    /// Compile DataPropertyRange axiom
    fn compile_data_property_range_axiom(&mut self, axiom: &crate::ontology::DataPropertyRangeAxiom) -> Result<Vec<DLClause>>;

    /// Compile FunctionalObjectProperty axiom
    fn compile_functional_object_property_axiom(&mut self, axiom: &crate::ontology::FunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile FunctionalDataProperty axiom
    fn compile_functional_data_property_axiom(&mut self, axiom: &crate::ontology::FunctionalDataPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile InverseFunctionalObjectProperty axiom
    fn compile_inverse_functional_object_property_axiom(&mut self, axiom: &crate::ontology::InverseFunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile ReflexiveObjectProperty axiom
    fn compile_reflexive_object_property_axiom(&mut self, axiom: &crate::ontology::ReflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile IrreflexiveObjectProperty axiom
    fn compile_irreflexive_object_property_axiom(&mut self, axiom: &crate::ontology::IrreflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile SymmetricObjectProperty axiom
    fn compile_symmetric_object_property_axiom(&mut self, axiom: &crate::ontology::SymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile AsymmetricObjectProperty axiom
    fn compile_asymmetric_object_property_axiom(&mut self, axiom: &crate::ontology::AsymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile TransitiveObjectProperty axiom
    fn compile_transitive_object_property_axiom(&mut self, axiom: &crate::ontology::TransitiveObjectPropertyAxiom) -> Result<Vec<DLClause>>;

    /// Compile InverseObjectProperties axiom
    fn compile_inverse_object_properties_axiom(&mut self, axiom: &crate::ontology::InverseObjectPropertiesAxiom) -> Result<Vec<DLClause>>;

    /// Compile EquivalentObjectProperties axiom
    fn compile_equivalent_object_properties_axiom(&mut self, axiom: &crate::ontology::EquivalentObjectPropertiesAxiom) -> Result<Vec<DLClause>>;

    /// Compile EquivalentDataProperties axiom
    fn compile_equivalent_data_properties_axiom(&mut self, axiom: &crate::ontology::EquivalentDataPropertiesAxiom) -> Result<Vec<DLClause>>;

    /// Compile DisjointObjectProperties axiom
    fn compile_disjoint_object_properties_axiom(&mut self, axiom: &crate::ontology::DisjointObjectPropertiesAxiom) -> Result<Vec<DLClause>>;

    /// Compile DisjointDataProperties axiom
    fn compile_disjoint_data_properties_axiom(&mut self, axiom: &crate::ontology::DisjointDataPropertiesAxiom) -> Result<Vec<DLClause>>;

    /// Compile SameIndividual axiom
    fn compile_same_individual_axiom(&mut self, axiom: &crate::ontology::SameIndividualAxiom) -> Result<Vec<DLClause>>;

    /// Compile DifferentIndividuals axiom
    fn compile_different_individuals_axiom(&mut self, axiom: &crate::ontology::DifferentIndividualsAxiom) -> Result<Vec<DLClause>>;

    /// Compile NegativeObjectPropertyAssertion axiom
    fn compile_negative_object_property_assertion_axiom(&mut self, axiom: &crate::ontology::NegativeObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>>;

    /// Compile NegativeDataPropertyAssertion axiom
    fn compile_negative_data_property_assertion_axiom(&mut self, axiom: &crate::ontology::NegativeDataPropertyAssertionAxiom) -> Result<Vec<DLClause>>;

    /// Compile HasKey axiom
    fn compile_has_key_axiom(&mut self, axiom: &crate::ontology::HasKeyAxiom) -> Result<Vec<DLClause>>;

    /// Compile SWRL Rule axiom
    fn compile_swrl_rule_axiom(&mut self, axiom: &crate::ontology::SWRLRuleAxiom) -> Result<Vec<DLClause>>;
}

// Implement the axiom compiler trait for DLClauseGenerator
impl AxiomCompiler for super::generator::DLClauseGenerator {
    /// Compile SubClassOf axiom
    fn compile_subclass_axiom(&mut self, axiom: &crate::ontology::SubClassOfAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let mut all_clauses = Vec::new();
        
        // Check if we should introduce definitions for complex expressions
        let (subclass_atom, mut subclass_def_clauses) = match &axiom.subclass {
            // Introduce definitions for complex subclass expressions
            ClassExpression::ObjectIntersectionOf(ops) if ops.len() > 2 => {
                self.introduce_definition(&axiom.subclass, &var_x)?
            }
            ClassExpression::ObjectUnionOf(ops) if ops.len() > 2 => {
                // Generate comprehensive disjunctive clauses for ObjectUnionOf
                let union_clauses = self.compile_union_disjunctive_clauses(ops, &var_x)?;
                all_clauses.extend(union_clauses);
                self.introduce_definition(&axiom.subclass, &var_x)?
            }
            ClassExpression::ObjectSomeValuesFrom { .. } => {
                self.introduce_definition(&axiom.subclass, &var_x)?
            }
            _ => {
                let atom = self.compile_class_expression_to_atom(&axiom.subclass, &var_x, true)?;
                (atom, vec![])
            }
        };
        
        let (superclass_atom, mut superclass_def_clauses) = match &axiom.superclass {
            // Introduce definitions for complex superclass expressions
            ClassExpression::ObjectIntersectionOf(ops) if ops.len() > 2 => {
                self.introduce_definition(&axiom.superclass, &var_x)?
            }
            ClassExpression::ObjectUnionOf(ops) if ops.len() > 2 => {
                // Generate comprehensive disjunctive clauses for ObjectUnionOf
                let union_clauses = self.compile_union_disjunctive_clauses(ops, &var_x)?;
                all_clauses.extend(union_clauses);
                self.introduce_definition(&axiom.superclass, &var_x)?
            }
            ClassExpression::ObjectSomeValuesFrom { .. } => {
                self.introduce_definition(&axiom.superclass, &var_x)?
            }
            _ => {
                let atom = self.compile_class_expression_to_atom(&axiom.superclass, &var_x, false)?;
                (atom, vec![])
            }
        };
        
        // Add definition clauses first
        all_clauses.append(&mut subclass_def_clauses);
        all_clauses.append(&mut superclass_def_clauses);
        
        // Add main subsumption clause: Subclass(X) → Superclass(X)
        let main_clause = DLClause::new(
            vec![superclass_atom],
            vec![subclass_atom],
            self.next_clause_id(),
        );
        all_clauses.push(main_clause);
        
        Ok(all_clauses)
    }

    /// Compile EquivalentClasses axiom
    fn compile_equivalent_classes_axiom(&mut self, axiom: &crate::ontology::EquivalentClassesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate bidirectional implications for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();
                
                // A(x) → B(x)
                let a_atom = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, false)?;
                
                clauses.push(DLClause::new(
                    vec![b_atom],
                    vec![a_atom],
                    self.next_clause_id(),
                ));
                
                // B(x) → A(x)
                let a_atom_2 = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, false)?;
                let b_atom_2 = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                
                clauses.push(DLClause::new(
                    vec![a_atom_2],
                    vec![b_atom_2],
                    self.next_clause_id(),
                ));
            }
        }
        
        Ok(clauses)
    }

    /// Compile DisjointClasses axiom with comprehensive disjunctive patterns
    fn compile_disjoint_classes_axiom(&mut self, axiom: &crate::ontology::DisjointClassesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate disjointness constraints for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();
                
                let a_atom = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                
                // Constraint: ¬(A(x) ∧ B(x))
                let clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![a_atom, b_atom],
                    self.next_clause_id(),
                );
                
                clauses.push(clause);
            }
        }

        // Generate comprehensive coverage disjunctions for complex expressions
        for class_expr in &axiom.classes {
            // Generate comprehensive union disjunctive clauses for ObjectUnionOf expressions
            if let ClassExpression::ObjectUnionOf(union_classes) = class_expr {
                let var_x = self.fresh_variable();
                clauses.extend(self.compile_union_disjunctive_clauses(union_classes, &var_x)?);
            }
        }
        
        Ok(clauses)
    }

    // Placeholder implementations for remaining methods
    fn compile_disjoint_union_axiom(&mut self, _axiom: &crate::ontology::DisjointUnionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_class_assertion_axiom(&mut self, _axiom: &crate::ontology::ClassAssertionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_object_property_assertion_axiom(&mut self, _axiom: &crate::ontology::ObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_data_property_assertion_axiom(&mut self, _axiom: &crate::ontology::DataPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_sub_object_property_axiom(&mut self, _axiom: &crate::ontology::SubObjectPropertyOfAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_sub_data_property_axiom(&mut self, _axiom: &crate::ontology::SubDataPropertyOfAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_object_property_domain_axiom(&mut self, _axiom: &crate::ontology::ObjectPropertyDomainAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_object_property_range_axiom(&mut self, _axiom: &crate::ontology::ObjectPropertyRangeAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_data_property_domain_axiom(&mut self, _axiom: &crate::ontology::DataPropertyDomainAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_data_property_range_axiom(&mut self, _axiom: &crate::ontology::DataPropertyRangeAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_functional_object_property_axiom(&mut self, _axiom: &crate::ontology::FunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_functional_data_property_axiom(&mut self, _axiom: &crate::ontology::FunctionalDataPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_inverse_functional_object_property_axiom(&mut self, _axiom: &crate::ontology::InverseFunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_reflexive_object_property_axiom(&mut self, _axiom: &crate::ontology::ReflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_irreflexive_object_property_axiom(&mut self, _axiom: &crate::ontology::IrreflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_symmetric_object_property_axiom(&mut self, _axiom: &crate::ontology::SymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_asymmetric_object_property_axiom(&mut self, _axiom: &crate::ontology::AsymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_transitive_object_property_axiom(&mut self, _axiom: &crate::ontology::TransitiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_inverse_object_properties_axiom(&mut self, _axiom: &crate::ontology::InverseObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_equivalent_object_properties_axiom(&mut self, _axiom: &crate::ontology::EquivalentObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_equivalent_data_properties_axiom(&mut self, _axiom: &crate::ontology::EquivalentDataPropertiesAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_disjoint_object_properties_axiom(&mut self, _axiom: &crate::ontology::DisjointObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_disjoint_data_properties_axiom(&mut self, _axiom: &crate::ontology::DisjointDataPropertiesAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_same_individual_axiom(&mut self, _axiom: &crate::ontology::SameIndividualAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_different_individuals_axiom(&mut self, _axiom: &crate::ontology::DifferentIndividualsAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_negative_object_property_assertion_axiom(&mut self, _axiom: &crate::ontology::NegativeObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_negative_data_property_assertion_axiom(&mut self, _axiom: &crate::ontology::NegativeDataPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_has_key_axiom(&mut self, _axiom: &crate::ontology::HasKeyAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }

    fn compile_swrl_rule_axiom(&mut self, _axiom: &crate::ontology::SWRLRuleAxiom) -> Result<Vec<DLClause>> {
        Ok(Vec::new())
    }
}
