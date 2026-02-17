//! SWRL Features Integration Module
//!
//! This module provides a unified interface for all implemented SWRL features,
//! integrating date/time built-ins, regex support, and missing built-ins
//! into a comprehensive SWRL processing system.

use crate::ontology::IRI;
use crate::swrl::{
    builtins::{SWRLBuiltInRegistry, SWRLValue},
    datetime_builtins::DateTimeBuiltInRegistry,
    extended_builtins::register_extended_builtins,
    regex_builtins::RegexBuiltInRegistry,
};
use crate::{Error, Result};
use std::collections::HashMap;

/// Comprehensive SWRL feature registry
pub struct SWRLFeatureRegistry {
    /// Main built-in registry
    main_registry: SWRLBuiltInRegistry,
    /// Date/time built-ins registry
    datetime_registry: DateTimeBuiltInRegistry,
    /// Regex built-ins registry
    regex_registry: RegexBuiltInRegistry,
    /// Feature status tracking
    feature_status: HashMap<String, bool>,
}

impl SWRLFeatureRegistry {
    /// Create a new comprehensive SWRL feature registry
    #[must_use] 
    pub fn new() -> Self {
        let mut main_registry = SWRLBuiltInRegistry::new();
        let datetime_registry = DateTimeBuiltInRegistry::new();
        let regex_registry = RegexBuiltInRegistry::new();

        // Register extended built-ins to main registry
        register_extended_builtins(&mut main_registry);

        let mut feature_status = HashMap::new();
        feature_status.insert("core_builtins".to_string(), true);
        feature_status.insert("datetime_builtins".to_string(), true);
        feature_status.insert("regex_builtins".to_string(), true);
        feature_status.insert("extended_builtins".to_string(), true);

        Self {
            main_registry,
            datetime_registry,
            regex_registry,
            feature_status,
        }
    }

    /// Execute a built-in predicate
    pub fn execute_builtin(&self, iri: &str, args: &[SWRLValue]) -> Result<SWRLValue> {
        // First try datetime built-ins
        if let Some(builtin) = self.datetime_registry.get_builtin(iri) {
            return builtin.execute(args);
        }

        // Then try regex built-ins
        if let Some(builtin) = self.regex_registry.get(iri) {
            return builtin.execute(args);
        }

        // Finally try main registry (core + missing built-ins)
        if let Some(builtin) = self.main_registry.get_builtin(&IRI::new(iri)) {
            return builtin.execute(args);
        }

        Err(Error::reasoning(format!(
            "Unknown built-in predicate: {iri}"
        )))
    }

    /// Check if a built-in is supported
    #[must_use] 
    pub fn is_builtin_supported(&self, iri: &str) -> bool {
        self.datetime_registry.get_builtin(iri).is_some()
            || self.regex_registry.get(iri).is_some()
            || self.main_registry.get_builtin(&IRI::new(iri)).is_some()
    }

    /// Get all supported built-in IRIs
    #[must_use] 
    pub fn get_all_builtin_iris(&self) -> Vec<String> {
        let mut iris = Vec::new();

        // Add datetime built-ins
        iris.extend(self.datetime_registry.get_builtin_iris());

        // Add regex built-ins
        iris.extend(self.regex_registry.get_all_iris());

        // Add main registry built-ins
        iris.extend(
            self.main_registry
                .get_builtin_iris()
                .into_iter()
                .map(|iri| iri.to_string()),
        );

        iris.sort();
        iris.dedup();
        iris
    }

    /// Get built-ins by category
    #[must_use] 
    pub fn get_builtins_by_category(&self) -> HashMap<String, Vec<String>> {
        let mut categories = HashMap::new();

        // Date/time built-ins
        categories.insert(
            "datetime".to_string(),
            self.datetime_registry.get_builtin_iris(),
        );

        // Regex built-ins
        categories.insert("regex".to_string(), self.regex_registry.get_all_iris());

        // Core and missing built-ins (categorize them)
        let main_iris: Vec<String> = self
            .main_registry
            .get_builtin_iris()
            .into_iter()
            .map(|iri| iri.to_string())
            .collect();

        let mut core_builtins = Vec::new();
        let mut math_builtins = Vec::new();
        let mut string_builtins = Vec::new();
        let mut comparison_builtins = Vec::new();
        let mut list_builtins = Vec::new();
        let mut uri_builtins = Vec::new();
        let mut boolean_builtins = Vec::new();

        for iri in main_iris {
            if iri.contains("equal")
                || iri.contains("notEqual")
                || iri.contains("lessThan")
                || iri.contains("greaterThan")
            {
                comparison_builtins.push(iri);
            } else if iri.contains("add")
                || iri.contains("subtract")
                || iri.contains("multiply")
                || iri.contains("divide")
                || iri.contains("mod")
                || iri.contains("abs")
                || iri.contains("ceiling")
                || iri.contains("floor")
                || iri.contains("round")
                || iri.contains("sin")
                || iri.contains("cos")
                || iri.contains("tan")
                || iri.contains("unary")
            {
                math_builtins.push(iri);
            } else if iri.contains("string")
                || iri.contains("String")
                || iri.contains("contains")
                || iri.contains("startsWith")
                || iri.contains("endsWith")
                || iri.contains("substring")
                || iri.contains("length")
                || iri.contains("normalize")
            {
                string_builtins.push(iri);
            } else if iri.contains("list")
                || iri.contains("List")
                || iri.contains("member")
                || iri.contains("concat")
            {
                list_builtins.push(iri);
            } else if iri.contains("URI") || iri.contains("uri") || iri.contains("resolve") {
                uri_builtins.push(iri);
            } else if iri.contains("boolean") || iri.contains("Boolean") {
                boolean_builtins.push(iri);
            } else {
                core_builtins.push(iri);
            }
        }

        categories.insert("core".to_string(), core_builtins);
        categories.insert("math".to_string(), math_builtins);
        categories.insert("string".to_string(), string_builtins);
        categories.insert("comparison".to_string(), comparison_builtins);
        categories.insert("list".to_string(), list_builtins);
        categories.insert("uri".to_string(), uri_builtins);
        categories.insert("boolean".to_string(), boolean_builtins);

        categories
    }

