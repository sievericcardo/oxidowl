//! SWRL Rule Validation
//!
//! This module provides validation functionality for SWRL rules,
//! ensuring they are well-formed, safe, and semantically correct.

use crate::ontology::*;
use crate::{Error, Result};
use log::debug;
use std::collections::{HashMap, HashSet};

/// SWRL Rule Validator
///
/// Validates SWRL rules for correctness, safety, and semantic constraints.
#[derive(Debug)]
pub struct SWRLValidator {
    /// Enable strict validation (more restrictive)
    strict_mode: bool,
}

impl Default for SWRLValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SWRLValidator {
    /// Create a new validator
    #[must_use]
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    /// Create a validator with strict mode enabled
    #[must_use]
    pub fn new_strict() -> Self {
        Self { strict_mode: true }
    }

    /// Validate a SWRL rule
    pub fn validate_rule(&self, rule: &SWRLRule) -> Result<ValidationResult> {
        debug!("Validating SWRL rule: {:?}", rule);

        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check basic structure
        if rule.head.is_empty() {
            issues.push(ValidationIssue::EmptyHead);
        }

        if rule.body.is_empty() {
            issues.push(ValidationIssue::EmptyBody);
        }

        // Check safety constraints
        if let Err(safety_issue) = self.check_safety(rule) {
            match safety_issue {
                Error::Reasoning { message } => {
                    issues.push(ValidationIssue::SafetyViolation(message));
                }
                _ => issues.push(ValidationIssue::SafetyViolation(
                    "Unknown safety violation".to_string(),
                )),
            }
        }

        // Check variable usage
        let variable_issues = self.check_variable_usage(rule);
        issues.extend(variable_issues);

        // Check atom structure
        let atom_issues = self.check_atoms(rule);
        issues.extend(atom_issues);

        // Check for potential infinite recursion
        if self.check_recursion_risk(rule) {
            warnings.push(ValidationWarning::RecursionRisk);
        }

        // Check built-in usage
        let builtin_issues = self.check_builtin_usage(rule);
        issues.extend(builtin_issues);

        // Determine overall validity
        let is_valid =
            issues.is_empty() || (!self.strict_mode && issues.iter().all(|i| i.is_warning_level()));

        Ok(ValidationResult {
            is_valid,
            issues,
            warnings,
        })
    }

    /// Check if the rule satisfies safety constraints
    fn check_safety(&self, rule: &SWRLRule) -> Result<()> {
        // A rule is safe if every variable in the head appears in the body
        let head_vars: HashSet<&SWRLVariable> =
            rule.head.iter().flat_map(|atom| atom.variables()).collect();

        let body_vars: HashSet<&SWRLVariable> =
            rule.body.iter().flat_map(|atom| atom.variables()).collect();

        // Check if all head variables appear in body
        for head_var in &head_vars {
            if !body_vars.contains(head_var) {
                return Err(Error::reasoning(format!(
                    "Variable {} appears in head but not in body (safety violation)",
                    head_var.iri
                )));
            }
        }

        // Additional safety checks for built-ins
        for atom in &rule.body {
            if let SWRLAtom::BuiltInAtom { arguments, .. } = atom {
                self.check_builtin_safety(arguments, &body_vars)?;
            }
        }

        Ok(())
    }

    /// Check built-in atom safety
    fn check_builtin_safety(
        &self,
        arguments: &[SWRLDArgument],
        body_vars: &HashSet<&SWRLVariable>,
    ) -> Result<()> {
        // For built-ins, typically all variables should be bound
        // (i.e., appear in non-built-in atoms in the body)
        let builtin_vars: HashSet<&SWRLVariable> = arguments
            .iter()
            .filter_map(|arg| match arg {
                SWRLDArgument::Variable(var) => Some(var),
                SWRLDArgument::Literal(_) => None,
            })
            .collect();

        for var in builtin_vars {
            if !body_vars.contains(&var) {
                return Err(Error::reasoning(format!(
                    "Built-in variable {} not bound in rule body",
                    var.iri
                )));
            }
        }

        Ok(())
    }

