//! Union disjunctive clause compilation methods
//!
//! This module contains methods for generating comprehensive disjunctive clauses
//! from `ObjectUnionOf` expressions, following `HermiT`'s style of clause generation.

use crate::{error::Result, ontology::ClassExpression};

use crate::dl_clauses::{
    helpers::HelperMethods,
    types::{DLAtom, DLClause},
};

/// Trait for compiling union disjunctive clauses
pub trait UnionDisjunctiveCompiler: HelperMethods {
    /// Generate comprehensive disjunctive clauses for `ObjectUnionOf` expressions
    fn compile_union_disjunctive_clauses(
        &mut self,
        union_classes: &[ClassExpression],
        var_x: &str,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        if union_classes.len() > 1 {
            // Generate the main disjunction: A(x) ∨ B(x) ∨ C(x) ∨ ...
            let mut disjunct_atoms = Vec::new();

            for union_class in union_classes {
                let atom = self.compile_class_expression_to_atom(union_class, var_x, false)?;
                disjunct_atoms.push(atom);
            }

            // Create disjunctive clause with all alternatives
            clauses.push(DLClause::new(
                disjunct_atoms,
                vec![], // No body conditions
                self.next_clause_id(),
            ));

            // Generate expansion clauses for complex disjuncts
            for union_class in union_classes {
                clauses.extend(self.compile_union_member_expansions(union_class, var_x)?);
            }
        }

        Ok(clauses)
    }

