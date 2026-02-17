//! Helper methods for DL clause generation
//!
//! This module contains utility methods for string conversion, atom creation,
//! and other helper functions used throughout the DL clause generation process.

use crate::{
    error::Result,
    ontology::{
        ClassExpression, DataPropertyExpression, DataRange, Individual, Literal,
        ObjectPropertyExpression,
    },
};

use crate::dl_clauses::types::{DLAtom, DLClause};

/// Trait containing helper methods for DL clause generation
pub trait HelperMethods {
    /// Generate a fresh variable name
    fn fresh_variable(&mut self) -> String;

    /// Generate a fresh clause ID
    fn next_clause_id(&mut self) -> String;

    /// Generate a fresh definition name
    #[allow(dead_code)]
    fn next_definition(&mut self) -> String;

    /// Compile a class expression to a DL atom
    fn compile_class_expression_to_atom(
        &mut self,
        expr: &ClassExpression,
        variable: &str,
        is_body: bool,
    ) -> Result<DLAtom>;

    /// Convert object property expression to string
    fn object_property_expression_to_string(&self, expr: &ObjectPropertyExpression) -> String;

    /// Convert data property expression to string
    fn data_property_expression_to_string(&self, expr: &DataPropertyExpression) -> String;

    /// Convert data range to string
    fn data_range_to_string(&self, range: &DataRange) -> String;

    /// Convert individual to string
    fn individual_to_string(&self, individual: &Individual) -> String;

    /// Convert literal to string
    fn literal_to_string(&self, literal: &Literal) -> String;

    /// Compile data range to constraint atom
    fn compile_data_range_to_constraint(
        &mut self,
        range: &DataRange,
        variable: &str,
    ) -> Result<DLAtom>;

    /// Convert class expression to range string (for cardinality atoms)
    fn class_expression_to_range_string(&self, expr: &ClassExpression) -> String;

    /// Create HermiT-style atLeast atom
    fn create_at_least_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom>;

    /// Create HermiT-style atMost atom  
    fn create_at_most_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom>;

    /// Create nominal atom for hasValue expressions
    fn create_nominal_atom(
        &mut self,
        value: &str,
        property: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom>;

    /// Get the range of a property for cardinality constraints  
    fn get_property_range(&self, property: &str) -> Option<String>;

    /// Introduce a definition for a complex class expression
    fn introduce_definition(
        &mut self,
        expr: &ClassExpression,
        variable: &str,
    ) -> Result<(DLAtom, Vec<DLClause>)>;

    /// Check if a class expression is a simple named class
    #[allow(dead_code)]
    fn is_named_class(&self, expr: &ClassExpression) -> bool;

    /// Convert SWRL individual argument to string
    fn swrl_argument_to_string(&self, arg: &crate::ontology::SWRLIArgument) -> String;

    /// Convert SWRL data argument to string  
    fn swrl_dargument_to_string(&self, arg: &crate::ontology::SWRLDArgument) -> String;
}

impl HelperMethods for super::generator::DLClauseGenerator {
    fn fresh_variable(&mut self) -> String {
        self.variable_counter += 1;
        format!("X{}", self.variable_counter)
    }

    fn next_clause_id(&mut self) -> String {
        self.clause_counter += 1;
        format!("clause_{}", self.clause_counter)
    }

    fn next_definition(&mut self) -> String {
        self.definition_counter += 1;
        format!("def:{}", self.definition_counter)
    }

    fn create_at_least_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("atLeast({cardinality},{property},{range})");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }

