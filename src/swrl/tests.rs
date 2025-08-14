//! SWRL Tests
//!
//! Comprehensive tests for SWRL rule support in oxidowl.

use crate::swrl::*;
use crate::ontology::{axioms::*, *};
use crate::Error;
use crate::core::reasoner::TableauReasoner;
use std::collections::HashMap;

/// Test basic SWRL rule creation and manipulation
#[cfg(test)]
mod rule_tests {
    use super::*;

    #[test]
    fn test_rule_creation() {
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
        
        assert!(rule.is_safe().unwrap());
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
        
        assert!(!rule.is_safe().unwrap());
    }
}

/// Test SWRL built-ins
#[cfg(test)]
mod builtin_tests {
    use super::*;

    #[test]
    fn test_builtin_registry_creation() {
        let registry = BuiltInRegistry::new();
        assert!(registry.is_registered(&IRI::new("http://www.w3.org/2003/11/swrlb#equal")));
        assert!(registry.is_registered(&IRI::new("http://www.w3.org/2003/11/swrlb#add")));
        assert!(!registry.is_registered(&IRI::new("http://example.org/unknown")));
    }

    #[test]
    fn test_builtin_execution_equal() {
        let registry = BuiltInRegistry::new();
        let equal_iri = IRI::new("http://www.w3.org/2003/11/swrlb#equal");
        
        let args = vec![
            SWRLValue::Integer(42),
            SWRLValue::Integer(42),
        ];
        
        let result = registry.execute(&equal_iri, &args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        let args2 = vec![
            SWRLValue::Integer(42),
            SWRLValue::Integer(24),
        ];
        
        let result2 = registry.execute(&equal_iri, &args2).unwrap();
        assert_eq!(result2, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_builtin_execution_add() {
        let registry = BuiltInRegistry::new();
        let add_iri = IRI::new("http://www.w3.org/2003/11/swrlb#add");
        
        let args = vec![
            SWRLValue::Integer(10),
            SWRLValue::Integer(5),
        ];
        
        let result = registry.execute(&add_iri, &args).unwrap();
        assert_eq!(result, SWRLValue::Integer(15));
    }

    #[test]
    fn test_builtin_execution_string_concat() {
        let registry = BuiltInRegistry::new();
        let concat_iri = IRI::new("http://www.w3.org/2003/11/swrlb#stringConcat");
        
        let args = vec![
            SWRLValue::String("Hello".to_string()),
            SWRLValue::String(" World".to_string()),
        ];
        
        let result = registry.execute(&concat_iri, &args).unwrap();
        assert_eq!(result, SWRLValue::String("Hello World".to_string()));
    }

    #[test]
    fn test_builtin_type_mismatch() {
        let registry = BuiltInRegistry::new();
        let add_iri = IRI::new("http://www.w3.org/2003/11/swrlb#add");
        
        let args = vec![
            SWRLValue::String("not a number".to_string()),
            SWRLValue::Integer(5),
        ];
        
        let result = registry.execute(&add_iri, &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_builtin_registration() {
        let mut registry = BuiltInRegistry::new();
        let custom_iri = IRI::new("http://example.org/custom#double");
        
        let double_fn = |args: &[SWRLValue]| -> Result<SWRLValue, Error> {
            if args.len() != 1 {
                return Err(Error::reasoning("Double builtin expects exactly one argument"));
            }
            
            match &args[0] {
                SWRLValue::Integer(n) => Ok(SWRLValue::Integer(n * 2)),
                SWRLValue::Float(f) => Ok(SWRLValue::Float(f * 2.0)),
                _ => Err(Error::reasoning("Double builtin expects numeric argument")),
            }
        };
        
        registry.register_builtin(custom_iri.clone(), Box::new(double_fn));
        assert!(registry.is_registered(&custom_iri));
        
        let args = vec![SWRLValue::Integer(21)];
        let result = registry.execute(&custom_iri, &args).unwrap();
        assert_eq!(result, SWRLValue::Integer(42));
    }
}

/// Test SWRL rule engine
#[cfg(test)]
mod engine_tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let reasoner = TableauReasoner::new();
        let engine = SWRLRuleEngine::new(reasoner);
        
        assert_eq!(engine.get_rules().len(), 0);
    }

    #[test]
    fn test_add_rule() {
        let reasoner = TableauReasoner::new();
        let mut engine = SWRLRuleEngine::new(reasoner);
        
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
        engine.add_rule(rule).unwrap();
        
        assert_eq!(engine.get_rules().len(), 1);
    }

    #[test]
    fn test_reject_unsafe_rule() {
        let reasoner = TableauReasoner::new();
        let mut engine = SWRLRuleEngine::new(reasoner);
        
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
        
        let unsafe_rule = SWRLRule::new(vec![head_atom], vec![body_atom]);
        let result = engine.add_rule(unsafe_rule);
        
        assert!(result.is_err());
        assert_eq!(engine.get_rules().len(), 0);
    }

    #[test]
    fn test_simple_rule_execution() {
        let reasoner = TableauReasoner::new();
        let mut engine = SWRLRuleEngine::new(reasoner);
        
        // Create rule: Person(?x) -> Student(?x)
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
        engine.add_rule(rule).unwrap();
        
        // Execute rules (this would normally interact with the reasoner)
        let result = engine.execute_rules();
        assert!(result.is_ok());
    }
}

/// Test SWRL interpreter
#[cfg(test)]
mod interpreter_tests {
    use super::*;

    #[test]
    fn test_interpreter_creation() {
        let interpreter = SWRLInterpreter::new();
        // Basic creation test
        assert!(true); // Placeholder - interpreter doesn't expose internal state
    }

    #[test]
    fn test_variable_binding() {
        let mut interpreter = SWRLInterpreter::new();
        let var = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let value = SWRLValue::String("John".to_string());
        
        interpreter.bind_variable(var.clone(), value.clone());
        
        let retrieved = interpreter.get_binding(&var);
        assert_eq!(retrieved, Some(&value));
    }

    #[test]
    fn test_atom_evaluation_class() {
        let interpreter = SWRLInterpreter::new();
        
        let john = NamedIndividual::new(IRI::new("http://example.org/John"));
        let person_class = Class::new(IRI::new("http://example.org/Person"));
        
        let atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(person_class),
            argument: SWRLIArgument::Individual(john),
        };
        
        // This would normally check against the reasoner
        let result = interpreter.evaluate_atom(&atom);
        assert!(result.is_ok());
    }

    #[test]
    fn test_atom_evaluation_builtin() {
        let interpreter = SWRLInterpreter::new();
        
        let atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#equal"),
            arguments: vec![
                SWRLDArgument::Literal(Literal::new("42", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
                SWRLDArgument::Literal(Literal::new("42", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
            ],
        };
        
        let result = interpreter.evaluate_atom(&atom);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rule_interpretation() {
        let interpreter = SWRLInterpreter::new();
        
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
        
        let result = interpreter.interpret_rule(&rule);
        assert!(result.is_ok());
    }
}

/// Test SWRL validation
#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::swrl::validation::*;

    #[test]
    fn test_validator_creation() {
        let validator = SWRLValidator::new();
        assert!(!validator.is_strict_mode());
        
        let strict_validator = SWRLValidator::new_strict();
        assert!(strict_validator.is_strict_mode());
    }

    #[test]
    fn test_safe_rule_validation() {
        let validator = SWRLValidator::new();
        
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
        let result = validator.validate_rule(&rule).unwrap();
        
        assert!(result.is_valid);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_unsafe_rule_validation() {
        let validator = SWRLValidator::new();
        
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
        let result = validator.validate_rule(&rule).unwrap();
        
        assert!(!result.is_valid);
        assert!(result.has_errors());
    }

    #[test]
    fn test_empty_rule_validation() {
        let validator = SWRLValidator::new();
        let empty_rule = SWRLRule::new(Vec::new(), Vec::new());
        
        let result = validator.validate_rule(&empty_rule).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.issues.len(), 2); // Empty head and body
    }

    #[test]
    fn test_builtin_validation() {
        let validator = SWRLValidator::new();
        
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_y = SWRLVariable::new(IRI::new("http://example.org/var#y"));
        
        let class_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };
        
        let builtin_atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#equal"),
            arguments: vec![
                SWRLDArgument::Variable(var_x.clone()),
                SWRLDArgument::Variable(var_y.clone()),
            ],
        };
        
        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_y),
        };
        
        let rule = SWRLRule::new(vec![head_atom], vec![class_atom, builtin_atom]);
        let result = validator.validate_rule(&rule).unwrap();
        
        assert!(result.is_valid);
    }

    #[test]
    fn test_unknown_builtin_validation() {
        let validator = SWRLValidator::new();
        
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        
        let class_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };
        
        let unknown_builtin = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://example.org/unknown#builtin"),
            arguments: vec![SWRLDArgument::Variable(var_x.clone())],
        };
        
        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };
        
        let rule = SWRLRule::new(vec![head_atom], vec![class_atom, unknown_builtin]);
        let result = validator.validate_rule(&rule).unwrap();
        
        assert!(result.issues.iter().any(|issue| 
            matches!(issue, ValidationIssue::UnknownBuiltIn(_))
        ));
    }

    #[test]
    fn test_strict_mode_validation() {
        let mut validator = SWRLValidator::new();
        validator.set_strict_mode(true);
        
        let var_x = SWRLVariable::new(IRI::new("http://example.org/badname"));
        
        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };
        
        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };
        
        let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);
        let result = validator.validate_rule(&rule).unwrap();
        
        // Should have naming convention warning in strict mode
        assert!(result.issues.iter().any(|issue| 
            matches!(issue, ValidationIssue::NonStandardVariableName(_))
        ));
    }
}

