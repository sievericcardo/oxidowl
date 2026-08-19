//! Simple SWRL Core Tests
//!
//! Basic tests for SWRL rule support in oxidowl.

use oxidowl::ontology::*;
use oxidowl::swrl::*;

/// Test basic SWRL rule creation
#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_swrl_variable_creation() {
        let var = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        assert_eq!(var.iri.to_string(), "http://example.org/var#x");
    }

    #[test]
    fn test_swrl_rule_empty() {
        let rule = SWRLRule::new(Vec::new(), Vec::new());
        assert_eq!(rule.head.len(), 0);
        assert_eq!(rule.body.len(), 0);
    }

    #[test]
    fn test_simple_class_atom() {
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        let atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        match &atom {
            SWRLAtom::ClassAtom { predicate, argument } => {
                assert_eq!(
                    predicate,
                    &ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")))
                );
                assert_eq!(
                    argument,
                    &SWRLIArgument::Variable(SWRLVariable::new(IRI::new(
                        "http://example.org/var#x"
                    )))
                );
            }
            _ => panic!("Expected ClassAtom, got: {atom:?}"),
        }
    }

    #[test]
    fn test_simple_rule_creation() {
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);

        assert_eq!(rule.head.len(), 1);
        assert_eq!(rule.body.len(), 1);
    }

    #[test]
    fn test_rule_variables() {
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_y = SWRLVariable::new(IRI::new("http://example.org/var#y"));

        let body_atom1 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let body_atom2 = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Adult"))),
            argument: SWRLIArgument::Variable(var_y.clone()),
        };

        let head_atom = SWRLAtom::SameIndividualAtom {
            first_argument: SWRLIArgument::Variable(var_x),
            second_argument: SWRLIArgument::Variable(var_y),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![body_atom1, body_atom2]);
        let variables = rule.variables();

        assert_eq!(variables.len(), 2);
    }

    #[test]
    fn test_rule_safety() {
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);

        // Test that is_safe returns a boolean
        let is_safe = rule.is_safe();
        assert!(is_safe);
    }

    #[test]
    fn test_rule_unsafe() {
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_y = SWRLVariable::new(IRI::new("http://example.org/var#y"));

        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_y),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);

        assert!(!rule.is_safe());
    }
}

/// Test SWRL built-ins if available
#[cfg(test)]
mod builtin_tests {
    use super::*;

    #[test]
    fn test_builtin_registry_basic() {
        let registry = BuiltInRegistry::new();

        // Test standard built-ins
        assert!(registry.is_registered(&IRI::new("http://www.w3.org/2003/11/swrlb#equal")));
        assert!(!registry.is_registered(&IRI::new("http://example.org/unknown")));
    }
}
