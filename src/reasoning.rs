//! High-level Reasoning Interface for Oxidowl
//! 
//! This module provides high-level reasoning services and query interfaces
//! that wrap the core tableau algorithm and provide convenient APIs for
//! common reasoning tasks.

// Re-export core reasoner types for public API
pub use crate::core::reasoner::{ReasoningTask, ReasoningResult, ClassificationResult, RealizationResult};
