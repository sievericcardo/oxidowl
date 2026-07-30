use crate::error::{Error, Result};
use crate::ontology::Literal;
use crate::swrl::temporal::{TemporalError, TemporalValue, utils};
use crate::swrl::{SWRLBuiltIn, SWRLValue};
use chrono::{NaiveDate, Timelike, Datelike};
use std::collections::HashMap;

/// Parse a day-time duration from a string
fn parse_day_time_duration(duration_str: &str) -> Result<chrono::Duration> {
    // Parse ISO 8601 duration format PT[n]H[n]M[n]S
    let cleaned = duration_str.trim_start_matches("PT");

    let mut hours = 0i64;
    let mut minutes = 0i64;
    let mut seconds = 0i64;

    // Parse hours, minutes, and seconds with proper regex-like parsing
    let mut remaining = cleaned;

    // Parse hours
    if let Some(h_pos) = remaining.find('H')
        && let Ok(h) = remaining[..h_pos].parse::<i64>()
    {
        hours = h;
        remaining = &remaining[h_pos + 1..];
    }

    // Parse minutes
    if let Some(m_pos) = remaining.find('M')
        && let Ok(m) = remaining[..m_pos].parse::<i64>()
    {
        minutes = m;
        remaining = &remaining[m_pos + 1..];
    }

    // Parse seconds
    if let Some(s_pos) = remaining.find('S')
        && let Ok(s) = remaining[..s_pos].parse::<i64>()
    {
        seconds = s;
    }

    Ok(chrono::Duration::hours(hours)
        + chrono::Duration::minutes(minutes)
        + chrono::Duration::seconds(seconds))
}

/// Format a duration as day-time duration string
fn format_day_time_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("PT{hours}H{minutes}M{seconds}S")
    } else if minutes > 0 {
        format!("PT{minutes}M{seconds}S")
    } else {
        format!("PT{seconds}S")
    }
}

/// Registry for date/time built-ins
pub struct DateTimeBuiltInRegistry {
    builtins: HashMap<String, Box<dyn SWRLBuiltIn>>,
}

