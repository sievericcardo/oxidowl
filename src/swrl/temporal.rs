use crate::error::Error;
use crate::ontology::Literal;
use crate::swrl::SWRLValue;
use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike,
};
use iso8601_duration::Duration as IsoDuration;
use std::str::FromStr;
use thiserror::Error as ThisError;

/// Represents temporal values used in SWRL date/time built-ins
#[derive(Debug, Clone, PartialEq)]
pub enum TemporalValue {
    /// Date without time (e.g., 2023-12-25)
    Date(NaiveDate),
    /// Time without date (e.g., 14:30:00)
    Time(NaiveTime),
    /// DateTime without timezone (e.g., 2023-12-25T14:30:00)
    DateTime(NaiveDateTime),
    /// DateTime with timezone (e.g., 2023-12-25T14:30:00Z)
    DateTimeWithTz(DateTime<FixedOffset>),
    /// Year-month duration (e.g., P1Y2M)
    YearMonthDuration(IsoDuration),
    /// Day-time duration (e.g., P1DT2H30M)
    DayTimeDuration(Duration),
    /// Year only (e.g., 2023)
    GYear(i32),
    /// Year and month (e.g., 2023-12)
    GYearMonth(i32, u32),
    /// Month and day (e.g., --12-25)
    GMonthDay(u32, u32),
    /// Month only (e.g., --12)
    GMonth(u32),
    /// Day only (e.g., ---25)
    GDay(u32),
}

/// Errors that can occur during temporal operations
#[derive(ThisError, Debug)]
pub enum TemporalError {
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),
    #[error("Invalid duration format: {0}")]
    InvalidDurationFormat(String),
    #[error("Arithmetic overflow in temporal operation")]
    ArithmeticOverflow,
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),
    #[error("Unsupported temporal operation: {0}")]
    UnsupportedOperation(String),
}

impl From<TemporalError> for Error {
    fn from(err: TemporalError) -> Self {
        Error::reasoning(format!("Temporal error: {}", err))
    }
}

impl PartialOrd for TemporalValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            // DateTime comparisons
            (TemporalValue::DateTime(dt1), TemporalValue::DateTime(dt2)) => dt1.partial_cmp(dt2),

            // Date comparisons
            (TemporalValue::Date(d1), TemporalValue::Date(d2)) => d1.partial_cmp(d2),

            // Time comparisons
            (TemporalValue::Time(t1), TemporalValue::Time(t2)) => t1.partial_cmp(t2),

            // For durations, we'll convert to total seconds for comparison
            (TemporalValue::DayTimeDuration(_), TemporalValue::DayTimeDuration(_)) => {
                let seconds1 = self.to_total_seconds();
                let seconds2 = other.to_total_seconds();
                seconds1.partial_cmp(&seconds2)
            }

            (TemporalValue::YearMonthDuration(_), TemporalValue::YearMonthDuration(_)) => {
                let months1 = self.to_total_months();
                let months2 = other.to_total_months();
                months1.partial_cmp(&months2)
            }

            // Cross-type comparisons
            (TemporalValue::DateTime(dt), TemporalValue::Date(d)) => {
                let dt_date = dt.date();
                dt_date.partial_cmp(d)
            }
            (TemporalValue::Date(d), TemporalValue::DateTime(dt)) => {
                let dt_date = dt.date();
                d.partial_cmp(&dt_date)
            }

            // For other cross-type comparisons, return None (incomparable)
            _ => None,
        }
    }
}

impl TemporalValue {
    /// Convert duration to total seconds (for comparison purposes)
    fn to_total_seconds(&self) -> f64 {
        match self {
            TemporalValue::DayTimeDuration(duration) => duration.num_seconds() as f64,
            _ => 0.0,
        }
    }

    /// Convert duration to total months (for comparison purposes)
    fn to_total_months(&self) -> i32 {
        match self {
            TemporalValue::YearMonthDuration(duration) => {
                let mut total_months = 0;

                // Years (convert f32 to i32)
                if duration.year > 0.0 {
                    total_months += (duration.year * 12.0) as i32;
                }

                // Months (convert f32 to i32)
                if duration.month > 0.0 {
                    total_months += duration.month as i32;
                }

                total_months
            }
            _ => 0,
        }
    }

