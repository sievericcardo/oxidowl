//! Classification and realisation operations
//!
//! This module implements complex reasoning operations like building class hierarchies,
//! property classification, and individual type inference.

use crate::{
    Result,
    cache::CacheManager,
    config::PerformanceConfig,
    core::{
        lock_helpers::read_lock,
        reasoner::{
            datatype_validation::DatatypeValidator,
            results::{
                ClassificationCounters, ClassificationResult, PropertyClassificationResult,
                RealisationResult,
            },
            statistics::ReasoningStatistics,
            tasks::ReasoningTaskService,
        },
        saturation::{
            DependencyClassifier, SaturationConfig, SaturationEngine, SaturationStatus, absorb,
            absorption::collect_named_classes,
        },
    },
    ontology::{
        ClassExpression, DataPropertyExpression, IRI, Individual, ObjectPropertyExpression,
        Ontology, OntologyRef,
    },
};
use log::{debug, info, warn};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

/// Hard recursion depth cap for
/// [`ClassificationService::check_instance_with_datatype_reasoning`].
///
/// Second safety net behind the [`INSTANCE_EVAL_STACK`] re-entrancy guard: a
/// pathological ontology whose evaluation legitimately nests more than this
/// many *distinct* `(individual, class expression)` states gets a conservative
/// `false` (logged once) instead of exhausting the native stack.  The caller's
/// tableau snapshot fallback remains authoritative for the cut state.
const INSTANCE_EVAL_MAX_DEPTH: usize = 256;

std::thread_local! {
    /// `(individual IRI, defined class IRI)` pairs whose complex definition is
    /// currently being evaluated by Strategy 5 of
    /// `ClassificationService::check_instance_with_datatype_reasoning`, or
    /// whose complex GCI (`SubClassOf(E, I)`) is being evaluated by Strategy 6
    /// under the same key (`individual`, `I`).
    ///
    /// A definition may reference other *named* classes (`D ≡ Tree ⊓ (age some
    /// int < 40)`), and checking those re-enters the named-class branch, which
    /// may evaluate further defined subclasses.  Cyclic definition references
    /// (`D ≡ E ⊓ …` with `E` reachable back from `D`) would otherwise recurse
    /// forever; the on-stack set breaks any such cycle by returning a
    /// conservative `false` for the re-entered pair (the tableau snapshot
    /// remains the caller's fallback).  The pair set is finite (asserted
    /// individuals × defined classes), so recursion always terminates; acyclic
    /// definition chains are evaluated in full.
    ///
    /// Kept as a helper-scoped guard *in addition to* the general
    /// [`INSTANCE_EVAL_STACK`] entry guard: it cuts named-class re-entry from
    /// *inside* the Strategy helpers (where the entry guard may not hold a
    /// matching frame), e.g. a Strategy-6 `check_complex_gci` re-entering the
    /// same named class through a complex filler.
    static DEFINED_EVAL_STACK: RefCell<HashSet<(String, IRI)>> =
        RefCell::new(HashSet::new());
}

std::thread_local! {
    /// General re-entrancy guard: every `(individual key, class expression)`
    /// state whose evaluation by
    /// [`ClassificationService::check_instance_with_datatype_reasoning`] is
    /// currently in progress on this thread.
    ///
    /// The individual key is the `Individual` Display form (named → IRI,
    /// anonymous → stable `_:id`).  Unlike [`DEFINED_EVAL_STACK`] — which is
    /// keyed only by `(individual, named class IRI)` and only inside Strategies
    /// 3/5/6 — this guard covers **every** entry into the checker, including
    /// the complex-expression branch that never touches a named class, and it
    /// is held for the *whole* call (Strategies 1–4 included) rather than only
    /// inside a helper, so sibling Strategy-5/6 paths that re-enter the same
    /// query state after the helper guard popped are cut as well.  Without it,
    /// recursion alternating through different class expressions / fillers
    /// (`check_instance ⇄ check_complex_expression`, e.g. the defined-subclass
    /// Strategy 5 re-entering the query state from each defined subclass, or
    /// `A ≡ ∃r.B` / `B ≡ ∃r.A` filler cycles) could nest without bound or
    /// re-evaluate the same state factorially often — observed as a
    /// non-terminating 100 % CPU spin in a consumer workload.
    ///
    /// A re-entered state returns a conservative `Ok(false)`; this is the
    /// co-inductive reading of a circular proof (any proof of the re-entered
    /// state would have to assume itself), and every caller — `get_instances`,
    /// `is_instance_of`, `get_types`, `realize` — keeps its tableau snapshot
    /// / `task_service.check_instance` fallback, which remains authoritative
    /// for memberships the structural checker cannot prove.
    static INSTANCE_EVAL_STACK: RefCell<HashSet<(String, ClassExpression)>> =
        RefCell::new(HashSet::new());

    /// Current nesting depth of guarded
    /// [`ClassificationService::check_instance_with_datatype_reasoning`]
    /// frames on this thread (guarded by [`INSTANCE_EVAL_STACK`] entries, so
    /// it equals the stack size).
    static INSTANCE_EVAL_DEPTH: Cell<usize> = const { Cell::new(0) };

    /// Whether the [`INSTANCE_EVAL_MAX_DEPTH`] warning was already logged
    /// (warn once per thread, not once per cut).
    static INSTANCE_EVAL_DEPTH_WARNED: Cell<bool> = const { Cell::new(false) };
}

/// RAII entry for [`INSTANCE_EVAL_STACK`] + [`INSTANCE_EVAL_DEPTH`]: inserts
/// the key and bumps the depth on creation and removes / decrements on drop,
/// so every exit path (`Ok`, `Err`, early `return`) keeps the guard balanced —
/// including nested calls.
struct InstanceEvalGuard((String, ClassExpression));

impl InstanceEvalGuard {
    /// Push `key` unless it is already on the stack (cycle → `None`) and
    /// unless the hard depth cap [`INSTANCE_EVAL_MAX_DEPTH`] would be exceeded
    /// (→ `None`, warn once).  The caller returns a conservative `Ok(false)`
    /// in both cases.
    fn enter(key: (String, ClassExpression)) -> Option<Self> {
        if INSTANCE_EVAL_STACK.with(|stack| stack.borrow().contains(&key)) {
            return None;
        }
        let depth = INSTANCE_EVAL_DEPTH.with(Cell::get);
        if depth >= INSTANCE_EVAL_MAX_DEPTH {
            if !INSTANCE_EVAL_DEPTH_WARNED.with(|warned| warned.replace(true)) {
                warn!(
                    "instance evaluation depth cap ({INSTANCE_EVAL_MAX_DEPTH}) exceeded; \
                     cutting the recursion with a conservative false — the tableau \
                     snapshot fallback remains authoritative"
                );
            }
            return None;
        }
        INSTANCE_EVAL_STACK.with(|stack| {
            stack.borrow_mut().insert(key.clone());
        });
        INSTANCE_EVAL_DEPTH.with(|cell| cell.set(depth + 1));
        Some(Self(key))
    }
}

