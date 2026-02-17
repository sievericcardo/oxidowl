//! Date/Time Constructor Built-in Predicates
//!
//! This module implements constructor built-ins for creating date, time, and duration values.

#![allow(dead_code)]

use crate::error::{Error, Result};
use crate::ontology::{IRI, Literal};
use crate::swrl::{SWRLBuiltIn, SWRLValue};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::collections::HashMap;

/// Registry for date/time constructor built-ins
pub struct DateTimeConstructorRegistry {
    builtins: HashMap<String, Box<dyn SWRLBuiltIn>>,
}

impl DateTimeConstructorRegistry {
    /// Create a new registry with all constructor built-ins
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            builtins: HashMap::new(),
        };

        // Date constructors
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#date",
            Box::new(DateBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTime",
            Box::new(DateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#time",
            Box::new(TimeBuiltIn),
        );

        // Duration constructors
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#yearMonthDuration",
            Box::new(YearMonthDurationBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dayTimeDuration",
            Box::new(DayTimeDurationBuiltIn),
        );

        // Special constructors
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeStamp",
            Box::new(DateTimeStampBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#gYear",
            Box::new(GYearBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#gYearMonth",
            Box::new(GYearMonthBuiltIn),
        );

        registry
    }

    /// Register a built-in predicate
    pub fn register_builtin(&mut self, iri: &str, builtin: Box<dyn SWRLBuiltIn>) {
        self.builtins.insert(iri.to_string(), builtin);
    }

    /// Get a built-in predicate by IRI
    #[must_use]
    pub fn get_builtin(&self, iri: &str) -> Option<&dyn SWRLBuiltIn> {
        self.builtins.get(iri).map(std::convert::AsRef::as_ref)
    }

    /// Get all registered built-in IRIs
    #[must_use]
    pub fn get_builtin_iris(&self) -> Vec<String> {
        self.builtins.keys().cloned().collect()
    }
}

impl Default for DateTimeConstructorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Extract integer from `SWRLValue`
fn extract_integer(value: &SWRLValue) -> Result<i64> {
    match value {
        SWRLValue::Integer(i) => Ok(*i),
        SWRLValue::Literal(lit)
            if lit
                .datatype
                .as_ref()
                .map(|dt| dt.as_str().contains("integer"))
                .unwrap_or(false) =>
        {
            lit.value
                .parse::<i64>()
                .map_err(|_| Error::reasoning("Invalid integer literal"))
        }
        _ => Err(Error::reasoning("Expected integer value")),
    }
}

/// Extract float from `SWRLValue`
fn extract_float(value: &SWRLValue) -> Result<f64> {
    match value {
        SWRLValue::Float(f) => Ok(*f),
        SWRLValue::Integer(i) => Ok(*i as f64),
        SWRLValue::Literal(lit) => lit
            .value
            .parse::<f64>()
            .map_err(|_| Error::reasoning("Invalid numeric literal")),
        _ => Err(Error::reasoning("Expected numeric value")),
    }
}

/// Create a date literal
fn create_date_literal(date: NaiveDate) -> SWRLValue {
    SWRLValue::Literal(Literal::with_datatype(
        date.format("%Y-%m-%d").to_string(),
        IRI::new("http://www.w3.org/2001/XMLSchema#date"),
    ))
}

/// Create a time literal
fn create_time_literal(time: NaiveTime) -> SWRLValue {
    SWRLValue::Literal(Literal::with_datatype(
        time.format("%H:%M:%S").to_string(),
        IRI::new("http://www.w3.org/2001/XMLSchema#time"),
    ))
}

/// Create a dateTime literal
fn create_datetime_literal(datetime: NaiveDateTime) -> SWRLValue {
    SWRLValue::Literal(Literal::with_datatype(
        datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
        IRI::new("http://www.w3.org/2001/XMLSchema#dateTime"),
    ))
}

/// Create a duration literal
fn create_duration_literal(value: String, duration_type: &str) -> SWRLValue {
    SWRLValue::Literal(Literal::with_datatype(value, IRI::new(duration_type)))
}

// =============================================================================
// CONSTRUCTOR BUILT-INS
// =============================================================================

/// Date constructor built-in: swrlb:date(result, year, month, day)
pub struct DateBuiltIn;

impl SWRLBuiltIn for DateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 4 {
            return Err(Error::reasoning(
                "date expects exactly 4 arguments (result, year, month, day)",
            ));
        }

        let year = extract_integer(&args[1])? as i32;
        let month = extract_integer(&args[2])? as u32;
        let day = extract_integer(&args[3])? as u32;

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| Error::reasoning("Invalid date components"))?;

        let expected_result = create_date_literal(date);

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#date"
    }

    fn arity(&self) -> Option<usize> {
        Some(4)
    }
}

