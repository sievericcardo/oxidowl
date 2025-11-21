# Error Handling Guide - oxidowl 0.6.0

## Quick Reference

### Lock Operations

```rust
use oxidowl::prelude::*;

// Read lock
let data = read_lock(&self.lock, "module: operation")?;

// Write lock
let mut data = write_lock(&self.lock, "module: operation")?;
```

### Error Construction

```rust
// Internal errors
Error::internal("description")

// Lock errors
Error::lock_poisoned("context")

// Data structure errors
Error::data_structure("what went wrong")

// Collection errors
Error::collection_error("operation failed")

// System errors
Error::system_error("system operation failed")

// Parse errors
Error::parse_error("parsing failed")
```

### Error Propagation

```rust
// Use ? operator
let result = operation()?;

// Chain operations
operation1()?
    .operation2()?
    .operation3()?
```

### Common Patterns

#### Pattern 1: Simple Lock Access
```rust
fn get_data(&self) -> Result<Data, Error> {
    let lock = read_lock(&self.data, "get_data: reading")?;
    Ok(lock.clone())
}
```

#### Pattern 2: Lock Modification
```rust
fn update_data(&self, new_data: Data) -> Result<(), Error> {
    let mut lock = write_lock(&self.data, "update_data: writing")?;
    *lock = new_data;
    Ok(())
}
```

#### Pattern 3: Multiple Locks
```rust
fn compare(&self, other: &Self) -> Result<bool, Error> {
    let a = read_lock(&self.data, "compare: self")?;
    let b = read_lock(&other.data, "compare: other")?;
    Ok(*a == *b)
}
```

#### Pattern 4: Async Operations
```rust
async fn process(&self) -> Result<Output, Error> {
    let data = read_lock(&self.data, "process: reading")?;
    let result = async_operation(&data).await?;
    Ok(result)
}
```

## Error Context Best Practices

### Good Context Strings

```rust
read_lock(&lock, "classification: building class hierarchy")
read_lock(&lock, "reasoner: checking consistency")
read_lock(&lock, "swrl: validating rule safety")
```

### Poor Context Strings

```rust
read_lock(&lock, "error")
read_lock(&lock, "lock")
read_lock(&lock, "reading")
```

### Context Format

Use the format: `"module/component: specific operation"`

Examples:
- `"classification: computing equivalences"`
- `"tableau: expanding node"`
- `"import_resolver: loading dependencies"`
- `"performance_monitor: recording statistics"`

## Error Handling in Different Contexts

### Synchronous Code

```rust
fn synchronous_operation(&self) -> Result<Output, Error> {
    let data = read_lock(&self.data, "sync: operation")?;
    let processed = process(&data)?;
    Ok(processed)
}
```

### Asynchronous Code

```rust
async fn async_operation(&self) -> Result<Output, Error> {
    let data = read_lock(&self.data, "async: operation")?;
    let result = some_async_work(&data).await?;
    Ok(result)
}
```

### Iterator Operations

```rust
fn process_all(&self) -> Result<Vec<Output>, Error> {
    let data = read_lock(&self.data, "process_all: reading")?;
    
    data.items()
        .iter()
        .map(|item| self.process_item(item))
        .collect::<Result<Vec<_>, _>>()
}
```

### Option Handling

```rust
// Convert Option to Result
let value = option_value
    .ok_or_else(|| Error::internal("value not found"))?;

// Chain with other operations
let result = get_optional()?
    .ok_or_else(|| Error::data_structure("missing data"))?
    .process()?;
```

## Performance Monitoring

All performance monitoring methods now return `Result`:

```rust
// Recording operations
monitor.record_operation("classify", duration)?;

// Getting statistics
let stats = monitor.get_statistics()?;

// Resetting
monitor.reset()?;
```

## Testing Error Handling

### Unit Tests

```rust
#[test]
fn test_error_handling() -> Result<(), Error> {
    let component = Component::new();
    let result = component.operation()?;
    assert_eq!(result, expected);
    Ok(())
}
```

### Error Case Tests

```rust
#[test]
fn test_lock_poisoning() {
    let data = Arc::new(RwLock::new(Vec::new()));
    
    // Poison the lock
    let data_clone = data.clone();
    let _ = std::thread::spawn(move || {
        let _guard = data_clone.write().unwrap();
        panic!("Poison");
    }).join();
    
    // Verify error handling
    let result = read_lock(&data, "test: poisoned lock");
    assert!(result.is_err());
    assert!(matches!(result, Err(Error::LockPoisoned { .. })));
}
```

## Migration from 0.5.x

### Before (Panic-Prone)

```rust
fn old_code(&self) {
    let data = self.lock.read().unwrap();
    // Use data...
}
```

### After (Safe)

```rust
fn new_code(&self) -> Result<(), Error> {
    let data = read_lock(&self.lock, "context")?;
    // Use data...
    Ok(())
}
```

## Common Mistakes to Avoid

### Mistake 1: Missing Result Return Type

```rust
// Wrong - will not compile
fn my_function(&self) {
    let data = read_lock(&self.lock, "context")?;  // Error!
}
```

```rust
// Correct
fn my_function(&self) -> Result<(), Error> {
    let data = read_lock(&self.lock, "context")?;
    Ok(())
}
```

### Mistake 2: Generic Context Strings

```rust
// Wrong - not helpful for debugging
read_lock(&lock, "error")
```

```rust
// Correct - descriptive and specific
read_lock(&lock, "classification: reading class hierarchy")
```

### Mistake 3: Swallowing Errors

```rust
// Wrong - loses error information
if let Ok(data) = read_lock(&lock, "context") {
    // Use data...
}
// Error silently ignored!
```

```rust
// Correct - propagate errors
let data = read_lock(&lock, "context")?;
// Use data...
```

### Mistake 4: Using Old Error Syntax

```rust
// Wrong - struct syntax no longer works
Error::Internal { message: "error".into() }
```

```rust
// Correct - use constructor function
Error::internal("error")
```

## Debugging Tips

### 1. Enable Detailed Logging

The context strings appear in error messages:

```
Error: Lock poisoned: classification: reading class hierarchy
```

### 2. Use Descriptive Contexts

Help yourself debug by being specific:

```rust
read_lock(&self.ontology, 
         "reasoner::classify::phase1: building initial hierarchy")
```

### 3. Check Error Variants

Pattern match to handle specific errors:

```rust
match operation() {
    Ok(result) => handle_success(result),
    Err(Error::LockPoisoned { message }) => {
        log::error!("Lock poisoned: {}", message);
        retry_operation()
    }
    Err(e) => Err(e),
}
```

## Summary

- Always use `read_lock()` and `write_lock()` helpers
- Provide descriptive context strings
- Return `Result<T, Error>` from functions using `?`
- Use constructor functions for errors
- Propagate errors with `?` operator
- Test error handling paths
- Import from prelude: `use oxidowl::prelude::*;`

For more details, see:

- [API Documentation](https://docs.rs/oxidowl)
