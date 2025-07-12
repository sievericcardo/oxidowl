//! Main reasoner implementation
//!
//! This module provides the primary reasoning interface, coordinating between
//! the tableau algorithm, caching systems, and high-level reasoning tasks.

use crate::{
    cache::{CacheManager},
    config::ReasonerConfig,
    core::{
        tableau::{Tableau, TableauBuilder, TableauState},
        blocking::BlockingStrategy,
        expansion::ExpansionStrategy,
    },
    ontology::{Ontology, OntologyFormat, ClassExpression, Individual, Axiom, ObjectPropertyExpression, DataPropertyExpression},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, info, trace, warn};