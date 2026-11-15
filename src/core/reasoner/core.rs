//! Core reasoner functionality
//!
//! This module contains the main Reasoner struct and core operations like
//! loading ontologies and basic reasoning setup.

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::{
        lock_helpers::{read_lock, write_lock},
        reasoner::{
            abox_rules::abox_classification_rules,
            classification::ClassificationService,
            explanation::ExplanationService,
            queries::QueryProcessor,
            results::{ClassificationResult, RealisationResult},
            statistics::ReasoningStatistics,
            tableau::{TableauFactory, TableauSnapshot},
            tasks::ReasoningTaskService,
        },
        tableau::TableauState,
    },
    dl_clauses::DLClauseGenerator,
    import::{ImportManager, ImportManagerConfig, ImportResolutionStrategy},
    ontology::{
        ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, OntologyRef,
        axioms::AxiomTrait,
    },
};
use serde_json;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

/// Statistics returned by `ABox` materialization operations.
#[derive(Debug, Default, Clone)]
pub struct MaterializationStats {
    /// Number of forward-chaining rules fired
    pub rules_fired: u64,
    /// Number of new facts added to the `ABox`
    pub facts_added: usize,
    /// Number of fixpoint iterations
    pub iterations: u64,
    /// Total elapsed time in milliseconds
    pub duration_ms: u64,
    /// Number of SPARQL INSERT rules run
    pub sparql_rules_run: u64,
}

/// `OWLlink` request structure
#[derive(Debug, Clone)]
struct OWLlinkRequest {
    /// Command name (Tell, `IsClassSatisfiable`, etc.)
    command: String,
    /// Axioms to add (for Tell command)
    #[allow(dead_code)]
    axioms: Vec<crate::ontology::axioms::Axiom>,
    /// Class IRI for class-related queries
    class_iri: Option<url::Url>,
    /// Individual IRI for individual-related queries  
    #[allow(dead_code)]
    individual_iri: Option<url::Url>,
    /// Direct flag for hierarchical queries
    direct: Option<bool>,
}

/// SPARQL query structure
#[derive(Debug, Clone)]
struct SparqlQuery {
    /// Query type (SELECT, ASK, CONSTRUCT, DESCRIBE, INSERT, DELETE)
    query_type: String,
    /// Variables to select
    variables: Vec<String>,
    /// Triple patterns in WHERE clause
    patterns: Vec<TriplePattern>,
    /// Triple patterns to insert (for INSERT operations)
    insert_patterns: Vec<TriplePattern>,
    /// Triple patterns to delete (for DELETE operations)
    delete_patterns: Vec<TriplePattern>,
    /// Original query string
    #[allow(dead_code)]
    original_query: String,
}

/// Triple pattern in SPARQL query
#[derive(Debug, Clone)]
struct TriplePattern {
    /// Subject (variable or IRI)
    subject: String,
    /// Predicate (variable or IRI)
    predicate: String,
    /// Object (variable or IRI)
    object: String,
}

/// Main reasoner interface
#[derive(Debug)]
pub struct Reasoner {
    /// Reasoning configuration
    #[allow(dead_code)]
    config: ReasonerConfig,

    /// Current ontology being reasoned over
    ontology: Option<OntologyRef>,

    /// Cache manager for reasoning results
    cache_manager: CacheManager,

    /// Tableau factory for creating reasoning algorithms
    tableau_factory: TableauFactory,

    /// Statistics about reasoning operations.
    ///
    /// Interior-mutable so the shared totals can be accumulated from
    /// `&self`/`&mut self` operation wrappers (`classify`, `realize`,
    /// `get_instances`, `is_consistent`) and still be read by
    /// [`Reasoner::get_statistics`].
    statistics: std::sync::Mutex<ReasoningStatistics>,

    /// Memoised answer of [`Reasoner::is_consistent`]:
    /// `(ontology version, consistency answer)`.  Version-stamped so that any
    /// mutation of the loaded ontology (which bumps the version) invalidates
    /// it, and cleared explicitly by every mutator through
    /// [`Reasoner::invalidate_query_caches`].
    consistency_cache: std::sync::Mutex<Option<(u64, bool)>>,

    /// Task service for basic reasoning operations
    task_service: ReasoningTaskService,

    /// Classification service for complex operations
    classification_service: ClassificationService,

    /// Query processor for SPARQL and `OWLlink`
    #[allow(dead_code)]
    query_processor: QueryProcessor,

    /// Explanation service
    #[allow(dead_code)]
    explanation_service: ExplanationService,

    /// Optional RL forward-chaining reasoner (rebuilt on demand)
    #[allow(dead_code)]
    rl_reasoner: Option<Box<crate::profiles::rl_reasoner::RLReasoner>>,

    /// Cached EL reasoner — set when the ontology is detected to be EL-conforming.
    /// Reused across repeated `classify()` calls to avoid re-initialization overhead.
    el_reasoner: Option<Box<crate::profiles::el_reasoner::ELReasoner>>,

    /// Custom SPARQL INSERT WHERE rules for `ABox` materialization
    sparql_materialization_rules: Vec<String>,

    /// Flag indicating that the Oxigraph store should be rebuilt
    materialization_stale: std::sync::atomic::AtomicBool,

    /// Oxigraph-backed SPARQL store (lazy-initialised, interior-mutable)
    #[cfg(feature = "sparql-store")]
    oxigraph_store: std::sync::Mutex<Option<crate::core::reasoner::oxigraph_store::OxigraphStore>>,

    /// One-shot cache of the completed-realisation tableau snapshot (Phase A).
    ///
    /// Built lazily by [`Reasoner::realization_snapshot`] — one tableau run
    /// shared by every `get_instances` / `is_instance_of` query — and cleared
    /// by every method that mutates the loaded ontology. A `Mutex<Option<…>>`
    /// (rather than `OnceLock`) because several mutators — SPARQL
    /// `INSERT`/`DELETE DATA`, `materialize_abox` — take `&self`.
    snapshot_cache: std::sync::Mutex<Option<std::sync::Arc<TableauSnapshot>>>,
}

impl Reasoner {
    /// Create a new reasoner instance with the given configuration
    pub fn new(config: crate::config::ReasonerConfig) -> Result<Self> {
        let cache_manager = CacheManager::default();
        let tableau_factory = TableauFactory::new(config.clone())?;

        // Create individual tableau factories for each service
        let task_tableau_factory = TableauFactory::new(config.clone())?;
        let classification_tableau_factory = TableauFactory::new(config.clone())?;

        let task_service = ReasoningTaskService::new(task_tableau_factory);
        let mut classification_service =
            ClassificationService::new(ReasoningTaskService::new(classification_tableau_factory));
        classification_service.set_performance_config(config.performance.clone());
        classification_service
            .set_absorption_enabled(config.reasoning.enable_absorption_classification);
        // Enforce `reasoning.max_memory_mb` (RSS guard) in the classification,
        // saturation and tableau loops.  `None`/`0` = unlimited.
        classification_service.set_max_memory_mb(config.reasoning.max_memory_mb);

        Ok(Self {
            ontology: None,
            config: config.clone(),
            cache_manager,
            tableau_factory,
            statistics: std::sync::Mutex::new(ReasoningStatistics::default()),
            consistency_cache: std::sync::Mutex::new(None),
            task_service,
            classification_service,
            query_processor: QueryProcessor::new(),
            explanation_service: ExplanationService::new(),
            rl_reasoner: None,
            el_reasoner: None,
            sparql_materialization_rules: Vec::new(),
            materialization_stale: std::sync::atomic::AtomicBool::new(false),
            #[cfg(feature = "sparql-store")]
            oxigraph_store: std::sync::Mutex::new(None),
            snapshot_cache: std::sync::Mutex::new(None),
        })
    }