    /// Check variable usage patterns
    fn check_variable_usage(&self, rule: &SWRLRule) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut variable_usage = HashMap::new();

        // Count variable occurrences
        for atom in &rule.head {
            for var in atom.variables() {
                *variable_usage.entry(var).or_insert(0) += 1;
            }
        }

        for atom in &rule.body {
            for var in atom.variables() {
                *variable_usage.entry(var).or_insert(0) += 1;
            }
        }

        // Check for single-use variables (potential typos)
        for (var, count) in &variable_usage {
            if *count == 1 {
                issues.push(ValidationIssue::SingleUseVariable(var.iri.to_string()));
            }
        }

        issues
    }

    /// Check atom structure and validity
    fn check_atoms(&self, rule: &SWRLRule) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check head atoms
        for atom in &rule.head {
            issues.extend(self.validate_atom(atom, true));
        }

        // Check body atoms
        for atom in &rule.body {
            issues.extend(self.validate_atom(atom, false));
        }

        issues
    }

    /// Validate an individual atom
    fn validate_atom(&self, atom: &SWRLAtom, is_head: bool) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => {
                if let ClassExpression::Class(_) = predicate {
                    // Valid simple class atom
                } else if self.strict_mode {
                    issues.push(ValidationIssue::ComplexClassExpression);
                }

                if let SWRLIArgument::Variable(var) = argument {
                    self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                }
            }

            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                if let ObjectPropertyExpression::PropertyChain(_) = predicate {
                    if self.strict_mode {
                        issues.push(ValidationIssue::ComplexPropertyExpression);
                    }
                }

                if let SWRLIArgument::Variable(var) = first_argument {
                    self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                }
                if let SWRLIArgument::Variable(var) = second_argument {
                    self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                }
            }

            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                // DataPropertyExpression pattern matching will always match DataProperty
                let _prop = match predicate {
                    DataPropertyExpression::DataProperty(prop) => prop,
                };

                if let SWRLIArgument::Variable(var) = first_argument {
                    self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                }
                if let SWRLDArgument::Variable(var) = second_argument {
                    self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                }
            }

            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => {
                // Built-ins should typically not appear in the head
                if is_head && self.strict_mode {
                    issues.push(ValidationIssue::BuiltInInHead);
                }

                // Check built-in IRI format
                if !self.is_valid_builtin_iri(predicate) {
                    issues.push(ValidationIssue::InvalidBuiltInIRI(predicate.to_string()));
                }

                // Check argument types
                for arg in arguments {
                    if let SWRLDArgument::Variable(var) = arg {
                        self.validate_variable_naming(&var.iri.to_string(), &mut issues);
                    }
                }
            }

            _ => {
                // Other atom types are generally valid
            }
        }

        issues
    }

    /// Check for recursion risk
    fn check_recursion_risk(&self, rule: &SWRLRule) -> bool {
        // Simple check: see if head predicates appear in body
        let head_predicates = self.extract_predicates_from_head(rule);
        let body_predicates = self.extract_predicates_from_body(rule);

        head_predicates
            .iter()
            .any(|pred| body_predicates.contains(pred))
    }

    /// Check built-in usage
    fn check_builtin_usage(&self, rule: &SWRLRule) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for atom in &rule.body {
            if let SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } = atom
            {
                // Check if built-in is known/standard
                if !self.is_standard_builtin(predicate) {
                    issues.push(ValidationIssue::UnknownBuiltIn(predicate.to_string()));
                }

                // Check arity
                if let Some(expected_arity) = self.get_expected_builtin_arity(predicate) {
                    if arguments.len() != expected_arity {
                        issues.push(ValidationIssue::IncorrectBuiltInArity {
                            builtin: predicate.to_string(),
                            expected: expected_arity,
                            actual: arguments.len(),
                        });
                    }
                }
            }
        }

        issues
    }

    /// Validate variable naming conventions
    fn validate_variable_naming(&self, var_name: &str, issues: &mut Vec<ValidationIssue>) {
        if self.strict_mode {
            // Check if variable follows naming conventions
            if !var_name.starts_with('?') && !var_name.contains("#var") {
                issues.push(ValidationIssue::NonStandardVariableName(
                    var_name.to_string(),
                ));
            }

            // Check for reserved names
            if var_name.to_lowercase().contains("owl") || var_name.to_lowercase().contains("rdf") {
                issues.push(ValidationIssue::ReservedVariableName(var_name.to_string()));
            }
        }
    }

    /// Check if an IRI is a valid built-in IRI
    fn is_valid_builtin_iri(&self, iri: &IRI) -> bool {
        let iri_str = iri.as_str();
        iri_str.starts_with("http://www.w3.org/2003/11/swrlb#")
            || iri_str.starts_with("http://www.w3.org/2003/11/swrlx#")
            || iri_str.starts_with("urn:swrl#")
    }

    /// Check if a built-in is standard
    fn is_standard_builtin(&self, iri: &IRI) -> bool {
        let standard_builtins = [
            // Comparison built-ins
            "http://www.w3.org/2003/11/swrlb#equal",
            "http://www.w3.org/2003/11/swrlb#notEqual",
            "http://www.w3.org/2003/11/swrlb#lessThan",
            "http://www.w3.org/2003/11/swrlb#lessThanOrEqual",
            "http://www.w3.org/2003/11/swrlb#greaterThan",
            "http://www.w3.org/2003/11/swrlb#greaterThanOrEqual",
            // Math built-ins
            "http://www.w3.org/2003/11/swrlb#add",
            "http://www.w3.org/2003/11/swrlb#subtract",
            "http://www.w3.org/2003/11/swrlb#multiply",
            "http://www.w3.org/2003/11/swrlb#divide",
            "http://www.w3.org/2003/11/swrlb#integerDivide",
            "http://www.w3.org/2003/11/swrlb#mod",
            "http://www.w3.org/2003/11/swrlb#pow",
            "http://www.w3.org/2003/11/swrlb#unaryPlus",
            "http://www.w3.org/2003/11/swrlb#unaryMinus",
            "http://www.w3.org/2003/11/swrlb#abs",
            "http://www.w3.org/2003/11/swrlb#ceiling",
            "http://www.w3.org/2003/11/swrlb#floor",
            "http://www.w3.org/2003/11/swrlb#round",
            "http://www.w3.org/2003/11/swrlb#roundHalfToEven",
            "http://www.w3.org/2003/11/swrlb#sin",
            "http://www.w3.org/2003/11/swrlb#cos",
            "http://www.w3.org/2003/11/swrlb#tan",
            // Boolean built-ins
            "http://www.w3.org/2003/11/swrlb#booleanNot",
            // String built-ins
            "http://www.w3.org/2003/11/swrlb#stringLength",
            "http://www.w3.org/2003/11/swrlb#stringConcat",
            "http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase",
            "http://www.w3.org/2003/11/swrlb#substring",
            "http://www.w3.org/2003/11/swrlb#normalizeSpace",
            "http://www.w3.org/2003/11/swrlb#upperCase",
            "http://www.w3.org/2003/11/swrlb#lowerCase",
            "http://www.w3.org/2003/11/swrlb#translate",
            "http://www.w3.org/2003/11/swrlb#contains",
            "http://www.w3.org/2003/11/swrlb#containsIgnoreCase",
            "http://www.w3.org/2003/11/swrlb#startsWith",
            "http://www.w3.org/2003/11/swrlb#endsWith",
            "http://www.w3.org/2003/11/swrlb#substringBefore",
            "http://www.w3.org/2003/11/swrlb#substringAfter",
            "http://www.w3.org/2003/11/swrlb#matches",
            "http://www.w3.org/2003/11/swrlb#replace",
            "http://www.w3.org/2003/11/swrlb#tokenize",
            // Date/Time/Duration built-ins
            "http://www.w3.org/2003/11/swrlb#yearMonthDuration",
            "http://www.w3.org/2003/11/swrlb#dayTimeDuration",
            "http://www.w3.org/2003/11/swrlb#dateTime",
            "http://www.w3.org/2003/11/swrlb#date",
            "http://www.w3.org/2003/11/swrlb#time",
            // URI built-ins
            "http://www.w3.org/2003/11/swrlb#resolveURI",
            "http://www.w3.org/2003/11/swrlb#anyURI",
            // List built-ins
            "http://www.w3.org/2003/11/swrlb#listConcat",
            "http://www.w3.org/2003/11/swrlb#listIntersection",
            "http://www.w3.org/2003/11/swrlb#listSubtraction",
            "http://www.w3.org/2003/11/swrlb#member",
            "http://www.w3.org/2003/11/swrlb#length",
            "http://www.w3.org/2003/11/swrlb#first",
            "http://www.w3.org/2003/11/swrlb#rest",
            "http://www.w3.org/2003/11/swrlb#sublist",
            "http://www.w3.org/2003/11/swrlb#empty",
        ];

        standard_builtins.contains(&iri.as_str())
    }

    /// Get expected arity for a built-in
    fn get_expected_builtin_arity(&self, iri: &IRI) -> Option<usize> {
        match iri.as_str() {
            "http://www.w3.org/2003/11/swrlb#equal" => Some(2),
            "http://www.w3.org/2003/11/swrlb#notEqual" => Some(2),
            "http://www.w3.org/2003/11/swrlb#lessThan" => Some(2),
            "http://www.w3.org/2003/11/swrlb#lessThanOrEqual" => Some(2),
            "http://www.w3.org/2003/11/swrlb#greaterThan" => Some(2),
            "http://www.w3.org/2003/11/swrlb#greaterThanOrEqual" => Some(2),
            "http://www.w3.org/2003/11/swrlb#add" => Some(3),
            "http://www.w3.org/2003/11/swrlb#subtract" => Some(3),
            "http://www.w3.org/2003/11/swrlb#multiply" => Some(3),
            "http://www.w3.org/2003/11/swrlb#divide" => Some(3),
            "http://www.w3.org/2003/11/swrlb#mod" => Some(3),
            "http://www.w3.org/2003/11/swrlb#pow" => Some(3),
            "http://www.w3.org/2003/11/swrlb#abs" => Some(2),
            "http://www.w3.org/2003/11/swrlb#stringLength" => Some(2),
            "http://www.w3.org/2003/11/swrlb#stringConcat" => Some(3),
            "http://www.w3.org/2003/11/swrlb#contains" => Some(2),
            "http://www.w3.org/2003/11/swrlb#startsWith" => Some(2),
            "http://www.w3.org/2003/11/swrlb#endsWith" => Some(2),
            "http://www.w3.org/2003/11/swrlb#substring" => Some(4),
            "http://www.w3.org/2003/11/swrlb#upperCase" => Some(2),
            "http://www.w3.org/2003/11/swrlb#lowerCase" => Some(2),
            _ => None,
        }
    }

    /// Extract predicates from rule head
    fn extract_predicates_from_head(&self, rule: &SWRLRule) -> HashSet<String> {
        let mut predicates = HashSet::new();

        for atom in &rule.head {
            match atom {
                SWRLAtom::ClassAtom { predicate, .. } => {
                    if let ClassExpression::Class(class) = predicate {
                        predicates.insert(class.iri.to_string());
                    }
                }
                SWRLAtom::ObjectPropertyAtom { predicate, .. } => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = predicate {
                        predicates.insert(prop.iri.to_string());
                    }
                }
                SWRLAtom::DataPropertyAtom { predicate, .. } => {
                    // DataPropertyExpression always matches DataProperty in this context
                    let prop = match predicate {
                        DataPropertyExpression::DataProperty(prop) => prop,
                    };
                    predicates.insert(prop.iri.to_string());
                }
                _ => {}
            }
        }

        predicates
    }

    /// Extract predicates from rule body
    fn extract_predicates_from_body(&self, rule: &SWRLRule) -> HashSet<String> {
        let mut predicates = HashSet::new();

        for atom in &rule.body {
            match atom {
                SWRLAtom::ClassAtom { predicate, .. } => {
                    if let ClassExpression::Class(class) = predicate {
                        predicates.insert(class.iri.to_string());
                    }
                }
                SWRLAtom::ObjectPropertyAtom { predicate, .. } => {
                    if let ObjectPropertyExpression::ObjectProperty(prop) = predicate {
                        predicates.insert(prop.iri.to_string());
                    }
                }
                SWRLAtom::DataPropertyAtom { predicate, .. } => {
                    // DataPropertyExpression always matches DataProperty in this context
                    let prop = match predicate {
                        DataPropertyExpression::DataProperty(prop) => prop,
                    };
                    predicates.insert(prop.iri.to_string());
                }
                _ => {}
            }
        }

        predicates
    }

    /// Enable or disable strict validation mode
    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    /// Check if strict mode is enabled
    #[must_use]
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }
}

