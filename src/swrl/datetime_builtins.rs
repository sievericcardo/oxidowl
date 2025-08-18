use crate::swrl::{SWRLBuiltIn, SWRLValue};
use crate::swrl::temporal::{TemporalValue, TemporalError, utils};
use crate::ontology::Literal;
use crate::error::{Result, Error};
use std::collections::HashMap;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime, Datelike, Timelike};

/// Registry for date/time built-ins
pub struct DateTimeBuiltInRegistry {
    builtins: HashMap<String, Box<dyn SWRLBuiltIn>>,
}

impl DateTimeBuiltInRegistry {
    /// Create a new registry with core date/time built-ins
    pub fn new() -> Self {
        let mut registry = Self {
            builtins: HashMap::new(),
        };
        
        // Register essential date/time built-ins (Phase 1)
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateTimeEqual", Box::new(DateTimeEqualBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#yearFromDateTime", Box::new(YearFromDateTimeBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDateTime", Box::new(AddDayTimeDurationToDateTimeBuiltIn));
        
        // Phase 2 - Comparison built-ins
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateTimeLessThan", Box::new(DateTimeLessThanBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateTimeLessThanOrEqual", Box::new(DateTimeLessThanOrEqualBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateTimeGreaterThan", Box::new(DateTimeGreaterThanBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateTimeGreaterThanOrEqual", Box::new(DateTimeGreaterThanOrEqualBuiltIn));
        
        // Phase 2 - Component extraction built-ins
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#monthFromDateTime", Box::new(MonthFromDateTimeBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dayFromDateTime", Box::new(DayFromDateTimeBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#hourFromDateTime", Box::new(HourFromDateTimeBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#minuteFromDateTime", Box::new(MinuteFromDateTimeBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#secondFromDateTime", Box::new(SecondFromDateTimeBuiltIn));
        
        // Phase 2 - Date and time specific operations
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateEqual", Box::new(DateEqualBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#dateLessThan", Box::new(DateLessThanBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#timeEqual", Box::new(TimeEqualBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#timeLessThan", Box::new(TimeLessThanBuiltIn));
        
        // Additional datetime built-ins for enhanced functionality
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#subtractDatesYieldingDayTimeDuration", Box::new(SubtractDatesYieldingDayTimeDurationBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#subtractTimesYieldingDayTimeDuration", Box::new(SubtractTimesYieldingDayTimeDurationBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#subtractDayTimeDurations", Box::new(SubtractDayTimeDurationsBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#multiplyDayTimeDuration", Box::new(MultiplyDayTimeDurationBuiltIn));
        registry.register_builtin("http://www.w3.org/2003/11/swrlb#divideDayTimeDuration", Box::new(DivideDayTimeDurationBuiltIn));
        
        registry
    }
    
    /// Register a built-in
    pub fn register_builtin(&mut self, iri: &str, builtin: Box<dyn SWRLBuiltIn>) {
        self.builtins.insert(iri.to_string(), builtin);
    }
    
    /// Get a built-in by IRI
    pub fn get_builtin(&self, iri: &str) -> Option<&dyn SWRLBuiltIn> {
        self.builtins.get(iri).map(|b| b.as_ref())
    }
    
    /// Get all registered built-in IRIs
    pub fn get_builtin_iris(&self) -> Vec<String> {
        self.builtins.keys().cloned().collect()
    }
}

impl Default for DateTimeBuiltInRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to extract temporal value from SWRL value
fn extract_temporal_value(value: &SWRLValue) -> std::result::Result<TemporalValue, TemporalError> {
    TemporalValue::from_swrl_value(value)
}

// ===== CORE BUILT-INS =====

/// dateTimeEqual built-in
#[derive(Debug, Clone)]
pub struct DateTimeEqualBuiltIn;

impl SWRLBuiltIn for DateTimeEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateTimeEqual expects exactly 2 arguments"));
        }
        
        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {}", e)))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {}", e)))?;
        
        Ok(SWRLValue::Boolean(dt1 == dt2))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeEqual"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// yearFromDateTime built-in
#[derive(Debug, Clone)]
pub struct YearFromDateTimeBuiltIn;

