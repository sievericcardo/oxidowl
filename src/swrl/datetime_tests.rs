use crate::swrl::datetime_builtins::{DateTimeBuiltInRegistry};
use crate::swrl::temporal::TemporalValue;
use crate::swrl::SWRLValue;
use crate::ontology::{Literal, IRI};
use chrono::{NaiveDate, Datelike};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::SWRLBuiltIn;

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
        
        // Create two identical date literals
        let date_iri = IRI::new("http://www.w3.org/2001/XMLSchema#date");
        let literal1 = Literal::with_datatype("2023-12-25".to_string(), date_iri.clone());
        let literal2 = Literal::with_datatype("2023-12-25".to_string(), date_iri);
        
        let args = vec![
            SWRLValue::Literal(literal1),
            SWRLValue::Literal(literal2),
        ];
        
        let result = builtin.execute(&args).unwrap();
        match result {
            SWRLValue::Boolean(true) => {}, // Expected
            _ => panic!("Expected true result for equal dates"),
        }
    }

    #[test]
    fn test_year_from_datetime_builtin() {
        let builtin = YearFromDateTimeBuiltIn;
        
        // Create a datetime literal
        let datetime_iri = IRI::new("http://www.w3.org/2001/XMLSchema#dateTime");
        let integer_iri = IRI::new("http://www.w3.org/2001/XMLSchema#integer");
        
        let datetime_literal = Literal::with_datatype("2023-12-25T14:30:00".to_string(), datetime_iri);
        let year_literal = Literal::with_datatype("2023".to_string(), integer_iri);
        
        let args = vec![
            SWRLValue::Literal(year_literal),
            SWRLValue::Literal(datetime_literal),
        ];
        
        let result = builtin.execute(&args).unwrap();
        match result {
            SWRLValue::Boolean(true) => {}, // Expected
            _ => panic!("Expected true result for year extraction"),
        }
    }

    #[test]
    fn test_builtin_arity() {
        let datetime_equal = DateTimeEqualBuiltIn;
        assert_eq!(datetime_equal.arity(), Some(2));
        
        let year_from_datetime = YearFromDateTimeBuiltIn;
        assert_eq!(year_from_datetime.arity(), Some(2));
    }

    #[test]
    fn test_builtin_names() {
        let datetime_equal = DateTimeEqualBuiltIn;
        assert_eq!(datetime_equal.name(), "http://www.w3.org/2003/11/swrlb#dateTimeEqual");
        
        let year_from_datetime = YearFromDateTimeBuiltIn;
        assert_eq!(year_from_datetime.name(), "http://www.w3.org/2003/11/swrlb#yearFromDateTime");
    }
}
