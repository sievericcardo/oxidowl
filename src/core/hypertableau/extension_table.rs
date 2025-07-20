//! Extension Tables Module
//!
//! This module implements efficient fact storage and retrieval for the hypertableau
//! algorithm. It provides delta management, incremental reasoning support, and 
//! integration with blocking and caching optimizations.

use crate::{
    core::{
        dependency::DependencySet,
    },
    ontology::{ClassExpression, ObjectPropertyExpression},
    Error, Result,
};

use super::{
    ground_disjunction::GroundDisjunction,
};

use std::{
    collections::{HashMap, HashSet, BTreeMap},
    fmt,
    hash::{Hash, Hasher},
};

use serde::{Serialize, Deserialize};

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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

impl ExtensionManager {
    /// Create a new extension manager
    pub fn new() -> Self {
        ExtensionManager {
            extension_tables: HashMap::new(),
            binary_extension_table: ExtensionTable::new(2),
            ternary_extension_table: ExtensionTable::new(3),
            clash_manager: ClashManager::new(),
            dependency_factory: DependencySetFactory::new(),
            binary_tuple_buffer: vec![String::new(); 2],
            ternary_tuple_buffer: vec![String::new(); 3],
            add_active: false,
            statistics: ExtensionStatistics::default(),
        }
    }
    
    /// Get extension table for specific arity
    pub fn get_extension_table(&mut self, arity: usize) -> &mut ExtensionTable {
        match arity {
            2 => &mut self.binary_extension_table,
            3 => &mut self.ternary_extension_table,
            _ => {
                if !self.extension_tables.contains_key(&arity) {
                    self.extension_tables.insert(arity, ExtensionTable::new(arity));
                }
                self.extension_tables.get_mut(&arity).unwrap()
            }
        }
    }
    
    /// Add a fact to the extension
    pub fn add_fact(&mut self, predicate: String, args: Vec<String>) -> Result<bool> {
        self.add_active = true;
        let arity = args.len();
        
        // Create tuple entry
        let tuple_entry = TupleEntry {
            tuple: args.clone(),
            predicate: predicate.clone(),
            dependency_set: self.dependency_factory.empty_set(),
            added_at: self.get_current_time(),
            is_core: false,
            is_active: true,
            tuple_hash: self.calculate_tuple_hash(&predicate, &args),
        };
        
        // Get appropriate table
        let arity = args.len();
        
        // Check if tuple already exists in cache first
        let tuple_key = TupleKey { predicate: predicate.clone(), args: args.clone() };
        {
            let table = self.get_extension_table(arity);
            if let Some(&existing_index) = table.tuple_cache.get(&tuple_key) {
                self.statistics.cache_hits += 1;
                return Ok(false); // Already exists
            }
        }
        
        self.statistics.cache_misses += 1;
        
        // Add tuple to table
        let tuple_index = {
            let table = self.get_extension_table(arity);
            table.add_tuple(tuple_entry)?
        };
        
        // Update cache
        {
            let table = self.get_extension_table(arity);
            table.tuple_cache.put(tuple_key, tuple_index);
        }
        
        // Check for clashes - avoid double mutable borrow by separating operations
        let clash_detected = {
            // First, ensure the table exists and get its data
            let table_exists = self.extension_tables.contains_key(&arity);
            if !table_exists {
                self.get_extension_table(arity); // This creates the table if it doesn't exist
            }
            
            // Check for clashes by examining the fact being added
            // Look for contradictory facts (e.g., A(x) and ¬A(x))
            let clash_detected = self.detect_fact_clash(&predicate, &args);
            
            clash_detected
        };
        
        if clash_detected {
            self.statistics.clashes_detected += 1;
            return Ok(false);
        }
        
        // Update statistics
        self.statistics.tuples_added += 1;
        self.add_active = false;
        
        Ok(true)
    }
    