impl SWRLBuiltIn for YearFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("yearFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(year) = temporal_value.year() {
            // Check if result matches expected value or return the year
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_year) = lit.value.parse::<i32>() {
                        Ok(SWRLValue::Boolean(year == expected_year))
                    } else {
                        Err(Error::reasoning("Expected numeric year value"))
                    }
                },
                _ => {
                    // Return the extracted year
                    Ok(SWRLValue::Integer(year as i64))
                }
            }
        } else {
            Err(Error::reasoning("Cannot extract year from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#yearFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// addDayTimeDurationToDateTime built-in
#[derive(Debug, Clone)]
pub struct AddDayTimeDurationToDateTimeBuiltIn;

impl SWRLBuiltIn for AddDayTimeDurationToDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("addDayTimeDurationToDateTime expects exactly 3 arguments"));
        }
        
        let datetime = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        let duration = extract_temporal_value(&args[2])
            .map_err(|e| Error::reasoning(format!("Invalid duration value: {}", e)))?;
        
        let result = utils::add_day_time_duration(&datetime, &duration)
            .map_err(|e| Error::reasoning(format!("Duration addition failed: {}", e)))?;
        
        // Check if result matches expected value
        match &args[0] {
            SWRLValue::Literal(_) => {
                let expected = extract_temporal_value(&args[0])
                    .map_err(|e| Error::reasoning(format!("Invalid expected result: {}", e)))?;
                Ok(SWRLValue::Boolean(result == expected))
            },
            _ => {
                // Return the computed result
                Ok(result.to_swrl_value())
            }
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

// ===== PHASE 2 BUILT-INS: COMPARISON OPERATIONS =====

/// dateTimeLessThan built-in
#[derive(Debug, Clone)]
pub struct DateTimeLessThanBuiltIn;

impl SWRLBuiltIn for DateTimeLessThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateTimeLessThan expects exactly 2 arguments"));
        }
        
        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {}", e)))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {}", e)))?;
        
        Ok(SWRLValue::Boolean(dt1 < dt2))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeLessThan"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// dateTimeLessThanOrEqual built-in
#[derive(Debug, Clone)]
pub struct DateTimeLessThanOrEqualBuiltIn;

impl SWRLBuiltIn for DateTimeLessThanOrEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateTimeLessThanOrEqual expects exactly 2 arguments"));
        }
        
        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {}", e)))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {}", e)))?;
        
        Ok(SWRLValue::Boolean(dt1 <= dt2))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeLessThanOrEqual"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// dateTimeGreaterThan built-in
#[derive(Debug, Clone)]
pub struct DateTimeGreaterThanBuiltIn;

impl SWRLBuiltIn for DateTimeGreaterThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateTimeGreaterThan expects exactly 2 arguments"));
        }
        
        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {}", e)))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {}", e)))?;
        
        Ok(SWRLValue::Boolean(dt1 > dt2))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeGreaterThan"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// dateTimeGreaterThanOrEqual built-in
#[derive(Debug, Clone)]
pub struct DateTimeGreaterThanOrEqualBuiltIn;

impl SWRLBuiltIn for DateTimeGreaterThanOrEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateTimeGreaterThanOrEqual expects exactly 2 arguments"));
        }
        
        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {}", e)))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {}", e)))?;
        
        Ok(SWRLValue::Boolean(dt1 >= dt2))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateTimeGreaterThanOrEqual"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

// ===== PHASE 2 BUILT-INS: COMPONENT EXTRACTION =====

/// monthFromDateTime built-in
#[derive(Debug, Clone)]
pub struct MonthFromDateTimeBuiltIn;

