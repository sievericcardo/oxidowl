//! Monitoring and Statistics for HyperTableau
//!
//! This module provides comprehensive monitoring, statistics collection,
//! and debugging support for the hypertableau algorithm implementation.

use crate::{
    ontology::Individual,
    Error,
};

use super::{
    dependency_tracking::{DependencyStats, BranchingPointId},
    branching::BranchingStats,
    hyperresolution::HyperresolutionStatistics,
    clause_evaluator::EvaluationStatistics,
    extension_tables::ExtensionStatistics,
};

use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use serde::{Serialize, Deserialize};

/// Different levels of monitoring detail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringLevel {
    /// No monitoring (best performance)
    None,
    /// Basic statistics only
    Basic,
    /// Detailed statistics and timing
    Detailed,
    /// Full debugging information
    Debug,
}

/// Types of events that can be monitored
#[derive(Debug, Clone, PartialEq)]
pub enum MonitoredEvent {
    /// Clause application
    ClauseApplication {
        clause_id: usize,
        premises: Vec<String>,
        conclusion: String,
        duration: Duration,
    },
    /// Ground disjunction processing
    GroundDisjunctionProcessing {
        disjunction: String,
        individual: Individual,
        choice_count: usize,
        duration: Duration,
    },
    /// Branching point creation
    BranchingPointCreated {
        branch_id: BranchingPointId,
        branching_type: String,
        choice_count: usize,
    },
    /// Branching choice made
    BranchingChoiceMade {
        branch_id: BranchingPointId,
        choice_index: usize,
        choice_description: String,
    },
    /// Backtracking operation
    Backtracking {
        branch_id: BranchingPointId,
        retracted_facts: usize,
        duration: Duration,
    },
    /// Clash detection
    ClashDetected {
        individual: Individual,
        conflicting_concepts: Vec<String>,
        duration: Duration,
    },
    /// Fact derivation
    FactDerived {
        fact_description: String,
        individual: Individual,
        dependency_level: usize,
    },
    /// Extension table operation
    ExtensionOperation {
        operation_type: String,
        concept: String,
        individual: Individual,
        duration: Duration,
    },
    /// Blocking operation
    BlockingOperation {
        blocker: Individual,
        blocked: Individual,
        blocking_type: String,
        duration: Duration,
    },
}

/// Comprehensive statistics for the entire reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStats {
    /// Overall timing information
    pub total_duration: Duration,
    pub startup_duration: Duration,
    pub reasoning_duration: Duration,
    pub cleanup_duration: Duration,
    
    /// Dependency tracking statistics
    pub dependency_stats: DependencyStats,
    
    /// Branching statistics
    pub branching_stats: BranchingStats,
    
    /// Hyperresolution statistics
    pub hyperresolution_stats: HyperresolutionStatistics,
    
    /// Clause evaluation statistics
    pub clause_evaluation_stats: EvaluationStatistics,
    
    /// Extension table statistics
    pub extension_stats: ExtensionStatistics,
    
    /// Memory usage information
    pub memory_usage: MemoryUsage,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Error and warning counts
    pub error_counts: ErrorCounts,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub peak_memory_bytes: usize,
    pub current_memory_bytes: usize,
    pub dependency_sets_memory: usize,
    pub extension_tables_memory: usize,
    pub branching_points_memory: usize,
    pub clause_index_memory: usize,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub facts_per_second: f64,
    pub clauses_per_second: f64,
    pub branching_points_per_second: f64,
    pub cache_hit_ratio: f64,
    pub average_reasoning_depth: f64,
    pub parallelization_efficiency: f64,
}

/// Error and warning counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCounts {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub clash_count: usize,
    pub timeout_count: usize,
    pub memory_limit_exceeded: usize,
    pub infinite_loop_detected: usize,
}

