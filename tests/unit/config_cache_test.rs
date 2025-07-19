//! Unit tests for configuration and caching

use oxidowl::{
    Result,
    config::{ReasonerConfig, LogLevel, TableauAlgorithm, MonitoringLevel},
    cache::{ReasoningCache, CacheEntry, CacheKey},
    ontology::{ClassExpression, Class, IRI},
};
use std::time::{Duration, Instant};

#[test]
fn test_reasoner_config_creation() {
    let config = ReasonerConfig::default();
    
    // Test default values
    assert_eq!(config.logging.level, LogLevel::Info);
    assert_eq!(config.reasoning.tableau_algorithm, TableauAlgorithm::Traditional);
    assert!(config.reasoning.enable_blocking);
    assert!(config.cache.enable_satisfiability_cache);
    
    println!("ReasonerConfig creation works");
}

#[test]
fn test_config_validation() -> Result<()> {
    let mut config = ReasonerConfig::default();
    
    // Valid config should pass
    config.validate()?;
    
    // Invalid timeout should fail
    config.reasoning.timeout = Some(Duration::from_secs(0));
    match config.validate() {
        Ok(_) => panic!("Should have failed with zero timeout"),
        Err(_) => println!("Correctly rejected zero timeout"),
    }
    
    // Reset to valid
    config.reasoning.timeout = Some(Duration::from_secs(30));
    config.validate()?;
    
    // Invalid memory limit should fail
    config.reasoning.max_memory_mb = Some(0);
    match config.validate() {
        Ok(_) => panic!("Should have failed with zero memory limit"),
        Err(_) => println!("Correctly rejected zero memory limit"),
    }
    
    println!("Config validation works");
    Ok(())
}

#[test]
fn test_config_presets() {
    // Test debug config
    let debug_config = ReasonerConfig::debug_config();
    assert_eq!(debug_config.logging.level, LogLevel::Debug);
    assert!(debug_config.reasoning.enable_explanations);
    
    // Test test config
    let test_config = ReasonerConfig::test_config();
    assert_eq!(test_config.logging.level, LogLevel::Debug);
    assert!(test_config.reasoning.enable_explanations);
    assert_eq!(test_config.performance.worker_threads, Some(2));
    
    // Test web service config
    let web_config = ReasonerConfig::web_service_config();
    assert_eq!(web_config.server.max_connections, 500);
    assert_eq!(web_config.server.request_timeout, Duration::from_secs(60));
    
    println!("Config presets work");
}

#[test]
fn test_logging_levels() {
    // Test all logging levels
    let levels = vec![
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ];
    
    for level in levels {
        let mut config = ReasonerConfig::default();
        config.logging.level = level;
        
        // Should be valid
        assert!(config.validate().is_ok());
    }
    
    println!("Logging levels work");
}

#[test]
fn test_tableau_algorithms() {
    // Test tableau algorithm selection
    let mut config = ReasonerConfig::default();
    
    config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
    assert!(config.validate().is_ok());
    
    config.reasoning.tableau_algorithm = TableauAlgorithm::HyperTableau;
    assert!(config.validate().is_ok());
    
    println!("Tableau algorithms work");
}

#[test]
fn test_monitoring_levels() {
    let levels = vec![
        MonitoringLevel::None,
        MonitoringLevel::Basic,
        MonitoringLevel::Detailed,
        MonitoringLevel::Debug,
    ];
    
    for level in levels {
        let mut config = ReasonerConfig::default();
        config.reasoning.monitoring_level = level;
        
        // Should be valid
        assert!(config.validate().is_ok());
    }
    
    println!("Monitoring levels work");
}

#[test]
fn test_cache_creation() {
    let cache = ReasoningCache::new();
    
    assert_eq!(cache.size(), 0);
    assert!(cache.is_empty());
    
    println!("ReasoningCache creation works");
}