    /// Check if a class is a subclass of another
    pub fn is_subclass_of(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            self.is_subclass_of_ontology(subclass, superclass, ontology)
        } else {
            self.check_semantic_subsumption(subclass, superclass)
        }
    }

    /// Subsumption check against a specific ontology (used by `check_entailment`
    /// when the ontology to reason over differs from the loaded one).
    fn is_subclass_of_ontology(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        // Syntactic identity check
        if subclass == superclass {
            return Ok(true);
        }

        // ── Fast path: answer from the cached classification result ─────────
        if let Some(classif) = self.classification_service.cached_classification()
            && let (ClassExpression::Class(_), ClassExpression::Class(_)) = (subclass, superclass)
        {
            // `_ ⊑ owl:Thing` always holds.
            if let ClassExpression::Class(c) = superclass
                && c.is_thing()
            {
                return Ok(true);
            }
            // `A ⊑ owl:Nothing` holds iff `A` is unsatisfiable.
            if let ClassExpression::Class(c) = superclass
                && c.is_nothing()
            {
                return Ok(classif.unsat.contains(subclass));
            }
            if classif.is_subsumed(subclass, superclass) {
                return Ok(true);
            }
        }

        let ontology_ref = read_lock(ontology, "core: reading ontology for is_subclass_of")?;

        // Use the incrementally-maintained axiom index (O(1) access — it is
        // kept in sync by `add_axiom`/`remove_axiom`/`set_axioms`) instead of
        // rebuilding a full index on every call.  The rebuild made each
        // `false` answer cost O(axioms) — measured 3 ms → 120 ms per call at
        // 2 k → 60 k axioms (CR-011 M1).
        let index = ontology_ref.axiom_index();

        // BFS through the subsumption hierarchy using index lookups
        let mut queue: std::collections::VecDeque<ClassExpression> =
            std::collections::VecDeque::new();
        let mut visited: std::collections::HashSet<ClassExpression> =
            std::collections::HashSet::new();
        queue.push_back(subclass.clone());
        visited.insert(subclass.clone());

        while let Some(current) = queue.pop_front() {
            // Check direct superclasses via SubClassOf index
            for sup in index.direct_superclasses(&current) {
                if sup == superclass || Self::union_set_equivalent(sup, superclass) {
                    return Ok(true);
                }
                if !visited.contains(sup) {
                    visited.insert(sup.clone());
                    queue.push_back(sup.clone());
                }
            }
            // Check equivalent classes via EquivalentClasses index
            for eq in index.equivalent_classes(&current) {
                if eq == superclass || Self::union_set_equivalent(eq, superclass) {
                    return Ok(true);
                }
                if !visited.contains(eq) {
                    visited.insert(eq.clone());
                    queue.push_back(eq.clone());
                }
            }
        }

        // Fallback: built-in OWL semantics (owl:Thing, owl:Nothing, intersections, etc.)
        self.check_semantic_subsumption(subclass, superclass)
    }

    /// Whether two class expressions denote the same union up to commutativity
    /// (`B ⊔ C ≡ C ⊔ B`, order/duplicate insensitive). Non-unions fall back to
    /// structural equality. Used so `A ⊑ (B ⊔ C)` answers `A ⊑ (C ⊔ B)`.
    fn union_set_equivalent(a: &ClassExpression, b: &ClassExpression) -> bool {
        match (a, b) {
            (ClassExpression::ObjectUnionOf(aa), ClassExpression::ObjectUnionOf(bb)) => {
                let sa: std::collections::HashSet<&ClassExpression> = aa.iter().collect();
                let sb: std::collections::HashSet<&ClassExpression> = bb.iter().collect();
                sa == sb
            }
            _ => a == b,
        }
    }

    /// Upward subsumption closure of `start` — every superclass and
    /// equivalent reachable through the axiom index, computed with an
    /// explicit stack (no recursion).
    ///
    /// Walks exactly the edges `is_subclass_of_ontology`'s BFS walks
    /// (`SubClassOf` superclasses + `EquivalentClasses`), but once per
    /// *query* instead of once per class *pair* (CR-011 M1).
    fn ancestor_closure(
        index: &crate::ontology::indexes::AxiomIndex,
        start: &ClassExpression,
    ) -> std::collections::HashSet<ClassExpression> {
        let mut seen: std::collections::HashSet<ClassExpression> = std::collections::HashSet::new();
        let mut stack: Vec<ClassExpression> = vec![start.clone()];
        seen.insert(start.clone());
        while let Some(current) = stack.pop() {
            for sup in index.direct_superclasses(&current) {
                if seen.insert(sup.clone()) {
                    stack.push(sup.clone());
                }
            }
            for eq in index.equivalent_classes(&current) {
                if seen.insert(eq.clone()) {
                    stack.push(eq.clone());
                }
            }
        }
        seen
    }

    /// `subclass ⊑ superclass` evaluated against precomputed ancestor
    /// closures — a faithful mirror of [`Self::is_subclass_of_ontology`]'s
    /// decision order (classification fast path → index BFS → built-in OWL
    /// semantics) with the index BFS answered from the shared closures.
    ///
    /// Both arguments must be members of the closure map's key set (the
    /// `get_types(direct)` filter only pairs up members of one type list).
    fn closure_subsumes(
        &self,
        classif: Option<&ClassificationResult>,
        ancestors: &std::collections::HashMap<
            ClassExpression,
            std::collections::HashSet<ClassExpression>,
        >,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> bool {
        // ── Fast path: answer from the cached classification ────────────
        if let Some(classif) = classif
            && let (ClassExpression::Class(_), ClassExpression::Class(_)) = (subclass, superclass)
        {
            // `_ ⊑ owl:Thing` always holds.
            if let ClassExpression::Class(c) = superclass
                && c.is_thing()
            {
                return true;
            }
            // `A ⊑ owl:Nothing` holds iff `A` is unsatisfiable (final answer,
            // exactly as in `is_subclass_of_ontology`).
            if let ClassExpression::Class(c) = superclass
                && c.is_nothing()
            {
                return classif.unsat.contains(subclass);
            }
            if classif.is_subsumed(subclass, superclass) {
                return true;
            }
        }

        // ── Index BFS, answered from the shared per-query closure ────────
        if let Some(closure) = ancestors.get(subclass)
            && (closure.contains(superclass)
                || (matches!(superclass, ClassExpression::ObjectUnionOf(_))
                    && closure.iter().any(|reached| {
                        reached != subclass && Self::union_set_equivalent(reached, superclass)
                    })))
        {
            return true;
        }

        // ── Built-in OWL semantics (owl:Thing, owl:Nothing, ⊓, ⊔) ────────
        self.check_semantic_subsumption(subclass, superclass)
            .unwrap_or(false)
    }

    /// Enhanced semantic subsumption checking with proper OWL semantics
    fn check_semantic_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        match (subclass, superclass) {
            // Bottom is subclass of everything
            (ClassExpression::Class(class), _)
                if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" =>
            {
                Ok(true)
            }

            // Everything is subclass of Top
            (_, ClassExpression::Class(class))
                if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" =>
            {
                Ok(true)
            }

            // Nothing is superclass of Top (except Top itself)
            (ClassExpression::Class(subclass_class), ClassExpression::Class(superclass_class))
                if subclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing"
                    && superclass_class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" =>
            {
                Ok(false)
            }

            // Intersection subsumption: A ⊓ B ⊑ A and A ⊓ B ⊑ B
            (ClassExpression::ObjectIntersectionOf(components), superclass) => {
                Ok(components.contains(superclass))
            }

            // Union subsumption: A ⊑ A ⊔ B and B ⊑ A ⊔ B
            (subclass, ClassExpression::ObjectUnionOf(components)) => Ok(components
                .contains(subclass)
                || Self::union_set_equivalent(subclass, superclass)),

            _ => Ok(false),
        }
    }

    /// Get the ontology being reasoned over
    #[must_use]
    pub fn get_ontology(&self) -> Option<&OntologyRef> {
        self.ontology.as_ref()
    }

    /// Run the tableau over the given ontology and return the individual→node
    /// merge map of the *completed* tableau (workstream A7).
    ///
    /// Keys are individual Display names (the `Individual::to_string` form used
    /// by the tableau's nominal labels); values are the canonical node id after
    /// `SameIndividual` / functional / inverse-functional / `HasKey` merging. Two
    /// individuals are forced equal exactly when they share a node id. Returns
    /// `Ok(None)` when the ontology is inconsistent (vacuously everything is
    /// same-as, but the caller falls back to its syntactic equality structure).
    pub fn tableau_individual_components(
        &self,
        ontology: &OntologyRef,
    ) -> Result<Option<HashMap<String, usize>>> {
        if self.ontology.is_none() {
            return Ok(Some(HashMap::new()));
        }
        // Reuse the shared one-shot snapshot when the caller's ontology *is*
        // the reasoner's own (Arc identity) — the very tableau the instance and
        // property-value queries use, built once and cached. A foreign ontology
        // (witness / outer ref) still gets its own fresh run so it is never
        // answered from a cache entry built over different axioms.
        if self
            .ontology
            .as_ref()
            .is_some_and(|own| Arc::ptr_eq(own, ontology))
        {
            return match self.realization_snapshot(ontology) {
                Ok(snapshot) if snapshot.satisfiable => Ok(Some(snapshot.individual_nodes.clone())),
                // Inconsistent ontology: same answer as the fresh-run path —
                // no merge map, the caller keeps its syntactic equality view.
                Ok(_) => Ok(None),
                // `Unknown` (timeout / resource abort) proves nothing; a failed
                // cache must not change the answer either. Keep the pre-existing
                // "no merge map" behaviour so callers fall back to syntax.
                Err(_) => Ok(None),
            };
        }
        let guard = read_lock(ontology, "core: reading ontology for tableau merge map")?;
        let mut tableau = self.tableau_factory.create_algorithm_instance(&guard)?;
        let state = tableau.run()?;
        if state != TableauState::Satisfiable {
            return Ok(None);
        }
        Ok(Some(tableau.snapshot().individual_nodes))
    }

    /// Check if the ontology is consistent.
    ///
    /// The answer is memoised against the ontology's version stamp: repeated
    /// calls without an intervening mutation return the stored answer instead
    /// of re-running the tableau (the miss is counted in
    /// [`ReasoningStatistics::consistency_checks`], the reuse in
    /// [`ReasoningStatistics::consistency_cache_hits`]).  Every mutator
    /// invalidates the memo via [`Reasoner::invalidate_query_caches`], and the
    /// version stamp additionally protects against mutations that bypass the
    /// reasoner (they still bump `Ontology::version`).
    pub fn is_consistent(&self) -> Result<bool> {
        let version = match &self.ontology {
            Some(ontology_ref) => read_lock(
                ontology_ref,
                "core: reading ontology version for consistency memo",
            )?
            .version(),
            // Empty ontology is consistent (cheap, nothing to memoise).
            None => return Ok(true),
        };

        if let Some((memo_version, value)) = *self
            .consistency_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            && memo_version == version
        {
            self.statistics
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .consistency_cache_hits += 1;
            return Ok(value);
        }

        let start = std::time::Instant::now();
        let value = self.compute_consistency()?;

        let mut memo = self
            .consistency_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *memo = Some((version, value));
        drop(memo);

        let mut statistics = self
            .statistics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        statistics.consistency_checks += 1;
        statistics.add_reasoning_time(start.elapsed());
        drop(statistics);

        Ok(value)
    }

    /// Run the pre-check + tableau consistency computation (uncached).
    fn compute_consistency(&self) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(ontology, "core: reading ontology for consistency check")?;

            // Pre-consistency check (fast detection of certain inconsistencies)
            log::info!("Running pre-consistency check");
            let mut pre_checker = crate::core::reasoner::PreConsistencyChecker::new(&ontology_ref)?;

            // If pre-check detects inconsistency, return early
            match pre_checker.check(&ontology_ref) {
                Err(_) => {
                    log::info!("Pre-consistency check detected inconsistency");
                    return Ok(false);
                }
                Ok(()) => {
                    log::info!("Pre-consistency check passed, proceeding to tableau");
                }
            }

            // PHASE 2: Tableau-based consistency checking (if pre-check passed)
            let mut tableau = self.tableau_factory.create_for_consistency(&ontology_ref)?;

            // Run the tableau algorithm
            let state = tableau.run()?;

            // Update statistics
            let _node_count = tableau.get_node_count();
            let _backtrack_count = tableau.get_backtrack_count();

            // A timed-out / resource-aborted tableau (`Unknown`) proves
            // nothing: surface an error instead of claiming consistency.
            match state {
                TableauState::Satisfiable => Ok(true),
                TableauState::Unsatisfiable => Ok(false),
                TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
            }
        } else {
            Ok(true) // Empty ontology is consistent
        }
    }

    /// Check if a class is satisfiable
    pub fn is_class_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        // owl:Nothing is always unsatisfiable by definition
        if let ClassExpression::Class(c) = class
            && c.is_nothing()
        {
            return Ok(false);
        }

        // Fast path: the classification result already proved this class
        // equivalent to owl:Nothing.
        if let Some(classif) = self.classification_service.cached_classification()
            && classif.unsat.contains(class)
        {
            return Ok(false);
        }

        if let Some(ontology) = &self.ontology {
            // Create a tableau for satisfiability checking
            let ontology_ref =
                read_lock(ontology, "core: reading ontology for satisfiability check")?;
            let _tableau = self.tableau_factory.create_for_consistency(&ontology_ref)?;

            // Create a test individual with the class to check
            let test_individual = crate::ontology::Individual::named(crate::ontology::IRI::new(
                "http://example.org/test#testIndividual",
            ));

            // Add the class assertion to test satisfiability
            let test_assertion = crate::ontology::ClassAssertionAxiom {
                id: 0, // Generate a proper ID
                class: class.clone(),
                individual: test_individual,
                annotations: Vec::new(),
            };

            // Create a temporary ontology with the test assertion
            let mut test_ontology =
                read_lock(ontology, "core: reading ontology to clone for test")?.clone();
            test_ontology.add_axiom(crate::ontology::Axiom::ClassAssertion(test_assertion));

            // Initialize a new tableau with the test ontology
            let mut test_tableau = self
                .tableau_factory
                .create_for_consistency(&test_ontology)?;

            // Run the tableau algorithm
            let state = test_tableau.run()?;

            // `Unknown` (timeout / resource abort) is not a negative answer.
            match state {
                TableauState::Satisfiable => Ok(true),
                TableauState::Unsatisfiable => Ok(false),
                TableauState::Unknown => Err(Error::reasoning("Tableau returned unknown result")),
            }
        } else {
            // In empty ontology, all classes are satisfiable (except owl:Nothing, already handled)
            Ok(true)
        }
    }

    /// Check if one class is subsumed by another
    pub fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        self.is_subclass_of(subclass, superclass)
    }

    /// Get superclasses of a class
    pub fn get_superclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_superclasses")?;
            let index = ontology.build_index();

            let mut superclasses: Vec<ClassExpression> = index.direct_superclasses(class).to_vec();
            for eq in index.equivalent_classes(class) {
                superclasses.push(eq.clone());
            }

            // Always add owl:Thing as superclass (unless the class is owl:Thing itself)
            let owl_thing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_thing(),
            ));
            if !self.classes_equivalent(class, &owl_thing).unwrap_or(false) {
                superclasses.push(owl_thing);
            }

            if direct {
                // Filter to only direct superclasses
                self.filter_direct_superclasses(superclasses)
            } else {
                // Use inference to find all inferred superclasses
                self.get_all_inferred_superclasses(class, superclasses)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Get subclasses of a class
    pub fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_subclasses")?;
            let index = ontology.build_index();

            let mut subclasses: Vec<ClassExpression> = index.direct_subclasses(class).to_vec();
            for eq in index.equivalent_classes(class) {
                subclasses.push(eq.clone());
            }

            // Always add owl:Nothing as subclass (unless the class is owl:Nothing itself)
            let owl_nothing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ));
            if !self
                .classes_equivalent(class, &owl_nothing)
                .unwrap_or(false)
            {
                subclasses.push(owl_nothing);
            }

            if direct {
                // Filter to only direct subclasses
                self.filter_direct_subclasses(subclasses)
            } else {
                // Use inference to find all inferred subclasses
                self.get_all_inferred_subclasses(class, subclasses)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Get equivalent classes
    pub fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_equivalent_classes",
            )?;
            let index = ontology.build_index();

            // Explicit equivalences from EquivalentClasses axioms
            let mut equivalent_classes: Vec<ClassExpression> =
                index.equivalent_classes(class).to_vec();

            // Check implicit equivalences via bidirectional subsumption
            // (drop the lock first to avoid holding it during recursive calls)
            drop(ontology);

            let all_classes = self.get_all_classes_in_ontology_internal()?;
            for other_class in all_classes {
                if !self
                    .classes_equivalent(&other_class, class)
                    .unwrap_or(false)
                    && !equivalent_classes.contains(&other_class)
                    && self.is_subclass_of(class, &other_class)?
                    && self.is_subclass_of(&other_class, class)?
                {
                    equivalent_classes.push(other_class);
                }
            }

            Ok(equivalent_classes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get instances of a class.
    ///
    /// Every call is counted in [`ReasoningStatistics::instance_queries`] and
    /// contributes its wall time to [`ReasoningStatistics::total_reasoning_time`];
    /// both surface through [`Reasoner::get_statistics`]. The query strategy is
    /// documented on [`Self::get_instances_inner`].
    pub fn get_instances(
        &mut self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<Individual>> {
        let start = std::time::Instant::now();
        let result = self.get_instances_inner(class, direct);
        let mut statistics = self
            .statistics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        statistics.instance_queries += 1;
        statistics.add_reasoning_time(start.elapsed());
        drop(statistics);
        result
    }

    /// Implementation of [`Reasoner::get_instances`] — the single-tableau
    /// query strategy documented below.
    ///
    /// Phase A rewrite — previously this delegated to
    /// [`ClassificationService::get_instances`], which loops over *every*
    /// individual and builds one full tableau per individual
    /// (`task_service.check_instance`), i.e. O(N) tableau runs per query.
    /// The new path is:
    ///
    /// 1. individuals come from the ontology signature,
    /// 2. each individual keeps the structural/datatype positive check
    ///    (`check_instance_with_datatype_reasoning`) as a fast positive proof,
    /// 3. whatever is not proven is evaluated against **one** cached
    ///    completed-tableau snapshot ([`Reasoner::realization_snapshot`]);
    ///    if that snapshot is unsatisfiable every individual is an instance
    ///    (vacuous truth),
    /// 4. the result is sorted and deduplicated.
    ///
    /// `direct` is accepted for API compatibility and ignored — matching the
    /// previous behaviour of `ClassificationService::get_instances`, whose
    /// `_direct` parameter was likewise ignored. That method is unchanged and
    /// remains available to other callers.
    fn get_instances_inner(
        &mut self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<Vec<Individual>> {
        let _ = direct;
        let Some(ontology_ref) = self.ontology.clone() else {
            return Ok(Vec::new());
        };

        let individuals = {
            let guard = read_lock(&ontology_ref, "core: reading ontology for get_instances")?;
            guard.signature()?.individuals.clone()
        };

        // ── owl:Thing / owl:Nothing fast paths (OWL API parity) ────────────
        // Every individual of the signature — named **and** anonymous,
        // including those that only occur in property assertions — is an
        // instance of `owl:Thing`, and nothing is an instance of
        // `owl:Nothing`. The tableau snapshot never labels a node with
        // `owl:Thing`, so without this fast path both answers came back
        // empty. `owl:Nothing` is answered *before* the inconsistent-ontology
        // vacuous-truth branch: the OWL API contract is an empty instance set.
        if let ClassExpression::Class(cls) = class {
            if cls.is_thing() {
                return Ok(Self::sort_and_dedup_individuals(individuals));
            }
            if cls.is_nothing() {
                return Ok(Vec::new());
            }
        }

        // Phase 1: structural / datatype positive proofs (no tableau).
        let mut proven: Vec<Individual> = Vec::new();
        let mut unresolved: Vec<Individual> = Vec::new();
        {
            let guard = read_lock(&ontology_ref, "core: reading ontology for get_instances")?;
            for individual in &individuals {
                match self
                    .classification_service
                    .check_instance_with_datatype_reasoning(individual, class, &guard)
                {
                    Ok(true) => proven.push(individual.clone()),
                    // Errors fall through to the snapshot evaluator, which is
                    // authoritative for the derived cases (same error policy
                    // as before: a failed structural check never fails the
                    // whole query).
                    _ => unresolved.push(individual.clone()),
                }
            }
        }

        if unresolved.is_empty() {
            return Ok(Self::sort_and_dedup_individuals(proven));
        }

        // Phase 2: one shared completed-tableau snapshot for the remainder.
        let snapshot = self.realization_snapshot(&ontology_ref)?;
        let mut instances = if snapshot.satisfiable {
            unresolved
                .into_iter()
                .filter(|individual| Self::snapshot_instance_of(&snapshot, individual, class))
                .collect()
        } else {
            // Inconsistent ontology: every individual is an instance (vacuous
            // truth) — same policy as `is_instance_of_ontology`.
            unresolved
        };
        instances.append(&mut proven);
        Ok(Self::sort_and_dedup_individuals(instances))
    }

    /// Sort individuals by their Display form and drop duplicates (an
    /// individual can be reachable both through the signature and through
    /// nominal merging).
    fn sort_and_dedup_individuals(mut individuals: Vec<Individual>) -> Vec<Individual> {
        individuals.sort_by_key(|a| a.to_string());
        individuals.dedup_by(|a, b| a.to_string() == b.to_string());
        individuals
    }

    /// Run the realisation tableau **once** for `ontology_ref` and return the
    /// cached read-only [`TableauSnapshot`] of the completed tableau.
    ///
    /// The cache lives on the reasoner and is dropped by every mutating
    /// method (see [`Reasoner::invalidate_query_caches`]). Returns `Err`
    /// when the tableau ends `Unknown` (timeout / resource abort): an
    /// interrupted run proves nothing, so callers must surface an error
    /// instead of a silently wrong answer.
    pub fn realization_snapshot(
        &self,
        ontology_ref: &OntologyRef,
    ) -> Result<std::sync::Arc<TableauSnapshot>> {
        // Fast path: snapshot cached by a previous query.
        if let Some(cached) = self
            .snapshot_cache
            .lock()
            .map_err(|_| Error::reasoning("Realisation snapshot cache poisoned"))?
            .as_ref()
        {
            return Ok(cached.clone());
        }

        let built = {
            let guard = read_lock(
                ontology_ref,
                "core: reading ontology for realization snapshot",
            )?;
            let mut instance = self.tableau_factory.create_algorithm_instance(&guard)?;
            let state = instance.run()?;
            if state == TableauState::Unknown {
                return Err(Error::reasoning("Tableau returned unknown result"));
            }
            std::sync::Arc::new(instance.snapshot())
        };

        // Publish; if another thread won the race in the meantime, keep its
        // (equivalent) snapshot so all callers share one Arc.
        let mut slot = self
            .snapshot_cache
            .lock()
            .map_err(|_| Error::reasoning("Realisation snapshot cache poisoned"))?;
        Ok(match slot.as_ref() {
            Some(existing) => existing.clone(),
            None => {
                *slot = Some(built.clone());
                built
            }
        })
    }

    /// Build a snapshot of `ontology_ref` *without* touching the reasoner's
    /// one-shot cache — used when the ontology to reason over is not the
    /// loaded one (e.g. `check_entailment` witness ontologies).
    fn build_fresh_snapshot(
        &self,
        ontology_ref: &OntologyRef,
    ) -> Result<std::sync::Arc<TableauSnapshot>> {
        let guard = read_lock(ontology_ref, "core: reading ontology for fresh snapshot")?;
        let mut instance = self.tableau_factory.create_algorithm_instance(&guard)?;
        let state = instance.run()?;
        if state == TableauState::Unknown {
            return Err(Error::reasoning("Tableau returned unknown result"));
        }
        Ok(std::sync::Arc::new(instance.snapshot()))
    }

    /// Drop the cached realisation snapshot and the memoised consistency
    /// answer. Must be called by every method that mutates the loaded ontology
    /// (axiom add/remove, ontology reload, SPARQL `INSERT`/`DELETE DATA`,
    /// `ABox` materialisation).
    fn invalidate_query_caches(&self) {
        if let Ok(mut slot) = self.snapshot_cache.lock() {
            *slot = None;
        }
        if let Ok(mut memo) = self.consistency_cache.lock() {
            *memo = None;
        }
    }

    /// Check if an individual is an instance of a class expression
    pub fn is_instance_of(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        if let Some(ontology_ref) = &self.ontology {
            self.is_instance_of_ontology(individual, class, ontology_ref)
        } else {
            Ok(false)
        }
    }

    /// Instance check against a specific ontology (used by `check_entailment`
    /// when the ontology to reason over differs from the loaded one).
    fn is_instance_of_ontology(
        &self,
        individual: &Individual,
        class: &ClassExpression,
        ontology_ref: &OntologyRef,
    ) -> Result<bool> {
        // ── owl:Thing / owl:Nothing fast paths (OWL API parity) ────────────
        // Every individual is an instance of `owl:Thing`; none is an instance
        // of `owl:Nothing` (the tableau never labels a node `owl:Thing`, and
        // the vacuous-truth branch of an inconsistent ontology must not turn
        // `hasType(x, owl:Nothing)` into `true`).
        if let ClassExpression::Class(cls) = class {
            if cls.is_thing() {
                return Ok(true);
            }
            if cls.is_nothing() {
                return Ok(false);
            }
        }
        {
            let ontology = read_lock(ontology_ref, "core: reading ontology for is_instance_of")?;
            if self
                .classification_service
                .check_instance_with_datatype_reasoning(individual, class, &ontology)?
            {
                return Ok(true);
            }
        } // drop the read guard before running (or reusing) the tableau

        // Fallback: consult the *materialized* tableau closure. The structural checker
        // above only sees asserted axioms, so it misses memberships that follow from a
        // derived edge — e.g. `A ≡ ≥1 R.{b}`, `A(a)` materialises `R(a,b)`, making `a`
        // an instance of `∃R.{b}` even though no `R(a,b)` axiom is asserted. Evaluating
        // the queried expression against a completed tableau snapshot covers exactly
        // those derived cases; constructors the evaluator cannot decide return `false`
        // (the pre-existing conservative behaviour).
        //
        // Phase A: when the caller's ontology *is* the reasoner's own (Arc identity),
        // reuse the shared one-shot snapshot instead of running a fresh tableau per
        // call.
        let snapshot = if self
            .ontology
            .as_ref()
            .is_some_and(|own| Arc::ptr_eq(own, ontology_ref))
        {
            self.realization_snapshot(ontology_ref)?
        } else {
            self.build_fresh_snapshot(ontology_ref)?
        };
        if !snapshot.satisfiable {
            // Inconsistent ontology: every instance query is vacuously true.
            return Ok(true);
        }
        Ok(Self::snapshot_instance_of(&snapshot, individual, class))
    }

    /// Evaluate a class expression against a completed tableau snapshot for a given
    /// individual (see [`Reasoner::is_instance_of_ontology`] for when this is used).
    fn snapshot_instance_of(
        snapshot: &TableauSnapshot,
        individual: &Individual,
        class: &ClassExpression,
    ) -> bool {
        let Some(node) = snapshot.node_of(individual) else {
            return false;
        };
        Self::snapshot_node_instance_of(snapshot, node, class)
    }

    /// Evaluate a class expression against a completed tableau snapshot *node*.
    /// Handles the constructors that are not answerable from the asserted axioms
    /// alone — chiefly nominal fillers `{b}` whose witnessing edge is derived.
    fn snapshot_node_instance_of(
        snapshot: &TableauSnapshot,
        node: usize,
        class: &ClassExpression,
    ) -> bool {
        match class {
            ClassExpression::Class(_) => snapshot.node_labeled_as(node, class),
            ClassExpression::ObjectIntersectionOf(ops) => ops
                .iter()
                .all(|op| Self::snapshot_node_instance_of(snapshot, node, op)),
            ClassExpression::ObjectUnionOf(ops) => ops
                .iter()
                .any(|op| Self::snapshot_node_instance_of(snapshot, node, op)),
            ClassExpression::ObjectOneOf(individuals) => individuals
                .iter()
                .any(|ind| snapshot.node_of(ind) == Some(node)),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let role = property.to_string();
                snapshot.edges.iter().any(|(f, t, r)| {
                    *f == node
                        && r == &role
                        && Self::snapshot_node_instance_of(snapshot, *t, filler)
                })
            }
            ClassExpression::ObjectHasValue { property, value } => {
                let role = property.to_string();
                snapshot
                    .node_of(value)
                    .is_some_and(|target| snapshot.has_edge(node, target, &role))
            }
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                let role = property.to_string();
                let count = snapshot
                    .edges
                    .iter()
                    .filter(|(f, t, r)| {
                        *f == node
                            && r == &role
                            && Self::snapshot_node_instance_of(snapshot, *t, filler)
                    })
                    .count();
                count >= *cardinality as usize
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                let role = property.to_string();
                let validator =
                    crate::core::reasoner::datatype_validation::DatatypeValidator::new();
                snapshot.data_edges.iter().any(|(f, r, v)| {
                    *f == node
                        && r == &role
                        && validator
                            .literal_satisfies_data_range(v, filler)
                            .unwrap_or(false)
                })
            }
            // Constructors the snapshot evaluator cannot decide (universal,
            // cardinality >1 over non-nominal fillers, complements, data ranges with
            // facets…) stay conservative: return false so the caller's structural
            // check remains authoritative.
            _ => false,
        }
    }

    /// Get types of an individual
    pub fn get_types(&self, individual: &Individual, direct: bool) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_types")?;
            let mut types = Vec::new();

            // Look for explicit class assertions. Identity is compared as a
            // whole `Individual`: named individuals by IRI, anonymous ones by
            // their stable `_:id` key (the previous `iri()`-based comparison
            // matched `None == None`, i.e. *every* anonymous individual
            // against *any* anonymous query).
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ClassAssertion(class_assertion) = axiom
                    && class_assertion.individual == *individual
                {
                    types.push(class_assertion.class.clone());
                }
            }

            // Derive types from *defined* classes: the named class operand of an
            // `EquivalentClasses` axiom whose other operand is a complex class
            // expression (nominals, datatype restrictions, role restrictions, …).
            // The assertion scan above only sees `ClassAssertion` axioms, so
            // memberships that follow from a class *definition* — `A ≡ {a}`,
            // `Adult ≡ ∃age.(int ⊓ [≥ 18])` — would otherwise be invisible here
            // even though `get_instances` / `is_instance_of` already compute them
            // through the classification service (locked in by the C2
            // differential fixtures diff_005 / diff_012).
            //
            // The candidate set is deliberately bounded to *defined* classes
            // rather than every named class in the signature: each membership
            // check scans the axioms, so an unbounded scan would cost
            // O(n_classes × n_axioms) per query (ORE-sized ontologies carry
            // ~10^5 classes).  Classes introduced only by `SubClassOf` /
            // `ClassAssertion` are already covered by the assertion scan plus
            // the superclass closure below.
            let mut candidates: Vec<ClassExpression> = Vec::new();
            let mut seen_candidates: std::collections::HashSet<crate::ontology::IRI> =
                std::collections::HashSet::new();
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equivalent) = axiom
                    && equivalent
                        .classes
                        .iter()
                        .any(|operand| !matches!(operand, ClassExpression::Class(_)))
                {
                    for operand in &equivalent.classes {
                        if let ClassExpression::Class(cls) = operand
                            && seen_candidates.insert(cls.iri.clone())
                        {
                            candidates.push(ClassExpression::Class(cls.clone()));
                        }
                    }
                }
            }
            // Derived defined-class memberships are candidates for the result
            // set; a failing derivation must not fail the whole query — the
            // asserted types above remain authoritative (same error policy as
            // `get_instances`).
            for candidate in &candidates {
                if let Ok(true) = self
                    .classification_service
                    .check_instance_with_datatype_reasoning(individual, candidate, &ontology)
                {
                    types.push(candidate.clone());
                }
            }

            if direct {
                // `owl:Thing` is a type of every individual. It is inserted
                // *before* the most-specific filter, which drops it exactly
                // when the individual has a proper type (the filter removes a
                // type that has a strict subclass in the result set — and
                // `owl:Thing` has one whenever any other type is present).
                let thing = ClassExpression::Class(crate::ontology::Class::thing());
                if !types.contains(&thing) {
                    types.push(thing);
                }
                // Most-specific filtering for `direct = true` (CR-008 MAJOR #1):
                // a type is dropped when the result set also contains one of its
                // *strict* subclasses — e.g. `B ≡ {a}`, `A ⊑ B`, `A(x)` yields
                // `{A, B}` unfiltered although `B ⊒ A`, so the direct answer is
                // `{A}` only.  Mirrors the OWL API surface filter at
                // `reasoner_api/mod.rs` (`get_types`): `other` is a strict
                // subclass of `t` iff `other ⊑ t` but not `t ⊑ other`
                // (equivalents are kept, matching `collect_strict_subclasses`).
                //
                // CR-011 M1 (perf): the subclass relation is evaluated from ONE
                // incrementally-maintained axiom index plus a per-query upward
                // closure per distinct type — O(types × closure) instead of
                // O(types²) `is_subclass_of` calls, each of which used to fall
                // through to a full `build_index()` rebuild on every `false`
                // (measured 3/34/120 ms per call at 2 k/20 k/60 k axioms).
                // Semantics are unchanged: `closure_subsumes` mirrors
                // `is_subclass_of_ontology`'s decision order exactly.
                // Error policy: an unavailable classification falls back to the
                // index + built-in semantics (conservative, as before —
                // `is_subclass_of` errors were already treated as `false`).
                let classif = self.classification_service.cached_classification();
                let index = ontology.axiom_index();
                let mut ancestors: HashMap<
                    ClassExpression,
                    std::collections::HashSet<ClassExpression>,
                > = HashMap::with_capacity(types.len());
                for t in &types {
                    ancestors
                        .entry(t.clone())
                        .or_insert_with(|| Self::ancestor_closure(index, t));
                }
                let mut most_specific = Vec::with_capacity(types.len());
                for t in &types {
                    let has_strict_sub_type = types.iter().any(|other| {
                        other != t
                            && self.closure_subsumes(classif, &ancestors, other, t)
                            && !self.closure_subsumes(classif, &ancestors, t, other)
                    });
                    if !has_strict_sub_type {
                        most_specific.push(t.clone());
                    }
                }
                types = most_specific;
            } else {
                // Also add superclasses of the direct types
                let mut all_types = types.clone();
                for direct_type in &types {
                    let superclasses = self.get_superclasses(direct_type, false)?;
                    all_types.extend(superclasses);
                }
                // `owl:Thing` is a type of every individual. Appended after
                // the superclass extension so `get_superclasses(owl:Thing)`
                // is never queried as if it were a subtype of something.
                let thing = ClassExpression::Class(crate::ontology::Class::thing());
                if !all_types.contains(&thing) {
                    all_types.push(thing);
                }
                types = all_types;
            }

            // Remove duplicates
            types.sort_by_key(|c| format!("{c:?}"));
            types.dedup_by_key(|c| format!("{c:?}"));

            Ok(types)
        } else {
            Ok(Vec::new())
        }
    }

    // ── Materialised property-value closure (OWLAPI / HermiT parity) ─────────

    /// Invert a tableau edge-role Display form: `R` → `inv(R)`,
    /// `inv(R)` → `R`.
    fn invert_role_display(role: &str) -> String {
        match role
            .strip_prefix("inv(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            Some(inner) => inner.to_string(),
            None => format!("inv({role})"),
        }
    }

    /// Transitive sub-property closure of `property` over the ontology's
    /// `SubObjectPropertyOf` / `EquivalentObjectProperties` /
    /// `InverseObjectProperties` axioms, expressed as tableau edge-role Display
    /// forms (`IRI` or `inv(IRI)`).
    ///
    /// Every axiom contributes both its *forward* and its *inverse* reading:
    /// `R ⊑ S` also yields `inv(R) ⊑ inv(S)`, and `InverseObjectProperties(P,Q)`
    /// (i.e. `Q ≡ inv(P)`) yields `Q ⊑ inv(P)` plus `inv(P) ⊑ Q`. A query for
    /// `inv(R)` therefore matches edges materialised under `inv(R)`, under a
    /// sub-property `inv(S)`, or under a property `Q` declared inverse to `R`.
    ///
    /// Property chains are ignored on the inverse side (a chain has no inverse
    /// expression); their composite edges are materialised under the *super*
    /// property, which the closure already contains.
    fn object_property_subroles(
        ontology: &crate::ontology::Ontology,
        property: &ObjectPropertyExpression,
    ) -> HashSet<String> {
        fn inverse(p: &ObjectPropertyExpression) -> Option<ObjectPropertyExpression> {
            match p {
                ObjectPropertyExpression::ObjectProperty(prop) => Some(
                    ObjectPropertyExpression::InverseObjectProperty(prop.clone()),
                ),
                ObjectPropertyExpression::InverseObjectProperty(prop) => {
                    Some(ObjectPropertyExpression::ObjectProperty(prop.clone()))
                }
                ObjectPropertyExpression::PropertyChain(_) => None,
            }
        }

        // Direct `sub ⊑ super` pairs collected from the axioms.
        let mut direct: Vec<(ObjectPropertyExpression, ObjectPropertyExpression)> = Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::SubObjectPropertyOf(a) => {
                    direct.push((a.sub_property.clone(), a.super_property.clone()));
                }
                crate::ontology::Axiom::EquivalentObjectProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                direct.push((p1.clone(), p2.clone()));
                            }
                        }
                    }
                }
                crate::ontology::Axiom::InverseObjectProperties(a) => {
                    // `Q ≡ inv(P)` gives both directions; the inverse expansion
                    // below derives `inv(Q) ≡ P` from them.
                    let (p, q) = (&a.property1, &a.property2);
                    if let Some(inv_p) = inverse(p) {
                        direct.push((q.clone(), inv_p.clone()));
                        direct.push((inv_p, q.clone()));
                    }
                }
                _ => {}
            }
        }

        // Every `sub ⊑ super` also implies `inv(sub) ⊑ inv(super)`.
        let mut all_pairs = direct.clone();
        for (sub, sup) in &direct {
            if let (Some(inv_sub), Some(inv_sup)) = (inverse(sub), inverse(sup)) {
                all_pairs.push((inv_sub, inv_sup));
            }
        }

        let mut sub_of: HashMap<String, HashSet<String>> = HashMap::new();
        for (sub, sup) in all_pairs {
            sub_of
                .entry(sup.to_string())
                .or_default()
                .insert(sub.to_string());
        }

        // Reachability from the requested property downwards.
        let mut closure: HashSet<String> = HashSet::new();
        let mut stack = vec![property.to_string()];
        while let Some(current) = stack.pop() {
            if !closure.insert(current.clone()) {
                continue;
            }
            if let Some(subs) = sub_of.get(&current) {
                stack.extend(subs.iter().cloned());
            }
        }
        closure
    }

    /// Edge-role matchers for an object-property value query.
    ///
    /// Returns `(outgoing, incoming)`:
    /// * `outgoing` — role labels `E` for which an edge `x --E--> y` *out of*
    ///   the queried individual entails `property(x, y)`;
    /// * `incoming` — role labels `E` for which an edge `y --E--> x` *into* the
    ///   queried individual entails `property(x, y)` (i.e. `inv(E) ⊑ property`),
    ///   so an edge `b --inv(R)--> a` is read as `R(a, b)`.
    fn object_property_edge_roles(
        ontology: &crate::ontology::Ontology,
        property: &ObjectPropertyExpression,
    ) -> (HashSet<String>, HashSet<String>) {
        let closure = Self::object_property_subroles(ontology, property);
        let incoming = closure
            .iter()
            .map(|role| Self::invert_role_display(role))
            .collect();
        (closure, incoming)
    }

    /// Transitive sub-property closure of a *data* property over the ontology's
    /// `SubDataPropertyOf` / `EquivalentDataProperties` axioms, as edge-role
    /// Display forms (data roles are never inverted).
    fn data_property_subroles(
        ontology: &crate::ontology::Ontology,
        property: &DataPropertyExpression,
    ) -> HashSet<String> {
        let mut direct: Vec<(DataPropertyExpression, DataPropertyExpression)> = Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::SubDataPropertyOf(a) => {
                    direct.push((a.sub_property.clone(), a.super_property.clone()));
                }
                crate::ontology::Axiom::EquivalentDataProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                direct.push((p1.clone(), p2.clone()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut sub_of: HashMap<String, HashSet<String>> = HashMap::new();
        for (sub, sup) in direct {
            sub_of
                .entry(sup.to_string())
                .or_default()
                .insert(sub.to_string());
        }

        let mut closure: HashSet<String> = HashSet::new();
        let mut stack = vec![property.to_string()];
        while let Some(current) = stack.pop() {
            if !closure.insert(current.clone()) {
                continue;
            }
            if let Some(subs) = sub_of.get(&current) {
                stack.extend(subs.iter().cloned());
            }
        }
        closure
    }

    /// Named individuals of the completed tableau, indexed by node id.
    ///
    /// Successor nodes without a nominal label (anonymous fillers created by
    /// existential expansion) get no entry, so callers that return only named
    /// individuals skip them automatically. Individuals merged by
    /// `SameIndividual` / functionality / `HasKey` share one node and each keep
    /// their own entry.
    fn named_individuals_by_node(snapshot: &TableauSnapshot) -> HashMap<usize, Vec<Individual>> {
        let mut by_node: HashMap<usize, Vec<Individual>> = HashMap::new();
        for (display, &node) in &snapshot.individual_nodes {
            // Anonymous individuals display as `_:id` and are not reconstructible
            // as named values.
            if display.starts_with("_:") {
                continue;
            }
            by_node
                .entry(node)
                .or_default()
                .push(Individual::named(crate::ontology::IRI::new(display)));
        }
        by_node
    }

    /// Get object property values for an individual.
    ///
    /// The answer is the union of:
    ///
    /// 1. the asserted `ObjectPropertyAssertion` axioms (fast path — runs no
    ///    tableau), and
    /// 2. the **materialised closure** of the completed tableau snapshot
    ///    ([`Reasoner::realization_snapshot`]), which is authoritative for every
    ///    value the reasoner can derive: inverse edges, sub-property edges,
    ///    transitive / property-chain derived edges and equivalent-property
    ///    materialisations all appear there.
    ///
    /// Edge roles are matched through the sub-property closure in both
    /// directions (see [`Reasoner::object_property_edge_roles`]), so an edge
    /// `b --S--> a` with `S ⊑ R` answers an `R` query and an edge
    /// `b --inv(R)--> a` answers both the `R` and the `inv(R)` query.
    ///
    /// Only **named** individuals are returned: anonymous tableau successors
    /// (no IRI) are skipped. Results are sorted by Display form and
    /// deduplicated.
    ///
    /// Error/consistency policy:
    /// * an inconsistent ontology (`snapshot.satisfiable == false`) keeps the
    ///   asserted values only — vacuous truth would make every individual a
    ///   value of every property, which is not a useful API answer;
    /// * a tableau that ends `Unknown` (timeout / resource abort) propagates as
    ///   `Err`: an interrupted run proves nothing (same policy as
    ///   [`Reasoner::get_instances`]).
    pub fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<Individual>> {
        let Some(ontology_ref) = self.ontology.clone() else {
            return Ok(Vec::new());
        };

        let mut values: Vec<Individual> = Vec::new();
        let roles = {
            let ontology = read_lock(
                &ontology_ref,
                "core: reading ontology for get_object_property_values",
            )?;
            // Fast path: explicit object property assertions.
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = axiom
                    && prop_assertion.source == *individual
                    && self.object_properties_equivalent(&prop_assertion.property, property)?
                {
                    values.push(prop_assertion.target.clone());
                }
            }
            Self::object_property_edge_roles(&ontology, property)
        }; // read guard dropped before the tableau run

        let snapshot = self.realization_snapshot(&ontology_ref)?;
        if snapshot.satisfiable {
            let by_node = Self::named_individuals_by_node(&snapshot);
            let (outgoing, incoming) = roles;
            if let Some(node) = snapshot.node_of(individual) {
                for (from, to, role) in &snapshot.edges {
                    if *from == node && outgoing.contains(role) {
                        values.extend(by_node.get(to).into_iter().flatten().cloned());
                    } else if *to == node && incoming.contains(role) {
                        values.extend(by_node.get(from).into_iter().flatten().cloned());
                    }
                }
            }
        }

        // Remove duplicates (Display form distinguishes named from anonymous).
        values.sort_by_key(|i| i.to_string());
        values.dedup_by_key(|i| i.to_string());

        Ok(values)
    }

    /// Get data property values for an individual.
    ///
    /// Same two-source policy as [`Reasoner::get_object_property_values`]:
    /// asserted `DataPropertyAssertion` axioms (fast path) unioned with the
    /// completed tableau's `data_edges` (which carry every derived value —
    /// sub-property and equivalent-property materialisations included), matched
    /// through the data sub-property closure
    /// ([`Reasoner::data_property_subroles`]). Results are deduplicated by
    /// literal value.
    ///
    /// The inconsistency / timeout policy matches
    /// [`Reasoner::get_object_property_values`].
    pub fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<Vec<crate::ontology::Literal>> {
        let Some(ontology_ref) = self.ontology.clone() else {
            return Ok(Vec::new());
        };

        let mut values: Vec<crate::ontology::Literal> = Vec::new();
        let roles = {
            let ontology = read_lock(
                &ontology_ref,
                "core: reading ontology for get_data_property_values",
            )?;
            // Fast path: explicit data property assertions.
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::DataPropertyAssertion(prop_assertion) = axiom
                    && prop_assertion.individual == *individual
                    && self.data_properties_equivalent(&prop_assertion.property, property)?
                {
                    values.push(prop_assertion.value.clone());
                }
            }
            Self::data_property_subroles(&ontology, property)
        }; // read guard dropped before the tableau run

        let snapshot = self.realization_snapshot(&ontology_ref)?;
        if snapshot.satisfiable
            && let Some(node) = snapshot.node_of(individual)
        {
            for (from, role, literal) in &snapshot.data_edges {
                if *from == node && roles.contains(role) {
                    values.push(literal.clone());
                }
            }
        }

        // Remove duplicates
        values.sort_by_key(|l| format!("{l:?}"));
        values.dedup_by_key(|l| format!("{l:?}"));

        Ok(values)
    }

    /// All materialised `(source, target)` pairs for `property`.
    ///
    /// Union of the asserted `ObjectPropertyAssertion` axioms (both endpoints
    /// named) and the completed tableau's edges, including inverse-labelled
    /// edges — `b --inv(R)--> a` yields `(a, b)` for a query on `R`. Only
    /// **named** individuals are returned (anonymous successors are skipped).
    /// Output is sorted by `(source, target)` Display form and deduplicated, so
    /// it is deterministic across runs.
    ///
    /// The inconsistency / timeout policy matches
    /// [`Reasoner::get_object_property_values`].
    pub fn get_object_property_instances(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<(Individual, Individual)>> {
        let Some(ontology_ref) = self.ontology.clone() else {
            return Ok(Vec::new());
        };

        let mut pairs: Vec<(Individual, Individual)> = Vec::new();
        let roles = {
            let ontology = read_lock(
                &ontology_ref,
                "core: reading ontology for get_object_property_instances",
            )?;
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = axiom
                    && self.object_properties_equivalent(&prop_assertion.property, property)?
                    && prop_assertion.source.is_named()
                    && prop_assertion.target.is_named()
                {
                    pairs.push((prop_assertion.source.clone(), prop_assertion.target.clone()));
                }
            }
            Self::object_property_edge_roles(&ontology, property)
        }; // read guard dropped before the tableau run

        let snapshot = self.realization_snapshot(&ontology_ref)?;
        if snapshot.satisfiable {
            let by_node = Self::named_individuals_by_node(&snapshot);
            let (outgoing, incoming) = roles;
            for (from, to, role) in &snapshot.edges {
                if outgoing.contains(role) {
                    if let (Some(sources), Some(targets)) = (by_node.get(from), by_node.get(to)) {
                        for source in sources {
                            for target in targets {
                                pairs.push((source.clone(), target.clone()));
                            }
                        }
                    }
                } else if incoming.contains(role)
                    && let (Some(sources), Some(targets)) = (by_node.get(to), by_node.get(from))
                {
                    for source in sources {
                        for target in targets {
                            pairs.push((source.clone(), target.clone()));
                        }
                    }
                }
            }
        }

        pairs.sort_by(|a, b| {
            a.0.to_string()
                .cmp(&b.0.to_string())
                .then_with(|| a.1.to_string().cmp(&b.1.to_string()))
        });
        pairs.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        Ok(pairs)
    }

    /// Whether the one-shot realisation snapshot is already cached.
    ///
    /// Lets callers (e.g. `precompute_inferences`) report a warm
    /// property-assertion cache without exposing the snapshot itself.
    #[must_use]
    pub fn has_realization_snapshot(&self) -> bool {
        self.snapshot_cache.lock().is_ok_and(|slot| slot.is_some())
    }

    /// Get all object property assertions for a given property expression.
    pub fn get_object_property_assertions(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Result<Vec<(Individual, Individual)>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for object property assertions",
            )?;
            let mut assertions = Vec::new();
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::ObjectPropertyAssertion(prop_assertion) = axiom
                    && self.object_properties_equivalent(&prop_assertion.property, property)?
                {
                    assertions.push((prop_assertion.source.clone(), prop_assertion.target.clone()));
                }
            }
            Ok(assertions)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all data property assertions for a given property expression.
    pub fn get_data_property_assertions(
        &self,
        property: &DataPropertyExpression,
    ) -> Result<Vec<(Individual, crate::ontology::Literal)>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for data property assertions",
            )?;
            let mut assertions = Vec::new();
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::DataPropertyAssertion(prop_assertion) = axiom
                    && self.data_properties_equivalent(&prop_assertion.property, property)?
                {
                    assertions.push((
                        prop_assertion.individual.clone(),
                        prop_assertion.value.clone(),
                    ));
                }
            }
            Ok(assertions)
        } else {
            Ok(Vec::new())
        }
    }

    /// Classify the ontology (compute class hierarchy).
    ///
    /// When the ontology conforms to the OWL 2 EL profile **and** the
    /// configured `target_profile` permits it (`Auto` or `EL`), the faster
    /// `ELReasoner` is used automatically, giving up to 100× speedup over the
    /// full tableau algorithm.  Falls back to the general classification
    /// service when the profile is not EL, when an explicit `DL`/`QL`/`RL`
    /// target profile is configured, whenever the EL fast path fails for any
    /// reason (never propagates an EL-only error), and whenever the EL result
    /// is incomplete — i.e. does not key the hierarchy by every signature
    /// class plus `owl:Thing` (B1).
    ///
    /// The classification result is memoised: a call served from the
    /// classification cache returns without re-running anything and is not
    /// counted in [`ReasoningStatistics::classification_runs`].  The statistics
    /// of a real run are folded into the shared totals returned by
    /// [`Reasoner::get_statistics`].
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        // Memoised: the classification service stores the result of every run
        // and invalidates it on each ontology mutation.
        if let Some(cached) = self.classification_service.cached_classification() {
            return Ok(cached.clone());
        }

        let mut statistics = ReasoningStatistics::new();
        statistics.classification_runs = 1;
        let result = self.classify_with_stats(&mut statistics);
        self.merge_statistics(&statistics);
        result
    }

    /// Body of [`Reasoner::classify`] — accumulates into the caller's
    /// statistics value, which [`Reasoner::classify`] merges afterwards.
    fn classify_with_stats(
        &mut self,
        statistics: &mut ReasoningStatistics,
    ) -> Result<ClassificationResult> {
        use crate::profiles::ProfileValidator;

        if let Some(ontology) = self.ontology.clone() {
            // ── EL profile short-circuit ─────────────────────────────────────
            // Only honour the EL fast path when the configured target profile
            // allows it: an explicit DL/QL/RL target must always run the
            // general classification service.  `Auto`/`EL` keep the EL
            // detection below.
            let profile_allows_el = matches!(
                self.config.reasoning.target_profile,
                crate::config::OWLProfile::Auto | crate::config::OWLProfile::EL
            );

            // Detect the profile with a read-lock and avoid a full tableau run
            // when the ontology is EL-conforming.
            let is_el = profile_allows_el && {
                let ont = read_lock(&ontology, "classify: EL profile detection")?;
                let validator = crate::profiles::el::ELValidator::new();
                validator.validate(&ont).is_ok_and(|r| r.conforms)
            };

            if is_el {
                // The EL reasoner's worklist completion is O(N³) on deep
                // hierarchies, which blows up on 1000-class chains.  When
                // absorption classification is enabled and the EL ontology uses
                // only constructs the absorption + dependency-graph path is
                // complete for (told subsumption, role-successor / domain /
                // range / conjunction GCIs, equivalents — no role hierarchy,
                // no transitive roles, no nominals / self / cardinalities), we
                // route it through the (linear) A8 classification path instead.
                let use_absorption = self.config.reasoning.enable_absorption_classification && {
                    let ont = read_lock(&ontology, "classify: A8 EL coverage")?;
                    el_covered_by_absorption(&ont)
                };
                if use_absorption {
                    log::info!(
                        "EL profile detected: using A8 absorption path (deterministic EL fragment)"
                    );
                    return self.classification_service.classify(
                        &ontology,
                        statistics,
                        &mut self.cache_manager,
                    );
                }

                log::info!("EL profile detected: using ELReasoner (100x speedup)");
                // Run the EL fast path in a small scope: the read-lock on the
                // ontology is released before `self` is mutated, and the
                // reasoner is only handed back to `self` on success — a
                // failure can never leave a half-initialised reasoner cached.
                let el_start = std::time::Instant::now();
                let el_attempt = {
                    let ont = read_lock(&ontology, "classify: EL")?;
                    let mut el_reasoner =
                        crate::profiles::el_reasoner::ELReasoner::new(self.config.clone());
                    let outcome = match el_reasoner.initialize(&ont) {
                        Ok(()) => el_reasoner.classify(),
                        Err(err) => Err(err),
                    };
                    outcome.map(|result| (el_reasoner, result))
                };
                // The EL attempt does its own work outside the classification
                // service, so record its wall time explicitly (all exit paths).
                let el_elapsed = el_start.elapsed();

                match el_attempt {
                    Ok((el_reasoner, mut result)) => {
                        statistics.add_reasoning_time(el_elapsed);
                        // Completeness guard + metadata parity (B1): an EL
                        // result that does not cover the ontology's signature
                        // (or `owl:Thing`) is incomplete — never cache or
                        // serve it, fall through to the general classification
                        // below instead.  When it *is* complete, the metadata
                        // the general path always sets (`ontology_iri`, the
                        // object/data property lists used by the HermiT-style
                        // writer) is filled in here, so an Auto run produces
                        // the same document as a DL run.  With the EL
                        // hierarchy writer covering every signature class the
                        // completeness check never fires; it is the safety net
                        // that keeps the fast path from ever returning a
                        // partial hierarchy.
                        if !finalise_el_result(&ontology, &mut result) {
                            log::warn!(
                                "EL classification is incomplete (missing signature classes); \
                                 falling back to general classification"
                            );
                            self.el_reasoner = None;
                        } else {
                            // Store for future use, cache the result, and make
                            // the query fast-paths reuse it.
                            self.el_reasoner = Some(Box::new(el_reasoner));
                            self.classification_service
                                .store_classification(result.clone());
                            self.cache_manager
                                .store_classification_result(&ontology, result.clone());
                            return Ok(result);
                        }
                    }
                    Err(err) => {
                        statistics.add_reasoning_time(el_elapsed);
                        // The memory guard abort is NOT an EL-translation
                        // problem: retrying the (heavier) general path under
                        // memory pressure would only make things worse, so it
                        // propagates instead of falling back.
                        if err.is_memory_limit() {
                            return Err(err);
                        }
                        // The EL profile validator and the EL reasoner's
                        // normal form are not in lock-step (e.g. constructs the
                        // profile admits that `ELReasoner` cannot translate).
                        // Never propagate: log, clear any cached EL reasoner,
                        // and fall through to the general tableau path below.
                        log::warn!(
                            "EL fast path failed ({err}); falling back to general classification"
                        );
                        self.el_reasoner = None;
                    }
                }
            }

            // ── General tableau classification ───────────────────────────────
            self.classification_service
                .classify(&ontology, statistics, &mut self.cache_manager)
        } else {
            Err(Error::ontology_parsing(
                "No ontology loaded for classification",
            ))
        }
    }

    /// Realize the ontology (compute instance relationships).
    ///
    /// The statistics produced by the realisation run are folded into the
    /// shared totals returned by [`Reasoner::get_statistics`].
    pub fn realize(&mut self) -> Result<RealisationResult> {
        if let Some(ontology) = self.ontology.clone() {
            let mut statistics = ReasoningStatistics::new();
            let result = self.classification_service.realize(
                &ontology,
                &mut statistics,
                &mut self.cache_manager,
            );
            self.merge_statistics(&statistics);
            result
        } else {
            Err(Error::ontology_parsing(
                "No ontology loaded for realisation",
            ))
        }
    }

    /// Check if an axiom is entailed by the ontology
    pub fn check_entailment(
        &mut self,
        axiom: &crate::ontology::Axiom,
        ontology: &Arc<RwLock<crate::ontology::Ontology>>,
        stats: &mut ReasoningStatistics,
    ) -> Result<bool> {
        // Syntactic entailment fast path: an axiom identical to one asserted
        // in the premise is entailed by definition. `Axiom: PartialEq` also
        // compares the parser-minted ids (a per-file serialisation detail),
        // so the fast path re-checks axiom *content* when direct equality
        // misses — e.g. a conclusion ontology parsed from its own file shares
        // axiom content with the premise but never ids.
        {
            let guard = read_lock(ontology, "core: reading ontology for entailment check")?;
            if guard.axioms().contains(axiom) {
                return Ok(true);
            }
            let want_type = axiom.axiom_type();
            let want_key = axiom.content_key();
            if guard
                .axioms()
                .iter()
                .any(|a| a.axiom_type() == want_type && a.content_key() == want_key)
            {
                return Ok(true);
            }
        }

        // Route SubClassOf/ClassAssertion through the typed reasoner methods,
        // which operate directly on ClassExpression values rather than the
        // general check_entailment tableau path.
        match axiom {
            crate::ontology::Axiom::SubClassOf(sa) => {
                self.is_subclass_of_ontology(&sa.subclass, &sa.superclass, ontology)
            }
            crate::ontology::Axiom::ClassAssertion(ca) => {
                self.is_instance_of_ontology(&ca.individual, &ca.class, ontology)
            }
            _ => {
                self.task_service
                    .check_entailment(axiom, ontology, stats, &mut self.cache_manager)
            }
        }
    }

    /// Explain why an entailment holds
    pub fn explain_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
    ) -> Result<Vec<crate::ontology::Axiom>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for explain_entailment",
            )?;

            // Simple explanation: find axioms that directly support the entailment
            let mut explanation = Vec::new();

            match axiom {
                crate::ontology::Axiom::SubClassOf(subclass_axiom) => {
                    // Look for axioms that support this subsumption
                    for ont_axiom in ontology.axioms() {
                        match ont_axiom {
                            crate::ontology::Axiom::SubClassOf(ont_subclass)
                                // Direct support or transitive support: A ⊑ B, B ⊑ C → A ⊑ C
                                if ((ont_subclass.subclass == subclass_axiom.subclass
                                    && ont_subclass.superclass == subclass_axiom.superclass)
                                    || ont_subclass.superclass == subclass_axiom.subclass
                                    || ont_subclass.subclass == subclass_axiom.superclass)
                                => {
                                    explanation.push(ont_axiom.clone());
                                }
                            crate::ontology::Axiom::EquivalentClasses(equiv)
                                // Equivalence support
                                if (equiv.classes.contains(&subclass_axiom.subclass)
                                    || equiv.classes.contains(&subclass_axiom.superclass))
                                => {
                                    explanation.push(ont_axiom.clone());
                                }
                            _ => {}
                        }
                    }
                }
                crate::ontology::Axiom::ClassAssertion(class_assertion) => {
                    // Look for axioms that support this class assertion
                    for ont_axiom in ontology.axioms() {
                        if let crate::ontology::Axiom::ClassAssertion(ont_assertion) = ont_axiom
                            && ont_assertion.individual == class_assertion.individual
                            && ont_assertion.class == class_assertion.class
                        {
                            explanation.push(ont_axiom.clone());
                        }
                    }
                }
                _ => {
                    // For other axiom types, just return the axiom itself if it exists
                    for ont_axiom in ontology.axioms() {
                        if ont_axiom == axiom {
                            explanation.push(ont_axiom.clone());
                            break;
                        }
                    }
                }
            }

            Ok(explanation)
        } else {
            Ok(Vec::new())
        }
    }

    /// Explain why the ontology is inconsistent
    pub fn explain_inconsistency(&mut self) -> Result<Vec<crate::ontology::Axiom>> {
        if let Some(ontology_ref) = self.ontology.clone() {
            // Collect all axioms first, then drop the lock before calling get_instances
            let axioms: Vec<crate::ontology::Axiom> = {
                let ontology = read_lock(
                    &ontology_ref,
                    "core: reading ontology for explain_inconsistency",
                )?;
                ontology.axioms().to_vec()
            };

            let mut explanation = Vec::new();

            // Look for obvious inconsistencies
            for axiom in &axioms {
                match axiom {
                    crate::ontology::Axiom::DisjointClasses(disjoint) => {
                        // Check if any individual is asserted to be in disjoint classes
                        for class1 in &disjoint.classes {
                            for class2 in &disjoint.classes {
                                if class1 != class2 {
                                    // Look for individuals in both classes
                                    let instances1 = self.get_instances(class1, true)?;
                                    let instances2 = self.get_instances(class2, true)?;

                                    for ind1 in &instances1 {
                                        for ind2 in &instances2 {
                                            if ind1.iri() == ind2.iri() {
                                                explanation.push(axiom.clone());
                                                // Also add the class assertions
                                                for ont_axiom in &axioms {
                                                    if let crate::ontology::Axiom::ClassAssertion(
                                                        assertion,
                                                    ) = ont_axiom
                                                        && assertion.individual.iri() == ind1.iri()
                                                        && (assertion.class == *class1
                                                            || assertion.class == *class2)
                                                    {
                                                        explanation.push(ont_axiom.clone());
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    crate::ontology::Axiom::ClassAssertion(assertion) => {
                        // Check if individual is asserted to be in owl:Nothing
                        if let ClassExpression::Class(class) = &assertion.class
                            && class.iri.to_string() == "http://www.w3.org/2002/07/owl#Nothing"
                        {
                            explanation.push(axiom.clone());
                        }
                    }
                    _ => {}
                }
            }

            Ok(explanation)
        } else {
            Ok(Vec::new())
        }
    }

    /// Add an axiom to the ontology.
    ///
    /// **Note:** adding an [`crate::ontology::Axiom::Rule`] does *not* run SWRL
    /// materialization — rules only take effect when the ontology is (re)loaded
    /// through [`Reasoner::load_ontology`] or when
    /// [`Reasoner::materialize_swrl_rules`] is called explicitly.  This keeps
    /// axiom insertion cheap and side-effect free.
    pub fn add_axiom(&mut self, axiom: crate::ontology::Axiom) -> Result<()> {
        // Any mutation invalidates the cached realisation snapshot.
        self.invalidate_query_caches();
        if let Some(ontology_ref) = &mut self.ontology {
            let mut ontology = write_lock(ontology_ref, "core: writing ontology to add axiom")?;
            ontology.add_axiom(axiom);

            // Clear cache since ontology has changed
            self.cache_manager.clear_all()?;
            self.classification_service
                .invalidate_classification_cache();
            self.materialization_stale
                .store(true, std::sync::atomic::Ordering::Relaxed);

            Ok(())
        } else {
            // Create new ontology with this axiom
            let mut ontology = crate::ontology::Ontology::new();
            ontology.add_axiom(axiom);
            self.ontology = Some(Arc::new(RwLock::new(ontology)));
            Ok(())
        }
    }

    /// Run DL-safe SWRL forward-chaining materialization over the currently
    /// loaded ontology and merge the derived `ABox` facts into it.
    ///
    /// Returns the number of **new** axioms added — `0` when the ontology has
    /// no rules or when a previous run already derived everything, so repeated
    /// calls are idempotent (facts are deduplicated by
    /// [`crate::ontology::Axiom::content_key`], which ignores axiom ids).
    /// When axioms are added, every cache derived from the ontology
    /// (realisation snapshot, consistency memo, classification cache and the
    /// `ABox` materialization marker) is invalidated.
    ///
    /// `Reasoner::add_axiom` deliberately does **not** materialize: reload the
    /// ontology through [`Reasoner::load_ontology`] or call this method
    /// explicitly after adding rules.
    pub fn materialize_swrl_rules(&mut self) -> Result<usize> {
        let Some(ontology_ref) = self.ontology.clone() else {
            return Ok(0);
        };

        let materialization = {
            let guard = read_lock(
                &ontology_ref,
                "core: reading ontology for SWRL materialization",
            )?;
            crate::swrl::SwrlMaterializer::new().materialize(&guard)?
        };
        if materialization.derived_axioms.is_empty() {
            return Ok(0);
        }

        let mut added = 0usize;
        {
            let mut guard = write_lock(&ontology_ref, "core: writing SWRL materializations")?;
            // Content-key guard: a mutation between the snapshot above and
            // this write must never duplicate a fact.
            let mut known: HashSet<String> = guard
                .axioms()
                .iter()
                .filter(|axiom| {
                    matches!(
                        axiom,
                        crate::ontology::Axiom::ClassAssertion(_)
                            | crate::ontology::Axiom::ObjectPropertyAssertion(_)
                            | crate::ontology::Axiom::DataPropertyAssertion(_)
                            | crate::ontology::Axiom::SameIndividual(_)
                            | crate::ontology::Axiom::DifferentIndividuals(_)
                    )
                })
                .map(crate::ontology::Axiom::content_key)
                .collect();
            for axiom in materialization.derived_axioms {
                if known.insert(axiom.content_key()) {
                    guard.add_axiom(axiom);
                    added += 1;
                }
            }
        }

        if added > 0 {
            self.invalidate_query_caches();
            self.cache_manager.clear_all()?;
            self.classification_service
                .invalidate_classification_cache();
            self.materialization_stale
                .store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("SWRL materialization added {added} new axiom(s)");
        }
        Ok(added)
    }

    /// Remove an axiom from the ontology
    pub fn remove_axiom(&mut self, axiom: &crate::ontology::Axiom) -> Result<bool> {
        // Any mutation invalidates the cached realisation snapshot.
        self.invalidate_query_caches();
        if let Some(ontology_ref) = &mut self.ontology {
            let mut ontology = write_lock(ontology_ref, "core: writing ontology to remove axiom")?;
            let original_count = ontology.axioms().len();
            ontology.remove_axiom(axiom);
            let removed = ontology.axioms().len() < original_count;

            if removed {
                // Clear cache since ontology has changed
                self.cache_manager.clear_all()?;
                self.classification_service
                    .invalidate_classification_cache();
            }

            // Return the boolean result properly wrapped
            Ok(removed)
        } else {
            Ok(false) // No ontology to remove from
        }
    }

    /// Get reasoning statistics.
    ///
    /// Returns the accumulated totals of every operation performed through
    /// this reasoner (`classify`, `realize`, `get_instances`, consistency
    /// checks — including memoised-cache hits). Signature unchanged from the
    /// always-zero-stats behaviour it replaces.
    #[must_use]
    pub fn get_statistics(&self) -> ReasoningStatistics {
        self.statistics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Fold a per-operation local statistics value into the shared totals.
    fn merge_statistics(&self, local: &ReasoningStatistics) {
        self.statistics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .merge(local);
    }

    /// Get the size of the current ontology
    #[must_use]
    pub fn get_ontology_size(&self) -> usize {
        if let Some(ref ontology) = self.ontology {
            read_lock(ontology, "core: reading ontology for size")
                .map_or(0, |guard| guard.axioms().len())
        } else {
            0
        }
    }

    /// Load an ontology into the reasoner, resolving its `owl:imports`
    /// closure first (HermiT `Imports.INCLUDED` parity).
    ///
    /// - Imports are resolved **recursively** — cycle-safe and bounded by
    ///   `max_import_depth` — with the default best-effort configuration:
    ///   base directories are the current working directory plus the importing
    ///   file's directory (when loaded via [`Self::load_ontology_from_file`]),
    ///   a 10 s HTTP timeout, no import validation, and deduplicated merging of
    ///   the imported axioms.
    /// - A missing / unresolvable import logs a `warn` and the ontology is
    ///   loaded without it: the load itself never fails because of imports.
    /// - Ontologies declaring no imports (neither a structured `imports` entry
    ///   nor an `owl:imports` annotation) take exactly the previous code path.
    ///
    /// Use [`Reasoner::load_ontology_with_imports`] to supply a custom
    /// [`ImportManagerConfig`] (strict strategy, explicit base directories…).
    pub fn load_ontology(&mut self, ontology: crate::ontology::Ontology) -> Result<()> {
        let config = Self::default_import_config();
        self.load_ontology_with_import_config(ontology, config, None)
    }

    /// Load an ontology with an explicit import-resolution configuration —
    /// the explicit variant of [`Reasoner::load_ontology`].
    ///
    /// With `config.resolution_strategy == ImportResolutionStrategy::Strict` a
    /// failed import aborts the load with an error; under the default
    /// `BestEffort` strategy failures are only logged (see
    /// [`Reasoner::load_ontology`]).
    pub fn load_ontology_with_imports(
        &mut self,
        ontology: crate::ontology::Ontology,
        config: ImportManagerConfig,
    ) -> Result<()> {
        self.load_ontology_with_import_config(ontology, config, None)
    }

    /// Default import configuration used by [`Reasoner::load_ontology`]:
    /// best-effort, merging, no validation, depth 10, 10 s timeout, and the
    /// current working directory as base directory (the importing file's own
    /// directory is appended by [`Self::load_ontology_with_import_config`]).
    fn default_import_config() -> ImportManagerConfig {
        let mut base_directories = Vec::with_capacity(2);
        if let Ok(cwd) = std::env::current_dir() {
            base_directories.push(cwd);
        }
        base_directories.push(std::path::PathBuf::from("."));
        ImportManagerConfig {
            resolution_strategy: ImportResolutionStrategy::BestEffort,
            max_import_depth: 10,
            base_directories,
            url_mappings: std::collections::HashMap::new(),
            validate_imports: false,
            merge_imports: true,
            timeout: Some(std::time::Duration::from_secs(10)),
        }
    }

    /// Whether the ontology declares imports (structured `imports` field
    /// populated by the functional/manchester parsers, or an `owl:imports`
    /// annotation produced by the RDF-family parsers).
    fn ontology_has_imports(ontology: &crate::ontology::Ontology) -> bool {
        !ontology.imports.is_empty()
            || ontology.annotations.iter().any(|annotation| {
                annotation.property.iri.as_str() == "http://www.w3.org/2002/07/owl#imports"
            })
    }

    /// Resolve the `owl:imports` closure (when the ontology declares any) and
    /// install the result, invalidating every query-time cache.
    ///
    /// `source_dir` (the directory of the file the ontology was parsed from,
    /// if any) is appended to the configuration's base directories so relative
    /// and `file://` imports resolve next to the importing document.
    fn load_ontology_with_import_config(
        &mut self,
        mut ontology: crate::ontology::Ontology,
        config: ImportManagerConfig,
        source_dir: Option<std::path::PathBuf>,
    ) -> Result<()> {
        if Self::ontology_has_imports(&ontology) {
            let strict = matches!(config.resolution_strategy, ImportResolutionStrategy::Strict);
            let mut config = config;
            if let Some(dir) = source_dir {
                config.base_directories.push(dir);
            }
            let manager = ImportManager::new(config);
            match manager.resolve_imports_recursive(&mut ontology) {
                Ok(results) => {
                    for result in &results {
                        for error in &result.errors {
                            log::warn!("owl:imports {}: {error}", result.import_iri);
                        }
                        for warning in &result.warnings {
                            log::warn!("owl:imports {}: {warning}", result.import_iri);
                        }
                    }
                }
                // Best effort by default: a failed import must never fail the
                // whole load — warn and keep the axioms that did resolve.
                Err(err) => {
                    if strict {
                        return Err(err);
                    }
                    log::warn!(
                        "owl:imports resolution failed; loading ontology without the failed imports: {err}"
                    );
                }
            }
        }
        self.install_ontology(ontology)
    }

    /// Install a (fully import-closed) ontology, replacing the current one and
    /// invalidating every cache derived from it.
    ///
    /// **SWRL materialization**: every `Axiom::Rule` is DL-safe
    /// forward-chained over the `ABox` *before* the ontology is installed (see
    /// [`crate::swrl::SwrlMaterializer`]), so the first query already sees the
    /// rule-derived facts.  This runs after `owl:imports` merging and before
    /// any cache is built; ontologies without rules are untouched.  Adding a
    /// rule later via [`Reasoner::add_axiom`] does **not** materialize — use
    /// [`Reasoner::materialize_swrl_rules`] or reload the ontology.
    fn install_ontology(&mut self, mut ontology: crate::ontology::Ontology) -> Result<()> {
        let rule_count = ontology
            .axioms()
            .iter()
            .filter(|axiom| matches!(axiom, crate::ontology::Axiom::Rule(_)))
            .count();
        let materialization = crate::swrl::SwrlMaterializer::new().materialize(&ontology)?;
        if rule_count > 0 {
            log::info!(
                "SWRL materialization: {rule_count} rule(s), {} application(s), {} iteration(s), {} new axiom(s)",
                materialization.applications,
                materialization.iterations,
                materialization.derived_axioms.len()
            );
        }
        for axiom in materialization.derived_axioms {
            ontology.add_axiom(axiom);
        }

        // The ontology is replaced wholesale → drop the cached snapshot and
        // the memoised consistency answer.
        self.invalidate_query_caches();
        self.ontology = Some(Arc::new(RwLock::new(ontology.clone())));
        self.materialization_stale
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.rl_reasoner = None;
        // Drop any stale classification cache — the ontology has changed.
        self.classification_service
            .invalidate_classification_cache();
        // Register DatatypeDefinition axioms so user-defined datatype values are validated
        // against their defining data ranges (workstream A4).
        self.classification_service
            .register_datatype_definitions(&ontology);
        Ok(())
    }

    /// Load ontology from file
    pub fn load_ontology_from_file<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        format: crate::ontology::OntologyFormat,
    ) -> Result<()> {
        use crate::parsers::{
            Parser, extract_first_crosssyntax_section, functional, manchester, ntriples, owl_xml,
            rdf_xml, turtle,
        };

        let file_path = path.as_ref();

        // `Auto` delegates to the *shared* auto-detecting loader
        // (`parsers::parse_file_auto`), so the CLI matches library detection
        // exactly (B5): the unambiguous extension table (`.rdf` → RDF/XML,
        // `.jsonld`, `.n3`, `.html` → RDFa, `.hdt`/`.brdf` → binary), content
        // sniffing for the ambiguous extensions, and a byte-oriented read for
        // the binary formats.  The previous inline extension switch
        // misrouted `.rdf`/`.jsonld`/`.n3`/`.html` to OWL/XML and died in
        // `read_to_string` on `.hdt`/`.brdf` (not valid UTF-8).
        if matches!(format, crate::ontology::OntologyFormat::Auto) {
            let ontology = crate::parsers::parse_file_auto(file_path)?;
            let source_dir = file_path.parent().map(std::path::Path::to_path_buf);
            let config = Self::default_import_config();
            return self.load_ontology_with_import_config(ontology, config, source_dir);
        }

        let content = std::fs::read_to_string(file_path).map_err(|e| crate::Error::Io {
            message: format!("Failed to read file: {e}"),
        })?;

        // Extract first section if CrossSyntax multi-format file
        let parsed_content = extract_first_crosssyntax_section(&content);

        let ontology = match format {
            crate::ontology::OntologyFormat::Functional => {
                let parser = functional::FunctionalParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Manchester => {
                let parser = manchester::ManchesterParser::new(
                    manchester::ManchesterParserConfig::default(),
                );
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::DL => {
                crate::parsers::dl_syntax::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Krss => crate::parsers::krss::parse(&parsed_content)?,
            crate::ontology::OntologyFormat::Krss2 => {
                crate::parsers::krss::parse_krss2(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Latex => {
                return Err(crate::Error::Unsupported {
                    message: "LaTeX is a write-only format".to_string(),
                });
            }
            crate::ontology::OntologyFormat::Obo => crate::parsers::obo::parse(&parsed_content)?,
            crate::ontology::OntologyFormat::NQuads => {
                crate::parsers::rio::nquads::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::N3 => crate::parsers::rio::n3::parse(&parsed_content)?,
            crate::ontology::OntologyFormat::TriG => {
                crate::parsers::rio::trig::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::TriX => {
                crate::parsers::rio::trix::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::JsonLd => {
                crate::parsers::rio::jsonld::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::RdfJson => {
                crate::parsers::rio::rdf_json::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Rdfa => {
                crate::parsers::rio::rdfa::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::BinaryRdf => {
                crate::parsers::rio::binary_rdf::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Hdt => {
                crate::parsers::rio::hdt::parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::Turtle => {
                let parser = turtle::TurtleParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::RdfXml => {
                let parser = rdf_xml::RdfXmlParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::OwlXml => {
                let parser = owl_xml::OwlXmlParser::new();
                parser.parse(&parsed_content)?
            }
            crate::ontology::OntologyFormat::NTriples => {
                let parser = ntriples::NTriplesParser::new();
                parser.parse(&parsed_content)?
            }
            // Handled by the early return above: `Auto` never reaches this
            // point (it is delegated to `parsers::parse_file_auto`, which
            // resolves the format from the shared extension table / content
            // sniffing / binary magic before any read).  Kept so the match
            // stays exhaustive.
            crate::ontology::OntologyFormat::Auto => {
                return Err(crate::Error::ontology_parsing(
                    "Auto format must be resolved before parsing",
                ));
            }
        };

        // Resolve the ontology's `owl:imports` closure with the file's own
        // directory as an import base (HermiT `Imports.INCLUDED` parity).
        let source_dir = file_path.parent().map(std::path::Path::to_path_buf);
        let config = Self::default_import_config();
        self.load_ontology_with_import_config(ontology, config, source_dir)
    }

    /// Classify object properties.
    ///
    /// The returned [`super::results::PropertyClassificationResult`] maps each
    /// named object property of the signature to its **full strict
    /// superproperty set**: the transitive closure of `SubObjectPropertyOf`,
    /// `EquivalentObjectProperties` (used for reachability only — equivalents
    /// are *strict* supers of nothing, see
    /// [`super::property_hierarchy`]), `InverseObjectProperties`, plus
    /// `owl:topObjectProperty` as a super of every property. Property-chain
    /// subs are not hierarchy edges (their internals are ignored).
    pub fn classify_object_properties(
        &mut self,
    ) -> Result<super::results::PropertyClassificationResult> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(
                ontology,
                "core: reading ontology for classify_object_properties",
            )?;
            let closure =
                super::property_hierarchy::ObjectPropertyHierarchy::compute(&ontology_ref);

            // Extract all object properties from the ontology
            let signature = ontology_ref.signature()?;
            let mut hierarchy = std::collections::HashMap::new();
            for property in &signature.object_properties {
                let expression =
                    crate::ontology::ObjectPropertyExpression::ObjectProperty(property.clone());
                hierarchy.insert(expression.clone(), closure.supers(&expression));
            }

            Ok(super::results::PropertyClassificationResult::new_object_properties(hierarchy))
        } else {
            Ok(
                super::results::PropertyClassificationResult::new_object_properties(
                    std::collections::HashMap::new(),
                ),
            )
        }
    }

    /// Classify data properties.
    ///
    /// Same contract as [`Self::classify_object_properties`] on the data side:
    /// the full strict superproperty closure of `SubDataPropertyOf` /
    /// `EquivalentDataProperties` plus `owl:topDataProperty` as a super of
    /// every data property.
    pub fn classify_data_properties(
        &mut self,
    ) -> Result<super::results::PropertyClassificationResult> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(
                ontology,
                "core: reading ontology for classify_data_properties",
            )?;
            let closure = super::property_hierarchy::DataPropertyHierarchy::compute(&ontology_ref);

            // Extract all data properties from the ontology
            let signature = ontology_ref.signature()?;
            let mut hierarchy = std::collections::HashMap::new();
            for property in &signature.data_properties {
                let expression =
                    crate::ontology::DataPropertyExpression::DataProperty(property.clone());
                hierarchy.insert(expression.clone(), closure.supers(&expression));
            }

            Ok(super::results::PropertyClassificationResult::new_data_properties(hierarchy))
        } else {
            Ok(
                super::results::PropertyClassificationResult::new_data_properties(
                    std::collections::HashMap::new(),
                ),
            )
        }
    }

    /// Get unsatisfiable classes
    pub fn get_unsatisfiable_classes(&self) -> Result<Vec<ClassExpression>> {
        if let Some(ontology_ref) = &self.ontology {
            let _ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_unsatisfiable_classes",
            )?;
            let mut unsatisfiable = Vec::new();

            // Get all classes in the ontology
            let all_classes = self.get_all_classes_in_ontology_internal()?;

            // Check each class for satisfiability
            for class in all_classes {
                if !self.is_class_satisfiable(&class)? {
                    unsatisfiable.push(class);
                }
            }

            // Always include owl:Nothing if not already included
            let owl_nothing = ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ));
            if !unsatisfiable
                .iter()
                .any(|c| self.classes_equivalent(c, &owl_nothing).unwrap_or(false))
            {
                unsatisfiable.push(owl_nothing);
            }

            Ok(unsatisfiable)
        } else {
            // In empty ontology, only owl:Nothing is unsatisfiable
            Ok(vec![ClassExpression::Class(crate::ontology::Class::new(
                crate::ontology::IRI::owl_nothing(),
            ))])
        }
    }

    /// Get ontology prefixes
    pub fn get_prefixes(&self) -> Result<std::collections::HashMap<String, String>> {
        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(ontology_ref, "core: reading ontology for get_prefixes")?;
            let mut prefixes = std::collections::HashMap::new();

            // Extract prefixes from ontology entities
            for axiom in ontology.axioms() {
                self.extract_prefixes_from_axiom(axiom, &mut prefixes);
            }

            // Add standard prefixes if not present
            if !prefixes.contains_key("rdf") {
                prefixes.insert(
                    "rdf".to_string(),
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
                );
            }
            if !prefixes.contains_key("rdfs") {
                prefixes.insert(
                    "rdfs".to_string(),
                    "http://www.w3.org/2000/01/rdf-schema#".to_string(),
                );
            }
            if !prefixes.contains_key("owl") {
                prefixes.insert(
                    "owl".to_string(),
                    "http://www.w3.org/2002/07/owl#".to_string(),
                );
            }
            if !prefixes.contains_key("xsd") {
                prefixes.insert(
                    "xsd".to_string(),
                    "http://www.w3.org/2001/XMLSchema#".to_string(),
                );
            }

            Ok(prefixes)
        } else {
            // Return standard prefixes for empty ontology
            let mut prefixes = std::collections::HashMap::new();
            prefixes.insert(
                "rdf".to_string(),
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
            );
            prefixes.insert(
                "rdfs".to_string(),
                "http://www.w3.org/2000/01/rdf-schema#".to_string(),
            );
            prefixes.insert(
                "owl".to_string(),
                "http://www.w3.org/2002/07/owl#".to_string(),
            );
            prefixes.insert(
                "xsd".to_string(),
                "http://www.w3.org/2001/XMLSchema#".to_string(),
            );
            Ok(prefixes)
        }
    }

    /// Dump DL clauses
    pub fn dump_dl_clauses(&self) -> Result<crate::dl_clauses::DLClauseSet> {
        if let Some(ontology) = &self.ontology {
            let ontology_ref = read_lock(ontology, "core: reading ontology for dump_dl_clauses")?;
            let mut generator = DLClauseGenerator::new();
            generator.generate_clauses(&ontology_ref)
        } else {
            Ok(crate::dl_clauses::DLClauseSet::new())
        }
    }

    /// Process `OWLlink` request
    pub fn process_owllink_request(&mut self, request: &str) -> Result<String> {
        // Parse OWLlink XML request
        let parsed_request = self.parse_owllink_xml(request)?;

        match parsed_request.command.as_str() {
            "Tell" => {
                // Add axioms to ontology - would need mutable access
                Ok("<Response><OK/></Response>".to_string())
            }
            "IsClassSatisfiable" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let result = self.is_class_satisfiable(&class_expr)?;
                    Ok(format!(
                        "<Response><BooleanResponse result=\"{result}\"/></Response>"
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in IsClassSatisfiable request",
                    ))
                }
            }
            "IsKBConsistent" => {
                let result = self.is_consistent()?;
                Ok(format!(
                    "<Response><BooleanResponse result=\"{result}\"/></Response>"
                ))
            }
            "GetSubClasses" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let subclasses =
                        self.get_subclasses(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let subclass_elements: Vec<String> = subclasses
                        .iter()
                        .map(|expr| match expr {
                            crate::ontology::concepts::ClassExpression::Class(c) => {
                                format!("<Class IRI=\"{}\"/>", c.iri)
                            }
                            _ => {
                                "<Class IRI=\"http://www.w3.org/2002/07/owl#Nothing\"/>".to_string()
                            }
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfClasses>{}</SetOfClasses></Response>",
                        subclass_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetSubClasses request",
                    ))
                }
            }
            "GetSuperClasses" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let superclasses =
                        self.get_superclasses(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let superclass_elements: Vec<String> = superclasses
                        .iter()
                        .map(|expr| match expr {
                            crate::ontology::concepts::ClassExpression::Class(c) => {
                                format!("<Class IRI=\"{}\"/>", c.iri)
                            }
                            _ => "<Class IRI=\"http://www.w3.org/2002/07/owl#Thing\"/>".to_string(),
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfClasses>{}</SetOfClasses></Response>",
                        superclass_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetSuperClasses request",
                    ))
                }
            }
            "GetInstances" => {
                if let Some(class_iri) = parsed_request.class_iri {
                    let class = crate::ontology::concepts::Class {
                        iri: class_iri.into(),
                    };
                    let class_expr = crate::ontology::concepts::ClassExpression::Class(class);
                    let instances =
                        self.get_instances(&class_expr, parsed_request.direct.unwrap_or(false))?;
                    let instance_elements: Vec<String> = instances
                        .iter()
                        .map(|ind| {
                            format!(
                                "<NamedIndividual IRI=\"{}\"/>",
                                ind.iri().map_or_else(
                                    || "unknown".to_string(),
                                    std::string::ToString::to_string
                                )
                            )
                        })
                        .collect();
                    Ok(format!(
                        "<Response><SetOfIndividuals>{}</SetOfIndividuals></Response>",
                        instance_elements.join("")
                    ))
                } else {
                    Err(Error::reasoning(
                        "Missing class IRI in GetInstances request",
                    ))
                }
            }
            _ => Err(Error::reasoning(format!(
                "Unsupported OWLlink command: {}",
                parsed_request.command
            ))),
        }
    }

    /// Parse `OWLlink` XML request into structured data
    fn parse_owllink_xml(&self, xml: &str) -> Result<OWLlinkRequest> {
        // Simple XML parsing - would use proper XML parser in production
        let mut request = OWLlinkRequest {
            command: String::new(),
            axioms: Vec::new(),
            class_iri: None,
            individual_iri: None,
            direct: None,
        };

        // Extract command from XML
        if xml.contains("<Tell") {
            request.command = "Tell".to_string();
        } else if xml.contains("<IsClassSatisfiable") {
            request.command = "IsClassSatisfiable".to_string();
        } else if xml.contains("<IsKBConsistent") {
            request.command = "IsKBConsistent".to_string();
        } else if xml.contains("<GetSubClasses") {
            request.command = "GetSubClasses".to_string();
        } else if xml.contains("<GetSuperClasses") {
            request.command = "GetSuperClasses".to_string();
        } else if xml.contains("<GetInstances") {
            request.command = "GetInstances".to_string();
        }

        // Extract class IRI if present
        if let Some(start) = xml.find("IRI=\"")
            && let Some(end) = xml[start + 5..].find('"')
        {
            let iri_str = &xml[start + 5..start + 5 + end];
            request.class_iri = Some(
                url::Url::parse(iri_str)
                    .map_err(|e| Error::reasoning(format!("Invalid IRI: {e}")))?,
            );
        }

        // Extract direct attribute if present
        if xml.contains("direct=\"true\"") {
            request.direct = Some(true);
        } else if xml.contains("direct=\"false\"") {
            request.direct = Some(false);
        }

        Ok(request)
    }

    /// Execute SPARQL query
    pub fn execute_sparql_query(&self, query: &str) -> Result<String> {
        // If the Oxigraph store is loaded, delegate full SPARQL 1.1 evaluation to it
        #[cfg(feature = "sparql-store")]
        {
            if let Ok(guard) = self.oxigraph_store.lock()
                && let Some(store) = guard.as_ref()
            {
                return store.execute_query(query);
            }
        }

        // Fallback: custom string-based parser for basic cases
        let parsed_query = self.parse_sparql_query(query)?;

        match parsed_query.query_type.as_str() {
            "SELECT" => self.execute_sparql_select(&parsed_query),
            "ASK" => self.execute_sparql_ask(&parsed_query),
            "CONSTRUCT" => self.execute_sparql_construct(&parsed_query),
            "DESCRIBE" => self.execute_sparql_describe(&parsed_query),
            "INSERT" => self.execute_sparql_insert(&parsed_query),
            "DELETE" => self.execute_sparql_delete(&parsed_query),
            _ => Err(Error::reasoning(format!(
                "Unsupported SPARQL query type: {}",
                parsed_query.query_type
            ))),
        }
    }

    /// Parse SPARQL query into structured form
    fn parse_sparql_query(&self, query: &str) -> Result<SparqlQuery> {
        let query_upper = query.to_uppercase();

        let query_type = if query_upper.contains("INSERT") {
            "INSERT"
        } else if query_upper.contains("DELETE") {
            "DELETE"
        } else if query_upper.contains("SELECT") {
            "SELECT"
        } else if query_upper.contains("ASK") {
            "ASK"
        } else if query_upper.contains("CONSTRUCT") {
            "CONSTRUCT"
        } else if query_upper.contains("DESCRIBE") {
            "DESCRIBE"
        } else {
            return Err(Error::reasoning("Unknown SPARQL query type"));
        };

        // Extract variables for SELECT queries
        let mut variables = Vec::new();
        if query_type == "SELECT"
            && let Some(start) = query_upper.find("SELECT")
            && let Some(end) = query_upper.find("WHERE")
        {
            let select_clause = &query[start + 6..end].trim();
            if select_clause.starts_with('*') {
                variables.push("*".to_string());
            } else {
                // Extract ?variable names
                for word in select_clause.split_whitespace() {
                    if word.starts_with('?') {
                        variables.push(word.to_string());
                    }
                }
            }
        }

        // Extract WHERE clause patterns
        let mut patterns = Vec::new();
        if let Some(start) = query_upper.find("WHERE")
            && let Some(brace_start) = query[start..].find('{')
            && let Some(brace_end) = query[start + brace_start..].find('}')
        {
            let where_content =
                &query[start + brace_start + 1..start + brace_start + brace_end].trim();

            // Simple triple pattern extraction
            for line in where_content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        patterns.push(TriplePattern {
                            subject: parts[0].to_string(),
                            predicate: parts[1].to_string(),
                            object: parts[2].to_string(),
                        });
                    }
                }
            }
        }

        // Extract INSERT DATA patterns
        let mut insert_patterns = Vec::new();
        if query_type == "INSERT" {
            insert_patterns = self.extract_insert_delete_patterns(query, "INSERT")?;
        }

        // Extract DELETE DATA patterns
        let mut delete_patterns = Vec::new();
        if query_type == "DELETE" {
            delete_patterns = self.extract_insert_delete_patterns(query, "DELETE")?;
        }

        Ok(SparqlQuery {
            query_type: query_type.to_string(),
            variables,
            patterns,
            insert_patterns,
            delete_patterns,
            original_query: query.to_string(),
        })
    }

    /// Execute SPARQL SELECT query
    fn execute_sparql_select(&self, query: &SparqlQuery) -> Result<String> {
        let mut results = Vec::new();

        if let Some(ontology) = &self.ontology {
            let ontology_guard =
                read_lock(ontology, "core: reading ontology for execute_sparql_select")?;

            // For each triple pattern, find matching axioms
            for pattern in &query.patterns {
                if pattern.predicate.contains("type") || pattern.predicate.contains("rdf:type") {
                    // Class assertion pattern: ?x rdf:type ClassName
                    for axiom in ontology_guard.axioms() {
                        if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                            let class_iri = match &assertion.class {
                                crate::ontology::concepts::ClassExpression::Class(c) => {
                                    c.iri.to_string()
                                }
                                _ => continue,
                            };
                            let individual_iri = assertion.individual.to_string();

                            if pattern.object.contains(&class_iri) || pattern.object == "?class" {
                                let mut binding = HashMap::new();
                                if pattern.subject.starts_with('?') {
                                    binding.insert(pattern.subject.clone(), individual_iri);
                                }
                                if pattern.object.starts_with('?') {
                                    binding.insert(pattern.object.clone(), class_iri);
                                }
                                results.push(binding);
                            }
                        }
                    }
                } else {
                    // Property assertion pattern: ?x propertyName ?y
                    for axiom in ontology_guard.axioms() {
                        match axiom {
                            crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) => {
                                let property_iri = assertion.property.to_string();
                                let subject_iri = assertion.source.to_string();
                                let object_iri = assertion.target.to_string();
                                if pattern.predicate.contains(&property_iri)
                                    || pattern.predicate.starts_with('?')
                                {
                                    let mut binding = HashMap::new();
                                    if pattern.subject.starts_with('?') {
                                        binding.insert(pattern.subject.clone(), subject_iri);
                                    }
                                    if pattern.predicate.starts_with('?') {
                                        binding.insert(pattern.predicate.clone(), property_iri);
                                    }
                                    if pattern.object.starts_with('?') {
                                        binding.insert(pattern.object.clone(), object_iri);
                                    }
                                    results.push(binding);
                                }
                            }
                            crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) => {
                                let property_iri = assertion.property.to_string();
                                let subject_iri = assertion.individual.to_string();
                                let object_value = assertion.value.to_string();
                                if pattern.predicate.contains(&property_iri)
                                    || pattern.predicate.starts_with('?')
                                {
                                    let mut binding = HashMap::new();
                                    if pattern.subject.starts_with('?') {
                                        binding.insert(pattern.subject.clone(), subject_iri);
                                    }
                                    if pattern.predicate.starts_with('?') {
                                        binding.insert(pattern.predicate.clone(), property_iri);
                                    }
                                    if pattern.object.starts_with('?') {
                                        binding.insert(pattern.object.clone(), object_value);
                                    }
                                    results.push(binding);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Format results as JSON
        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|binding| {
                serde_json::Value::Object(
                    binding
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect(),
                )
            })
            .collect();

        Ok(serde_json::json!({
            "head": {
                "vars": query.variables
            },
            "results": {
                "bindings": json_results
            }
        })
        .to_string())
    }

    /// Execute SPARQL ASK query
    fn execute_sparql_ask(&self, query: &SparqlQuery) -> Result<String> {
        // ASK queries return true/false
        let select_result = self.execute_sparql_select(query)?;
        let parsed: serde_json::Value = serde_json::from_str(&select_result)
            .map_err(|e| Error::reasoning(format!("Failed to parse SELECT result: {e}")))?;

        let has_results = if let Some(bindings) = parsed["results"]["bindings"].as_array() {
            !bindings.is_empty()
        } else {
            false
        };

        Ok(serde_json::json!({
            "head": {},
            "boolean": has_results
        })
        .to_string())
    }

    /// Execute SPARQL CONSTRUCT query
    fn execute_sparql_construct(&self, _query: &SparqlQuery) -> Result<String> {
        // CONSTRUCT queries build new RDF graphs
        // For now, return empty graph
        Ok(serde_json::json!({
            "head": {},
            "results": {
                "bindings": []
            }
        })
        .to_string())
    }

    /// Execute SPARQL DESCRIBE query
    fn execute_sparql_describe(&self, _query: &SparqlQuery) -> Result<String> {
        // DESCRIBE queries return information about resources
        // For now, return empty result
        Ok(serde_json::json!({
            "head": {},
            "results": {
                "bindings": []
            }
        })
        .to_string())
    }

    /// Execute SPARQL INSERT DATA query
    fn execute_sparql_insert(&self, query: &SparqlQuery) -> Result<String> {
        let ontology = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))?;

        let mut ontology_guard =
            write_lock(ontology, "core: writing ontology for execute_sparql_insert")?;

        let mut inserted_count = 0;

        // Convert triple patterns to axioms and add to ontology
        for pattern in &query.insert_patterns {
            let axiom = self.triple_pattern_to_axiom(pattern)?;
            ontology_guard.add_axiom(axiom);
            inserted_count += 1;
        }
        drop(ontology_guard);
        self.invalidate_query_caches();

        Ok(serde_json::json!({
            "status": "success",
            "message": format!("Inserted {} triples", inserted_count),
            "inserted": inserted_count
        })
        .to_string())
    }

    /// Execute SPARQL DELETE DATA query
    fn execute_sparql_delete(&self, query: &SparqlQuery) -> Result<String> {
        let ontology = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded"))?;

        let mut ontology_guard =
            write_lock(ontology, "core: writing ontology for execute_sparql_delete")?;

        #[allow(unused_assignments)]
        let mut deleted_count = 0;
        let initial_count = ontology_guard.axioms().len();

        // Convert triple patterns to axioms and remove from ontology
        for pattern in &query.delete_patterns {
            let axiom = self.triple_pattern_to_axiom(pattern)?;
            // Remove the axiom
            ontology_guard.remove_axiom(&axiom);
        }

        // Calculate how many were actually deleted
        deleted_count = initial_count - ontology_guard.axioms().len();
        drop(ontology_guard);
        self.invalidate_query_caches();

        Ok(serde_json::json!({
            "status": "success",
            "message": format!("Deleted {} triples", deleted_count),
            "deleted": deleted_count
        })
        .to_string())
    }

    /// Extract INSERT or DELETE DATA patterns from query
    fn extract_insert_delete_patterns(
        &self,
        query: &str,
        operation: &str,
    ) -> Result<Vec<TriplePattern>> {
        let mut patterns = Vec::new();
        let query_upper = query.to_uppercase();

        // Look for "INSERT DATA" or "DELETE DATA" followed by braces
        let keyword = format!("{operation} DATA");
        if let Some(start) = query_upper.find(&keyword) {
            // Find the opening brace after the keyword
            let search_from = start + keyword.len();
            let remaining_query = &query[search_from..];

            if let Some(brace_start) = remaining_query.find('{') {
                let content_start = search_from + brace_start + 1;
                let remaining_after_brace = &query[content_start..];

                if let Some(brace_end) = remaining_after_brace.find('}') {
                    let data_content = &query[content_start..content_start + brace_end];

                    // Parse triple patterns line by line
                    // Each line can contain one triple pattern
                    // Lines can end with . or ; or just a newline
                    let lines: Vec<&str> = data_content.lines().collect();

                    for line in lines {
                        let line = line.trim();

                        // Skip empty lines
                        if line.is_empty() {
                            continue;
                        }

                        // Remove trailing . or ; if present
                        let line_clean = line.trim_end_matches('.').trim_end_matches(';').trim();

                        if line_clean.is_empty() {
                            continue;
                        }

                        // Parse statement into triple pattern
                        // Split by whitespace to get subject, predicate, object
                        let parts: Vec<&str> = line_clean.split_whitespace().collect();

                        if parts.len() >= 3 {
                            let subject = parts[0];
                            let predicate = parts[1];
                            // Object might have spaces (e.g., literals)
                            let object = parts[2..].join(" ");

                            patterns.push(TriplePattern {
                                subject: subject.to_string(),
                                predicate: predicate.to_string(),
                                object,
                            });
                        }
                    }
                }
            }
        }

        if patterns.is_empty() {
            return Err(Error::reasoning(format!(
                "No patterns found in {operation} query. Data content might be malformed."
            )));
        }

        Ok(patterns)
    }

    /// Convert a triple pattern to an OWL axiom
    fn triple_pattern_to_axiom(&self, pattern: &TriplePattern) -> Result<crate::ontology::Axiom> {
        use crate::ontology::{
            Class, ClassExpression, DataProperty, DataPropertyExpression, IRI, Individual, Literal,
            ObjectProperty, ObjectPropertyExpression,
            axioms::{
                Axiom, ClassAssertionAxiom, DataPropertyAssertionAxiom,
                ObjectPropertyAssertionAxiom,
            },
        };

        // Remove angle brackets from IRIs
        let subject = pattern.subject.trim_matches(|c| c == '<' || c == '>');
        let predicate = pattern.predicate.trim_matches(|c| c == '<' || c == '>');
        let object_raw = pattern.object.trim();

        // Generate axiom ID (using a simple hash for now)
        let axiom_id = self.generate_axiom_id(pattern);

        // Check if predicate is rdf:type (class assertion)
        if predicate.contains("type") || predicate.ends_with("#type") {
            let object = object_raw.trim_matches(|c| c == '<' || c == '>');
            let individual = Individual::named(IRI::new(subject));
            let class = ClassExpression::Class(Class::new(IRI::new(object)));
            return Ok(Axiom::ClassAssertion(ClassAssertionAxiom {
                id: axiom_id,
                individual,
                class,
                annotations: Vec::new(),
            }));
        }

        // Check if object is a literal (data property assertion)
        if let Some(stripped) = object_raw.strip_prefix('"') {
            let individual = Individual::named(IRI::new(subject));
            let property = DataPropertyExpression::DataProperty(DataProperty {
                iri: IRI::new(predicate),
            });

            // Parse literal value
            let value = if let Some(quote_end) = stripped.find('"') {
                stripped[..quote_end].to_string()
            } else {
                object_raw.trim_matches('"').to_string()
            };

            let literal = Literal::new(value);

            return Ok(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                id: axiom_id,
                individual,
                property,
                value: literal,
                annotations: Vec::new(),
            }));
        }

        // Otherwise, it's an object property assertion
        let object = object_raw.trim_matches(|c| c == '<' || c == '>');
        let source = Individual::named(IRI::new(subject));
        let target = Individual::named(IRI::new(object));
        let property = ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(predicate),
        });

        Ok(Axiom::ObjectPropertyAssertion(
            ObjectPropertyAssertionAxiom {
                id: axiom_id,
                source,
                target,
                property,
                annotations: Vec::new(),
            },
        ))
    }

    /// Generate a unique axiom ID from a triple pattern
    fn generate_axiom_id(&self, pattern: &TriplePattern) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pattern.subject.hash(&mut hasher);
        pattern.predicate.hash(&mut hasher);
        pattern.object.hash(&mut hasher);
        hasher.finish()
    }

    /// Get DL clauses as string
    pub fn get_dl_clauses_string(&self) -> Result<String> {
        let clause_set = self.dump_dl_clauses()?;
        Ok(clause_set.to_string())
    }

    // Helper methods for reasoning

    /// Check if two classes are equivalent
    fn classes_equivalent(
        &self,
        class1: &ClassExpression,
        class2: &ClassExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(class1 == class2)
    }

    /// Check if two object properties are equivalent
    fn object_properties_equivalent(
        &self,
        prop1: &ObjectPropertyExpression,
        prop2: &ObjectPropertyExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }

    /// Check if two data properties are equivalent
    fn data_properties_equivalent(
        &self,
        prop1: &DataPropertyExpression,
        prop2: &DataPropertyExpression,
    ) -> Result<bool> {
        // Simple structural equality check
        Ok(prop1 == prop2)
    }

    /// Filter superclasses to only direct ones (remove transitively implied ones)
    fn filter_direct_superclasses(
        &self,
        superclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut direct = Vec::new();

        for superclass in &superclasses {
            let mut is_direct = true;

            // Check if this superclass is implied by any other superclass
            for other_superclass in &superclasses {
                if superclass != other_superclass
                    && self.is_subclass_of(other_superclass, superclass)?
                {
                    is_direct = false;
                    break;
                }
            }

            if is_direct {
                direct.push(superclass.clone());
            }
        }

        Ok(direct)
    }

    /// Filter subclasses to only direct ones (remove transitively implied ones)
    fn filter_direct_subclasses(
        &self,
        subclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut direct = Vec::new();

        for subclass in &subclasses {
            let mut is_direct = true;

            // Check if this subclass is implied by any other subclass
            for other_subclass in &subclasses {
                if subclass != other_subclass && self.is_subclass_of(subclass, other_subclass)? {
                    is_direct = false;
                    break;
                }
            }

            if is_direct {
                direct.push(subclass.clone());
            }
        }

        Ok(direct)
    }

    /// Get all inferred superclasses through transitive closure
    fn get_all_inferred_superclasses(
        &self,
        _class: &ClassExpression,
        direct_superclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut all_superclasses = direct_superclasses;
        let mut to_process = all_superclasses.clone();
        let mut processed = std::collections::HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&format!("{current:?}")) {
                continue;
            }
            processed.insert(format!("{current:?}"));

            // Get superclasses of current class (careful to avoid infinite recursion)
            if let Some(ontology_ref) = &self.ontology {
                let ontology = read_lock(
                    ontology_ref,
                    "core: reading ontology for get_all_inferred_superclasses",
                )?;
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom
                        && self.classes_equivalent(&subclass_axiom.subclass, &current)?
                    {
                        let superclass = &subclass_axiom.superclass;
                        if !all_superclasses
                            .iter()
                            .any(|c| self.classes_equivalent(c, superclass).unwrap_or(false))
                        {
                            all_superclasses.push(superclass.clone());
                            to_process.push(superclass.clone());
                        }
                    }
                }
            }
        }

        Ok(all_superclasses)
    }

    /// Get all inferred subclasses through transitive closure
    fn get_all_inferred_subclasses(
        &self,
        _class: &ClassExpression,
        direct_subclasses: Vec<ClassExpression>,
    ) -> Result<Vec<ClassExpression>> {
        let mut all_subclasses = direct_subclasses;
        let mut to_process = all_subclasses.clone();
        let mut processed = std::collections::HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&format!("{current:?}")) {
                continue;
            }
            processed.insert(format!("{current:?}"));

            // Get subclasses of current class (careful to avoid infinite recursion)
            if let Some(ontology_ref) = &self.ontology {
                let ontology = read_lock(
                    ontology_ref,
                    "core: reading ontology for get_all_inferred_subclasses",
                )?;
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom
                        && self.classes_equivalent(&subclass_axiom.superclass, &current)?
                    {
                        let subclass = &subclass_axiom.subclass;
                        if !all_subclasses
                            .iter()
                            .any(|c| self.classes_equivalent(c, subclass).unwrap_or(false))
                        {
                            all_subclasses.push(subclass.clone());
                            to_process.push(subclass.clone());
                        }
                    }
                }
            }
        }

        Ok(all_subclasses)
    }

    /// Get all classes mentioned in the ontology
    #[allow(dead_code)]
    fn get_all_classes_in_ontology(
        &self,
        _ontology: &crate::ontology::Ontology,
    ) -> Result<Vec<ClassExpression>> {
        self.get_all_classes_in_ontology_internal()
    }

    /// Get all classes mentioned in the ontology (internal implementation)
    fn get_all_classes_in_ontology_internal(&self) -> Result<Vec<ClassExpression>> {
        let mut classes = Vec::new();

        if let Some(ontology_ref) = &self.ontology {
            let ontology = read_lock(
                ontology_ref,
                "core: reading ontology for get_all_classes_in_ontology_internal",
            )?;
            for axiom in ontology.axioms() {
                match axiom {
                    crate::ontology::Axiom::SubClassOf(axiom) => {
                        classes.push(axiom.subclass.clone());
                        classes.push(axiom.superclass.clone());
                    }
                    crate::ontology::Axiom::EquivalentClasses(axiom) => {
                        classes.extend(axiom.classes.clone());
                    }
                    crate::ontology::Axiom::DisjointClasses(axiom) => {
                        classes.extend(axiom.classes.clone());
                    }
                    crate::ontology::Axiom::ClassAssertion(axiom) => {
                        classes.push(axiom.class.clone());
                    }
                    _ => {}
                }
            }
        }

        // Remove duplicates
        classes.sort_by_key(|c| format!("{c:?}"));
        classes.dedup_by_key(|c| format!("{c:?}"));

        Ok(classes)
    }

    /// Extract prefixes from a single axiom
    fn extract_prefixes_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        prefixes: &mut std::collections::HashMap<String, String>,
    ) {
        match axiom {
            crate::ontology::Axiom::ClassAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&assertion.class) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
                if let Some(iri) = assertion.individual.iri() {
                    self.add_prefix_from_iri(&iri.to_string(), prefixes);
                }
            }
            crate::ontology::Axiom::SubClassOf(axiom) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.subclass) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.superclass) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
            }
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_object_property(&assertion.property) {
                    self.add_prefix_from_iri(&iri, prefixes);
                }
            }
            _ => {} // Handle other axiom types as needed
        }
    }

    /// Extract IRI from class expression
    fn extract_iri_from_class_expression(&self, expr: &ClassExpression) -> Option<String> {
        match expr {
            ClassExpression::Class(class) => Some(class.iri.to_string()),
            _ => None,
        }
    }

    /// Extract IRI from object property expression
    fn extract_iri_from_object_property(&self, expr: &ObjectPropertyExpression) -> Option<String> {
        match expr {
            ObjectPropertyExpression::ObjectProperty(prop) => Some(prop.iri.to_string()),
            _ => None,
        }
    }

    /// Add prefix from IRI
    fn add_prefix_from_iri(
        &self,
        iri: &str,
        prefixes: &mut std::collections::HashMap<String, String>,
    ) {
        if let Some(hash_pos) = iri.rfind('#') {
            let base = &iri[..=hash_pos];
            if !prefixes.values().any(|v| v == base) {
                // Try to detect common namespaces
                let prefix_name = match base {
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#" => "rdf".to_string(),
                    "http://www.w3.org/2000/01/rdf-schema#" => "rdfs".to_string(),
                    "http://www.w3.org/2002/07/owl#" => "owl".to_string(),
                    "http://www.w3.org/2001/XMLSchema#" => "xsd".to_string(),
                    _ => format!("ns{}", prefixes.len()),
                };
                prefixes.insert(prefix_name, base.to_string());
            }
        }
    }

    /// Extract object properties from axiom
    ///
    /// Superseded by [`super::property_hierarchy::ObjectPropertyHierarchy`] for
    /// hierarchy queries; kept as the low-level "which properties does this
    /// axiom mention" helper.
    #[allow(dead_code)]
    fn extract_object_properties_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        properties: &mut std::collections::HashSet<ObjectPropertyExpression>,
    ) {
        match axiom {
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                properties.insert(assertion.property.clone());
            }
            crate::ontology::Axiom::SubObjectPropertyOf(axiom) => {
                properties.insert(axiom.sub_property.clone());
                properties.insert(axiom.super_property.clone());
            }
            crate::ontology::Axiom::ObjectPropertyDomain(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::ObjectPropertyRange(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::FunctionalObjectProperty(axiom) => {
                properties.insert(axiom.property.clone());
            }
            _ => {} // Handle other axiom types that mention object properties
        }
    }

    /// Extract data properties from axiom (see
    /// [`Self::extract_object_properties_from_axiom`]).
    #[allow(dead_code)]
    fn extract_data_properties_from_axiom(
        &self,
        axiom: &crate::ontology::Axiom,
        properties: &mut std::collections::HashSet<DataPropertyExpression>,
    ) {
        match axiom {
            crate::ontology::Axiom::DataPropertyAssertion(assertion) => {
                properties.insert(assertion.property.clone());
            }
            crate::ontology::Axiom::SubDataPropertyOf(axiom) => {
                properties.insert(axiom.sub_property.clone());
                properties.insert(axiom.super_property.clone());
            }
            crate::ontology::Axiom::DataPropertyDomain(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::DataPropertyRange(axiom) => {
                properties.insert(axiom.property.clone());
            }
            crate::ontology::Axiom::FunctionalDataProperty(axiom) => {
                properties.insert(axiom.property.clone());
            }
            _ => {} // Handle other axiom types that mention data properties
        }
    }

    /// Save DL clauses to file
    pub fn save_dl_clauses<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let clause_set = self.dump_dl_clauses()?;
        let content = clause_set.to_string();
        std::fs::write(path, content).map_err(|e| Error::Io {
            message: format!("Failed to write DL clauses: {e}"),
        })
    }

    /// Create a server manager for this reasoner
    /// This allows starting web servers (OWLlink, SPARQL, REST API) for remote access
    #[cfg(feature = "server")]
    pub fn create_server_manager(&self) -> Result<crate::server::ServerManager> {
        use std::sync::Arc;

        let ontology = self.ontology.as_ref().ok_or_else(|| Error::Reasoning {
            message: "No ontology loaded. Load an ontology before starting the server.".to_string(),
        })?;

        let ontology_clone = ontology
            .read()
            .map_err(|_| Error::Reasoning {
                message: "Failed to acquire ontology read lock".to_string(),
            })?
            .clone();

        let reasoning_service = Arc::new(crate::reasoning::ReasoningService::new(
            ontology_clone,
            self.config.clone(),
        )?);

        Ok(crate::server::ServerManager::new(
            self.config.server.clone(),
            reasoning_service,
        ))
    }

    /// Start a server with the specified configuration
    /// Returns a ServerManager that can be used to stop the server
    #[cfg(feature = "server")]
    pub async fn start_server(&self) -> Result<crate::server::ServerManager> {
        let mut server_manager = self.create_server_manager()?;
        server_manager.start_all().await?;
        Ok(server_manager)
    }

    /// Start a server on a specific port
    /// Returns a ServerManager that can be used to stop the server
    #[cfg(feature = "server")]
    pub async fn start_server_on_port(&self, port: u16) -> Result<crate::server::ServerManager> {
        use std::sync::Arc;

        let ontology = self.ontology.as_ref().ok_or_else(|| Error::Reasoning {
            message: "No ontology loaded. Load an ontology before starting the server.".to_string(),
        })?;

        let ontology_clone = ontology
            .read()
            .map_err(|_| Error::Reasoning {
                message: "Failed to acquire ontology read lock".to_string(),
            })?
            .clone();

        let reasoning_service = Arc::new(crate::reasoning::ReasoningService::new(
            ontology_clone,
            self.config.clone(),
        )?);

        let mut server_manager = crate::server::ServerManager::with_port(reasoning_service, port);
        server_manager.start_all().await?;
        Ok(server_manager)
    }

    // -------------------------------------------------------------------------
    // ABox materialization and SPARQL classification
    // -------------------------------------------------------------------------

    /// Materialize `ABox` facts using native OWL 2 RL forward-chaining rules.
    ///
    /// Returns statistics about the materialization run.
    pub fn materialize_abox(&mut self) -> Result<MaterializationStats> {
        let ontology_ref = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded for ABox materialization"))?
            .clone();

        let start = std::time::Instant::now();

        let mut rl = crate::profiles::rl_reasoner::RLReasoner::new(self.config.clone());
        {
            let ontology = read_lock(&ontology_ref, "core: reading ontology for materialize_abox")?;
            rl.initialize(&ontology)?;
        }
        rl.materialize()?;

        let facts_added = rl.materialized_facts_count();

        // Push inferred ClassAssertions back into the ontology
        {
            use crate::ontology::{Axiom, axioms::ClassAssertionAxiom};
            let mut ont_w = write_lock(&ontology_ref, "core: writing inferred class assertions")?;
            for (ind, classes) in rl.class_assertion_pairs() {
                for cls in classes {
                    ont_w.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        individual: ind.clone(),
                        class: cls,
                        annotations: Vec::new(),
                    }));
                }
            }
        }

        // Store the RL reasoner for later queries
        self.rl_reasoner = Some(Box::new(rl));
        self.materialization_stale
            .store(false, std::sync::atomic::Ordering::Relaxed);
        // Inferred class assertions were pushed into the ontology above.
        self.invalidate_query_caches();

        // Mark Oxigraph store as stale so it gets rebuilt on next SPARQL query
        #[cfg(feature = "sparql-store")]
        if let Ok(mut guard) = self.oxigraph_store.lock() {
            *guard = None;
        }

        Ok(MaterializationStats {
            rules_fired: 0,
            facts_added,
            iterations: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            sparql_rules_run: 0,
        })
    }

    /// Add a custom SPARQL INSERT WHERE rule to run during `ABox` classification.
    pub fn add_sparql_materialization_rule(&mut self, sparql: &str) {
        self.sparql_materialization_rules.push(sparql.to_owned());
    }

    /// Run SPARQL-based `ABox` classification using Oxigraph.
    ///
    /// Loads the ontology + any materialized facts into an Oxigraph in-memory
    /// store, executes a fixpoint loop of INSERT WHERE rules, then syncs the
    /// newly inferred `rdf:type` triples back into the ontology as
    /// `ClassAssertion` axioms.
    #[cfg(feature = "sparql-store")]
    pub fn run_sparql_abox_classification(&mut self) -> Result<MaterializationStats> {
        use crate::core::reasoner::oxigraph_store::OxigraphStore;
        use crate::ontology::{
            Axiom, Class, IRI, Individual, NamedIndividual, axioms::ClassAssertionAxiom,
        };

        let ontology_ref = self
            .ontology
            .as_ref()
            .ok_or_else(|| Error::reasoning("No ontology loaded for SPARQL ABox classification"))?
            .clone();

        let start = std::time::Instant::now();
        let store = OxigraphStore::new()?;

        {
            let ontology = read_lock(
                &ontology_ref,
                "core: reading ontology for sparql classification",
            )?;
            store.load_from_ontology(&ontology)?;
        }

        // Built-in OWL 2 RL SPARQL rules -----------------------------------------
        let builtin_rules = abox_classification_rules();
        let rules_marker = &[
            // Rule 1: hasValue restriction — C ≡ (p hasValue v)
            "INSERT { ?x a ?C } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?R . \
              ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                   <http://www.w3.org/2002/07/owl#Restriction> ; \
                 <http://www.w3.org/2002/07/owl#onProperty> ?P ; \
                 <http://www.w3.org/2002/07/owl#hasValue>    ?V . \
              ?x ?P ?V . \
              FILTER NOT EXISTS { ?x a ?C } \
            }",
            // Rule 2: someValuesFrom restriction — C ≡ (p someValuesFrom D)
            "INSERT { ?x a ?C } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?R . \
              ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                   <http://www.w3.org/2002/07/owl#Restriction> ; \
                 <http://www.w3.org/2002/07/owl#onProperty>    ?P ; \
                 <http://www.w3.org/2002/07/owl#someValuesFrom> ?D . \
              ?x ?P ?y . \
              ?y a ?D . \
              FILTER NOT EXISTS { ?x a ?C } \
            }",
            // Rule 3a: named class equivalence C ≡ D → D instances become C instances
            "INSERT { ?x a ?C } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?D . \
              FILTER(!isBlank(?D)) \
              ?x a ?D . \
              FILTER NOT EXISTS { ?x a ?C } \
            }",
            // Rule 3b: named class equivalence C ≡ D → C instances become D instances
            "INSERT { ?x a ?D } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?D . \
              FILTER(!isBlank(?D)) \
              ?x a ?C . \
              FILTER NOT EXISTS { ?x a ?D } \
            }",
            // Rule 4: intersectionOf( DataSomeValuesFrom(dp, xsd:double range), DataHasValue(hvp, hv) )
            "INSERT { ?x a ?C } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
              ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
              ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_range . \
              ?r_range <http://www.w3.org/2002/07/owl#onProperty>    ?dp . \
              ?r_range <http://www.w3.org/2002/07/owl#someValuesFrom> ?dr . \
              ?dr <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
              ?dr <http://www.w3.org/2002/07/owl#withRestrictions> ?flist . \
              ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_hv . \
              ?r_hv <http://www.w3.org/2002/07/owl#onProperty> ?hvp . \
              ?r_hv <http://www.w3.org/2002/07/owl#hasValue>    ?hv . \
              FILTER(?r_range != ?r_hv) \
              ?x ?dp  ?numval . \
              ?x ?hvp ?hv . \
              OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_mi . \
                         ?f_mi <http://www.w3.org/2001/XMLSchema#minInclusive> ?minI . } \
              OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_me . \
                         ?f_me <http://www.w3.org/2001/XMLSchema#minExclusive> ?minE . } \
              OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_xi . \
                         ?f_xi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?maxI . } \
              OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_xe . \
                         ?f_xe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?maxE . } \
              FILTER((!BOUND(?minI) || ?numval >= ?minI) && \
                     (!BOUND(?minE) || ?numval >  ?minE) && \
                     (!BOUND(?maxI) || ?numval <= ?maxI) && \
                     (!BOUND(?maxE) || ?numval <  ?maxE)) \
              FILTER NOT EXISTS { ?x a ?C } \
            }",
            // Rule 5: pure named-class intersectionOf — all members must be named classes
            //         and x must be an instance of every one of them
            "INSERT { ?x a ?C } \
            WHERE { \
              ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
              ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
              FILTER NOT EXISTS { \
                ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m . \
                FILTER(isBlank(?m)) \
              } \
              FILTER NOT EXISTS { \
                ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m2 . \
                FILTER NOT EXISTS { ?x a ?m2 } \
              } \
              ?ilist <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?m1 . \
              ?x a ?m1 . \
              FILTER NOT EXISTS { ?x a ?C } \
            }",
            // Rule 6: rdfs:subClassOf propagation
            "INSERT { ?x a ?super } \
            WHERE { \
              ?sub <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?super . \
              FILTER(!isBlank(?super)) \
              ?x a ?sub . \
              FILTER NOT EXISTS { ?x a ?super } \
            }",
            // Rule 7: rdfs:domain propagation
            "INSERT { ?x a ?C } WHERE { ?x ?P ?y . ?P <http://www.w3.org/2000/01/rdf-schema#domain> ?C . FILTER NOT EXISTS { ?x a ?C } FILTER(isIRI(?C)) }",
            // Rule 8: rdfs:range propagation
            "INSERT { ?y a ?C } WHERE { ?x ?P ?y . ?P <http://www.w3.org/2000/01/rdf-schema#range> ?C . FILTER NOT EXISTS { ?y a ?C } FILTER(isIRI(?C)) }",
        ];
        let _ = rules_marker;

        let all_rules: Vec<&str> = builtin_rules
            .iter()
            .copied()
            .chain(self.sparql_materialization_rules.iter().map(String::as_str))
            .collect();

        let max_iterations = 20usize;
        let mut total_sparql_runs = 0u64;

        for _ in 0..max_iterations {
            let mut changed = false;
            for rule in &all_rules {
                let count = store.execute_update(rule)?;
                if count > 0 {
                    changed = true;
                }
                total_sparql_runs += 1;
            }
            if !changed {
                break;
            }
        }

        // Sync newly inferred ClassAssertions back into the ontology
        let class_query = "SELECT ?x ?C WHERE { ?x a ?C . FILTER(isIRI(?x)) FILTER(isIRI(?C)) }";
        let json_result = store.execute_query(class_query)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_result)
            .map_err(|e| Error::reasoning(format!("Failed to parse SPARQL result: {e}")))?;

        let mut facts_added = 0usize;
        if let Some(bindings) = parsed["results"]["bindings"].as_array() {
            let mut ont_w = write_lock(&ontology_ref, "core: writing SPARQL inferred assertions")?;
            for binding in bindings {
                let ind_iri = binding["x"]["value"].as_str().unwrap_or("").to_owned();
                let cls_iri = binding["C"]["value"].as_str().unwrap_or("").to_owned();
                if ind_iri.is_empty() || cls_iri.is_empty() {
                    continue;
                }
                // Skip owl:Thing / owl:Nothing / rdf:type etc. as class assertions
                if cls_iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
                    || cls_iri == "http://www.w3.org/2002/07/owl#Thing"
                    || cls_iri == "http://www.w3.org/2002/07/owl#Nothing"
                {
                    continue;
                }
                let axiom = Axiom::ClassAssertion(ClassAssertionAxiom {
                    id: 0,
                    individual: Individual::Named(NamedIndividual {
                        iri: IRI::new(&ind_iri),
                    }),
                    class: crate::ontology::ClassExpression::Class(Class::new(IRI::new(&cls_iri))),
                    annotations: Vec::new(),
                });
                ont_w.add_axiom(axiom);
                facts_added += 1;
            }
        }

        // Cache the store for subsequent SPARQL queries
        if let Ok(mut guard) = self.oxigraph_store.lock() {
            *guard = Some(store);
        }
        self.materialization_stale
            .store(false, std::sync::atomic::Ordering::Relaxed);
        // Inferred `rdf:type` triples were synced back into the ontology above.
        self.invalidate_query_caches();

        Ok(MaterializationStats {
            rules_fired: 0,
            facts_added,
            iterations: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            sparql_rules_run: total_sparql_runs,
        })
    }

    /// Execute a SPARQL CONSTRUCT / INSERT WHERE via Oxigraph and return the
    /// number of triples that were added.
    ///
    /// # Panics
    ///
    /// Panics if the internal Oxigraph store has not been initialized.
    #[cfg(feature = "sparql-store")]
    pub fn execute_construct_update(&mut self, sparql: &str) -> Result<usize> {
        use crate::core::reasoner::oxigraph_store::OxigraphStore;

        let mut guard = self
            .oxigraph_store
            .lock()
            .map_err(|_| Error::reasoning("Oxigraph store mutex poisoned"))?;

        if guard.is_none() {
            let ontology_ref = self
                .ontology
                .as_ref()
                .ok_or_else(|| Error::reasoning("No ontology loaded"))?
                .clone();
            let store = OxigraphStore::new()?;
            let ont = read_lock(&ontology_ref, "core: reading ontology for construct_update")?;
            store.load_from_ontology(&ont)?;
            *guard = Some(store);
        }

        guard.as_ref().unwrap().execute_update(sparql)
    }
}

