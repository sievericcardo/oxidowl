//! SWRL Additional Comparison Built-in Predicates
//!
//! This module implements additional comparison built-ins that are part
//! of the standard SWRL built-in predicates but not yet implemented.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

// =============================================================================
// ADDITIONAL COMPARISON BUILT-INS
// =============================================================================

/// Built-in for comparing if two values are unequal (same as notEqual but different IRI)
pub struct NotEqualToBuiltIn;

impl SWRLBuiltIn for NotEqualToBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("NotEqualTo expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(s1), SWRLValue::String(s2)) => Ok(SWRLValue::Boolean(s1 != s2)),
            (SWRLValue::Integer(i1), SWRLValue::Integer(i2)) => Ok(SWRLValue::Boolean(i1 != i2)),
            (SWRLValue::Float(f1), SWRLValue::Float(f2)) => {
                Ok(SWRLValue::Boolean((f1 - f2).abs() > f64::EPSILON))
            }
            (SWRLValue::Boolean(b1), SWRLValue::Boolean(b2)) => Ok(SWRLValue::Boolean(b1 != b2)),
            (SWRLValue::Integer(i), SWRLValue::Float(f)) => {
                Ok(SWRLValue::Boolean((*i as f64 - f).abs() > f64::EPSILON))
            }
            (SWRLValue::Float(f), SWRLValue::Integer(i)) => {
                Ok(SWRLValue::Boolean((f - *i as f64).abs() > f64::EPSILON))
            }
            _ => Ok(SWRLValue::Boolean(true)), // Different types are not equal
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#notEqualTo"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// String matching with pattern (simple pattern matching, not regex)
pub struct MatchesBuiltIn;

impl SWRLBuiltIn for MatchesBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Matches expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(text), SWRLValue::String(pattern)) => {
                // Simple wildcard matching: * matches any sequence, ? matches any single char
                let result = simple_pattern_match(text, pattern);
                Ok(SWRLValue::Boolean(result))
            }
            _ => Err(Error::reasoning("Matches requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#matches"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Boolean AND operation
pub struct BooleanAndBuiltIn;

impl SWRLBuiltIn for BooleanAndBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("BooleanAnd expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Boolean(b1), SWRLValue::Boolean(b2)) => Ok(SWRLValue::Boolean(*b1 && *b2)),
            _ => Err(Error::reasoning("BooleanAnd requires boolean arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#booleanAnd"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Boolean OR operation
pub struct BooleanOrBuiltIn;

impl SWRLBuiltIn for BooleanOrBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("BooleanOr expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Boolean(b1), SWRLValue::Boolean(b2)) => Ok(SWRLValue::Boolean(*b1 || *b2)),
            _ => Err(Error::reasoning("BooleanOr requires boolean arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#booleanOr"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Min value between two numbers
pub struct MinBuiltIn;

impl SWRLBuiltIn for MinBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "Min expects exactly 3 arguments (result, value1, value2)",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::Integer(result), SWRLValue::Integer(v1), SWRLValue::Integer(v2)) => {
                let min_val = v1.min(v2);
                Ok(SWRLValue::Boolean(*result == *min_val))
            }
            (SWRLValue::Float(result), SWRLValue::Float(v1), SWRLValue::Float(v2)) => {
                let min_val = v1.min(*v2);
                Ok(SWRLValue::Boolean((result - min_val).abs() < f64::EPSILON))
            }
            (SWRLValue::Float(result), SWRLValue::Integer(v1), SWRLValue::Float(v2)) => {
                let min_val = (*v1 as f64).min(*v2);
                Ok(SWRLValue::Boolean((result - min_val).abs() < f64::EPSILON))
            }
            (SWRLValue::Float(result), SWRLValue::Float(v1), SWRLValue::Integer(v2)) => {
                let min_val = v1.min(*v2 as f64);
                Ok(SWRLValue::Boolean((result - min_val).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("Min requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#min"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Max value between two numbers
pub struct MaxBuiltIn;

impl SWRLBuiltIn for MaxBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "Max expects exactly 3 arguments (result, value1, value2)",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::Integer(result), SWRLValue::Integer(v1), SWRLValue::Integer(v2)) => {
                let max_val = v1.max(v2);
                Ok(SWRLValue::Boolean(*result == *max_val))
            }
            (SWRLValue::Float(result), SWRLValue::Float(v1), SWRLValue::Float(v2)) => {
                let max_val = v1.max(*v2);
                Ok(SWRLValue::Boolean((result - max_val).abs() < f64::EPSILON))
            }
            (SWRLValue::Float(result), SWRLValue::Integer(v1), SWRLValue::Float(v2)) => {
                let max_val = (*v1 as f64).max(*v2);
                Ok(SWRLValue::Boolean((result - max_val).abs() < f64::EPSILON))
            }
            (SWRLValue::Float(result), SWRLValue::Float(v1), SWRLValue::Integer(v2)) => {
                let max_val = v1.max(*v2 as f64);
                Ok(SWRLValue::Boolean((result - max_val).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("Max requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#max"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

// Helper function for simple pattern matching
fn simple_pattern_match(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    fn match_recursive(text: &[char], pattern: &[char]) -> bool {
        match (text.first(), pattern.first()) {
            (None, None) => true,
            (Some(_), None) => false,
            (None, Some('*')) => match_recursive(text, &pattern[1..]),
            (None, Some(_)) => false,
            (Some(tc), Some('*')) => {
                // Try matching with consuming character or without
                match_recursive(text, &pattern[1..]) || match_recursive(&text[1..], pattern)
            }
            (Some(tc), Some('?')) => match_recursive(&text[1..], &pattern[1..]),
            (Some(tc), Some(pc)) => {
                if tc == pc {
                    match_recursive(&text[1..], &pattern[1..])
                } else {
                    false
                }
            }
        }
    }

    match_recursive(&text_chars, &pattern_chars)
}

/// Function to register all additional comparison built-ins to a registry
pub fn register_additional_comparison_builtins(
    registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry,
) {
    use crate::ontology::IRI;

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#notEqualTo"),
        Box::new(NotEqualToBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#booleanAnd"),
        Box::new(BooleanAndBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#booleanOr"),
        Box::new(BooleanOrBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#min"),
        Box::new(MinBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#max"),
        Box::new(MaxBuiltIn),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_equal_to_builtin() {
        let builtin = NotEqualToBuiltIn;

        let args = vec![
            SWRLValue::String("hello".to_string()),
            SWRLValue::String("world".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![SWRLValue::Integer(5), SWRLValue::Integer(5)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(simple_pattern_match("hello", "hello"));
        assert!(simple_pattern_match("hello", "h*"));
        assert!(simple_pattern_match("hello", "*o"));
        assert!(simple_pattern_match("hello", "h?llo"));
        assert!(!simple_pattern_match("hello", "world"));
    }

    #[test]
    fn test_boolean_operations() {
        let and_builtin = BooleanAndBuiltIn;
        let or_builtin = BooleanOrBuiltIn;

        let args = vec![SWRLValue::Boolean(true), SWRLValue::Boolean(false)];
        let result = and_builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));

        let result = or_builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_min_max_builtins() {
        let min_builtin = MinBuiltIn;
        let max_builtin = MaxBuiltIn;

        let args = vec![
            SWRLValue::Integer(5),
            SWRLValue::Integer(10),
            SWRLValue::Integer(7),
        ];
        let result = min_builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false)); // 5 != min(10, 7)

        let args = vec![
            SWRLValue::Integer(10),
            SWRLValue::Integer(10),
            SWRLValue::Integer(7),
        ];
        let result = max_builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true)); // 10 == max(10, 7)
    }
}