impl SWRLBuiltIn for MonthFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("monthFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(month) = temporal_value.month() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_month) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(month == expected_month))
                    } else {
                        Err(Error::reasoning("Expected numeric month value"))
                    }
                },
                _ => Ok(SWRLValue::Integer(month as i64))
            }
        } else {
            Err(Error::reasoning("Cannot extract month from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#monthFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// dayFromDateTime built-in
#[derive(Debug, Clone)]
pub struct DayFromDateTimeBuiltIn;

impl SWRLBuiltIn for DayFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dayFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(day) = temporal_value.day() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_day) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(day == expected_day))
                    } else {
                        Err(Error::reasoning("Expected numeric day value"))
                    }
                },
                _ => Ok(SWRLValue::Integer(day as i64))
            }
        } else {
            Err(Error::reasoning("Cannot extract day from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dayFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// hourFromDateTime built-in
#[derive(Debug, Clone)]
pub struct HourFromDateTimeBuiltIn;

impl SWRLBuiltIn for HourFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("hourFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(hour) = temporal_value.hour() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_hour) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(hour == expected_hour))
                    } else {
                        Err(Error::reasoning("Expected numeric hour value"))
                    }
                },
                _ => Ok(SWRLValue::Integer(hour as i64))
            }
        } else {
            Err(Error::reasoning("Cannot extract hour from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#hourFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// minuteFromDateTime built-in
#[derive(Debug, Clone)]
pub struct MinuteFromDateTimeBuiltIn;

impl SWRLBuiltIn for MinuteFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("minuteFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(minute) = temporal_value.minute() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_minute) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(minute == expected_minute))
                    } else {
                        Err(Error::reasoning("Expected numeric minute value"))
                    }
                },
                _ => Ok(SWRLValue::Integer(minute as i64))
            }
        } else {
            Err(Error::reasoning("Cannot extract minute from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#minuteFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// secondFromDateTime built-in
#[derive(Debug, Clone)]
pub struct SecondFromDateTimeBuiltIn;

impl SWRLBuiltIn for SecondFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("secondFromDateTime expects exactly 2 arguments"));
        }
        
        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {}", e)))?;
        
        if let Some(second) = temporal_value.second() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_second) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(second == expected_second))
                    } else {
                        Err(Error::reasoning("Expected numeric second value"))
                    }
                },
                _ => Ok(SWRLValue::Integer(second as i64))
            }
        } else {
            Err(Error::reasoning("Cannot extract second from temporal value"))
        }
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#secondFromDateTime"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

// ===== PHASE 2 BUILT-INS: DATE AND TIME SPECIFIC OPERATIONS =====

/// dateEqual built-in
#[derive(Debug, Clone)]
pub struct DateEqualBuiltIn;

impl SWRLBuiltIn for DateEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateEqual expects exactly 2 arguments"));
        }
        
        let date1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first date: {}", e)))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second date: {}", e)))?;
        
        // Compare only date components
        let dates_equal = date1.year() == date2.year() &&
                         date1.month() == date2.month() &&
                         date1.day() == date2.day();
        
        Ok(SWRLValue::Boolean(dates_equal))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateEqual"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// dateLessThan built-in
#[derive(Debug, Clone)]
pub struct DateLessThanBuiltIn;

impl SWRLBuiltIn for DateLessThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("dateLessThan expects exactly 2 arguments"));
        }
        
        let date1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first date: {}", e)))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second date: {}", e)))?;
        
        // Create date values for comparison (ignoring time components)
        let d1_year = date1.year().unwrap_or(1970);
        let d1_month = date1.month().unwrap_or(1);
        let d1_day = date1.day().unwrap_or(1);
        
        let d2_year = date2.year().unwrap_or(1970);
        let d2_month = date2.month().unwrap_or(1);
        let d2_day = date2.day().unwrap_or(1);
        
        let date_less = d1_year < d2_year ||
                       (d1_year == d2_year && d1_month < d2_month) ||
                       (d1_year == d2_year && d1_month == d2_month && d1_day < d2_day);
        
        Ok(SWRLValue::Boolean(date_less))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#dateLessThan"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// timeEqual built-in
#[derive(Debug, Clone)]
pub struct TimeEqualBuiltIn;

impl SWRLBuiltIn for TimeEqualBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("timeEqual expects exactly 2 arguments"));
        }
        
        let time1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first time: {}", e)))?;
        let time2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second time: {}", e)))?;
        
        // Compare only time components
        let times_equal = time1.hour() == time2.hour() &&
                         time1.minute() == time2.minute() &&
                         time1.second() == time2.second();
        
        Ok(SWRLValue::Boolean(times_equal))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#timeEqual"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// timeLessThan built-in
#[derive(Debug, Clone)]
pub struct TimeLessThanBuiltIn;

impl SWRLBuiltIn for TimeLessThanBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("timeLessThan expects exactly 2 arguments"));
        }
        
        let time1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first time: {}", e)))?;
        let time2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second time: {}", e)))?;
        
        // Create time values for comparison (ignoring date components)
        let t1_hour = time1.hour().unwrap_or(0);
        let t1_minute = time1.minute().unwrap_or(0);
        let t1_second = time1.second().unwrap_or(0);
        
        let t2_hour = time2.hour().unwrap_or(0);
        let t2_minute = time2.minute().unwrap_or(0);
        let t2_second = time2.second().unwrap_or(0);
        
        let time_less = t1_hour < t2_hour ||
                       (t1_hour == t2_hour && t1_minute < t2_minute) ||
                       (t1_hour == t2_hour && t1_minute == t2_minute && t1_second < t2_second);
        
        Ok(SWRLValue::Boolean(time_less))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#timeLessThan"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// subtractDatesYieldingDayTimeDuration built-in
