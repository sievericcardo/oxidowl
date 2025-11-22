//! Simple test for the year-month duration parser function

#[cfg(test)]
mod simple_tests {
    use oxidowl::swrl::datetime_builtins::parse_year_month_duration;

    #[test]
    fn test_year_month_duration_parser() {
        // Test valid duration formats
        assert_eq!(
            parse_year_month_duration("P1Y").expect("Test operation failed"),
            (1, 0)
        );
        assert_eq!(
            parse_year_month_duration("P6M").expect("Test operation failed"),
            (0, 6)
        );
        assert_eq!(
            parse_year_month_duration("P2Y5M").expect("Test operation failed"),
            (2, 5)
        );
        assert_eq!(
            parse_year_month_duration("P0Y0M").expect("Test operation failed"),
            (0, 0)
        );

        // Test invalid formats
        assert!(parse_year_month_duration("1Y").is_err()); // Missing P
        assert!(parse_year_month_duration("PY").is_err()); // Missing number
        assert!(parse_year_month_duration("P1D").is_err()); // Invalid unit
    }
}