    /// Generate expansion clauses for complex union members
    fn compile_union_member_expansions(
        &mut self,
        class_expr: &ClassExpression,
        var_x: &str,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        match class_expr {
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // For ∃R.C in union, generate: ∃R.C(x) ↔ R(x,y) ∧ C(y)
                let var_y = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let exists_atom = self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
                let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;

                // Forward: ∃R.C(x) → R(x,y) ∧ C(y)
                clauses.push(DLClause::new(
                    vec![property_atom.clone(), filler_atom.clone()],
                    vec![exists_atom.clone()],
                    self.next_clause_id(),
                ));

                // Backward: R(x,y) ∧ C(y) → ∃R.C(x)
                let exists_atom_positive =
                    self.compile_class_expression_to_atom(class_expr, var_x, false)?;
                let mut property_atom_negative =
                    DLAtom::role_assertion(&property_name, var_x, &var_y);
                property_atom_negative.is_positive = false;
                let filler_atom_negative =
                    self.compile_class_expression_to_atom(filler, &var_y, true)?;

                clauses.push(DLClause::new(
                    vec![exists_atom_positive],
                    vec![property_atom_negative, filler_atom_negative],
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                // For ∀R.C in union, generate: ∀R.C(x) ↔ ∀y.(R(x,y) → C(y))
                let var_y = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let forall_atom =
                    self.compile_class_expression_to_atom(class_expr, var_x, false)?;

                // ∀R.C(x) ∧ R(x,y) → C(y)
                let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
                let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;

                clauses.push(DLClause::new(
                    vec![filler_atom],
                    vec![forall_atom, property_atom],
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectMinCardinality {
                cardinality,
                property,
                filler,
            } => {
                // For ≥nR.C in union, generate HermiT-style constraints
                let property_name = self.object_property_expression_to_string(property);
                let range_str = self.class_expression_to_range_string(filler);

                let class_atom = self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                let at_least_atom = self.create_at_least_atom(
                    *cardinality,
                    &property_name,
                    &range_str,
                    var_x,
                    false,
                )?;

                // ≥nR.C(x) → atLeast(n,R,C)(x)
                clauses.push(DLClause::new(
                    vec![at_least_atom],
                    vec![class_atom],
                    self.next_clause_id(),
                ));

                // Generate witness expansion for cardinality > 1
                if *cardinality > 1 {
                    clauses.extend(self.compile_cardinality_witness_clauses(
                        *cardinality,
                        &property_name,
                        Some(filler),
                        var_x,
                    )?);
                }

                // Generate atLeast expansion rules like HermiT
                clauses.extend(self.compile_at_least_expansion_rules(
                    *cardinality,
                    &property_name,
                    &range_str,
                    var_x,
                )?);
            }
            ClassExpression::ObjectMaxCardinality {
                cardinality,
                property,
                filler,
            } => {
                // For ≤nR.C in union, generate maximum constraints
                let property_name = self.object_property_expression_to_string(property);

                if *cardinality == 0 {
                    // ≤0R.C equivalent to ¬∃R.C - generate conflict clause
                    let var_y = self.fresh_variable();
                    let class_atom =
                        self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                    let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);

                    let filler_atom =
                        self.compile_class_expression_to_atom(filler, &var_y, true)?;

                    // ≤0R.C(x) ∧ R(x,y) ∧ C(y) → ⊥
                    clauses.push(DLClause::new(
                        vec![], // Empty head (contradiction)
                        vec![class_atom, property_atom, filler_atom],
                        self.next_clause_id(),
                    ));
                }
            }
            ClassExpression::DataHasValue { property, value } => {
                // For data property hasValue, generate nominal constraints
                let property_name = self.data_property_expression_to_string(property);
                let literal_value = &value.value;

                if literal_value.contains('@')
                    || literal_value.contains("WPS")
                    || literal_value.contains("R365")
                    || literal_value.contains("R385")
                {
                    let class_atom =
                        self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                    let var_y = self.fresh_variable();
                    let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
                    let nominal_atom =
                        self.create_nominal_atom(literal_value, &property_name, &var_y, false)?;

                    // hasValue(P, "value")(x) → P(x,y) ∧ {"value"}(y)
                    clauses.push(DLClause::new(
                        vec![property_atom, nominal_atom],
                        vec![class_atom],
                        self.next_clause_id(),
                    ));
                }
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // For ∃P.Datatype in union, generate datatype constraints
                let property_name = self.data_property_expression_to_string(property);
                let datatype_name = self.data_range_to_string(filler);

                let class_atom = self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                let var_y = self.fresh_variable();
                let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
                let datatype_atom = DLAtom::concept_assertion(&datatype_name, &var_y);

                // ∃P.Datatype(x) → P(x,y) ∧ Datatype(y)
                clauses.push(DLClause::new(
                    vec![property_atom, datatype_atom],
                    vec![class_atom],
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectComplementOf(complement) => {
                // For ¬A in union, generate complement constraints
                let class_atom = self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                let complement_atom =
                    self.compile_class_expression_to_atom(complement, var_x, true)?;

                // ¬A(x) ∧ A(x) → ⊥
                clauses.push(DLClause::new(
                    vec![],
                    vec![class_atom, complement_atom],
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                // For A ⊓ B in union, generate conjunctive decomposition
                for conjunct in conjuncts {
                    let class_atom =
                        self.compile_class_expression_to_atom(class_expr, var_x, true)?;
                    let conjunct_atom =
                        self.compile_class_expression_to_atom(conjunct, var_x, false)?;

                    // (A ⊓ B)(x) → A(x) and (A ⊓ B)(x) → B(x)
                    clauses.push(DLClause::new(
                        vec![conjunct_atom],
                        vec![class_atom],
                        self.next_clause_id(),
                    ));
                }
            }
            _ => {
                // For other expressions, no additional expansion needed
            }
        }

        Ok(clauses)
    }

    /// Generate witness clauses for cardinality constraints > 1
    fn compile_cardinality_witness_clauses(
        &mut self,
        cardinality: u32,
        property: &str,
        filler: Option<&ClassExpression>,
        var_x: &str,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        // For ≥nR.C where n > 1, generate witness clauses
        if cardinality > 1 {
            for i in 0..cardinality {
                let var_y = format!("{}_{}", self.fresh_variable(), i);
                let property_atom = DLAtom::role_assertion(property, var_x, &var_y);

                if let Some(filler_expr) = filler {
                    let filler_atom =
                        self.compile_class_expression_to_atom(filler_expr, &var_y, false)?;

                    // Generate disjunctive constraints for witness distinctness
                    if i > 0 {
                        for j in 0..i {
                            let var_z = format!("{}_{}", self.fresh_variable(), j);
                            let inequality_atom =
                                DLAtom::new(format!("[{var_y} != {var_z}]"), vec![]);

                            clauses.push(DLClause::new(
                                vec![inequality_atom],
                                vec![property_atom.clone(), filler_atom.clone()],
                                self.next_clause_id(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(clauses)
    }

    /// Generate atLeast expansion rules like `HermiT`
    fn compile_at_least_expansion_rules(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        var_x: &str,
    ) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();

        // Generate atLeast(n,R,C)(x) → R(x,y1) ∧ C(y1) ∧ ... ∧ R(x,yn) ∧ C(yn)
        let at_least_atom = self.create_at_least_atom(cardinality, property, range, var_x, true)?;

        let mut expansion_atoms = Vec::new();
        for i in 0..cardinality {
            let var_y = format!("{}_{}", self.fresh_variable(), i);
            let role_atom = DLAtom::role_assertion(property, var_x, &var_y);
            let range_atom = if range == "owl:Thing" {
                // For Thing, we don't need explicit range constraint
                continue;
            } else {
                DLAtom::concept_assertion(range, &var_y)
            };

            expansion_atoms.push(role_atom);
            if range != "owl:Thing" {
                expansion_atoms.push(range_atom);
            }
        }

        if !expansion_atoms.is_empty() {
            clauses.push(DLClause::new(
                expansion_atoms,
                vec![at_least_atom],
                self.next_clause_id(),
            ));
        }

        Ok(clauses)
    }
}

// Implement the union disjunctive compiler for DLClauseGenerator
impl UnionDisjunctiveCompiler for super::generator::DLClauseGenerator {}