/// Validation result for a SWRL rule
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the rule is valid
    pub is_valid: bool,
    /// List of validation issues found
    pub issues: Vec<ValidationIssue>,
    /// List of warnings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Check if there are any errors (not just warnings)
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|issue| !issue.is_warning_level())
    }

    /// Get all error-level issues
    #[must_use]
    pub fn get_errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| !issue.is_warning_level())
            .collect()
    }

    /// Get all warning-level issues
    #[must_use]
    pub fn get_warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.is_warning_level())
            .collect()
    }
}

/// Validation issue types
#[derive(Debug, Clone)]
pub enum ValidationIssue {
    /// Rule has empty head
    EmptyHead,
    /// Rule has empty body
    EmptyBody,
    /// Safety violation
    SafetyViolation(String),
    /// Variable used only once (potential typo)
    SingleUseVariable(String),
    /// Complex class expression in atom
    ComplexClassExpression,
    /// Complex property expression in atom
    ComplexPropertyExpression,
    /// Built-in atom in rule head
    BuiltInInHead,
    /// Invalid built-in IRI format
    InvalidBuiltInIRI(String),
    /// Unknown built-in predicate
    UnknownBuiltIn(String),
    /// Incorrect built-in arity
    IncorrectBuiltInArity {
        builtin: String,
        expected: usize,
        actual: usize,
    },
    /// Non-standard variable naming
    NonStandardVariableName(String),
    /// Reserved variable name used
    ReservedVariableName(String),
}

