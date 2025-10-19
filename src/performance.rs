//! Performance Monitoring System
//!
//! This module provides comprehensive performance monitoring capabilities including:
//! - Memory usage tracking (heap, cache, reasoning state)
//! - Query execution profiling and timing
//! - System resource monitoring (CPU, memory)
//! - Performance metrics collection and reporting

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Memory usage snapshot
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// Total heap allocated memory in bytes
    pub heap_allocated: usize,
    /// Cache memory usage in bytes
    pub cache_size: usize,
    /// Reasoning state memory in bytes
    pub reasoning_state_size: usize,
    /// System available memory in bytes
    pub system_available: usize,
    /// Timestamp of snapshot
    pub timestamp: Instant,
}

impl MemorySnapshot {
    /// Get total memory usage in bytes
    #[must_use]
    pub fn total_used(&self) -> usize {
        self.heap_allocated + self.cache_size + self.reasoning_state_size
    }

    /// Get total memory usage in megabytes
    #[must_use]
    pub fn total_used_mb(&self) -> f64 {
        self.total_used() as f64 / (1024.0 * 1024.0)
    }

    /// Get system available memory in megabytes
    #[must_use]
    pub fn system_available_mb(&self) -> f64 {
        self.system_available as f64 / (1024.0 * 1024.0)
    }
}

/// Memory tracker with platform-specific implementations
#[derive(Debug, Clone)]
pub struct MemoryTracker {
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    max_snapshots: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker
    #[must_use]
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots,
        }
    }

    /// Take a memory snapshot
    pub fn snapshot(&self, cache_size: usize, reasoning_state_size: usize) -> MemorySnapshot {
        let snapshot = MemorySnapshot {
            heap_allocated: Self::get_heap_allocated(),
            cache_size,
            reasoning_state_size,
            system_available: Self::get_system_available_memory(),
            timestamp: Instant::now(),
        };

        let mut snapshots = self.snapshots.write().unwrap();
        snapshots.push(snapshot.clone());

        // Keep only the most recent snapshots
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        snapshot
    }

    /// Get heap allocated memory in bytes
    #[cfg(target_os = "linux")]
    fn get_heap_allocated() -> usize {
        use std::fs;
        
        // Read from /proc/self/status
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = value.parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
        0
    }

    #[cfg(target_os = "macos")]
    fn get_heap_allocated() -> usize {
        use std::process::Command;
        
        // Use ps command to get memory usage
        if let Ok(output) = Command::new("ps")
            .args(["-o", "rss=", "-p"])
            .arg(std::process::id().to_string())
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                if let Ok(kb) = output_str.trim().parse::<usize>() {
                    return kb * 1024; // Convert KB to bytes
                }
            }
        }
        0
    }

    #[cfg(target_os = "windows")]
    fn get_heap_allocated() -> usize {
        use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
        use windows::Win32::System::Threading::GetCurrentProcess;
        
        unsafe {
            let process = GetCurrentProcess();
            let mut pmc: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
            
            if GetProcessMemoryInfo(
                process,
                &mut pmc as *mut _ as *mut _,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ).is_ok() {
                return pmc.WorkingSetSize;
            }
        }
        
        0
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn get_heap_allocated() -> usize {
        0
    }

    /// Get system available memory in bytes
    #[cfg(target_os = "linux")]
    fn get_system_available_memory() -> usize {
        use std::fs;
        
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = value.parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
        0
    }

    #[cfg(target_os = "macos")]
    fn get_system_available_memory() -> usize {
        use std::process::Command;
        
        // Use vm_stat to get memory statistics
        if let Ok(output) = Command::new("vm_stat").output() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                let mut free_pages = 0usize;
                let mut inactive_pages = 0usize;
                
                for line in output_str.lines() {
                    if line.contains("Pages free:") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Ok(pages) = value.trim().trim_end_matches('.').parse::<usize>() {
                                free_pages = pages;
                            }
                        }
                    } else if line.contains("Pages inactive:") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Ok(pages) = value.trim().trim_end_matches('.').parse::<usize>() {
                                inactive_pages = pages;
                            }
                        }
                    }
                }
                
                // Page size is typically 4096 bytes on macOS
                return (free_pages + inactive_pages) * 4096;
            }
        }
        0
    }

    #[cfg(target_os = "windows")]
    fn get_system_available_memory() -> usize {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        
        unsafe {
            let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            
            if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
                return mem_status.ullAvailPhys as usize;
            }
        }
        
        0
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn get_system_available_memory() -> usize {
        0
    }

    /// Get all snapshots
    #[must_use]
    pub fn get_snapshots(&self) -> Vec<MemorySnapshot> {
        self.snapshots.read().unwrap().clone()
    }

    /// Get memory usage statistics
    #[must_use]
    pub fn get_stats(&self) -> MemoryStats {
        let snapshots = self.snapshots.read().unwrap();
        
        if snapshots.is_empty() {
            return MemoryStats::default();
        }

        let total_used: Vec<usize> = snapshots.iter().map(|s| s.total_used()).collect();
        let heap_allocated: Vec<usize> = snapshots.iter().map(|s| s.heap_allocated).collect();
        let cache_sizes: Vec<usize> = snapshots.iter().map(|s| s.cache_size).collect();

        MemoryStats {
            current_total_mb: snapshots.last().map(|s| s.total_used_mb()).unwrap_or(0.0),
            peak_total_mb: total_used.iter().max().copied().unwrap_or(0) as f64 / (1024.0 * 1024.0),
            avg_total_mb: (total_used.iter().sum::<usize>() as f64 / total_used.len() as f64) / (1024.0 * 1024.0),
            current_heap_mb: snapshots.last().map(|s| s.heap_allocated as f64 / (1024.0 * 1024.0)).unwrap_or(0.0),
            peak_heap_mb: heap_allocated.iter().max().copied().unwrap_or(0) as f64 / (1024.0 * 1024.0),
            current_cache_mb: snapshots.last().map(|s| s.cache_size as f64 / (1024.0 * 1024.0)).unwrap_or(0.0),
            peak_cache_mb: cache_sizes.iter().max().copied().unwrap_or(0) as f64 / (1024.0 * 1024.0),
            system_available_mb: snapshots.last().map(|s| s.system_available_mb()).unwrap_or(0.0),
        }
    }

    /// Get system available memory in bytes (public helper)
    /// 
    /// Returns the amount of available system memory. Falls back to 1GB if unavailable.
    #[must_use]
    pub fn query_system_available_memory() -> usize {
        Self::get_system_available_memory()
    }

    /// Clear all snapshots
    pub fn clear(&self) {
        self.snapshots.write().unwrap().clear();
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub current_total_mb: f64,
    pub peak_total_mb: f64,
    pub avg_total_mb: f64,
    pub current_heap_mb: f64,
    pub peak_heap_mb: f64,
    pub current_cache_mb: f64,
    pub peak_cache_mb: f64,
    pub system_available_mb: f64,
}