impl DateTimeBuiltInRegistry {
    /// Create a new registry with core date/time built-ins
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            builtins: HashMap::new(),
        };

        // Register essential date/time built-ins
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeEqual",
            Box::new(DateTimeEqualBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#yearFromDateTime",
            Box::new(YearFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDateTime",
            Box::new(AddDayTimeDurationToDateTimeBuiltIn),
        );

        // Phase 2 - Comparison built-ins
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeLessThan",
            Box::new(DateTimeLessThanBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeLessThanOrEqual",
            Box::new(DateTimeLessThanOrEqualBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeGreaterThan",
            Box::new(DateTimeGreaterThanBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateTimeGreaterThanOrEqual",
            Box::new(DateTimeGreaterThanOrEqualBuiltIn),
        );

        // Phase 2 - Component extraction built-ins
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#monthFromDateTime",
            Box::new(MonthFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dayFromDateTime",
            Box::new(DayFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#hourFromDateTime",
            Box::new(HourFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#minuteFromDateTime",
            Box::new(MinuteFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#secondFromDateTime",
            Box::new(SecondFromDateTimeBuiltIn),
        );

        // Phase 2 - Date and time specific operations
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateEqual",
            Box::new(DateEqualBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#dateLessThan",
            Box::new(DateLessThanBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#timeEqual",
            Box::new(TimeEqualBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#timeLessThan",
            Box::new(TimeLessThanBuiltIn),
        );

        // Additional datetime built-ins for enhanced functionality
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDatesYieldingDayTimeDuration",
            Box::new(SubtractDatesYieldingDayTimeDurationBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractTimesYieldingDayTimeDuration",
            Box::new(SubtractTimesYieldingDayTimeDurationBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurations",
            Box::new(SubtractDayTimeDurationsBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#multiplyDayTimeDuration",
            Box::new(MultiplyDayTimeDurationBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#divideDayTimeDuration",
            Box::new(DivideDayTimeDurationBuiltIn),
        );

        // Missing built-ins from the plan
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#timezone",
            Box::new(TimezoneBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addYearMonthDuration",
            Box::new(AddYearMonthDurationBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addYearMonthDurations",
            Box::new(AddYearMonthDurationsBuiltIn),
        );

        // Phase 3 — Missing date arithmetic operations
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDates",
            Box::new(SubtractDatesBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractTimes",
            Box::new(SubtractTimesBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurations",
            Box::new(SubtractYearMonthDurationsBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#multiplyYearMonthDurations",
            Box::new(MultiplyYearMonthDurationsBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#divideYearMonthDurations",
            Box::new(DivideYearMonthDurationsBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addDayTimeDurations",
            Box::new(AddDayTimeDurationsBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurationFromDateTime",
            Box::new(SubtractYearMonthDurationFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromDateTime",
            Box::new(SubtractDayTimeDurationFromDateTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addYearMonthDurationToDate",
            Box::new(AddYearMonthDurationToDateBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDate",
            Box::new(AddDayTimeDurationToDateBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurationFromDate",
            Box::new(SubtractYearMonthDurationFromDateBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromDate",
            Box::new(SubtractDayTimeDurationFromDateBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToTime",
            Box::new(AddDayTimeDurationToTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromTime",
            Box::new(SubtractDayTimeDurationFromTimeBuiltIn),
        );
        registry.register_builtin(
            "http://www.w3.org/2003/11/swrlb#subtractDateTimesYieldingYearMonthDuration",
            Box::new(SubtractDateTimesYieldingYearMonthDurationBuiltIn),
        );

        registry
    }

    /// Register a built-in
    pub fn register_builtin(&mut self, iri: &str, builtin: Box<dyn SWRLBuiltIn>) {
        self.builtins.insert(iri.to_string(), builtin);
    }

    /// Get a built-in by IRI
    #[must_use]
    pub fn get_builtin(&self, iri: &str) -> Option<&dyn SWRLBuiltIn> {
        self.builtins.get(iri).map(std::convert::AsRef::as_ref)
    }

    /// Get all registered built-in IRIs
    #[must_use]
    pub fn get_builtin_iris(&self) -> Vec<String> {
        self.builtins.keys().cloned().collect()
    }

    /// Parse dayTimeDuration string into `chrono::Duration`
    pub fn parse_day_time_duration(&self, duration_str: &str) -> crate::Result<chrono::Duration> {
        // Parse ISO 8601 duration format like PT1H30M45S
        if !duration_str.starts_with('P') {
            return Err(crate::Error::reasoning("Invalid duration format"));
        }

        let mut duration = chrono::Duration::zero();
        let mut current_number = String::new();
        let mut in_time_part = false;

        for ch in duration_str.chars().skip(1) {
            match ch {
                'T' => in_time_part = true,
                'D' if !in_time_part => {
                    let days = current_number
                        .parse::<i64>()
                        .map_err(|_| crate::Error::reasoning("Invalid day value in duration"))?;
                    duration += chrono::Duration::days(days);
                    current_number.clear();
                }
                'H' if in_time_part => {
                    let hours = current_number
                        .parse::<i64>()
                        .map_err(|_| crate::Error::reasoning("Invalid hour value in duration"))?;
                    duration += chrono::Duration::hours(hours);
                    current_number.clear();
                }
                'M' if in_time_part => {
                    let minutes = current_number
                        .parse::<i64>()
                        .map_err(|_| crate::Error::reasoning("Invalid minute value in duration"))?;
                    duration += chrono::Duration::minutes(minutes);
                    current_number.clear();
                }
                'S' if in_time_part => {
                    let seconds = current_number
                        .parse::<f64>()
                        .map_err(|_| crate::Error::reasoning("Invalid second value in duration"))?;
                    duration += chrono::Duration::nanoseconds((seconds * 1_000_000_000.0) as i64);
                    current_number.clear();
                }
                ch if ch.is_ascii_digit() || ch == '.' => {
                    current_number.push(ch);
                }
                _ => return Err(crate::Error::reasoning("Invalid character in duration")),
            }
        }

        Ok(duration)
    }

    /// Format `chrono::Duration` as dayTimeDuration string
    #[must_use]
    pub fn format_day_time_duration(&self, duration: chrono::Duration) -> String {
        let total_seconds = duration.num_seconds();
        let days = total_seconds / 86400;
        let remaining = total_seconds % 86400;
        let hours = remaining / 3600;
        let remaining = remaining % 3600;
        let minutes = remaining / 60;
        let seconds = remaining % 60;

        if days > 0 {
            format!("P{days}DT{hours}H{minutes}M{seconds}S")
        } else {
            format!("PT{hours}H{minutes}M{seconds}S")
        }
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
            return Err(Error::reasoning(
                "dateTimeEqual expects exactly 2 arguments",
            ));
        }

        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {e}")))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {e}")))?;

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
            return Err(Error::reasoning(
                "yearFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(year) = temporal_value.year() {
            // Check if result matches expected value or return the year
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_year) = lit.value.parse::<i32>() {
                        Ok(SWRLValue::Boolean(year == expected_year))
                    } else {
                        Err(Error::reasoning("Expected numeric year value"))
                    }
                }
                _ => {
                    // Return the extracted year
                    Ok(SWRLValue::Integer(i64::from(year)))
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
            return Err(Error::reasoning(
                "addDayTimeDurationToDateTime expects exactly 3 arguments",
            ));
        }

        let datetime = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;
        let duration = extract_temporal_value(&args[2])
            .map_err(|e| Error::reasoning(format!("Invalid duration value: {e}")))?;

        let result = utils::add_day_time_duration(&datetime, &duration)
            .map_err(|e| Error::reasoning(format!("Duration addition failed: {e}")))?;

        // Check if result matches expected value
        match &args[0] {
            SWRLValue::Literal(_) => {
                let expected = extract_temporal_value(&args[0])
                    .map_err(|e| Error::reasoning(format!("Invalid expected result: {e}")))?;
                Ok(SWRLValue::Boolean(result == expected))
            }
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
            return Err(Error::reasoning(
                "dateTimeLessThan expects exactly 2 arguments",
            ));
        }

        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {e}")))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {e}")))?;

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
            return Err(Error::reasoning(
                "dateTimeLessThanOrEqual expects exactly 2 arguments",
            ));
        }

        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {e}")))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {e}")))?;

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
            return Err(Error::reasoning(
                "dateTimeGreaterThan expects exactly 2 arguments",
            ));
        }

        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {e}")))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {e}")))?;

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
            return Err(Error::reasoning(
                "dateTimeGreaterThanOrEqual expects exactly 2 arguments",
            ));
        }

        let dt1 = extract_temporal_value(&args[0])
            .map_err(|e| Error::reasoning(format!("Invalid first datetime: {e}")))?;
        let dt2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second datetime: {e}")))?;

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
            return Err(Error::reasoning(
                "monthFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(month) = temporal_value.month() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_month) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(month == expected_month))
                    } else {
                        Err(Error::reasoning("Expected numeric month value"))
                    }
                }
                _ => Ok(SWRLValue::Integer(i64::from(month))),
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
            return Err(Error::reasoning(
                "dayFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(day) = temporal_value.day() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_day) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(day == expected_day))
                    } else {
                        Err(Error::reasoning("Expected numeric day value"))
                    }
                }
                _ => Ok(SWRLValue::Integer(i64::from(day))),
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
            return Err(Error::reasoning(
                "hourFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(hour) = temporal_value.hour() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_hour) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(hour == expected_hour))
                    } else {
                        Err(Error::reasoning("Expected numeric hour value"))
                    }
                }
                _ => Ok(SWRLValue::Integer(i64::from(hour))),
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
            return Err(Error::reasoning(
                "minuteFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(minute) = temporal_value.minute() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_minute) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(minute == expected_minute))
                    } else {
                        Err(Error::reasoning("Expected numeric minute value"))
                    }
                }
                _ => Ok(SWRLValue::Integer(i64::from(minute))),
            }
        } else {
            Err(Error::reasoning(
                "Cannot extract minute from temporal value",
            ))
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
            return Err(Error::reasoning(
                "secondFromDateTime expects exactly 2 arguments",
            ));
        }

        let temporal_value = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid datetime value: {e}")))?;

        if let Some(second) = temporal_value.second() {
            match &args[0] {
                SWRLValue::Literal(lit) => {
                    if let Ok(expected_second) = lit.value.parse::<u32>() {
                        Ok(SWRLValue::Boolean(second == expected_second))
                    } else {
                        Err(Error::reasoning("Expected numeric second value"))
                    }
                }
                _ => Ok(SWRLValue::Integer(i64::from(second))),
            }
        } else {
            Err(Error::reasoning(
                "Cannot extract second from temporal value",
            ))
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
            .map_err(|e| Error::reasoning(format!("Invalid first date: {e}")))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second date: {e}")))?;

        // Compare only date components
        let dates_equal = date1.year() == date2.year()
            && date1.month() == date2.month()
            && date1.day() == date2.day();

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
            .map_err(|e| Error::reasoning(format!("Invalid first date: {e}")))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second date: {e}")))?;

        // Create date values for comparison (ignoring time components)
        let d1_year = date1.year().unwrap_or(1970);
        let d1_month = date1.month().unwrap_or(1);
        let d1_day = date1.day().unwrap_or(1);

        let d2_year = date2.year().unwrap_or(1970);
        let d2_month = date2.month().unwrap_or(1);
        let d2_day = date2.day().unwrap_or(1);

        let date_less = d1_year < d2_year
            || (d1_year == d2_year && d1_month < d2_month)
            || (d1_year == d2_year && d1_month == d2_month && d1_day < d2_day);

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
            .map_err(|e| Error::reasoning(format!("Invalid first time: {e}")))?;
        let time2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second time: {e}")))?;

        // Compare only time components
        let times_equal = time1.hour() == time2.hour()
            && time1.minute() == time2.minute()
            && time1.second() == time2.second();

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
            .map_err(|e| Error::reasoning(format!("Invalid first time: {e}")))?;
        let time2 = extract_temporal_value(&args[1])
            .map_err(|e| Error::reasoning(format!("Invalid second time: {e}")))?;

        // Create time values for comparison (ignoring date components)
        let t1_hour = time1.hour().unwrap_or(0);
        let t1_minute = time1.minute().unwrap_or(0);
        let t1_second = time1.second().unwrap_or(0);

        let t2_hour = time2.hour().unwrap_or(0);
        let t2_minute = time2.minute().unwrap_or(0);
        let t2_second = time2.second().unwrap_or(0);

        let time_less = t1_hour < t2_hour
            || (t1_hour == t2_hour && t1_minute < t2_minute)
            || (t1_hour == t2_hour && t1_minute == t2_minute && t1_second < t2_second);

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
            return Err(crate::Error::reasoning(
                "subtractDatesYieldingDayTimeDuration requires exactly 2 arguments",
            ));
        }

        let date1 = extract_temporal_value(&args[0])
            .map_err(|_| crate::Error::reasoning("Invalid first date value"))?;
        let date2 = extract_temporal_value(&args[1])
            .map_err(|_| crate::Error::reasoning("Invalid second date value"))?;

        // Calculate duration in days between dates
        let duration_days = match (&date1, &date2) {
            (TemporalValue::Date(d1), TemporalValue::Date(d2)) => {
                let dt1 = d1.and_hms_opt(0, 0, 0).ok_or_else(|| {
                    crate::Error::reasoning("Invalid date for datetime conversion")
                })?;
                let dt2 = d2.and_hms_opt(0, 0, 0).ok_or_else(|| {
                    crate::Error::reasoning("Invalid date for datetime conversion")
                })?;
                (dt1 - dt2).num_days()
            }
            _ => return Err(crate::Error::reasoning("Both arguments must be dates")),
        };

        Ok(SWRLValue::Literal(Literal::new(format!(
            "P{duration_days}D"
        ))))
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
            return Err(crate::Error::reasoning(
                "subtractTimesYieldingDayTimeDuration requires exactly 2 arguments",
            ));
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
                i64::from(t1_seconds) - i64::from(t2_seconds)
            }
            _ => return Err(crate::Error::reasoning("Both arguments must be times")),
        };

        Ok(SWRLValue::Literal(Literal::new(format!(
            "PT{duration_seconds}S"
        ))))
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
            return Err(crate::Error::reasoning(
                "subtractDayTimeDurations requires exactly 2 arguments",
            ));
        }

        // Parse both durations and subtract them
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(duration1), SWRLValue::Literal(duration2)) => {
                let dur1 = parse_day_time_duration(&duration1.value)?;
                let dur2 = parse_day_time_duration(&duration2.value)?;

                let result_seconds = dur1.num_seconds() - dur2.num_seconds();
                let result_duration = chrono::Duration::seconds(result_seconds);

                let result_str = format_day_time_duration(result_duration);
                Ok(SWRLValue::Literal(Literal::new(result_str)))
            }
            _ => Err(crate::Error::reasoning(
                "Invalid argument types for subtractDayTimeDurations",
            )),
        }
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
            return Err(crate::Error::reasoning(
                "multiplyDayTimeDuration requires exactly 2 arguments",
            ));
        }

        // Parse duration and multiply by scalar
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(duration_lit), SWRLValue::Literal(factor_lit)) => {
                let duration = parse_day_time_duration(&duration_lit.value)?;
                let factor = factor_lit
                    .value
                    .parse::<f64>()
                    .map_err(|_| crate::Error::reasoning("Invalid numeric factor"))?;

                let result_seconds = (duration.num_seconds() as f64 * factor) as i64;
                let result_duration = chrono::Duration::seconds(result_seconds);

                let result_str = format_day_time_duration(result_duration);
                Ok(SWRLValue::Literal(Literal::new(result_str)))
            }
            _ => Err(crate::Error::reasoning(
                "Invalid argument types for multiplyDayTimeDuration",
            )),
        }
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
            return Err(crate::Error::reasoning(
                "divideDayTimeDuration requires exactly 2 arguments",
            ));
        }

        // Parse duration and divide by scalar
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(duration_lit), SWRLValue::Literal(divisor_lit)) => {
                let duration = parse_day_time_duration(&duration_lit.value)?;
                let divisor = divisor_lit
                    .value
                    .parse::<f64>()
                    .map_err(|_| crate::Error::reasoning("Invalid numeric divisor"))?;

                if divisor == 0.0 {
                    return Err(crate::Error::reasoning("Division by zero"));
                }

                let result_seconds = (duration.num_seconds() as f64 / divisor) as i64;
                let result_duration = chrono::Duration::seconds(result_seconds);

                let result_str = format_day_time_duration(result_duration);
                Ok(SWRLValue::Literal(Literal::new(result_str)))
            }
            _ => Err(crate::Error::reasoning(
                "Invalid argument types for divideDayTimeDuration",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#divideDayTimeDuration"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// timezone built-in: Extract timezone from dateTime value
/// swrlb:timezone(result, dateTime)
#[derive(Debug, Clone)]
pub struct TimezoneBuiltIn;

impl SWRLBuiltIn for TimezoneBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "timezone expects exactly 2 arguments (result, dateTime)",
            ));
        }

        match &args[1] {
            SWRLValue::Literal(literal) => {
                let datetime_str = &literal.value;

                // Extract timezone portion from ISO 8601 datetime string
                // Examples: "2023-01-01T10:00:00Z", "2023-01-01T10:00:00+05:00", "2023-01-01T10:00:00-08:00"
                let timezone = if datetime_str.ends_with('Z') {
                    "Z".to_string()
                } else if let Some(t_pos) = datetime_str.find('T') {
                    // Look for timezone markers only after the 'T' (time part)
                    let time_part = &datetime_str[t_pos..];
                    if let Some(tz_pos) = time_part.rfind(['+', '-']) {
                        // Make sure it's actually a timezone offset (not part of seconds)
                        let tz_candidate = &time_part[tz_pos..];
                        if tz_candidate.len() >= 3
                            && (tz_candidate.contains(':') || tz_candidate.len() == 3)
                        {
                            tz_candidate.to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    // No 'T' found, probably not a valid datetime
                    String::new()
                };

                Ok(SWRLValue::Literal(Literal {
                    value: timezone,
                    datatype: Some(
                        url::Url::parse("http://www.w3.org/2001/XMLSchema#string")
                            .map_err(|e| Error::reasoning(format!("Invalid URL: {e}")))?,
                    ),
                    language: None,
                }))
            }
            _ => Err(Error::reasoning("timezone requires a dateTime literal")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#timezone"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// addYearMonthDuration built-in: Add year-month duration to date
/// swrlb:addYearMonthDuration(result, date, yearMonthDuration)
#[derive(Debug, Clone)]
pub struct AddYearMonthDurationBuiltIn;

impl SWRLBuiltIn for AddYearMonthDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "addYearMonthDuration expects exactly 3 arguments (result, date, yearMonthDuration)",
            ));
        }

        match (&args[1], &args[2]) {
            (SWRLValue::Literal(date_lit), SWRLValue::Literal(duration_lit)) => {
                let date_str = &date_lit.value;
                let duration_str = &duration_lit.value;

                // Parse date (YYYY-MM-DD format)
                let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|e| Error::reasoning(format!("Invalid date format: {e}")))?;

                // Parse yearMonth duration (P1Y2M format)
                let (years, months) = parse_year_month_duration(duration_str)?;

                // Add years and months
                let new_date = date
                    .checked_add_months(chrono::Months::new((years * 12 + months) as u32))
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))?;

                Ok(SWRLValue::Literal(Literal {
                    value: new_date.format("%Y-%m-%d").to_string(),
                    datatype: Some(
                        url::Url::parse("http://www.w3.org/2001/XMLSchema#date")
                            .map_err(|e| Error::reasoning(format!("Invalid URL: {e}")))?,
                    ),
                    language: None,
                }))
            }
            _ => Err(Error::reasoning(
                "addYearMonthDuration requires date and yearMonthDuration literals",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#addYearMonthDuration"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// addYearMonthDurations built-in: Add two year-month durations
/// swrlb:addYearMonthDurations(result, duration1, duration2)
#[derive(Debug, Clone)]
pub struct AddYearMonthDurationsBuiltIn;

impl SWRLBuiltIn for AddYearMonthDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "addYearMonthDurations expects exactly 3 arguments (result, duration1, duration2)",
            ));
        }

        match (&args[1], &args[2]) {
            (SWRLValue::Literal(dur1_lit), SWRLValue::Literal(dur2_lit)) => {
                let dur1_str = &dur1_lit.value;
                let dur2_str = &dur2_lit.value;

                // Parse both yearMonth durations
                let (years1, months1) = parse_year_month_duration(dur1_str)?;
                let (years2, months2) = parse_year_month_duration(dur2_str)?;

                // Add durations
                let total_months = years1 * 12 + months1 + years2 * 12 + months2;
                let result_years = total_months / 12;
                let result_months = total_months % 12;

                // Format result as P{years}Y{months}M
                let result = if result_years > 0 && result_months > 0 {
                    format!("P{result_years}Y{result_months}M")
                } else if result_years > 0 {
                    format!("P{result_years}Y")
                } else if result_months > 0 {
                    format!("P{result_months}M")
                } else {
                    "P0M".to_string()
                };

                Ok(SWRLValue::Literal(Literal {
                    value: result,
                    datatype: Some(
                        url::Url::parse("http://www.w3.org/2001/XMLSchema#yearMonthDuration")
                            .map_err(|e| Error::reasoning(format!("Invalid URL: {e}")))?,
                    ),
                    language: None,
                }))
            }
            _ => Err(Error::reasoning(
                "addYearMonthDurations requires two yearMonthDuration literals",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#addYearMonthDurations"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Parse a date string (YYYY-MM-DD) into a NaiveDate
fn parse_date(date_str: &str) -> Result<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| Error::reasoning(format!("Invalid date '{date_str}': {e}")))
}

/// Parse a time string (HH:MM:SS) into a NaiveTime
fn parse_time(time_str: &str) -> Result<chrono::NaiveTime> {
    chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .or_else(|_| chrono::NaiveTime::parse_from_str(time_str, "%H:%M"))
        .map_err(|e| Error::reasoning(format!("Invalid time '{time_str}': {e}")))
}

/// Parse a dateTime string (ISO 8601)
fn parse_date_time(dt_str: &str) -> Result<chrono::NaiveDateTime> {
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M") {
        return Ok(dt);
    }
    Err(Error::reasoning(format!("Invalid dateTime '{dt_str}'")))
}