#[test]
fn test_cache_operations() {
    let mut cache = ReasoningCache::new();
    
    // Create cache key and entry
    let key = CacheKey::Satisfiability {
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        ontology_hash: 12345,
    };
    
    let entry = CacheEntry::Boolean {
        result: true,
        timestamp: Instant::now(),
        computation_time: Duration::from_millis(100),
    };
    
    // Test insertion
    cache.insert(key.clone(), entry.clone());
    assert_eq!(cache.size(), 1);
    assert!(!cache.is_empty());
    
    // Test retrieval
    let retrieved = cache.get(&key);
    assert!(retrieved.is_some());
    
    match retrieved.unwrap() {
        CacheEntry::Boolean { result, .. } => {
            assert_eq!(*result, true);
        }
        _ => panic!("Expected boolean cache entry"),
    }
    
    println!("Cache operations work");
}

#[test]
fn test_cache_key_types() {
    let class_expr = ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal")));
    let ontology_hash = 54321;
    
    // Test different cache key types
    let satisfiability_key = CacheKey::Satisfiability {
        class: class_expr.clone(),
        ontology_hash,
    };
    
    let subsumption_key = CacheKey::Subsumption {
        subclass: class_expr.clone(),
        superclass: class_expr.clone(),
        ontology_hash,
    };
    
    let consistency_key = CacheKey::Consistency {
        ontology_hash,
    };
    
    // Keys should be different
    assert_ne!(satisfiability_key, subsumption_key);
    assert_ne!(satisfiability_key, consistency_key);
    assert_ne!(subsumption_key, consistency_key);
    
    println!("Cache key types work");
}

#[test]
fn test_cache_entry_types() {
    let now = Instant::now();
    let duration = Duration::from_millis(50);
    
    // Test different cache entry types
    let boolean_entry = CacheEntry::Boolean {
        result: true,
        timestamp: now,
        computation_time: duration,
    };
    
    let classes_entry = CacheEntry::Classes {
        result: vec![
            ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
            ClassExpression::Class(Class::new(IRI::new("http://example.org/Cat"))),
        ],
        timestamp: now,
        computation_time: duration,
    };
    
    // Verify entry types
    match boolean_entry {
        CacheEntry::Boolean { result, .. } => assert!(result),
        _ => panic!("Expected boolean entry"),
    }
    
    match classes_entry {
        CacheEntry::Classes { result, .. } => assert_eq!(result.len(), 2),
        _ => panic!("Expected classes entry"),
    }
    
    println!("Cache entry types work");
}

#[test]
fn test_cache_expiration() {
    let mut cache = ReasoningCache::new();
    
    // Set short expiration time
    cache.set_expiration_time(Duration::from_millis(10));
    
    let key = CacheKey::Consistency { ontology_hash: 123 };
    let entry = CacheEntry::Boolean {
        result: true,
        timestamp: Instant::now(),
        computation_time: Duration::from_millis(5),
    };
    
    cache.insert(key.clone(), entry);
    assert!(cache.get(&key).is_some());
    
    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));
    
    // Entry should be expired
    cache.cleanup_expired();
    assert!(cache.get(&key).is_none());
    
    println!("Cache expiration works");
}

#[test]
fn test_cache_size_limits() {
    let mut cache = ReasoningCache::new();
    
    // Set small max size
    cache.set_max_size(2);
    
    // Add entries
    for i in 0..5 {
        let key = CacheKey::Consistency { ontology_hash: i };
        let entry = CacheEntry::Boolean {
            result: true,
            timestamp: Instant::now(),
            computation_time: Duration::from_millis(1),
        };
        cache.insert(key, entry);
    }
    
    // Should not exceed max size
    assert!(cache.size() <= 2);
    
    println!("Cache size limits work");
}