/// Query execution timing information
#[derive(Debug, Clone)]
pub struct QueryTiming {
    /// Total query execution time
    pub total_duration: Duration,
    /// Time spent on atom evaluation
    pub atom_evaluation_duration: Duration,
    /// Time spent on join operations
    pub join_duration: Duration,
    /// Time spent on result materialization
    pub materialization_duration: Duration,
    /// Number of atoms evaluated
    pub atoms_evaluated: usize,
    /// Number of join operations
    pub joins_performed: usize,
    /// Result set size
    pub result_size: usize,
}

impl QueryTiming {
    /// Create a new query timing record
    #[must_use]
    pub fn new(
        total_duration: Duration,
        atom_evaluation_duration: Duration,
        join_duration: Duration,
        materialization_duration: Duration,
        atoms_evaluated: usize,
        joins_performed: usize,
        result_size: usize,
    ) -> Self {
        Self {
            total_duration,
            atom_evaluation_duration,
            join_duration,
            materialization_duration,
            atoms_evaluated,
            joins_performed,
            result_size,
        }
    }

    /// Get average time per atom in microseconds
    #[must_use]
    pub fn avg_time_per_atom_us(&self) -> f64 {
        if self.atoms_evaluated == 0 {
            return 0.0;
        }
        self.atom_evaluation_duration.as_micros() as f64 / self.atoms_evaluated as f64
    }

    /// Get average time per join in microseconds
    #[must_use]
    pub fn avg_time_per_join_us(&self) -> f64 {
        if self.joins_performed == 0 {
            return 0.0;
        }
        self.join_duration.as_micros() as f64 / self.joins_performed as f64
    }
}

/// Query profiler that tracks execution timing
#[derive(Debug, Clone)]
pub struct QueryProfiler {
    timings: Arc<RwLock<Vec<QueryTiming>>>,
    max_timings: usize,
}

impl QueryProfiler {
    /// Create a new query profiler
    #[must_use]
    pub fn new(max_timings: usize) -> Self {
        Self {
            timings: Arc::new(RwLock::new(Vec::new())),
            max_timings,
        }
    }

    /// Record a query timing
    pub fn record(&self, timing: QueryTiming) {
        let mut timings = self.timings.write().unwrap();
        timings.push(timing);

        // Keep only the most recent timings
        if timings.len() > self.max_timings {
            timings.remove(0);
        }
    }

