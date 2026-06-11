//! This module provides implementation of standard SWRL built-in predicates
//! and a registry system for managing and executing them.

use crate::ontology::{IRI, Individual, Literal};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fmt;

/// SWRL value type for built-in operations
#[derive(Debug, Clone, PartialEq)]
pub enum SWRLValue {
    /// String value
    String(String),
    /// Integer value  
    Integer(i64),
    /// Float value
    Float(f64),
    /// Decimal value
    Decimal(f64),
    /// Boolean value
    Boolean(bool),
    /// Date/time value (as string for now)
    DateTime(String),
    /// URI value
    Uri(String),
    /// Individual reference
    Individual(Individual),
    /// Literal value
    Literal(Literal),
}

impl Eq for SWRLValue {}

impl std::hash::Hash for SWRLValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            SWRLValue::String(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            SWRLValue::Integer(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            SWRLValue::Float(f) => {
                2u8.hash(state);
                // Hash float as bits to handle NaN/infinity properly
                f.to_bits().hash(state);
            }
            SWRLValue::Decimal(d) => {
                3u8.hash(state);
                // Hash decimal as bits to handle NaN/infinity properly
                d.to_bits().hash(state);
            }
            SWRLValue::Boolean(b) => {
                4u8.hash(state);
                b.hash(state);
            }
            SWRLValue::DateTime(dt) => {
                5u8.hash(state);
                dt.hash(state);
            }
            SWRLValue::Uri(uri) => {
                6u8.hash(state);
                uri.hash(state);
            }
            SWRLValue::Individual(ind) => {
                7u8.hash(state);
                ind.hash(state);
            }
            SWRLValue::Literal(lit) => {
                8u8.hash(state);
                lit.hash(state);
            }
        }
    }
}

impl SWRLValue {
    /// Check if this value represents a variable (not applicable for built-ins)
    #[must_use]
    pub fn is_variable(&self) -> bool {
        false // Built-in values are always concrete
    }

    /// Get the individual if this is an individual value
    #[must_use]
    pub fn as_individual(&self) -> Option<&Individual> {
        match self {
            SWRLValue::Individual(ind) => Some(ind),
            _ => None,
        }
    }

    /// Get the literal if this is a literal value
    #[must_use]
    pub fn as_literal(&self) -> Option<&Literal> {
        match self {
            SWRLValue::Literal(lit) => Some(lit),
            _ => None,
        }
    }
}

impl fmt::Display for SWRLValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SWRLValue::String(s) => write!(f, "\"{s}\""),
            SWRLValue::Integer(i) => write!(f, "{i}"),
            SWRLValue::Float(fl) => write!(f, "{fl}"),
            SWRLValue::Decimal(d) => write!(f, "{d}"),
            SWRLValue::Boolean(b) => write!(f, "{b}"),
            SWRLValue::DateTime(dt) => write!(f, "\"{dt}\""),
            SWRLValue::Uri(uri) => write!(f, "<{uri}>"),
            SWRLValue::Individual(ind) => write!(f, "{ind:?}"),
            SWRLValue::Literal(lit) => write!(f, "{lit}"),
        }
    }
}

/// Trait for SWRL built-in predicates
pub trait SWRLBuiltIn: Send + Sync {
    /// Execute the built-in with given arguments
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue>;
    /// Get the name/IRI of this built-in
    fn name(&self) -> &str;
    /// Get expected arity (number of arguments)
    fn arity(&self) -> Option<usize>;
    /// Check if the built-in can be evaluated with the given arguments
    fn can_evaluate(&self, args: &[SWRLValue]) -> bool {
        if let Some(expected_arity) = self.arity() {
            args.len() == expected_arity
        } else {
            !args.is_empty() // At least one argument for variable arity
        }
    }
    /// Validate the argument count
    fn validate_argument_count(&self, count: usize) -> bool {
        if let Some(expected_arity) = self.arity() {
            count == expected_arity
        } else {
            count > 0 // At least one argument for variable arity
        }
    }
    /// Get expected argument count
    fn expected_argument_count(&self) -> usize {
        self.arity().unwrap_or(1) // Default to 1 for variable arity
    }
    /// Validate argument types
    fn validate_argument_types(&self, _args: &[SWRLValue]) -> bool {
        // Default implementation accepts all types
        // Specific built-ins can override for type checking
        true
    }
}

