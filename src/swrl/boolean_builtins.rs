//! SWRL Boolean Built-in Predicates
//!
//! This module implements boolean logic built-in predicates for SWRL.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

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

/// Function to register all boolean built-ins to a registry
pub fn register_boolean_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#booleanNot"),
        Box::new(BooleanNotBuiltIn),
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
        let result = builtin.execute(&args).expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));

        // Test false -> true
        let args = vec![SWRLValue::Boolean(true), SWRLValue::Boolean(false)];
        let result = builtin.execute(&args).expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));

        // Test mismatch
        let args = vec![SWRLValue::Boolean(true), SWRLValue::Boolean(true)];
        let result = builtin.execute(&args).expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(false));
    }
}