/// `DateTime` constructor built-in: swrlb:dateTime(result, year, month, day, hour, minute, second)
pub struct DateTimeBuiltIn;

impl SWRLBuiltIn for DateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 7 {
            return Err(Error::reasoning(
                "dateTime expects exactly 7 arguments (result, year, month, day, hour, minute, second)",
            ));
        }

        let year = extract_integer(&args[1])? as i32;
        let month = extract_integer(&args[2])? as u32;
        let day = extract_integer(&args[3])? as u32;
        let hour = extract_integer(&args[4])? as u32;
        let minute = extract_integer(&args[5])? as u32;
        let second = extract_integer(&args[6])? as u32;

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| Error::reasoning("Invalid date components"))?;
        let time = NaiveTime::from_hms_opt(hour, minute, second)
            .ok_or_else(|| Error::reasoning("Invalid time components"))?;
        let datetime = NaiveDateTime::new(date, time);

        let expected_result = create_datetime_literal(datetime);

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTime"
    }

    fn arity(&self) -> Option<usize> {
        Some(7)
    }
}

/// Time constructor built-in: swrlb:time(result, hour, minute, second)
pub struct TimeBuiltIn;

impl SWRLBuiltIn for TimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 4 {
            return Err(Error::reasoning(
                "time expects exactly 4 arguments (result, hour, minute, second)",
            ));
        }

        let hour = extract_integer(&args[1])? as u32;
        let minute = extract_integer(&args[2])? as u32;
        let second = extract_integer(&args[3])? as u32;

        let time = NaiveTime::from_hms_opt(hour, minute, second)
            .ok_or_else(|| Error::reasoning("Invalid time components"))?;

        let expected_result = create_time_literal(time);

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#time"
    }

    fn arity(&self) -> Option<usize> {
        Some(4)
    }
}

/// Year-month duration constructor: swrlb:yearMonthDuration(result, years, months)
pub struct YearMonthDurationBuiltIn;

impl SWRLBuiltIn for YearMonthDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "yearMonthDuration expects exactly 3 arguments (result, years, months)",
            ));
        }

        let years = extract_integer(&args[1])?;
        let months = extract_integer(&args[2])?;

        // ISO 8601 duration format: P[n]Y[n]M
        let duration_str = format!("P{years}Y{months}M");

        let expected_result = create_duration_literal(
            duration_str,
            "http://www.w3.org/2001/XMLSchema#yearMonthDuration",
        );

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#yearMonthDuration"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Day-time duration constructor: swrlb:dayTimeDuration(result, days, hours, minutes, seconds)
pub struct DayTimeDurationBuiltIn;

impl SWRLBuiltIn for DayTimeDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 5 {
            return Err(Error::reasoning(
                "dayTimeDuration expects exactly 5 arguments (result, days, hours, minutes, seconds)",
            ));
        }

        let days = extract_integer(&args[1])?;
        let hours = extract_integer(&args[2])?;
        let minutes = extract_integer(&args[3])?;
        let seconds = extract_float(&args[4])?;

        // ISO 8601 duration format: P[n]DT[n]H[n]M[n]S
        let duration_str = if seconds.fract() == 0.0 {
            format!("P{}DT{}H{}M{}S", days, hours, minutes, seconds as i64)
        } else {
            format!("P{days}DT{hours}H{minutes}M{seconds}S")
        };

        let expected_result = create_duration_literal(
            duration_str,
            "http://www.w3.org/2001/XMLSchema#dayTimeDuration",
        );

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dayTimeDuration"
    }

    fn arity(&self) -> Option<usize> {
        Some(5)
    }
}

/// `DateTime` stamp constructor: swrlb:dateTimeStamp(result, year, month, day, hour, minute, second, timezone)
pub struct DateTimeStampBuiltIn;

impl SWRLBuiltIn for DateTimeStampBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 8 {
            return Err(Error::reasoning(
                "dateTimeStamp expects exactly 8 arguments (result, year, month, day, hour, minute, second, timezone)",
            ));
        }

        let year = extract_integer(&args[1])? as i32;
        let month = extract_integer(&args[2])? as u32;
        let day = extract_integer(&args[3])? as u32;
        let hour = extract_integer(&args[4])? as u32;
        let minute = extract_integer(&args[5])? as u32;
        let second = extract_integer(&args[6])? as u32;

        // For timezone, we'll accept a string like "+05:00" or "Z"
        let timezone_str = match &args[7] {
            SWRLValue::String(tz) => tz.clone(),
            SWRLValue::Literal(lit) => lit.value.clone(),
            _ => return Err(Error::reasoning("Timezone must be a string")),
        };

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| Error::reasoning("Invalid date components"))?;
        let time = NaiveTime::from_hms_opt(hour, minute, second)
            .ok_or_else(|| Error::reasoning("Invalid time components"))?;
        let datetime = NaiveDateTime::new(date, time);

        let datetime_stamp_str =
            format!("{}{}", datetime.format("%Y-%m-%dT%H:%M:%S"), timezone_str);

        let expected_result = SWRLValue::Literal(Literal::with_datatype(
            datetime_stamp_str,
            IRI::new("http://www.w3.org/2001/XMLSchema#dateTimeStamp"),
        ));

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeStamp"
    }

    fn arity(&self) -> Option<usize> {
        Some(8)
    }
}

