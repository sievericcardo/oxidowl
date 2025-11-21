//! Error Propagation Tests
//!
//! This test suite validates that errors propagate correctly through the system
//! with consistent messaging format and proper error types.

use oxidowl::{
    Error,
    core::lock_helpers::{read_lock, write_lock},
};
use std::sync::RwLock;

#[test]
fn test_error_format_consistency() {
    // Test that all error types follow the simple "{module}: {context}" format

    let test_cases = vec![
        (
            Error::lock_poisoned("classification: reading ontology"),
            "Lock poisoned: classification: reading ontology",
        ),
        (
            Error::data_structure("tableau: stack empty"),
            "Data structure error: tableau: stack empty",
        ),
        (
            Error::collection_error("expansion: queue exhausted"),
            "Collection error: expansion: queue exhausted",
        ),
        (
            Error::system_error("performance: time calculation failed"),
            "System error: performance: time calculation failed",
        ),
        (
            Error::internal("reasoner: unexpected state"),
            "Internal logic error: reasoner: unexpected state",
        ),
    ];

    for (error, expected_message) in test_cases {
        let error_string = error.to_string();
        assert_eq!(
            error_string, expected_message,
            "Error format should be consistent"
        );
    }
}

#[test]
fn test_error_categories() {
    // Test that errors are categorized correctly

    use oxidowl::error::ErrorCategory;

    assert_eq!(
        Error::lock_poisoned("test").category(),
        ErrorCategory::Internal
    );
    assert_eq!(
        Error::data_structure("test").category(),
        ErrorCategory::Internal
    );
    assert_eq!(
        Error::collection_error("test").category(),
        ErrorCategory::Internal
    );
    assert_eq!(
        Error::system_error("test").category(),
        ErrorCategory::Internal
    );
    assert_eq!(
        Error::reasoning("test").category(),
        ErrorCategory::Reasoning
    );
    assert_eq!(
        Error::ontology_parsing("test").category(),
        ErrorCategory::Input
    );
}

#[test]
fn test_error_recoverability() {
    // Test that errors have appropriate recoverability flags

    // Non-recoverable errors
    assert!(!Error::lock_poisoned("test").is_recoverable());
    assert!(!Error::data_structure("test").is_recoverable());
    assert!(!Error::collection_error("test").is_recoverable());
    assert!(!Error::system_error("test").is_recoverable());
    assert!(!Error::internal("test").is_recoverable());

    // Recoverable errors
    assert!(Error::reasoning("test").is_recoverable());
}

#[test]
fn test_error_chain_propagation() {
    // Test that errors propagate through multiple function calls

    fn level3() -> Result<i32, Error> {
        Err(Error::data_structure("level3: data not found"))
    }

    fn level2() -> Result<i32, Error> {
        level3()?;
        Ok(42)
    }

    fn level1() -> Result<i32, Error> {
        level2()?;
        Ok(100)
    }

    let result = level1();

    assert!(result.is_err());
    match result {
        Err(Error::DataStructure { message }) => {
            assert!(message.contains("level3"));
            assert!(message.contains("data not found"));
        }
        _ => panic!("Expected DataStructure error"),
    }
}

#[test]
fn test_lock_helper_error_messages() {
    // Test that lock helpers produce consistent error messages

    let data = RwLock::new(42);

    // Test successful read lock
    {
        let result = read_lock(&data, "test module: reading value");
        assert!(result.is_ok());
        // Read lock is dropped here when going out of scope
    }

    // Test successful write lock (after read lock is dropped)
    {
        let result = write_lock(&data, "test module: writing value");
        assert!(result.is_ok());
        // Write lock is dropped here when going out of scope
    }
}

#[test]
fn test_result_conversion() {
    // Test that different error types can be converted and propagated

    fn operation_that_fails() -> Result<(), Error> {
        // Simulate IO error conversion
        let _file = std::fs::File::open("/nonexistent/path/file.txt")?;
        Ok(())
    }

    let result = operation_that_fails();
    assert!(result.is_err());

    // Should be converted to Error::Io
    match result {
        Err(Error::Io { .. }) => {
            // Success - IO error was converted properly
        }
        _ => panic!("Expected Io error from From<std::io::Error> impl"),
    }
}

#[test]
fn test_error_display_vs_debug() {
    // Test that Display and Debug provide different levels of information

    let error = Error::internal("test: internal error");

    // Display should be user-friendly
    let display_str = format!("{}", error);
    assert!(display_str.contains("Internal logic error"));
    assert!(display_str.contains("test: internal error"));

    // Debug should include more details
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Internal"));

    // Note: Backtrace is not automatically included in thiserror-based errors
    // unless explicitly configured with backtrace feature
}

