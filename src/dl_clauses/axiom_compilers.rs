//! Axiom compilation methods for converting OWL axioms to DL clauses

use crate::{
    error::Result,
    ontology::{Axiom, ClassExpression},
};
use log::debug;

use crate::dl_clauses::{
    helpers::HelperMethods,
    types::{DLAtom, DLClause},
    union_disjunctive::UnionDisjunctiveCompiler,
};

/// Axiom compilation trait that extends `DLClauseGenerator`
pub trait AxiomCompiler: HelperMethods + UnionDisjunctiveCompiler {
    /// Compile a single axiom to DL clauses
    fn compile_axiom(&mut self, axiom: &Axiom) -> Result<Vec<DLClause>> {
        match axiom {
            Axiom::Declaration(_) => {
                // Declaration axioms don't generate DL clauses - they just declare entities
                debug!("Skipping Declaration axiom - no clauses generated");
                Ok(Vec::new())
            }
            Axiom::SubClassOf(axiom) => self.compile_subclass_axiom(axiom),
            Axiom::EquivalentClasses(axiom) => self.compile_equivalent_classes_axiom(axiom),
            Axiom::DisjointClasses(axiom) => self.compile_disjoint_classes_axiom(axiom),
            Axiom::DisjointUnion(axiom) => self.compile_disjoint_union_axiom(axiom),
            Axiom::ClassAssertion(axiom) => self.compile_class_assertion_axiom(axiom),
            Axiom::ObjectPropertyAssertion(axiom) => {
                self.compile_object_property_assertion_axiom(axiom)
            }
            Axiom::DataPropertyAssertion(axiom) => {
                self.compile_data_property_assertion_axiom(axiom)
            }
            Axiom::SubObjectPropertyOf(axiom) => self.compile_sub_object_property_axiom(axiom),
            Axiom::SubDataPropertyOf(axiom) => self.compile_sub_data_property_axiom(axiom),
            Axiom::ObjectPropertyDomain(axiom) => self.compile_object_property_domain_axiom(axiom),
            Axiom::ObjectPropertyRange(axiom) => self.compile_object_property_range_axiom(axiom),
            Axiom::DataPropertyDomain(axiom) => self.compile_data_property_domain_axiom(axiom),
            Axiom::DataPropertyRange(axiom) => self.compile_data_property_range_axiom(axiom),
            Axiom::FunctionalObjectProperty(axiom) => {
                self.compile_functional_object_property_axiom(axiom)
            }
            Axiom::FunctionalDataProperty(axiom) => {
                self.compile_functional_data_property_axiom(axiom)
            }
            Axiom::InverseFunctionalObjectProperty(axiom) => {
                self.compile_inverse_functional_object_property_axiom(axiom)
            }
            Axiom::ReflexiveObjectProperty(axiom) => {
                debug!("Found ReflexiveObjectProperty axiom");
                self.compile_reflexive_object_property_axiom(axiom)
            }
            Axiom::IrreflexiveObjectProperty(axiom) => {
                debug!("Found IrreflexiveObjectProperty axiom");
                self.compile_irreflexive_object_property_axiom(axiom)
            }
            Axiom::SymmetricObjectProperty(axiom) => {
                debug!("Found SymmetricObjectProperty axiom");
                self.compile_symmetric_object_property_axiom(axiom)
            }
            Axiom::AsymmetricObjectProperty(axiom) => {
                debug!("Found AsymmetricObjectProperty axiom");
                self.compile_asymmetric_object_property_axiom(axiom)
            }
            Axiom::TransitiveObjectProperty(axiom) => {
                debug!("Found TransitiveObjectProperty axiom");
                self.compile_transitive_object_property_axiom(axiom)
            }
            Axiom::InverseObjectProperties(axiom) => {
                debug!("Found InverseObjectProperties axiom");
                self.compile_inverse_object_properties_axiom(axiom)
            }
            Axiom::EquivalentObjectProperties(axiom) => {
                self.compile_equivalent_object_properties_axiom(axiom)
            }
            Axiom::EquivalentDataProperties(axiom) => {
                self.compile_equivalent_data_properties_axiom(axiom)
            }
            Axiom::DisjointObjectProperties(axiom) => {
                self.compile_disjoint_object_properties_axiom(axiom)
            }
            Axiom::DisjointDataProperties(axiom) => {
                self.compile_disjoint_data_properties_axiom(axiom)
            }
            Axiom::SameIndividual(axiom) => {
                debug!("Found SameIndividual axiom");
                self.compile_same_individual_axiom(axiom)
            }
            Axiom::DifferentIndividuals(axiom) => {
                debug!("Found DifferentIndividuals axiom");
                self.compile_different_individuals_axiom(axiom)
            }
            Axiom::NegativeObjectPropertyAssertion(axiom) => {
                debug!("Found NegativeObjectPropertyAssertion axiom");
                self.compile_negative_object_property_assertion_axiom(axiom)
            }
            Axiom::NegativeDataPropertyAssertion(axiom) => {
                debug!("Found NegativeDataPropertyAssertion axiom");
                self.compile_negative_data_property_assertion_axiom(axiom)
            }
            Axiom::HasKey(axiom) => {
                debug!("Found HasKey axiom");
                self.compile_has_key_axiom(axiom)
            }
            Axiom::Rule(axiom) => {
                debug!("Found SWRL Rule axiom");
                self.compile_swrl_rule_axiom(axiom)
            }
            _ => {
                // For remaining unsupported axiom types, return empty clause set for now
                debug!(
                    "Unsupported axiom type: {:?}",
                    std::mem::discriminant(axiom)
                );
                Ok(Vec::new())
            }
        }
    }