impl Default for MemoryUsage {
    fn default() -> Self {
        Self {
            peak_memory_bytes: 0,
            current_memory_bytes: 0,
            dependency_sets_memory: 0,
            extension_tables_memory: 0,
            branching_points_memory: 0,
            clause_index_memory: 0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            facts_per_second: 0.0,
            clauses_per_second: 0.0,
            branching_points_per_second: 0.0,
            cache_hit_ratio: 0.0,
            average_reasoning_depth: 0.0,
            parallelization_efficiency: 0.0,
        }
    }
}

impl Default for ErrorCounts {
    fn default() -> Self {
        Self {
            total_errors: 0,
            total_warnings: 0,
            clash_count: 0,
            timeout_count: 0,
            memory_limit_exceeded: 0,
            infinite_loop_detected: 0,
        }
    }
}

/// Event listener for monitoring
pub trait EventListener: Send + Sync {
    /// Called when an event occurs
    fn on_event(&self, event: &MonitoredEvent);
    
    /// Called when reasoning starts
    fn on_reasoning_start(&self) {}
    
    /// Called when reasoning completes
    fn on_reasoning_complete(&self, stats: &ReasoningStats) {}
    
    /// Called when an error occurs
    fn on_error(&self, error: &Error) {}
}

/// Simple console event listener for debugging
#[derive(Debug)]
pub struct ConsoleEventListener {
    pub level: MonitoringLevel,
}

impl ConsoleEventListener {
    pub fn new(level: MonitoringLevel) -> Self {
        Self { level }
    }
}

impl EventListener for ConsoleEventListener {
    fn on_event(&self, event: &MonitoredEvent) {
        if self.level == MonitoringLevel::None {
            return;
        }
        
        match self.level {
            MonitoringLevel::Basic => {
                match event {
                    MonitoredEvent::ClashDetected { .. } => {
                        println!("CLASH DETECTED");
                    }
                    MonitoredEvent::BranchingPointCreated { choice_count, .. } => {
                        println!("BRANCHING: {} choices", choice_count);
                    }
                    _ => {}
                }
            }
            MonitoringLevel::Detailed | MonitoringLevel::Debug => {
                println!("EVENT: {:?}", event);
            }
            _ => {}
        }
    }
    
    fn on_reasoning_start(&self) {
        if self.level != MonitoringLevel::None {
            println!("REASONING STARTED");
        }
    }
    
    fn on_reasoning_complete(&self, stats: &ReasoningStats) {
        if self.level != MonitoringLevel::None {
            println!("REASONING COMPLETED: {:?}", stats.total_duration);
            if self.level == MonitoringLevel::Debug {
                println!("FULL STATS: {:#?}", stats);
            }
        }
    }
    
    fn on_error(&self, error: &Error) {
        if self.level != MonitoringLevel::None {
            println!("ERROR: {:?}", error);
        }
    }
}

/// Main monitoring and statistics collection system
pub struct TableauMonitor {
    /// Current monitoring level
    monitoring_level: MonitoringLevel,
    
    /// Event listeners
    listeners: Vec<Box<dyn EventListener>>,
    
    /// Event history (for debugging)
    event_history: VecDeque<MonitoredEvent>,
    max_history_size: usize,
    
    /// Timing information
    start_time: Option<Instant>,
    reasoning_start_time: Option<Instant>,
    
    /// Statistics accumulation
    stats: ReasoningStats,
    
    /// Event counters
    event_counters: HashMap<String, usize>,
    
    /// Performance tracking
    facts_derived: usize,
    clauses_applied: usize,
    branching_points_created: usize,
    
    /// Memory tracking
    memory_samples: VecDeque<(Instant, usize)>,
    memory_sample_interval: Duration,
    last_memory_sample: Instant,
    
    /// Cache for performance calculations
    performance_cache: PerformanceCache,
}

#[derive(Debug)]
struct PerformanceCache {
    last_calculation: Instant,
    cached_metrics: PerformanceMetrics,
    calculation_interval: Duration,
}

