//! Missing SWRL Built-in Predicates
//!
//! This module implements the missing built-in predicates from the W3C SWRL specification
//! that are not yet implemented in the current built-ins registry.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};
use std::collections::VecDeque;

// =============================================================================
// BOOLEAN BUILT-INS
// =============================================================================

/// Boolean NOT built-in predicate
pub struct BooleanNotBuiltIn;

impl SWRLBuiltIn for BooleanNotBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("BooleanNot expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Boolean(result), SWRLValue::Boolean(input)) => {
                if *result == !*input {
                    Ok(SWRLValue::Boolean(true))
                } else {
                    Ok(SWRLValue::Boolean(false))
                }
            }
            _ => Err(Error::reasoning("BooleanNot requires boolean arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#booleanNot"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

// =============================================================================
// ADDITIONAL MATH BUILT-INS
// =============================================================================

/// Integer division built-in predicate
pub struct IntegerDivideBuiltIn;

impl SWRLBuiltIn for IntegerDivideBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("IntegerDivide expects exactly 3 arguments"));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::Integer(result), SWRLValue::Integer(dividend), SWRLValue::Integer(divisor)) => {
                if *divisor == 0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                let expected = dividend / divisor; // Integer division in Rust truncates
                Ok(SWRLValue::Boolean(*result == expected))
            }
            _ => Err(Error::reasoning("IntegerDivide requires integer arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#integerDivide"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Unary plus built-in predicate
pub struct UnaryPlusBuiltIn;

impl SWRLBuiltIn for UnaryPlusBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("UnaryPlus expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Integer(input)) => {
                Ok(SWRLValue::Boolean(*result == *input))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == *input))
            }
            _ => Err(Error::reasoning("UnaryPlus requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#unaryPlus"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Unary minus built-in predicate
pub struct UnaryMinusBuiltIn;

impl SWRLBuiltIn for UnaryMinusBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("UnaryMinus expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Integer(input)) => {
                Ok(SWRLValue::Boolean(*result == -*input))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == -*input))
            }
            _ => Err(Error::reasoning("UnaryMinus requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#unaryMinus"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Ceiling built-in predicate
pub struct CeilingBuiltIn;

impl SWRLBuiltIn for CeilingBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Ceiling expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.ceil() as i64))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.ceil()))
            }
            _ => Err(Error::reasoning("Ceiling requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#ceiling"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Floor built-in predicate
pub struct FloorBuiltIn;

impl SWRLBuiltIn for FloorBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Floor expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.floor() as i64))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.floor()))
            }
            _ => Err(Error::reasoning("Floor requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#floor"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Round built-in predicate
pub struct RoundBuiltIn;

impl SWRLBuiltIn for RoundBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Round expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.round() as i64))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                Ok(SWRLValue::Boolean(*result == input.round()))
            }
            _ => Err(Error::reasoning("Round requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#round"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Trigonometric sine built-in predicate
pub struct SinBuiltIn;

impl SWRLBuiltIn for SinBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Sin expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                let computed = input.sin();
                // Use epsilon comparison for floating point
                Ok(SWRLValue::Boolean((result - computed).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("Sin requires float arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#sin"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Trigonometric cosine built-in predicate
pub struct CosBuiltIn;

impl SWRLBuiltIn for CosBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Cos expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                let computed = input.cos();
                Ok(SWRLValue::Boolean((result - computed).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("Cos requires float arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#cos"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Trigonometric tangent built-in predicate
pub struct TanBuiltIn;

impl SWRLBuiltIn for TanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Tan expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                let computed = input.tan();
                Ok(SWRLValue::Boolean((result - computed).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("Tan requires float arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#tan"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

// =============================================================================
// ADDITIONAL STRING BUILT-INS
// =============================================================================

/// String equal ignore case built-in predicate
pub struct StringEqualIgnoreCaseBuiltIn;

impl SWRLBuiltIn for StringEqualIgnoreCaseBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("StringEqualIgnoreCase expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(a), SWRLValue::String(b)) => {
                Ok(SWRLValue::Boolean(a.to_lowercase() == b.to_lowercase()))
            }
            _ => Err(Error::reasoning("StringEqualIgnoreCase requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Normalize space built-in predicate
pub struct NormalizeSpaceBuiltIn;

impl SWRLBuiltIn for NormalizeSpaceBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("NormalizeSpace expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(result), SWRLValue::String(input)) => {
                let normalized = input
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join(" ");
                Ok(SWRLValue::Boolean(*result == normalized))
            }
            _ => Err(Error::reasoning("NormalizeSpace requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#normalizeSpace"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Contains ignore case built-in predicate
pub struct ContainsIgnoreCaseBuiltIn;

impl SWRLBuiltIn for ContainsIgnoreCaseBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("ContainsIgnoreCase expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(haystack), SWRLValue::String(needle)) => {
                Ok(SWRLValue::Boolean(
                    haystack.to_lowercase().contains(&needle.to_lowercase())
                ))
            }
            _ => Err(Error::reasoning("ContainsIgnoreCase requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#containsIgnoreCase"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Substring before built-in predicate
pub struct SubstringBeforeBuiltIn;

impl SWRLBuiltIn for SubstringBeforeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("SubstringBefore expects exactly 3 arguments"));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(result), SWRLValue::String(string), SWRLValue::String(delimiter)) => {
                let substring = if let Some(pos) = string.find(delimiter) {
                    &string[..pos]
                } else {
                    ""
                };
                Ok(SWRLValue::Boolean(*result == substring))
            }
            _ => Err(Error::reasoning("SubstringBefore requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#substringBefore"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Substring after built-in predicate
pub struct SubstringAfterBuiltIn;

impl SWRLBuiltIn for SubstringAfterBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("SubstringAfter expects exactly 3 arguments"));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(result), SWRLValue::String(string), SWRLValue::String(delimiter)) => {
                let substring = if let Some(pos) = string.find(delimiter) {
                    &string[pos + delimiter.len()..]
                } else {
                    ""
                };
                Ok(SWRLValue::Boolean(*result == substring))
            }
            _ => Err(Error::reasoning("SubstringAfter requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#substringAfter"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

// =============================================================================
// URI BUILT-INS
// =============================================================================

/// Resolve URI built-in predicate
pub struct ResolveUriBuiltIn;

impl SWRLBuiltIn for ResolveUriBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("ResolveURI expects exactly 3 arguments"));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::Uri(result), SWRLValue::Uri(relative), SWRLValue::Uri(base)) => {
                // Simplified URI resolution - for full implementation would need URI parsing
                let resolved = if relative.starts_with("http://") || relative.starts_with("https://") {
                    relative.clone()
                } else if base.ends_with('/') {
                    format!("{}{}", base, relative)
                } else {
                    format!("{}/{}", base, relative)
                };
                Ok(SWRLValue::Boolean(*result == resolved))
            }
            _ => Err(Error::reasoning("ResolveURI requires URI arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#resolveURI"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

// =============================================================================
// LIST BUILT-INS (Basic Implementation)
// =============================================================================

/// List concatenation built-in predicate
pub struct ListConcatBuiltIn;

impl SWRLBuiltIn for ListConcatBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 {
            return Err(Error::reasoning("ListConcat expects at least 2 arguments"));
        }

        // For this simplified implementation, we'll represent lists as comma-separated strings
        if let SWRLValue::String(result) = &args[0] {
            let mut concatenated = String::new();
            for arg in &args[1..] {
                if let SWRLValue::String(list_str) = arg {
                    if !concatenated.is_empty() && !list_str.is_empty() {
                        concatenated.push(',');
                    }
                    concatenated.push_str(list_str);
                }
            }
            Ok(SWRLValue::Boolean(*result == concatenated))
        } else {
            Err(Error::reasoning("ListConcat first argument must be a string"))
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#listConcat"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity
    }
}

/// List member built-in predicate
pub struct MemberBuiltIn;

impl SWRLBuiltIn for MemberBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Member expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (element, SWRLValue::String(list_str)) => {
                // Simple list representation as comma-separated values
                let members: Vec<&str> = list_str.split(',').map(|s| s.trim()).collect();
                let element_str = match element {
                    SWRLValue::String(s) => s.as_str(),
                    SWRLValue::Integer(i) => return Ok(SWRLValue::Boolean(members.iter().any(|m| m.parse::<i64>().map_or(false, |n| n == *i)))),
                    SWRLValue::Float(f) => return Ok(SWRLValue::Boolean(members.iter().any(|m| m.parse::<f64>().map_or(false, |n| (n - f).abs() < f64::EPSILON)))),
                    _ => return Err(Error::reasoning("Unsupported element type for list membership")),
                };
                Ok(SWRLValue::Boolean(members.contains(&element_str)))
            }
            _ => Err(Error::reasoning("Member requires element and list arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#member"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// List length built-in predicate
pub struct ListLengthBuiltIn;

impl SWRLBuiltIn for ListLengthBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("ListLength expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::String(list_str)) => {
                let length = if list_str.is_empty() {
                    0
                } else {
                    list_str.split(',').count() as i64
                };
                Ok(SWRLValue::Boolean(*result == length))
            }
            _ => Err(Error::reasoning("ListLength requires integer result and list arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#length"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Function to register all missing built-ins to a registry
pub fn register_missing_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

    // Boolean built-ins
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#booleanNot"),
        Box::new(BooleanNotBuiltIn),
    );

    // Additional math built-ins
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#integerDivide"),
        Box::new(IntegerDivideBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#unaryPlus"),
        Box::new(UnaryPlusBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#unaryMinus"),
        Box::new(UnaryMinusBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#ceiling"),
        Box::new(CeilingBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#floor"),
        Box::new(FloorBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#round"),
        Box::new(RoundBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#sin"),
        Box::new(SinBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#cos"),
        Box::new(CosBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#tan"),
        Box::new(TanBuiltIn),
    );

    // Additional string built-ins
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase"),
        Box::new(StringEqualIgnoreCaseBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#normalizeSpace"),
        Box::new(NormalizeSpaceBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#containsIgnoreCase"),
        Box::new(ContainsIgnoreCaseBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#substringBefore"),
        Box::new(SubstringBeforeBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#substringAfter"),
        Box::new(SubstringAfterBuiltIn),
    );

    // URI built-ins
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#resolveURI"),
        Box::new(ResolveUriBuiltIn),
    );

    // List built-ins (basic implementation)
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#listConcat"),
        Box::new(ListConcatBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#member"),
        Box::new(MemberBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#length"),
        Box::new(ListLengthBuiltIn),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLValue;

    #[test]
    fn test_boolean_not() {
        let builtin = BooleanNotBuiltIn;
        
        // Test true -> false
        let args = vec![SWRLValue::Boolean(false), SWRLValue::Boolean(true)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        // Test false -> true
        let args = vec![SWRLValue::Boolean(true), SWRLValue::Boolean(false)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        // Test mismatch
        let args = vec![SWRLValue::Boolean(true), SWRLValue::Boolean(true)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_ceiling() {
        let builtin = CeilingBuiltIn;
        
        let args = vec![SWRLValue::Integer(4), SWRLValue::Float(3.2)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        let args = vec![SWRLValue::Integer(3), SWRLValue::Float(3.2)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_string_equal_ignore_case() {
        let builtin = StringEqualIgnoreCaseBuiltIn;
        
        let args = vec![SWRLValue::String("Hello".to_string()), SWRLValue::String("HELLO".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        let args = vec![SWRLValue::String("Hello".to_string()), SWRLValue::String("World".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_list_member() {
        let builtin = MemberBuiltIn;
        
        let args = vec![SWRLValue::String("apple".to_string()), SWRLValue::String("apple,banana,cherry".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
        
        let args = vec![SWRLValue::String("grape".to_string()), SWRLValue::String("apple,banana,cherry".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }
}