    /// Add fact with dependency set
    pub fn add_fact_with_dependencies(
        &mut self, 
        predicate: String, 
        args: Vec<String>, 
        dependencies: DependencySet
    ) -> Result<bool> {
        self.add_active = true;
        let arity = args.len();
        
        let tuple_entry = TupleEntry {
            tuple: args.clone(),
            predicate: predicate.clone(),
            dependency_set: dependencies,
            added_at: self.get_current_time(),
            is_core: false,
            is_active: true,
            tuple_hash: self.calculate_tuple_hash(&predicate, &args),
        };
        
        let table = self.get_extension_table(arity);
        // Add tuple first
        let tuple_index = {
            let table = self.get_extension_table(arity);
            table.add_tuple(tuple_entry)?
        };
        
        // Then check for clashes separately - avoid double mutable borrow
        let clash_detected = {
            // First, ensure the table exists and get its data
            let table_exists = self.extension_tables.contains_key(&arity);
            if !table_exists {
                self.get_extension_table(arity); // This creates the table if it doesn't exist
            }
            
            // Now check for clash without borrowing the table mutably
            // Simple clash detection that works with immutable references
            self.detect_fact_clash(&predicate, &args)
        };
        
        if clash_detected {
            self.statistics.clashes_detected += 1;
            return Ok(false);
        }
        
        self.statistics.tuples_added += 1;
        self.add_active = false;
        
        Ok(true)
    }
    
    /// Create a retrieval for querying facts
    pub fn create_retrieval(
        &mut self, 
        arity: usize, 
        binding_pattern: Vec<bool>, 
        view: RetrievalView
    ) -> Result<usize> {
        let table = self.get_extension_table(arity);
        let retrieval_id = table.create_retrieval(binding_pattern, view)?;
        self.statistics.retrievals_performed += 1;
        Ok(retrieval_id)
    }
    
    /// Open a retrieval for iteration
    pub fn open_retrieval(&mut self, retrieval_id: usize, arity: usize) -> Result<()> {
        let table = self.get_extension_table(arity);
        table.open_retrieval(retrieval_id)
    }
    
    /// Get next tuple from retrieval
    pub fn next_tuple(&mut self, retrieval_id: usize, arity: usize) -> Result<Option<Vec<String>>> {
        let table = self.get_extension_table(arity);
        table.next_tuple(retrieval_id)
    }
    
    /// Check if retrieval has more tuples
    pub fn has_more_tuples(&self, retrieval_id: usize, arity: usize) -> Result<bool> {
        let table = self.extension_tables.get(&arity)
            .or_else(|| if arity == 2 { Some(&self.binary_extension_table) } 
                     else if arity == 3 { Some(&self.ternary_extension_table) } 
                     else { None })
            .ok_or_else(|| Error::invalid_input("Invalid arity"))?;
        table.has_more_tuples(retrieval_id)
    }
    
    /// Close a retrieval
    pub fn close_retrieval(&mut self, retrieval_id: usize, arity: usize) -> Result<()> {
        let table = self.get_extension_table(arity);
        table.close_retrieval(retrieval_id)
    }
    
    /// Check if extension contains a clash
    pub fn contains_clash(&self) -> bool {
        self.clash_manager.has_clash
    }
    
    /// Get clash dependency set
    pub fn get_clash_dependencies(&self) -> Option<&DependencySet> {
        self.clash_manager.clash_dependencies.as_ref()
    }
    
    /// Clear all extension data
    pub fn clear(&mut self) {
        for table in self.extension_tables.values_mut() {
            table.clear();
        }
        self.binary_extension_table.clear();
        self.ternary_extension_table.clear();
        self.clash_manager.clear();
        self.statistics = ExtensionStatistics::default();
    }
    
    /// Advance to next delta iteration
    pub fn advance_delta(&mut self) -> Result<()> {
        for table in self.extension_tables.values_mut() {
            table.advance_delta()?;
        }
        self.binary_extension_table.advance_delta()?;
        self.ternary_extension_table.advance_delta()?;
        self.statistics.delta_operations += 1;
        Ok(())
    }
    