    fn create_at_most_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("atMost({cardinality},{property},{range})");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }

    fn create_nominal_atom(
        &mut self,
        value: &str,
        _property: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("{{{value}}}");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }

    fn compile_class_expression_to_atom(
        &mut self,
        expr: &ClassExpression,
        variable: &str,
        is_body: bool,
    ) -> Result<DLAtom> {
        match expr {
            ClassExpression::Class(class) => {
                let class_name = self.iri_to_string(&class.iri);
                Ok(DLAtom::concept_assertion(&class_name, variable).with_negation(!is_body))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // For complex expressions, use a simplified representation
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.class_expression_to_simple_string(filler);
                let predicate = format!("∃{property_name}.{filler_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.class_expression_to_simple_string(filler);
                let predicate = format!("∀{property_name}.{filler_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectMinCardinality {
                cardinality,
                property,
                filler,
            } => {
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.class_expression_to_simple_string(filler);
                let predicate = format!("≥{cardinality}{property_name}.{filler_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectMaxCardinality {
                cardinality,
                property,
                filler,
            } => {
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.class_expression_to_simple_string(filler);
                let predicate = format!("≤{cardinality}{property_name}.{filler_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectExactCardinality {
                cardinality,
                property,
                filler,
            } => {
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.class_expression_to_simple_string(filler);
                let predicate = format!("={cardinality}{property_name}.{filler_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectHasSelf { property } => {
                let property_name = self.object_property_expression_to_string(property);
                let predicate = format!("∃{property_name}.Self");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectHasValue { property, value } => {
                let property_name = self.object_property_expression_to_string(property);
                let individual_name = self.individual_to_string(value);
                let predicate = format!("∃{property_name}.{{{individual_name}}}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::DataHasValue { property, value } => {
                let property_name = self.data_property_expression_to_string(property);
                let literal_value = &value.value;
                let predicate = format!("∃{property_name}.{{{literal_value}}}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                let property_name = self.data_property_expression_to_string(property);
                let datatype_name = self.data_range_to_string(filler);
                let predicate = format!("∃{property_name}.{datatype_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                let property_name = self.data_property_expression_to_string(property);
                let datatype_name = self.data_range_to_string(filler);
                let predicate = format!("∀{property_name}.{datatype_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectIntersectionOf(operands) => {
                let intersection_parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.class_expression_to_simple_string(op))
                    .collect();
                let predicate = format!("({})", intersection_parts.join(" ⊓ "));
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectUnionOf(operands) => {
                let union_parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.class_expression_to_simple_string(op))
                    .collect();
                let predicate = format!("({})", union_parts.join(" ⊔ "));
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectComplementOf(operand) => {
                let operand_name = self.class_expression_to_simple_string(operand);
                let predicate = format!("¬{operand_name}");
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            ClassExpression::ObjectOneOf(individuals) => {
                let individual_names: Vec<String> = individuals
                    .iter()
                    .map(|ind| self.individual_to_string(ind))
                    .collect();
                let predicate = format!("{{{}}}", individual_names.join(","));
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
            _ => {
                // Fallback for unsupported expressions
                let predicate = format!("UnsupportedExpression_{:?}", std::mem::discriminant(expr));
                Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(!is_body))
            }
        }
    }

    fn object_property_expression_to_string(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(prop) => self.iri_to_string(&prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("inv({})", self.iri_to_string(&prop.iri))
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                let chain_parts: Vec<String> = chain
                    .iter()
                    .map(|prop| self.object_property_expression_to_string(prop))
                    .collect();
                format!("({})", chain_parts.join(" ∘ "))
            }
        }
    }

    fn data_property_expression_to_string(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(prop) => self.iri_to_string(&prop.iri),
        }
    }

    fn data_range_to_string(&self, range: &DataRange) -> String {
        match range {
            DataRange::Datatype(datatype) => self.iri_to_string(datatype),
            DataRange::DataIntersectionOf(operands) => {
                let intersection_parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.data_range_to_string(op))
                    .collect();
                format!("({})", intersection_parts.join(" ⊓ "))
            }
            DataRange::DataUnionOf(operands) => {
                let union_parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.data_range_to_string(op))
                    .collect();
                format!("({})", union_parts.join(" ⊔ "))
            }
            DataRange::DataComplementOf(operand) => {
                format!("¬{}", self.data_range_to_string(operand))
            }
            DataRange::DataOneOf(literals) => {
                let literal_values: Vec<String> = literals
                    .iter()
                    .map(|lit| format!("\"{}\"", lit.value))
                    .collect();
                format!("{{{}}}", literal_values.join(","))
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                let base_type = self.iri_to_string(datatype);
                if restrictions.is_empty() {
                    base_type
                } else {
                    // Simplified representation of facet restrictions
                    format!("{base_type}[restrictions]")
                }
            }
        }
    }

    fn individual_to_string(&self, individual: &Individual) -> String {
        match individual {
            Individual::Named(named) => self.iri_to_string(&named.iri),
            Individual::Anonymous(anon) => format!("_:{}", anon.id),
        }
    }

    fn literal_to_string(&self, literal: &Literal) -> String {
        literal.value.clone()
    }

    fn compile_data_range_to_constraint(
        &mut self,
        range: &DataRange,
        variable: &str,
    ) -> Result<DLAtom> {
        match range {
            DataRange::Datatype(dt) => {
                let datatype_str = format!("{dt}");
                Ok(DLAtom::new(
                    format!("{datatype_str}({variable})"),
                    vec![variable.to_string()],
                ))
            }
            _ => {
                let range_string = self.data_range_to_string(range);
                Ok(DLAtom::new(
                    format!("{range_string}({variable})"),
                    vec![variable.to_string()],
                ))
            }
        }
    }

    fn swrl_argument_to_string(&self, arg: &crate::ontology::SWRLIArgument) -> String {
        match arg {
            crate::ontology::SWRLIArgument::Individual(ind) => self.individual_to_string(ind),
            crate::ontology::SWRLIArgument::Variable(var) => format!("?{}", var.iri),
        }
    }

    fn swrl_dargument_to_string(&self, arg: &crate::ontology::SWRLDArgument) -> String {
        match arg {
            crate::ontology::SWRLDArgument::Literal(lit) => self.literal_to_string(lit),
            crate::ontology::SWRLDArgument::Variable(var) => format!("?{}", var.iri),
        }
    }

    fn class_expression_to_range_string(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => self.iri_to_string(&class.iri),
            _ => {
                // For complex expressions, use a simplified representation
                self.class_expression_to_simple_string(expr)
            }
        }
    }

    fn get_property_range(&self, _property: &str) -> Option<String> {
        // This would look up the range of the property in the ontology
        // For now, return a default range
        Some("owl:Thing".to_string())
    }

    fn introduce_definition(
        &mut self,
        expr: &ClassExpression,
        variable: &str,
    ) -> Result<(DLAtom, Vec<DLClause>)> {
        let def_name = self.next_definition();
        let def_atom = DLAtom::concept_assertion(&def_name, variable);

        // Generate definition clauses
        let mut def_clauses = Vec::new();

        match expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                // def:N(x) ↔ A(x) ∧ B(x) ∧ ...

                // Forward: def:N(x) → A(x), def:N(x) → B(x), ...
                for conjunct in conjuncts {
                    let conjunct_atom =
                        self.compile_class_expression_to_atom(conjunct, variable, false)?;
                    def_clauses.push(DLClause::new(
                        vec![conjunct_atom],
                        vec![def_atom.clone()],
                        self.next_clause_id(),
                    ));
                }

                // Backward: A(x) ∧ B(x) ∧ ... → def:N(x)
                let mut body_atoms = Vec::new();
                for conjunct in conjuncts {
                    let conjunct_atom =
                        self.compile_class_expression_to_atom(conjunct, variable, true)?;
                    body_atoms.push(conjunct_atom);
                }
                def_clauses.push(DLClause::new(
                    vec![def_atom.clone()],
                    body_atoms,
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // def:N(x) ↔ ∃R.C(x)
                let var_y = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let property_atom = DLAtom::role_assertion(&property_name, variable, &var_y);
                let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;

                // Forward: def:N(x) → R(x,y) ∧ C(y)
                def_clauses.push(DLClause::new(
                    vec![property_atom.clone(), filler_atom.clone()],
                    vec![def_atom.clone()],
                    self.next_clause_id(),
                ));

                // Backward: R(x,y) ∧ C(y) → def:N(x)
                def_clauses.push(DLClause::new(
                    vec![def_atom.clone()],
                    vec![property_atom, filler_atom],
                    self.next_clause_id(),
                ));
            }
            _ => {
                // For other expressions, generate a simple equivalence
                let expr_atom = self.compile_class_expression_to_atom(expr, variable, false)?;

                // Forward: def:N(x) → Expr(x)
                def_clauses.push(DLClause::new(
                    vec![expr_atom.clone()],
                    vec![def_atom.clone()],
                    self.next_clause_id(),
                ));

                // Backward: Expr(x) → def:N(x)
                def_clauses.push(DLClause::new(
                    vec![def_atom.clone()],
                    vec![expr_atom],
                    self.next_clause_id(),
                ));
            }
        }

        Ok((def_atom, def_clauses))
    }

    fn is_named_class(&self, expr: &ClassExpression) -> bool {
        matches!(expr, ClassExpression::Class(_))
    }
}

impl super::generator::DLClauseGenerator {
    /// Convert IRI to string with prefix handling
    #[must_use]
    pub fn iri_to_string(&self, iri: &crate::ontology::IRI) -> String {
        let iri_str = iri.as_str();

        // Try to find a matching prefix
        for (prefix, namespace) in &self.prefixes {
            if iri_str.starts_with(namespace) {
                let local_name = &iri_str[namespace.len()..];
                if prefix.is_empty() {
                    return local_name.to_string();
                } else {
                    return format!("{prefix}:{local_name}");
                }
            }
        }

        // If no prefix found, try to extract a reasonable local name
        if let Some(hash_pos) = iri_str.rfind('#') {
            return iri_str[hash_pos + 1..].to_string();
        } else if let Some(slash_pos) = iri_str.rfind('/') {
            return iri_str[slash_pos + 1..].to_string();
        }

        // Fallback to full IRI
        iri_str.to_string()
    }

    /// Convert URL to string with prefix handling
    #[must_use]
    pub fn url_to_string(&self, url: &url::Url) -> String {
        let url_str = url.as_str();

        // Try to find a matching prefix
        for (prefix, namespace) in &self.prefixes {
            if url_str.starts_with(namespace) {
                let local_name = &url_str[namespace.len()..];
                if prefix.is_empty() {
                    return local_name.to_string();
                } else {
                    return format!("{prefix}:{local_name}");
                }
            }
        }

        // If no prefix found, try to extract a reasonable local name
        if let Some(hash_pos) = url_str.rfind('#') {
            return url_str[hash_pos + 1..].to_string();
        } else if let Some(slash_pos) = url_str.rfind('/') {
            return url_str[slash_pos + 1..].to_string();
        }

        // Fallback to full URL
        url_str.to_string()
    }

    /// Convert class expression to a simple string representation
    #[must_use]
    pub fn class_expression_to_simple_string(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => self.iri_to_string(&class.iri),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!(
                    "∃{}.{}",
                    self.object_property_expression_to_string(property),
                    self.class_expression_to_simple_string(filler)
                )
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!(
                    "∀{}.{}",
                    self.object_property_expression_to_string(property),
                    self.class_expression_to_simple_string(filler)
                )
            }
            ClassExpression::ObjectMinCardinality {
                cardinality,
                property,
                filler,
            } => {
                format!(
                    "≥{}{}.{}",
                    cardinality,
                    self.object_property_expression_to_string(property),
                    self.class_expression_to_simple_string(filler)
                )
            }
            ClassExpression::ObjectMaxCardinality {
                cardinality,
                property,
                filler,
            } => {
                format!(
                    "≤{}{}.{}",
                    cardinality,
                    self.object_property_expression_to_string(property),
                    self.class_expression_to_simple_string(filler)
                )
            }
            ClassExpression::ObjectExactCardinality {
                cardinality,
                property,
                filler,
            } => {
                format!(
                    "={}{}.{}",
                    cardinality,
                    self.object_property_expression_to_string(property),
                    self.class_expression_to_simple_string(filler)
                )
            }
            ClassExpression::ObjectComplementOf(operand) => {
                format!("¬{}", self.class_expression_to_simple_string(operand))
            }
            ClassExpression::ObjectIntersectionOf(operands) => {
                let parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.class_expression_to_simple_string(op))
                    .collect();
                format!("({})", parts.join(" ⊓ "))
            }
            ClassExpression::ObjectUnionOf(operands) => {
                let parts: Vec<String> = operands
                    .iter()
                    .map(|op| self.class_expression_to_simple_string(op))
                    .collect();
                format!("({})", parts.join(" ⊔ "))
            }
            _ => {
                // Simplified representation for other expressions
                format!("Complex_{:?}", std::mem::discriminant(expr))
            }
        }
    }

    #[allow(dead_code)]
    fn create_at_least_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("atLeast({cardinality},{property},{range})");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }

    #[allow(dead_code)]
    fn create_at_most_atom(
        &mut self,
        cardinality: u32,
        property: &str,
        range: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("atMost({cardinality},{property},{range})");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }

    #[allow(dead_code)]
    fn create_nominal_atom(
        &mut self,
        value: &str,
        _property: &str,
        variable: &str,
        is_negative: bool,
    ) -> Result<DLAtom> {
        let predicate = format!("{{{value}}}");
        Ok(DLAtom::new(predicate, vec![variable.to_string()]).with_negation(is_negative))
    }
}