#[derive(Debug, Clone)]
pub struct SubtractDatesYieldingDayTimeDurationBuiltIn;

impl SWRLBuiltIn for SubtractDatesYieldingDayTimeDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(crate::Error::reasoning("subtractDatesYieldingDayTimeDuration requires exactly 2 arguments"));
        }
        
        let date1 = extract_temporal_value(&args[0])
            .map_err(|_| crate::Error::reasoning("Invalid first date value"))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|_| crate::Error::reasoning("Invalid second date value"))?;
        
        // Calculate duration in days between dates
        let duration_days = match (&date1, &date2) {
            (TemporalValue::Date(d1), TemporalValue::Date(d2)) => {
                let dt1 = d1.and_hms_opt(0, 0, 0).unwrap();
                let dt2 = d2.and_hms_opt(0, 0, 0).unwrap();
                (dt1 - dt2).num_days()
            },
            _ => return Err(crate::Error::reasoning("Both arguments must be dates")),
        };
        
        Ok(SWRLValue::Literal(Literal::new(format!("P{}D", duration_days))))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subtractDatesYieldingDayTimeDuration"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// subtractTimesYieldingDayTimeDuration built-in
#[derive(Debug, Clone)]
pub struct SubtractTimesYieldingDayTimeDurationBuiltIn;

impl SWRLBuiltIn for SubtractTimesYieldingDayTimeDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(crate::Error::reasoning("subtractTimesYieldingDayTimeDuration requires exactly 2 arguments"));
        }
        
        let time1 = extract_temporal_value(&args[0])
            .map_err(|_| crate::Error::reasoning("Invalid first time value"))?;
        let time2 = extract_temporal_value(&args[1])
            .map_err(|_| crate::Error::reasoning("Invalid second time value"))?;
        
        // Calculate duration between times
        let duration_seconds = match (&time1, &time2) {
            (TemporalValue::Time(t1), TemporalValue::Time(t2)) => {
                let t1_seconds = t1.hour() * 3600 + t1.minute() * 60 + t1.second();
                let t2_seconds = t2.hour() * 3600 + t2.minute() * 60 + t2.second();
                (t1_seconds as i64) - (t2_seconds as i64)
            },
            _ => return Err(crate::Error::reasoning("Both arguments must be times")),
        };
        
        Ok(SWRLValue::Literal(Literal::new(format!("PT{}S", duration_seconds))))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subtractTimesYieldingDayTimeDuration"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// subtractDayTimeDurations built-in
#[derive(Debug, Clone)]
pub struct SubtractDayTimeDurationsBuiltIn;

impl SWRLBuiltIn for SubtractDayTimeDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(crate::Error::reasoning("subtractDayTimeDurations requires exactly 2 arguments"));
        }
        
        // For now, return a placeholder duration
        // Full implementation would parse duration strings and perform arithmetic
        Ok(SWRLValue::Literal(Literal::new("PT0S".to_string())))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurations"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// multiplyDayTimeDuration built-in
#[derive(Debug, Clone)]
pub struct MultiplyDayTimeDurationBuiltIn;

impl SWRLBuiltIn for MultiplyDayTimeDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(crate::Error::reasoning("multiplyDayTimeDuration requires exactly 2 arguments"));
        }
        
        // For now, return a placeholder duration
        // Full implementation would parse duration string and multiply by scalar
        Ok(SWRLValue::Literal(Literal::new("PT0S".to_string())))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#multiplyDayTimeDuration"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// divideDayTimeDuration built-in
#[derive(Debug, Clone)]
pub struct DivideDayTimeDurationBuiltIn;

impl SWRLBuiltIn for DivideDayTimeDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(crate::Error::reasoning("divideDayTimeDuration requires exactly 2 arguments"));
        }
        
        // For now, return a placeholder duration
        // Full implementation would parse duration string and divide by scalar
        Ok(SWRLValue::Literal(Literal::new("PT0S".to_string())))
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#divideDayTimeDuration"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}
