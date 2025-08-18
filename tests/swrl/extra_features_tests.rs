//! Tests for the newly implemented missing SWRL features
//! 
//! Tests the timezone, addYearMonthDuration, and addYearMonthDurations built-ins

use oxidowl::swrl::{SWRLValue, datetime_builtins::DateTimeBuiltInRegistry};
use oxidowl::ontology::Literal;

#[cfg(test)]
mod missing_features_tests {
    use super::*;

    #[test]
    fn test_timezone_builtin() {
        let registry = DateTimeBuiltInRegistry::new();
        
        // Test timezone extraction built-in
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#timezone") {
            // Test UTC timezone 
            let result_var = SWRLValue::String("result".to_string());
            let dt_utc = SWRLValue::Literal(Literal {
                value: "2023-01-01T10:00:00Z".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
                language: None,
            });
            
            let result = builtin.execute(&[result_var, dt_utc]).expect("timezone execution should succeed");
            
            match result {
                SWRLValue::Literal(literal) => {
                    assert_eq!(literal.value, "Z");
                    assert_eq!(literal.datatype, Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap()));
                }
                _ => panic!("Expected literal result")
            }
            
            // Test positive timezone offset
            let result_var2 = SWRLValue::String("result2".to_string());
            let dt_plus5 = SWRLValue::Literal(Literal {
                value: "2023-01-01T10:00:00+05:00".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
                language: None,
            });
            
            let result2 = builtin.execute(&[result_var2, dt_plus5]).expect("timezone execution should succeed");
            
            match result2 {
                SWRLValue::Literal(literal) => {
                    assert_eq!(literal.value, "+05:00");
                }
                _ => panic!("Expected literal result")
            }
            
            // Test negative timezone offset
            let result_var3 = SWRLValue::String("result3".to_string());
            let dt_minus3 = SWRLValue::Literal(Literal {
                value: "2023-01-01T10:00:00-03:00".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
                language: None,
            });
            
            let result3 = builtin.execute(&[result_var3, dt_minus3]).expect("timezone execution should succeed");
            
            match result3 {
                SWRLValue::Literal(literal) => {
                    assert_eq!(literal.value, "-03:00");
                }
                _ => panic!("Expected literal result")
            }
        } else {
            panic!("timezone built-in not found in registry");
        }
    }

    #[test]
    fn test_add_year_month_duration_builtin() {
        let registry = DateTimeBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#addYearMonthDuration") {
            // Test adding 2 years and 3 months to a date
            let result_var = SWRLValue::String("result".to_string());
            let base_date = SWRLValue::Literal(Literal {
                value: "2023-01-15".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#date").unwrap()),
                language: None,
            });
            let duration = SWRLValue::Literal(Literal {
                value: "P2Y3M".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration").unwrap()),
                language: None,
            });
            
            let result = builtin.execute(&[result_var, base_date, duration]).expect("addYearMonthDuration execution should succeed");
            
            match result {
                SWRLValue::Literal(literal) => {
                    // Should be 2025-04-15
                    assert_eq!(literal.value, "2025-04-15");
                    assert_eq!(literal.datatype, Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#date").unwrap()));
                }
                _ => panic!("Expected literal result")
            }
        } else {
            panic!("addYearMonthDuration built-in not found in registry");
        }
    }

    #[test]
    fn test_add_year_month_durations_builtin() {
        let registry = DateTimeBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#addYearMonthDurations") {
            // Test adding two year-month durations
            let result_var = SWRLValue::String("result".to_string());
            let duration1 = SWRLValue::Literal(Literal {
                value: "P1Y6M".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration").unwrap()),
                language: None,
            });
            let duration2 = SWRLValue::Literal(Literal {
                value: "P2Y3M".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration").unwrap()),
                language: None,
            });
            
            let result = builtin.execute(&[result_var, duration1, duration2]).expect("addYearMonthDurations execution should succeed");
            
            match result {
                SWRLValue::Literal(literal) => {
                    // Should be P3Y9M (1 year 6 months + 2 years 3 months)
                    assert_eq!(literal.value, "P3Y9M");
                    assert_eq!(literal.datatype, Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration").unwrap()));
                }
                _ => panic!("Expected literal result")
            }
        } else {
            panic!("addYearMonthDurations built-in not found in registry");
        }
    }

    #[test]
    fn test_timezone_edge_cases() {
        let registry = DateTimeBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#timezone") {
            // Test datetime without timezone
            let result_var = SWRLValue::String("result".to_string());
            let dt_no_tz = SWRLValue::Literal(Literal {
                value: "2023-01-01T10:00:00".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
                language: None,
            });
            
            let result = builtin.execute(&[result_var, dt_no_tz]).expect("timezone execution should succeed");
            
            match result {
                SWRLValue::Literal(literal) => {
                    // Should return empty string for no timezone
                    assert_eq!(literal.value, "");
                }
                _ => panic!("Expected literal result")
            }
        }
    }

    #[test]
    fn test_duration_edge_cases() {
        let registry = DateTimeBuiltInRegistry::new();
        
        if let Some(builtin) = registry.get_builtin("http://www.w3.org/2003/11/swrlb#addYearMonthDuration") {
            // Test zero duration
            let result_var = SWRLValue::String("result".to_string());
            let base_date = SWRLValue::Literal(Literal {
                value: "2023-06-15".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#date").unwrap()),
                language: None,
            });
            let zero_duration = SWRLValue::Literal(Literal {
                value: "P0Y0M".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration").unwrap()),
                language: None,
            });
            
            let result = builtin.execute(&[result_var, base_date, zero_duration]).expect("zero duration execution should succeed");
            
            match result {
                SWRLValue::Literal(literal) => {
                    // Should return the same date
                    assert_eq!(literal.value, "2023-06-15");
                }
                _ => panic!("Expected literal result")
            }
        }
    }
}
