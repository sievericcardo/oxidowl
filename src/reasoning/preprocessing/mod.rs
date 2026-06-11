//! Preprocessing Pipeline Orchestrator
//!
//! This module chains all preprocessing steps into a single configurable
//! pipeline that runs before tableau expansion.  Each step is optional and
//! can be individually enabled or disabled via `PreprocessingConfig`.
//!
//! # Pipeline stages (in order)
//!
//! 1. **GCI Absorption** (`absorption`) — triggered-implication GCI absorption.
//! 2. **Common Disjunct Extraction** (`common_disjunct`) — factor out shared disjuncts.
//! 3. **Disjunct Sorting** (`disjunct_sorting`) — reorder disjuncts per heuristic.
//! 4. **Role Chain Automata** (`role_automata`) — compile role chains into automata.
//! 5. **Nominal Schema Extraction** (`nominal_schema`) — extract and ground nominal schemas.

pub mod absorption;
pub mod common_disjunct;
pub mod disjunct_sorting;
pub mod nominal_schema;
pub mod role_automata;

pub use absorption::{MergedAbsorptionResult, TriggeredImplicationAbsorber, TriggeredPattern};
pub use common_disjunct::{CommonDisjunctExtractor, CommonDisjunctResult, CommonDisjunctStats};
pub use disjunct_sorting::{
    ConceptStatCollector, ConceptStats, DisjunctSorter, DisjunctSortingStrategy,
};
pub use nominal_schema::{
    GroundedNominalSchema, NominalSchemaExtractor, NominalSchemaGrounder, NominalSchemaStats,
    NominalSchemaTemplate, TemplateAtom,
};
pub use role_automata::{
    RoleAutomataRegistry, RoleAutomataStats, RoleAutomaton, RoleAxioms, build_registry,
};
/// Alias kept for backward-compatibility.
pub type RoleAutomata = role_automata::RoleAutomaton;

use crate::core::tableau::absorption::ClauseAbsorber;
use crate::dl_clauses::DLClauseSet;

/// Configuration for the preprocessing pipeline.
#[derive(Debug, Clone)]
pub struct PreprocessingConfig {
    /// Enable triggered-implication GCI absorption.
    pub enable_absorption: bool,
    /// Enable common disjunct extraction.
    pub enable_common_disjunct: bool,
    /// Enable disjunct sorting.
    pub enable_disjunct_sorting: bool,
    /// Strategy for disjunct sorting.
    pub disjunct_sorting_strategy: DisjunctSortingStrategy,
    /// Enable role chain automata compilation.
    pub enable_role_automata: bool,
    /// Enable nominal schema extraction and grounding.
    pub enable_nominal_schemas: bool,
    /// Known individuals for nominal schema grounding.
    pub known_individuals: Vec<String>,
    /// Role axioms for automata compilation.
    pub role_axioms: RoleAxioms,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            enable_absorption: true,
            enable_common_disjunct: true,
            enable_disjunct_sorting: true,
            disjunct_sorting_strategy: DisjunctSortingStrategy::CheapFirst,
            enable_role_automata: true,
            enable_nominal_schemas: false, // off by default (experimental)
            known_individuals: Vec::new(),
            role_axioms: RoleAxioms::default(),
        }
    }
}

/// The fully-evaluated output of the preprocessing pipeline.
#[derive(Debug)]
pub struct PreprocessingResult {
    /// Rewritten clause set after all transformations.
    pub clause_set: DLClauseSet,
    /// Triggered absorption patterns (ready for tableau use).
    pub triggered_patterns: Vec<TriggeredPattern>,
    /// Merged patterns from both basic and triggered absorption.
    pub merged_absorption: Option<MergedAbsorptionResult>,
    /// Common disjunct extraction result (if enabled).
    pub common_disjunct: Option<CommonDisjunctResult>,
    /// Compiled role automata registry (if enabled).
    pub role_automata: Option<RoleAutomataRegistry>,
    /// Grounded nominal schemas (if enabled).
    pub grounded_nominal_schemas: Vec<GroundedNominalSchema>,
    /// Combined statistics.
    pub stats: PreprocessingStats,
}

/// Summary statistics for the entire pipeline.
#[derive(Debug, Default)]
pub struct PreprocessingStats {
    pub absorption_rate: f64,
    pub common_disjuncts_found: usize,
    pub role_automata_compiled: usize,
    pub nominal_groundings: usize,
}

/// Executes the preprocessing pipeline.
pub struct PreprocessingPipeline {
    config: PreprocessingConfig,
}

impl PreprocessingPipeline {
    #[must_use]
    pub fn new(config: PreprocessingConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            config: PreprocessingConfig::default(),
        }
    }

    /// Run all enabled preprocessing stages on `clause_set`.
    #[must_use]
    pub fn run(&self, clause_set: &DLClauseSet) -> PreprocessingResult {
        let mut working = clause_set.clone();
        let mut stats = PreprocessingStats::default();

        // ── Stage 1: GCI Absorption ──────────────────────────────────────────
        let (triggered_patterns, merged_absorption) = if self.config.enable_absorption {
            let basic = ClauseAbsorber::absorb(&working);
            let triggered = TriggeredImplicationAbsorber::absorb(&working);
            stats.absorption_rate = triggered.stats.absorption_rate();
            let merged = triggered.merge_with_basic(&basic);
            let patterns = triggered.patterns.clone();
            (patterns, Some(merged))
        } else {
            (Vec::new(), None)
        };

        // ── Stage 2: Common Disjunct Extraction ──────────────────────────────
        let common_disjunct = if self.config.enable_common_disjunct {
            let result = CommonDisjunctExtractor::extract(&working);
            stats.common_disjuncts_found = result.stats.common_disjuncts_found;
            working = result.rewritten.clone();
            Some(result)
        } else {
            None
        };

        // ── Stage 3: Disjunct Sorting ─────────────────────────────────────────
        if self.config.enable_disjunct_sorting {
            disjunct_sorting::sort_disjuncts(
                &mut working.disjunctive_clauses,
                self.config.disjunct_sorting_strategy,
                &working.deterministic_clauses,
            );
        }

        // ── Stage 4: Role Chain Automata ──────────────────────────────────────
        let role_automata = if self.config.enable_role_automata {
            let (registry, ra_stats) = build_registry(&self.config.role_axioms);
            stats.role_automata_compiled =
                ra_stats.atomic_roles + ra_stats.transitive_roles + ra_stats.chain_roles;
            Some(registry)
        } else {
            None
        };

        // ── Stage 5: Nominal Schema Extraction & Grounding ───────────────────
        let grounded_nominal_schemas = if self.config.enable_nominal_schemas {
            let extractor = NominalSchemaExtractor::default();
            let grounder = NominalSchemaGrounder::new(self.config.known_individuals.clone());
            let mut ns_stats = NominalSchemaStats::default();
            let groundings = grounder.ground(&extractor, &mut ns_stats);
            stats.nominal_groundings = ns_stats.groundings_produced;
            groundings
        } else {
            Vec::new()
        };

        PreprocessingResult {
            clause_set: working,
            triggered_patterns,
            merged_absorption,
            common_disjunct,
            role_automata,
            grounded_nominal_schemas,
            stats,
        }
    }
}