    /// Get facts for a predicate (used by hyperresolution)
    pub fn get_facts(&self, predicate: &str, view: &RetrievalView) -> Result<Vec<Vec<String>>> {
        let mut results = Vec::new();
        
        // Search in all tables
        for table in self.extension_tables.values() {
            results.extend(table.get_facts_for_predicate(predicate, view)?);
        }
        
        // Search binary table
        results.extend(self.binary_extension_table.get_facts_for_predicate(predicate, view)?);
        
        // Search ternary table
        results.extend(self.ternary_extension_table.get_facts_for_predicate(predicate, view)?);
        
        Ok(results)
    }
    
    /// Get delta old tuples for a predicate
    pub fn get_delta_old_tuples(&self, predicate: &str) -> Result<Vec<Vec<String>>> {
        self.get_facts(predicate, &RetrievalView::DeltaOld)
    }
    
    /// Get new tuples for a predicate (for hyperresolution)
    pub fn get_new_tuples(&self, predicate: &str) -> Option<Vec<Vec<String>>> {
        // For now, return all current facts as "new" tuples
        // In a full implementation, this would track actual delta changes
        self.get_facts(predicate, &RetrievalView::Complete).ok()
    }
    
    /// Get concepts for a node (used by hyperresolution)
    pub fn get_node_concepts(&self, node: &str) -> Result<Vec<String>> {
        let mut concepts = Vec::new();
        
        // Look for unary predicates (concepts) with this node
        for table in self.extension_tables.values() {
            if table.arity == 1 {
                concepts.extend(table.get_concepts_for_node(node)?);
            }
        }
        
        Ok(concepts)
    }
    
    /// Check if a node has a specific concept
    pub fn has_concept(&self, individual: &str, concept: &str) -> Result<bool> {
        // Check in unary tables
        for table in self.extension_tables.values() {
            if table.arity == 1 && table.has_fact(concept, &[individual.to_string()])? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Add ground disjunction
    pub fn add_ground_disjunction(&mut self, disjunction: GroundDisjunction) -> Result<()> {
        // For now, just add as a special fact
        self.add_fact("GroundDisjunction".to_string(), vec![disjunction.to_string()])?;
        Ok(())
    }
    
    /// Add dependency
    pub fn add_dependency(&mut self, target: String, dependencies: DependencySet) -> Result<()> {
        // Store dependency information
        // In a full implementation, this would integrate with the dependency tracker
        Ok(())
    }
    
    /// Helper methods
    fn get_current_time(&self) -> u64 {
        // Simple counter for ordering
        self.statistics.tuples_added + self.statistics.tuples_removed
    }
    
    fn calculate_tuple_hash(&self, predicate: &str, args: &[String]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        predicate.hash(&mut hasher);
        for arg in args {
            arg.hash(&mut hasher);
        }
        hasher.finish()
    }
    
    /// Get statistics
    pub fn get_statistics(&self) -> &ExtensionStatistics {
        &self.statistics
    }

    /// Check if a concept assertion exists
    pub fn contains_concept_assertion(&self, node_id: &str, concept: &crate::ontology::ClassExpression) -> bool {
        // For simple named classes, check if node is in that class's extension
        if let crate::ontology::ClassExpression::Class(class) = concept {
            let class_name = class.iri.as_str();
            if let Some(table) = self.get_table_for_arity(1) {
                return table.has_fact(class_name, &[node_id.to_string()]).unwrap_or(false);
            }
        }
        // For complex expressions, we'd need more sophisticated handling
        false
    }
    
    /// Check if a role assertion exists
    pub fn contains_role_assertion(&self, subj_id: &str, property: &crate::ontology::ObjectPropertyExpression, obj_id: &str) -> bool {
        match property {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                let property_name = prop.iri.as_str();
                // Check if we have the assertion in the binary table
                if let Some(table) = self.get_table_for_arity(2) {
                    table.has_fact(property_name, &[subj_id.to_string(), obj_id.to_string()]).unwrap_or(false)
                } else {
                    false
                }
            }
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                // For inverse property P^-, check if (obj, subj) is in P
                let property_name = prop.iri.as_str();
                if let Some(table) = self.get_table_for_arity(2) {
                    // Inverse property: if we want to check P^-(a,b), we check P(b,a)
                    table.has_fact(property_name, &[obj_id.to_string(), subj_id.to_string()]).unwrap_or(false)
                } else {
                    false
                }
            }
            crate::ontology::ObjectPropertyExpression::PropertyChain(chain) => {
                // For property chains, we need to check if there's a path
                // This is a simplified implementation - full reasoning would require more sophisticated path checking
                if chain.len() == 2 {
                    // Simple case: check if there exists an intermediate individual z such that
                    // first_prop(subj, z) and second_prop(z, obj)
                    self.check_property_chain_simple(subj_id, obj_id, chain)
                } else {
                    // For longer chains, we'd need recursive checking
                    // For now, return false as this requires complex reasoning
                    false
                }
            }
        }
    }
    
