//! Extension Tables Module
//!
//! This module implements efficient fact storage and retrieval for the hypertableau
//! algorithm. It provides delta management, incremental reasoning support, and 
//! integration with blocking and caching optimizations.

use crate::{
    core::{
        tableau::{TableauNode, TableauEdge},
        dependency::DependencySet,
        completion::CompletionRule,
    },
    ontology::{Ontology, ClassExpression, Individual, Axiom},
    Error, Result,
};

use super::{
    ground_disjunction::{GroundDisjunction, GroundDisjunctionHeader},
    hyperresolution::{DLClause, Atom},
    dependency_tracking::DependencyTracker,
};

use std::{
    collections::{HashMap, HashSet, VecDeque, BTreeMap},
    sync::{Arc, Mutex, RwLock},
    fmt,
    hash::{Hash, Hasher},
};

/// Extension manager for fact storage and retrieval
#[derive(Debug)]
pub struct ExtensionManager {
    /// Extension tables by arity
    extension_tables: HashMap<usize, ExtensionTable>,
    
    /// Binary extension table (most common)
    binary_extension_table: ExtensionTable,
    
    /// Ternary extension table
    ternary_extension_table: ExtensionTable,
    
    /// Clash detection and management
    clash_manager: ClashManager,
    
    /// Dependency set factory
    dependency_factory: DependencySetFactory,
    
    /// Auxiliary tuple buffers for efficiency
    binary_tuple_buffer: Vec<String>,
    ternary_tuple_buffer: Vec<String>,
    
    /// Active flag for add operations
    add_active: bool,
    
    /// Statistics
    statistics: ExtensionStatistics,
}

/// Extension table for storing facts of specific arity
#[derive(Debug)]
pub struct ExtensionTable {
    /// Arity of tuples in this table
    arity: usize,
    
    /// Main storage for facts
    tuples: Vec<TupleEntry>,
    
    /// Index by predicate for fast lookup
    predicate_index: HashMap<String, Vec<usize>>,
    
    /// Delta management for incremental reasoning
    delta_new: HashSet<usize>,
    delta_old: HashSet<usize>,
    
    /// Retrieval operations
    active_retrievals: Vec<Retrieval>,
    
    /// Tuple cache for performance
    tuple_cache: LRUCache<TupleKey, usize>,
    
    /// Blocking and optimization data
    blocking_data: BlockingData,
    
    /// Size tracking
    current_size: usize,
    max_size: usize,
}

/// Entry in an extension table
#[derive(Debug, Clone)]
pub struct TupleEntry {
    /// The actual tuple data
    tuple: Vec<String>,
    
    /// Predicate this tuple belongs to
    predicate: String,
    
    /// Dependency set for this tuple
    dependency_set: DependencySet,
    
    /// When this tuple was added (for delta management)
    added_at: u64,
    
    /// Core flag for core blocking
    is_core: bool,
    
    /// Active flag
    is_active: bool,
    
    /// Hash for fast comparison
    tuple_hash: u64,
}

/// Retrieval operation for querying facts
#[derive(Debug)]
pub struct Retrieval {
    /// ID for this retrieval
    id: usize,
    
    /// Arity of tuples being retrieved
    arity: usize,
    
    /// Binding pattern (which positions are bound)
    binding_pattern: Vec<bool>,
    
    /// Bound values
    bindings: Vec<Option<String>>,
    
    /// View type for retrieval
    view: RetrievalView,
    
    /// Current position in results
    position: usize,
    
    /// Cached results
    results: Vec<usize>,
    
    /// Is retrieval open and active
    is_open: bool,
    
    /// Current tuple buffer
    tuple_buffer: Vec<String>,
}

/// View types for fact retrieval
#[derive(Debug, Clone)]
pub enum RetrievalView {
    /// All facts in the extension
    Extension,
    
    /// Only new facts (current delta)
    DeltaNew,
    
    /// Old facts (previous deltas)
    DeltaOld,
    
    /// Extension facts (non-delta)
    ExtensionThis,
    
    /// Complete view (extension + delta)
    Complete,
}

/// Key for tuple identification and caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleKey {
    predicate: String,
    args: Vec<String>,
}

/// LRU Cache for tuple lookup optimization
#[derive(Debug)]
pub struct LRUCache<K, V> {
    capacity: usize,
    map: HashMap<K, (V, usize)>,
    access_order: BTreeMap<usize, K>,
    next_access: usize,
}

/// Blocking data for optimization
#[derive(Debug)]
pub struct BlockingData {
    /// Blocked tuple indices
    blocked_tuples: HashSet<usize>,
    
    /// Blocking relationships
    blocking_relationships: HashMap<usize, HashSet<usize>>,
    
    /// Signature cache for blocking
    signature_cache: HashMap<String, Vec<String>>,
}

/// Clash detection and management
#[derive(Debug)]
pub struct ClashManager {
    /// Current clash state
    has_clash: bool,
    
    /// Clash dependency set
    clash_dependencies: Option<DependencySet>,
    
    /// Clash detection rules
    clash_rules: Vec<ClashRule>,
    
    /// Clash history for learning
    clash_history: Vec<ClashInfo>,
}

/// Clash detection rule
#[derive(Debug, Clone)]
pub struct ClashRule {
    /// Positive predicates that cause clash
    positive_predicates: HashSet<String>,
    
    /// Negative predicates that cause clash
    negative_predicates: HashSet<String>,
    
    /// Rule priority
    priority: i32,
}

/// Information about a detected clash
#[derive(Debug, Clone)]
pub struct ClashInfo {
    /// Tuple indices involved in clash
    tuple_indices: Vec<usize>,
    
    /// Dependency set for the clash
    dependencies: DependencySet,
    
    /// Clash type
    clash_type: ClashType,
    
    /// When clash was detected
    detected_at: u64,
}

/// Types of clashes
#[derive(Debug, Clone)]
pub enum ClashType {
    /// Complementary concepts
    ComplementaryConcepts,
    
    /// Inequality clash
    Inequality,
    
    /// Datatype clash
    Datatype,
    
    /// Cardinality clash
    Cardinality,
    
    /// Custom clash
    Custom(String),
}

/// Dependency set factory
#[derive(Debug)]
pub struct DependencySetFactory {
    /// Empty dependency set
    empty_set: DependencySet,
    
    /// Singleton sets cache
    singleton_cache: HashMap<String, DependencySet>,
    
    /// Union cache for performance
    union_cache: LRUCache<(DependencySet, DependencySet), DependencySet>,
    
    /// Next dependency ID
    next_id: u64,
}

/// Statistics for extension management
#[derive(Debug, Default)]
pub struct ExtensionStatistics {
    /// Total tuples added
    pub tuples_added: u64,
    
    /// Total tuples removed
    pub tuples_removed: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Cache misses
    pub cache_misses: u64,
    
    /// Clashes detected
    pub clashes_detected: u64,
    
    /// Retrievals performed
    pub retrievals_performed: u64,
    
    /// Delta operations
    pub delta_operations: u64,
    
    /// Memory usage (bytes)
    pub memory_usage: usize,
}