impl std::fmt::Debug for TableauMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableauMonitor")
            .field("monitoring_level", &self.monitoring_level)
            .field("listeners_count", &self.listeners.len())
            .field("event_history_size", &self.event_history.len())
            .field("max_history_size", &self.max_history_size)
            .field("start_time", &self.start_time)
            .field("reasoning_start_time", &self.reasoning_start_time)
            .field("stats", &self.stats)
            .field("event_counters", &self.event_counters)
            .field("facts_derived", &self.facts_derived)
            .field("clauses_applied", &self.clauses_applied)
            .field("branching_points_created", &self.branching_points_created)
            .field("memory_samples_count", &self.memory_samples.len())
            .field("memory_sample_interval", &self.memory_sample_interval)
            .field("last_memory_sample", &self.last_memory_sample)
            .field("performance_cache", &self.performance_cache)
            .finish()
    }
}

impl TableauMonitor {
    /// Create a new tableau monitor
    pub fn new(monitoring_level: MonitoringLevel) -> Self {
        let mut monitor = Self {
            monitoring_level,
            listeners: Vec::new(),
            event_history: VecDeque::new(),
            max_history_size: 1000,
            start_time: None,
            reasoning_start_time: None,
            stats: ReasoningStats {
                total_duration: Duration::default(),
                startup_duration: Duration::default(),
                reasoning_duration: Duration::default(),
                cleanup_duration: Duration::default(),
                dependency_stats: DependencyStats::default(),
                branching_stats: BranchingStats::default(),
                hyperresolution_stats: HyperresolutionStatistics::default(),
                clause_evaluation_stats: EvaluationStatistics::default(),
                extension_stats: ExtensionStatistics::default(),
                memory_usage: MemoryUsage::default(),
                performance_metrics: PerformanceMetrics::default(),
                error_counts: ErrorCounts::default(),
            },
            event_counters: HashMap::new(),
            facts_derived: 0,
            clauses_applied: 0,
            branching_points_created: 0,
            memory_samples: VecDeque::new(),
            memory_sample_interval: Duration::from_secs(1),
            last_memory_sample: Instant::now(),
            performance_cache: PerformanceCache {
                last_calculation: Instant::now(),
                cached_metrics: PerformanceMetrics::default(),
                calculation_interval: Duration::from_secs(5),
            },
        };
        
        // Add default console listener if monitoring is enabled
        if monitoring_level != MonitoringLevel::None {
            monitor.add_listener(Box::new(ConsoleEventListener::new(monitoring_level)));
        }
        
        monitor
    }
    
    /// Add an event listener
    pub fn add_listener(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }
    