/// gYear constructor: swrlb:gYear(result, year)
pub struct GYearBuiltIn;

impl SWRLBuiltIn for GYearBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "gYear expects exactly 2 arguments (result, year)",
            ));
        }

        let year = extract_integer(&args[1])?;

        let expected_result = SWRLValue::Literal(Literal::with_datatype(
            format!("{year:04}"),
            IRI::new("http://www.w3.org/2001/XMLSchema#gYear"),
        ));

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#gYear"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// gYearMonth constructor: swrlb:gYearMonth(result, year, month)
pub struct GYearMonthBuiltIn;

impl SWRLBuiltIn for GYearMonthBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "gYearMonth expects exactly 3 arguments (result, year, month)",
            ));
        }

        let year = extract_integer(&args[1])?;
        let month = extract_integer(&args[2])?;

        if !(1..=12).contains(&month) {
            return Err(Error::reasoning("Month must be between 1 and 12"));
        }

        let expected_result = SWRLValue::Literal(Literal::with_datatype(
            format!("{year:04}-{month:02}"),
            IRI::new("http://www.w3.org/2001/XMLSchema#gYearMonth"),
        ));

        match &args[0] {
            SWRLValue::Literal(lit) => Ok(SWRLValue::Boolean(
                lit.value
                    == expected_result
                        .as_literal()
                        .expect("Failed to convert SWRL result to literal value")
                        .value,
            )),
            _ => Ok(expected_result),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#gYearMonth"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

// Extension trait to get literal from SWRLValue
trait SWRLValueExt {
    fn as_literal(&self) -> Option<&Literal>;
}

impl SWRLValueExt for SWRLValue {
    fn as_literal(&self) -> Option<&Literal> {
        match self {
            SWRLValue::Literal(lit) => Some(lit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_constructor() {
        let builtin = DateBuiltIn;

        // Test date construction
        let args = vec![
            SWRLValue::Literal(Literal::with_datatype(
                "2023-06-15".to_string(),
                IRI::new("http://www.w3.org/2001/XMLSchema#date"),
            )),
            SWRLValue::Integer(2023),
            SWRLValue::Integer(6),
            SWRLValue::Integer(15),
        ];

        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_datetime_constructor() {
        let builtin = DateTimeBuiltIn;

        let args = vec![
            SWRLValue::Literal(Literal::with_datatype(
                "2023-06-15T14:30:45".to_string(),
                IRI::new("http://www.w3.org/2001/XMLSchema#dateTime"),
            )),
            SWRLValue::Integer(2023),
            SWRLValue::Integer(6),
            SWRLValue::Integer(15),
            SWRLValue::Integer(14),
            SWRLValue::Integer(30),
            SWRLValue::Integer(45),
        ];

        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_year_month_duration_constructor() {
        let builtin = YearMonthDurationBuiltIn;

        let args = vec![
            SWRLValue::Literal(Literal::with_datatype(
                "P2Y6M".to_string(),
                IRI::new("http://www.w3.org/2001/XMLSchema#yearMonthDuration"),
            )),
            SWRLValue::Integer(2),
            SWRLValue::Integer(6),
        ];

        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_day_time_duration_constructor() {
        let builtin = DayTimeDurationBuiltIn;

        let args = vec![
            SWRLValue::Literal(Literal::with_datatype(
                "P5DT4H30M45S".to_string(),
                IRI::new("http://www.w3.org/2001/XMLSchema#dayTimeDuration"),
            )),
            SWRLValue::Integer(5),  // days
            SWRLValue::Integer(4),  // hours
            SWRLValue::Integer(30), // minutes
            SWRLValue::Float(45.0), // seconds
        ];

        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_gyear_constructor() {
        let builtin = GYearBuiltIn;

        let args = vec![
            SWRLValue::Literal(Literal::with_datatype(
                "2023".to_string(),
                IRI::new("http://www.w3.org/2001/XMLSchema#gYear"),
            )),
            SWRLValue::Integer(2023),
        ];

        let result = builtin
            .execute(&args)
            .expect("Failed to execute SWRL builtin with given arguments");
        assert_eq!(result, SWRLValue::Boolean(true));
    }
}