    /// Compile `SubClassOf` axiom
    fn compile_subclass_axiom(
        &mut self,
        axiom: &crate::ontology::SubClassOfAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `EquivalentClasses` axiom
    fn compile_equivalent_classes_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentClassesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DisjointClasses` axiom
    fn compile_disjoint_classes_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointClassesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DisjointUnion` axiom
    fn compile_disjoint_union_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointUnionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `ClassAssertion` axiom
    fn compile_class_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::ClassAssertionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `ObjectPropertyAssertion` axiom
    fn compile_object_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DataPropertyAssertion` axiom
    fn compile_data_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `SubObjectPropertyOf` axiom
    fn compile_sub_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::SubObjectPropertyOfAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `SubDataPropertyOf` axiom
    fn compile_sub_data_property_axiom(
        &mut self,
        axiom: &crate::ontology::SubDataPropertyOfAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `ObjectPropertyDomain` axiom
    fn compile_object_property_domain_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyDomainAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `ObjectPropertyRange` axiom
    fn compile_object_property_range_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyRangeAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DataPropertyDomain` axiom
    fn compile_data_property_domain_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyDomainAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DataPropertyRange` axiom
    fn compile_data_property_range_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyRangeAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `FunctionalObjectProperty` axiom
    fn compile_functional_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::FunctionalObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `FunctionalDataProperty` axiom
    fn compile_functional_data_property_axiom(
        &mut self,
        axiom: &crate::ontology::FunctionalDataPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `InverseFunctionalObjectProperty` axiom
    fn compile_inverse_functional_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::InverseFunctionalObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `ReflexiveObjectProperty` axiom
    fn compile_reflexive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::ReflexiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `IrreflexiveObjectProperty` axiom
    fn compile_irreflexive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::IrreflexiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `SymmetricObjectProperty` axiom
    fn compile_symmetric_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::SymmetricObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `AsymmetricObjectProperty` axiom
    fn compile_asymmetric_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::AsymmetricObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `TransitiveObjectProperty` axiom
    fn compile_transitive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::TransitiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `InverseObjectProperties` axiom
    fn compile_inverse_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::InverseObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `EquivalentObjectProperties` axiom
    fn compile_equivalent_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `EquivalentDataProperties` axiom
    fn compile_equivalent_data_properties_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentDataPropertiesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DisjointObjectProperties` axiom
    fn compile_disjoint_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DisjointDataProperties` axiom
    fn compile_disjoint_data_properties_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointDataPropertiesAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `SameIndividual` axiom
    fn compile_same_individual_axiom(
        &mut self,
        axiom: &crate::ontology::SameIndividualAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `DifferentIndividuals` axiom
    fn compile_different_individuals_axiom(
        &mut self,
        axiom: &crate::ontology::DifferentIndividualsAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `NegativeObjectPropertyAssertion` axiom
    fn compile_negative_object_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::NegativeObjectPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `NegativeDataPropertyAssertion` axiom
    fn compile_negative_data_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::NegativeDataPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile `HasKey` axiom
    fn compile_has_key_axiom(
        &mut self,
        axiom: &crate::ontology::HasKeyAxiom,
    ) -> Result<Vec<DLClause>>;

    /// Compile SWRL Rule axiom
    fn compile_swrl_rule_axiom(
        &mut self,
        axiom: &crate::ontology::SWRLRuleAxiom,
    ) -> Result<Vec<DLClause>>;
}

// Implement the axiom compiler trait for DLClauseGenerator
impl AxiomCompiler for super::generator::DLClauseGenerator {
    /// Compile `SubClassOf` axiom
    fn compile_subclass_axiom(
        &mut self,
        axiom: &crate::ontology::SubClassOfAxiom,
    ) -> Result<Vec<DLClause>> {
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
                let atom =
                    self.compile_class_expression_to_atom(&axiom.superclass, &var_x, false)?;
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

    /// Compile `EquivalentClasses` axiom
    ///
    /// For each pair (Ci, Cj), generates bidirectional Horn clauses.
    /// Complex class expressions (e.g. `ObjectSomeValuesFrom`) are handled
    /// via auxiliary definitions so that role-level clauses are properly split
    /// into concept-level atoms that the clause checker can evaluate.
    fn compile_equivalent_classes_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentClassesAxiom,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        // Helper: return (atom, def_clauses) for an expression, introducing a
        // fresh definition name for complex expressions just as compile_subclass_axiom does.
        // Generate bidirectional implications for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();
                let mut pair_clauses: Vec<DLClause> = Vec::new();

                // --- Compile side i ---
                let (atom_i_body, mut def_i) = match &axiom.classes[i] {
                    ClassExpression::ObjectSomeValuesFrom { .. }
                    | ClassExpression::ObjectAllValuesFrom { .. }
                    | ClassExpression::ObjectHasValue { .. }
                    | ClassExpression::ObjectHasSelf { .. }
                    | ClassExpression::ObjectMinCardinality { .. }
                    | ClassExpression::ObjectMaxCardinality { .. }
                    | ClassExpression::ObjectExactCardinality { .. } => {
                        self.introduce_definition(&axiom.classes[i], &var_x)?
                    }
                    ClassExpression::ObjectIntersectionOf(ops) if ops.len() > 2 => {
                        self.introduce_definition(&axiom.classes[i], &var_x)?
                    }
                    ClassExpression::ObjectUnionOf(ops) if ops.len() > 2 => {
                        let union_clauses = self.compile_union_disjunctive_clauses(ops, &var_x)?;
                        pair_clauses.extend(union_clauses);
                        self.introduce_definition(&axiom.classes[i], &var_x)?
                    }
                    _ => {
                        let a =
                            self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                        (a, vec![])
                    }
                };

                // --- Compile side j ---
                let (atom_j_body, mut def_j) = match &axiom.classes[j] {
                    ClassExpression::ObjectSomeValuesFrom { .. }
                    | ClassExpression::ObjectAllValuesFrom { .. }
                    | ClassExpression::ObjectHasValue { .. }
                    | ClassExpression::ObjectHasSelf { .. }
                    | ClassExpression::ObjectMinCardinality { .. }
                    | ClassExpression::ObjectMaxCardinality { .. }
                    | ClassExpression::ObjectExactCardinality { .. } => {
                        self.introduce_definition(&axiom.classes[j], &var_x)?
                    }
                    ClassExpression::ObjectIntersectionOf(ops) if ops.len() > 2 => {
                        self.introduce_definition(&axiom.classes[j], &var_x)?
                    }
                    ClassExpression::ObjectUnionOf(ops) if ops.len() > 2 => {
                        let union_clauses = self.compile_union_disjunctive_clauses(ops, &var_x)?;
                        pair_clauses.extend(union_clauses);
                        self.introduce_definition(&axiom.classes[j], &var_x)?
                    }
                    _ => {
                        let a =
                            self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                        (a, vec![])
                    }
                };

                // Collect auxiliary definition clauses first
                pair_clauses.append(&mut def_i);
                pair_clauses.append(&mut def_j);

                // Produce head-form (positive) atoms for conclusions
                // Using same variable as the body atoms so the clause is well-formed.
                // atom_i_body / atom_j_body are already positive (is_positive=true);
                // we reuse them directly in both body and head positions.

                // Ci(x) → Cj(x):  body=[atom_i], head=[atom_j]
                pair_clauses.push(DLClause::new(
                    vec![atom_j_body.clone()],
                    vec![atom_i_body.clone()],
                    self.next_clause_id(),
                ));

                // Cj(x) → Ci(x):  body=[atom_j], head=[atom_i]
                pair_clauses.push(DLClause::new(
                    vec![atom_i_body.clone()],
                    vec![atom_j_body.clone()],
                    self.next_clause_id(),
                ));

                clauses.extend(pair_clauses);
            }
        }

        Ok(clauses)
    }

    /// Compile `DisjointClasses` axiom with comprehensive disjunctive patterns
    fn compile_disjoint_classes_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointClassesAxiom,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        // Generate disjointness constraints for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();

                let a_atom =
                    self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom =
                    self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;

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

    // Enhanced implementations for remaining DL clause compilation methods
    fn compile_disjoint_union_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointUnionAxiom,
    ) -> Result<Vec<DLClause>> {
        // DisjointUnion(C, C1, C2, ..., Cn) generates:
        // 1. C ≡ (C1 ⊔ C2 ⊔ ... ⊔ Cn) - coverage
        // 2. Disjointness constraints for all pairs
        let mut clauses = Vec::new();
        let var_x = self.fresh_variable();

        // Generate coverage: C(x) ↔ (C1(x) ⊔ C2(x) ⊔ ... ⊔ Cn(x))
        // Forward direction: C(x) → (C1(x) ⊔ C2(x) ⊔ ... ⊔ Cn(x))
        let class_atom = self.compile_class_expression_to_atom(&axiom.class, &var_x, true)?;
        let mut union_atoms = Vec::new();
        for union_class in &axiom.disjoint_classes {
            union_atoms.push(self.compile_class_expression_to_atom(union_class, &var_x, false)?);
        }

        // C(x) → C1(x) ⊔ C2(x) ⊔ ... ⊔ Cn(x)
        clauses.push(DLClause::new(
            union_atoms,
            vec![class_atom.clone()],
            self.next_clause_id(),
        ));

        // Backward direction: Ci(x) → C(x) for each i
        for union_class in &axiom.disjoint_classes {
            let union_atom = self.compile_class_expression_to_atom(union_class, &var_x, true)?;
            let class_atom_back =
                self.compile_class_expression_to_atom(&axiom.class, &var_x, false)?;

            clauses.push(DLClause::new(
                vec![class_atom_back],
                vec![union_atom],
                self.next_clause_id(),
            ));
        }

        // Generate disjointness constraints for all pairs: ¬(Ci(x) ∧ Cj(x))
        for i in 0..axiom.disjoint_classes.len() {
            for j in (i + 1)..axiom.disjoint_classes.len() {
                let var_x_disj = self.fresh_variable();
                let ci_atom = self.compile_class_expression_to_atom(
                    &axiom.disjoint_classes[i],
                    &var_x_disj,
                    true,
                )?;
                let cj_atom = self.compile_class_expression_to_atom(
                    &axiom.disjoint_classes[j],
                    &var_x_disj,
                    true,
                )?;

                // Constraint: ¬(Ci(x) ∧ Cj(x))
                clauses.push(DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![ci_atom, cj_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok(clauses)
    }

    fn compile_class_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::ClassAssertionAxiom,
    ) -> Result<Vec<DLClause>> {
        // ClassAssertion(C, a) generates fact: C(a)
        let individual_name = self.individual_to_string(&axiom.individual);
        let class_atom =
            self.compile_class_expression_to_atom(&axiom.class, &individual_name, false)?;

        let clause = DLClause::new(
            vec![class_atom],
            vec![], // Fact
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_object_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>> {
        // ObjectPropertyAssertion(P, a, b) generates fact: P(a, b)
        let subject_name = self.individual_to_string(&axiom.source);
        let object_name = self.individual_to_string(&axiom.target);
        let property_name = self.object_property_expression_to_string(&axiom.property);

        let property_atom = DLAtom::role_assertion(&property_name, &subject_name, &object_name);

        let clause = DLClause::new(
            vec![property_atom],
            vec![], // Fact
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_data_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>> {
        // DataPropertyAssertion(P, a, v) generates fact: P(a, v)
        let subject_name = self.individual_to_string(&axiom.individual);
        let value_string = self.literal_to_string(&axiom.value);
        let property_name = self.data_property_expression_to_string(&axiom.property);

        let property_atom =
            DLAtom::datatype_assertion(&property_name, &subject_name, &value_string);

        let clause = DLClause::new(
            vec![property_atom],
            vec![], // Fact
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_sub_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::SubObjectPropertyOfAxiom,
    ) -> Result<Vec<DLClause>> {
        // SubObjectPropertyOf(P, Q) generates: P(x,y) → Q(x,y)
        // Handle property chain case: SubObjectPropertyOf(PropertyChain(P1, P2, ..., Pn), Q)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        match &axiom.sub_property {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(_prop) => {
                // Simple case: P(x,y) → Q(x,y)
                let sub_prop_name = self.object_property_expression_to_string(&axiom.sub_property);
                let super_prop_name =
                    self.object_property_expression_to_string(&axiom.super_property);

                let sub_atom = DLAtom::role_assertion(&sub_prop_name, &var_x, &var_y);
                let super_atom = DLAtom::role_assertion(&super_prop_name, &var_x, &var_y);

                let clause = DLClause::new(vec![super_atom], vec![sub_atom], self.next_clause_id());

                Ok(vec![clause])
            }
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(inv_prop) => {
                // Inverse property case: P⁻(x,y) → Q(x,y), which is P(y,x) → Q(x,y)
                let sub_prop_name = self.object_property_expression_to_string(
                    &crate::ontology::ObjectPropertyExpression::ObjectProperty(inv_prop.clone()),
                );
                let super_prop_name =
                    self.object_property_expression_to_string(&axiom.super_property);

                let sub_atom = DLAtom::role_assertion(&sub_prop_name, &var_y, &var_x); // Note: reversed arguments
                let super_atom = DLAtom::role_assertion(&super_prop_name, &var_x, &var_y);

                let clause = DLClause::new(vec![super_atom], vec![sub_atom], self.next_clause_id());

                Ok(vec![clause])
            }
            // Handle property chains: R₁ ∘ R₂ ∘ ... ∘ Rₙ ⊑ S
            // Generates: R₁(x,y₁) ∧ R₂(y₁,y₂) ∧ ... ∧ Rₙ(yₙ₋₁,z) → S(x,z)
            crate::ontology::ObjectPropertyExpression::PropertyChain(chain) => {
                if chain.is_empty() {
                    return Ok(Vec::new());
                }

                // Create variables for the chain: x, y₁, y₂, ..., yₙ₋₁, z
                let var_x = self.fresh_variable();
                let var_z = self.fresh_variable();

                // Create intermediate variables for chain links
                let mut intermediate_vars = Vec::new();
                for _ in 0..(chain.len().saturating_sub(1)) {
                    intermediate_vars.push(self.fresh_variable());
                }

                // Build body atoms for each property in the chain
                let mut body_atoms = Vec::new();
                for (i, prop_expr) in chain.iter().enumerate() {
                    let prop_name = self.object_property_expression_to_string(prop_expr);

                    let subject = if i == 0 {
                        var_x.clone()
                    } else {
                        intermediate_vars[i - 1].clone()
                    };

                    let object = if i == chain.len() - 1 {
                        var_z.clone()
                    } else {
                        intermediate_vars[i].clone()
                    };

                    body_atoms.push(DLAtom::role_assertion(&prop_name, &subject, &object));
                }

                // Build head atom for super property
                let super_prop_name =
                    self.object_property_expression_to_string(&axiom.super_property);
                let head_atom = DLAtom::role_assertion(&super_prop_name, &var_x, &var_z);

                let clause = DLClause::new(vec![head_atom], body_atoms, self.next_clause_id());
                Ok(vec![clause])
            }
        }
    }

    fn compile_sub_data_property_axiom(
        &mut self,
        axiom: &crate::ontology::SubDataPropertyOfAxiom,
    ) -> Result<Vec<DLClause>> {
        // SubDataPropertyOf(P, Q) generates: P(x,v) → Q(x,v)
        let var_x = self.fresh_variable();
        let var_v = self.fresh_variable();

        let sub_prop_name = self.data_property_expression_to_string(&axiom.sub_property);
        let super_prop_name = self.data_property_expression_to_string(&axiom.super_property);

        let sub_atom = DLAtom::datatype_assertion(&sub_prop_name, &var_x, &var_v);
        let super_atom = DLAtom::datatype_assertion(&super_prop_name, &var_x, &var_v);

        let clause = DLClause::new(vec![super_atom], vec![sub_atom], self.next_clause_id());

        Ok(vec![clause])
    }

    fn compile_object_property_domain_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyDomainAxiom,
    ) -> Result<Vec<DLClause>> {
        // ObjectPropertyDomain(P, C) generates: P(x,y) → C(x)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let domain_atom = self.compile_class_expression_to_atom(&axiom.domain, &var_x, false)?;
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);

        let clause = DLClause::new(
            vec![domain_atom],
            vec![property_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_object_property_range_axiom(
        &mut self,
        axiom: &crate::ontology::ObjectPropertyRangeAxiom,
    ) -> Result<Vec<DLClause>> {
        // ObjectPropertyRange(P, C) generates: P(x,y) → C(y)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let range_atom = self.compile_class_expression_to_atom(&axiom.range, &var_y, false)?;
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);

        let clause = DLClause::new(vec![range_atom], vec![property_atom], self.next_clause_id());

        Ok(vec![clause])
    }

    fn compile_data_property_domain_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyDomainAxiom,
    ) -> Result<Vec<DLClause>> {
        // DataPropertyDomain(P, C) generates: P(x,v) → C(x)
        let var_x = self.fresh_variable();
        let var_v = self.fresh_variable();

        let property_name = self.data_property_expression_to_string(&axiom.property);
        let domain_atom = self.compile_class_expression_to_atom(&axiom.domain, &var_x, false)?;
        let property_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_v);

        let clause = DLClause::new(
            vec![domain_atom],
            vec![property_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_data_property_range_axiom(
        &mut self,
        axiom: &crate::ontology::DataPropertyRangeAxiom,
    ) -> Result<Vec<DLClause>> {
        // DataPropertyRange(P, R) generates: P(x,v) → R(v)
        let var_x = self.fresh_variable();
        let var_v = self.fresh_variable();

        let property_name = self.data_property_expression_to_string(&axiom.property);
        let range_constraint = self.compile_data_range_to_constraint(&axiom.range, &var_v)?;
        let property_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_v);

        let clause = DLClause::new(
            vec![range_constraint],
            vec![property_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_functional_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::FunctionalObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // FunctionalObjectProperty(P) generates: P(x,y1) ∧ P(x,y2) → y1 = y2
        let var_x = self.fresh_variable();
        let var_y1 = self.fresh_variable();
        let var_y2 = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y1);
        let prop2_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y2);

        // Functional constraint: P(x,y1) ∧ P(x,y2) → y1 = y2
        let equality_atom = DLAtom::new(format!("[{var_y1} == {var_y2}]"), vec![]);

        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );

        // Also generate HermiT-style atMost constraint
        let range_str = self
            .get_property_range(&property_name)
            .unwrap_or_else(|| "owl:Thing".to_string());

        let at_most_atom =
            self.create_at_most_atom(1, &property_name, &range_str, &var_x, false)?;
        let functional_clause = DLClause::new(vec![at_most_atom], vec![], self.next_clause_id());

        Ok(vec![clause, functional_clause])
    }

    fn compile_functional_data_property_axiom(
        &mut self,
        axiom: &crate::ontology::FunctionalDataPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // FunctionalDataProperty(P) generates: P(x,v1) ∧ P(x,v2) → v1 = v2
        let var_x = self.fresh_variable();
        let var_v1 = self.fresh_variable();
        let var_v2 = self.fresh_variable();

        let property_name = self.data_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_v1);
        let prop2_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_v2);

        // Functional constraint: P(x,v1) ∧ P(x,v2) → v1 = v2
        let equality_atom = DLAtom::new(format!("[{var_v1} == {var_v2}]"), vec![]);

        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_inverse_functional_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::InverseFunctionalObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // InverseFunctionalObjectProperty(P) generates: P(x1,y) ∧ P(x2,y) → x1 = x2
        let var_x1 = self.fresh_variable();
        let var_x2 = self.fresh_variable();
        let var_y = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::role_assertion(&property_name, &var_x1, &var_y);
        let prop2_atom = DLAtom::role_assertion(&property_name, &var_x2, &var_y);

        // Inverse functional constraint: P(x1,y) ∧ P(x2,y) → x1 = x2
        let equality_atom = DLAtom::new(format!("[{var_x1} == {var_x2}]"), vec![]);

        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_reflexive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::ReflexiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // ReflexiveObjectProperty(P) generates: Thing(x) → P(x,x)
        // Note: In practice, this should use the domain of the property
        let var_x = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let reflexive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);

        // For reflexive properties, we use Thing(x) as a general domain
        // In a more sophisticated implementation, we would determine the actual domain
        let thing_atom = DLAtom::concept_assertion("owl:Thing", &var_x);

        let clause = DLClause::new(
            vec![reflexive_atom],
            vec![thing_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_irreflexive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::IrreflexiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // IrreflexiveObjectProperty(P) generates constraint: ¬P(x,x)
        let var_x = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let reflexive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);

        // Irreflexive property: ¬P(x,x) - constraint clause (empty head)
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![reflexive_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_symmetric_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::SymmetricObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // SymmetricObjectProperty(P) generates: P(x,y) → P(y,x)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let forward_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let backward_atom = DLAtom::role_assertion(&property_name, &var_y, &var_x);

        // Symmetric property: P(x,y) → P(y,x)
        let clause = DLClause::new(
            vec![backward_atom],
            vec![forward_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_asymmetric_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::AsymmetricObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // AsymmetricObjectProperty(P) generates constraint: ¬(P(x,y) ∧ P(y,x))
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let forward_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let backward_atom = DLAtom::role_assertion(&property_name, &var_y, &var_x);

        // Asymmetric property: ¬(P(x,y) ∧ P(y,x)) - constraint clause (empty head)
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![forward_atom, backward_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_transitive_object_property_axiom(
        &mut self,
        axiom: &crate::ontology::TransitiveObjectPropertyAxiom,
    ) -> Result<Vec<DLClause>> {
        // TransitiveObjectProperty(P) generates: P(x,y) ∧ P(y,z) → P(x,z)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        let var_z = self.fresh_variable();

        let property_name = self.object_property_expression_to_string(&axiom.property);
        let first_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let second_atom = DLAtom::role_assertion(&property_name, &var_y, &var_z);
        let transitive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_z);

        // Transitive property: P(x,y) ∧ P(y,z) → P(x,z)
        let clause = DLClause::new(
            vec![transitive_atom],
            vec![first_atom, second_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_inverse_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::InverseObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>> {
        // InverseObjectProperties(P,Q) generates: P(x,y) ↔ Q(y,x)
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();

        let prop1_name = self.object_property_expression_to_string(&axiom.property1);
        let prop2_name = self.object_property_expression_to_string(&axiom.property2);

        let prop1_atom = DLAtom::role_assertion(&prop1_name, &var_x, &var_y);
        let prop2_atom = DLAtom::role_assertion(&prop2_name, &var_y, &var_x);

        // P(x,y) → Q(y,x)
        let forward_clause = DLClause::new(
            vec![prop2_atom.clone()],
            vec![prop1_atom.clone()],
            self.next_clause_id(),
        );

        // Q(y,x) → P(x,y)
        let backward_clause =
            DLClause::new(vec![prop1_atom], vec![prop2_atom], self.next_clause_id());

        Ok(vec![forward_clause, backward_clause])
    }

    fn compile_equivalent_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>> {
        // EquivalentObjectProperties(P1, P2, ..., Pn) generates bidirectional implications
        // Pi(x,y) ↔ Pj(x,y) for all pairs i,j
        let mut clauses = Vec::new();

        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_y = self.fresh_variable();

                let prop1_name = self.object_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.object_property_expression_to_string(&axiom.properties[j]);

                // Pi(x,y) → Pj(x,y)
                let prop1_atom = DLAtom::role_assertion(&prop1_name, &var_x, &var_y);
                let prop2_atom = DLAtom::role_assertion(&prop2_name, &var_x, &var_y);

                clauses.push(DLClause::new(
                    vec![prop2_atom.clone()],
                    vec![prop1_atom.clone()],
                    self.next_clause_id(),
                ));

                // Pj(x,y) → Pi(x,y)
                clauses.push(DLClause::new(
                    vec![prop1_atom],
                    vec![prop2_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok(clauses)
    }

    fn compile_equivalent_data_properties_axiom(
        &mut self,
        axiom: &crate::ontology::EquivalentDataPropertiesAxiom,
    ) -> Result<Vec<DLClause>> {
        // EquivalentDataProperties(P1, P2, ..., Pn) generates bidirectional implications
        // Pi(x,v) ↔ Pj(x,v) for all pairs i,j
        let mut clauses = Vec::new();

        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_v = self.fresh_variable();

                let prop1_name = self.data_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.data_property_expression_to_string(&axiom.properties[j]);

                // Pi(x,v) → Pj(x,v)
                let prop1_atom = DLAtom::datatype_assertion(&prop1_name, &var_x, &var_v);
                let prop2_atom = DLAtom::datatype_assertion(&prop2_name, &var_x, &var_v);

                clauses.push(DLClause::new(
                    vec![prop2_atom.clone()],
                    vec![prop1_atom.clone()],
                    self.next_clause_id(),
                ));

                // Pj(x,v) → Pi(x,v)
                clauses.push(DLClause::new(
                    vec![prop1_atom],
                    vec![prop2_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok(clauses)
    }

    fn compile_disjoint_object_properties_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointObjectPropertiesAxiom,
    ) -> Result<Vec<DLClause>> {
        // DisjointObjectProperties(P1, P2, ..., Pn) generates disjointness constraints
        // ¬(Pi(x,y) ∧ Pj(x,y)) for all pairs i,j
        let mut clauses = Vec::new();

        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_y = self.fresh_variable();

                let prop1_name = self.object_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.object_property_expression_to_string(&axiom.properties[j]);

                let prop1_atom = DLAtom::role_assertion(&prop1_name, &var_x, &var_y);
                let prop2_atom = DLAtom::role_assertion(&prop2_name, &var_x, &var_y);

                // Constraint: ¬(Pi(x,y) ∧ Pj(x,y))
                clauses.push(DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![prop1_atom, prop2_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok(clauses)
    }

    fn compile_disjoint_data_properties_axiom(
        &mut self,
        axiom: &crate::ontology::DisjointDataPropertiesAxiom,
    ) -> Result<Vec<DLClause>> {
        // DisjointDataProperties(P1, P2, ..., Pn) generates disjointness constraints
        // ¬(Pi(x,v) ∧ Pj(x,v)) for all pairs i,j
        let mut clauses = Vec::new();

        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_v = self.fresh_variable();

                let prop1_name = self.data_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.data_property_expression_to_string(&axiom.properties[j]);

                let prop1_atom = DLAtom::datatype_assertion(&prop1_name, &var_x, &var_v);
                let prop2_atom = DLAtom::datatype_assertion(&prop2_name, &var_x, &var_v);

                // Constraint: ¬(Pi(x,v) ∧ Pj(x,v))
                clauses.push(DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![prop1_atom, prop2_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok(clauses)
    }

    fn compile_same_individual_axiom(
        &mut self,
        axiom: &crate::ontology::SameIndividualAxiom,
    ) -> Result<Vec<DLClause>> {
        // SameIndividual(a,b,c) generates equality facts: a=b, a=c, b=c
        let mut clauses = Vec::new();

        // Generate equality clauses for all pairs
        for i in 0..axiom.individuals.len() {
            for j in (i + 1)..axiom.individuals.len() {
                let ind1_name = self.individual_to_string(&axiom.individuals[i]);
                let ind2_name = self.individual_to_string(&axiom.individuals[j]);

                let equality_atom = DLAtom::new(format!("[{ind1_name} == {ind2_name}]"), vec![]);

                let clause = DLClause::new(
                    vec![equality_atom],
                    vec![], // Fact
                    self.next_clause_id(),
                );

                clauses.push(clause);
            }
        }

        Ok(clauses)
    }

    fn compile_different_individuals_axiom(
        &mut self,
        axiom: &crate::ontology::DifferentIndividualsAxiom,
    ) -> Result<Vec<DLClause>> {
        // DifferentIndividuals(a,b,c) generates inequality constraints: ¬(a=b), ¬(a=c), ¬(b=c)
        let mut clauses = Vec::new();

        // Generate inequality constraint clauses for all pairs
        for i in 0..axiom.individuals.len() {
            for j in (i + 1)..axiom.individuals.len() {
                let ind1_name = self.individual_to_string(&axiom.individuals[i]);
                let ind2_name = self.individual_to_string(&axiom.individuals[j]);

                let equality_atom = DLAtom::new(format!("[{ind1_name} == {ind2_name}]"), vec![]);

                let clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![equality_atom],
                    self.next_clause_id(),
                );

                clauses.push(clause);
            }
        }

        Ok(clauses)
    }

    fn compile_negative_object_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::NegativeObjectPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>> {
        // NegativeObjectPropertyAssertion(P, a, b) generates constraint: ¬P(a, b)
        let subject_name = self.individual_to_string(&axiom.source);
        let object_name = self.individual_to_string(&axiom.target);
        let property_name = self.object_property_expression_to_string(&axiom.property);

        let property_atom = DLAtom::role_assertion(&property_name, &subject_name, &object_name);

        // Negative assertion: ¬P(a, b) - constraint clause (empty head)
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![property_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_negative_data_property_assertion_axiom(
        &mut self,
        axiom: &crate::ontology::NegativeDataPropertyAssertionAxiom,
    ) -> Result<Vec<DLClause>> {
        // NegativeDataPropertyAssertion(P, a, v) generates constraint: ¬P(a, v)
        let subject_name = self.individual_to_string(&axiom.individual);
        let value_string = self.literal_to_string(&axiom.value);
        let property_name = self.data_property_expression_to_string(&axiom.property);

        let property_atom =
            DLAtom::datatype_assertion(&property_name, &subject_name, &value_string);

        // Negative assertion: ¬P(a, v) - constraint clause (empty head)
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![property_atom],
            self.next_clause_id(),
        );

        Ok(vec![clause])
    }

    fn compile_has_key_axiom(
        &mut self,
        axiom: &crate::ontology::HasKeyAxiom,
    ) -> Result<Vec<DLClause>> {
        // HasKey(C, P1, P2, ..., Pn) generates functional dependency constraints
        // C(x) ∧ C(y) ∧ P1(x,v1) ∧ P1(y,v1) ∧ ... ∧ Pn(x,vn) ∧ Pn(y,vn) → x = y
        // This means that if two individuals of class C have the same values for all key properties, they must be the same individual

        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        let mut body_atoms = Vec::new();

        // Add class constraints: C(x) ∧ C(y)
        let class_x_atom = self.compile_class_expression_to_atom(&axiom.class, &var_x, true)?;
        let class_y_atom = self.compile_class_expression_to_atom(&axiom.class, &var_y, true)?;
        body_atoms.push(class_x_atom);
        body_atoms.push(class_y_atom);

        // Add key property constraints
        for (idx, key_property) in axiom.object_properties.iter().enumerate() {
            let var_v = format!("V{idx}");
            let prop_name = self.object_property_expression_to_string(key_property);

            // Pi(x, vi) ∧ Pi(y, vi)
            body_atoms.push(DLAtom::role_assertion(&prop_name, &var_x, &var_v));
            body_atoms.push(DLAtom::role_assertion(&prop_name, &var_y, &var_v));
        }

        for (idx, key_property) in axiom.data_properties.iter().enumerate() {
            let var_v = format!("DV{idx}");
            let prop_name = self.data_property_expression_to_string(key_property);

            // Pi(x, vi) ∧ Pi(y, vi)
            body_atoms.push(DLAtom::datatype_assertion(&prop_name, &var_x, &var_v));
            body_atoms.push(DLAtom::datatype_assertion(&prop_name, &var_y, &var_v));
        }

        // Create equality constraint: x = y
        let equality_atom = DLAtom::new(format!("[{var_x} == {var_y}]"), vec![]);

        let clause = DLClause::new(vec![equality_atom], body_atoms, self.next_clause_id());

        Ok(vec![clause])
    }

    fn compile_swrl_rule_axiom(
        &mut self,
        axiom: &crate::ontology::SWRLRuleAxiom,
    ) -> Result<Vec<DLClause>> {
        // SWRL Rule compilation: Body → Head
        // Each SWRL rule is essentially a DL clause where body atoms are conditions and head atoms are conclusions
        let mut body_atoms = Vec::new();
        let mut head_atoms = Vec::new();

        // Compile body atoms (antecedent)
        for body_atom in &axiom.rule.body {
            match body_atom {
                crate::ontology::SWRLAtom::ClassAtom {
                    predicate,
                    argument,
                } => {
                    let arg_string = self.swrl_argument_to_string(argument);
                    let class_atom_dl =
                        self.compile_class_expression_to_atom(predicate, &arg_string, true)?;
                    body_atoms.push(class_atom_dl);
                }
                crate::ontology::SWRLAtom::ObjectPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    let prop_name = self.object_property_expression_to_string(predicate);
                    body_atoms.push(DLAtom::role_assertion(
                        &prop_name,
                        &arg1_string,
                        &arg2_string,
                    ));
                }
                crate::ontology::SWRLAtom::DataPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_dargument_to_string(second_argument);
                    let prop_name = self.data_property_expression_to_string(predicate);
                    body_atoms.push(DLAtom::datatype_assertion(
                        &prop_name,
                        &arg1_string,
                        &arg2_string,
                    ));
                }
                crate::ontology::SWRLAtom::SameIndividualAtom {
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    body_atoms.push(DLAtom::new(
                        format!("[{arg1_string} == {arg2_string}]"),
                        vec![],
                    ));
                }
                crate::ontology::SWRLAtom::DifferentIndividualsAtom {
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    // Different individuals as negated equality in body means this is a constraint
                    body_atoms.push(DLAtom::new(
                        format!("[{arg1_string} == {arg2_string}]"),
                        vec![],
                    ));
                }
                crate::ontology::SWRLAtom::BuiltInAtom {
                    predicate,
                    arguments,
                } => {
                    // For built-in atoms, create a special predicate representation
                    let builtin_name = format!("builtin:{predicate}");
                    let mut args = Vec::new();
                    for arg in arguments {
                        args.push(self.swrl_dargument_to_string(arg));
                    }
                    body_atoms.push(DLAtom::new(
                        format!("{}({})", builtin_name, args.join(", ")),
                        args,
                    ));
                }
                crate::ontology::SWRLAtom::DataRangeAtom {
                    predicate,
                    argument,
                } => {
                    // Data range atom: D(z) - check if a data value belongs to a data range
                    let arg_string = self.swrl_dargument_to_string(argument);
                    let range_constraint =
                        self.compile_data_range_to_constraint(predicate, &arg_string)?;
                    body_atoms.push(range_constraint);
                }
            }
        }

        // Compile head atoms (consequent)
        for head_atom in &axiom.rule.head {
            match head_atom {
                crate::ontology::SWRLAtom::ClassAtom {
                    predicate,
                    argument,
                } => {
                    let arg_string = self.swrl_argument_to_string(argument);
                    let class_atom_dl =
                        self.compile_class_expression_to_atom(predicate, &arg_string, false)?;
                    head_atoms.push(class_atom_dl);
                }
                crate::ontology::SWRLAtom::ObjectPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    let prop_name = self.object_property_expression_to_string(predicate);
                    head_atoms.push(DLAtom::role_assertion(
                        &prop_name,
                        &arg1_string,
                        &arg2_string,
                    ));
                }
                crate::ontology::SWRLAtom::DataPropertyAtom {
                    predicate,
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_dargument_to_string(second_argument);
                    let prop_name = self.data_property_expression_to_string(predicate);
                    head_atoms.push(DLAtom::datatype_assertion(
                        &prop_name,
                        &arg1_string,
                        &arg2_string,
                    ));
                }
                crate::ontology::SWRLAtom::SameIndividualAtom {
                    first_argument,
                    second_argument,
                } => {
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    head_atoms.push(DLAtom::new(
                        format!("[{arg1_string} == {arg2_string}]"),
                        vec![],
                    ));
                }
                crate::ontology::SWRLAtom::DifferentIndividualsAtom {
                    first_argument,
                    second_argument,
                } => {
                    // Different individuals in head would create a constraint - unusual but possible
                    let arg1_string = self.swrl_argument_to_string(first_argument);
                    let arg2_string = self.swrl_argument_to_string(second_argument);
                    head_atoms.push(DLAtom::new(
                        format!("[{arg1_string} != {arg2_string}]"),
                        vec![],
                    ));
                }
                crate::ontology::SWRLAtom::BuiltInAtom {
                    predicate,
                    arguments,
                } => {
                    // Built-in atoms in head are less common but possible
                    let builtin_name = format!("builtin:{predicate}");
                    let mut args = Vec::new();
                    for arg in arguments {
                        args.push(self.swrl_dargument_to_string(arg));
                    }
                    head_atoms.push(DLAtom::new(
                        format!("{}({})", builtin_name, args.join(", ")),
                        args,
                    ));
                }
                crate::ontology::SWRLAtom::DataRangeAtom {
                    predicate,
                    argument,
                } => {
                    // Data range atom in head: conclude that a data value belongs to a data range
                    let arg_string = self.swrl_dargument_to_string(argument);
                    let range_constraint =
                        self.compile_data_range_to_constraint(predicate, &arg_string)?;
                    head_atoms.push(range_constraint);
                }
            }
        }

        let clause = DLClause::new(head_atoms, body_atoms, self.next_clause_id());

        Ok(vec![clause])
    }
}
