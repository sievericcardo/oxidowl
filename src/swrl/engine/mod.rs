//! SWRL Rule Engine Module
//!
//! This module implements a modular SWRL rule execution engine with focused submodules:
//! - `core`: Main SWRLRuleEngine implementation and public API
//! - `inference`: Rule execution strategies (forward/backward chaining)
//! - `matching`: Pattern matching and unification algorithms
//! - `validation`: Rule validation and goal satisfaction logic

pub mod core;
pub mod inference;
pub mod matching;
pub mod validation;

pub use core::SWRLRuleEngine;
pub use inference::{ForwardChaining, BackwardChaining, HybridReasoning};
pub use matching::{UnificationEngine, PatternMatcher};
pub use validation::{RuleValidator, GoalChecker};