//! Regex Built-in Predicates for SWRL
//!
//! This module implements regex-based string processing built-ins for SWRL.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// =============================================================================
// REGEX BUILT-IN REGISTRY
// =============================================================================

/// Registry for regex built-in predicates with caching
pub struct RegexBuiltInRegistry {
    builtins: HashMap<String, Box<dyn SWRLBuiltIn>>,
    regex_cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl RegexBuiltInRegistry {
    /// Create a new registry with all regex built-ins
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            builtins: HashMap::new(),
            regex_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        let cache = Arc::clone(&registry.regex_cache);

        // Core pattern matching
        registry.register(
            "http://www.w3.org/2003/11/swrlb#matches",
            Box::new(MatchesBuiltIn::new(Arc::clone(&cache))),
        );
        registry.register(
            "http://www.w3.org/2003/11/swrlb#replace",
            Box::new(ReplaceBuiltIn::new(Arc::clone(&cache))),
        );

        // Advanced operations
        registry.register(
            "http://www.w3.org/2003/11/swrlb#regexReplace",
            Box::new(RegexReplaceBuiltIn::new(Arc::clone(&cache))),
        );
        registry.register(
            "http://www.w3.org/2003/11/swrlb#tokenize",
            Box::new(TokenizeBuiltIn::new(Arc::clone(&cache))),
        );
        registry.register(
            "http://www.w3.org/2003/11/swrlb#split",
            Box::new(SplitBuiltIn::new(Arc::clone(&cache))),
        );

        // Extract operations
        registry.register(
            "http://www.w3.org/2003/11/swrlb#extract",
            Box::new(ExtractBuiltIn::new(Arc::clone(&cache))),
        );
        registry.register(
            "http://www.w3.org/2003/11/swrlb#extractAll",
            Box::new(ExtractAllBuiltIn::new(Arc::clone(&cache))),
        );

        // Validation
        registry.register(
            "http://www.w3.org/2003/11/swrlb#isValidPattern",
            Box::new(IsValidPatternBuiltIn),
        );

        registry
    }

    /// Register a built-in predicate
    pub fn register(&mut self, iri: &str, builtin: Box<dyn SWRLBuiltIn>) {
        self.builtins.insert(iri.to_string(), builtin);
    }

    /// Get a built-in predicate by IRI
    #[must_use]
    pub fn get(&self, iri: &str) -> Option<&dyn SWRLBuiltIn> {
        self.builtins.get(iri).map(std::convert::AsRef::as_ref)
    }

    /// Get all registered built-in IRIs
    #[must_use]
    pub fn get_all_iris(&self) -> Vec<String> {
        self.builtins.keys().cloned().collect()
    }

    /// Get count of registered built-ins
    #[must_use]
    pub fn count(&self) -> usize {
        self.builtins.len()
    }

    /// Clear regex cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.regex_cache.lock() {
            cache.clear();
        }
    }
}

impl Default for RegexBuiltInRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get compiled regex from cache or compile and cache it
fn get_or_compile_regex(
    cache: &Arc<Mutex<HashMap<String, Regex>>>,
    pattern: &str,
    case_insensitive: bool,
) -> Result<Regex> {
    let cache_key = if case_insensitive {
        format!("(?i){pattern}")
    } else {
        pattern.to_string()
    };

    // Try to get from cache first
    if let Ok(cache_guard) = cache.lock()
        && let Some(regex) = cache_guard.get(&cache_key)
    {
        return Ok(regex.clone());
    }

    // Compile new regex
    let regex = if case_insensitive {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|e| Error::reasoning(format!("Invalid regex pattern '{pattern}': {e}")))?
    } else {
        Regex::new(pattern)
            .map_err(|e| Error::reasoning(format!("Invalid regex pattern '{pattern}': {e}")))?
    };

    // Cache the compiled regex
    if let Ok(mut cache_guard) = cache.lock() {
        cache_guard.insert(cache_key, regex.clone());
    }

    Ok(regex)
}

/// Extract string from `SWRLValue`
fn extract_string(value: &SWRLValue) -> Result<&str> {
    match value {
        SWRLValue::String(s) => Ok(s),
        _ => Err(Error::reasoning("Expected string value")),
    }
}

