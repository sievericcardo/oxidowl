//! Core reasoning components
//!
//! This module contains the main reasoning engine, including the tableau algorithm
//! implementation, reasoning tasks, and result management.

pub mod blocking;
pub mod completion;
pub mod dependency;
pub mod expansion;
pub mod hypertableau;
pub mod reasoner;
pub mod tableau;

pub use blocking::{BlockingChecker, BlockingStrategy};
pub use completion::{CompletionRule, RuleApplication};
pub use dependency::{DependencySet, DependencyTracker};
pub use expansion::{ExpansionManager, ExpansionStrategy};
pub use reasoner::Reasoner;
pub use tableau::{Tableau, TableauState};
