//! DateTime Value Space Handler (xsd:dateTime, xsd:date, xsd:time, xsd:duration)

use super::ValueSpaceHandler;
use crate::error::Error;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime};

/// Handler for xsd:dateTime.
#[derive(Debug, Clone, Default)]
pub struct DateTimeValueSpace;

/// Handler for xsd:date.
#[derive(Debug, Clone, Default)]
pub struct DateValueSpace;

/// Handler for xsd:time.
#[derive(Debug, Clone, Default)]
pub struct TimeValueSpace;

impl ValueSpaceHandler for DateTimeValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#dateTime"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        parse_datetime(value).is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        parse_datetime(value)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|_| value.to_string())
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let da = parse_datetime(a)?;
        let db = parse_datetime(b)?;
        Ok(da == db)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        let v = parse_datetime(value)?;
        let fv = parse_datetime(facet_value)?;
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minInclusive" => Ok(v >= fv),
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => Ok(v <= fv),
            "http://www.w3.org/2001/XMLSchema#minExclusive" => Ok(v > fv),
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => Ok(v < fv),
            other => Err(Error::invalid_input(format!("Unsupported facet: {other}"))),
        }
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

impl ValueSpaceHandler for DateValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#date"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(|d| d.to_string())
            .unwrap_or_else(|_| value.to_string())
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let da = NaiveDate::parse_from_str(a, "%Y-%m-%d")
            .map_err(|_| Error::invalid_input(format!("Invalid date: {a}")))?;
        let db = NaiveDate::parse_from_str(b, "%Y-%m-%d")
            .map_err(|_| Error::invalid_input(format!("Invalid date: {b}")))?;
        Ok(da == db)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        let v = NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| Error::invalid_input(format!("Invalid date: {value}")))?;
        let fv = NaiveDate::parse_from_str(facet_value, "%Y-%m-%d")
            .map_err(|_| Error::invalid_input(format!("Invalid date: {facet_value}")))?;
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minInclusive" => Ok(v >= fv),
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => Ok(v <= fv),
            "http://www.w3.org/2001/XMLSchema#minExclusive" => Ok(v > fv),
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => Ok(v < fv),
            other => Err(Error::invalid_input(format!("Unsupported date facet: {other}"))),
        }
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

impl ValueSpaceHandler for TimeValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#time"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        NaiveTime::parse_from_str(value, "%H:%M:%S").is_ok()
            || NaiveTime::parse_from_str(value, "%H:%M:%S%.f").is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
            .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
            .map(|t| t.to_string())
            .unwrap_or_else(|_| value.to_string())
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let ta = parse_time(a)?;
        let tb = parse_time(b)?;
        Ok(ta == tb)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        let v = parse_time(value)?;
        let fv = parse_time(facet_value)?;
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minInclusive" => Ok(v >= fv),
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => Ok(v <= fv),
            "http://www.w3.org/2001/XMLSchema#minExclusive" => Ok(v > fv),
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => Ok(v < fv),
            other => Err(Error::invalid_input(format!("Unsupported time facet: {other}"))),
        }
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<FixedOffset>, Error> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| Error::invalid_input(format!("Invalid dateTime: {s}")))
}

fn parse_time(s: &str) -> Result<NaiveTime, Error> {
    NaiveTime::parse_from_str(s, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| Error::invalid_input(format!("Invalid time: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_valid() {
        let h = DateTimeValueSpace;
        assert!(h.is_valid_literal("2024-01-15T10:30:00+00:00"));
        assert!(!h.is_valid_literal("not-a-date"));
    }

    #[test]
    fn test_date_facet() {
        let h = DateValueSpace;
        assert!(h.satisfies_facet(
            "2024-06-01",
            "http://www.w3.org/2001/XMLSchema#minInclusive",
            "2024-01-01",
        ).unwrap());
    }
}