/// Extract boolean flag from `SWRLValue` (optional parameter)
fn extract_optional_bool(value: Option<&SWRLValue>) -> Result<bool> {
    match value {
        Some(SWRLValue::Boolean(b)) => Ok(*b),
        Some(_) => Err(Error::reasoning("Expected boolean value for flag")),
        None => Ok(false), // Default to false
    }
}

// =============================================================================
// CORE REGEX BUILT-INS
// =============================================================================

/// Pattern matching built-in
pub struct MatchesBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl MatchesBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for MatchesBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 || args.len() > 3 {
            return Err(Error::reasoning(
                "matches expects 2 or 3 arguments (string, pattern, [case_insensitive])",
            ));
        }

        let text = extract_string(&args[0])?;
        let pattern = extract_string(&args[1])?;
        let case_insensitive = extract_optional_bool(args.get(2))?;

        let regex = get_or_compile_regex(&self.cache, pattern, case_insensitive)?;
        Ok(SWRLValue::Boolean(regex.is_match(text)))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#matches"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (2-3)
    }
}

/// Basic replace built-in
pub struct ReplaceBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl ReplaceBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for ReplaceBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 4 {
            return Err(Error::reasoning(
                "replace expects exactly 4 arguments (result, string, pattern, replacement)",
            ));
        }

        let expected_result = extract_string(&args[0])?;
        let text = extract_string(&args[1])?;
        let pattern = extract_string(&args[2])?;
        let replacement = extract_string(&args[3])?;

        let regex = get_or_compile_regex(&self.cache, pattern, false)?;
        let actual_result = regex.replace_all(text, replacement);

        Ok(SWRLValue::Boolean(*expected_result == actual_result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#replace"
    }

    fn arity(&self) -> Option<usize> {
        Some(4)
    }
}

/// Advanced regex replace with flags
pub struct RegexReplaceBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl RegexReplaceBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for RegexReplaceBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 4 || args.len() > 5 {
            return Err(Error::reasoning(
                "regexReplace expects 4 or 5 arguments (result, string, pattern, replacement, [case_insensitive])",
            ));
        }

        let expected_result = extract_string(&args[0])?;
        let text = extract_string(&args[1])?;
        let pattern = extract_string(&args[2])?;
        let replacement = extract_string(&args[3])?;
        let case_insensitive = extract_optional_bool(args.get(4))?;

        let regex = get_or_compile_regex(&self.cache, pattern, case_insensitive)?;
        let actual_result = regex.replace_all(text, replacement);

        Ok(SWRLValue::Boolean(*expected_result == actual_result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#regexReplace"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (4-5)
    }
}

/// Tokenize string using regex pattern
pub struct TokenizeBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl TokenizeBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for TokenizeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 || args.len() > 3 {
            return Err(Error::reasoning(
                "tokenize expects 2 or 3 arguments (string, pattern, [case_insensitive])",
            ));
        }

        let text = extract_string(&args[0])?;
        let pattern = extract_string(&args[1])?;
        let case_insensitive = extract_optional_bool(args.get(2))?;

        let regex = get_or_compile_regex(&self.cache, pattern, case_insensitive)?;

        // Find all matches and return as comma-separated string
        let matches: Vec<&str> = regex.find_iter(text).map(|m| m.as_str()).collect();
        let result = matches.join(",");

        Ok(SWRLValue::String(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#tokenize"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (2-3)
    }
}

/// Split string using regex pattern as delimiter
pub struct SplitBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl SplitBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for SplitBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 || args.len() > 3 {
            return Err(Error::reasoning(
                "split expects 2 or 3 arguments (string, delimiter_pattern, [case_insensitive])",
            ));
        }

        let text = extract_string(&args[0])?;
        let delimiter_pattern = extract_string(&args[1])?;
        let case_insensitive = extract_optional_bool(args.get(2))?;

        let regex = get_or_compile_regex(&self.cache, delimiter_pattern, case_insensitive)?;

        // Split text and return as comma-separated string
        let parts: Vec<&str> = regex.split(text).collect();
        let result = parts.join(",");

        Ok(SWRLValue::String(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#split"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (2-3)
    }
}

