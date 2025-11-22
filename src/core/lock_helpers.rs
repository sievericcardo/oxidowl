//! Lock helper utilities for safe RwLock access
//!
//! This module provides helper functions that convert `PoisonError` from RwLock
//! operations into proper `Error` types with context and backtraces (in debug builds).
//!
//! # Usage
//!
//! ```rust
//! use oxidowl::prelude::*;
//! use std::sync::RwLock;
//!
//! # fn main() -> Result<()> {
//! let data = RwLock::new(vec![1, 2, 3]);
//!
//! // Instead of:
//! // let guard = data.read().expect("Failed to complete operation successfully");
//!
//! // Use:
//! let guard = read_lock(&data, "accessing data")?;
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Acquire a read lock on an RwLock with proper error handling
///
/// This function attempts to acquire a read lock on the provided RwLock.
/// If the lock is poisoned, it returns a `LockPoisoned` error with context
/// and a backtrace (in debug builds).
///
/// # Arguments
///
/// * `lock` - The RwLock to acquire a read lock on
/// * `context` - Contextual information about what is being locked (used in error messages)
///
/// # Returns
///
/// Returns a `RwLockReadGuard` on success, or an `Error::LockPoisoned` if the lock is poisoned.
///
/// # Examples
///
/// ```rust,no_run
/// use oxidowl::core::lock_helpers::read_lock;
/// use std::sync::RwLock;
///
/// let data = RwLock::new(42);
/// let guard = read_lock(&data, "reading counter")?;
/// println!("Value: {}", *guard);
/// # Ok::<(), oxidowl::error::Error>(())
/// ```
pub fn read_lock<'a, T>(lock: &'a RwLock<T>, context: &str) -> Result<RwLockReadGuard<'a, T>> {
    lock.read()
        .map_err(|e| Error::lock_poisoned(format!("Read lock poisoned: {} - {}", context, e)))
}

/// Acquire a write lock on an RwLock with proper error handling
///
/// This function attempts to acquire a write lock on the provided RwLock.
/// If the lock is poisoned, it returns a `LockPoisoned` error with context
/// and a backtrace (in debug builds).
///
/// # Arguments
///
/// * `lock` - The RwLock to acquire a write lock on
/// * `context` - Contextual information about what is being locked (used in error messages)
///
/// # Returns
///
/// Returns a `RwLockWriteGuard` on success, or an `Error::LockPoisoned` if the lock is poisoned.
///
/// # Examples
///
/// ```rust,no_run
/// use oxidowl::core::lock_helpers::write_lock;
/// use std::sync::RwLock;
///
/// let data = RwLock::new(42);
/// let mut guard = write_lock(&data, "updating counter")?;
/// *guard += 1;
/// # Ok::<(), oxidowl::error::Error>(())
/// ```
pub fn write_lock<'a, T>(lock: &'a RwLock<T>, context: &str) -> Result<RwLockWriteGuard<'a, T>> {
    lock.write()
        .map_err(|e| Error::lock_poisoned(format!("Write lock poisoned: {} - {}", context, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_read_lock_success() {
        let data = RwLock::new(42);
        let guard = read_lock(&data, "test read").expect("Failed to acquire read lock in test");
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_write_lock_success() {
        let data = RwLock::new(42);
        let mut guard =
            write_lock(&data, "test write").expect("Failed to acquire write lock in test");
        *guard = 100;
        drop(guard);

        let guard = read_lock(&data, "test read after write")
            .expect("Failed to acquire read lock after write in test");
        assert_eq!(*guard, 100);
    }

    #[test]
    fn test_poisoned_lock_error() {
        let data = Arc::new(RwLock::new(42));
        let data_clone = Arc::clone(&data);

        // Poison the lock by panicking while holding it
        let handle = thread::spawn(move || {
            let _guard = data_clone
                .write()
                .expect("Failed to acquire write lock for poisoning test");
            panic!("Intentional panic to poison lock");
        });

        // Wait for the thread to panic
        let _ = handle.join();

        // Now try to acquire the lock - should get LockPoisoned error
        let result = read_lock(&data, "reading poisoned lock");
        assert!(result.is_err());

        if let Err(Error::LockPoisoned { message, .. }) = result {
            assert!(message.contains("Read lock poisoned"));
            assert!(message.contains("reading poisoned lock"));
        } else {
            panic!("Expected LockPoisoned error");
        }
    }

    #[test]
    fn test_write_lock_poisoned() {
        let data = Arc::new(RwLock::new(42));
        let data_clone = Arc::clone(&data);

        // Poison the lock
        let handle = thread::spawn(move || {
            let _guard = data_clone
                .write()
                .expect("Failed to acquire write lock for poisoning test");
            panic!("Intentional panic");
        });
        let _ = handle.join();

        // Try to acquire write lock
        let result = write_lock(&data, "writing to poisoned lock");
        assert!(result.is_err());

        if let Err(Error::LockPoisoned { message, .. }) = result {
            assert!(message.contains("Write lock poisoned"));
            assert!(message.contains("writing to poisoned lock"));
        } else {
            panic!("Expected LockPoisoned error");
        }
    }
}
