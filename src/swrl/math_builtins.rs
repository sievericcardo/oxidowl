//! SWRL Mathematical Built-in Predicates
//!
//! This module implements additional mathematical built-in predicates for SWRL
//! that extend the core mathematical operations.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

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

/// Round half to even built-in predicate (IEEE 754 rounding)
pub struct RoundHalfToEvenBuiltIn;

impl SWRLBuiltIn for RoundHalfToEvenBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("RoundHalfToEven expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::Float(input)) => {
                // IEEE 754 round-half-to-even
                let rounded = if input.fract() == 0.5 || input.fract() == -0.5 {
                    let truncated = input.trunc();
                    if (truncated as i64) % 2 == 0 {
                        truncated
                    } else {
                        if *input > 0.0 { truncated + 1.0 } else { truncated - 1.0 }
                    }
                } else {
                    input.round()
                };
                Ok(SWRLValue::Boolean(*result == rounded as i64))
            }
            (SWRLValue::Float(result), SWRLValue::Float(input)) => {
                let rounded = if input.fract() == 0.5 || input.fract() == -0.5 {
                    let truncated = input.trunc();
                    if (truncated as i64) % 2 == 0 {
                        truncated
                    } else {
                        if *input > 0.0 { truncated + 1.0 } else { truncated - 1.0 }
                    }
                } else {
                    input.round()
                };
                Ok(SWRLValue::Boolean((result - rounded).abs() < f64::EPSILON))
            }
            _ => Err(Error::reasoning("RoundHalfToEven requires numeric arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#roundHalfToEven"
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

/// Function to register all math built-ins to a registry
pub fn register_math_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

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
        IRI::new("http://www.w3.org/2003/11/swrlb#roundHalfToEven"),
        Box::new(RoundHalfToEvenBuiltIn),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLValue;

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
    fn test_integer_divide() {
        let builtin = IntegerDivideBuiltIn;
        
        let args = vec![SWRLValue::Integer(2), SWRLValue::Integer(7), SWRLValue::Integer(3)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true)); // 7 / 3 = 2 (integer division)
        
        let args = vec![SWRLValue::Integer(3), SWRLValue::Integer(7), SWRLValue::Integer(3)];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }
}