    /// Parse a temporal value from a literal
    pub fn from_literal(literal: &Literal) -> Result<Self, TemporalError> {
        let value = &literal.value;
        let datatype = literal
            .datatype
            .as_ref()
            .ok_or_else(|| TemporalError::InvalidDateFormat("No datatype specified".to_string()))?;

        match datatype.as_str() {
            "http://www.w3.org/2001/XMLSchema#date" => Self::parse_date(value),
            "http://www.w3.org/2001/XMLSchema#time" => Self::parse_time(value),
            "http://www.w3.org/2001/XMLSchema#dateTime" => Self::parse_datetime(value),
            "http://www.w3.org/2001/XMLSchema#duration" => Self::parse_duration(value),
            "http://www.w3.org/2001/XMLSchema#dayTimeDuration" => {
                Self::parse_day_time_duration(value)
            }
            "http://www.w3.org/2001/XMLSchema#yearMonthDuration" => {
                Self::parse_year_month_duration(value)
            }
            "http://www.w3.org/2001/XMLSchema#gYear" => Self::parse_gyear(value),
            "http://www.w3.org/2001/XMLSchema#gYearMonth" => Self::parse_gyear_month(value),
            "http://www.w3.org/2001/XMLSchema#gMonthDay" => Self::parse_gmonth_day(value),
            "http://www.w3.org/2001/XMLSchema#gMonth" => Self::parse_gmonth(value),
            "http://www.w3.org/2001/XMLSchema#gDay" => Self::parse_gday(value),
            _ => Err(TemporalError::UnsupportedOperation(format!(
                "Unsupported temporal datatype: {}",
                datatype
            ))),
        }
    }