impl fmt::Debug for dyn SWRLBuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SWRLBuiltIn")
            .field("name", &self.name())
            .field("arity", &self.arity())
            .finish()
    }
}

/// Registry for SWRL built-in predicates
pub struct SWRLBuiltInRegistry {
    builtins: HashMap<String, Box<dyn SWRLBuiltIn>>,
}

impl fmt::Debug for SWRLBuiltInRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SWRLBuiltInRegistry")
            .field("builtins_count", &self.builtins.len())
            .finish()
    }
}

impl Default for SWRLBuiltInRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SWRLBuiltInRegistry {
    /// Create a new built-in registry with standard built-ins
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            builtins: HashMap::new(),
        };

        // Register standard built-ins
        registry.register_standard_builtins();
        registry
    }

    /// Register a built-in predicate
    pub fn register_builtin(&mut self, iri: IRI, builtin: Box<dyn SWRLBuiltIn>) {
        self.builtins.insert(iri.to_string(), builtin);
    }

    /// Check if a built-in is registered
    #[must_use]
    pub fn is_registered(&self, iri: &IRI) -> bool {
        self.builtins.contains_key(&iri.to_string())
    }

    /// Execute a built-in predicate
    pub fn execute(&self, iri: &IRI, args: &[SWRLValue]) -> Result<SWRLValue> {
        if let Some(builtin) = self.builtins.get(&iri.to_string()) {
            builtin.execute(args)
        } else {
            Err(Error::reasoning(format!("Unknown built-in: {iri}")))
        }
    }

    /// Get a built-in by IRI
    #[must_use]
    pub fn get_builtin(&self, iri: &IRI) -> Option<&dyn SWRLBuiltIn> {
        self.builtins
            .get(&iri.to_string())
            .map(std::convert::AsRef::as_ref)
    }

    /// Get all registered built-in IRIs
    #[must_use]
    pub fn get_builtin_iris(&self) -> Vec<IRI> {
        self.builtins.keys().map(|s| IRI::new(s)).collect()
    }

    /// Register all standard SWRL built-ins
    fn register_standard_builtins(&mut self) {
        // Comparison built-ins
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#equal"),
            Box::new(EqualBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#notEqual"),
            Box::new(NotEqualBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#lessThan"),
            Box::new(LessThanBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#lessThanOrEqual"),
            Box::new(LessThanOrEqualBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#greaterThan"),
            Box::new(GreaterThanBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#greaterThanOrEqual"),
            Box::new(GreaterThanOrEqualBuiltIn),
        );

        // Math built-ins
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#add"),
            Box::new(AddBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#subtract"),
            Box::new(SubtractBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#multiply"),
            Box::new(MultiplyBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#divide"),
            Box::new(DivideBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#mod"),
            Box::new(ModBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#pow"),
            Box::new(PowBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#abs"),
            Box::new(AbsBuiltIn),
        );

        // String built-ins
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#stringLength"),
            Box::new(StringLengthBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#stringConcat"),
            Box::new(StringConcatBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#contains"),
            Box::new(ContainsBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#startsWith"),
            Box::new(StartsWithBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#endsWith"),
            Box::new(EndsWithBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#substring"),
            Box::new(SubstringBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#upperCase"),
            Box::new(UpperCaseBuiltIn),
        );
        self.register_builtin(
            IRI::new("http://www.w3.org/2003/11/swrlb#lowerCase"),
            Box::new(LowerCaseBuiltIn),
        );

        // Register additional built-ins from extended_builtins module
        crate::swrl::extended_builtins::register_extended_builtins(self);
    }
}

// Implementation of standard built-ins

/// Equal built-in predicate
struct EqualBuiltIn;

impl SWRLBuiltIn for EqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Equal expects exactly 2 arguments"));
        }
        Ok(SWRLValue::Boolean(args[0] == args[1]))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#equal"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Not equal built-in predicate
struct NotEqualBuiltIn;

impl SWRLBuiltIn for NotEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("NotEqual expects exactly 2 arguments"));
        }
        Ok(SWRLValue::Boolean(args[0] != args[1]))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#notEqual"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Less than built-in predicate
struct LessThanBuiltIn;

impl SWRLBuiltIn for LessThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("LessThan expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => a < b,
            (SWRLValue::Float(a), SWRLValue::Float(b)) => a < b,
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => (*a as f64) < *b,
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => *a < (*b as f64),
            _ => return Err(Error::reasoning("LessThan requires numeric arguments")),
        };

        Ok(SWRLValue::Boolean(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#lessThan"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Less than or equal built-in predicate
struct LessThanOrEqualBuiltIn;

impl SWRLBuiltIn for LessThanOrEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "LessThanOrEqual expects exactly 2 arguments",
            ));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => a <= b,
            (SWRLValue::Float(a), SWRLValue::Float(b)) => a <= b,
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => (*a as f64) <= *b,
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => *a <= (*b as f64),
            _ => {
                return Err(Error::reasoning(
                    "LessThanOrEqual requires numeric arguments",
                ));
            }
        };

        Ok(SWRLValue::Boolean(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#lessThanOrEqual"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Greater than built-in predicate
struct GreaterThanBuiltIn;

impl SWRLBuiltIn for GreaterThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("GreaterThan expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => a > b,
            (SWRLValue::Float(a), SWRLValue::Float(b)) => a > b,
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => (*a as f64) > *b,
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => *a > (*b as f64),
            _ => return Err(Error::reasoning("GreaterThan requires numeric arguments")),
        };

        Ok(SWRLValue::Boolean(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#greaterThan"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Greater than or equal built-in predicate
struct GreaterThanOrEqualBuiltIn;

impl SWRLBuiltIn for GreaterThanOrEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "GreaterThanOrEqual expects exactly 2 arguments",
            ));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => a >= b,
            (SWRLValue::Float(a), SWRLValue::Float(b)) => a >= b,
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => (*a as f64) >= *b,
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => *a >= (*b as f64),
            _ => {
                return Err(Error::reasoning(
                    "GreaterThanOrEqual requires numeric arguments",
                ));
            }
        };

        Ok(SWRLValue::Boolean(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#greaterThanOrEqual"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Add built-in predicate
struct AddBuiltIn;

impl SWRLBuiltIn for AddBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Add expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => SWRLValue::Integer(a + b),
            (SWRLValue::Float(a), SWRLValue::Float(b)) => SWRLValue::Float(a + b),
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => SWRLValue::Float(*a as f64 + b),
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => SWRLValue::Float(a + *b as f64),
            _ => return Err(Error::reasoning("Add requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#add"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Subtract built-in predicate
struct SubtractBuiltIn;

impl SWRLBuiltIn for SubtractBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Subtract expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => SWRLValue::Integer(a - b),
            (SWRLValue::Float(a), SWRLValue::Float(b)) => SWRLValue::Float(a - b),
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => SWRLValue::Float(*a as f64 - b),
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => SWRLValue::Float(a - *b as f64),
            _ => return Err(Error::reasoning("Subtract requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subtract"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Multiply built-in predicate
struct MultiplyBuiltIn;

impl SWRLBuiltIn for MultiplyBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Multiply expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => SWRLValue::Integer(a * b),
            (SWRLValue::Float(a), SWRLValue::Float(b)) => SWRLValue::Float(a * b),
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => SWRLValue::Float(*a as f64 * b),
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => SWRLValue::Float(a * *b as f64),
            _ => return Err(Error::reasoning("Multiply requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#multiply"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Divide built-in predicate
struct DivideBuiltIn;

impl SWRLBuiltIn for DivideBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Divide expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                SWRLValue::Float(*a as f64 / *b as f64)
            }
            (SWRLValue::Float(a), SWRLValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                SWRLValue::Float(a / b)
            }
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                SWRLValue::Float(*a as f64 / b)
            }
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                SWRLValue::Float(a / *b as f64)
            }
            _ => return Err(Error::reasoning("Divide requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#divide"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Modulo built-in predicate
struct ModBuiltIn;

impl SWRLBuiltIn for ModBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Mod expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::reasoning("Modulo by zero"));
                }
                SWRLValue::Integer(a % b)
            }
            (SWRLValue::Float(a), SWRLValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::reasoning("Modulo by zero"));
                }
                SWRLValue::Float(a % b)
            }
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::reasoning("Modulo by zero"));
                }
                SWRLValue::Float((*a as f64) % b)
            }
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::reasoning("Modulo by zero"));
                }
                SWRLValue::Float(a % (*b as f64))
            }
            _ => return Err(Error::reasoning("Mod requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#mod"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Power built-in predicate
struct PowBuiltIn;

impl SWRLBuiltIn for PowBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Pow expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::Integer(a), SWRLValue::Integer(b)) => {
                SWRLValue::Float((*a as f64).powf(*b as f64))
            }
            (SWRLValue::Float(a), SWRLValue::Float(b)) => SWRLValue::Float(a.powf(*b)),
            (SWRLValue::Integer(a), SWRLValue::Float(b)) => SWRLValue::Float((*a as f64).powf(*b)),
            (SWRLValue::Float(a), SWRLValue::Integer(b)) => SWRLValue::Float(a.powf(*b as f64)),
            _ => return Err(Error::reasoning("Pow requires numeric arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#pow"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Absolute value built-in predicate
struct AbsBuiltIn;

impl SWRLBuiltIn for AbsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("Abs expects exactly 1 argument"));
        }

        let result = match &args[0] {
            SWRLValue::Integer(a) => SWRLValue::Integer(a.abs()),
            SWRLValue::Float(a) => SWRLValue::Float(a.abs()),
            _ => return Err(Error::reasoning("Abs requires numeric argument")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#abs"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

/// String length built-in predicate
struct StringLengthBuiltIn;

impl SWRLBuiltIn for StringLengthBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("StringLength expects exactly 1 argument"));
        }

        let result = match &args[0] {
            SWRLValue::String(s) => SWRLValue::Integer(s.len() as i64),
            _ => return Err(Error::reasoning("StringLength requires string argument")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#stringLength"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

/// String concatenation built-in predicate
struct StringConcatBuiltIn;

impl SWRLBuiltIn for StringConcatBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("StringConcat expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::String(a), SWRLValue::String(b)) => SWRLValue::String(format!("{a}{b}")),
            _ => return Err(Error::reasoning("StringConcat requires string arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#stringConcat"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Contains built-in predicate
struct ContainsBuiltIn;

impl SWRLBuiltIn for ContainsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Contains expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::String(haystack), SWRLValue::String(needle)) => {
                SWRLValue::Boolean(haystack.contains(needle))
            }
            _ => return Err(Error::reasoning("Contains requires string arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#contains"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Starts with built-in predicate
struct StartsWithBuiltIn;

impl SWRLBuiltIn for StartsWithBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("StartsWith expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::String(string), SWRLValue::String(prefix)) => {
                SWRLValue::Boolean(string.starts_with(prefix))
            }
            _ => return Err(Error::reasoning("StartsWith requires string arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#startsWith"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Ends with built-in predicate
struct EndsWithBuiltIn;

impl SWRLBuiltIn for EndsWithBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("EndsWith expects exactly 2 arguments"));
        }

        let result = match (&args[0], &args[1]) {
            (SWRLValue::String(string), SWRLValue::String(suffix)) => {
                SWRLValue::Boolean(string.ends_with(suffix))
            }
            _ => return Err(Error::reasoning("EndsWith requires string arguments")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#endsWith"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Substring built-in predicate
struct SubstringBuiltIn;

impl SWRLBuiltIn for SubstringBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("Substring expects exactly 3 arguments"));
        }

        let result = match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(string), SWRLValue::Integer(start), SWRLValue::Integer(end)) => {
                let start_idx = *start as usize;
                let end_idx = *end as usize;

                if start_idx <= end_idx && end_idx <= string.len() {
                    SWRLValue::String(string[start_idx..end_idx].to_string())
                } else {
                    return Err(Error::reasoning("Invalid substring indices"));
                }
            }
            _ => {
                return Err(Error::reasoning(
                    "Substring requires string and integer arguments",
                ));
            }
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#substring"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Upper case built-in predicate
struct UpperCaseBuiltIn;

impl SWRLBuiltIn for UpperCaseBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("UpperCase expects exactly 1 argument"));
        }

        let result = match &args[0] {
            SWRLValue::String(s) => SWRLValue::String(s.to_uppercase()),
            _ => return Err(Error::reasoning("UpperCase requires string argument")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#upperCase"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

/// Lower case built-in predicate
struct LowerCaseBuiltIn;

impl SWRLBuiltIn for LowerCaseBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("LowerCase expects exactly 1 argument"));
        }

        let result = match &args[0] {
            SWRLValue::String(s) => SWRLValue::String(s.to_lowercase()),
            _ => return Err(Error::reasoning("LowerCase requires string argument")),
        };

        Ok(result)
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#lowerCase"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}