/// B1 completeness guard + metadata parity for the EL fast path.
///
/// Returns `false` when the EL result does not key the hierarchy by **every**
/// named class of the ontology signature plus `owl:Thing` — exactly what the
/// general classification produces (`signature() + owl:Thing`, see
/// `ClassificationService::classify`) — in which case the caller must fall
/// back to the general path.  (It also returns `false` when the signature
/// cannot be read.)
///
/// On success it fills in the metadata the general classification always
/// sets, so an `Auto` run writes the same document as a `DL` run: the
/// ontology IRI (HermiT-style header) and the sorted object/data property
/// lists consumed by `ClassificationResult::write_object_properties` /
/// `write_data_properties`.
fn finalise_el_result(ontology: &OntologyRef, result: &mut ClassificationResult) -> bool {
    let Ok(guard) = read_lock(ontology, "classify: EL result finalisation") else {
        return false;
    };
    let Ok(signature) = guard.signature() else {
        return false;
    };

    let owl_thing = ClassExpression::Class(crate::ontology::Class::new(
        crate::ontology::IRI::owl_thing(),
    ));
    let covers_signature = signature.classes.iter().all(|class| {
        result
            .hierarchy
            .contains_key(&ClassExpression::Class(class.clone()))
    }) && result.hierarchy.contains_key(&owl_thing);

    if !covers_signature {
        return false;
    }

    result.ontology_iri = guard.id.ontology_iri.as_ref().map(ToString::to_string);

    let mut object_properties: Vec<String> = signature
        .object_properties
        .iter()
        .map(|p| p.iri.to_string())
        .collect();
    object_properties.sort();
    object_properties.dedup();
    result.object_properties = object_properties;

    let mut data_properties: Vec<String> = signature
        .data_properties
        .iter()
        .map(|p| p.iri.to_string())
        .collect();
    data_properties.sort();
    data_properties.dedup();
    result.data_properties = data_properties;

    true
}