#[test]
fn test_option_to_result_patterns() {
    // Test common patterns for converting Option to Result

    // Pattern 1: ok_or_else with error constructor
    let opt: Option<i32> = None;
    let result = opt.ok_or_else(|| Error::collection_error("pattern1: value missing"));
    assert!(result.is_err());

    // Pattern 2: ok_or with pre-constructed error
    let opt2: Option<String> = None;
    let result2 = opt2.ok_or(Error::data_structure("pattern2: string not found"));
    assert!(result2.is_err());

    // Pattern 3: map_or_else for transformation
    let opt3: Option<Vec<u8>> = Some(vec![1, 2, 3]);
    let result3 = opt3.map_or_else(
        || Err(Error::collection_error("pattern3: vec missing")),
        |v| Ok(v.len()),
    );
    assert!(result3.is_ok());
    assert_eq!(result3.unwrap(), 3);
}

#[test]
fn test_error_constructors() {
    // Test that all error constructors work correctly

    let _e1 = Error::ontology_parsing("test");
    let _e2 = Error::reasoning("test");
    let _e3 = Error::config("test");
    let _e4 = Error::network("test");
    let _e5 = Error::io("test");
    let _e6 = Error::xml_parsing("test");
    let _e7 = Error::sparql("test");
    let _e8 = Error::cache("test");
    let _e9 = Error::dl_query("test");
    let _e10 = Error::resource_exhaustion("test");
    let _e11 = Error::timeout("test");
    let _e12 = Error::unsupported("test");
    let _e13 = Error::internal("test");
    let _e14 = Error::invalid_input("test");
    let _e15 = Error::lock_poisoned("test");
    let _e16 = Error::data_structure("test");
    let _e17 = Error::collection_error("test");
    let _e18 = Error::system_error("test");

    // If we got here, all constructors work
}

#[test]
fn test_error_in_iterator_chain() {
    // Test that errors work well with iterator combinators

    fn process_item(n: i32) -> Result<i32, Error> {
        if n < 0 {
            Err(Error::invalid_input("negative number"))
        } else {
            Ok(n * 2)
        }
    }

    let items = vec![1, 2, 3];
    let results: Result<Vec<i32>, Error> = items.into_iter().map(process_item).collect();

    assert!(results.is_ok());
    assert_eq!(results.unwrap(), vec![2, 4, 6]);

    // Test with error
    let items_with_error = vec![1, -2, 3];
    let results_err: Result<Vec<i32>, Error> =
        items_with_error.into_iter().map(process_item).collect();

    assert!(results_err.is_err());
}

#[test]
fn test_nested_error_handling() {
    // Test nested Result handling

    fn inner() -> Result<Option<i32>, Error> {
        Ok(Some(42))
    }

    fn middle() -> Result<i32, Error> {
        let opt = inner()?;
        opt.ok_or_else(|| Error::data_structure("middle: value missing"))
    }

    fn outer() -> Result<String, Error> {
        let value = middle()?;
        Ok(format!("Value: {}", value))
    }

    let result = outer();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Value: 42");
}

#[test]
fn test_error_with_context_string_quality() {
    // Test that context strings in errors are descriptive

    let errors = vec![
        Error::lock_poisoned("classification: reading ontology for subsumption check"),
        Error::data_structure("tableau: branching stack empty during backtrack"),
        Error::collection_error("expansion: pending queue exhausted before completion"),
        Error::system_error("performance: failed to capture timestamp"),
    ];

    for error in errors {
        let error_str = error.to_string();

        // Should contain module name
        assert!(
            error_str.contains("classification")
                || error_str.contains("tableau")
                || error_str.contains("expansion")
                || error_str.contains("performance"),
            "Error should contain module name"
        );

        // Should contain operation context
        assert!(
            error_str.split(':').count() >= 2,
            "Error should have module and context"
        );
    }
}

#[test]
fn test_error_size() {
    // Ensure error types don't get too large (impacts performance)

    use std::mem::size_of;

    let error_size = size_of::<Error>();

    // Error should be reasonable size (not too bloated)
    // With backtrace in debug, it will be larger
    #[cfg(debug_assertions)]
    {
        // In debug, backtrace adds significant size, but still should be manageable
        assert!(
            error_size < 1024,
            "Error size should be < 1KB even with backtrace"
        );
    }

    #[cfg(not(debug_assertions))]
    {
        // In release, without backtrace, should be much smaller
        assert!(
            error_size < 256,
            "Error size should be < 256 bytes in release"
        );
    }
}

#[test]
fn test_result_type_alias() {
    // Test that the Result type alias works correctly

    fn operation() -> oxidowl::Result<i32> {
        Ok(42)
    }

    fn failing_operation() -> oxidowl::Result<i32> {
        Err(Error::internal("failed"))
    }

    assert!(operation().is_ok());
    assert!(failing_operation().is_err());
}

#[test]
fn test_error_cloning() {
    // Test that errors can be cloned (important for caching/logging)

    let error1 = Error::reasoning("test error");
    let error2 = error1.clone();

    assert_eq!(error1.to_string(), error2.to_string());
}

#[test]
fn test_error_send_sync() {
    // Test that errors are Send + Sync (required for multi-threading)

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<Error>();
    assert_sync::<Error>();
}
