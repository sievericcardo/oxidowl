//! Lock Poisoning Tests
//!
//! This test suite ensures that poisoned RwLocks are handled gracefully with proper
//! error messages and backtraces, rather than propagating panics or ignoring the poison.

use oxidowl::{
    Error,
    core::lock_helpers::{read_lock, write_lock},
};
use std::sync::{Arc, RwLock};
use std::thread;

#[test]
fn test_read_lock_poisoned() {
    let data = Arc::new(RwLock::new(vec![1, 2, 3]));
    let data_clone = Arc::clone(&data);

    // Poison the lock by panicking while holding it
    let handle = thread::spawn(move || {
        let _guard = data_clone.write().expect("Lock for poisoning");
        panic!("Intentional panic to poison lock");
    });

    // Wait for the thread to panic
    let _ = handle.join();

    // Now try to acquire a read lock - should return LockPoisoned error
    let result = read_lock(&data, "test: reading poisoned lock");

    assert!(result.is_err(), "Reading poisoned lock should return error");

    match result {
        Err(Error::LockPoisoned { message, .. }) => {
            assert!(
                message.contains("Read lock poisoned"),
                "Error message should mention read lock"
            );
            assert!(
                message.contains("test: reading poisoned lock"),
                "Error should include context"
            );
        }
        Err(e) => panic!("Expected LockPoisoned error, got: {:?}", e),
        Ok(_) => panic!("Should not succeed reading poisoned lock"),
    }
}

#[test]
fn test_write_lock_poisoned() {
    let data = Arc::new(RwLock::new(42));
    let data_clone = Arc::clone(&data);

    // Poison the lock
    let handle = thread::spawn(move || {
        let _guard = data_clone.write().expect("Lock for poisoning");
        panic!("Poisoning the lock");
    });

    let _ = handle.join();

    // Try to acquire a write lock
    let result = write_lock(&data, "test: writing to poisoned lock");

    assert!(
        result.is_err(),
        "Writing to poisoned lock should return error"
    );

    match result {
        Err(Error::LockPoisoned { message, .. }) => {
            assert!(
                message.contains("Write lock poisoned"),
                "Error message should mention write lock"
            );
            assert!(
                message.contains("test: writing to poisoned lock"),
                "Error should include context"
            );
        }
        Err(e) => panic!("Expected LockPoisoned error, got: {:?}", e),
        Ok(_) => panic!("Should not succeed writing to poisoned lock"),
    }
}

#[test]
fn test_lock_poisoning_error_propagation() {
    // Test that lock poisoning errors propagate correctly through ? operator

    fn operation_with_poisoned_lock(lock: &RwLock<i32>) -> Result<i32, Error> {
        let guard = read_lock(lock, "operation: reading value")?;
        Ok(*guard)
    }

    let data = Arc::new(RwLock::new(100));
    let data_clone = Arc::clone(&data);

    // Poison the lock
    let handle = thread::spawn(move || {
        let _guard = data_clone.write().expect("Lock for poisoning");
        panic!("Poison!");
    });
    let _ = handle.join();

    // Call function that uses ? operator
    let result = operation_with_poisoned_lock(&data);

    assert!(result.is_err());
    match result {
        Err(Error::LockPoisoned { .. }) => {
            // Success - error propagated correctly
        }
        _ => panic!("Expected LockPoisoned error to propagate"),
    }
}

#[test]
fn test_multiple_threads_with_poisoned_lock() {
    // Test that multiple threads all receive the LockPoisoned error

    let data = Arc::new(RwLock::new(String::from("test")));
    let data_clone = Arc::clone(&data);

    // Poison the lock
    let poison_handle = thread::spawn(move || {
        let _guard = data_clone.write().expect("Lock for poisoning");
        panic!("Poison the lock");
    });
    let _ = poison_handle.join();

    // Spawn multiple threads trying to access the poisoned lock
    let mut handles = vec![];

    for i in 0..5 {
        let data_clone = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let result = read_lock(&data_clone, &format!("thread {}: reading", i));
            assert!(result.is_err(), "Thread {} should see poisoned lock", i);

            match result {
                Err(Error::LockPoisoned { .. }) => true,
                _ => false,
            }
        });
        handles.push(handle);
    }

    // All threads should detect the poison
    for handle in handles {
        let thread_result = handle.join().expect("Thread should complete");
        assert!(thread_result, "Thread should detect poisoned lock");
    }
}