impl ValidationIssue {
    /// Check if this issue is only a warning
    #[must_use]
    pub fn is_warning_level(&self) -> bool {
        matches!(
            self,
            ValidationIssue::SingleUseVariable(_)
                | ValidationIssue::NonStandardVariableName(_)
                | ValidationIssue::ComplexClassExpression
                | ValidationIssue::ComplexPropertyExpression
        )
    }

    /// Get a human-readable description of the issue
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            ValidationIssue::EmptyHead => "Rule has no head atoms".to_string(),
            ValidationIssue::EmptyBody => "Rule has no body atoms".to_string(),
            ValidationIssue::SafetyViolation(msg) => format!("Safety violation: {msg}"),
            ValidationIssue::SingleUseVariable(var) => format!("Variable {var} used only once"),
            ValidationIssue::ComplexClassExpression => {
                "Complex class expression in atom".to_string()
            }
            ValidationIssue::ComplexPropertyExpression => {
                "Complex property expression in atom".to_string()
            }
            ValidationIssue::BuiltInInHead => {
                "Built-in predicate should not appear in rule head".to_string()
            }
            ValidationIssue::InvalidBuiltInIRI(iri) => format!("Invalid built-in IRI: {iri}"),
            ValidationIssue::UnknownBuiltIn(iri) => format!("Unknown built-in predicate: {iri}"),
            ValidationIssue::IncorrectBuiltInArity {
                builtin,
                expected,
                actual,
            } => format!("Built-in {builtin} expects {expected} arguments, got {actual}"),
            ValidationIssue::NonStandardVariableName(var) => {
                format!("Variable {var} doesn't follow naming conventions")
            }
            ValidationIssue::ReservedVariableName(var) => {
                format!("Variable {var} uses reserved namespace")
            }
        }
    }
}

