//! Performance Profiling Infrastructure
//!
//! This module provides flamegraph generation, heap profiling, and performance
//! counter tracking to identify bottlenecks and optimize the reasoner.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(feature = "profiling")]
use pprof::ProfilerGuard;

/// Performance counter for tracking operations
#[derive(Debug, Clone, Default)]
pub struct PerformanceCounter {
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
}

impl PerformanceCounter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, duration: Duration) {
        self.count += 1;
        self.total_duration += duration;

        self.min_duration = Some(match self.min_duration {
            Some(min) if min < duration => min,
            _ => duration,
        });

        self.max_duration = Some(match self.max_duration {
            Some(max) if max > duration => max,
            _ => duration,
        });
    }

    #[must_use]
    pub fn average_duration(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.count as u32
        }
    }
}

/// Performance profiler for classification and reasoning operations
pub struct PerformanceProfiler {
    counters: Arc<Mutex<HashMap<String, PerformanceCounter>>>,
    #[cfg(feature = "profiling")]
    profiler_guard: Option<ProfilerGuard<'static>>,
}

impl std::fmt::Debug for PerformanceProfiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerformanceProfiler")
            .field("counters", &self.counters)
            .finish_non_exhaustive()
    }
}

impl PerformanceProfiler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "profiling")]
            profiler_guard: None,
        }
    }

    /// Start profiling (flamegraph generation)
    #[cfg(feature = "profiling")]
    pub fn start_profiling(&mut self, frequency: i32) -> Result<(), String> {
        use pprof::ProfilerGuardBuilder;

        let guard = ProfilerGuardBuilder::default()
            .frequency(frequency)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .map_err(|e| format!("Failed to start profiler: {}", e))?;

        self.profiler_guard = Some(guard);
        Ok(())
    }

    #[cfg(not(feature = "profiling"))]
    pub fn start_profiling(&mut self, _frequency: i32) -> Result<(), String> {
        Err("Profiling feature not enabled. Rebuild with --features profiling".to_string())
    }

    /// Stop profiling and generate flamegraph
    #[cfg(feature = "profiling")]
    pub fn stop_profiling_and_report(&mut self, output_path: &str) -> Result<(), String> {
        if let Some(guard) = self.profiler_guard.take() {
            let report = guard
                .report()
                .build()
                .map_err(|e| format!("Failed to build profiling report: {}", e))?;

            let file = std::fs::File::create(output_path)
                .map_err(|e| format!("Failed to create flamegraph file: {}", e))?;

            report
                .flamegraph(file)
                .map_err(|e| format!("Failed to generate flamegraph: {}", e))?;

            Ok(())
        } else {
            Err("No active profiling session".to_string())
        }
    }

    #[cfg(not(feature = "profiling"))]
    pub fn stop_profiling_and_report(&mut self, _output_path: &str) -> Result<(), String> {
        Err("Profiling feature not enabled".to_string())
    }

    /// Record a timed operation
    pub fn record_operation(&self, operation: &str, duration: Duration) {
        if let Ok(mut counters) = self.counters.lock() {
            let counter = counters
                .entry(operation.to_string())
                .or_insert_with(PerformanceCounter::new);
            counter.record(duration);
        }
    }

    /// Start timing an operation
    #[must_use]
    pub fn start_timer(&self, operation: &str) -> OperationTimer {
        OperationTimer {
            operation: operation.to_string(),
            start: Instant::now(),
            profiler: self.counters.clone(),
        }
    }

    /// Get all recorded counters
    #[must_use]
    pub fn get_counters(&self) -> HashMap<String, PerformanceCounter> {
        self.counters.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Print performance summary
    pub fn print_summary(&self) {
        if let Ok(counters) = self.counters.lock() {
            println!("\n=== Performance Summary ===");
            for (operation, counter) in counters.iter() {
                println!(
                    "{}: count={}, total={:?}, avg={:?}, min={:?}, max={:?}",
                    operation,
                    counter.count,
                    counter.total_duration,
                    counter.average_duration(),
                    counter.min_duration.unwrap_or(Duration::ZERO),
                    counter.max_duration.unwrap_or(Duration::ZERO)
                );
            }
            println!("========================\n");
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        if let Ok(mut counters) = self.counters.lock() {
            counters.clear();
        }
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII timer that records duration when dropped
pub struct OperationTimer {
    operation: String,
    start: Instant,
    profiler: Arc<Mutex<HashMap<String, PerformanceCounter>>>,
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        if let Ok(mut counters) = self.profiler.lock() {
            let counter = counters
                .entry(self.operation.clone())
                .or_insert_with(PerformanceCounter::new);
            counter.record(duration);
        }
    }
}

/// Heap profiling support using dhat
#[cfg(feature = "profiling")]
pub mod heap {
    use dhat::Profiler;

    pub struct HeapProfiler {
        _profiler: Profiler,
    }

    impl HeapProfiler {
        pub fn new() -> Self {
            Self {
                _profiler: Profiler::new_heap(),
            }
        }
    }

    impl Default for HeapProfiler {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "profiling"))]
pub mod heap {
    pub struct HeapProfiler;

    impl HeapProfiler {
        #[must_use]
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for HeapProfiler {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_counter() {
        let mut counter = PerformanceCounter::new();

        counter.record(Duration::from_millis(100));
        counter.record(Duration::from_millis(200));
        counter.record(Duration::from_millis(150));

        assert_eq!(counter.count, 3);
        assert_eq!(counter.min_duration, Some(Duration::from_millis(100)));
        assert_eq!(counter.max_duration, Some(Duration::from_millis(200)));
        assert_eq!(counter.average_duration(), Duration::from_millis(150));
    }

    #[test]
    fn test_operation_timer() {
        let profiler = PerformanceProfiler::new();

        {
            let _timer = profiler.start_timer("test_operation");
            thread::sleep(Duration::from_millis(10));
        }

        let counters = profiler.get_counters();
        assert_eq!(counters.get("test_operation").map(|c| c.count), Some(1));
    }

    #[test]
    fn test_profiler_record_operation() {
        let profiler = PerformanceProfiler::new();

        profiler.record_operation("op1", Duration::from_millis(100));
        profiler.record_operation("op1", Duration::from_millis(200));
        profiler.record_operation("op2", Duration::from_millis(50));

        let counters = profiler.get_counters();
        assert_eq!(counters.get("op1").map(|c| c.count), Some(2));
        assert_eq!(counters.get("op2").map(|c| c.count), Some(1));
    }
}
