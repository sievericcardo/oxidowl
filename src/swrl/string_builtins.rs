//! SWRL String Built-in Predicates
//!
//! This module implements string manipulation built-in predicates for SWRL.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

// =============================================================================
// STRING BUILT-INS
// =============================================================================

/// String equal ignore case built-in predicate
pub struct StringEqualIgnoreCaseBuiltIn;

impl SWRLBuiltIn for StringEqualIgnoreCaseBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "StringEqualIgnoreCase expects exactly 2 arguments",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(a), SWRLValue::String(b)) => {
                Ok(SWRLValue::Boolean(a.to_lowercase() == b.to_lowercase()))
            }
            _ => Err(Error::reasoning(
                "StringEqualIgnoreCase requires string arguments",
            )),
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
            return Err(Error::reasoning(
                "NormalizeSpace expects exactly 2 arguments",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(result), SWRLValue::String(input)) => {
                let normalized = input.split_whitespace().collect::<Vec<&str>>().join(" ");
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
            return Err(Error::reasoning(
                "ContainsIgnoreCase expects exactly 2 arguments",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(haystack), SWRLValue::String(needle)) => Ok(SWRLValue::Boolean(
                haystack.to_lowercase().contains(&needle.to_lowercase()),
            )),
            _ => Err(Error::reasoning(
                "ContainsIgnoreCase requires string arguments",
            )),
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
            return Err(Error::reasoning(
                "SubstringBefore expects exactly 3 arguments",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (
                SWRLValue::String(result),
                SWRLValue::String(string),
                SWRLValue::String(delimiter),
            ) => {
                let substring = if let Some(pos) = string.find(delimiter) {
                    &string[..pos]
                } else {
                    ""
                };
                Ok(SWRLValue::Boolean(*result == substring))
            }
            _ => Err(Error::reasoning(
                "SubstringBefore requires string arguments",
            )),
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
            return Err(Error::reasoning(
                "SubstringAfter expects exactly 3 arguments",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (
                SWRLValue::String(result),
                SWRLValue::String(string),
                SWRLValue::String(delimiter),
            ) => {
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

/// Translate string built-in predicate
pub struct TranslateBuiltIn;

impl SWRLBuiltIn for TranslateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 4 {
            return Err(Error::reasoning(
                "Translate expects exactly 4 arguments (result, string, from_chars, to_chars)",
            ));
        }

        match (&args[0], &args[1], &args[2], &args[3]) {
            (
                SWRLValue::String(result),
                SWRLValue::String(input),
                SWRLValue::String(from_chars),
                SWRLValue::String(to_chars),
            ) => {
                let from_vec: Vec<char> = from_chars.chars().collect();
                let to_vec: Vec<char> = to_chars.chars().collect();

                let mut translated = String::new();
                for ch in input.chars() {
                    if let Some(pos) = from_vec.iter().position(|&c| c == ch) {
                        if pos < to_vec.len() {
                            translated.push(to_vec[pos]);
                        }
                        // If position >= to_vec.len(), character is deleted (no push)
                    } else {
                        translated.push(ch);
                    }
                }

                Ok(SWRLValue::Boolean(*result == translated))
            }
            _ => Err(Error::reasoning("Translate requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#translate"
    }

    fn arity(&self) -> Option<usize> {
        Some(4)
    }
}

/// Function to register all string built-ins to a registry
pub fn register_string_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

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
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#translate"),
        Box::new(TranslateBuiltIn),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLValue;

    #[test]
    fn test_string_equal_ignore_case() {
        let builtin = StringEqualIgnoreCaseBuiltIn;

        let args = vec![
            SWRLValue::String("Hello".to_string()),
            SWRLValue::String("HELLO".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![
            SWRLValue::String("Hello".to_string()),
            SWRLValue::String("World".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_normalize_space() {
        let builtin = NormalizeSpaceBuiltIn;

        let args = vec![
            SWRLValue::String("hello world test".to_string()),
            SWRLValue::String("  hello   world    test  ".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_substring_before() {
        let builtin = SubstringBeforeBuiltIn;

        let args = vec![
            SWRLValue::String("hello".to_string()),
            SWRLValue::String("hello world".to_string()),
            SWRLValue::String(" ".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }
}