/// Validation warning types
#[derive(Debug, Clone)]
pub enum ValidationWarning {
    /// Rule may cause infinite recursion
    RecursionRisk,
    /// Rule may be inefficient
    PerformanceWarning(String),
}

impl ValidationWarning {
    /// Get a human-readable description of the warning
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            ValidationWarning::RecursionRisk => "Rule may cause infinite recursion".to_string(),
            ValidationWarning::PerformanceWarning(msg) => format!("Performance warning: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, ClassExpression};

    #[test]
    fn test_validator_creation() {
        let validator = SWRLValidator::new();
        assert!(!validator.is_strict_mode());

        let strict_validator = SWRLValidator::new_strict();
        assert!(strict_validator.is_strict_mode());
    }

    #[test]
    fn test_empty_rule_validation() {
        let validator = SWRLValidator::new();
        let empty_rule = SWRLRule::new(Vec::new(), Vec::new());

        let result = validator.validate_rule(&empty_rule).expect("Failed to validate SWRL rule");
        assert!(!result.is_valid);
        assert_eq!(result.issues.len(), 2); // Empty head and empty body
    }

    #[test]
    fn test_safety_validation() {
        let validator = SWRLValidator::new();

        // Create a safe rule: Person(?x) -> Student(?x)
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let safe_rule = SWRLRule::new(vec![head_atom], vec![body_atom]);
        let result = validator.validate_rule(&safe_rule).expect("Failed to validate SWRL rule");

        assert!(result.is_valid);
    }

    #[test]
    fn test_unsafe_rule_validation() {
        let validator = SWRLValidator::new();

        // Create an unsafe rule: Person(?x) -> Student(?y)
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
        let result = validator.validate_rule(&unsafe_rule).expect("Failed to validate SWRL rule");

        assert!(!result.is_valid);
        assert!(result.has_errors());
    }

    #[test]
    fn test_builtin_validation() {
        let validator = SWRLValidator::new();

        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));
        let var_y = SWRLVariable::new(IRI::new("http://example.org/var#y"));

        // Valid built-in usage
        let builtin_atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#equal"),
            arguments: vec![
                SWRLDArgument::Variable(var_x.clone()),
                SWRLDArgument::Variable(var_y.clone()),
            ],
        };

        let class_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_y),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![class_atom, builtin_atom]);
        let result = validator.validate_rule(&rule).expect("Failed to validate SWRL rule");

        // Should be valid (both variables are used in class atom)
        assert!(result.is_valid);
    }

    #[test]
    fn test_unknown_builtin_validation() {
        let validator = SWRLValidator::new();

        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        // Unknown built-in
        let builtin_atom = SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://example.org/unknown#builtin"),
            arguments: vec![SWRLDArgument::Variable(var_x.clone())],
        };

        let class_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![class_atom, builtin_atom]);
        let result = validator.validate_rule(&rule).expect("Failed to validate SWRL rule");

        // Should have an unknown built-in issue
        assert!(
            result
                .issues
                .iter()
                .any(|issue| matches!(issue, ValidationIssue::UnknownBuiltIn(_)))
        );
    }
}
