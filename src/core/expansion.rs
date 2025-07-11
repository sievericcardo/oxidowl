//! Existential expansion strategies and management
//!
//! This module implements expansion strategies for managing how existential
//! concepts are expanded in the tableau, based on the sophisticated expansion
//! management systems from Konclude, HermiT, and Pellet.

use crate::{
    core::{
        completion::{CompletionRule, RuleApplication, RuleContext, RulePriority},
        dependency::{DependencySet, DependencyTracker, DependencyType},
    },
    ontology::{ClassExpression, Individual, Role, ObjectPropertyExpression},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet, VecDeque, BinaryHeap},
    cmp::Ordering,
    fmt,
};