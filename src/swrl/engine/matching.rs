//! SWRL Pattern Matching and Unification
//!
//! This module implements pattern matching algorithms for SWRL atoms
//! and unification logic for rule matching and goal resolution.

use crate::Result;
use crate::ontology::{ClassExpression, ObjectPropertyExpression};
use crate::swrl::{SWRLAtom, SWRLDArgument, SWRLIArgument};
use std::collections::HashMap;

/// Unification engine for SWRL atoms and arguments
#[derive(Debug)]
pub struct UnificationEngine {
    /// Variable substitution map
    substitutions: HashMap<String, SWRLIArgument>,
    /// Data variable substitution map
    data_substitutions: HashMap<String, SWRLDArgument>,
}

impl UnificationEngine {
    /// Create a new unification engine
    pub fn new() -> Self {
        Self {
            substitutions: HashMap::new(),
            data_substitutions: HashMap::new(),
        }
    }

    /// Check if two atoms can be unified
    pub fn atoms_unify(&mut self, atom1: &SWRLAtom, atom2: &SWRLAtom) -> Result<bool> {
        match (atom1, atom2) {
            (
                SWRLAtom::ClassAtom {
                    predicate: p1,
                    argument: a1,
                },
                SWRLAtom::ClassAtom {
                    predicate: p2,
                    argument: a2,
                },
            ) => Ok(self.class_expressions_match(p1, p2) && self.arguments_unify(a1, a2)),
            (
                SWRLAtom::DataRangeAtom {
                    predicate: p1,
                    argument: a1,
                },
                SWRLAtom::DataRangeAtom {
                    predicate: p2,
                    argument: a2,
                },
            ) => Ok(self.data_ranges_match(p1, p2) && self.data_arguments_unify(a1, a2)),
            (
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p1,
                    first_argument: a1,
                    second_argument: b1,
                },
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p2,
                    first_argument: a2,
                    second_argument: b2,
                },
            ) => Ok(self.object_properties_match(p1, p2)
                && self.arguments_unify(a1, a2)
                && self.arguments_unify(b1, b2)),
            (
                SWRLAtom::DataPropertyAtom {
                    predicate: p1,
                    first_argument: a1,
                    second_argument: b1,
                },
                SWRLAtom::DataPropertyAtom {
                    predicate: p2,
                    first_argument: a2,
                    second_argument: b2,
                },
            ) => Ok(self.data_properties_match(p1, p2)
                && self.arguments_unify(a1, a2)
                && self.data_arguments_unify(b1, b2)),
            (
                SWRLAtom::SameIndividualAtom {
                    first_argument: a1,
                    second_argument: b1,
                },
                SWRLAtom::SameIndividualAtom {
                    first_argument: a2,
                    second_argument: b2,
                },
            ) => Ok(self.arguments_unify(a1, a2) && self.arguments_unify(b1, b2)),
            (
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: a1,
                    second_argument: b1,
                },
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: a2,
                    second_argument: b2,
                },
            ) => Ok(self.arguments_unify(a1, a2) && self.arguments_unify(b1, b2)),
            (
                SWRLAtom::BuiltInAtom {
                    predicate: p1,
                    arguments: args1,
                },
                SWRLAtom::BuiltInAtom {
                    predicate: p2,
                    arguments: args2,
                },
            ) => Ok(p1.as_str() == p2.as_str()
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| self.data_arguments_unify(a1, a2))),
            _ => Ok(false), // Different atom types don't unify
        }
    }

    /// Check if two individual arguments can be unified
    fn arguments_unify(&mut self, arg1: &SWRLIArgument, arg2: &SWRLIArgument) -> bool {
        match (arg1, arg2) {
            (SWRLIArgument::Individual(ind1), SWRLIArgument::Individual(ind2)) => {
                ind1.iri().map(|i| i.as_str()) == ind2.iri().map(|i| i.as_str())
            }
            (SWRLIArgument::Variable(var), SWRLIArgument::Individual(ind)) => self
                .bind_individual_variable(
                    var.iri.to_string(),
                    SWRLIArgument::Individual(ind.clone()),
                ),
            (SWRLIArgument::Individual(ind), SWRLIArgument::Variable(var)) => self
                .bind_individual_variable(
                    var.iri.to_string(),
                    SWRLIArgument::Individual(ind.clone()),
                ),
            (SWRLIArgument::Variable(var1), SWRLIArgument::Variable(var2)) => {
                if var1.iri == var2.iri {
                    true
                } else {
                    // For now, assume different variables can unify
                    self.bind_individual_variable(
                        var1.iri.to_string(),
                        SWRLIArgument::Variable(var2.clone()),
                    )
                }
            }
        }
    }

    /// Check if two data arguments can be unified
    fn data_arguments_unify(&mut self, arg1: &SWRLDArgument, arg2: &SWRLDArgument) -> bool {
        match (arg1, arg2) {
            (SWRLDArgument::Literal(lit1), SWRLDArgument::Literal(lit2)) => {
                lit1.value == lit2.value && lit1.datatype == lit2.datatype
            }
            (SWRLDArgument::Variable(var), SWRLDArgument::Literal(lit)) => {
                self.bind_data_variable(var.iri.to_string(), SWRLDArgument::Literal(lit.clone()))
            }
            (SWRLDArgument::Literal(lit), SWRLDArgument::Variable(var)) => {
                self.bind_data_variable(var.iri.to_string(), SWRLDArgument::Literal(lit.clone()))
            }
            (SWRLDArgument::Variable(var1), SWRLDArgument::Variable(var2)) => {
                if var1.iri == var2.iri {
                    true
                } else {
                    // For now, assume different variables can unify
                    self.bind_data_variable(
                        var1.iri.to_string(),
                        SWRLDArgument::Variable(var2.clone()),
                    )
                }
            }
        }
    }

    /// Bind an individual variable to a value
    fn bind_individual_variable(&mut self, var_name: String, value: SWRLIArgument) -> bool {
        if let Some(existing) = self.substitutions.get(&var_name) {
            // Check consistency with existing binding
            match (existing, &value) {
                (SWRLIArgument::Individual(ind1), SWRLIArgument::Individual(ind2)) => {
                    ind1.iri().map(|i| i.as_str()) == ind2.iri().map(|i| i.as_str())
                }
                _ => false, // More complex consistency checking could be added
            }
        } else {
            self.substitutions.insert(var_name, value);
            true
        }
    }

    /// Bind a data variable to a value
    fn bind_data_variable(&mut self, var_name: String, value: SWRLDArgument) -> bool {
        if let Some(existing) = self.data_substitutions.get(&var_name) {
            // Check consistency with existing binding
            match (existing, &value) {
                (SWRLDArgument::Literal(lit1), SWRLDArgument::Literal(lit2)) => {
                    lit1.value == lit2.value && lit1.datatype == lit2.datatype
                }
                _ => false, // More complex consistency checking could be added
            }
        } else {
            self.data_substitutions.insert(var_name, value);
            true
        }
    }

    /// Check if two class expressions match
    fn class_expressions_match(&self, expr1: &ClassExpression, expr2: &ClassExpression) -> bool {
        match (expr1, expr2) {
            (ClassExpression::Class(c1), ClassExpression::Class(c2)) => {
                c1.iri.as_str() == c2.iri.as_str()
            }
            // Add more sophisticated matching for complex expressions
            _ => false,
        }
    }

    /// Check if two data ranges match
    fn data_ranges_match(
        &self,
        range1: &crate::ontology::DataRange,
        range2: &crate::ontology::DataRange,
    ) -> bool {
        use crate::ontology::DataRange;
        
        match (range1, range2) {
            // Same datatype
            (DataRange::Datatype(dt1), DataRange::Datatype(dt2)) => {
                dt1.as_str() == dt2.as_str()
            }
            // Datatype restrictions - check base datatype and facets
            (DataRange::DatatypeRestriction { datatype: dt1, restrictions: r1 },
             DataRange::DatatypeRestriction { datatype: dt2, restrictions: r2 }) => {
                dt1.as_str() == dt2.as_str() && r1 == r2
            }
            // Data intersections - check if sets of ranges are equal
            (DataRange::DataIntersectionOf(ranges1), DataRange::DataIntersectionOf(ranges2)) => {
                ranges1.len() == ranges2.len() &&
                ranges1.iter().zip(ranges2.iter()).all(|(r1, r2)| self.data_ranges_match(r1, r2))
            }
            // Data unions - check if sets of ranges are equal
            (DataRange::DataUnionOf(ranges1), DataRange::DataUnionOf(ranges2)) => {
                ranges1.len() == ranges2.len() &&
                ranges1.iter().zip(ranges2.iter()).all(|(r1, r2)| self.data_ranges_match(r1, r2))
            }
            // Data complements - check if complemented ranges match
            (DataRange::DataComplementOf(r1), DataRange::DataComplementOf(r2)) => {
                self.data_ranges_match(r1, r2)
            }
            // Data oneOf - check if literal lists are equal
            (DataRange::DataOneOf(literals1), DataRange::DataOneOf(literals2)) => {
                literals1 == literals2
            }
            // Different types don't match
            _ => false,
        }
    }

    /// Check if two object properties match
    fn object_properties_match(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                ObjectPropertyExpression::ObjectProperty(p1),
                ObjectPropertyExpression::ObjectProperty(p2),
            ) => p1.iri.as_str() == p2.iri.as_str(),
            // Add more sophisticated matching for property expressions
            _ => false,
        }
    }

    /// Check if two data properties match
    fn data_properties_match(
        &self,
        prop1: &crate::ontology::DataPropertyExpression,
        prop2: &crate::ontology::DataPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                crate::ontology::DataPropertyExpression::DataProperty(p1),
                crate::ontology::DataPropertyExpression::DataProperty(p2),
            ) => p1.iri.as_str() == p2.iri.as_str(),
        }
    }

    /// Reset substitutions
    pub fn reset(&mut self) {
        self.substitutions.clear();
        self.data_substitutions.clear();
    }

    /// Get current substitutions
    pub fn get_substitutions(&self) -> &HashMap<String, SWRLIArgument> {
        &self.substitutions
    }

    /// Get current data substitutions
    pub fn get_data_substitutions(&self) -> &HashMap<String, SWRLDArgument> {
        &self.data_substitutions
    }
}

