//! High-level Reasoning Interface for Oxidowl
//! 
//! This module provides high-level reasoning services and query interfaces
//! that wrap the core tableau algorithm and provide convenient APIs for
//! common reasoning tasks.

// Re-export core reasoner types for public API
pub use crate::core::reasoner::{ReasoningTask, ReasoningResult, ClassificationResult, RealizationResult};

use create::{
    Error, Result,
    ontology::{Ontology, ClassExpression, Individual, ObjectPropertyExpression, DataProperty, DataPropertyExpression, Axiom},
    core::{
        reasoner::Reasoner,
        tableau::Tableau,
    }
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{Duration, Instant};
};

/// Reasoning service that provides high-level reasoning capabilities
#[derive(Debug, Clone)]
pub struct ReasoningService {
    reasoner: Arc<RwLock<Reasoner>>,
    cache_manager: Arc<RwLock<CacheManager>>,
    config: ReasonerConfig,
}
