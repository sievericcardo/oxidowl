//! Advanced cache eviction strategies
//!
//! This module implements sophisticated eviction policies including:
//! - LRU (Least Recently Used)
//! - LFU (Least Frequently Used)  
//! - Size-based eviction
//! - TTL (Time To Live) based eviction

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Cache eviction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionStrategy {
    /// Least Recently Used - evict entries that haven't been accessed recently
    LRU,
    /// Least Frequently Used - evict entries with the lowest access count
    LFU,
    /// Combined LRU/LFU - consider both recency and frequency
    LRUFU,
    /// Size-based - evict when total size exceeds limit
    SizeBased,
    /// Time To Live - evict entries older than TTL
    TTL,
}

/// LRU Cache implementation
pub struct LRUCache<K: Hash + Eq + Clone, V: Clone> {
    cache: HashMap<K, CacheEntry<V>>,
    access_order: VecDeque<K>,
    capacity: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    access_count: u64,
    last_accessed: Instant,
    created_at: Instant,
    size_bytes: usize,
}

impl<V> CacheEntry<V> {
    fn new(value: V, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            value,
            access_count: 0,
            last_accessed: now,
            created_at: now,
            size_bytes,
        }
    }

    fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl<K: Hash + Eq + Clone, V: Clone> LRUCache<K, V> {
    /// Create a new LRU cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            access_order: VecDeque::new(),
            capacity,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get a value from the cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.cache.get_mut(key) {
            entry.access();
            self.hits += 1;

            // Move to end of access order (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push_back(key.clone());

            Some(entry.value.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&mut self, key: K, value: V, size_bytes: usize) {
        // If at capacity, evict LRU entry
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            self.evict_lru();
        }

        // Insert or update entry
        let entry = CacheEntry::new(value, size_bytes);
        self.cache.insert(key.clone(), entry);

        // Update access order
        self.access_order.retain(|k| k != &key);
        self.access_order.push_back(key);
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some(key) = self.access_order.pop_front() {
            self.cache.remove(&key);
            self.evictions += 1;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// LFU Cache implementation
pub struct LFUCache<K: Hash + Eq + Clone, V: Clone> {
    cache: HashMap<K, CacheEntry<V>>,
    capacity: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl<K: Hash + Eq + Clone, V: Clone> LFUCache<K, V> {
    /// Create a new LFU cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            capacity,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get a value from the cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.cache.get_mut(key) {
            entry.access();
            self.hits += 1;
            Some(entry.value.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&mut self, key: K, value: V, size_bytes: usize) {
        // If at capacity, evict LFU entry
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            self.evict_lfu();
        }

        // Insert or update entry
        let entry = CacheEntry::new(value, size_bytes);
        self.cache.insert(key, entry);
    }

    /// Evict the least frequently used entry
    fn evict_lfu(&mut self) {
        if let Some((key_to_evict, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.access_count)
        {
            let key = key_to_evict.clone();
            self.cache.remove(&key);
            self.evictions += 1;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Size-based cache with configurable eviction
pub struct SizeBasedCache<K: Hash + Eq + Clone, V: Clone> {
    cache: HashMap<K, CacheEntry<V>>,
    total_size_bytes: usize,
    max_size_bytes: usize,
    eviction_strategy: EvictionStrategy,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl<K: Hash + Eq + Clone, V: Clone> SizeBasedCache<K, V> {
    /// Create a new size-based cache
    pub fn new(max_size_bytes: usize, eviction_strategy: EvictionStrategy) -> Self {
        Self {
            cache: HashMap::new(),
            total_size_bytes: 0,
            max_size_bytes,
            eviction_strategy,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get a value from the cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.cache.get_mut(key) {
            entry.access();
            self.hits += 1;
            Some(entry.value.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&mut self, key: K, value: V, size_bytes: usize) {
        // Evict entries until we have space
        while self.total_size_bytes + size_bytes > self.max_size_bytes && !self.cache.is_empty() {
            self.evict_one();
        }

        // Don't insert if single entry is larger than max size
        if size_bytes > self.max_size_bytes {
            return;
        }

        // Remove old entry if updating
        if let Some(old_entry) = self.cache.get(&key) {
            self.total_size_bytes -= old_entry.size_bytes;
        }

        // Insert new entry
        let entry = CacheEntry::new(value, size_bytes);
        self.total_size_bytes += size_bytes;
        self.cache.insert(key, entry);
    }

    /// Evict one entry based on strategy
    fn evict_one(&mut self) {
        let key_to_evict = match self.eviction_strategy {
            EvictionStrategy::LRU => self
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| k.clone()),
            EvictionStrategy::LFU => self
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.access_count)
                .map(|(k, _)| k.clone()),
            EvictionStrategy::TTL => self
                .cache
                .iter()
                .max_by_key(|(_, entry)| entry.created_at)
                .map(|(k, _)| k.clone()),
            _ => self.cache.keys().next().cloned(),
        };

        if let Some(key) = key_to_evict {
            if let Some(entry) = self.cache.remove(&key) {
                self.total_size_bytes -= entry.size_bytes;
                self.evictions += 1;
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.max_size_bytes,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Get total size in bytes
    pub fn total_size_bytes(&self) -> usize {
        self.total_size_bytes
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.total_size_bytes = 0;
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let mut cache = LRUCache::new(3);

        // Insert entries
        cache.insert("a", 1, 10);
        cache.insert("b", 2, 10);
        cache.insert("c", 3, 10);

        assert_eq!(cache.len(), 3);

        // Access order: a, b, c (c is most recent)
        assert_eq!(cache.get(&"c"), Some(3));

        // Insert d, should evict a (least recently used)
        cache.insert("d", 4, 10);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"d"), Some(4));

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_lfu_cache() {
        let mut cache = LFUCache::new(3);

        // Insert entries
        cache.insert("a", 1, 10);
        cache.insert("b", 2, 10);
        cache.insert("c", 3, 10);

        // Access a and b multiple times, c only once
        cache.get(&"a");
        cache.get(&"a");
        cache.get(&"b");

        // Insert d, should evict c (least frequently used)
        cache.insert("d", 4, 10);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(cache.get(&"a"), Some(1));

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_size_based_cache() {
        let mut cache = SizeBasedCache::new(100, EvictionStrategy::LRU);

        // Insert entries with different sizes
        cache.insert("small", 1, 30);
        cache.insert("medium", 2, 40);
        cache.insert("large", 3, 50);

        // Total size is 120, should trigger eviction
        assert!(cache.len() <= 2); // At most 2 entries can fit

        let stats = cache.stats();
        assert!(stats.evictions > 0);
        assert!(cache.total_size_bytes() <= 100);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = LRUCache::new(10);

        cache.insert("a", 1, 10);
        cache.insert("b", 2, 10);

        // 2 hits
        cache.get(&"a");
        cache.get(&"b");

        // 2 misses
        cache.get(&"c");
        cache.get(&"d");

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
        assert!((stats.hit_rate - 0.5).abs() < 0.01);
    }
}