/// Extract first match from string
pub struct ExtractBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl ExtractBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for ExtractBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 || args.len() > 3 {
            return Err(Error::reasoning(
                "extract expects 2 or 3 arguments (string, pattern, [case_insensitive])",
            ));
        }

        let text = extract_string(&args[0])?;
        let pattern = extract_string(&args[1])?;
        let case_insensitive = extract_optional_bool(args.get(2))?;

        let regex = get_or_compile_regex(&self.cache, pattern, case_insensitive)?;

        // Find first match
        if let Some(match_result) = regex.find(text) {
            Ok(SWRLValue::String(match_result.as_str().to_string()))
        } else {
            Ok(SWRLValue::String(String::new())) // Empty string if no match
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#extract"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (2-3)
    }
}

/// Extract all matches from string
pub struct ExtractAllBuiltIn {
    cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl ExtractAllBuiltIn {
    pub fn new(cache: Arc<Mutex<HashMap<String, Regex>>>) -> Self {
        Self { cache }
    }
}

impl SWRLBuiltIn for ExtractAllBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 || args.len() > 3 {
            return Err(Error::reasoning(
                "extractAll expects 2 or 3 arguments (string, pattern, [case_insensitive])",
            ));
        }

        let text = extract_string(&args[0])?;
        let pattern = extract_string(&args[1])?;
        let case_insensitive = extract_optional_bool(args.get(2))?;

        let regex = get_or_compile_regex(&self.cache, pattern, case_insensitive)?;

        // Find all matches and return as comma-separated string
        let matches: Vec<&str> = regex.find_iter(text).map(|m| m.as_str()).collect();
        let result = matches.join(",");

        Ok(SWRLValue::String(result))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#extractAll"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity (2-3)
    }
}

/// Validate regex pattern
pub struct IsValidPatternBuiltIn;

impl SWRLBuiltIn for IsValidPatternBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning(
                "isValidPattern expects exactly 1 argument (pattern)",
            ));
        }

        let pattern = extract_string(&args[0])?;

        // Try to compile the regex to check validity
        let is_valid = Regex::new(pattern).is_ok();
        Ok(SWRLValue::Boolean(is_valid))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#isValidPattern"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_builtin() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let builtin = MatchesBuiltIn::new(cache);

        // Test basic pattern matching
        let args = vec![
            SWRLValue::String("hello world".to_string()),
            SWRLValue::String(r"world".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));

        // Test no match
        let args = vec![
            SWRLValue::String("hello world".to_string()),
            SWRLValue::String(r"xyz".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(false));

        // Test case insensitive
        let args = vec![
            SWRLValue::String("Hello World".to_string()),
            SWRLValue::String(r"hello".to_string()),
            SWRLValue::Boolean(true),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_replace_builtin() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let builtin = ReplaceBuiltIn::new(cache);

        let args = vec![
            SWRLValue::String("hello universe".to_string()),
            SWRLValue::String("hello world".to_string()),
            SWRLValue::String(r"world".to_string()),
            SWRLValue::String("universe".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_tokenize_builtin() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let builtin = TokenizeBuiltIn::new(cache);

        let args = vec![
            SWRLValue::String("apple,banana,cherry".to_string()),
            SWRLValue::String(r"\w+".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::String("apple,banana,cherry".to_string()));
    }

    #[test]
    fn test_split_builtin() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let builtin = SplitBuiltIn::new(cache);

        let args = vec![
            SWRLValue::String("apple,banana,cherry".to_string()),
            SWRLValue::String(r",".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::String("apple,banana,cherry".to_string()));
    }

    #[test]
    fn test_extract_builtin() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let builtin = ExtractBuiltIn::new(cache);

        let args = vec![
            SWRLValue::String("Contact: john@example.com".to_string()),
            SWRLValue::String(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string()),
        ];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::String("john@example.com".to_string()));
    }

    #[test]
    fn test_is_valid_pattern() {
        let builtin = IsValidPatternBuiltIn;

        // Valid pattern
        let args = vec![SWRLValue::String(r"\d+".to_string())];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));

        // Invalid pattern
        let args = vec![SWRLValue::String(r"[".to_string())];
        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(false));
    }
}
