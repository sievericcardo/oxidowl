//! Configuration for saturation engine

use serde::{Deserialize, Serialize};

/// Configuration for the saturation engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaturationConfig {
    /// Maximum number of non-deterministic branches before marking as `RequiresFullTableau`
    pub max_branches: usize,

    /// Enable aggressive saturation (may increase memory usage)
    pub aggressive_saturation: bool,

    /// Enable caching of saturation results
    pub enable_caching: bool,

    /// Maximum number of saturation iterations
    pub max_iterations: usize,

    /// Strategy for handling complex concepts
    pub strategy: SaturationStrategy,

    /// Enable parallel saturation
    pub enable_parallel: bool,

    /// Enable saturation statistics tracking
    pub track_statistics: bool,
}

impl Default for SaturationConfig {
    fn default() -> Self {
        Self {
            max_branches: 5,
            aggressive_saturation: false,
            enable_caching: true,
            max_iterations: 1000,
            strategy: SaturationStrategy::Balanced,
            enable_parallel: true,
            track_statistics: true,
        }
    }
}

/// Strategy for handling complex concepts during saturation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaturationStrategy {
    /// Conservative: Only saturate simple deterministic rules
    Conservative,

    /// Balanced: Saturate most deterministic rules (default)
    Balanced,

    /// Aggressive: Attempt to saturate as much as possible
    Aggressive,

    /// Custom: Use custom configuration
    Custom,
}

impl SaturationConfig {
    /// Create a new saturation configuration
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of branches
    #[must_use] 
    pub fn with_max_branches(mut self, max_branches: usize) -> Self {
        self.max_branches = max_branches;
        self
    }

    /// Enable or disable aggressive saturation
    #[must_use] 
    pub fn with_aggressive_saturation(mut self, enable: bool) -> Self {
        self.aggressive_saturation = enable;
        self
    }

    /// Set the saturation strategy
    #[must_use] 
    pub fn with_strategy(mut self, strategy: SaturationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Enable or disable parallel saturation
    #[must_use] 
    pub fn with_parallel(mut self, enable: bool) -> Self {
        self.enable_parallel = enable;
        self
    }

    /// Get a conservative configuration
    #[must_use] 
    pub fn conservative() -> Self {
        Self {
            max_branches: 3,
            aggressive_saturation: false,
            strategy: SaturationStrategy::Conservative,
            ..Default::default()
        }
    }

    /// Get an aggressive configuration
    #[must_use] 
    pub fn aggressive() -> Self {
        Self {
            max_branches: 10,
            aggressive_saturation: true,
            strategy: SaturationStrategy::Aggressive,
            ..Default::default()
        }
    }
}