    /// Start monitoring (call at beginning of reasoning)
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        
        for listener in &self.listeners {
            listener.on_reasoning_start();
        }
    }
    
    /// Start reasoning phase
    pub fn start_reasoning(&mut self) {
        self.reasoning_start_time = Some(Instant::now());
        
        if let Some(start_time) = self.start_time {
            self.stats.startup_duration = self.reasoning_start_time.unwrap() - start_time;
        }
    }
    
    /// Log an event
    pub fn log_event(&mut self, event: MonitoredEvent) {
        if self.monitoring_level == MonitoringLevel::None {
            return;
        }
        
        // Update counters based on event type
        match &event {
            MonitoredEvent::ClauseApplication { .. } => {
                self.clauses_applied += 1;
            }
            MonitoredEvent::FactDerived { .. } => {
                self.facts_derived += 1;
            }
            MonitoredEvent::BranchingPointCreated { .. } => {
                self.branching_points_created += 1;
            }
            MonitoredEvent::ClashDetected { .. } => {
                self.stats.error_counts.clash_count += 1;
            }
            _ => {}
        }
        
        // Update event counter
        let event_type = self.get_event_type_name(&event);
        *self.event_counters.entry(event_type).or_insert(0) += 1;
        
        // Store in history if detailed monitoring
        if self.monitoring_level == MonitoringLevel::Debug {
            self.event_history.push_back(event.clone());
            if self.event_history.len() > self.max_history_size {
                self.event_history.pop_front();
            }
        }
        
        // Notify listeners
        for listener in &self.listeners {
            listener.on_event(&event);
        }
        
        // Sample memory periodically
        self.maybe_sample_memory();
    }
    
    /// Log an error
    pub fn log_error(&mut self, error: &Error) {
        self.stats.error_counts.total_errors += 1;
        
        for listener in &self.listeners {
            listener.on_error(error);
        }
    }
    
    /// Log a warning
    pub fn log_warning(&mut self, _warning: &str) {
        self.stats.error_counts.total_warnings += 1;
    }
    
    /// Update statistics from external components
    pub fn update_dependency_stats(&mut self, stats: &DependencyStats) {
        self.stats.dependency_stats = stats.clone();
    }
    
    pub fn update_branching_stats(&mut self, stats: &BranchingStats) {
        self.stats.branching_stats = stats.clone();
    }
    
    pub fn update_hyperresolution_stats(&mut self, stats: &HyperresolutionStatistics) {
        self.stats.hyperresolution_stats = stats.clone();
    }
    
    pub fn update_clause_evaluation_stats(&mut self, stats: &EvaluationStatistics) {
        self.stats.clause_evaluation_stats = stats.clone();
    }
    
    pub fn update_extension_stats(&mut self, stats: &ExtensionStatistics) {
        self.stats.extension_stats = stats.clone();
    }
    
    /// Finish monitoring and get final statistics
    pub fn finish(&mut self) -> ReasoningStats {
        let now = Instant::now();
        
        if let Some(start_time) = self.start_time {
            self.stats.total_duration = now - start_time;
        }
        
        if let Some(reasoning_start) = self.reasoning_start_time {
            self.stats.reasoning_duration = now - reasoning_start;
            self.stats.cleanup_duration = 
                self.stats.total_duration - self.stats.startup_duration - self.stats.reasoning_duration;
        }
        
        // Calculate final performance metrics
        self.calculate_performance_metrics();
        
        // Update memory usage
        self.update_memory_usage();
        
        // Notify listeners
        for listener in &self.listeners {
            listener.on_reasoning_complete(&self.stats);
        }
        
        self.stats.clone()
    }
    
    /// Get current statistics
    pub fn get_stats(&mut self) -> &ReasoningStats {
        // Update performance metrics if enough time has passed
        let now = Instant::now();
        if now - self.performance_cache.last_calculation > self.performance_cache.calculation_interval {
            self.calculate_performance_metrics();
            self.performance_cache.last_calculation = now;
        }
        
        &self.stats
    }
    
    /// Get event history (for debugging)
    pub fn get_event_history(&self) -> &VecDeque<MonitoredEvent> {
        &self.event_history
    }
    
    /// Get event counters
    pub fn get_event_counters(&self) -> &HashMap<String, usize> {
        &self.event_counters
    }
    
    /// Set monitoring level
    pub fn set_monitoring_level(&mut self, level: MonitoringLevel) {
        self.monitoring_level = level;
    }
    
    /// Get monitoring level
    pub fn get_monitoring_level(&self) -> MonitoringLevel {
        self.monitoring_level
    }
    
    /// Helper function to get event type name
    fn get_event_type_name(&self, event: &MonitoredEvent) -> String {
        match event {
            MonitoredEvent::ClauseApplication { .. } => "ClauseApplication".to_string(),
            MonitoredEvent::GroundDisjunctionProcessing { .. } => "GroundDisjunctionProcessing".to_string(),
            MonitoredEvent::BranchingPointCreated { .. } => "BranchingPointCreated".to_string(),
            MonitoredEvent::BranchingChoiceMade { .. } => "BranchingChoiceMade".to_string(),
            MonitoredEvent::Backtracking { .. } => "Backtracking".to_string(),
            MonitoredEvent::ClashDetected { .. } => "ClashDetected".to_string(),
            MonitoredEvent::FactDerived { .. } => "FactDerived".to_string(),
            MonitoredEvent::ExtensionOperation { .. } => "ExtensionOperation".to_string(),
            MonitoredEvent::BlockingOperation { .. } => "BlockingOperation".to_string(),
        }
    }
    
    /// Sample memory usage if interval has passed
    fn maybe_sample_memory(&mut self) {
        let now = Instant::now();
        if now - self.last_memory_sample > self.memory_sample_interval {
            let memory = self.get_current_memory_usage();
            self.memory_samples.push_back((now, memory));
            
            // Keep only recent samples
            let cutoff = now - Duration::from_secs(60);
            while let Some(&(sample_time, _)) = self.memory_samples.front() {
                if sample_time < cutoff {
                    self.memory_samples.pop_front();
                } else {
                    break;
                }
            }
            
            self.last_memory_sample = now;
        }
    }
    
    /// Get current memory usage (simplified)
    fn get_current_memory_usage(&self) -> usize {
        // In a real implementation, this would query the system for memory usage
        // For now, we'll estimate based on data structure sizes
        std::mem::size_of_val(&self.event_history) +
        std::mem::size_of_val(&self.event_counters) +
        std::mem::size_of_val(&self.memory_samples)
    }
    
    /// Calculate performance metrics
    fn calculate_performance_metrics(&mut self) {
        let duration = self.reasoning_start_time
            .map(|start| Instant::now() - start)
            .unwrap_or_default();
        
        let duration_secs = duration.as_secs_f64();
        
        if duration_secs > 0.0 {
            self.stats.performance_metrics.facts_per_second = 
                self.facts_derived as f64 / duration_secs;
            self.stats.performance_metrics.clauses_per_second = 
                self.clauses_applied as f64 / duration_secs;
            self.stats.performance_metrics.branching_points_per_second = 
                self.branching_points_created as f64 / duration_secs;
        }
        
        // Update cached metrics
        self.performance_cache.cached_metrics = self.stats.performance_metrics.clone();
    }
    
    /// Update memory usage statistics
    fn update_memory_usage(&mut self) {
        let current_memory = self.get_current_memory_usage();
        self.stats.memory_usage.current_memory_bytes = current_memory;
        
        // Update peak memory
        if current_memory > self.stats.memory_usage.peak_memory_bytes {
            self.stats.memory_usage.peak_memory_bytes = current_memory;
        }
    }
}