#[test]
fn test_cache_statistics() {
    let mut cache = ReasoningCache::new();
    
    let key = CacheKey::Consistency { ontology_hash: 999 };
    let entry = CacheEntry::Boolean {
        result: false,
        timestamp: Instant::now(),
        computation_time: Duration::from_millis(200),
    };
    
    // Insert and access
    cache.insert(key.clone(), entry);
    cache.get(&key); // Hit
    cache.get(&CacheKey::Consistency { ontology_hash: 888 }); // Miss
    
    let stats = cache.get_statistics();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.entries, 1);
    
    println!("Cache statistics work");
}

#[test]
fn test_cache_persistence() -> Result<()> {
    let mut cache = ReasoningCache::new();
    
    // Add some entries
    let key1 = CacheKey::Consistency { ontology_hash: 111 };
    let entry1 = CacheEntry::Boolean {
        result: true,
        timestamp: Instant::now(),
        computation_time: Duration::from_millis(50),
    };
    cache.insert(key1.clone(), entry1);
    
    // Test save
    let temp_dir = tempfile::tempdir()?;
    let cache_file = temp_dir.path().join("cache.bin");
    
    cache.save_to_file(&cache_file)?;
    assert!(cache_file.exists());
    
    // Test load
    let mut new_cache = ReasoningCache::new();
    new_cache.load_from_file(&cache_file)?;
    
    assert_eq!(new_cache.size(), 1);
    assert!(new_cache.get(&key1).is_some());
    
    println!("Cache persistence works");
    Ok(())
}

#[test]
fn test_cache_thread_safety() {
    use std::sync::Arc;
    use std::thread;
    
    let cache = Arc::new(ReasoningCache::new());
    let mut handles = vec![];
    
    // Spawn multiple threads to access cache
    for i in 0..5 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            let key = CacheKey::Consistency { ontology_hash: i };
            let entry = CacheEntry::Boolean {
                result: i % 2 == 0,
                timestamp: Instant::now(),
                computation_time: Duration::from_millis(10),
            };
            
            cache_clone.insert(key.clone(), entry);
            
            // Try to retrieve
            cache_clone.get(&key)
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_some());
    }
    
    assert_eq!(cache.size(), 5);
    
    println!("Cache thread safety works");
}

#[test]
fn test_config_serialization() -> Result<()> {
    let config = ReasonerConfig::test_config();
    
    // Test JSON serialization
    let json = serde_json::to_string(&config)?;
    assert!(!json.is_empty());
    
    // Test deserialization
    let deserialized: ReasonerConfig = serde_json::from_str(&json)?;
    assert_eq!(deserialized.logging.level, config.logging.level);
    
    println!("Config serialization works");
    Ok(())
}

#[test]
fn test_config_file_loading() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config_file = temp_dir.path().join("config.toml");
    
    // Create test config file
    let config_content = r#"
[logging]
level = "Debug"
enable_file_logging = true

[reasoning]
enable_blocking = true
enable_explanations = true
timeout_seconds = 60

[cache]
enable_satisfiability_cache = true
max_cache_size_mb = 256
"#;
    
    std::fs::write(&config_file, config_content)?;
    
    // Load config from file
    let config = ReasonerConfig::from_file(&config_file)?;
    
    assert_eq!(config.logging.level, LogLevel::Debug);
    assert!(config.logging.enable_file_logging);
    assert!(config.reasoning.enable_blocking);
    assert_eq!(config.reasoning.timeout, Some(Duration::from_secs(60)));
    
    println!("Config file loading works");
    Ok(())
}

#[test]
fn test_environment_config() {
    // Test environment variable override
    std::env::set_var("OXIDOWL_LOG_LEVEL", "Error");
    std::env::set_var("OXIDOWL_ENABLE_CACHE", "false");
    
    let config = ReasonerConfig::from_environment();
    
    assert_eq!(config.logging.level, LogLevel::Error);
    assert!(!config.cache.enable_satisfiability_cache);
    
    // Clean up
    std::env::remove_var("OXIDOWL_LOG_LEVEL");
    std::env::remove_var("OXIDOWL_ENABLE_CACHE");
    
    println!("Environment config works");
}