impl Drop for InstanceEvalGuard {
    fn drop(&mut self) {
        INSTANCE_EVAL_STACK.with(|stack| {
            stack.borrow_mut().remove(&self.0);
        });
        INSTANCE_EVAL_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

/// RAII entry for [`DEFINED_EVAL_STACK`]: inserts the pair on creation and
/// removes it on drop so every exit path (including `Err` propagation) keeps
/// the stack balanced.
struct DefinedEvalGuard((String, IRI));

impl DefinedEvalGuard {
    /// Push `key` unless it is already on the stack (cycle → `None`).
    fn enter(key: (String, IRI)) -> Option<Self> {
        if DEFINED_EVAL_STACK.with(|stack| stack.borrow().contains(&key)) {
            return None;
        }
        DEFINED_EVAL_STACK.with(|stack| {
            stack.borrow_mut().insert(key.clone());
        });
        Some(Self(key))
    }
}

impl Drop for DefinedEvalGuard {
    fn drop(&mut self) {
        DEFINED_EVAL_STACK.with(|stack| {
            stack.borrow_mut().remove(&self.0);
        });
    }
}

/// Collect the *named* classes of a (possibly nested) `ObjectIntersectionOf`
/// into `out` (`D ⊑ E ⊓ (F ⊓ G)` yields `E`, `F`, `G`). A bare named class
/// is collected as-is so nested positions resolve through the recursion.
///
/// Only intersection operands are decomposed: union, existential, universal
/// and data-restriction operands are **not** entailed as named subclasses
/// (`D ⊑ E ⊔ F` does not entail `D ⊑ E`), so they are skipped.
fn named_intersection_conjuncts(expr: &ClassExpression, out: &mut Vec<IRI>) {
    match expr {
        ClassExpression::Class(cls) => out.push(cls.iri.clone()),
        ClassExpression::ObjectIntersectionOf(parts) => {
            for part in parts {
                named_intersection_conjuncts(part, out);
            }
        }
        _ => {}
    }
}

/// Service for complex reasoning operations like classification and realisation
#[derive(Debug)]
pub struct ClassificationService {
    task_service: ReasoningTaskService,
    datatype_validator: DatatypeValidator,
    saturation_engine: SaturationEngine,
    performance_config: PerformanceConfig,
    /// Cached classification result, invalidated on ontology change.
    classification_cache: Option<ClassificationResult>,
    /// Enable absorption + dependency-graph (Elgot-style) classification.
    absorption_enabled: bool,
    /// RSS cap from `reasoning.max_memory_mb` (`None`/`Some(0)` = unlimited).
    /// Enforced by a [`crate::performance::MemoryGuard`] owned by the
    /// `classify` loops (see `set_max_memory_mb`).
    max_memory_mb: Option<u64>,
    /// Maximum number of full-tableau checks in the phase-3 fallback.
    max_real_tableau_checks: usize,
    /// Total time budget for full-tableau checks in the phase-3 fallback.
    max_real_tableau_time: std::time::Duration,
}

impl ClassificationService {
    /// Create a new classification service
    #[must_use]
    pub fn new(task_service: ReasoningTaskService) -> Self {
        Self {
            task_service,
            datatype_validator: DatatypeValidator::new(),
            saturation_engine: SaturationEngine::new(SaturationConfig::default()),
            performance_config: PerformanceConfig::from_env(),
            classification_cache: None,
            absorption_enabled: true,
            max_memory_mb: None,
            max_real_tableau_checks: 512,
            max_real_tableau_time: std::time::Duration::from_millis(250),
        }
    }

    /// Create a new classification service with custom saturation config
    #[must_use]
    pub fn with_saturation_config(
        task_service: ReasoningTaskService,
        saturation_config: SaturationConfig,
    ) -> Self {
        Self {
            task_service,
            datatype_validator: DatatypeValidator::new(),
            saturation_engine: SaturationEngine::new(saturation_config),
            performance_config: PerformanceConfig::from_env(),
            classification_cache: None,
            absorption_enabled: true,
            max_memory_mb: None,
            max_real_tableau_checks: 512,
            max_real_tableau_time: std::time::Duration::from_millis(250),
        }
    }

    /// Set the performance configuration
    pub fn set_performance_config(&mut self, config: PerformanceConfig) {
        self.performance_config = config;
    }

    /// Enforce `reasoning.max_memory_mb` in the classification loops
    /// (per-class phase-3 loop + A8 phase boundaries) and forward the cap to
    /// the saturation engine.  `None`/`Some(0)` = unlimited.
    pub fn set_max_memory_mb(&mut self, max_memory_mb: Option<u64>) {
        self.max_memory_mb = max_memory_mb;
        self.saturation_engine.set_max_memory_mb(max_memory_mb);
    }

    /// Enable or disable the absorption + dependency-graph classification path.
    pub fn set_absorption_enabled(&mut self, enabled: bool) {
        self.absorption_enabled = enabled;
    }

    /// Invalidate the cached classification result (ontology changed).
    pub fn invalidate_classification_cache(&mut self) {
        self.classification_cache = None;
    }

    /// Store a classification result produced outside this service (e.g. the
    /// EL-reasoner fast path) so that query fast-paths can reuse it.
    pub fn store_classification(&mut self, mut result: ClassificationResult) {
        // Rebuild the inverse / equivalence indexes if they are empty.
        if result.subsumees.is_empty() {
            result.rebuild_subsumees();
        }
        if result.equivalents.is_empty() {
            for (sub, sups) in &result.hierarchy {
                for sup in sups {
                    if result.hierarchy.get(sup).is_some_and(|s| s.contains(sub)) {
                        result
                            .equivalents
                            .entry(sub.clone())
                            .or_default()
                            .insert(sup.clone());
                        result
                            .equivalents
                            .entry(sup.clone())
                            .or_default()
                            .insert(sub.clone());
                    }
                }
            }
        }
        self.classification_cache = Some(result);
    }

    /// Borrow the cached classification result, if any.
    #[must_use]
    pub fn cached_classification(&self) -> Option<&ClassificationResult> {
        self.classification_cache.as_ref()
    }

    /// Register the ontology's `DatatypeDefinition` axioms with the datatype validator so
    /// literals of user-defined datatypes are checked against their defining ranges.
    pub fn register_datatype_definitions(&mut self, ontology: &Ontology) {
        self.datatype_validator
            .register_ontology_datatypes(ontology);
    }

    /// Perform classification (build class hierarchy)
    pub fn classify(
        &mut self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<ClassificationResult> {
        let start_time = Instant::now();

        info!("Starting classification with saturation-based optimization");

        // Check if we have a cached classification result
        if let Some(cached_result) = &self.classification_cache {
            debug!("Classification result found in service cache");
            statistics.add_reasoning_time(start_time.elapsed());
            return Ok(cached_result.clone());
        }
        if let Some(cached_result) = cache.get_classification_result(ontology) {
            debug!("Classification result found in cache");
            statistics.add_reasoning_time(start_time.elapsed());
            return Ok(cached_result);
        }

        // RSS guard for the loops below.  Owned by `classify` (the loops live
        // here): samples at most once per ~300 ms, and `None`/`Some(0)`
        // (`reasoning.max_memory_mb` unset or 0) never samples at all.
        let mut memory_guard =
            crate::performance::MemoryGuard::from_max_memory_mb(self.max_memory_mb);

        let ontology_guard = read_lock(ontology, "classification: reading ontology for classify")?;

        // Get all named classes from the ontology.  The signature is reused at
        // the tail for the object/data property lists (no second O(A·C) scan).
        let signature = ontology_guard.signature()?;
        let mut classes: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        // Add owl:Thing if not present
        let owl_thing = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::owl_thing().to_url()?.into(),
        });
        if !classes.contains(&owl_thing) {
            classes.push(owl_thing.clone());
        }

        // Process inferred classes from complex axioms
        let inferred_classes = self.discover_inferred_classes(&ontology_guard)?;
        classes.extend(inferred_classes);

        info!("Classifying {} classes", classes.len());

        let named_count = classes.iter().filter(|c| c.is_named_class()).count();

        // === Extract told subsumers from axioms ===
        let phase1_start = Instant::now();
        let mut hierarchy = self.extract_told_subsumers(&classes, &ontology_guard)?;
        info!("(told subsumers) completed in {:?}", phase1_start.elapsed());

        let mut counters = ClassificationCounters {
            told_subsumers: hierarchy.values().map(HashSet::len).sum(),
            ..Default::default()
        };

        // === PHASE 2: Absorption + dependency-graph propagation (A8) ===
        // When absorption is enabled the dependency-graph propagation subsumes
        // the saturation fast path (told closure, equivalents, role-domain /
        // role-range propagation) so we skip the (heavier) saturation engine
        // entirely.  The saturation engine remains the fallback when the
        // absorption path is disabled.
        memory_guard.check()?; // before the A8 absorb + dependency classify
        let phase2_start = Instant::now();
        let mut may_need_tableau: HashSet<ClassExpression> = HashSet::new();
        let mut absorption_opt: Option<crate::core::saturation::AbsorptionResult> = None;
        let mut statuses: HashMap<ClassExpression, SaturationStatus> = HashMap::new();

        if self.absorption_enabled {
            let absorption = absorb(&ontology_guard);
            counters.absorbed_gcis = absorption.absorbed_count;
            counters.remaining_gcis = absorption.remaining_count;
            counters.role_successor_clauses = absorption.role_successor_count();
            counters.role_domain_clauses = absorption.role_domains.values().map(Vec::len).sum();
            counters.role_range_clauses = absorption.role_ranges.values().map(Vec::len).sum();

            let classifier = DependencyClassifier;
            let dep = classifier.classify(&classes, &absorption, &hierarchy);
            counters.dependency_sccs = dep.scc_count;
            counters.propagation_rounds = dep.rounds;
            counters.dependency_edges = dep
                .derivations
                .values()
                .map(|d| d.subsumers.len() + d.existentials.len())
                .sum();
            may_need_tableau.clone_from(&dep.may_need_tableau);

            // Build the hierarchy directly from the derivations (they already
            // include the told subsumers plus the transitive closure), avoiding
            // a redundant told+merge pass.  The derivations are *moved* into
            // the hierarchy (not cloned), so the O(N·D) subsumer sets are
            // handed off once.
            let mut new_hierarchy: HashMap<ClassExpression, HashSet<ClassExpression>> =
                HashMap::with_capacity(classes.len());
            for c in &classes {
                new_hierarchy.entry(c.clone()).or_default();
            }
            for (concept, deriv) in dep.derivations {
                new_hierarchy.insert(concept, deriv.subsumers);
            }
            hierarchy = new_hierarchy;
            absorption_opt = Some(absorption);
        } else {
            // Pre-A8 fallback: index-based saturation.
            let saturation_result = self.saturation_engine.saturate_ontology(&ontology_guard)?;
            counters.saturation_concepts = saturation_result.statistics.concepts_processed;
            counters.saturation_complete = saturation_result.statistics.concepts_complete;
            counters.saturation_tableau_required =
                saturation_result.statistics.concepts_requiring_tableau;
            for (concept, subsumers) in &saturation_result.subsumptions {
                hierarchy
                    .entry(concept.clone())
                    .or_insert_with(HashSet::new)
                    .extend(subsumers.iter().cloned());
            }
            for class in &classes {
                if let Some(node) = saturation_result.get_node(class) {
                    statuses.insert(class.clone(), node.status);
                }
            }
        }

        info!(
            "Phase 2 (classification) completed in {:?} - {} classes avoided the tableau",
            phase2_start.elapsed(),
            named_count.saturating_sub(may_need_tableau.len())
        );

        // === PHASE 3: Tableau expansion for genuinely non-deterministic classes ===
        memory_guard.check()?; // after A8 / saturation, before the pairwise loop
        let phase3_start = Instant::now();
        let mut tableau_checks = 0usize;
        let mut real_tableau_checks = 0usize;
        let mut classes_needing_tableau: HashSet<ClassExpression> = HashSet::new();
        let mut real_tableau_time = std::time::Duration::ZERO;

        // A class needs the tableau fallback when the absorption path marks it
        // (non-deterministic construct in its closure, or a non-absorbed GCI
        // touching it) or when saturation flagged it.
        let needs_tableau = |class: &ClassExpression, may: &HashSet<ClassExpression>| -> bool {
            if let Some(&status) = statuses.get(class)
                && (status == SaturationStatus::RequiresFullTableau
                    || status == SaturationStatus::NonDeterministic)
            {
                return true;
            }
            may.contains(class)
        };

        // Process pairs directly without intermediate Vec to avoid O(N²) memory.
        // Named heads of non-absorbed GCIs are always plausible candidates for
        // a non-deterministic class (e.g. `≥2R.C ⊑ D` can add `D`).
        let owl_thing_candidate = ClassExpression::Class(crate::ontology::Class::new(
            crate::ontology::IRI::owl_thing(),
        ));

        for subclass in &classes {
            // Per-class memory check (interval-gated; aborts a runaway phase 3).
            memory_guard.check()?;
            if !needs_tableau(subclass, &may_need_tableau) {
                continue;
            }
            classes_needing_tableau.insert(subclass.clone());

            // Candidate superclasses for the real-tableau probes.  A named
            // class `T` can *newly* subsume `subclass` (i.e. beyond what
            // absorption + the dependency-graph propagation already derived)
            // only if some remaining GCI `C ⊑ E` can fire on `subclass` and
            // mention `T` inside `E` — every other route is already handled
            // deterministically (told subsumers, role-successor / domain /
            // range / conjunction absorption, and the union-decomposition
            // rule).  So we collect `T` from:
            //   (a) named atoms of `E` (and their named subsumers) for each
            //       remaining GCI that is *not consumed* (`E` already in the
            //       deterministic hierarchy) and whose subclass side overlaps
            //       the class's derivation (conservative plausibility check),
            //   (b) named atoms of non-union complex subsumers of the class
            //       and their named subsumers (union subsumers are fully
            //       decided by the union-decomposition rule below),
            //   (c) `owl:Thing`.
            // Restricting the (expensive) full-tableau probes to these
            // genuinely plausible positive candidates keeps the fallback cheap
            // without losing completeness for the absorbed fragment.
            let mut candidates: HashSet<ClassExpression> = HashSet::new();
            candidates.insert(owl_thing_candidate.clone());

            // Visible named classes of `subclass` (its subsumer atoms plus
            // itself): used by the plausibility overlap test below.
            let mut visible: HashSet<ClassExpression> = HashSet::new();
            visible.insert(subclass.clone());
            if let Some(sups) = hierarchy.get(subclass) {
                for sup in sups {
                    collect_named_classes(sup, &mut visible);
                }
            }

            if let Some(absorption) = &absorption_opt {
                for gci in &absorption.remaining_gcis {
                    // `E` already derived deterministically ⇒ the GCI is
                    // consumed for this class and can add nothing new.
                    if hierarchy
                        .get(subclass)
                        .is_some_and(|sups| sups.contains(&gci.superclass))
                    {
                        continue;
                    }
                    // Plausible firing: some named atom of the GCI's subclass
                    // side is visible from this class's derivation.  When the
                    // subclass side has no named atoms (e.g. datatype-only)
                    // we conservatively assume it can fire.
                    let mut c_atoms: HashSet<ClassExpression> = HashSet::new();
                    collect_named_classes(&gci.subclass, &mut c_atoms);
                    let plausible =
                        c_atoms.is_empty() || c_atoms.iter().any(|a| visible.contains(a));
                    if !plausible {
                        continue;
                    }
                    let mut e_atoms: HashSet<ClassExpression> = HashSet::new();
                    collect_named_classes(&gci.superclass, &mut e_atoms);
                    for atom in e_atoms {
                        candidates.insert(atom.clone());
                        if let Some(atom_sups) = hierarchy.get(&atom) {
                            candidates
                                .extend(atom_sups.iter().filter(|c| c.is_named_class()).cloned());
                        }
                    }
                }
            }

            if let Some(sups) = hierarchy.get(subclass) {
                for sup in sups {
                    if matches!(sup, ClassExpression::ObjectUnionOf(_)) {
                        continue; // union-decomposition decides these
                    }
                    let mut atoms: HashSet<ClassExpression> = HashSet::new();
                    collect_named_classes(sup, &mut atoms);
                    for atom in atoms {
                        candidates.insert(atom.clone());
                        if let Some(atom_sups) = hierarchy.get(&atom) {
                            candidates
                                .extend(atom_sups.iter().filter(|c| c.is_named_class()).cloned());
                        }
                    }
                }
            }

            // Union subsumers of this class, used by the union-decomposition
            // rule to discharge disjunction without the tableau.
            let union_subsumers: Vec<Vec<ClassExpression>> = hierarchy
                .get(subclass)
                .map(|sups| {
                    sups.iter()
                        .filter_map(|s| {
                            if let ClassExpression::ObjectUnionOf(arms) = s {
                                Some(arms.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            for superclass in &classes {
                if subclass == superclass {
                    continue;
                }
                if hierarchy
                    .get(subclass)
                    .is_some_and(|sups| sups.contains(superclass))
                {
                    continue;
                }
                // Cheap deterministic probe first (syntactic DFS over axioms).
                // Run for every pair (preserving pre-A8 behaviour); it is cheap
                // because only genuinely non-deterministic classes reach here.
                if self.check_subsumption_from_axioms(subclass, superclass, &ontology_guard)? {
                    hierarchy
                        .entry(subclass.clone())
                        .or_insert_with(HashSet::new)
                        .insert(superclass.clone());
                } else if !union_subsumers.is_empty()
                    && union_subsumers.iter().any(|arms| {
                        arms.iter().all(|a| {
                            *a == *superclass
                                || hierarchy
                                    .get(a)
                                    .is_some_and(|sups| sups.contains(superclass))
                        })
                    })
                {
                    // Union decomposition (sound, no tableau): if `S ⊑ (A₁ ⊔ … ⊔ Aₙ)`
                    // and every arm `Aᵢ ⊑ T`, then `S ⊑ T`.  This discharges the
                    // common disjunction pattern without a full tableau.
                    hierarchy
                        .entry(subclass.clone())
                        .or_insert_with(HashSet::new)
                        .insert(superclass.clone());
                } else if real_tableau_checks < self.max_real_tableau_checks
                    && real_tableau_time < self.max_real_tableau_time
                    && candidates.contains(superclass)
                {
                    // Full tableau probe for the genuinely non-deterministic
                    // remainder (cardinalities, nominals, nested disjunctions).
                    // Bounded by both a count cap and a total-time budget so a
                    // slow tableau degrades gracefully instead of dominating
                    // the classification.
                    let probe_start = Instant::now();
                    let probe = self.task_service.check_subsumption_expressions(
                        subclass, superclass, ontology, statistics, cache,
                    )?;
                    real_tableau_time += probe_start.elapsed();
                    if probe {
                        hierarchy
                            .entry(subclass.clone())
                            .or_insert_with(HashSet::new)
                            .insert(superclass.clone());
                    }
                    real_tableau_checks += 1;
                }
                tableau_checks += 1;
                if tableau_checks.is_multiple_of(1000) {
                    debug!("Tableau checks progress: {tableau_checks}");
                    // Per-iteration memory check for long pairwise runs.
                    memory_guard.check()?;
                }
            }
        }

        counters.tableau_checks = tableau_checks;
        counters.real_tableau_checks = real_tableau_checks;
        counters.classes_needing_tableau = classes_needing_tableau.len();
        counters.classes_avoiding_tableau =
            named_count.saturating_sub(classes_needing_tableau.len());

        info!(
            "Phase 3 (tableau expansion) completed in {:?} with {} checks ({} real tableaus)",
            phase3_start.elapsed(),
            tableau_checks,
            real_tableau_checks
        );

        // Extract ontology IRI before dropping the read lock
        let ontology_iri = ontology_guard
            .id
            .ontology_iri
            .as_ref()
            .map(std::string::ToString::to_string);

        // Collect object and data property full IRIs from the ontology signature
        // (reused from the top of `classify` — no second O(A·C) signature scan).
        let object_properties: Vec<String> = {
            let mut props: Vec<String> = signature
                .object_properties
                .iter()
                .map(|p| p.iri.to_string())
                .collect();
            props.sort();
            props.dedup();
            props
        };
        let data_properties: Vec<String> = {
            let mut props: Vec<String> = signature
                .data_properties
                .iter()
                .map(|p| p.iri.to_string())
                .collect();
            props.sort();
            props.dedup();
            props
        };

        // Drop the read lock before creating result
        drop(ontology_guard);

        let mut result = ClassificationResult::new_with_iri(hierarchy, ontology_iri);
        result.object_properties = object_properties;
        result.data_properties = data_properties;
        result.counters = counters;

        // Equivalence map: classes mutually subsuming each other.
        for (sub, sups) in &result.hierarchy {
            for sup in sups {
                if result.hierarchy.get(sup).is_some_and(|s| s.contains(sub)) {
                    result
                        .equivalents
                        .entry(sub.clone())
                        .or_default()
                        .insert(sup.clone());
                    result
                        .equivalents
                        .entry(sup.clone())
                        .or_default()
                        .insert(sub.clone());
                }
            }
        }

        // Unsatisfiable classes: derived subsumer set contains owl:Nothing.
        let owl_nothing = ClassExpression::Class(crate::ontology::Class::new(
            crate::ontology::IRI::owl_nothing(),
        ));
        for (sub, sups) in &result.hierarchy {
            if sups.contains(&owl_nothing) {
                result.unsat.insert(sub.clone());
            }
        }

        result.rebuild_subsumees();

        // Cache the result on the service and in the cache manager
        self.classification_cache = Some(result.clone());
        cache.store_classification_result(ontology, result.clone());

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!(
            "Classification completed in {:?} ({} classes avoided the tableau)",
            reasoning_time, result.counters.classes_avoiding_tableau
        );
        Ok(result)
    }

    /// Extract told subsumers directly from axioms
    fn extract_told_subsumers(
        &self,
        classes: &[ClassExpression],
        ontology: &Ontology,
    ) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>> {
        let mut hierarchy = HashMap::new();

        // O(1) membership instead of a per-axiom linear scan of `classes`.
        let class_set: HashSet<&ClassExpression> = classes.iter().collect();

        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom)
                    // Direct told subsumption
                    if class_set.contains(&subclass_axiom.subclass) => {
                        let entry = hierarchy
                            .entry(subclass_axiom.subclass.clone())
                            .or_insert_with(HashSet::new);
                        entry.insert(subclass_axiom.superclass.clone());
                    }
                crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) => {
                    // Equivalent classes are mutual subsumers
                    for class1 in &equiv_axiom.classes {
                        if class_set.contains(class1) {
                            let entry =
                                hierarchy.entry(class1.clone()).or_insert_with(HashSet::new);
                            for class2 in &equiv_axiom.classes {
                                if class1 != class2 {
                                    entry.insert(class2.clone());
                                }
                            }
                        }
                    }
                }
                crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) => {
                    // Disjoint union members are subclasses of the union
                    for disjoint_class in &disjoint_union.disjoint_classes {
                        if class_set.contains(disjoint_class) {
                            let entry = hierarchy
                                .entry(disjoint_class.clone())
                                .or_insert_with(HashSet::new);
                            entry.insert(disjoint_union.class.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // Ensure all classes have an entry (even if empty)
        for class in classes {
            hierarchy.entry(class.clone()).or_insert_with(HashSet::new);
        }

        Ok(hierarchy)
    }

    /// Classify object properties.
    ///
    /// Answers from the shared [`property_hierarchy::ObjectPropertyHierarchy`]
    /// closure (transitive `SubObjectPropertyOf`, equivalence reachability,
    /// inverse pairs, `top`/`bottom`) instead of pairwise asserted-edge
    /// checks, matching [`crate::core::reasoner::Reasoner::classify_object_properties`].
    pub fn classify_object_properties(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting object property classification");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for object property classification",
        )?;

        let closure = crate::core::reasoner::property_hierarchy::ObjectPropertyHierarchy::compute(
            &ontology_guard,
        );

        // Get all object properties from the ontology
        let signature = ontology_guard.signature()?;
        let properties: Vec<ObjectPropertyExpression> = signature
            .object_properties
            .iter()
            .map(|p| ObjectPropertyExpression::ObjectProperty(p.clone()))
            .collect();

        let mut hierarchy = HashMap::new();

        info!("Classifying {} object properties", properties.len());

        for property in &properties {
            hierarchy.insert(property.clone(), closure.supers(property));
        }

        let result = PropertyClassificationResult::new_object_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Object property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Classify data properties (see [`Self::classify_object_properties`]).
    pub fn classify_data_properties(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<PropertyClassificationResult> {
        let start_time = Instant::now();

        info!("Starting data property classification");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for data property classification",
        )?;

        let closure = crate::core::reasoner::property_hierarchy::DataPropertyHierarchy::compute(
            &ontology_guard,
        );

        // Get all data properties from the ontology
        let signature = ontology_guard.signature()?;
        let properties: Vec<DataPropertyExpression> = signature
            .data_properties
            .iter()
            .map(|p| DataPropertyExpression::DataProperty(p.clone()))
            .collect();

        let mut hierarchy = HashMap::new();

        info!("Classifying {} data properties", properties.len());

        for property in &properties {
            hierarchy.insert(property.clone(), closure.supers(property));
        }

        let result = PropertyClassificationResult::new_data_properties(hierarchy);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Data property classification completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Perform realisation (find most specific classes for individuals)
    pub fn realize(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<RealisationResult> {
        let start_time = Instant::now();

        info!("Starting realisation");

        // Check if we have a cached realisation result
        if let Some(cached_result) = cache.get_realisation_result(ontology) {
            debug!("Realisation result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard =
            read_lock(ontology, "classification: reading ontology for realisation")?;

        // Get all named individuals and classes
        let individuals: Vec<Individual> = ontology_guard.signature()?.individuals.clone();

        let classes: Vec<ClassExpression> = ontology_guard
            .signature()?
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut realisation = HashMap::new();

        info!(
            "Realizing {} individuals against {} classes",
            individuals.len(),
            classes.len()
        );

        // ── Build asserted named-class hierarchy indices ──────────────────────────
        // superclass_map: class IRI → Vec<direct named superclass IRIs>
        // Derived from SubClassOf(named, named) AND EquivalentClasses(named, named) axioms.
        let mut superclass_map: HashMap<IRI, Vec<IRI>> = HashMap::new();
        for axiom in ontology_guard.axioms() {
            match axiom {
                crate::ontology::axioms::Axiom::SubClassOf(ax)
                    if matches!(&ax.subclass, ClassExpression::Class(_))
                        && matches!(&ax.superclass, ClassExpression::Class(_)) =>
                {
                    if let (ClassExpression::Class(sub), ClassExpression::Class(sup)) =
                        (&ax.subclass, &ax.superclass)
                    {
                        superclass_map
                            .entry(sub.iri.clone())
                            .or_default()
                            .push(sup.iri.clone());
                    }
                }
                crate::ontology::axioms::Axiom::EquivalentClasses(ax) => {
                    // Collect all named classes in this equivalence group
                    let named: Vec<IRI> = ax
                        .classes
                        .iter()
                        .filter_map(|c| {
                            if let ClassExpression::Class(cls) = c {
                                Some(cls.iri.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    // Add bidirectional edges so BFS treats them as mutually reachable
                    for i in 0..named.len() {
                        for j in 0..named.len() {
                            if i != j {
                                superclass_map
                                    .entry(named[i].clone())
                                    .or_default()
                                    .push(named[j].clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Build a fast lookup: IRI → ClassExpression::Class for named classes
        let class_by_iri: HashMap<IRI, ClassExpression> = classes
            .iter()
            .filter_map(|ce| {
                if let ClassExpression::Class(c) = ce {
                    Some((c.iri.clone(), ce.clone()))
                } else {
                    None
                }
            })
            .collect();

        // ── Defined-class candidates ────────────────────────────────────────────
        // Named operands of `EquivalentClasses` groups that contain a *complex*
        // operand (`A ≡ {a}`, `C ≡ D[int ≥ 18]`, …).  Membership in such a class
        // never appears as a `ClassAssertion`, yet it is a genuine type — the
        // individual that occurs inside the definition (e.g. the nominal `a` of
        // `A ≡ {a}`) must be realised with it.  Same bounded candidate policy as
        // `Reasoner::get_types` (core.rs): only *defined* classes are scanned so
        // the cost stays O(#defined × #individuals), not O(#classes × #individuals).
        let mut defined_candidates: Vec<ClassExpression> = Vec::new();
        let mut seen_candidates: HashSet<IRI> = HashSet::new();
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::EquivalentClasses(equivalent) = axiom
                && equivalent
                    .classes
                    .iter()
                    .any(|operand| !matches!(operand, ClassExpression::Class(_)))
            {
                for operand in &equivalent.classes {
                    if let ClassExpression::Class(cls) = operand
                        && seen_candidates.insert(cls.iri.clone())
                    {
                        defined_candidates.push(ClassExpression::Class(cls.clone()));
                    }
                }
            }
        }

        // Classes with complex (non-named) expressions — always need structural or tableau check
        let complex_classes: Vec<&ClassExpression> = classes
            .iter()
            .filter(|ce| !matches!(ce, ClassExpression::Class(_)))
            .collect();

        for individual in &individuals {
            let mut instance_classes: HashSet<ClassExpression> = HashSet::new();

            // ── Step 1: collect explicitly asserted named types ──────────────────
            // Identity is compared as a whole `Individual`, so **anonymous**
            // individuals (`_:x`) get their own asserted types instead of
            // collapsing onto the shared "anonymous" fallback key (or, with an
            // `iri()`-based comparison, onto each other: `None == None`).
            let mut bfs: VecDeque<IRI> = VecDeque::new();
            let mut visited_iri: HashSet<IRI> = HashSet::new();

            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::ClassAssertion(ca) = axiom
                    && ca.individual == *individual
                {
                    match &ca.class {
                        ClassExpression::Class(cls) => {
                            if visited_iri.insert(cls.iri.clone()) {
                                bfs.push_back(cls.iri.clone());
                                instance_classes.insert(ClassExpression::Class(cls.clone()));
                            }
                        }
                        other => {
                            // Complex asserted type — add directly
                            instance_classes.insert(other.clone());
                        }
                    }
                }
            }

            // ── Step 1b: memberships from *defined* classes ─────────────────────
            // `A ≡ {a}` (or any complex definition) makes the named class a type
            // of the individuals inside the definition without a ClassAssertion.
            // Feed hits into the Step 2 BFS so the asserted hierarchy still
            // closes upward from them.  (Without this, individuals that occur
            // only inside nominals — surfaced by the `signature()` fix for
            // CR-008 MAJOR #2 — would be realised with zero types and the OWL
            // API trait surface would miss them.)
            for candidate in &defined_candidates {
                if instance_classes.contains(candidate) {
                    continue;
                }
                if let Ok(true) = self.check_instance_with_datatype_reasoning(
                    individual,
                    candidate,
                    &ontology_guard,
                ) {
                    instance_classes.insert(candidate.clone());
                    if let ClassExpression::Class(cls) = candidate
                        && visited_iri.insert(cls.iri.clone())
                    {
                        bfs.push_back(cls.iri.clone());
                    }
                }
            }

            // ── Step 2: close upward through asserted SubClassOf hierarchy ───────
            // Key property: if a ∈ D and D ⊑ C then a ∈ C (no tableau needed).
            while let Some(iri) = bfs.pop_front() {
                if let Some(supers) = superclass_map.get(&iri) {
                    for sup_iri in supers {
                        if visited_iri.insert(sup_iri.clone()) {
                            bfs.push_back(sup_iri.clone());
                            if let Some(ce) = class_by_iri.get(sup_iri) {
                                instance_classes.insert(ce.clone());
                            } else {
                                // Class is in the hierarchy but not in the signature (e.g. owl:Thing)
                                instance_classes.insert(ClassExpression::Class(
                                    crate::ontology::Class {
                                        iri: sup_iri.clone(),
                                    },
                                ));
                            }
                        }
                    }
                }
            }

            // ── Step 3: complex class expressions — try structural check, then tableau
            for class in &complex_classes {
                if instance_classes.contains(*class) {
                    continue;
                }
                if let Ok(true) =
                    self.check_instance_with_datatype_reasoning(individual, class, &ontology_guard)
                {
                    instance_classes.insert((*class).clone());
                } else if self
                    .task_service
                    .check_instance(individual, class, ontology, statistics, cache)?
                {
                    instance_classes.insert((*class).clone());
                }
            }

            // Named class membership not reachable via the hierarchy (SubClassOf /
            // EquivalentClasses BFS above) would require a precomputed TBox
            // classification pass.  We omit it here to keep realisation tractable.

            realisation.insert(individual.clone(), instance_classes);
        }

        let result = RealisationResult::new(realisation);

        // Cache the result
        cache.store_realisation_result(ontology, result.clone());

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Realisation completed in {reasoning_time:?}");
        Ok(result)
    }

    /// Get all unsatisfiable classes (equivalent to owl:Nothing)
    pub fn get_unsatisfiable_classes(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<Vec<ClassExpression>> {
        let start_time = Instant::now();

        info!("Finding unsatisfiable classes");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for unsatisfiable classes",
        )?;

        // Get all named classes from the ontology
        let signature = ontology_guard.signature()?;
        let classes: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .collect();

        let mut unsatisfiable_classes = Vec::new();

        for class in &classes {
            if let ClassExpression::Class(cls) = class
                && !self.task_service.check_satisfiability(
                    &cls.iri.to_string(),
                    ontology,
                    statistics,
                    cache,
                )?
            {
                unsatisfiable_classes.push(class.clone());
            }
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!(
            "Found {} unsatisfiable classes in {reasoning_time:?}",
            unsatisfiable_classes.len()
        );
        Ok(unsatisfiable_classes)
    }

    /// Get all superclasses of a class expression
    ///
    /// # Panics
    ///
    /// Panics if the ontology signature cannot be extracted.
    pub fn get_superclasses(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        _direct: bool,
        cache: &mut CacheManager,
    ) -> Result<Vec<ClassExpression>> {
        // Fast path: named class already classified → answer from the cache.
        if let Some(classif) = &self.classification_cache
            && let Some(sups) = classif.hierarchy.get(concept)
        {
            let out: Vec<ClassExpression> =
                sups.iter().filter(|s| *s != concept).cloned().collect();
            return Ok(out);
        }

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_superclasses",
        )?;
        let mut superclasses = Vec::new();

        // Get all classes from the signature
        for class in &ontology_guard
            .signature()
            .expect("Failed to extract ontology signature")
            .classes
        {
            let class_expr = ClassExpression::Class(class.clone());
            if self.task_service.check_subsumption_expressions(
                concept,
                &class_expr,
                ontology,
                statistics,
                cache,
            )? && concept != &class_expr
            {
                superclasses.push(class_expr);
            }
        }

        Ok(superclasses)
    }

    /// Get all subclasses of a class expression
    pub fn get_subclasses(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        _direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        // Fast path: named class already classified → O(1) via subsumees index.
        if let ClassExpression::Class(_) = concept
            && let Some(classif) = &self.classification_cache
            && let Some(subs) = classif.subsumees.get(concept)
        {
            return Ok(subs.iter().cloned().collect());
        }

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_subclasses",
        )?;
        let mut subclasses = Vec::new();

        if let ClassExpression::Class(target_class) = concept {
            debug!("Looking for subclasses of: {}", target_class.iri.as_str());

            // Look for SubClassOf axioms in the ontology
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                    // Check if the superclass matches our target
                    if let ClassExpression::Class(super_class) = &subclass_axiom.superclass
                        && target_class.iri.as_str() == super_class.iri.as_str()
                    {
                        subclasses.push(subclass_axiom.subclass.clone());
                    }
                }
            }
        }

        Ok(subclasses)
    }

    /// Get all equivalent classes of a class expression
    pub fn get_equivalent_classes(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<Vec<ClassExpression>> {
        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_equivalent_classes",
        )?;
        let mut equivalent_classes = Vec::new();

        // Special handling for union queries - check DisjointUnion axioms
        if let ClassExpression::ObjectUnionOf(union_classes) = concept {
            debug!(
                "Processing union query with {} classes",
                union_classes.len()
            );
            // Find any DisjointUnion axiom that matches this union
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                    // Check if the union in the query matches the disjoint classes in this axiom
                    if self.union_matches_disjoint_classes(
                        union_classes,
                        &disjoint_union.disjoint_classes,
                    ) {
                        equivalent_classes.push(disjoint_union.class.clone());
                    }
                }
            }
        }

        // Fast path: named class already classified → answer from `equivalents`.
        if let ClassExpression::Class(_) = concept
            && let Some(classif) = &self.classification_cache
            && let Some(eqs) = classif.equivalents.get(concept)
        {
            equivalent_classes.extend(eqs.iter().cloned());
            return Ok(equivalent_classes);
        }

        // General case: Check all classes from the signature for bidirectional subsumption
        for class in &ontology_guard.signature()?.classes {
            let class_expr = ClassExpression::Class(class.clone());
            if concept != &class_expr {
                let subsumes_1_2 = self.task_service.check_subsumption_expressions(
                    concept,
                    &class_expr,
                    ontology,
                    statistics,
                    cache,
                )?;
                let subsumes_2_1 = self.task_service.check_subsumption_expressions(
                    &class_expr,
                    concept,
                    ontology,
                    statistics,
                    cache,
                )?;
                if subsumes_1_2 && subsumes_2_1 {
                    equivalent_classes.push(class_expr);
                }
            }
        }

        Ok(equivalent_classes)
    }

    /// Get all instances of a class expression
    pub fn get_instances(
        &self,
        concept: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        _direct: bool,
        cache: &mut CacheManager,
    ) -> Result<Vec<Individual>> {
        let individuals = {
            let ontology_guard = read_lock(
                ontology,
                "classification: reading ontology for get_instances individuals",
            )?;
            // Get all individuals from the signature.  Nominal-only individuals
            // (`a` in `A ≡ ObjectOneOf({a})`, or in `ObjectHasValue(r.{a})`) are
            // included by `Ontology::signature()` itself since the CR-008
            // MAJOR #2 root fix, so the temporary `CollectIndividuals` walker
            // that used to be merged in here has been removed: `signature()` is
            // now a superset of what that walk collected (it also covers SWRL
            // atoms, which the walker does not traverse).

            ontology_guard.signature()?.individuals.clone()
        }; // Drop the read lock here

        let mut instances = Vec::new();
        let mut determined: HashSet<Individual> = HashSet::new();

        // Fast path: named class — reuse the classification result to answer
        // instances whose asserted types are subclasses of `concept` without
        // running a tableau per individual.
        if let ClassExpression::Class(_) = concept
            && let Some(classif) = &self.classification_cache
        {
            let subclasses: HashSet<ClassExpression> =
                classif.subsumees.get(concept).cloned().unwrap_or_default();
            let asserted_types: HashMap<Individual, Vec<ClassExpression>> = {
                let ontology_guard = read_lock(
                    ontology,
                    "classification: reading ontology for get_instances asserted types",
                )?;
                let mut types: HashMap<Individual, Vec<ClassExpression>> = HashMap::new();
                for axiom in ontology_guard.axioms() {
                    if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                        types
                            .entry(assertion.individual.clone())
                            .or_default()
                            .push(assertion.class.clone());
                    }
                }
                types
            };
            for individual in &individuals {
                let is_instance = asserted_types.get(individual).is_some_and(|types| {
                    types
                        .iter()
                        .any(|t| *t == *concept || subclasses.contains(t))
                });
                if is_instance {
                    instances.push(individual.clone());
                    determined.insert(individual.clone());
                }
            }
        }

        for individual in &individuals {
            if determined.contains(individual) {
                continue;
            }

            // First, try datatype reasoning for certain class expressions
            let mut is_instance = false;

            // Try direct datatype reasoning first
            {
                let ontology_guard = read_lock(
                    ontology,
                    "classification: reading ontology for get_instances datatype reasoning",
                )?;
                if let Ok(true) = self.check_instance_with_datatype_reasoning(
                    individual,
                    concept,
                    &ontology_guard,
                ) {
                    is_instance = true;
                }
            }

            // If not determined by datatype reasoning, use tableau
            if !is_instance {
                is_instance = self
                    .task_service
                    .check_instance(individual, concept, ontology, statistics, cache)?;
            }

            if is_instance {
                instances.push(individual.clone());
            }
        }

        Ok(instances)
    }

    // Private helper methods

    /// Discover inferred classes from complex axioms (equivalent classes, union classes, etc.)
    fn discover_inferred_classes(&self, _ontology: &Ontology) -> Result<Vec<ClassExpression>> {
        let inferred_classes = Vec::new();

        // For now, let's simplify this to avoid complex pattern matching issues
        // The main classification should handle the basic relationships

        Ok(inferred_classes)
    }

    /// Check subsumption using axioms from the ontology
    fn check_subsumption_from_axioms(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        let mut visited = HashSet::new();
        self.check_subsumption_from_axioms_with_visited(
            subclass,
            superclass,
            ontology,
            &mut visited,
        )
    }

    fn check_subsumption_from_axioms_with_visited(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        visited: &mut HashSet<(ClassExpression, ClassExpression)>,
    ) -> Result<bool> {
        // Prevent infinite recursion
        let key = (subclass.clone(), superclass.clone());
        if visited.contains(&key) {
            return Ok(false);
        }
        visited.insert(key);

        // First check for direct SubClassOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom
                && subclass_axiom.subclass == *subclass
                && subclass_axiom.superclass == *superclass
            {
                return Ok(true);
            }
        }

        // Check for equivalent classes
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                let classes = &equiv_axiom.classes;
                if classes.contains(subclass) && classes.contains(superclass) {
                    return Ok(true);
                }

                // If subclass is equivalent to something that is a subclass of superclass
                if classes.contains(subclass) {
                    for equiv_class in classes {
                        if equiv_class != subclass
                            && self.check_subsumption_from_axioms_with_visited(
                                equiv_class,
                                superclass,
                                ontology,
                                visited,
                            )?
                        {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Check for DisjointUnion relationships
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                // If subclass is one of the disjoint classes, it's a subclass of the union class
                if disjoint_union.disjoint_classes.contains(subclass) {
                    if disjoint_union.class == *superclass {
                        return Ok(true);
                    }
                    // Also check if the union class is a subclass of superclass
                    if self.check_subsumption_from_axioms_with_visited(
                        &disjoint_union.class,
                        superclass,
                        ontology,
                        visited,
                    )? {
                        return Ok(true);
                    }
                }

                // If subclass is the union class and superclass is owl:Thing or a superclass of the union
                if disjoint_union.class == *subclass
                    && let ClassExpression::Class(super_cls) = superclass
                    && super_cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing"
                {
                    return Ok(true);
                }
            }
        }

        // Check for transitive subsumption
        self.check_transitive_subsumption_with_visited(subclass, superclass, ontology, visited)
    }

    /// Check transitive subsumption relationships
    fn check_transitive_subsumption_with_visited(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &Ontology,
        _visited: &mut HashSet<(ClassExpression, ClassExpression)>,
    ) -> Result<bool> {
        // Recursion is already guarded by the caller's `visited` set; the
        // top-level pair is inserted by `check_subsumption_from_axioms_with_visited`
        // *before* this is reached, so we must not bail on it here (that would
        // make the direct `A ⊑ owl:Thing` / transitive cases always report
        // false and force an expensive tableau probe).  Cycle protection for
        // the DFS below is handled by `local_visited`.

        // Use a simple depth-first search to find transitive relationships
        let mut local_visited = HashSet::new();
        let mut stack = vec![subclass.clone()];

        while let Some(current) = stack.pop() {
            if local_visited.contains(&current) {
                continue;
            }
            local_visited.insert(current.clone());

            // Check direct subsumption
            for axiom in ontology.axioms() {
                if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom
                    && subclass_axiom.subclass == current
                {
                    if subclass_axiom.superclass == *superclass {
                        return Ok(true);
                    }
                    // Add to stack for further exploration
                    if !local_visited.contains(&subclass_axiom.superclass) {
                        stack.push(subclass_axiom.superclass.clone());
                    }
                }
            }
        }

        // Check if subclass is ultimately a subclass of owl:Thing
        if let ClassExpression::Class(super_class) = superclass
            && super_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing"
        {
            // Everything is a subclass of owl:Thing except owl:Nothing
            if let ClassExpression::Class(sub_class) = subclass {
                return Ok(sub_class.iri.as_str() != "http://www.w3.org/2002/07/owl#Nothing");
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if a union expression matches the disjoint classes in a `DisjointUnion` axiom
    fn union_matches_disjoint_classes(
        &self,
        union_classes: &[ClassExpression],
        disjoint_classes: &[ClassExpression],
    ) -> bool {
        // Recursively extract all classes from the union (handling nested unions)
        let mut union_iris = HashSet::new();
        for expr in union_classes {
            self.extract_all_union_classes(expr, &mut union_iris);
        }

        let disjoint_iris: HashSet<String> = disjoint_classes
            .iter()
            .filter_map(|expr| {
                if let ClassExpression::Class(class) = expr {
                    Some(class.iri.to_string())
                } else {
                    None
                }
            })
            .collect();

        // The union matches if it contains exactly the same classes as the disjoint union
        union_iris == disjoint_iris && !union_iris.is_empty()
    }

    /// Recursively extract all class IRIs from a union expression (handling nested unions)
    fn extract_all_union_classes(&self, expr: &ClassExpression, result: &mut HashSet<String>) {
        self.extract_all_union_classes_with_depth(expr, result, 0);
    }

    /// Maximum recursion depth for union extraction to prevent stack overflow
    const MAX_UNION_EXTRACTION_DEPTH: usize = 500;

    fn extract_all_union_classes_with_depth(
        &self,
        expr: &ClassExpression,
        result: &mut HashSet<String>,
        depth: usize,
    ) {
        // Prevent stack overflow on deeply nested unions
        if depth > Self::MAX_UNION_EXTRACTION_DEPTH {
            return;
        }

        match expr {
            ClassExpression::Class(class) => {
                result.insert(class.iri.to_string());
            }
            ClassExpression::ObjectUnionOf(union_classes) => {
                for nested_expr in union_classes {
                    self.extract_all_union_classes_with_depth(nested_expr, result, depth + 1);
                }
            }
            _ => {
                // For other expressions, we don't extract classes
            }
        }
    }

    /// Check if one object property is a subproperty of another
    ///
    /// Superseded by [`crate::core::reasoner::property_hierarchy::ObjectPropertyHierarchy`]
    /// for hierarchy queries; kept for the asserted-edge chain helpers below.
    #[allow(dead_code)]
    fn is_subproperty_of(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for direct SubObjectPropertyOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom
                && &sub_axiom.sub_property == subproperty
                && &sub_axiom.super_property == superproperty
            {
                return Ok(true);
            }
        }

        // Add sophisticated property chain reasoning
        self.check_property_chain_entailment(subproperty, superproperty, ontology)
    }

    /// Check property chain entailment for complex property relationships
    #[allow(dead_code)]
    fn check_property_chain_entailment(
        &self,
        subproperty: &ObjectPropertyExpression,
        superproperty: &ObjectPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for property chain axioms that might imply this relationship
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sub_axiom) = axiom {
                // Check if the superproperty is involved in property chains
                if &sub_axiom.super_property == superproperty
                    && let ObjectPropertyExpression::PropertyChain(chain) = &sub_axiom.sub_property
                {
                    // Check if subproperty is part of this chain or can be derived from it
                    if self.property_in_chain_or_derivable(subproperty, chain, ontology)? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check if a property is in a chain or derivable from it
    fn property_in_chain_or_derivable(
        &self,
        property: &ObjectPropertyExpression,
        chain: &[ObjectPropertyExpression],
        ontology: &Ontology,
    ) -> Result<bool> {
        // Direct membership in chain
        if chain.contains(property) {
            return Ok(true);
        }

        // Check if property is equivalent to the entire chain
        if chain.len() == 1 && &chain[0] == property {
            return Ok(true);
        }

        // Check if property is a subproperty of any property in the chain
        for chain_prop in chain {
            if self.is_subproperty_of(property, chain_prop, ontology)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if one data property is a subproperty of another.
    ///
    /// Superseded by [`crate::core::reasoner::property_hierarchy::DataPropertyHierarchy`]
    /// for hierarchy queries; kept for callers that need the raw asserted
    /// edge check.
    #[allow(dead_code)]
    fn is_data_subproperty_of(
        &self,
        subproperty: &DataPropertyExpression,
        superproperty: &DataPropertyExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Check for direct SubDataPropertyOf axioms
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::SubDataPropertyOf(sub_axiom) = axiom
                && &sub_axiom.sub_property == subproperty
                && &sub_axiom.super_property == superproperty
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get types of a given individual
    ///
    /// # Panics
    ///
    /// Panics if an individual's IRI cannot be converted to a URL.
    pub fn get_types(
        &self,
        individual: &str,
        direct: bool,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting types for individual: {individual}");

        let ontology_guard = read_lock(ontology, "classification: reading ontology for get_types")?;
        let mut types = Vec::new();

        // Find all class assertions for this individual
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom
                && let crate::ontology::Individual::Named(ind) = &assertion.individual
                && ind.iri.as_str() == individual
                && let ClassExpression::Class(class) = &assertion.class
            {
                types.push(class.iri.to_string());
            }
        }

        // If direct is false, also infer types through subclass relationships
        if !direct {
            let mut all_types = HashSet::new();
            for t in &types {
                all_types.insert(t.clone());

                // Get all superclasses of this type
                // Convert string to ClassExpression for the call
                let class_expr = ClassExpression::Class(crate::ontology::Class {
                    iri: crate::ontology::IRI::new(t)
                        .to_url()
                        .expect("Failed to convert IRI to URL")
                        .into(),
                });
                if let Ok(supertypes) = {
                    let mut local_cache = CacheManager::default();
                    self.get_superclasses(
                        &class_expr,
                        ontology,
                        statistics,
                        false,
                        &mut local_cache,
                    )
                } {
                    for supertype in supertypes {
                        if let ClassExpression::Class(class) = supertype {
                            all_types.insert(class.iri.to_string());
                        }
                    }
                }
            }
            types = all_types.into_iter().collect();
        }

        types.sort();
        Ok(types)
    }

    /// Get object property values for an individual
    pub fn get_object_property_values(
        &self,
        individual: &str,
        property: &str,
        ontology: &OntologyRef,
        _statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting object property values for {individual} -> {property}");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_object_property_values",
        )?;
        let mut values = Vec::new();

        // Find all object property assertions for this individual and property
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) = axiom
                && let (
                    crate::ontology::Individual::Named(subj),
                    crate::ontology::Individual::Named(obj),
                ) = (&assertion.source, &assertion.target)
                && subj.iri.as_str() == individual
                && let ObjectPropertyExpression::ObjectProperty(prop) = &assertion.property
                && prop.iri.as_str() == property
            {
                values.push(obj.iri.to_string());
            }
        }

        values.sort();
        Ok(values)
    }

    /// Get data property values for an individual
    pub fn get_data_property_values(
        &self,
        individual: &str,
        property: &str,
        ontology: &OntologyRef,
        _statistics: &mut ReasoningStatistics,
    ) -> Result<Vec<String>> {
        info!("Getting data property values for {individual} -> {property}");

        let ontology_guard = read_lock(
            ontology,
            "classification: reading ontology for get_data_property_values",
        )?;
        let mut values = Vec::new();

        // Find all data property assertions for this individual and property
        for axiom in ontology_guard.axioms() {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom
                && let crate::ontology::Individual::Named(subj) = &assertion.individual
                && subj.iri.as_str() == individual
            {
                let DataPropertyExpression::DataProperty(prop) = &assertion.property;
                if prop.iri.as_str() == property {
                    values.push(assertion.value.to_string());
                }
            }
        }

        values.sort();
        Ok(values)
    }

    /// Check if an individual satisfies a datatype restriction based on its data property values
    /// This is a helper for proper datatype reasoning
    fn check_datatype_restriction_satisfaction(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
        data_range: &crate::ontology::DataRange,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Get all data property values for this individual and property
        // (whole-`Individual` identity: anonymous individuals match too).
        let mut has_matching_value = false;

        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom
                && assertion.individual == *individual
            {
                // Check if this is the right property
                if self.data_properties_match(property, &assertion.property) {
                    // Check if the value satisfies the data range
                    if self.literal_satisfies_data_range(&assertion.value, data_range)? {
                        has_matching_value = true;
                        break;
                    }
                }
            }
        }

        Ok(has_matching_value)
    }

    /// Check if two object properties match (simple IRI equality; inverse and chains are not unfolded)
    fn object_properties_match(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> bool {
        use crate::ontology::ObjectPropertyExpression;
        match (prop1, prop2) {
            (
                ObjectPropertyExpression::ObjectProperty(p1),
                ObjectPropertyExpression::ObjectProperty(p2),
            ) => p1.iri == p2.iri,
            _ => false,
        }
    }

    /// Check if two data properties match
    fn data_properties_match(
        &self,
        prop1: &DataPropertyExpression,
        prop2: &DataPropertyExpression,
    ) -> bool {
        match (prop1, prop2) {
            (
                DataPropertyExpression::DataProperty(p1),
                DataPropertyExpression::DataProperty(p2),
            ) => p1.iri == p2.iri,
        }
    }

    /// Check if a literal value satisfies a data range (with facet restrictions).
    ///
    /// Delegates to the shared value-space engine in [`datatype_validation`] so that
    /// classification and the tableau agree on facet evaluation (length/pattern/
    /// totalDigits/fractionDigits/langRange in addition to the numeric ordering facets),
    /// user-defined datatypes (`DatatypeDefinition`) and `owl:real`/`owl:rational`.
    fn literal_satisfies_data_range(
        &self,
        literal: &crate::ontology::Literal,
        data_range: &crate::ontology::DataRange,
    ) -> Result<bool> {
        self.datatype_validator
            .literal_satisfies_data_range(literal, data_range)
    }

    /// Enhanced instance checking that also checks datatype restrictions
    /// This checks if an individual satisfies a class expression based on data property values
    ///
    /// # Re-entrancy
    ///
    /// Every call runs under the general [`INSTANCE_EVAL_STACK`] guard keyed
    /// on `(individual key, whole class expression)`: a state that is already
    /// being evaluated higher up the stack returns a conservative `Ok(false)`
    /// instead of recursing.  This bounds *all* recursive paths through this
    /// method — the complex-expression branch (`check_complex_expression`
    /// operands, existential fillers) and Strategies 3/5/6 alike — and is the
    /// termination guarantee for cyclic TBoxes (`A ≡ ∃r.B` / `B ≡ ∃r.A`,
    /// self-referential `C ≡ C ⊓ D`) and for the nested defined-subclass
    /// (Strategy 5) re-entry that previously re-evaluated the same query state
    /// from sibling paths after the helper-scoped [`DEFINED_EVAL_STACK`] keys
    /// had already popped.  [`INSTANCE_EVAL_MAX_DEPTH`] (256) is a hard second
    /// safety net against unbounded *distinct*-state nesting.  Cut states are
    /// conservative `false`: every caller keeps its tableau snapshot /
    /// `task_service.check_instance` fallback, which remains authoritative.
    pub fn check_instance_with_datatype_reasoning(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // General re-entrancy guard — see the method docs.  Keyed on the
        // whole `ClassExpression` (Hash + Eq), so nested evaluation of the
        // *same* state — through any alternation of class expressions,
        // fillers and named classes — is cut here before any strategy runs.
        let Some(_guard) = InstanceEvalGuard::enter((individual.to_string(), class_expr.clone()))
        else {
            // Cycle or depth cap: conservative false; the caller's tableau
            // snapshot fallback remains authoritative.
            return Ok(false);
        };
        // Handle specific class expression types that we can check directly
        match class_expr {
            ClassExpression::Class(cls) => {
                // Strategy 1: Check if individual is directly asserted to be in this class
                if self.check_explicit_class_assertion(individual, cls, ontology)? {
                    return Ok(true);
                }

                // Strategy 2: Check if individual is asserted to be in a subclass of this class
                // Get all asserted types of the individual. The stable key is
                // the `Individual` Display form (named → IRI, anonymous →
                // `_:id`); it also keys the defined-evaluation cycle guard.
                let individual_key = individual.to_string();
                let mut asserted_classes = Vec::new();

                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom
                        && assertion.individual == *individual
                        && let ClassExpression::Class(asserted_cls) = &assertion.class
                    {
                        asserted_classes.push(asserted_cls.iri.clone());
                    }
                }

                // Check if any of the asserted classes is a subclass of the target class
                for asserted_class_iri in &asserted_classes {
                    if self.is_subclass_of_iri(asserted_class_iri, &cls.iri, ontology)? {
                        return Ok(true);
                    }
                }

                // Strategy 3: Check if there's an EquivalentClasses axiom that defines this class
                // with a complex expression we can evaluate.  Runs under the
                // `DEFINED_EVAL_STACK` cycle guard: a definition that (transitively)
                // references this very (individual, class) pair — `C ≡ C ⊓ …`, or
                // `C ≡ D ⊓ …` with `D ≡ C ⊓ …` — would otherwise recurse forever; the
                // re-entered evaluation is skipped (conservative `false`, the tableau
                // snapshot remains the caller's fallback).
                if self.check_own_equivalent_definition(
                    individual,
                    &cls.iri,
                    &individual_key,
                    ontology,
                )? {
                    return Ok(true);
                }

                // Strategy 4: iterative BFS through the named subclass hierarchy.
                // Direct subclasses are checked with ClassAssertion axioms only (no
                // recursive call — avoids a stack overflow on large ontologies, e.g.
                // 2995 classes deep); *defined* subclasses additionally get their
                // complex equivalent definition evaluated (Strategy 5, guarded by
                // `DEFINED_EVAL_STACK` for termination), and every visited class —
                // including `C` itself — gets its complex GCIs
                // (`SubClassOf(E, I)`, Strategy 6) evaluated for the individual.
                {
                    // Pre-build, in ONE pass over the axioms:
                    //  * `subclass_index`: superclass-IRI → named subclasses
                    //    (SubClassOf named→named, the named-conjunct
                    //    decompositions of `SubClassOf(D, E ⊓ …)` /
                    //    `EquivalentClasses(D, E ⊓ …)` — both entail
                    //    `D ⊑ E` —, DisjointUnion members, and the
                    //    bidirectional edges of EquivalentClasses(named, named) —
                    //    `A ≡ B` means `A ⊑ B` and `B ⊑ A`), so the BFS inner loop
                    //    is an O(1) lookup instead of a full axiom scan per frontier
                    //    step (O(n×m) → O(n+m));
                    //  * `defined_index`: named class → its complex equivalent
                    //    definitions (non-named operands of an EquivalentClasses
                    //    axiom that has the class as a named operand);
                    //  * `complex_subclass_index`: named superclass → complex
                    //    subclass expressions of `SubClassOf(E, I)` axioms
                    //    (Strategy 6).
                    // All three are built in the same pass so Strategies 5 and 6
                    // pay no extra axiom scan to find their candidates; only the
                    // *defined* definitions and complex GCIs the BFS reaches are
                    // ever evaluated (bounded work — see Strategies 5 and 6
                    // below).
                    let mut subclass_index: std::collections::HashMap<
                        crate::ontology::IRI,
                        Vec<crate::ontology::concepts::Class>,
                    > = std::collections::HashMap::new();
                    let mut defined_index: std::collections::HashMap<
                        crate::ontology::IRI,
                        Vec<ClassExpression>,
                    > = std::collections::HashMap::new();
                    // `complex_subclass_index`: named superclass → *complex*
                    // subclass expressions (`SubClassOf(E, I)` where `E` is not a
                    // named class — intersection, union, someValuesFrom,
                    // hasValue, data restrictions, … — and `I` is named).
                    // Strategy 6 evaluates the entries headed by every class the
                    // BFS visits: if `individual ∈ E` and `E ⊑ I` with
                    // `I ⊑* C`, then `individual ∈ C`.  Built in the same
                    // O(#axioms) pass, so the BFS pays no extra axiom scan to
                    // *find* the candidates; only the GCIs headed by visited
                    // classes are ever evaluated (bounded work — Strategy 6).
                    let mut complex_subclass_index: std::collections::HashMap<
                        crate::ontology::IRI,
                        Vec<ClassExpression>,
                    > = std::collections::HashMap::new();
                    for axiom in ontology.axioms() {
                        if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom {
                            match (&subclass_axiom.subclass, &subclass_axiom.superclass) {
                                (
                                    ClassExpression::Class(sub_cls),
                                    ClassExpression::Class(superclass),
                                ) => {
                                    subclass_index
                                        .entry(superclass.iri.clone())
                                        .or_default()
                                        .push(sub_cls.clone());
                                }
                                // Complex subclass with a *named* head → the
                                // complex-GCI index (Strategy 6).
                                (complex_sub, ClassExpression::Class(superclass)) => {
                                    complex_subclass_index
                                        .entry(superclass.iri.clone())
                                        .or_default()
                                        .push(complex_sub.clone());
                                }
                                // Named subclass + intersection superclass:
                                // `D ⊑ E ⊓ F` entails `D ⊑ E` and `D ⊑ F` —
                                // and recursively, `D ⊑` every *named*
                                // conjunct of a nested intersection — so the
                                // BFS can reach `D` downward from any
                                // conjunct (e.g. `D ⊑ E ⊓ F` + `E ⊑ C` ⇒
                                // `C → E → D`). Union / existential /
                                // universal / data-restriction superclasses
                                // are deliberately NOT decomposed: `D ⊑ E ⊔ F`
                                // does not entail `D ⊑ E`, so they would be
                                // unsound as named subclass edges (they fall
                                // through to `_` below, as does
                                // `SubClassOf(complex, complex)` — no named
                                // head to index).
                                (
                                    ClassExpression::Class(sub_cls),
                                    ClassExpression::ObjectIntersectionOf(_),
                                ) => {
                                    let mut conjuncts = Vec::new();
                                    named_intersection_conjuncts(
                                        &subclass_axiom.superclass,
                                        &mut conjuncts,
                                    );
                                    for conjunct_iri in conjuncts {
                                        if conjunct_iri != sub_cls.iri {
                                            subclass_index
                                                .entry(conjunct_iri)
                                                .or_default()
                                                .push(sub_cls.clone());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        // DisjointUnion(A, [B, C]): each member is a subclass of A, so the
                        // instance BFS must reach B/C when walking down from A.
                        if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom
                            && let ClassExpression::Class(union_cls) = &disjoint_union.class
                        {
                            for member in &disjoint_union.disjoint_classes {
                                if let ClassExpression::Class(member_cls) = member {
                                    subclass_index
                                        .entry(union_cls.iri.clone())
                                        .or_default()
                                        .push(member_cls.clone());
                                }
                            }
                        }
                        // EquivalentClasses: named↔named edges (mutual subclass) plus
                        // the complex definitions attached to every named operand.
                        if let crate::ontology::axioms::Axiom::EquivalentClasses(equivalent) = axiom
                        {
                            let named: Vec<crate::ontology::IRI> = equivalent
                                .classes
                                .iter()
                                .filter_map(|c| match c {
                                    ClassExpression::Class(cls)
                                        if cls.iri.as_str()
                                            != "http://www.w3.org/2002/07/owl#Class" =>
                                    {
                                        Some(cls.iri.clone())
                                    }
                                    _ => None,
                                })
                                .collect();
                            let complex: Vec<ClassExpression> = equivalent
                                .classes
                                .iter()
                                .filter(|c| !matches!(c, ClassExpression::Class(_)))
                                .cloned()
                                .collect();
                            if !complex.is_empty() {
                                for iri in &named {
                                    defined_index
                                        .entry(iri.clone())
                                        .or_default()
                                        .extend(complex.iter().cloned());
                                }
                            }
                            for (i, a) in named.iter().enumerate() {
                                for (j, b) in named.iter().enumerate() {
                                    if i != j {
                                        subclass_index.entry(a.clone()).or_default().push(
                                            crate::ontology::concepts::Class { iri: b.clone() },
                                        );
                                    }
                                }
                            }
                            // `D ≡ E ⊓ …`: the `D ⊑ E ⊓ …` half entails
                            // `D ⊑ E` for every *named* conjunct (recursively
                            // through nested intersections) — a sound
                            // named→named edge that lets the BFS reach `D`
                            // downward from a conjunct. The converse half
                            // (`E ⊓ … ⊑ D`) does NOT entail `E ⊑ D`
                            // (`Student ≡ Student ⊓ Minor` does not make
                            // every Student a Minor), so no
                            // conjunct→`D` edge is added.
                            let mut conjuncts: Vec<crate::ontology::IRI> = Vec::new();
                            for operand in &complex {
                                named_intersection_conjuncts(operand, &mut conjuncts);
                            }
                            for conjunct_iri in &conjuncts {
                                for named_iri in &named {
                                    if named_iri != conjunct_iri {
                                        subclass_index
                                            .entry(conjunct_iri.clone())
                                            .or_default()
                                            .push(crate::ontology::concepts::Class {
                                                iri: named_iri.clone(),
                                            });
                                    }
                                }
                            }
                        }
                    }

                    let mut worklist: std::collections::VecDeque<crate::ontology::IRI> =
                        std::collections::VecDeque::new();
                    let mut visited_s4: std::collections::HashSet<crate::ontology::IRI> =
                        std::collections::HashSet::new();
                    worklist.push_back(cls.iri.clone());
                    visited_s4.insert(cls.iri.clone());

                    while let Some(current_iri) = worklist.pop_front() {
                        // Strategy 6: complex GCIs headed by the visited class.
                        // The BFS reached `current_iri` because
                        // `current_iri ⊑* C`, so an individual satisfying any
                        // `SubClassOf(E, current_iri)` is an instance of `C`.
                        // Evaluated for *every* visited class (including `C`
                        // itself on the first pop) — the head need not be
                        // asserted for the individual.
                        if let Some(complex_subclasses) = complex_subclass_index.get(&current_iri) {
                            for complex_sub in complex_subclasses {
                                if self.check_complex_gci(
                                    individual,
                                    &current_iri,
                                    complex_sub,
                                    ontology,
                                )? {
                                    return Ok(true);
                                }
                            }
                        }
                        if let Some(sub_classes) = subclass_index.get(&current_iri) {
                            for sub_cls in sub_classes {
                                if !visited_s4.contains(&sub_cls.iri) {
                                    visited_s4.insert(sub_cls.iri.clone());
                                    // Check only direct assertion — no recursive call.
                                    if self.check_explicit_class_assertion(
                                        individual, sub_cls, ontology,
                                    )? {
                                        return Ok(true);
                                    }
                                    // Strategy 5: upward inference from a *defined*
                                    // subclass.  The BFS reached `D` because `D ⊑* C`;
                                    // if `D` has a complex equivalent definition and the
                                    // individual satisfies it, then individual ∈ D ⊑ C.
                                    // Only *defined* candidates are evaluated — bounded
                                    // by the number of defined subclasses of C below
                                    // (each check is one axiom scan), never all classes.
                                    if let Some(definitions) = defined_index.get(&sub_cls.iri)
                                        && self.check_defined_subclass(
                                            individual,
                                            sub_cls,
                                            definitions,
                                            ontology,
                                        )?
                                    {
                                        return Ok(true);
                                    }
                                    worklist.push_back(sub_cls.iri.clone());
                                }
                            }
                        }
                    }
                }

                // Couldn't determine membership
                Ok(false)
            }
            // For complex expressions, delegate to specialized handler
            _ => self.check_complex_expression(individual, class_expr, ontology),
        }
    }

    /// Strategy 3 helper: membership from `class_iri`'s own complex equivalent
    /// definition(s) — the non-named operands of every `EquivalentClasses` axiom
    /// that has `class_iri` as a named operand.
    ///
    /// Runs under the [`DEFINED_EVAL_STACK`] cycle guard keyed on
    /// (`individual_iri`, `class_iri`): the guard is acquired for the whole
    /// evaluation and released on every exit path (including `Err`), so a
    /// re-entrant evaluation of the same pair returns `Ok(false)` instead of
    /// recursing forever.
    fn check_own_equivalent_definition(
        &self,
        individual: &Individual,
        class_iri: &IRI,
        individual_iri: &str,
        ontology: &Ontology,
    ) -> Result<bool> {
        let Some(_guard) = DefinedEvalGuard::enter((individual_iri.to_string(), class_iri.clone()))
        else {
            return Ok(false);
        };
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                // Check if our class is in the equivalent classes
                let our_class_in_equiv = equiv_axiom.classes.iter().any(|c| {
                    if let ClassExpression::Class(eq_cls) = c {
                        eq_cls.iri == *class_iri
                    } else {
                        false
                    }
                });

                if our_class_in_equiv {
                    // Check all the other equivalent class expressions
                    for equiv_class in &equiv_axiom.classes {
                        if let ClassExpression::Class(eq_cls) = equiv_class {
                            // Skip the class itself
                            if eq_cls.iri == *class_iri {
                                continue;
                            }
                            // Skip owl:Class which appears due to parsing artifacts
                            if eq_cls.iri.as_str() == "http://www.w3.org/2002/07/owl#Class" {
                                continue;
                            }
                        }

                        // Try to match against this equivalent expression.
                        // Covers all complex constructors that check_complex_expression handles.
                        match equiv_class {
                            ClassExpression::ObjectIntersectionOf(_)
                            | ClassExpression::ObjectUnionOf(_)
                            | ClassExpression::ObjectSomeValuesFrom { .. }
                            | ClassExpression::ObjectHasValue { .. }
                            | ClassExpression::ObjectOneOf(_)
                            | ClassExpression::DataSomeValuesFrom { .. }
                            | ClassExpression::DataHasValue { .. }
                                if self.check_complex_expression(
                                    individual,
                                    equiv_class,
                                    ontology,
                                )? =>
                            {
                                return Ok(true);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Strategy 5 helper: evaluate the complex equivalent definitions of the
    /// *defined* subclass `defined_class` (reached by the named-subclass BFS of
    /// [`Self::check_instance_with_datatype_reasoning`]) for `individual`.
    ///
    /// Runs under the [`DEFINED_EVAL_STACK`] cycle guard keyed on
    /// (`individual`, `defined_class`): definitions may reference named classes
    /// whose own definition chains lead back here; the re-entered pair is
    /// skipped with a conservative `false` (the tableau snapshot remains the
    /// fallback), which also bounds recursion to the finite pair set.
    ///
    /// Returns `Ok(true)` as soon as one definition is satisfied.
    fn check_defined_subclass(
        &self,
        individual: &Individual,
        defined_class: &crate::ontology::concepts::Class,
        definitions: &[ClassExpression],
        ontology: &Ontology,
    ) -> Result<bool> {
        // Same key derivation as `check_own_equivalent_definition` so the two
        // guards see the same pair for the same individual.
        let individual_key = individual
            .iri()
            .map_or_else(|| "anonymous".to_string(), std::string::ToString::to_string);
        let Some(_guard) = DefinedEvalGuard::enter((individual_key, defined_class.iri.clone()))
        else {
            // Cycle: this pair's definitions are already being evaluated higher
            // in the stack — conservative false.
            return Ok(false);
        };
        for definition in definitions {
            if self.check_complex_expression(individual, definition, ontology)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Strategy 6 helper: membership from a *complex GCI* — `SubClassOf(E, I)`
    /// where `E` is a complex class expression and `I` a named class — reached
    /// by the named-subclass BFS of
    /// [`Self::check_instance_with_datatype_reasoning`].
    ///
    /// If `individual ∈ E` and `I ⊑* C` (the BFS reached `I` from the queried
    /// class `C`), then `individual ∈ C`.
    ///
    /// Runs under the [`DEFINED_EVAL_STACK`] cycle guard keyed on
    /// (`individual`, `I`): evaluating `E` re-enters the named-class branch for
    /// every named operand of `E`, and a GCI cycle (`A ⊓ B ⊑ C` evaluated while
    /// an evaluation of `C` is already on the stack) would otherwise recurse
    /// forever.  The re-entered pair returns a conservative `false` (the tableau
    /// snapshot remains the caller's fallback); the key set is finite
    /// (individuals × named classes), so recursion is bounded to one frame per
    /// pair.  Acyclic chains — e.g. `∃r.D ⊑ C` where `D`'s own check reaches
    /// further GCIs — are evaluated in full.
    fn check_complex_gci(
        &self,
        individual: &Individual,
        superclass_iri: &IRI,
        subclass_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Same key derivation as `check_own_equivalent_definition` so the two
        // guards see the same pair for the same individual (a re-entered
        // Strategy 3/5 evaluation of this very class is likewise skipped).
        let individual_key = individual.to_string();
        let Some(_guard) = DefinedEvalGuard::enter((individual_key, superclass_iri.clone())) else {
            // Cycle: this class's complex GCIs are already being evaluated
            // higher in the stack — conservative false.
            return Ok(false);
        };
        self.check_complex_expression(individual, subclass_expr, ontology)
    }

    /// Check complex class expressions (non-named classes) directly
    fn check_complex_expression(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &Ontology,
    ) -> Result<bool> {
        match class_expr {
            ClassExpression::ObjectIntersectionOf(operands) => {
                // Every operand must be satisfied (conjunction)
                for operand in operands {
                    if !self
                        .check_instance_with_datatype_reasoning(individual, operand, ontology)?
                    {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ClassExpression::ObjectUnionOf(operands) => {
                // At least one operand must be satisfied (disjunction)
                for operand in operands {
                    if self.check_instance_with_datatype_reasoning(individual, operand, ontology)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // ∃ r.C : individual must have at least one r-successor that satisfies C
                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) =
                        axiom
                        && assertion.source == *individual
                        && self.object_properties_match(property, &assertion.property)
                        && self.check_instance_with_datatype_reasoning(
                            &assertion.target,
                            filler,
                            ontology,
                        )?
                    {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ClassExpression::ObjectHasValue {
                property,
                value: filler_ind,
            } => {
                // ∃ r.{o} : individual must have exactly the specified individual as an r-value
                // (whole-`Individual` identity on both endpoints, so
                // anonymous individuals participate as well).
                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) =
                        axiom
                        && assertion.source == *individual
                        && self.object_properties_match(property, &assertion.property)
                        && assertion.target == *filler_ind
                    {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ClassExpression::ObjectOneOf(individuals) => {
                // {a, b, …}: nominal enumeration. The queried individual is a
                // member iff it *is* one of the listed individuals — whole
                // `Individual` identity, so anonymous members (`_:x`) match
                // their own nominal and never a named or foreign one.
                Ok(individuals.iter().any(|ind| ind == individual))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // Check if individual has a data property value that satisfies the restriction
                self.check_datatype_restriction_satisfaction(individual, property, filler, ontology)
            }
            ClassExpression::DataHasValue { property, value } => {
                // Check if individual has this specific data property value
                // (whole-`Individual` identity: anonymous individuals match).
                for axiom in ontology.axioms() {
                    if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom
                        && assertion.individual == *individual
                        && self.data_properties_match(property, &assertion.property)
                        && crate::core::reasoner::datatype_validation::DatatypeValidator::
                            literal_values_equal(&assertion.value, value)
                    {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            // Named classes are handled by check_instance_with_datatype_reasoning;
            // reaching this point means the caller passed a Class directly — delegate back.
            ClassExpression::Class(_) => {
                self.check_instance_with_datatype_reasoning(individual, class_expr, ontology)
            }
            _ => {
                // For unsupported constructors (AllValuesFrom, cardinality, …)
                // conservatively return false so tableau picks it up instead.
                Ok(false)
            }
        }
    }

    /// Check if an individual is explicitly asserted to be in a named class
    fn check_explicit_class_assertion(
        &self,
        individual: &Individual,
        class: &crate::ontology::Class,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Whole-`Individual` identity: named individuals match by IRI and
        // anonymous individuals by their stable `_:id` Display key.
        for axiom in ontology.axioms() {
            if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom
                && assertion.individual == *individual
                && let ClassExpression::Class(asserted_class) = &assertion.class
                && asserted_class.iri == class.iri
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if one class (by IRI) is a subclass of another class (by IRI)
    /// This includes transitive subclass relationships and `DisjointUnion`-derived
    /// subsumptions (`DisjointUnion(A, [B, C])` makes each member a subclass of A).
    fn is_subclass_of_iri(
        &self,
        subclass_iri: &IRI,
        superclass_iri: &IRI,
        ontology: &Ontology,
    ) -> Result<bool> {
        // Direct equality
        if subclass_iri == superclass_iri {
            return Ok(true);
        }

        // Iterative BFS through the superclass hierarchy to avoid stack overflow
        // on deep or cyclic class graphs.
        let mut worklist: std::collections::VecDeque<IRI> = std::collections::VecDeque::new();
        let mut visited: std::collections::HashSet<IRI> = std::collections::HashSet::new();
        worklist.push_back(subclass_iri.clone());
        visited.insert(subclass_iri.clone());

        while let Some(current) = worklist.pop_front() {
            // DisjointUnion(A, [B, C]): every member is a subclass of the union class.
            for axiom in ontology.axioms() {
                if let crate::ontology::axioms::Axiom::DisjointUnion(disjoint_union) = axiom {
                    let member_match = disjoint_union.disjoint_classes.iter().any(
                        |member| matches!(member, ClassExpression::Class(m) if m.iri == current),
                    );
                    if member_match {
                        let union_iri = &disjoint_union.class;
                        if let ClassExpression::Class(uc) = union_iri {
                            if uc.iri == *superclass_iri {
                                return Ok(true);
                            }
                            if !visited.contains(&uc.iri) {
                                visited.insert(uc.iri.clone());
                                worklist.push_back(uc.iri.clone());
                            }
                        }
                    }
                }
            }
            for axiom in ontology.axioms() {
                if let crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) = axiom
                    && let ClassExpression::Class(sub) = &subclass_axiom.subclass
                    && let ClassExpression::Class(sup) = &subclass_axiom.superclass
                    && sub.iri == current
                {
                    if sup.iri == *superclass_iri {
                        return Ok(true);
                    }
                    if !visited.contains(&sup.iri) {
                        visited.insert(sup.iri.clone());
                        worklist.push_back(sup.iri.clone());
                    }
                }
            }
        }

        Ok(false)
    }
}
