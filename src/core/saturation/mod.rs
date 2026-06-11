//! Saturation-based reasoning engine for OWL 2 DL
//!
//! This module implements a saturation engine that precomputes deterministic consequences
//! for efficient classification and reasoning. The engine identifies concepts that can be
//! fully saturated deterministically and marks those requiring tableau expansion.

pub mod config;
pub mod cycle_detection;
pub mod engine;
pub mod node;
pub mod rules;

pub use config::{SaturationConfig, SaturationStrategy};
pub use engine::{SaturationEngine, SaturationResult};
pub use node::{SaturationNode, SaturationStatus};
pub use rules::{SaturationRule, SaturationRuleSet};