    /// Parse date from string
    fn parse_date(value: &str) -> Result<Self, TemporalError> {
        NaiveDate::from_str(value)
            .map(TemporalValue::Date)
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))
    }

    /// Parse time from string
    fn parse_time(value: &str) -> Result<Self, TemporalError> {
        NaiveTime::from_str(value)
            .map(TemporalValue::Time)
            .map_err(|e| TemporalError::InvalidTimeFormat(format!("{}: {}", value, e)))
    }

    /// Parse datetime from string
    fn parse_datetime(value: &str) -> Result<Self, TemporalError> {
        // Try to detect timezone
        if Self::has_timezone(value) {
            // Parse with timezone
            let dt = DateTime::parse_from_rfc3339(value)
                .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;
            
            // Convert to UTC (offset 0) - this operation is infallible for valid DateTime
            let utc_dt = dt.with_timezone(&FixedOffset::east_opt(0)
                .expect("Zero offset is always valid"));
            Ok(TemporalValue::DateTimeWithTz(utc_dt))
        } else {
            // Parse without timezone
            NaiveDateTime::from_str(value)
                .map(TemporalValue::DateTime)
                .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))
        }
    }

    /// Check if a datetime string has timezone information
    fn has_timezone(value: &str) -> bool {
        if value.len() < 20 {
            return false;
        }

        let suffix = &value[19..];
        suffix.starts_with('Z') || suffix.starts_with('+') || suffix.starts_with('-')
    }

    /// Parse duration from string
    fn parse_duration(value: &str) -> Result<Self, TemporalError> {
        // Try to parse as ISO 8601 duration first
        if let Ok(iso_duration) = IsoDuration::from_str(value) {
            // Determine if it's a year-month or day-time duration
            if iso_duration.year > 0.0 || iso_duration.month > 0.0 {
                Ok(TemporalValue::YearMonthDuration(iso_duration))
            } else {
                // Convert to chrono Duration
                let mut total_seconds = 0i64;

                if iso_duration.day > 0.0 {
                    total_seconds += (iso_duration.day * 24.0 * 3600.0) as i64;
                }
                if iso_duration.hour > 0.0 {
                    total_seconds += (iso_duration.hour * 3600.0) as i64;
                }
                if iso_duration.minute > 0.0 {
                    total_seconds += (iso_duration.minute * 60.0) as i64;
                }
                if iso_duration.second > 0.0 {
                    total_seconds += iso_duration.second as i64;
                }

                Duration::try_seconds(total_seconds)
                    .map(TemporalValue::DayTimeDuration)
                    .ok_or_else(|| TemporalError::ArithmeticOverflow)
            }
        } else {
            Err(TemporalError::InvalidDurationFormat(value.to_string()))
        }
    }

    /// Parse day-time duration from string
    fn parse_day_time_duration(value: &str) -> Result<Self, TemporalError> {
        let iso_duration = IsoDuration::from_str(value)
            .map_err(|e| TemporalError::InvalidDurationFormat(format!("{}: {:?}", value, e)))?;

        // Convert to chrono Duration
        let mut total_seconds = 0i64;

        if iso_duration.day > 0.0 {
            total_seconds += (iso_duration.day * 24.0 * 3600.0) as i64;
        }
        if iso_duration.hour > 0.0 {
            total_seconds += (iso_duration.hour * 3600.0) as i64;
        }
        if iso_duration.minute > 0.0 {
            total_seconds += (iso_duration.minute * 60.0) as i64;
        }
        if iso_duration.second > 0.0 {
            total_seconds += iso_duration.second as i64;
        }

        Duration::try_seconds(total_seconds)
            .map(TemporalValue::DayTimeDuration)
            .ok_or_else(|| TemporalError::ArithmeticOverflow)
    }

    /// Parse year-month duration from string
    fn parse_year_month_duration(value: &str) -> Result<Self, TemporalError> {
        IsoDuration::from_str(value)
            .map(TemporalValue::YearMonthDuration)
            .map_err(|e| TemporalError::InvalidDurationFormat(format!("{}: {:?}", value, e)))
    }

    /// Parse gYear from string
    fn parse_gyear(value: &str) -> Result<Self, TemporalError> {
        value
            .parse::<i32>()
            .map(TemporalValue::GYear)
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))
    }

    /// Parse gYearMonth from string  
    fn parse_gyear_month(value: &str) -> Result<Self, TemporalError> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 2 {
            return Err(TemporalError::InvalidDateFormat(value.to_string()));
        }

        let year = parts[0]
            .parse::<i32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;
        let month = parts[1]
            .parse::<u32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;

        Ok(TemporalValue::GYearMonth(year, month))
    }

    /// Parse gMonthDay from string
    fn parse_gmonth_day(value: &str) -> Result<Self, TemporalError> {
        if !value.starts_with("--") {
            return Err(TemporalError::InvalidDateFormat(value.to_string()));
        }

        let value = &value[2..]; // Remove --
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 2 {
            return Err(TemporalError::InvalidDateFormat(value.to_string()));
        }

        let month = parts[0]
            .parse::<u32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;
        let day = parts[1]
            .parse::<u32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;

        Ok(TemporalValue::GMonthDay(month, day))
    }

    /// Parse gMonth from string
    fn parse_gmonth(value: &str) -> Result<Self, TemporalError> {
        if !value.starts_with("--") {
            return Err(TemporalError::InvalidDateFormat(value.to_string()));
        }

        let month_str = &value[2..];
        let month = month_str
            .parse::<u32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;

        Ok(TemporalValue::GMonth(month))
    }

    /// Parse gDay from string
    fn parse_gday(value: &str) -> Result<Self, TemporalError> {
        if !value.starts_with("---") {
            return Err(TemporalError::InvalidDateFormat(value.to_string()));
        }

        let day_str = &value[3..];
        let day = day_str
            .parse::<u32>()
            .map_err(|e| TemporalError::InvalidDateFormat(format!("{}: {}", value, e)))?;

        Ok(TemporalValue::GDay(day))
    }

    /// Convert TemporalValue to SWRLValue
    pub fn to_swrl_value(&self) -> SWRLValue {
        match self {
            TemporalValue::Date(date) => SWRLValue::String(date.to_string()),
            TemporalValue::Time(time) => SWRLValue::String(time.to_string()),
            TemporalValue::DateTime(dt) => SWRLValue::String(dt.to_string()),
            TemporalValue::DateTimeWithTz(dt) => SWRLValue::String(dt.to_rfc3339()),
            TemporalValue::DayTimeDuration(duration) => {
                SWRLValue::String(format!("PT{}S", duration.num_seconds()))
            }
            TemporalValue::YearMonthDuration(duration) => SWRLValue::String(duration.to_string()),
            TemporalValue::GYear(year) => SWRLValue::String(year.to_string()),
            TemporalValue::GYearMonth(year, month) => {
                SWRLValue::String(format!("{:04}-{:02}", year, month))
            }
            TemporalValue::GMonthDay(month, day) => {
                SWRLValue::String(format!("--{:02}-{:02}", month, day))
            }
            TemporalValue::GMonth(month) => SWRLValue::String(format!("--{:02}", month)),
            TemporalValue::GDay(day) => SWRLValue::String(format!("---{:02}", day)),
        }
    }

    /// Convert from SWRLValue
    pub fn from_swrl_value(value: &SWRLValue) -> Result<Self, TemporalError> {
        match value {
            SWRLValue::String(s) => {
                // Try to parse as different temporal types
                Self::parse_datetime(s)
                    .or_else(|_| Self::parse_date(s))
                    .or_else(|_| Self::parse_time(s))
                    .or_else(|_| Self::parse_duration(s))
            }
            SWRLValue::Literal(literal) => Self::from_literal(literal),
            _ => Err(TemporalError::UnsupportedOperation(
                "Cannot convert to temporal value".to_string(),
            )),
        }
    }

    /// Get year component
    pub fn year(&self) -> Option<i32> {
        match self {
            TemporalValue::Date(date) => Some(date.year()),
            TemporalValue::DateTime(dt) => Some(dt.year()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.year()),
            TemporalValue::GYear(year) => Some(*year),
            TemporalValue::GYearMonth(year, _) => Some(*year),
            _ => None,
        }
    }

    /// Get month component
    pub fn month(&self) -> Option<u32> {
        match self {
            TemporalValue::Date(date) => Some(date.month()),
            TemporalValue::DateTime(dt) => Some(dt.month()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.month()),
            TemporalValue::GYearMonth(_, month) => Some(*month),
            TemporalValue::GMonthDay(month, _) => Some(*month),
            TemporalValue::GMonth(month) => Some(*month),
            _ => None,
        }
    }

    /// Get day component
    pub fn day(&self) -> Option<u32> {
        match self {
            TemporalValue::Date(date) => Some(date.day()),
            TemporalValue::DateTime(dt) => Some(dt.day()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.day()),
            TemporalValue::GMonthDay(_, day) => Some(*day),
            TemporalValue::GDay(day) => Some(*day),
            _ => None,
        }
    }

    /// Get hour component
    pub fn hour(&self) -> Option<u32> {
        match self {
            TemporalValue::Time(time) => Some(time.hour()),
            TemporalValue::DateTime(dt) => Some(dt.hour()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.hour()),
            _ => None,
        }
    }

    /// Get minute component
    pub fn minute(&self) -> Option<u32> {
        match self {
            TemporalValue::Time(time) => Some(time.minute()),
            TemporalValue::DateTime(dt) => Some(dt.minute()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.minute()),
            _ => None,
        }
    }

    /// Get second component
    pub fn second(&self) -> Option<u32> {
        match self {
            TemporalValue::Time(time) => Some(time.second()),
            TemporalValue::DateTime(dt) => Some(dt.second()),
            TemporalValue::DateTimeWithTz(dt) => Some(dt.second()),
            _ => None,
        }
    }

    /// Add a duration to this temporal value
    pub fn add_duration(&self, duration: &TemporalValue) -> Result<TemporalValue, TemporalError> {
        match (self, duration) {
            (TemporalValue::DateTime(dt), TemporalValue::DayTimeDuration(dur)) => {
                Ok(TemporalValue::DateTime(*dt + *dur))
            }
            (TemporalValue::Date(date), TemporalValue::DayTimeDuration(dur)) => {
                let dt = date.and_hms_opt(0, 0, 0)
                    .ok_or_else(|| TemporalError::InvalidDateFormat(
                        "Failed to convert date to datetime".to_string()))?;
                let result_dt = dt + *dur;
                Ok(TemporalValue::Date(result_dt.date()))
            }
            _ => Err(TemporalError::UnsupportedOperation(
                "Unsupported duration addition".to_string(),
            )),
        }
    }

    /// Subtract a duration from this temporal value
    pub fn subtract_duration(
        &self,
        duration: &TemporalValue,
    ) -> Result<TemporalValue, TemporalError> {
        match (self, duration) {
            (TemporalValue::DateTime(dt), TemporalValue::DayTimeDuration(dur)) => {
                Ok(TemporalValue::DateTime(*dt - *dur))
            }
            (TemporalValue::Date(date), TemporalValue::DayTimeDuration(dur)) => {
                let dt = date.and_hms_opt(0, 0, 0)
                    .ok_or_else(|| TemporalError::InvalidDateFormat(
                        "Failed to convert date to datetime".to_string()))?;
                let result_dt = dt - *dur;
                Ok(TemporalValue::Date(result_dt.date()))
            }
            _ => Err(TemporalError::UnsupportedOperation(
                "Unsupported duration subtraction".to_string(),
            )),
        }
    }
}

pub mod utils {
    use super::*;

    /// Add day-time duration to a temporal value
    pub fn add_day_time_duration(
        temporal: &TemporalValue,
        duration: &TemporalValue,
    ) -> Result<TemporalValue, TemporalError> {
        temporal.add_duration(duration)
    }

    /// Subtract day-time duration from a temporal value
    pub fn subtract_day_time_duration(
        temporal: &TemporalValue,
        duration: &TemporalValue,
    ) -> Result<TemporalValue, TemporalError> {
        temporal.subtract_duration(duration)
    }
}
