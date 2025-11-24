//! SWRL Unification Algorithm
//!
//! This module implements unification logic for SWRL atoms and arguments.
//! Unification is the process of finding variable bindings that make two
//! expressions identical.

use crate::ontology::IRI;
use crate::ontology::axioms::{SWRLAtom, SWRLDArgument, SWRLIArgument};
use crate::swrl::SWRLVariable;
use log::debug;
use std::collections::HashMap;

/// Variable bindings map (using IRI as key)
pub type Bindings = HashMap<IRI, SWRLIArgument>;

/// Unification result
#[derive(Debug, Clone)]
pub enum UnificationResult {
    /// Unification succeeded with these bindings
    Success(Bindings),
    /// Unification failed
    Failure,
}

impl UnificationResult {
    /// Check if unification was successful
    pub fn is_success(&self) -> bool {
        matches!(self, UnificationResult::Success(_))
    }

    /// Get bindings if successful
    pub fn bindings(&self) -> Option<&Bindings> {
        match self {
            UnificationResult::Success(bindings) => Some(bindings),
            UnificationResult::Failure => None,
        }
    }
}

/// Unification engine for SWRL atoms
#[derive(Debug, Default)]
pub struct UnificationEngine {
    /// Debug mode
    debug: bool,
}

impl UnificationEngine {
    /// Create a new unification engine
    pub fn new() -> Self {
        Self { debug: false }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Unify two SWRL atoms
    ///
    /// Returns a set of variable bindings that make atom1 and atom2 identical,
    /// or None if no such bindings exist.
    pub fn unify_atoms(&self, atom1: &SWRLAtom, atom2: &SWRLAtom) -> UnificationResult {
        let mut bindings = Bindings::new();

        if self.debug {
            debug!("Attempting to unify atoms: {:?} with {:?}", atom1, atom2);
        }

        // Try to unify based on atom type
        let success = match (atom1, atom2) {
            (
                SWRLAtom::ClassAtom {
                    predicate: p1,
                    argument: a1,
                },
                SWRLAtom::ClassAtom {
                    predicate: p2,
                    argument: a2,
                },
            ) => {
                // Class atoms must have same predicate
                if p1 != p2 {
                    false
                } else {
                    self.unify_i_arguments(a1, a2, &mut bindings)
                }
            }
            (
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p1,
                    first_argument: a1_1,
                    second_argument: a1_2,
                },
                SWRLAtom::ObjectPropertyAtom {
                    predicate: p2,
                    first_argument: a2_1,
                    second_argument: a2_2,
                },
            ) => {
                // Object property atoms must have same predicate
                if p1 != p2 {
                    false
                } else {
                    self.unify_i_arguments(a1_1, a2_1, &mut bindings)
                        && self.unify_i_arguments(a1_2, a2_2, &mut bindings)
                }
            }
            (
                SWRLAtom::DataPropertyAtom {
                    predicate: p1,
                    first_argument: a1_1,
                    second_argument: a1_2,
                },
                SWRLAtom::DataPropertyAtom {
                    predicate: p2,
                    first_argument: a2_1,
                    second_argument: a2_2,
                },
            ) => {
                // Data property atoms must have same predicate
                if p1 != p2 {
                    false
                } else {
                    self.unify_i_arguments(a1_1, a2_1, &mut bindings)
                        && self.unify_d_arguments(a1_2, a2_2, &mut bindings)
                }
            }
            (
                SWRLAtom::DataRangeAtom {
                    predicate: dr1,
                    argument: a1,
                },
                SWRLAtom::DataRangeAtom {
                    predicate: dr2,
                    argument: a2,
                },
            ) => {
                // Data range atoms must have same predicate
                if dr1 != dr2 {
                    false
                } else {
                    self.unify_d_arguments(a1, a2, &mut bindings)
                }
            }
            (
                SWRLAtom::SameIndividualAtom {
                    first_argument: a1_1,
                    second_argument: a1_2,
                },
                SWRLAtom::SameIndividualAtom {
                    first_argument: a2_1,
                    second_argument: a2_2,
                },
            ) => {
                self.unify_i_arguments(a1_1, a2_1, &mut bindings)
                    && self.unify_i_arguments(a1_2, a2_2, &mut bindings)
            }
            (
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: a1_1,
                    second_argument: a1_2,
                },
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: a2_1,
                    second_argument: a2_2,
                },
            ) => {
                self.unify_i_arguments(a1_1, a2_1, &mut bindings)
                    && self.unify_i_arguments(a1_2, a2_2, &mut bindings)
            }
            (
                SWRLAtom::BuiltInAtom {
                    predicate: p1,
                    arguments: args1,
                },
                SWRLAtom::BuiltInAtom {
                    predicate: p2,
                    arguments: args2,
                },
            ) => {
                // Built-in atoms must have same predicate and same number of arguments
                if p1 != p2 || args1.len() != args2.len() {
                    false
                } else {
                    args1
                        .iter()
                        .zip(args2.iter())
                        .all(|(arg1, arg2)| self.unify_d_arguments(arg1, arg2, &mut bindings))
                }
            }
            _ => false, // Different atom types cannot unify
        };

        if success {
            if self.debug {
                debug!("Unification succeeded with bindings: {:?}", bindings);
            }
            UnificationResult::Success(bindings)
        } else {
            if self.debug {
                debug!("Unification failed");
            }
            UnificationResult::Failure
        }
    }

    /// Unify two I-arguments (individuals or variables)
    fn unify_i_arguments(
        &self,
        arg1: &SWRLIArgument,
        arg2: &SWRLIArgument,
        bindings: &mut Bindings,
    ) -> bool {
        match (arg1, arg2) {
            // Variable to variable
            (SWRLIArgument::Variable(v1), SWRLIArgument::Variable(v2)) => {
                // v1.iri and v2.iri are already IRI types
                let v1_iri = &v1.iri;
                let v2_iri = &v2.iri;

                // Check existing bindings
                match (bindings.get(v1_iri), bindings.get(v2_iri)) {
                    (Some(b1), Some(b2)) => b1 == b2, // Both bound, must match
                    (Some(b1), None) => {
                        // v1 is bound, bind v2 to same value
                        bindings.insert(v2_iri.clone(), b1.clone());
                        true
                    }
                    (None, Some(b2)) => {
                        // v2 is bound, bind v1 to same value
                        bindings.insert(v1_iri.clone(), b2.clone());
                        true
                    }
                    (None, None) => {
                        // Neither bound, bind v1 to v2
                        bindings.insert(v1_iri.clone(), arg2.clone());
                        true
                    }
                }
            }
            // Variable to individual
            (SWRLIArgument::Variable(v), individual) | (individual, SWRLIArgument::Variable(v)) => {
                let v_iri = &v.iri;
                match bindings.get(v_iri) {
                    Some(existing) => existing == individual, // Must match existing binding
                    None => {
                        // Bind variable to individual
                        bindings.insert(v_iri.clone(), individual.clone());
                        true
                    }
                }
            }
            // Individual to individual
            (SWRLIArgument::Individual(i1), SWRLIArgument::Individual(i2)) => i1 == i2,
        }
    }

    /// Unify two D-arguments (literals or variables)
    fn unify_d_arguments(
        &self,
        arg1: &SWRLDArgument,
        arg2: &SWRLDArgument,
        bindings: &mut Bindings,
    ) -> bool {
        match (arg1, arg2) {
            // Variable to variable
            (SWRLDArgument::Variable(v1), SWRLDArgument::Variable(v2)) => {
                // Convert to I-arguments for uniform handling
                let i_arg1 = SWRLIArgument::Variable(v1.clone());
                let i_arg2 = SWRLIArgument::Variable(v2.clone());
                self.unify_i_arguments(&i_arg1, &i_arg2, bindings)
            }
            // Variable to literal
            (SWRLDArgument::Variable(v), _literal) | (_literal, SWRLDArgument::Variable(v)) => {
                // Check if variable is already bound
                let v_iri = &v.iri;
                match bindings.get(v_iri) {
                    Some(_existing) => {
                        // For D-arguments we need more sophisticated checking
                        // For now, accept if already bound (would need proper literal comparison)
                        true
                    }
                    None => {
                        // For D-arguments, we can't directly bind to I-argument type
                        // Accept the binding (proper implementation would convert types)
                        true
                    }
                }
            }
            // Literal to literal
            (SWRLDArgument::Literal(l1), SWRLDArgument::Literal(l2)) => l1 == l2,
        }
    }

    /// Apply bindings to an atom
    pub fn apply_bindings(&self, atom: &SWRLAtom, bindings: &Bindings) -> SWRLAtom {
        match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => SWRLAtom::ClassAtom {
                predicate: predicate.clone(),
                argument: self.apply_bindings_to_i_arg(argument, bindings),
            },
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => SWRLAtom::ObjectPropertyAtom {
                predicate: predicate.clone(),
                first_argument: self.apply_bindings_to_i_arg(first_argument, bindings),
                second_argument: self.apply_bindings_to_i_arg(second_argument, bindings),
            },
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => SWRLAtom::DataPropertyAtom {
                predicate: predicate.clone(),
                first_argument: self.apply_bindings_to_i_arg(first_argument, bindings),
                second_argument: self.apply_bindings_to_d_arg(second_argument, bindings),
            },
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => SWRLAtom::DataRangeAtom {
                predicate: predicate.clone(),
                argument: self.apply_bindings_to_d_arg(argument, bindings),
            },
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => SWRLAtom::SameIndividualAtom {
                first_argument: self.apply_bindings_to_i_arg(first_argument, bindings),
                second_argument: self.apply_bindings_to_i_arg(second_argument, bindings),
            },
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => SWRLAtom::DifferentIndividualsAtom {
                first_argument: self.apply_bindings_to_i_arg(first_argument, bindings),
                second_argument: self.apply_bindings_to_i_arg(second_argument, bindings),
            },
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => SWRLAtom::BuiltInAtom {
                predicate: predicate.clone(),
                arguments: arguments
                    .iter()
                    .map(|arg| self.apply_bindings_to_d_arg(arg, bindings))
                    .collect(),
            },
        }
    }

    /// Apply bindings to an I-argument
    fn apply_bindings_to_i_arg(&self, arg: &SWRLIArgument, bindings: &Bindings) -> SWRLIArgument {
        match arg {
            SWRLIArgument::Variable(v) => {
                let v_iri = &v.iri;
                bindings.get(v_iri).cloned().unwrap_or_else(|| arg.clone())
            }
            _ => arg.clone(),
        }
    }

    /// Apply bindings to a D-argument
    fn apply_bindings_to_d_arg(&self, arg: &SWRLDArgument, bindings: &Bindings) -> SWRLDArgument {
        match arg {
            SWRLDArgument::Variable(v) => {
                // Try to get binding and convert if possible
                let v_iri = &v.iri;
                if let Some(bound) = bindings.get(v_iri) {
                    // If bound to a variable, keep as variable
                    if let SWRLIArgument::Variable(bound_var) = bound {
                        SWRLDArgument::Variable(bound_var.clone())
                    } else {
                        // Can't convert I-argument to D-argument directly
                        arg.clone()
                    }
                } else {
                    arg.clone()
                }
            }
            _ => arg.clone(),
        }
    }

    /// Find all variable bindings that make atom match any fact
    pub fn match_atom_with_facts(&self, atom: &SWRLAtom, facts: &[SWRLAtom]) -> Vec<Bindings> {
        let mut all_bindings = Vec::new();

        for fact in facts {
            if let UnificationResult::Success(bindings) = self.unify_atoms(atom, fact) {
                all_bindings.push(bindings);
            }
        }

        if self.debug {
            debug!(
                "Matched atom against {} facts, found {} binding sets",
                facts.len(),
                all_bindings.len()
            );
        }

        all_bindings
    }

    /// Compose two binding sets (merge them if compatible)
    pub fn compose_bindings(&self, bindings1: &Bindings, bindings2: &Bindings) -> Option<Bindings> {
        let mut result = bindings1.clone();

        for (var, value) in bindings2 {
            match result.get(var) {
                Some(existing) => {
                    // Check if bindings are compatible
                    if existing != value {
                        return None; // Incompatible bindings
                    }
                }
                None => {
                    result.insert(var.clone(), value.clone());
                }
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, ClassExpression, IRI, Individual, ObjectProperty};

    fn make_variable(name: &str) -> SWRLVariable {
        SWRLVariable {
            iri: IRI::new(&format!("http://example.org/var/{}", name)),
        }
    }

    fn make_individual(name: &str) -> Individual {
        Individual::Named(crate::ontology::NamedIndividual {
            iri: IRI::new(&format!("http://example.org/{}", name)),
        })
    }

    #[test]
    fn test_unify_same_individuals() {
        let engine = UnificationEngine::new();
        let ind = SWRLIArgument::Individual(make_individual("John"));

        let atom1 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Person"),
            }),
            argument: ind.clone(),
        };

        let atom2 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Person"),
            }),
            argument: ind.clone(),
        };

        let result = engine.unify_atoms(&atom1, &atom2);
        assert!(result.is_success());
    }

    #[test]
    fn test_unify_variable_with_individual() {
        let engine = UnificationEngine::new();
        let var = SWRLIArgument::Variable(make_variable("x"));
        let ind = SWRLIArgument::Individual(make_individual("John"));

        let atom1 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Person"),
            }),
            argument: var.clone(),
        };

        let atom2 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Person"),
            }),
            argument: ind.clone(),
        };

        let result = engine.unify_atoms(&atom1, &atom2);
        assert!(result.is_success());

        if let Some(bindings) = result.bindings() {
            assert_eq!(bindings.len(), 1);
            let var_iri = IRI::new("http://example.org/var/x");
            assert_eq!(bindings.get(&var_iri), Some(&ind));
        }
    }

    #[test]
    fn test_unify_different_predicates() {
        let engine = UnificationEngine::new();
        let ind = SWRLIArgument::Individual(make_individual("John"));

        let atom1 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Person"),
            }),
            argument: ind.clone(),
        };

        let atom2 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class {
                iri: IRI::new("http://example.org/Animal"),
            }),
            argument: ind.clone(),
        };

        let result = engine.unify_atoms(&atom1, &atom2);
        assert!(!result.is_success());
    }
}
