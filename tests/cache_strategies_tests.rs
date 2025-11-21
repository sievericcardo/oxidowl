//! Integration tests for cache eviction strategies

use oxidowl::cache_strategies::{EvictionStrategy, LFUCache, LRUCache, SizeBasedCache};

#[test]
fn test_lru_eviction_basic() {
    let mut cache = LRUCache::new(3);

    // Fill cache
    cache.insert("key1", "value1", 100);
    cache.insert("key2", "value2", 100);
    cache.insert("key3", "value3", 100);

    assert_eq!(cache.len(), 3);

    // Access key1, making it most recently used
    assert_eq!(cache.get(&"key1"), Some("value1"));

    // Insert key4, should evict key2 (least recently used)
    cache.insert("key4", "value4", 100);

    assert_eq!(cache.len(), 3);
    assert_eq!(cache.get(&"key1"), Some("value1"));
    assert_eq!(cache.get(&"key2"), None); // Evicted
    assert_eq!(cache.get(&"key3"), Some("value3"));
    assert_eq!(cache.get(&"key4"), Some("value4"));

    let stats = cache.stats();
    assert_eq!(stats.evictions, 1);
}

#[test]
fn test_lru_update_order() {
    let mut cache = LRUCache::new(2);

    cache.insert("a", 1, 10);
    cache.insert("b", 2, 10);

    // Access a to make it most recent
    cache.get(&"a");

    // Insert c, should evict b
    cache.insert("c", 3, 10);

    assert_eq!(cache.get(&"a"), Some(1));
    assert_eq!(cache.get(&"b"), None);
    assert_eq!(cache.get(&"c"), Some(3));
}

#[test]
fn test_lfu_eviction_basic() {
    let mut cache = LFUCache::new(3);

    cache.insert("key1", "value1", 100);
    cache.insert("key2", "value2", 100);
    cache.insert("key3", "value3", 100);

    // Access key1 and key2 multiple times
    cache.get(&"key1");
    cache.get(&"key1");
    cache.get(&"key2");

    // key3 has lowest frequency (0), should be evicted
    cache.insert("key4", "value4", 100);

    assert_eq!(cache.len(), 3);
    assert_eq!(cache.get(&"key1"), Some("value1"));
    assert_eq!(cache.get(&"key2"), Some("value2"));
    assert_eq!(cache.get(&"key3"), None); // Evicted
    assert_eq!(cache.get(&"key4"), Some("value4"));

    let stats = cache.stats();
    assert_eq!(stats.evictions, 1);
}

#[test]
fn test_lfu_frequency_tracking() {
    let mut cache = LFUCache::new(2);

    cache.insert("a", 1, 10);
    cache.insert("b", 2, 10);

    // Make 'a' more frequently used
    for _ in 0..5 {
        cache.get(&"a");
    }
    cache.get(&"b");

    // Insert c, should evict b (lower frequency)
    cache.insert("c", 3, 10);

    assert_eq!(cache.get(&"a"), Some(1));
    assert_eq!(cache.get(&"b"), None);
    assert_eq!(cache.get(&"c"), Some(3));
}

#[test]
fn test_size_based_eviction() {
    let mut cache = SizeBasedCache::new(250, EvictionStrategy::LRU);

    // Insert items totaling 250 bytes
    cache.insert("key1", "value1", 100);
    cache.insert("key2", "value2", 100);
    cache.insert("key3", "value3", 100);

    // Should have evicted at least one to stay under 250 bytes
    assert!(cache.total_size_bytes() <= 250);
    let stats = cache.stats();
    assert!(stats.evictions > 0);
}

#[test]
fn test_size_based_lru_eviction_order() {
    let mut cache = SizeBasedCache::new(200, EvictionStrategy::LRU);

    cache.insert("a", 1, 80);
    cache.insert("b", 2, 80);

    // Access a to make it most recent
    cache.get(&"a");

    // Now insert c (80 bytes), total would be 240, should evict b (least recently used)
    cache.insert("c", 3, 80);

    // Total should be under 200, and b should have been evicted
    assert!(cache.total_size_bytes() <= 200);
    assert_eq!(cache.get(&"b"), None); // b was LRU, should be evicted
    assert_eq!(cache.get(&"a"), Some(1));
    assert_eq!(cache.get(&"c"), Some(3));
}

#[test]
fn test_size_based_lfu_eviction_order() {
    let mut cache = SizeBasedCache::new(200, EvictionStrategy::LFU);

    cache.insert("a", 1, 80);
    cache.insert("b", 2, 80);

    // Make 'a' more frequently used
    cache.get(&"a");
    cache.get(&"a");

    cache.insert("c", 3, 80);

    // Total is 240, over limit. With LFU, b should be evicted (lowest frequency)
    assert!(cache.total_size_bytes() <= 200);
    assert_eq!(cache.get(&"b"), None); // b was LFU, should be evicted
    assert_eq!(cache.get(&"a"), Some(1));
}

#[test]
fn test_cache_stats_accuracy() {
    let mut cache = LRUCache::new(10);

    cache.insert("a", 1, 10);
    cache.insert("b", 2, 10);

    // 3 hits
    cache.get(&"a");
    cache.get(&"a");
    cache.get(&"b");

    // 2 misses
    cache.get(&"x");
    cache.get(&"y");

    let stats = cache.stats();
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.size, 2);
    assert_eq!(stats.capacity, 10);
    assert!((stats.hit_rate - 0.6).abs() < 0.01);
}

#[test]
fn test_lru_clear() {
    let mut cache = LRUCache::new(10);

    cache.insert("a", 1, 10);
    cache.insert("b", 2, 10);

    assert_eq!(cache.len(), 2);

    cache.clear();

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
    assert_eq!(cache.get(&"a"), None);
}

#[test]
fn test_oversized_entry_handling() {
    let mut cache = SizeBasedCache::new(100, EvictionStrategy::LRU);

    // Try to insert an entry larger than max size
    cache.insert("huge", 1, 150);

    // Should not be inserted
    assert_eq!(cache.get(&"huge"), None);
    assert_eq!(cache.total_size_bytes(), 0);

    // Normal-sized entry should work
    cache.insert("normal", 2, 50);
    assert_eq!(cache.get(&"normal"), Some(2));
}

#[test]
fn test_realistic_cache_workload() {
    let mut cache = LRUCache::new(100);

    // Simulate realistic access pattern
    for i in 0..150 {
        cache.insert(format!("key{}", i), i, 10);
    }

    // Cache should be at capacity
    assert_eq!(cache.len(), 100);

    // Access some recent entries (should be hits)
    for i in 100..150 {
        assert!(cache.get(&format!("key{}", i)).is_some());
    }

    // Access some old entries (should be misses)
    for i in 0..50 {
        assert!(cache.get(&format!("key{}", i)).is_none());
    }

    let stats = cache.stats();
    assert_eq!(stats.size, 100);
    assert_eq!(stats.evictions, 50);
    assert!(stats.hit_rate > 0.0);
}

#[test]
fn test_cache_update_without_eviction() {
    let mut cache = LRUCache::new(3);

    cache.insert("a", 1, 10);
    cache.insert("b", 2, 10);

    // Update existing key should not trigger eviction
    cache.insert("a", 10, 10);

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&"a"), Some(10));

    let stats = cache.stats();
    assert_eq!(stats.evictions, 0);
}