    /// Get feature implementation status
    #[must_use] 
    pub fn get_feature_status(&self) -> &HashMap<String, bool> {
        &self.feature_status
    }

    /// Get statistics about implemented built-ins
    #[must_use] 
    pub fn get_statistics(&self) -> SWRLFeatureStatistics {
        let datetime_count = self.datetime_registry.get_builtin_iris().len();
        let regex_count = self.regex_registry.count();
        let main_count = self.main_registry.get_builtin_iris().len();

        let categories = self.get_builtins_by_category();

        SWRLFeatureStatistics {
            total_builtins: datetime_count + regex_count + main_count,
            datetime_builtins: datetime_count,
            regex_builtins: regex_count,
            core_builtins: categories.get("core").map_or(0, std::vec::Vec::len),
            math_builtins: categories.get("math").map_or(0, std::vec::Vec::len),
            string_builtins: categories.get("string").map_or(0, std::vec::Vec::len),
            comparison_builtins: categories.get("comparison").map_or(0, std::vec::Vec::len),
            list_builtins: categories.get("list").map_or(0, std::vec::Vec::len),
            uri_builtins: categories.get("uri").map_or(0, std::vec::Vec::len),
            boolean_builtins: categories.get("boolean").map_or(0, std::vec::Vec::len),
            feature_coverage: self.calculate_feature_coverage(),
        }
    }

    /// Calculate feature coverage percentage
    fn calculate_feature_coverage(&self) -> f64 {
        // Based on W3C SWRL built-ins specification
        // Estimated total: ~50-60 built-ins across all categories
        let estimated_total = 55.0;

        // Calculate total directly instead of calling get_statistics() to avoid infinite recursion
        let datetime_count = self.datetime_registry.get_builtin_iris().len();
        let regex_count = self.regex_registry.count();
        let main_count = self.main_registry.get_builtin_iris().len();
        let implemented = (datetime_count + regex_count + main_count) as f64;

        (implemented / estimated_total * 100.0).min(100.0)
    }

    /// Clear regex cache for memory management
    pub fn clear_regex_cache(&self) {
        self.regex_registry.clear_cache();
    }

    /// Validate a built-in call
    pub fn validate_builtin_call(&self, iri: &str, args: &[SWRLValue]) -> Result<ValidationResult> {
        if !self.is_builtin_supported(iri) {
            return Ok(ValidationResult {
                valid: false,
                errors: vec![format!("Unsupported built-in: {}", iri)],
                warnings: vec![],
            });
        }

        // Check arity if known
        let expected_arity = self.get_builtin_arity(iri);
        let mut warnings = Vec::new();

        if let Some(arity) = expected_arity {
            if args.len() != arity {
                return Ok(ValidationResult {
                    valid: false,
                    errors: vec![format!(
                        "Built-in {} expects {} arguments, got {}",
                        iri,
                        arity,
                        args.len()
                    )],
                    warnings,
                });
            }
        } else {
            warnings.push(format!("Variable arity built-in: {iri}"));
        }

        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings,
        })
    }

    /// Get expected arity for a built-in
    #[must_use] 
    pub fn get_builtin_arity(&self, iri: &str) -> Option<usize> {
        if let Some(builtin) = self.datetime_registry.get_builtin(iri) {
            return builtin.arity();
        }

        if let Some(builtin) = self.regex_registry.get(iri) {
            return builtin.arity();
        }

        if let Some(builtin) = self.main_registry.get_builtin(&IRI::new(iri)) {
            return builtin.arity();
        }

        None
    }
}

