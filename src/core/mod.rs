//! Core reasoning components
//!
//! This module contains the main reasoning engine, including the tableau algorithm
//! implementation, reasoning tasks, and result management.

pub mod reasoner;
pub mod tableau;
pub mod hypertableau;
pub mod blocking;
pub mod expansion;
pub mod completion;
pub mod dependency;

pub use reasoner::{Reasoner},
pub use tableau::{Tableau, TableauState};
pub use blocking::{BlockingStrategy, BlockingChecker};
pub use completion::{CompletionRule, RuleApplication};
pub use dependency::{DependencyTracker, DependencySet};
pub use expansion::{ExpansionStrategy, ExpansionManager};