impl Default for UnificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern matcher for SWRL rules and goals
#[derive(Debug)]
pub struct PatternMatcher {
    unification_engine: UnificationEngine,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self {
            unification_engine: UnificationEngine::new(),
        }
    }

    /// Check if a goal matches a rule head
    pub fn goal_matches_rule_head(
        &mut self,
        goal: &SWRLAtom,
        rule_head: &SWRLAtom,
    ) -> Result<bool> {
        self.unification_engine.reset();
        Ok(self.unification_engine.atoms_unify(goal, rule_head)?)
    }

    /// Find variable bindings that make atoms unify
    pub fn find_bindings(
        &mut self,
        atom1: &SWRLAtom,
        atom2: &SWRLAtom,
    ) -> Result<Option<VariableBindings>> {
        self.unification_engine.reset();

        if self.unification_engine.atoms_unify(atom1, atom2)? {
            Ok(Some(VariableBindings {
                individual_bindings: self.unification_engine.get_substitutions().clone(),
                data_bindings: self.unification_engine.get_data_substitutions().clone(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Check if two atoms are structurally similar (ignoring variable names)
    pub fn atoms_structurally_similar(&self, atom1: &SWRLAtom, atom2: &SWRLAtom) -> bool {
        std::mem::discriminant(atom1) == std::mem::discriminant(atom2)
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable bindings result from unification
#[derive(Debug, Clone)]
pub struct VariableBindings {
    /// Individual variable bindings
    pub individual_bindings: HashMap<String, SWRLIArgument>,
    /// Data variable bindings
    pub data_bindings: HashMap<String, SWRLDArgument>,
}

impl VariableBindings {
    /// Create empty variable bindings
    pub fn new() -> Self {
        Self {
            individual_bindings: HashMap::new(),
            data_bindings: HashMap::new(),
        }
    }

    /// Check if bindings are empty
    pub fn is_empty(&self) -> bool {
        self.individual_bindings.is_empty() && self.data_bindings.is_empty()
    }
}

impl Default for VariableBindings {
    fn default() -> Self {
        Self::new()
    }
}