    /// Get all timings
    #[must_use]
    pub fn get_timings(&self) -> Vec<QueryTiming> {
        self.timings.read().unwrap().clone()
    }

    /// Get profiling statistics
    #[must_use]
    pub fn get_stats(&self) -> QueryProfilingStats {
        let timings = self.timings.read().unwrap();
        
        if timings.is_empty() {
            return QueryProfilingStats::default();
        }

        let total_durations: Vec<Duration> = timings.iter().map(|t| t.total_duration).collect();
        let atom_durations: Vec<Duration> = timings.iter().map(|t| t.atom_evaluation_duration).collect();
        let join_durations: Vec<Duration> = timings.iter().map(|t| t.join_duration).collect();

        QueryProfilingStats {
            total_queries: timings.len(),
            avg_total_duration_ms: Self::avg_duration_ms(&total_durations),
            avg_atom_evaluation_ms: Self::avg_duration_ms(&atom_durations),
            avg_join_duration_ms: Self::avg_duration_ms(&join_durations),
            slowest_query_ms: total_durations.iter().max().map(|d| d.as_millis() as f64).unwrap_or(0.0),
            fastest_query_ms: total_durations.iter().min().map(|d| d.as_millis() as f64).unwrap_or(0.0),
            total_atoms_evaluated: timings.iter().map(|t| t.atoms_evaluated).sum(),
            total_joins_performed: timings.iter().map(|t| t.joins_performed).sum(),
        }
    }

    fn avg_duration_ms(durations: &[Duration]) -> f64 {
        if durations.is_empty() {
            return 0.0;
        }
        let total_ms: u128 = durations.iter().map(|d| d.as_millis()).sum();
        total_ms as f64 / durations.len() as f64
    }

    /// Clear all timings
    pub fn clear(&self) {
        self.timings.write().unwrap().clear();
    }
}

impl Default for QueryProfiler {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Query profiling statistics
#[derive(Debug, Clone, Default)]
pub struct QueryProfilingStats {
    pub total_queries: usize,
    pub avg_total_duration_ms: f64,
    pub avg_atom_evaluation_ms: f64,
    pub avg_join_duration_ms: f64,
    pub slowest_query_ms: f64,
    pub fastest_query_ms: f64,
    pub total_atoms_evaluated: usize,
    pub total_joins_performed: usize,
}

/// Performance monitor that coordinates all monitoring components
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    memory_tracker: MemoryTracker,
    query_profiler: QueryProfiler,
    enabled: bool,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            memory_tracker: MemoryTracker::default(),
            query_profiler: QueryProfiler::default(),
            enabled,
        }
    }

    /// Check if monitoring is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable monitoring
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable monitoring
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Take a memory snapshot
    pub fn snapshot_memory(&self, cache_size: usize, reasoning_state_size: usize) -> Option<MemorySnapshot> {
        if !self.enabled {
            return None;
        }
        Some(self.memory_tracker.snapshot(cache_size, reasoning_state_size))
    }

    /// Record query timing
    pub fn record_query_timing(&self, timing: QueryTiming) {
        if !self.enabled {
            return;
        }
        self.query_profiler.record(timing);
    }

    /// Get memory tracker
    #[must_use]
    pub fn memory_tracker(&self) -> &MemoryTracker {
        &self.memory_tracker
    }

    /// Get query profiler
    #[must_use]
    pub fn query_profiler(&self) -> &QueryProfiler {
        &self.query_profiler
    }

    /// Get comprehensive performance report
    #[must_use]
    pub fn get_report(&self) -> PerformanceReport {
        PerformanceReport {
            memory_stats: self.memory_tracker.get_stats(),
            query_stats: self.query_profiler.get_stats(),
        }
    }

    /// Clear all monitoring data
    pub fn clear(&self) {
        self.memory_tracker.clear();
        self.query_profiler.clear();
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Comprehensive performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub memory_stats: MemoryStats,
    pub query_stats: QueryProfilingStats,
}

