//! Integration tests for performance monitoring

use oxidowl::performance::{MemoryTracker, PerformanceMonitor, QueryProfiler, QueryTiming};
use std::time::Duration;

#[test]
fn test_memory_tracking_integration() {
    let tracker = MemoryTracker::new(100);
    
    // Simulate cache and reasoning state growth
    for i in 1..=10 {
        let cache_size = 1024 * 1024 * i; // Growing cache
        let reasoning_state = 512 * 1024 * i; // Growing reasoning state
        tracker.snapshot(cache_size, reasoning_state);
    }
    
    let stats = tracker.get_stats().expect("Failed to get memory stats");
    
    // Verify statistics
    assert!(stats.peak_total_mb > 0.0);
    assert!(stats.avg_total_mb > 0.0);
    assert!(stats.current_total_mb > 0.0);
    
    // Peak should be >= current (last snapshot)
    assert!(stats.peak_total_mb >= stats.current_total_mb);
    
    println!("Memory tracking stats:");
    println!("  Current: {:.2} MB", stats.current_total_mb);
    println!("  Peak: {:.2} MB", stats.peak_total_mb);
    println!("  Average: {:.2} MB", stats.avg_total_mb);
}

#[test]
fn test_query_profiling_integration() {
    let profiler = QueryProfiler::new(100);
    
    // Simulate various query executions
    for i in 1..=20 {
        let timing = QueryTiming::new(
            Duration::from_millis(50 * i),
            Duration::from_millis(20 * i),
            Duration::from_millis(15 * i),
            Duration::from_millis(15 * i),
            10 * i as usize,
            5 * i as usize,
            100 * i as usize,
        );
        profiler.record(timing);
    }
    
    let stats = profiler.get_stats().expect("Failed to get query stats");
    
    // Verify statistics
    assert_eq!(stats.total_queries, 20);
    assert!(stats.avg_total_duration_ms > 0.0);
    assert!(stats.avg_atom_evaluation_ms > 0.0);
    assert!(stats.avg_join_duration_ms > 0.0);
    assert!(stats.slowest_query_ms >= stats.fastest_query_ms);
    assert_eq!(stats.total_atoms_evaluated, (1..=20).map(|i| 10 * i).sum::<usize>());
    assert_eq!(stats.total_joins_performed, (1..=20).map(|i| 5 * i).sum::<usize>());
    
    println!("Query profiling stats:");
    println!("  Total queries: {}", stats.total_queries);
    println!("  Avg duration: {:.2} ms", stats.avg_total_duration_ms);
    println!("  Slowest: {:.2} ms", stats.slowest_query_ms);
    println!("  Fastest: {:.2} ms", stats.fastest_query_ms);
}

#[test]
fn test_performance_monitor_integration() {
    let mut monitor = PerformanceMonitor::new(true);
    
    // Simulate query execution with memory snapshots
    for i in 1..=5 {
        monitor.snapshot_memory(1024 * 1024 * i, 512 * 1024 * i);
        
        let timing = QueryTiming::new(
            Duration::from_millis((100 * i) as u64),
            Duration::from_millis((40 * i) as u64),
            Duration::from_millis((30 * i) as u64),
            Duration::from_millis((30 * i) as u64),
            10 * i as usize,
            5 * i as usize,
            100 * i as usize,
        );
        monitor.record_query_timing(timing);
    }
    
    // Get comprehensive report
    let report = monitor.get_report().expect("Failed to get performance report");
    
    // Verify memory stats
    assert!(report.memory_stats.current_total_mb > 0.0);
    assert!(report.memory_stats.peak_total_mb > 0.0);
    
    // Verify query stats
    assert_eq!(report.query_stats.total_queries, 5);
    assert!(report.query_stats.avg_total_duration_ms > 0.0);
    
    // Test report formatting
    let formatted = report.format();
    assert!(formatted.contains("Performance Report"));
    assert!(formatted.contains("Memory:"));
    assert!(formatted.contains("Query Profiling:"));
    
    println!("\n{}", formatted);
}

#[test]
fn test_performance_monitor_disabled() {
    let mut monitor = PerformanceMonitor::new(false);
    
    // When disabled, snapshots should return Ok(None)
    let snapshot = monitor.snapshot_memory(1024 * 1024, 512 * 1024).expect("Should return Ok");
    assert!(snapshot.is_none());
    
    // Recording should not fail but won't record anything
    let timing = QueryTiming::new(
        Duration::from_millis(100),
        Duration::from_millis(40),
        Duration::from_millis(30),
        Duration::from_millis(30),
        10,
        5,
        100,
    );
    monitor.record_query_timing(timing);
    
    // Stats should show zero queries
    let report = monitor.get_report().expect("Failed to get report");
    assert_eq!(report.query_stats.total_queries, 0);
}

#[test]
fn test_memory_snapshot_calculations() {
    let tracker = MemoryTracker::new(10);
    
    // 10 MB heap + 2 MB cache + 3 MB reasoning state = 15 MB total
    tracker.snapshot(2 * 1024 * 1024, 3 * 1024 * 1024);
    
    let snapshots = tracker.get_snapshots().expect("Failed to get snapshots");
    assert_eq!(snapshots.len(), 1);
    
    let snapshot = &snapshots[0];
    assert_eq!(snapshot.cache_size, 2 * 1024 * 1024);
    assert_eq!(snapshot.reasoning_state_size, 3 * 1024 * 1024);
    
    // Total should be sum of all components
    let expected_total = snapshot.heap_allocated + snapshot.cache_size + snapshot.reasoning_state_size;
    assert_eq!(snapshot.total_used(), expected_total);
}

#[test]
fn test_query_timing_calculations() {
    let timing = QueryTiming::new(
        Duration::from_millis(100),
        Duration::from_millis(50),
        Duration::from_millis(30),
        Duration::from_millis(20),
        10,
        5,
        100,
    );
    
    // Test average time per atom
    let avg_atom_time = timing.avg_time_per_atom_us();
    assert!(avg_atom_time > 0.0);
    assert_eq!(avg_atom_time, 50_000.0 / 10.0); // 50ms / 10 atoms = 5000 microseconds per atom
    
    // Test average time per join
    let avg_join_time = timing.avg_time_per_join_us();
    assert!(avg_join_time > 0.0);
    assert_eq!(avg_join_time, 30_000.0 / 5.0); // 30ms / 5 joins = 6000 microseconds per join
}

#[test]
fn test_profiler_max_timings_limit() {
    let profiler = QueryProfiler::new(5); // Only keep 5 timings
    
    // Add 10 timings
    for i in 1..=10 {
        let timing = QueryTiming::new(
            Duration::from_millis(i),
            Duration::from_millis(i),
            Duration::from_millis(i),
            Duration::from_millis(i),
            i as usize,
            i as usize,
            i as usize,
        );
        profiler.record(timing);
    }
    
    // Should only have the last 5
    let timings = profiler.get_timings().expect("Failed to get timings");
    assert_eq!(timings.len(), 5);
    
    // The oldest should be timing #6 (timings 1-5 should be evicted)
    assert_eq!(timings[0].atoms_evaluated, 6);
    assert_eq!(timings[4].atoms_evaluated, 10);
}

#[test]
fn test_tracker_max_snapshots_limit() {
    let tracker = MemoryTracker::new(3); // Only keep 3 snapshots
    
    // Take 5 snapshots
    for i in 1..=5 {
        tracker.snapshot(1024 * i, 512 * i);
    }
    
    // Should only have the last 3
    let snapshots = tracker.get_snapshots().expect("Failed to get snapshots");
    assert_eq!(snapshots.len(), 3);
}