    /// Check simple property chain of length 2
    fn check_property_chain_simple(&self, subj_id: &str, obj_id: &str, chain: &[crate::ontology::ObjectPropertyExpression]) -> bool {
        if chain.len() != 2 {
            return false;
        }
        
        // Get the property names
        let first_prop = match &chain[0] {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => prop.iri.as_str(),
            _ => return false, // Skip complex expressions in chains for now
        };
        
        let second_prop = match &chain[1] {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => prop.iri.as_str(),
            _ => return false,
        };
        
        // Check if there exists an intermediate individual
        if let Some(table) = self.get_table_for_arity(2) {
            // Get all facts for the first property
            if let Ok(first_facts) = table.get_facts_for_predicate(first_prop, &crate::core::hypertableau::extension_table::RetrievalView::Complete) {
                for fact in first_facts {
                    if fact.len() == 2 && fact[0] == subj_id {
                        let intermediate = &fact[1];
                        // Check if second property connects intermediate to obj
                        if table.has_fact(second_prop, &[intermediate.clone(), obj_id.to_string()]).unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// Check if two nodes are equal
    pub fn are_nodes_equal(&self, left_id: &str, right_id: &str) -> bool {
        // Check direct equality
        if left_id == right_id {
            return true;
        }
        
        // Check if there's an explicit equality fact
        let equality_key = "SameAs";
        if let Some(table) = self.get_table_for_arity(2) {
            return table.has_fact(equality_key, &[left_id.to_string(), right_id.to_string()]).unwrap_or(false) ||
                   table.has_fact(equality_key, &[right_id.to_string(), left_id.to_string()]).unwrap_or(false);
        }
        
        false
    }
    
    /// Detect if adding a fact would create a clash
    fn detect_fact_clash(&self, predicate: &str, args: &[String]) -> bool {
        let arity = args.len();
        
        // Check for explicit negation clashes
        if predicate.starts_with("¬") || predicate.starts_with("neg_") {
            // If we're adding ¬A(x), check if A(x) already exists
            let positive_pred = if predicate.starts_with("¬") {
                &predicate[3..] // Remove ¬ prefix (3 bytes for UTF-8)
            } else {
                &predicate[4..] // Remove "neg_" prefix
            };
            
            // Check if the positive fact already exists
            if let Some(table) = self.get_table_for_arity(arity) {
                if table.has_fact(positive_pred, args).unwrap_or(false) {
                    return true; // Clash detected
                }
            }
        } else {
            // If we're adding A(x), check if ¬A(x) already exists
            let neg_pred1 = format!("¬{}", predicate);
            let neg_pred2 = format!("neg_{}", predicate);
            
            if let Some(table) = self.get_table_for_arity(arity) {
                if table.has_fact(&neg_pred1, args).unwrap_or(false) ||
                   table.has_fact(&neg_pred2, args).unwrap_or(false) {
                    return true; // Clash detected
                }
            }
        }
        
        // Check for disjoint class conflicts (basic implementation)
        if args.len() == 1 && !predicate.contains("_") {
            // This is likely a concept assertion C(a)
            // Check if there are any known disjoint concepts
            let disjoint_concepts = self.get_disjoint_concepts(predicate);
            if let Some(table) = self.get_table_for_arity(arity) {
                for disjoint in disjoint_concepts {
                    if table.has_fact(&disjoint, args).unwrap_or(false) {
                        return true; // Disjoint class clash
                    }
                }
            }
        }
        
        false
    }
    
    /// Get table for specific arity (immutable access)
    fn get_table_for_arity(&self, arity: usize) -> Option<&ExtensionTable> {
        match arity {
            2 => Some(&self.binary_extension_table),
            3 => Some(&self.ternary_extension_table),
            _ => self.extension_tables.get(&arity),
        }
    }
    
    /// Get concepts known to be disjoint with the given concept
    fn get_disjoint_concepts(&self, _concept: &str) -> Vec<String> {
        // Placeholder: In a full implementation, this would query the ontology
        // for disjoint class axioms
        vec![]
    }

    /// Check if two nodes are unequal
    pub fn are_nodes_unequal(&self, left_id: &str, right_id: &str) -> bool {
        // Check if there's an explicit inequality fact
        let inequality_key = "DifferentFrom";
        if let Some(table) = self.get_table_for_arity(2) {
            if table.has_fact(inequality_key, &[left_id.to_string(), right_id.to_string()]).unwrap_or(false) ||
               table.has_fact(inequality_key, &[right_id.to_string(), left_id.to_string()]).unwrap_or(false) {
                return true;
            }
        }
        
        // If they are explicitly equal, they cannot be unequal
        if self.are_nodes_equal(left_id, right_id) {
            return false;
        }
        
        // By default, different individuals are assumed to be unequal unless proven otherwise
        // This implements the unique name assumption
        left_id != right_id
    }

    /// Ensure an individual exists in the extension tables
    pub fn ensure_individual_exists(&mut self, individual_name: &str) -> Result<()> {
        // Add individual to internal tracking if not already present
        self.add_fact("Individual".to_string(), vec![individual_name.to_string()])?;
        Ok(())
    }

    /// Get all individuals in the extension tables
    pub fn get_all_individuals(&self) -> Result<Vec<String>> {
        // Simplified implementation - return individuals from the "Individual" predicate
        self.get_facts("Individual", &RetrievalView::Complete)
            .map(|facts| facts.into_iter().map(|fact| fact.into_iter().next().unwrap_or_default()).collect())
    }

    /// Get all concepts for an individual
    pub fn get_individual_concepts(&self, individual: &str) -> Result<Vec<String>> {
        // Simplified implementation - would need to scan concept facts
        Ok(Vec::new())
    }

    /// Add blocking relationship
    pub fn add_blocking(&mut self, blocker: String, blocked: String) -> Result<()> {
        self.add_fact("Blocks".to_string(), vec![blocker, blocked])?;
        Ok(())
    }

    /// Add concept assertion with dependency
    pub fn add_concept_assertion_with_dependency(
        &mut self,
        individual: &str,
        concept: &ClassExpression,
        dependencies: DependencySet,
    ) -> Result<bool> {
        let concept_name = match concept {
            ClassExpression::Class(class) => class.iri.to_string(),
            _ => "ComplexConcept".to_string(), // Simplified for complex concepts
        };
        
        self.add_fact_with_dependencies(
            concept_name,
            vec![individual.to_string()],
            dependencies,
        )
    }

    /// Add role assertion with dependency
    pub fn add_role_assertion_with_dependency(
        &mut self,
        subject: &str,
        property: &ObjectPropertyExpression,
        object: &str,
        dependencies: DependencySet,
    ) -> Result<bool> {
        let property_name = match property {
            ObjectPropertyExpression::ObjectProperty(prop) => prop.iri.to_string(),
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("inverse({})", prop.iri.to_string())
            },
            ObjectPropertyExpression::PropertyChain(_) => {
                return Err(crate::Error::Reasoning { 
                    message: "Property chains not supported in assertions".to_string() 
                });
            }
        };
        
        self.add_fact_with_dependencies(
            property_name,
            vec![subject.to_string(), object.to_string()],
            dependencies,
        )
    }

    /// Add equality with dependency
    pub fn add_equality_with_dependency(
        &mut self,
        left: &str,
        right: &str,
        dependencies: DependencySet,
    ) -> Result<bool> {
        self.add_fact_with_dependencies(
            "SameAs".to_string(),
            vec![left.to_string(), right.to_string()],
            dependencies,
        )
    }

    /// Add inequality with dependency
    pub fn add_inequality_with_dependency(
        &mut self,
        left: &str,
        right: &str,
        dependencies: DependencySet,
    ) -> Result<bool> {
        self.add_fact_with_dependencies(
            "DifferentFrom".to_string(),
            vec![left.to_string(), right.to_string()],
            dependencies,
        )
    }

    /// Add concept assertion (simplified interface)
    pub fn add_concept_assertion(&mut self, individual: &str, concept: &ClassExpression) -> Result<()> {
        let dependencies = DependencySet::empty();
        self.add_concept_assertion_with_dependency(individual, concept, dependencies)?;
        Ok(())
    }

    /// Add role assertion (simplified interface)
    pub fn add_role_assertion(&mut self, subject: &str, property: &ObjectPropertyExpression, object: &str) -> Result<()> {
        let dependencies = DependencySet::empty();
        self.add_role_assertion_with_dependency(subject, property, object, dependencies)?;
        Ok(())
    }
    
    /// Reset the extension manager
    pub fn reset(&mut self) {
        // Clear all extension tables
        self.extension_tables.clear();
        
        // Reset binary and ternary tables
        self.binary_extension_table = ExtensionTable::new(2);
        self.ternary_extension_table = ExtensionTable::new(3);
        
        // Reset clash manager
        self.clash_manager = ClashManager::new();
        
        // Reset dependency factory
        self.dependency_factory = DependencySetFactory::new();
        
        // Clear buffers
        self.binary_tuple_buffer.clear();
        self.binary_tuple_buffer.resize(2, String::new());
        self.ternary_tuple_buffer.clear();
        self.ternary_tuple_buffer.resize(3, String::new());
        
        // Reset flags and statistics
        self.add_active = false;
        self.statistics = ExtensionStatistics::default();
    }
}

impl ExtensionTable {
    /// Create a new extension table
    pub fn new(arity: usize) -> Self {
        ExtensionTable {
            arity,
            tuples: Vec::new(),
            predicate_index: HashMap::new(),
            delta_new: HashSet::new(),
            delta_old: HashSet::new(),
            active_retrievals: Vec::new(),
            tuple_cache: LRUCache::new(10000), // 10K cache size
            blocking_data: BlockingData::new(),
            current_size: 0,
            max_size: 1_000_000, // 1M tuples max
        }
    }
    
    /// Add a tuple to the table
    pub fn add_tuple(&mut self, tuple_entry: TupleEntry) -> Result<usize> {
        if self.current_size >= self.max_size {
            return Err(Error::resource_exhausted("Extension table full"));
        }
        
        let index = self.tuples.len();
        let predicate = tuple_entry.predicate.clone();
        
        // Add to predicate index
        self.predicate_index.entry(predicate).or_insert_with(Vec::new).push(index);
        
        // Add to delta new
        self.delta_new.insert(index);
        
        // Store tuple
        self.tuples.push(tuple_entry);
        self.current_size += 1;
        
        Ok(index)
    }
    
    /// Create a new retrieval
    pub fn create_retrieval(&mut self, binding_pattern: Vec<bool>, view: RetrievalView) -> Result<usize> {
        let retrieval_id = self.active_retrievals.len();
        let retrieval = Retrieval {
            id: retrieval_id,
            arity: self.arity,
            binding_pattern,
            bindings: vec![None; self.arity],
            view,
            position: 0,
            results: Vec::new(),
            is_open: false,
            tuple_buffer: vec![String::new(); self.arity],
        };
        
        self.active_retrievals.push(retrieval);
        Ok(retrieval_id)
    }
    
    /// Open retrieval for iteration
    pub fn open_retrieval(&mut self, retrieval_id: usize) -> Result<()> {
        // Collect the data first to avoid borrowing conflicts
        let extension_indices = self.get_extension_indices();
        let delta_new: Vec<usize> = self.delta_new.iter().copied().collect();
        let delta_old: Vec<usize> = self.delta_old.iter().copied().collect();

        if let Some(retrieval) = self.active_retrievals.get_mut(retrieval_id) {
            retrieval.is_open = true;
            retrieval.position = 0;
            
            // Populate results based on view
            retrieval.results = match retrieval.view {
                RetrievalView::Extension => extension_indices,
                RetrievalView::DeltaNew => delta_new,
                RetrievalView::DeltaOld => delta_old,
                RetrievalView::ExtensionThis => extension_indices,
                RetrievalView::Complete => {
                    let mut indices = extension_indices;
                    indices.extend(&delta_new);
                    indices.extend(&delta_old);
                    indices
                }
            };
            
            Ok(())
        } else {
            Err(Error::InvalidInput { message: "Invalid retrieval ID".to_string() })
        }
    }
    
    /// Get next tuple from retrieval
    pub fn next_tuple(&mut self, retrieval_id: usize) -> Result<Option<Vec<String>>> {
        if let Some(retrieval) = self.active_retrievals.get_mut(retrieval_id) {
            if retrieval.position < retrieval.results.len() {
                let tuple_index = retrieval.results[retrieval.position];
                retrieval.position += 1;
                
                if let Some(tuple_entry) = self.tuples.get(tuple_index) {
                    retrieval.tuple_buffer = tuple_entry.tuple.clone();
                    Ok(Some(tuple_entry.tuple.clone()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Err(Error::InvalidInput { message: "Invalid retrieval ID".to_string() })
        }
    }
    
    /// Check if retrieval has more tuples
    pub fn has_more_tuples(&self, retrieval_id: usize) -> Result<bool> {
        if let Some(retrieval) = self.active_retrievals.get(retrieval_id) {
            Ok(retrieval.position < retrieval.results.len())
        } else {
            Err(Error::InvalidInput { message: "Invalid retrieval ID".to_string() })
        }
    }
    
    /// Close retrieval
    pub fn close_retrieval(&mut self, retrieval_id: usize) -> Result<()> {
        if let Some(retrieval) = self.active_retrievals.get_mut(retrieval_id) {
            retrieval.is_open = false;
            retrieval.results.clear();
            Ok(())
        } else {
            Err(Error::InvalidInput { message: "Invalid retrieval ID".to_string() })
        }
    }
    
    /// Clear table
    pub fn clear(&mut self) {
        self.tuples.clear();
        self.predicate_index.clear();
        self.delta_new.clear();
        self.delta_old.clear();
        self.active_retrievals.clear();
        self.tuple_cache.clear();
        self.blocking_data.clear();
        self.current_size = 0;
    }
    
    /// Advance delta
    pub fn advance_delta(&mut self) -> Result<()> {
        // Move delta_new to delta_old
        self.delta_old.extend(&self.delta_new);
        self.delta_new.clear();
        Ok(())
    }
    
    /// Get facts for predicate
    pub fn get_facts_for_predicate(&self, predicate: &str, view: &RetrievalView) -> Result<Vec<Vec<String>>> {
        let mut results = Vec::new();
        
        if let Some(indices) = self.predicate_index.get(predicate) {
            for &index in indices {
                let include = match view {
                    RetrievalView::Extension => !self.delta_new.contains(&index) && !self.delta_old.contains(&index),
                    RetrievalView::DeltaNew => self.delta_new.contains(&index),
                    RetrievalView::DeltaOld => self.delta_old.contains(&index),
                    RetrievalView::ExtensionThis => !self.delta_new.contains(&index) && !self.delta_old.contains(&index),
                    RetrievalView::Complete => true,
                };
                
                if include {
                    if let Some(tuple_entry) = self.tuples.get(index) {
                        results.push(tuple_entry.tuple.clone());
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get concepts for node
    pub fn get_concepts_for_node(&self, node: &str) -> Result<Vec<String>> {
        let mut concepts = Vec::new();
        
        if self.arity == 1 {
            for tuple_entry in &self.tuples {
                if tuple_entry.is_active && 
                   tuple_entry.tuple.len() == 1 && 
                   tuple_entry.tuple[0] == node {
                    concepts.push(tuple_entry.predicate.clone());
                }
            }
        }
        
        Ok(concepts)
    }
    
    /// Check if table has a specific fact
    pub fn has_fact(&self, predicate: &str, args: &[String]) -> Result<bool> {
        if let Some(indices) = self.predicate_index.get(predicate) {
            for &index in indices {
                if let Some(tuple_entry) = self.tuples.get(index) {
                    if tuple_entry.is_active && tuple_entry.tuple == args {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
    
    /// Get extension indices (non-delta)
    fn get_extension_indices(&self) -> Vec<usize> {
        (0..self.tuples.len())
            .filter(|&i| !self.delta_new.contains(&i) && !self.delta_old.contains(&i))
            .collect()
    }
}

impl<K: Clone + Eq + Hash, V: Clone> LRUCache<K, V> {
    fn new(capacity: usize) -> Self {
        LRUCache {
            capacity,
            map: HashMap::new(),
            access_order: BTreeMap::new(),
            next_access: 0,
        }
    }
    
    fn get(&mut self, key: &K) -> Option<&V> {
        if let Some((value, _)) = self.map.get_mut(key) {
            let access_time = self.next_access;
            self.next_access += 1;
            self.access_order.insert(access_time, key.clone());
            Some(value)
        } else {
            None
        }
    }
    
    fn put(&mut self, key: K, value: V) {
        if self.map.len() >= self.capacity {
            // Remove least recently used
            if let Some((_, lru_key)) = self.access_order.pop_first() {
                self.map.remove(&lru_key);
            }
        }
        
        let access_time = self.next_access;
        self.next_access += 1;
        self.map.insert(key.clone(), (value, access_time));
        self.access_order.insert(access_time, key);
    }
    
    fn clear(&mut self) {
        self.map.clear();
        self.access_order.clear();
        self.next_access = 0;
    }
}

impl BlockingData {
    fn new() -> Self {
        BlockingData {
            blocked_tuples: HashSet::new(),
            blocking_relationships: HashMap::new(),
            signature_cache: HashMap::new(),
        }
    }
    
    fn clear(&mut self) {
        self.blocked_tuples.clear();
        self.blocking_relationships.clear();
        self.signature_cache.clear();
    }
}

impl ClashManager {
    fn new() -> Self {
        ClashManager {
            has_clash: false,
            clash_dependencies: None,
            clash_rules: Vec::new(),
            clash_history: Vec::new(),
        }
    }
    
    fn check_for_clash(&mut self, predicate: &str, args: &[String], table: &ExtensionTable) -> Result<bool> {
        // Simple clash detection - look for complementary concepts
        if predicate.starts_with("¬") {
            let positive_predicate = &predicate[2..]; // Remove ¬ prefix
            if table.has_fact(positive_predicate, args)? {
                self.has_clash = true;
                return Ok(true);
            }
        } else {
            let negative_predicate = format!("¬{}", predicate);
            if table.has_fact(&negative_predicate, args)? {
                self.has_clash = true;
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn clear(&mut self) {
        self.has_clash = false;
        self.clash_dependencies = None;
        self.clash_history.clear();
    }
}

impl DependencySetFactory {
    fn new() -> Self {
        DependencySetFactory {
            empty_set: DependencySet::empty(),
            singleton_cache: HashMap::new(),
            union_cache: LRUCache::new(1000),
            next_id: 0,
        }
    }
    
    fn empty_set(&self) -> DependencySet {
        self.empty_set.clone()
    }
}

impl fmt::Display for ExtensionStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "Extension Statistics:\n\
             Tuples Added: {}\n\
             Tuples Removed: {}\n\
             Cache Hits: {}\n\
             Cache Misses: {}\n\
             Clashes Detected: {}\n\
             Retrievals Performed: {}\n\
             Delta Operations: {}\n\
             Memory Usage: {} bytes",
            self.tuples_added, self.tuples_removed, self.cache_hits, self.cache_misses,
            self.clashes_detected, self.retrievals_performed, self.delta_operations, self.memory_usage
        )
    }
}

/// Generate a unique identifier for objects
pub fn generate_unique_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Generate a unique string identifier for objects  
pub fn generate_unique_string_id() -> String {
    let id = generate_unique_id();
    format!("id_{}", id)
}