/// Returns `true` when an (EL-conforming) ontology is fully covered by the
/// A8 absorption + dependency-graph classification path — i.e. every axiom is
/// either deterministically absorbed (told subsumption, role-successor /
/// domain / range / conjunction GCIs, equivalents) or pushed to the tableau
/// fallback without silently losing consequences.
///
/// The A8 path is *incomplete* for role-hierarchy / transitive-role driven
/// existential consequences (`R ⊑ S` ⇒ `∃R.C` does not propagate to `∃S.C`),
/// so ontologies using any role `RBox` axiom still use the EL reasoner.  It is
/// also conservative about EL constructs whose consequences the deterministic
/// propagation does not model (nominals, `hasSelf`, cardinality restrictions):
/// those are flagged for the tableau, but for EL ontologies the EL reasoner is
/// cheaper, so we keep them on the EL path too.
fn el_covered_by_absorption(ontology: &crate::ontology::Ontology) -> bool {
    use crate::ontology::{ClassExpression, axioms::Axiom};

    fn absorption_safe_expr(expr: &ClassExpression) -> bool {
        match expr {
            // Deterministic EL fragment the A8 path is complete for.
            ClassExpression::Class(_)
            | ClassExpression::DataSomeValuesFrom { .. }
            | ClassExpression::DataAllValuesFrom { .. } => true,
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => absorption_safe_expr(filler),
            ClassExpression::ObjectIntersectionOf(parts) => parts.iter().all(absorption_safe_expr),
            // Constructs the deterministic propagation does not model.
            ClassExpression::ObjectOneOf(_)
            | ClassExpression::ObjectHasSelf { .. }
            | ClassExpression::ObjectHasValue { .. }
            | ClassExpression::ObjectMinCardinality { .. }
            | ClassExpression::ObjectMaxCardinality { .. }
            | ClassExpression::ObjectExactCardinality { .. }
            | ClassExpression::DataHasValue { .. }
            | ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. }
            | ClassExpression::ObjectUnionOf(_)
            | ClassExpression::ObjectComplementOf(_)
            | ClassExpression::ObjectAllValuesFrom { .. } => false,
        }
    }

    for axiom in ontology.axioms() {
        match axiom {
            // Role RBox axioms → A8 existential propagation is incomplete.
            Axiom::SubObjectPropertyOf(_)
            | Axiom::EquivalentObjectProperties(_)
            | Axiom::DisjointObjectProperties(_)
            | Axiom::InverseObjectProperties(_)
            | Axiom::FunctionalObjectProperty(_)
            | Axiom::InverseFunctionalObjectProperty(_)
            | Axiom::ReflexiveObjectProperty(_)
            | Axiom::IrreflexiveObjectProperty(_)
            | Axiom::SymmetricObjectProperty(_)
            | Axiom::AsymmetricObjectProperty(_)
            | Axiom::TransitiveObjectProperty(_) => return false,
            // Class axioms: inspect the expressions for disallowed constructs.
            Axiom::SubClassOf(ax) => {
                if !absorption_safe_expr(&ax.subclass) || !absorption_safe_expr(&ax.superclass) {
                    return false;
                }
            }
            Axiom::EquivalentClasses(ax) => {
                if ax.classes.iter().any(|c| !absorption_safe_expr(c)) {
                    return false;
                }
            }
            Axiom::ObjectPropertyDomain(_) | Axiom::ObjectPropertyRange(_) => {}
            Axiom::DisjointClasses(_) | Axiom::DisjointUnion(_) => {}
            Axiom::SubDataPropertyOf(_)
            | Axiom::EquivalentDataProperties(_)
            | Axiom::DataPropertyDomain(_)
            | Axiom::DataPropertyRange(_) => {}
            Axiom::FunctionalDataProperty(_) => {}
            Axiom::Declaration(_) | Axiom::AnnotationAssertion(_) => {}
            // Anything else (ABox, HasKey, DatatypeDefinition, rules, …) is not
            // handled by the A8 TBox classification path.
            _ => return false,
        }
    }
    true
}
