//! Comprehensive tests for all implemented SWRL features
//!
//! This module provides tests for the complete SWRL implementation including:
//! - Phase 2 date/time built-ins
//! - Regex built-ins  
//! - Missing built-ins
//! - Feature integration

use crate::swrl::{
    SWRLValue,
    datetime_builtins::DateTimeBuiltInRegistry,
    regex_builtins::RegexBuiltInRegistry,
    integration::SWRLFeatureRegistry,
    temporal::TemporalValue,
};
use crate::ontology::Literal;

#[cfg(test)]
mod comprehensive_tests {
    use super::*;

    #[test]
    fn test_phase2_datetime_builtins() {
        let registry = DateTimeBuiltInRegistry::new();
        
        // Test that Phase 2 built-ins are registered
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#dateTimeLessThan").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#monthFromDateTime").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#dayFromDateTime").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#hourFromDateTime").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#dateEqual").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#timeEqual").is_some());
        
        // Count check - should have significantly more than Phase 1 (3 built-ins)
        let builtin_count = registry.get_builtin_iris().len();
        assert!(builtin_count >= 15, "Expected at least 15 datetime built-ins, got {}", builtin_count);
    }

    #[test]
    fn test_datetime_comparison_operations() {
        let registry = DateTimeBuiltInRegistry::new();
        
        // Test dateTimeLessThan
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#dateTimeLessThan") {
            let dt1 = SWRLValue::Literal(Literal {
                value: "2023-01-01T10:00:00".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#dateTime".to_string()),
                language: None,
            });
            let dt2 = SWRLValue::Literal(Literal {
                value: "2023-01-01T11:00:00".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#dateTime".to_string()),
                language: None,
            });
            
            let result = builtin.execute(&[dt1, dt2]);
            assert!(result.is_ok(), "dateTimeLessThan should execute successfully");
        }
    }

    #[test] 
    fn test_datetime_component_extraction() {
        let registry = DateTimeBuiltInRegistry::new();
        
        // Test monthFromDateTime
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#monthFromDateTime") {
            let expected_month = SWRLValue::Literal(Literal {
                value: "6".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
                language: None,
            });
            let datetime = SWRLValue::Literal(Literal {
                value: "2023-06-15T14:30:00".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#dateTime".to_string()),
                language: None,
            });
            
            let result = builtin.execute(&[expected_month, datetime]);
            assert!(result.is_ok(), "monthFromDateTime should execute successfully");
        }
    }

    #[test]
    fn test_regex_builtins_registry() {
        let registry = RegexBuiltInRegistry::new();
        
        // Test that all major regex built-ins are registered
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#matches").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#replace").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#regexReplace").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#tokenize").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#split").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#extract").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#extractAll").is_some());
        assert!(registry.get("http://www.w3.org/2003/11/swrlb#isValidPattern").is_some());
        
        // Should have 8 regex built-ins as planned
        assert_eq!(registry.count(), 8);
    }

    #[test]
    fn test_regex_pattern_matching() {
        let registry = RegexBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get("http://www.w3.org/2003/11/swrlb#matches") {
            // Test email pattern matching
            let args = vec![
                SWRLValue::String("contact@example.com".to_string()),
                SWRLValue::String(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string()),
            ];
            let result = builtin.execute(&args).unwrap();
            assert_eq!(result, SWRLValue::Boolean(true));
            
            // Test non-matching pattern
            let args = vec![
                SWRLValue::String("not-an-email".to_string()),
                SWRLValue::String(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string()),
            ];
            let result = builtin.execute(&args).unwrap();
            assert_eq!(result, SWRLValue::Boolean(false));
        }
    }

    #[test]
    fn test_regex_replacement() {
        let registry = RegexBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get("http://www.w3.org/2003/11/swrlb#replace") {
            let args = vec![
                SWRLValue::String("Hello Universe".to_string()),
                SWRLValue::String("Hello World".to_string()),
                SWRLValue::String(r"World".to_string()),
                SWRLValue::String("Universe".to_string()),
            ];
            let result = builtin.execute(&args).unwrap();
            assert_eq!(result, SWRLValue::Boolean(true));
        }
    }

    #[test]
    fn test_regex_tokenization() {
        let registry = RegexBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get("http://www.w3.org/2003/11/swrlb#tokenize") {
            let args = vec![
                SWRLValue::String("The quick brown fox".to_string()),
                SWRLValue::String(r"\w+".to_string()),
            ];
            let result = builtin.execute(&args).unwrap();
            if let SWRLValue::String(tokens) = result {
                assert!(tokens.contains("The"));
                assert!(tokens.contains("quick"));
                assert!(tokens.contains("brown"));
                assert!(tokens.contains("fox"));
            } else {
                panic!("Expected string result from tokenize");
            }
        }
    }

    #[test]
    fn test_regex_extraction() {
        let registry = RegexBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get("http://www.w3.org/2003/11/swrlb#extract") {
            let args = vec![
                SWRLValue::String("Contact me at john@example.com or jane@test.org".to_string()),
                SWRLValue::String(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string()),
            ];
            let result = builtin.execute(&args).unwrap();
            if let SWRLValue::String(email) = result {
                assert_eq!(email, "john@example.com");
            } else {
                panic!("Expected string result from extract");
            }
        }
    }

    #[test]
    fn test_regex_validation() {
        let registry = RegexBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get("http://www.w3.org/2003/11/swrlb#isValidPattern") {
            // Test valid pattern
            let args = vec![SWRLValue::String(r"\d+".to_string())];
            let result = builtin.execute(&args).unwrap();
            assert_eq!(result, SWRLValue::Boolean(true));
            
            // Test invalid pattern
            let args = vec![SWRLValue::String(r"[".to_string())];
            let result = builtin.execute(&args).unwrap();
            assert_eq!(result, SWRLValue::Boolean(false));
        }
    }

    #[test]
    fn test_comprehensive_feature_integration() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test that all major categories are supported
        let categories = registry.get_builtins_by_category();
        assert!(categories.contains_key("datetime"));
        assert!(categories.contains_key("regex"));
        assert!(categories.contains_key("math"));
        assert!(categories.contains_key("string"));
        assert!(categories.contains_key("boolean"));
        
        // Test statistics
        let stats = registry.get_statistics();
        assert!(stats.total_builtins >= 30, "Expected at least 30 total built-ins, got {}", stats.total_builtins);
        assert!(stats.datetime_builtins >= 15, "Expected at least 15 datetime built-ins, got {}", stats.datetime_builtins);
        assert_eq!(stats.regex_builtins, 8, "Expected exactly 8 regex built-ins, got {}", stats.regex_builtins);
        assert!(stats.feature_coverage >= 50.0, "Expected at least 50% feature coverage, got {:.1}%", stats.feature_coverage);
    }

    #[test]
    fn test_integrated_execution() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test datetime built-in execution through integration
        let datetime_result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeEqual",
            &[
                SWRLValue::String("2023-01-01T00:00:00".to_string()),
                SWRLValue::String("2023-01-01T00:00:00".to_string()),
            ]
        );
        assert!(datetime_result.is_ok());
        
        // Test regex built-in execution through integration
        let regex_result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#matches",
            &[
                SWRLValue::String("test123".to_string()),
                SWRLValue::String(r"\d+".to_string()),
            ]
        );
        assert!(regex_result.is_ok());
        assert_eq!(regex_result.unwrap(), SWRLValue::Boolean(true));
        
        // Test missing built-in execution through integration
        let boolean_result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#booleanNot",
            &[
                SWRLValue::Boolean(false),
                SWRLValue::Boolean(true),
            ]
        );
        assert!(boolean_result.is_ok());
        assert_eq!(boolean_result.unwrap(), SWRLValue::Boolean(true));
    }

    #[test]
    fn test_builtin_validation() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test validation of supported built-in
        let args = vec![SWRLValue::String("test".to_string()), SWRLValue::String("pattern".to_string())];
        let result = registry.validate_builtin_call("http://www.w3.org/2003/11/swrlb#matches", &args).unwrap();
        assert!(result.valid);
        
        // Test validation of unsupported built-in
        let result = registry.validate_builtin_call("http://example.org/unsupported", &args).unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_arity_checking() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test built-in with known fixed arity
        let arity = registry.get_builtin_arity("http://www.w3.org/2003/11/swrlb#isValidPattern");
        assert_eq!(arity, Some(1));
        
        // Test built-in with variable arity
        let arity = registry.get_builtin_arity("http://www.w3.org/2003/11/swrlb#matches");
        assert!(arity.is_none() || arity == Some(2) || arity == Some(3));
    }

    #[test]
    fn test_feature_completeness() {
        let registry = SWRLFeatureRegistry::new();
        let all_iris = registry.get_all_builtin_iris();
        
        // Verify key built-ins from each category are present
        let key_builtins = vec![
            // Datetime built-ins
            "http://www.w3.org/2003/11/swrlb#dateTimeEqual",
            "http://www.w3.org/2003/11/swrlb#dateTimeLessThan",
            "http://www.w3.org/2003/11/swrlb#yearFromDateTime",
            "http://www.w3.org/2003/11/swrlb#monthFromDateTime",
            "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDateTime",
            
            // Regex built-ins
            "http://www.w3.org/2003/11/swrlb#matches",
            "http://www.w3.org/2003/11/swrlb#replace",
            "http://www.w3.org/2003/11/swrlb#tokenize",
            "http://www.w3.org/2003/11/swrlb#extract",
            
            // Missing built-ins
            "http://www.w3.org/2003/11/swrlb#booleanNot",
            "http://www.w3.org/2003/11/swrlb#ceiling",
            "http://www.w3.org/2003/11/swrlb#floor",
            "http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase",
        ];
        
        for builtin in key_builtins {
            assert!(all_iris.contains(&builtin.to_string()), 
                   "Missing key built-in: {}", builtin);
        }
        
        println!("✓ Feature implementation complete with {} built-ins", all_iris.len());
        
        // Print summary for verification
        let stats = registry.get_statistics();
        stats.print_summary();
    }

    #[test]
    fn test_performance_characteristics() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test regex caching by calling same pattern multiple times
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = registry.execute_builtin(
                "http://www.w3.org/2003/11/swrlb#matches",
                &[
                    SWRLValue::String("test@example.com".to_string()),
                    SWRLValue::String(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string()),
                ]
            );
        }
        let duration = start.elapsed();
        
        // Should complete quickly due to regex caching
        assert!(duration.as_millis() < 100, "Regex operations too slow: {}ms", duration.as_millis());
        
        // Test cache clearing
        registry.clear_regex_cache();
    }

    #[test]
    fn test_error_handling() {
        let registry = SWRLFeatureRegistry::new();
        
        // Test invalid regex pattern
        let result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#matches",
            &[
                SWRLValue::String("test".to_string()),
                SWRLValue::String("[invalid".to_string()),
            ]
        );
        assert!(result.is_err());
        
        // Test wrong argument types
        let result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#matches",
            &[
                SWRLValue::Integer(123),
                SWRLValue::String("pattern".to_string()),
            ]
        );
        assert!(result.is_err());
        
        // Test wrong arity
        let result = registry.execute_builtin(
            "http://www.w3.org/2003/11/swrlb#isValidPattern",
            &[
                SWRLValue::String("pattern1".to_string()),
                SWRLValue::String("pattern2".to_string()),
            ]
        );
        assert!(result.is_err());
    }
}