#[test]
fn test_lock_poison_with_nested_operations() {
    // Test that poison is detected even in nested lock operations

    fn nested_operation(lock1: &RwLock<i32>, lock2: &RwLock<i32>) -> Result<i32, Error> {
        let guard1 = read_lock(lock1, "nested: lock1")?;
        let guard2 = read_lock(lock2, "nested: lock2")?;
        Ok(*guard1 + *guard2)
    }

    let lock1 = Arc::new(RwLock::new(10));
    let lock2 = Arc::new(RwLock::new(20));
    let lock2_clone = Arc::clone(&lock2);

    // Poison lock2
    let handle = thread::spawn(move || {
        let _guard = lock2_clone.write().expect("Lock for poisoning");
        panic!("Poison lock2");
    });
    let _ = handle.join();

    // Try nested operation - should fail on lock2
    let result = nested_operation(&lock1, &lock2);

    assert!(result.is_err());
    match result {
        Err(Error::LockPoisoned { message, .. }) => {
            assert!(message.contains("lock2"), "Should mention lock2 in error");
        }
        _ => panic!("Expected LockPoisoned error"),
    }
}

#[test]
fn test_lock_helpers_with_normal_operations() {
    // Test that lock helpers work correctly with normal (non-poisoned) locks

    let data = Arc::new(RwLock::new(42));

    // Read lock should work
    let read_result = read_lock(&data, "test: normal read");
    assert!(read_result.is_ok());
    assert_eq!(*read_result.expect("Test operation failed"), 42);

    // Write lock should work
    {
        let write_result = write_lock(&data, "test: normal write");
        assert!(write_result.is_ok());
        *write_result.expect("Test operation failed") = 100;
    }

    // Verify write
    let read_again = read_lock(&data, "test: verify write");
    assert!(read_again.is_ok());
    assert_eq!(*read_again.expect("Test operation failed"), 100);
}

#[test]
fn test_concurrent_readers_no_poison() {
    // Test that multiple concurrent readers work without issues

    let data = Arc::new(RwLock::new(vec![1, 2, 3, 4, 5]));
    let mut handles = vec![];

    // Spawn multiple reader threads
    for i in 0..10 {
        let data_clone = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let result = read_lock(&data_clone, &format!("reader {}", i));
            assert!(result.is_ok(), "Reader {} should succeed", i);

            let guard = result.expect("Test operation failed");
            assert_eq!(guard.len(), 5, "Reader {} should see 5 elements", i);
            guard.clone()
        });
        handles.push(handle);
    }

    // All readers should succeed
    for handle in handles {
        let data = handle.join().expect("Reader thread should complete");
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }
}

#[test]
fn test_error_message_quality() {
    // Test that error messages are descriptive and helpful

    let data = Arc::new(RwLock::new("test data"));
    let data_clone = Arc::clone(&data);

    // Poison the lock
    let handle = thread::spawn(move || {
        let _guard = data_clone.write().expect("Lock for poisoning");
        panic!("Test poison");
    });
    let _ = handle.join();

    // Check error message quality
    let result = read_lock(&data, "important operation: loading configuration");

    match result {
        Err(Error::LockPoisoned { message, .. }) => {
            // Error should mention:
            // 1. That it's a read lock
            assert!(message.contains("Read lock"), "Should mention read lock");

            // 2. That it's poisoned
            assert!(message.contains("poisoned"), "Should mention poisoned");

            // 3. The context we provided
            assert!(
                message.contains("important operation"),
                "Should include operation context"
            );
            assert!(
                message.contains("loading configuration"),
                "Should include detailed context"
            );

            println!("Lock poisoning error message: {}", message);
        }
        _ => panic!("Expected LockPoisoned error with descriptive message"),
    }
}