/// Integration tests combining multiple SWRL components
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_swrl_workflow() {
        // Create a complete SWRL workflow: validation -> interpretation -> execution
        
        // 1. Create a rule
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_age = SWRLVariable::new(IRI::new("http://example.org/var#age"));
        
        let person_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };
        
        let age_atom = SWRLAtom::DataPropertyAtom {
            predicate: DataPropertyExpression::DataProperty(DataProperty::new(IRI::new("http://example.org/age"))),
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLDArgument::Variable(var_age.clone()),
        };
        
        let age_check = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#greaterThanOrEqual"),
            arguments: vec![
                SWRLDArgument::Variable(var_age),
                SWRLDArgument::Literal(Literal::new("18", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
            ],
        };
        
        let adult_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Adult"))),
            argument: SWRLIArgument::Variable(var_x),
        };
        
        let rule = SWRLRule::new(
            vec![adult_atom], 
            vec![person_atom, age_atom, age_check]
        );
        
        // 2. Validate the rule
        let validator = SWRLValidator::new();
        let validation_result = validator.validate_rule(&rule).unwrap();
        assert!(validation_result.is_valid);
        
        // 3. Add to engine
        let reasoner = TableauReasoner::new();
        let mut engine = SWRLRuleEngine::new(reasoner);
        engine.add_rule(rule).unwrap();
        
        // 4. Execute
        let execution_result = engine.execute_rules();
        assert!(execution_result.is_ok());
    }

    #[test]
    fn test_builtin_chaining() {
        // Test chaining multiple built-ins
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_y = SWRLVariable::new(IRI::new("http://example.org/var#y"));
        let var_z = SWRLVariable::new(IRI::new("http://example.org/var#z"));
        
        let person_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };
        
        let add_atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#add"),
            arguments: vec![
                SWRLDArgument::Literal(Literal::new("10", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
                SWRLDArgument::Literal(Literal::new("5", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
                SWRLDArgument::Variable(var_y.clone()),
            ],
        };
        
        let multiply_atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#multiply"),
            arguments: vec![
                SWRLDArgument::Variable(var_y),
                SWRLDArgument::Literal(Literal::new("2", Some(DataType::new(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))))),
                SWRLDArgument::Variable(var_z.clone()),
            ],
        };
        
        let result_atom = SWRLAtom::DataPropertyAtom {
            predicate: DataPropertyExpression::DataProperty(DataProperty::new(IRI::new("http://example.org/result"))),
            first_argument: SWRLIArgument::Variable(var_x),
            second_argument: SWRLDArgument::Variable(var_z),
        };
        
        let rule = SWRLRule::new(
            vec![result_atom],
            vec![person_atom, add_atom, multiply_atom]
        );
        
        // Validate the chained rule
        let validator = SWRLValidator::new();
        let validation_result = validator.validate_rule(&rule).unwrap();
        assert!(validation_result.is_valid);
    }

    #[test]
    fn test_error_handling() {
        // Test error handling in various scenarios
        
        // Invalid built-in execution
        let registry = BuiltInRegistry::new();
        let unknown_builtin = IRI::new("http://example.org/unknown");
        
        let result = registry.execute(&unknown_builtin, &[]);
        assert!(result.is_err());
        
        // Type mismatch in built-in
        let add_iri = IRI::new("http://www.w3.org/2003/11/swrlb#add");
        let bad_args = vec![
            SWRLValue::String("not a number".to_string()),
            SWRLValue::Integer(5),
        ];
        
        let result = registry.execute(&add_iri, &bad_args);
        assert!(result.is_err());
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_large_rule_set() {
        let reasoner = TableauReasoner::new();
        let mut engine = SWRLRuleEngine::new(reasoner);
        
        // Add many rules to test performance
        for i in 0..100 {
            let var_x = SWRLVariable::new(IRI::new(&format!("http://example.org/var#x{i}")));
            
            let body_atom = SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(&format!("http://example.org/Class{i}")))),
                argument: SWRLIArgument::Variable(var_x.clone()),
            };
            
            let head_atom = SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(&format!("http://example.org/Subclass{i}")))),
                argument: SWRLIArgument::Variable(var_x),
            };
            
            let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);
            engine.add_rule(rule).unwrap();
        }
        
        assert_eq!(engine.get_rules().len(), 100);
        
        // Execute all rules
        let result = engine.execute_rules();
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_rule_validation() {
        let validator = SWRLValidator::new();
        
        // Create a complex rule with many variables and atoms
        let vars: Vec<_> = (0..10)
            .map(|i| SWRLVariable::new(IRI::new(&format!("http://example.org/var#x{i}"))))
            .collect();
        
        let mut body_atoms = Vec::new();
        let mut head_atoms = Vec::new();
        
        // Create interconnected atoms
        for i in 0..5 {
            let body_atom = SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(&format!("http://example.org/Class{i}")))),
                argument: SWRLIArgument::Variable(vars[i].clone()),
            };
            body_atoms.push(body_atom);
            
            if i < 4 {
                let relation_atom = SWRLAtom::ObjectPropertyAtom {
                    predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty::new(IRI::new("http://example.org/relatedTo"))),
                    first_argument: SWRLIArgument::Variable(vars[i].clone()),
                    second_argument: SWRLIArgument::Variable(vars[i + 1].clone()),
                };
                body_atoms.push(relation_atom);
            }
            
            let head_atom = SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(&format!("http://example.org/Result{i}")))),
                argument: SWRLIArgument::Variable(vars[i].clone()),
            };
            head_atoms.push(head_atom);
        }
        
        let complex_rule = SWRLRule::new(head_atoms, body_atoms);
        
        let result = validator.validate_rule(&complex_rule).unwrap();
        assert!(result.is_valid);
    }
}
