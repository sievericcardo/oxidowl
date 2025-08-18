//! Date/Time Built-ins Tests
//!
//! Tests for SWRL date/time functionality

use oxidowl::swrl::datetime_builtins::{DateTimeBuiltInRegistry, DateTimeEqualBuiltIn, YearFromDateTimeBuiltIn};
use oxidowl::swrl::temporal::TemporalValue;
use oxidowl::swrl::SWRLValue;
use oxidowl::ontology::{Literal, IRI};
use chrono::{NaiveDate, Datelike};

#[cfg(test)]
mod tests {
    use super::*;
    use oxidowl::swrl::SWRLBuiltIn;

    #[test]
    fn test_datetime_built_in_registry() {
        let registry = DateTimeBuiltInRegistry::new();
        
        // Check that some built-ins are registered
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#dateTimeEqual").is_some());
        assert!(registry.get_builtin("http://www.w3.org/2003/11/swrlb#yearFromDateTime").is_some());
        
        // Check IRI list
        let iris = registry.get_builtin_iris();
        assert!(iris.contains(&"http://www.w3.org/2003/11/swrlb#dateTimeEqual".to_string()));
        assert!(iris.contains(&"http://www.w3.org/2003/11/swrlb#yearFromDateTime".to_string()));
    }

    #[test]
    fn test_temporal_value_creation() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let temporal_value = TemporalValue::Date(date);
        
        assert_eq!(temporal_value.year(), Some(2023));
        assert_eq!(temporal_value.month(), Some(12));
        assert_eq!(temporal_value.day(), Some(25));
    }

    #[test]
    fn test_temporal_value_from_literal() {
        // Create a date literal
        let date_iri = IRI::new("http://www.w3.org/2001/XMLSchema#date");
        let literal = Literal::with_datatype("2023-12-25".to_string(), date_iri);
        let temporal_value = TemporalValue::from_literal(&literal).unwrap();
        
        match temporal_value {
            TemporalValue::Date(date) => {
                assert_eq!(date.year(), 2023);
                assert_eq!(date.month(), 12);
                assert_eq!(date.day(), 25);
            },
            _ => panic!("Expected date temporal value"),
        }
    }

    #[test]
    fn test_datetime_equal_builtin() {
        let builtin = DateTimeEqualBuiltIn;
        
        let dt1 = SWRLValue::Literal(Literal {
            value: "2023-12-25T10:30:00".to_string(),
            datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
            language: None,
        });
        
        let dt2 = SWRLValue::Literal(Literal {
            value: "2023-12-25T10:30:00".to_string(),
            datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
            language: None,
        });
        
        let result = builtin.execute(&[dt1, dt2]).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_year_from_datetime_builtin() {
        let builtin = YearFromDateTimeBuiltIn;
        
        let year_result = SWRLValue::Literal(Literal {
            value: "2023".to_string(),
            datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#integer").unwrap()),
            language: None,
        });
        
        let datetime = SWRLValue::Literal(Literal {
            value: "2023-12-25T10:30:00".to_string(),
            datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#dateTime").unwrap()),
            language: None,
        });
        
        let result = builtin.execute(&[year_result, datetime]).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_temporal_value_parsing() {
        // Test various datetime formats
        let formats = vec![
            ("2023-12-25", "http://www.w3.org/2001/XMLSchema#date"),
            ("2023-12-25T10:30:00", "http://www.w3.org/2001/XMLSchema#dateTime"),
            ("10:30:00", "http://www.w3.org/2001/XMLSchema#time"),
        ];
        
        for (value, datatype_iri) in formats {
            let literal = Literal {
                value: value.to_string(),
                datatype: Some(url::Url::parse(datatype_iri).unwrap()),
                language: None,
            };
            
            let temporal_value = TemporalValue::from_literal(&literal);
            assert!(temporal_value.is_ok(), "Failed to parse {} as {}", value, datatype_iri);
        }
    }

    #[test]
    fn test_invalid_temporal_values() {
        let invalid_literals = vec![
            Literal {
                value: "not-a-date".to_string(),
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#date").unwrap()),
                language: None,
            },
            Literal {
                value: "2023-13-01".to_string(), // Invalid month
                datatype: Some(url::Url::parse("http://www.w3.org/2001/XMLSchema#date").unwrap()),
                language: None,
            },
        ];
        
        for literal in invalid_literals {
            let result = TemporalValue::from_literal(&literal);
            assert!(result.is_err(), "Should fail to parse invalid temporal value: {}", literal.value);
        }
    }
}