impl Default for SWRLFeatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about SWRL feature implementation
#[derive(Debug, Clone)]
pub struct SWRLFeatureStatistics {
    pub total_builtins: usize,
    pub datetime_builtins: usize,
    pub regex_builtins: usize,
    pub core_builtins: usize,
    pub math_builtins: usize,
    pub string_builtins: usize,
    pub comparison_builtins: usize,
    pub list_builtins: usize,
    pub uri_builtins: usize,
    pub boolean_builtins: usize,
    pub feature_coverage: f64,
}

impl SWRLFeatureStatistics {
    /// Print formatted statistics
    pub fn print_summary(&self) {
        println!("SWRL Feature Implementation Statistics:");
        println!("  Total Built-ins: {}", self.total_builtins);
        println!("  Feature Coverage: {:.1}%", self.feature_coverage);
        println!();
        println!("Built-ins by Category:");
        println!("  Date/Time: {}", self.datetime_builtins);
        println!("  Regex: {}", self.regex_builtins);
        println!("  Math: {}", self.math_builtins);
        println!("  String: {}", self.string_builtins);
        println!("  Comparison: {}", self.comparison_builtins);
        println!("  Boolean: {}", self.boolean_builtins);
        println!("  List: {}", self.list_builtins);
        println!("  URI: {}", self.uri_builtins);
        println!("  Core: {}", self.core_builtins);
    }
}

/// Built-in validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// SWRL feature integration tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_registry_creation() {
        let registry = SWRLFeatureRegistry::new();
        let stats = registry.get_statistics();

        assert!(stats.total_builtins > 0);
        assert!(stats.datetime_builtins > 0);
        assert!(stats.regex_builtins > 0);
        assert!(stats.feature_coverage > 0.0);
    }

    #[test]
    fn test_builtin_support_check() {
        let registry = SWRLFeatureRegistry::new();

        // Test datetime built-in
        assert!(registry.is_builtin_supported("http://www.w3.org/2003/11/swrlb#dateTimeEqual"));

        // Test regex built-in
        assert!(registry.is_builtin_supported("http://www.w3.org/2003/11/swrlb#matches"));

        // Test missing built-in
        assert!(registry.is_builtin_supported("http://www.w3.org/2003/11/swrlb#booleanNot"));

        // Test unsupported built-in
        assert!(!registry.is_builtin_supported("http://example.org/unsupported"));
    }

    #[test]
    fn test_categorization() {
        let registry = SWRLFeatureRegistry::new();
        let categories = registry.get_builtins_by_category();

        assert!(categories.contains_key("datetime"));
        assert!(categories.contains_key("regex"));
        assert!(categories.contains_key("math"));
        assert!(categories.contains_key("string"));

        // Check that datetime category has expected built-ins
        let datetime_builtins = categories
            .get("datetime")
            .expect("Failed to get datetime category from SWRL builtin registry");
        assert!(
            datetime_builtins
                .contains(&"http://www.w3.org/2003/11/swrlb#dateTimeEqual".to_string())
        );
    }

    #[test]
    fn test_validation() {
        let registry = SWRLFeatureRegistry::new();

        // Test valid call
        let args = vec![
            SWRLValue::String("test".to_string()),
            SWRLValue::String("pattern".to_string()),
        ];
        let result = registry
            .validate_builtin_call("http://www.w3.org/2003/11/swrlb#matches", &args)
            .expect("Failed to validate SWRL builtin call for matches");
        assert!(result.valid);

        // Test unsupported built-in
        let result = registry
            .validate_builtin_call("http://example.org/unsupported", &args)
            .expect("Failed to validate SWRL builtin call for unsupported builtin");
        assert!(!result.valid);
    }

    #[test]
    fn test_datetime_execution() {
        let registry = SWRLFeatureRegistry::new();

        // Test datetime equality
        let args = vec![
            SWRLValue::String("2023-01-01T00:00:00".to_string()),
            SWRLValue::String("2023-01-01T00:00:00".to_string()),
        ];

        // This should not fail (though specific result depends on temporal module implementation)
        let result =
            registry.execute_builtin("http://www.w3.org/2003/11/swrlb#dateTimeEqual", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_execution() {
        let registry = SWRLFeatureRegistry::new();

        // Test regex matching
        let args = vec![
            SWRLValue::String("hello world".to_string()),
            SWRLValue::String("world".to_string()),
        ];

        let result = registry
            .execute_builtin("http://www.w3.org/2003/11/swrlb#matches", &args)
            .expect("Failed to execute SWRL matches builtin");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_extended_builtins_execution() {
        let registry = SWRLFeatureRegistry::new();

        // Test boolean not
        let args = vec![SWRLValue::Boolean(false), SWRLValue::Boolean(true)];

        let result = registry
            .execute_builtin("http://www.w3.org/2003/11/swrlb#booleanNot", &args)
            .expect("Failed to execute SWRL booleanNot builtin");
        assert_eq!(result, SWRLValue::Boolean(true));
    }
}