/// Format a year-month duration as ISO 8601
fn format_year_month_duration(years: i32, months: i32) -> String {
    let mut result = String::from("P");
    if years != 0 {
        result.push_str(&format!("{years}Y"));
    }
    if months != 0 {
        result.push_str(&format!("{months}M"));
    }
    if years == 0 && months == 0 {
        result.push_str("0M");
    }
    result
}

/// subtractDates built-in: (result, date1, date2) → dayTimeDuration
#[derive(Debug, Clone)]
pub struct SubtractDatesBuiltIn;

impl SWRLBuiltIn for SubtractDatesBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractDates requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(d1), SWRLValue::Literal(d2)) => {
                let date1 = parse_date(&d1.value)?;
                let date2 = parse_date(&d2.value)?;
                let duration = date1.signed_duration_since(date2);
                Ok(SWRLValue::Literal(Literal::new(format_day_time_duration(duration))))
            }
            _ => Err(Error::reasoning("subtractDates requires literal date arguments")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractDates" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractTimes built-in: (result, time1, time2) → dayTimeDuration
#[derive(Debug, Clone)]
pub struct SubtractTimesBuiltIn;

impl SWRLBuiltIn for SubtractTimesBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractTimes requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(t1), SWRLValue::Literal(t2)) => {
                let time1 = parse_time(&t1.value)?;
                let time2 = parse_time(&t2.value)?;
                let secs1 = time1.num_seconds_from_midnight() as i64;
                let secs2 = time2.num_seconds_from_midnight() as i64;
                let diff = secs1 - secs2;
                Ok(SWRLValue::Literal(Literal::new(format_day_time_duration(
                    chrono::Duration::seconds(diff),
                ))))
            }
            _ => Err(Error::reasoning("subtractTimes requires literal time arguments")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractTimes" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractYearMonthDurations built-in
#[derive(Debug, Clone)]
pub struct SubtractYearMonthDurationsBuiltIn;

impl SWRLBuiltIn for SubtractYearMonthDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractYearMonthDurations requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(d1), SWRLValue::Literal(d2)) => {
                let (y1, m1) = parse_year_month_duration(&d1.value)?;
                let (y2, m2) = parse_year_month_duration(&d2.value)?;
                let total_months = (y1 * 12 + m1) - (y2 * 12 + m2);
                Ok(SWRLValue::Literal(Literal::new(format_year_month_duration(
                    total_months / 12, total_months % 12,
                ))))
            }
            _ => Err(Error::reasoning("subtractYearMonthDurations requires literal duration arguments")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurations" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// multiplyYearMonthDurations built-in
#[derive(Debug, Clone)]
pub struct MultiplyYearMonthDurationsBuiltIn;

impl SWRLBuiltIn for MultiplyYearMonthDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("multiplyYearMonthDurations requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(d), SWRLValue::Literal(f)) => {
                let (y, m) = parse_year_month_duration(&d.value)?;
                let factor: f64 = f.value.parse().map_err(|_| Error::reasoning("Invalid scalar"))?;
                let total_months = ((y * 12 + m) as f64 * factor) as i32;
                Ok(SWRLValue::Literal(Literal::new(format_year_month_duration(
                    total_months / 12, total_months % 12,
                ))))
            }
            _ => Err(Error::reasoning("multiplyYearMonthDurations requires literal duration and scalar")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#multiplyYearMonthDurations" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// divideYearMonthDurations built-in
#[derive(Debug, Clone)]
pub struct DivideYearMonthDurationsBuiltIn;

impl SWRLBuiltIn for DivideYearMonthDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("divideYearMonthDurations requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(d), SWRLValue::Literal(f)) => {
                let (y, m) = parse_year_month_duration(&d.value)?;
                let divisor: f64 = f.value.parse().map_err(|_| Error::reasoning("Invalid scalar"))?;
                if divisor == 0.0 {
                    return Err(Error::reasoning("Division by zero"));
                }
                let total_months = ((y * 12 + m) as f64 / divisor) as i32;
                Ok(SWRLValue::Literal(Literal::new(format_year_month_duration(
                    total_months / 12, total_months % 12,
                ))))
            }
            _ => Err(Error::reasoning("divideYearMonthDurations requires literal duration and scalar")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#divideYearMonthDurations" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// addDayTimeDurations built-in
#[derive(Debug, Clone)]
pub struct AddDayTimeDurationsBuiltIn;

impl SWRLBuiltIn for AddDayTimeDurationsBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("addDayTimeDurations requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(d1), SWRLValue::Literal(d2)) => {
                let dur1 = parse_day_time_duration(&d1.value)?;
                let dur2 = parse_day_time_duration(&d2.value)?;
                let result = dur1.checked_add(&dur2)
                    .ok_or_else(|| Error::reasoning("Duration addition overflow"))?;
                Ok(SWRLValue::Literal(Literal::new(format_day_time_duration(result))))
            }
            _ => Err(Error::reasoning("addDayTimeDurations requires literal duration arguments")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#addDayTimeDurations" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractYearMonthDurationFromDateTime built-in
#[derive(Debug, Clone)]
pub struct SubtractYearMonthDurationFromDateTimeBuiltIn;

impl SWRLBuiltIn for SubtractYearMonthDurationFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractYearMonthDurationFromDateTime requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(dt), SWRLValue::Literal(dur)) => {
                let datetime = parse_date_time(&dt.value)?;
                let (y, m) = parse_year_month_duration(&dur.value)?;
                let total_months = y * 12 + m;
                let result = if total_months >= 0 {
                    datetime.checked_sub_months(chrono::Months::new(total_months as u32))
                } else {
                    datetime.checked_add_months(chrono::Months::new((-total_months) as u32))
                };
                result.map(|r| SWRLValue::Literal(Literal::new(r.format("%Y-%m-%dT%H:%M:%S").to_string())))
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))
            }
            _ => Err(Error::reasoning("subtractYearMonthDurationFromDateTime requires literal datetime and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurationFromDateTime" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractDayTimeDurationFromDateTime built-in
#[derive(Debug, Clone)]
pub struct SubtractDayTimeDurationFromDateTimeBuiltIn;

impl SWRLBuiltIn for SubtractDayTimeDurationFromDateTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractDayTimeDurationFromDateTime requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(dt), SWRLValue::Literal(dur)) => {
                let datetime = parse_date_time(&dt.value)?;
                let duration = parse_day_time_duration(&dur.value)?;
                let result = datetime.checked_sub_signed(duration)
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))?;
                Ok(SWRLValue::Literal(Literal::new(result.format("%Y-%m-%dT%H:%M:%S").to_string())))
            }
            _ => Err(Error::reasoning("subtractDayTimeDurationFromDateTime requires literal datetime and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromDateTime" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// addYearMonthDurationToDate built-in
#[derive(Debug, Clone)]
pub struct AddYearMonthDurationToDateBuiltIn;

impl SWRLBuiltIn for AddYearMonthDurationToDateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("addYearMonthDurationToDate requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(date), SWRLValue::Literal(dur)) => {
                let d = parse_date(&date.value)?;
                let (y, m) = parse_year_month_duration(&dur.value)?;
                let total_months = y * 12 + m;
                let result = if total_months >= 0 {
                    d.checked_add_months(chrono::Months::new(total_months as u32))
                } else {
                    d.checked_sub_months(chrono::Months::new((-total_months) as u32))
                };
                result.map(|r| SWRLValue::Literal(Literal::new(r.format("%Y-%m-%d").to_string())))
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))
            }
            _ => Err(Error::reasoning("addYearMonthDurationToDate requires literal date and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#addYearMonthDurationToDate" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// addDayTimeDurationToDate built-in
#[derive(Debug, Clone)]
pub struct AddDayTimeDurationToDateBuiltIn;

impl SWRLBuiltIn for AddDayTimeDurationToDateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("addDayTimeDurationToDate requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(date), SWRLValue::Literal(dur)) => {
                let d = parse_date(&date.value)?;
                let duration = parse_day_time_duration(&dur.value)?;
                let dt = d.and_hms_opt(0, 0, 0)
                    .ok_or_else(|| Error::reasoning("Invalid date"))?;
                let result = dt.checked_add_signed(duration)
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))?;
                Ok(SWRLValue::Literal(Literal::new(result.date().format("%Y-%m-%d").to_string())))
            }
            _ => Err(Error::reasoning("addDayTimeDurationToDate requires literal date and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToDate" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractYearMonthDurationFromDate built-in
#[derive(Debug, Clone)]
pub struct SubtractYearMonthDurationFromDateBuiltIn;

impl SWRLBuiltIn for SubtractYearMonthDurationFromDateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractYearMonthDurationFromDate requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(date), SWRLValue::Literal(dur)) => {
                let d = parse_date(&date.value)?;
                let (y, m) = parse_year_month_duration(&dur.value)?;
                let total_months = y * 12 + m;
                let result = if total_months >= 0 {
                    d.checked_sub_months(chrono::Months::new(total_months as u32))
                } else {
                    d.checked_add_months(chrono::Months::new((-total_months) as u32))
                };
                result.map(|r| SWRLValue::Literal(Literal::new(r.format("%Y-%m-%d").to_string())))
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))
            }
            _ => Err(Error::reasoning("subtractYearMonthDurationFromDate requires literal date and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractYearMonthDurationFromDate" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractDayTimeDurationFromDate built-in
#[derive(Debug, Clone)]
pub struct SubtractDayTimeDurationFromDateBuiltIn;

impl SWRLBuiltIn for SubtractDayTimeDurationFromDateBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractDayTimeDurationFromDate requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(date), SWRLValue::Literal(dur)) => {
                let d = parse_date(&date.value)?;
                let duration = parse_day_time_duration(&dur.value)?;
                let dt = d.and_hms_opt(0, 0, 0)
                    .ok_or_else(|| Error::reasoning("Invalid date"))?;
                let result = dt.checked_sub_signed(duration)
                    .ok_or_else(|| Error::reasoning("Date arithmetic overflow"))?;
                Ok(SWRLValue::Literal(Literal::new(result.date().format("%Y-%m-%d").to_string())))
            }
            _ => Err(Error::reasoning("subtractDayTimeDurationFromDate requires literal date and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromDate" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// addDayTimeDurationToTime built-in
#[derive(Debug, Clone)]
pub struct AddDayTimeDurationToTimeBuiltIn;

impl SWRLBuiltIn for AddDayTimeDurationToTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("addDayTimeDurationToTime requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(t), SWRLValue::Literal(dur)) => {
                let time = parse_time(&t.value)?;
                let duration = parse_day_time_duration(&dur.value)?;
                let secs = time.num_seconds_from_midnight() as i64 + duration.num_seconds();
                let wrapped = secs.rem_euclid(86400);
                let h = wrapped / 3600;
                let m = (wrapped % 3600) / 60;
                let s = wrapped % 60;
                Ok(SWRLValue::Literal(Literal::new(format!("{h:02}:{m:02}:{s:02}"))))
            }
            _ => Err(Error::reasoning("addDayTimeDurationToTime requires literal time and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#addDayTimeDurationToTime" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractDayTimeDurationFromTime built-in
#[derive(Debug, Clone)]
pub struct SubtractDayTimeDurationFromTimeBuiltIn;

impl SWRLBuiltIn for SubtractDayTimeDurationFromTimeBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractDayTimeDurationFromTime requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(t), SWRLValue::Literal(dur)) => {
                let time = parse_time(&t.value)?;
                let duration = parse_day_time_duration(&dur.value)?;
                let secs = time.num_seconds_from_midnight() as i64 - duration.num_seconds();
                let wrapped = secs.rem_euclid(86400);
                let h = wrapped / 3600;
                let m = (wrapped % 3600) / 60;
                let s = wrapped % 60;
                Ok(SWRLValue::Literal(Literal::new(format!("{h:02}:{m:02}:{s:02}"))))
            }
            _ => Err(Error::reasoning("subtractDayTimeDurationFromTime requires literal time and duration")),
        }
    }
    fn name(&self) -> &str { "http://www.w3.org/2003/11/swrlb#subtractDayTimeDurationFromTime" }
    fn arity(&self) -> Option<usize> { Some(2) }
}

/// subtractDateTimesYieldingYearMonthDuration built-in
#[derive(Debug, Clone)]
pub struct SubtractDateTimesYieldingYearMonthDurationBuiltIn;

impl SWRLBuiltIn for SubtractDateTimesYieldingYearMonthDurationBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("subtractDateTimesYieldingYearMonthDuration requires exactly 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (SWRLValue::Literal(dt1), SWRLValue::Literal(dt2)) => {
                let d1 = parse_date_time(&dt1.value)?;
                let d2 = parse_date_time(&dt2.value)?;
                let months_diff =
                    (d1.year() - d2.year()) * 12 + (d1.month() as i32 - d2.month() as i32);
                Ok(SWRLValue::Literal(Literal::new(format_year_month_duration(
                    months_diff / 12,
                    months_diff % 12,
                ))))
            }
            _ => Err(Error::reasoning(
                "subtractDateTimesYieldingYearMonthDuration requires literal datetime arguments",
            )),
        }
    }
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subtractDateTimesYieldingYearMonthDuration"
    }
    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Helper function to parse year-month duration string
pub fn parse_year_month_duration(duration_str: &str) -> Result<(i32, i32)> {
    if !duration_str.starts_with('P') {
        return Err(Error::reasoning(
            "Invalid yearMonthDuration format: must start with 'P'",
        ));
    }

    let duration_part = &duration_str[1..]; // Remove 'P'
    let mut years = 0;
    let mut months = 0;

    let mut current_num = String::new();

    for ch in duration_part.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else if ch == 'Y' {
            years = current_num
                .parse::<i32>()
                .map_err(|_| Error::reasoning("Invalid year value in yearMonthDuration"))?;
            current_num.clear();
        } else if ch == 'M' {
            months = current_num
                .parse::<i32>()
                .map_err(|_| Error::reasoning("Invalid month value in yearMonthDuration"))?;
            current_num.clear();
        } else {
            return Err(Error::reasoning("Invalid character in yearMonthDuration"));
        }
    }

    Ok((years, months))
}