impl PerformanceReport {
    /// Format as human-readable string
    #[must_use]
    pub fn format(&self) -> String {
        format!(
            "Performance Report:\n\
             Memory:\n\
             - Current Total: {:.2} MB (Peak: {:.2} MB, Avg: {:.2} MB)\n\
             - Heap: {:.2} MB (Peak: {:.2} MB)\n\
             - Cache: {:.2} MB (Peak: {:.2} MB)\n\
             - System Available: {:.2} MB\n\
             Query Profiling:\n\
             - Total Queries: {}\n\
             - Avg Total Duration: {:.2} ms\n\
             - Avg Atom Evaluation: {:.2} ms\n\
             - Avg Join Duration: {:.2} ms\n\
             - Slowest Query: {:.2} ms\n\
             - Fastest Query: {:.2} ms\n\
             - Total Atoms Evaluated: {}\n\
             - Total Joins Performed: {}",
            self.memory_stats.current_total_mb,
            self.memory_stats.peak_total_mb,
            self.memory_stats.avg_total_mb,
            self.memory_stats.current_heap_mb,
            self.memory_stats.peak_heap_mb,
            self.memory_stats.current_cache_mb,
            self.memory_stats.peak_cache_mb,
            self.memory_stats.system_available_mb,
            self.query_stats.total_queries,
            self.query_stats.avg_total_duration_ms,
            self.query_stats.avg_atom_evaluation_ms,
            self.query_stats.avg_join_duration_ms,
            self.query_stats.slowest_query_ms,
            self.query_stats.fastest_query_ms,
            self.query_stats.total_atoms_evaluated,
            self.query_stats.total_joins_performed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_snapshot() {
        let snapshot = MemorySnapshot {
            heap_allocated: 1024 * 1024 * 10, // 10 MB
            cache_size: 1024 * 1024 * 2,      // 2 MB
            reasoning_state_size: 1024 * 1024 * 3, // 3 MB
            system_available: 1024 * 1024 * 1024,  // 1 GB
            timestamp: Instant::now(),
        };

        assert_eq!(snapshot.total_used(), 1024 * 1024 * 15); // 15 MB
        assert!((snapshot.total_used_mb() - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new(10);
        
        // Take some snapshots
        tracker.snapshot(1024 * 1024, 512 * 1024);
        tracker.snapshot(2048 * 1024, 1024 * 1024);
        tracker.snapshot(3072 * 1024, 1536 * 1024);

        let snapshots = tracker.get_snapshots();
        assert_eq!(snapshots.len(), 3);

        let stats = tracker.get_stats();
        assert!(stats.peak_total_mb > 0.0);
    }

    #[test]
    fn test_query_timing() {
        let timing = QueryTiming::new(
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_millis(30),
            Duration::from_millis(20),
            10,
            5,
            100,
        );

        assert_eq!(timing.atoms_evaluated, 10);
        assert_eq!(timing.joins_performed, 5);
        assert!(timing.avg_time_per_atom_us() > 0.0);
        assert!(timing.avg_time_per_join_us() > 0.0);
    }

    #[test]
    fn test_query_profiler() {
        let profiler = QueryProfiler::new(10);
        
        // Record some timings
        profiler.record(QueryTiming::new(
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_millis(30),
            Duration::from_millis(20),
            10,
            5,
            100,
        ));
        
        profiler.record(QueryTiming::new(
            Duration::from_millis(200),
            Duration::from_millis(100),
            Duration::from_millis(60),
            Duration::from_millis(40),
            20,
            10,
            200,
        ));

        let stats = profiler.get_stats();
        assert_eq!(stats.total_queries, 2);
        assert!(stats.avg_total_duration_ms > 0.0);
        assert!(stats.slowest_query_ms >= stats.fastest_query_ms);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new(true);
        assert!(monitor.is_enabled());

        // Take memory snapshot
        let snapshot = monitor.snapshot_memory(1024 * 1024, 512 * 1024);
        assert!(snapshot.is_some());

        // Record query timing
        monitor.record_query_timing(QueryTiming::new(
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_millis(30),
            Duration::from_millis(20),
            10,
            5,
            100,
        ));

        // Get report
        let report = monitor.get_report();
        assert_eq!(report.query_stats.total_queries, 1);

        // Test disable
        monitor.disable();
        assert!(!monitor.is_enabled());
        let snapshot = monitor.snapshot_memory(1024 * 1024, 512 * 1024);
        assert!(snapshot.is_none());
    }

    #[test]
    fn test_performance_report_format() {
        let report = PerformanceReport {
            memory_stats: MemoryStats {
                current_total_mb: 15.0,
                peak_total_mb: 20.0,
                avg_total_mb: 17.5,
                current_heap_mb: 10.0,
                peak_heap_mb: 12.0,
                current_cache_mb: 5.0,
                peak_cache_mb: 8.0,
                system_available_mb: 1024.0,
            },
            query_stats: QueryProfilingStats {
                total_queries: 100,
                avg_total_duration_ms: 50.0,
                avg_atom_evaluation_ms: 25.0,
                avg_join_duration_ms: 15.0,
                slowest_query_ms: 200.0,
                fastest_query_ms: 10.0,
                total_atoms_evaluated: 1000,
                total_joins_performed: 500,
            },
        };

        let formatted = report.format();
        assert!(formatted.contains("Performance Report"));
        assert!(formatted.contains("15.00 MB"));
        assert!(formatted.contains("100"));
    }
}
