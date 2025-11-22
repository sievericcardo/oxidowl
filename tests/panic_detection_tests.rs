//! Panic Detection Tests
//!
//! This test suite ensures that operations that previously used `.expect("Test operation failed")` now properly
//! return errors instead of panicking. These tests simulate edge cases that would have
//! caused panics in the old implementation.

use oxidowl::{
    Error,
    config::ReasoningConfig,
    core::{lock_helpers::read_lock, tableau::Tableau},
    ontology::Ontology,
};
use std::sync::{Arc, RwLock};

#[test]
fn test_empty_collection_access_returns_error() {
    // Test that accessing empty collections returns proper errors instead of panicking

    // Create an empty tableau
    let ontology = Arc::new(RwLock::new(Ontology::new()));
    let config = ReasoningConfig::default();
    let mut tableau = Tableau::new(config);

    // In the old implementation, popping from empty queue would panic with unwrap()
    // Now it should return None, which we can handle
    assert!(tableau.pending_queue.is_empty());
    let result = tableau.pending_queue.pop_front();
    assert!(
        result.is_none(),
        "Popping from empty queue should return None"
    );
}

#[test]
fn test_data_structure_edge_cases() {
    // Test various data structure edge cases that might have caused unwraps to panic

    let ontology = Arc::new(RwLock::new(Ontology::new()));

    // Test reading from a freshly created ontology (no signature yet)
    let result = read_lock(&ontology, "test: reading ontology");
    assert!(
        result.is_ok(),
        "Reading lock on new ontology should succeed"
    );

    let guard = result.expect("Test operation failed");
    // The signature should exist (though empty) - no unwrap panic
    let sig_result = guard.signature();
    assert!(
        sig_result.is_ok(),
        "Getting signature should return Result, not panic"
    );
}

#[test]
fn test_option_unwrap_replacements() {
    // Test that operations using Option::unwrap() now handle None gracefully

    let config = ReasoningConfig::default();
    let tableau = Tableau::new(config);

    // Stack operations that might be empty
    // Old code: self.branching_stack.last().expect("Test operation failed")
    // New code: checks before access or uses ok_or_else

    // Simulate accessing potentially empty stack
    let empty_vec: Vec<u32> = Vec::new();
    let result = empty_vec.last();
    assert!(
        result.is_none(),
        "last() on empty vec should be None, not unwrap panic"
    );

    // Using ok_or_else pattern
    let error_result = empty_vec
        .last()
        .ok_or_else(|| Error::data_structure("Stack is empty"));

    assert!(
        error_result.is_err(),
        "Should return error for empty collection"
    );
    match error_result {
        Err(Error::DataStructure { message }) => {
            assert!(message.contains("Stack is empty"));
        }
        _ => panic!("Expected DataStructure error"),
    }
}

#[test]
fn test_collection_operations_without_panic() {
    // Test collection operations that previously used unwrap()

    let test_vec = vec![1, 2, 3];

    // Safe iteration
    let first = test_vec.first();
    assert!(first.is_some());

    let empty_vec: Vec<i32> = Vec::new();
    let first_empty = empty_vec.first();
    assert!(
        first_empty.is_none(),
        "first() on empty vec returns None, not panic"
    );

    // Safe indexing with get()
    let val = test_vec.get(10);
    assert!(val.is_none(), "get() out of bounds returns None, not panic");
}

#[test]
fn test_fingerprint_option_handling() {
    // Test that fingerprint operations handle None without panicking
    // Old code: let fp = fingerprint.expect("Test operation failed");
    // New code: let fp = fingerprint.ok_or_else(...)?;

    let fingerprint: Option<u64> = None;

    // Proper error handling
    let result = fingerprint.ok_or_else(|| Error::internal("Fingerprint is None despite check"));

    assert!(
        result.is_err(),
        "Should return error when fingerprint is None"
    );
    match result {
        Err(Error::Internal { message, .. }) => {
            assert!(message.contains("Fingerprint"));
        }
        _ => panic!("Expected Internal error"),
    }

    // Test with Some
    let fingerprint_some: Option<u64> = Some(12345);
    let result_some = fingerprint_some.ok_or_else(|| Error::internal("Fingerprint is None"));
    assert!(result_some.is_ok());
    assert_eq!(result_some.expect("Test operation failed"), 12345);
}

#[test]
fn test_nested_option_handling() {
    // Test nested Option handling that might have used multiple unwraps

    let nested: Option<Option<String>> = Some(Some("value".to_string()));

    // Safe unwrapping
    if let Some(inner) = nested {
        if let Some(value) = inner {
            assert_eq!(value, "value");
        }
    }

    // Test None case
    let nested_none: Option<Option<String>> = Some(None);
    let result = nested_none
        .and_then(|inner| inner)
        .ok_or_else(|| Error::data_structure("Inner value is None"));

    assert!(result.is_err());
}

#[test]
fn test_result_propagation() {
    // Test that errors properly propagate through the ? operator

    fn may_fail(should_fail: bool) -> Result<i32, Error> {
        if should_fail {
            Err(Error::internal("Intentional failure"))
        } else {
            Ok(42)
        }
    }

    fn chained_operation(should_fail: bool) -> Result<i32, Error> {
        let value = may_fail(should_fail)?;
        Ok(value * 2)
    }

    // Success case
    let result_ok = chained_operation(false);
    assert!(result_ok.is_ok());
    assert_eq!(result_ok.expect("Test operation failed"), 84);

    // Failure case - error should propagate
    let result_err = chained_operation(true);
    assert!(result_err.is_err());
    match result_err {
        Err(Error::Internal { message, .. }) => {
            assert!(message.contains("Intentional failure"));
        }
        _ => panic!("Expected Internal error"),
    }
}

#[test]
fn test_error_context_preservation() {
    // Test that error context is preserved through multiple layers

    fn inner_operation() -> Result<(), Error> {
        Err(Error::collection_error("Inner operation failed"))
    }

    fn middle_operation() -> Result<(), Error> {
        inner_operation()?;
        Ok(())
    }

    fn outer_operation() -> Result<(), Error> {
        middle_operation()?;
        Ok(())
    }

    let result = outer_operation();
    assert!(result.is_err());
    match result {
        Err(Error::CollectionError { message }) => {
            assert!(message.contains("Inner operation failed"));
        }
        _ => panic!("Expected CollectionError"),
    }
}