impl Default for TableauMonitor {
    fn default() -> Self {
        Self::new(MonitoringLevel::Basic)
    }
}

/// Helper functions for creating monitoring events
pub mod events {
    use super::*;
    
    pub fn clause_application(
        clause_id: usize,
        premises: Vec<String>,
        conclusion: String,
        duration: Duration,
    ) -> MonitoredEvent {
        MonitoredEvent::ClauseApplication {
            clause_id,
            premises,
            conclusion,
            duration,
        }
    }
    
    pub fn ground_disjunction_processing(
        disjunction: String,
        individual: Individual,
        choice_count: usize,
        duration: Duration,
    ) -> MonitoredEvent {
        MonitoredEvent::GroundDisjunctionProcessing {
            disjunction,
            individual,
            choice_count,
            duration,
        }
    }
    
    pub fn clash_detected(
        individual: Individual,
        conflicting_concepts: Vec<String>,
        duration: Duration,
    ) -> MonitoredEvent {
        MonitoredEvent::ClashDetected {
            individual,
            conflicting_concepts,
            duration,
        }
    }
    
    pub fn fact_derived(
        fact_description: String,
        individual: Individual,
        dependency_level: usize,
    ) -> MonitoredEvent {
        MonitoredEvent::FactDerived {
            fact_description,
            individual,
            dependency_level,
        }
    }
}