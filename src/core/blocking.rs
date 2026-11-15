//! Blocking strategies for tableau reasoning
//!
//! This module implements various blocking techniques used to ensure termination
//! of tableau algorithms while preserving correctness and completeness.
//!
//! # Phase A — cached signatures
//!
//! Every checker keeps a per-node [`NodeSignature`] cache keyed by
//! [`TableauNode::label_version`]: a node's signature is re-hashed only when
//! its label set actually changed (insert, clear, backtrack restore, merge).
//! Pairwise comparisons are then `u64` binary searches plus [`ConceptLabel`]
//! equality only inside a hash-collision bucket — the previous implementation
//! re-cloned and re-hashed every node's *entire* concept set for *every* pair
//! of every `update_blocking` pass (which runs after each rule application),
//! making blocking O(n² × labels) hashing per rule application.
//!
//! # Phase C — strategy identity without formatting
//!
//! Adaptive checkers ([`DynamicBlocking`], [`OptimalBlocking`]) identify their
//! inner strategy through [`BlockingChecker::kind`] (a [`StrategyKind`] `Copy`
//! comparison) instead of `format!("{:?}", …).contains("…Blocking")`, which
//! rendered the strategy *and its signature cache* on every pass.
//!
//! # Phase E — fast hashing, two-pointer subset, blocker candidacy
//!
//! 1. **Deterministic fast hashing.** [`concept_sort_key`] and the
//!    [`SignatureCache`] map now use [`FastHasher`](crate::core::fast_hash)
//!    (seedless FNV-1a) instead of SipHash. The digest is a pure function of
//!    the label's `Hash` bytes — same value every process/run/architecture —
//!    and remains an internal cache key only: every positive answer is
//!    re-verified with [`ConceptLabel`] equality, so a collision can never
//!    change a decision.
//! 2. **Two-pointer subset/equality.** [`NodeSignature::subsumes`] and
//!    [`NodeSignature::equals`] merge the two hash-sorted entry vectors in
//!    O(n + m) instead of running a binary search per entry (O(n log m)),
//!    resolving equal hashes with exact [`ConceptLabel`] equality.
//! 3. **Blocker candidacy.** [`NodeType::Root`] and [`NodeType::Nominal`]
//!    nodes are never used as *blockers* (see [`may_be_blocker`]): they are
//!    not legitimate witnesses for generated-node blocking, and skipping them
//!    removes the per-pass signature comparison against every nominal node
//!    (one per individual) from the O(n²) candidate scan.
use crate::{
    Result,
    config::BlockingStrategy as ConfigBlockingStrategy,
    core::{
        fast_hash::{FastBuildHasher, fast_hash},
        tableau::{ConceptLabel, NodeId, NodeType, TableauNode},
    },
};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Compute a deterministic sort key for a [`ConceptLabel`].
///
/// Zero-allocation alternative to `format!("{c:?}")`. Phase E: hashed with
/// the seedless [`FastHasher`](crate::core::fast_hash) — deterministic across
/// processes (unlike `RandomState`) and ~2-3× cheaper than SipHash on the
/// per-label hot path. See `fast_hash` for the full determinism contract.
#[inline]
fn concept_sort_key(c: &ConceptLabel) -> u64 {
    fast_hash(c)
}

/// Whether a node carries a nominal label `{a}`.
///
/// Nominal labels are "root-like": a node whose label set contains a nominal is a
/// genuine individual witness and must never be blocked (blocked nodes are not
/// expanded, which would be unsound for nominals).
///
/// Phase E: nodes of type [`NodeType::Nominal`] are additionally excluded as
/// *blockers* (see [`may_be_blocker`]); a nominal *label* carried by an
/// otherwise-eligible blocker still participates in subsumption — the
/// blocker's `{a}` must be present in the blocked node's label for it to
/// block.
#[must_use]
pub fn contains_nominal_label(node: &TableauNode) -> bool {
    node.concepts
        .iter()
        .any(|c| matches!(c, ConceptLabel::Nominal(_)))
}

/// Whether `node` may be *marked* blocked by a checker.
///
/// Root nodes encode `TBox` axioms (meta-level artifacts) and nominal nodes are
/// genuine individual witnesses; the executor clears their blocking flags after
/// every pass anyway, so the checkers skip them up front.
#[inline]
fn may_be_blocked(node: &TableauNode) -> bool {
    !matches!(node.node_type, NodeType::Nominal | NodeType::Root) && !contains_nominal_label(node)
}

/// Whether `node` may serve as a *blocker* for another node (Phase E).
///
/// Root and Nominal nodes are never legitimate witnesses for generated-node
/// blocking:
///
/// * the **Root** encodes `TBox` axioms — a meta-level artifact, not an
///   individual witness (the executor post-filter already rejected
///   root-blocked nodes before Phase E; this moves the rejection into the
///   candidate scan itself);
/// * a **Nominal** node is a named-individual witness that is itself never
///   blocked. In the greenhouse workload there is one nominal per individual,
///   and they dominated the O(n²) candidate scan while never legitimately
///   blocking generated nodes.
///
/// # Why blocking decisions do not change (nominals)
///
/// Every `Nominal`-typed node carries its `{a}` [`ConceptLabel::Nominal`]
/// label, and every node that [`may_be_blocked`] has *no* `{a}` label (by
/// definition of `may_be_blocked`). A nominal node's label set therefore can
/// never be a subset of a blockable node's label set — excluding nominal
/// candidates removes comparisons that could never have succeeded anyway.
/// Root blockers were already rejected by the executor post-filter, so
/// moving the filter here is decision-preserving on both counts.
///
/// # Termination (weakened blocking, finite labels)
///
/// Ignoring candidates only *weakens* blocking in general, so termination
/// needs its own argument: labels are drawn from the **finite** set of
/// ontology subexpressions, and under any subset/equality strategy
/// (`Anywhere`, `Pairwise`, `Single`, `Core`, `Indexed`, and the fast paths
/// of `Optimal`/`Dynamic`) two unblocked non-nominal nodes must have
/// **distinct** labels — equal labels would make the earlier eligible
/// non-nominal node block the later one. Hence at most
/// |subexpressions| non-nominal nodes are ever unblocked, blocked nodes are
/// never expanded, and the excluded nominal/root nodes are finitely many
/// (one per individual + the root) whose own rule queues drain without
/// depending on blocking. `AncestorBlocking` keeps its ancestor-chain
/// argument: a generated node's equal-label ancestor is itself non-nominal
/// (a nominal ancestor always carries the extra `{a}` label), so excluding
/// nominal ancestors removes no equality candidate from a chain either.
#[inline]
pub fn may_be_blocker(node: &TableauNode) -> bool {
    !matches!(node.node_type, NodeType::Root | NodeType::Nominal)
}

/// A hash-sorted view of one node's concept set, computed once per
/// `label_version` and reused across all pairwise comparisons.
#[derive(Debug, Clone)]
pub struct NodeSignature {
    /// `(concept_hash, label)` pairs sorted ascending by `concept_hash`.
    /// `concept_hash = concept_sort_key(label)` is computed exactly once per
    /// label per recomputation.
    entries: Vec<(u64, ConceptLabel)>,
    /// Order-independent fold of the concept hashes. Equal label sets always
    /// produce an equal `set_hash`, so a *mismatch proves inequality* (fast
    /// negative check). It is never used as a positive proof: positives are
    /// always verified by [`ConceptLabel`] equality.
    set_hash: u64,
}

impl NodeSignature {
    /// Build a signature from an iterator of labels (hash, sort, fold).
    fn build<'a>(labels: impl Iterator<Item = &'a ConceptLabel>) -> Self {
        let mut entries: Vec<(u64, ConceptLabel)> =
            labels.map(|l| (concept_sort_key(l), l.clone())).collect();
        // Sort by the pre-computed hash key — ties keep arbitrary order; the
        // binary search below always resolves them with label equality.
        entries.sort_unstable_by_key(|(h, _)| *h);
        let set_hash = entries.iter().fold(0u64, |acc, (h, _)| {
            // Multiset (order-independent) combination: equal sets ⇒ equal sum.
            acc.wrapping_add(h.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        });
        Self { entries, set_hash }
    }

    /// Build the signature of a tableau node's current label set.
    #[must_use]
    pub fn of_node(node: &TableauNode) -> Self {
        Self::build(node.concepts.iter())
    }

    /// Number of concepts in the signature.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the signature covers no concepts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Exact membership: binary search on the concept hash, then
    /// [`ConceptLabel`] equality over the (usually single-element) collision
    /// bucket — hash equality alone is never trusted.
    #[must_use]
    pub fn contains(&self, hash: u64, label: &ConceptLabel) -> bool {
        let start = self.entries.partition_point(|(h, _)| *h < hash);
        self.entries[start..]
            .iter()
            .take_while(|(h, _)| *h == hash)
            .any(|(_, l)| l == label)
    }

    /// Exact membership of a label (re-hashes `label` once).
    #[must_use]
    pub fn contains_label(&self, label: &ConceptLabel) -> bool {
        self.contains(concept_sort_key(label), label)
    }

    /// Two-pointer subset test over the two hash-sorted entry vectors
    /// (Phase E).
    ///
    /// `self ⊆ other`: advance a cursor through `other`; for each `self`
    /// entry skip `other` entries with a smaller hash, then scan the
    /// (usually single-element) hash-collision bucket for an exact
    /// [`ConceptLabel`] equality. O(n + m) with no per-entry binary search
    /// and no allocation. Hash equality alone is never trusted — a match is
    /// always confirmed by label equality.
    fn is_subset_of(&self, other: &Self) -> bool {
        // Length pre-filter is valid for label *sets* (what nodes hold).
        if self.entries.len() > other.entries.len() {
            return false;
        }
        let a = &self.entries;
        let b = &other.entries;
        let mut j = 0usize;
        for (ha, la) in a {
            // Advance past `other` entries whose hash is strictly smaller.
            while j < b.len() && b[j].0 < *ha {
                j += 1;
            }
            // Scan the collision bucket [j, k) of equal hashes for label equality.
            let mut k = j;
            let mut found = false;
            while k < b.len() && b[k].0 == *ha {
                if &b[k].1 == la {
                    found = true;
                    break;
                }
                k += 1;
            }
            if !found {
                return false;
            }
            // `j` intentionally stays at the bucket start: later `self`
            // entries may share this hash (collision) and re-scan the bucket.
        }
        true
    }

    /// Whether `self ⊆ other` (exact; hash collisions resolved by label
    /// equality). The length pre-filter is valid for label *sets* (which is
    /// what nodes hold — `concepts` is a `HashSet`).
    #[must_use]
    pub fn subsumes(&self, other: &Self) -> bool {
        self.is_subset_of(other)
    }

    /// Exact set equality: length, `set_hash` fast reject, then the
    /// two-pointer subset check (equal lengths + subset over unique labels
    /// ⇒ equality).
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        // Sound negative: equal sets always fold to the same `set_hash`.
        if self.set_hash != other.set_hash {
            return false;
        }
        self.is_subset_of(other)
    }
}

/// Per-checker cache of node signatures, valid as long as the recorded
/// `label_version` matches [`TableauNode::label_version`].
///
/// Phase E: keyed by [`FastBuildHasher`](crate::core::fast_hash) — the map
/// only ever needs a stable bucket key, and SipHash on `NodeId` keys was a
/// measurable slice of `update_blocking`.
#[derive(Debug, Default)]
struct SignatureCache {
    /// `node id → (label_version, signature)`.
    entries: HashMap<NodeId, (u64, NodeSignature), FastBuildHasher>,
    /// Debug counter: total signature recomputations over the checker's life.
    recomputations: u64,
}

impl SignatureCache {
    /// Drop entries for node ids that no longer exist (backtracking truncation).
    fn prune(&mut self, node_count: usize) {
        self.entries.retain(|&id, _| id < node_count);
    }

    /// Recompute the node's signature only when its `label_version` changed.
    fn refresh(&mut self, node: &TableauNode) {
        match self.entries.get(&node.id) {
            Some((v, _)) if *v == node.label_version => {}
            _ => {
                self.entries
                    .insert(node.id, (node.label_version, NodeSignature::of_node(node)));
                self.recomputations = self.recomputations.saturating_add(1);
            }
        }
    }

    /// Cached signature for `node`, if it is valid for the current version.
    fn get(&self, node: &TableauNode) -> Option<&NodeSignature> {
        self.entries
            .get(&node.id)
            .and_then(|(v, sig)| (*v == node.label_version).then_some(sig))
    }
}

/// A signature reference that is either borrowed from the cache (hot path —
/// `update_blocking` refreshes every node first) or computed on the spot
/// (cold path — a direct `is_blocked` call on a fresh checker).
enum SignatureRef<'a> {
    Cached(&'a NodeSignature),
    Fresh(NodeSignature),
}

impl std::ops::Deref for SignatureRef<'_> {
    type Target = NodeSignature;

    fn deref(&self) -> &NodeSignature {
        match self {
            Self::Cached(sig) => sig,
            Self::Fresh(sig) => sig,
        }
    }
}

/// Signature of `node` as seen by `cache` (recomputed transiently when the
/// cache has no valid entry — never inserted, so `&self` paths stay sound).
fn signature_of<'a>(cache: &'a SignatureCache, node: &'a TableauNode) -> SignatureRef<'a> {
    match cache.get(node) {
        Some(sig) => SignatureRef::Cached(sig),
        None => SignatureRef::Fresh(NodeSignature::of_node(node)),
    }
}

/// The trait's `Vec<ConceptLabel>` view (sorted by concept hash) of a node's
/// signature — cloned only when a caller actually asks for it.
fn signature_vec(cache: &SignatureCache, node: &TableauNode) -> Vec<ConceptLabel> {
    signature_of(cache, node)
        .entries
        .iter()
        .map(|(_, l)| l.clone())
        .collect()
}

/// Prune stale ids and (re)compute signatures for every live node so the
/// decision loop below only ever borrows cached signatures.
fn refresh_all(cache: &mut SignatureCache, nodes: &[TableauNode]) {
    cache.prune(nodes.len());
    for node in nodes {
        cache.refresh(node);
    }
}

/// Generic pairwise blocker: the first node `other` (in node order) that is
/// `eligible`, is a legitimate witness ([`may_be_blocker`]), and whose
/// signature is a subset of `node`'s blocks `node`.
fn first_blocker<F>(
    cache: &SignatureCache,
    node: &TableauNode,
    nodes: &[TableauNode],
    eligible: F,
) -> Option<NodeId>
where
    F: Fn(NodeId) -> bool,
{
    if !may_be_blocked(node) {
        return None;
    }
    let node_signature = signature_of(cache, node);
    for other in nodes {
        // Phase E: Root/Nominal nodes are never blockers (see `may_be_blocker`).
        if other.id == node.id || !may_be_blocker(other) || !eligible(other.id) {
            continue;
        }
        let other_signature = signature_of(cache, other);
        if other_signature.subsumes(&node_signature) {
            return Some(other.id);
        }
    }
    None
}

/// The set of strict ancestors of `node` derived from the parent map
/// (cycle-guarded: role-successor structure may contain loops after merges).
fn ancestor_set(node: NodeId, parents: &HashMap<NodeId, NodeId>) -> HashSet<NodeId> {
    let mut out = HashSet::new();
    let mut current = node;
    while let Some(&parent) = parents.get(&current) {
        if !out.insert(parent) {
            break; // cycle
        }
        current = parent;
        if out.len() > parents.len() {
            break; // defensive bound
        }
    }
    out
}

/// Identity of a blocking strategy, readable on hot paths **without** formatting.
///
/// Phase C: the adaptive checkers ([`DynamicBlocking`], [`OptimalBlocking`])
/// previously identified their inner strategy by `format!("{:?}",
/// self.current_strategy).contains("AncestorBlocking")` — a full `Debug`
/// rendering of the strategy *including its signature cache* on every
/// `update_blocking` pass (≈10% of `AnywhereBlocking`/`OptimalBlocking` time in
/// the consumer profile, spent inside `alloc::fmt::format_inner`). This enum
/// answers the same question with a `Copy` comparison.
///
/// The `Debug`-substring test and the enum test agree for every strategy this
/// crate builds: each concrete checker's `Debug` name is its type name, and no
/// checker is ever nested inside a *different* kind of checker (the adaptive
/// wrappers only ever hold `Anywhere` → `Ancestor`/`Core`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyKind {
    /// [`AnywhereBlocking`] — subset blocking against any earlier node.
    Anywhere,
    /// [`AncestorBlocking`] — subset blocking against ancestors only.
    Ancestor,
    /// [`PairwiseBlocking`] — subset blocking against not-yet-compared nodes.
    Pairwise,
    /// [`DynamicBlocking`] — adaptive wrapper around another strategy.
    Dynamic,
    /// [`SingleBlocking`] — equality (HermiT-style) blocking.
    Single,
    /// [`CoreBlocking`] — HermiT core (identity on this label representation).
    Core,
    /// [`OptimalBlocking`] — adaptive wrapper around another strategy.
    Optimal,
    /// [`IndexedAnywhereBlocking`] — anywhere blocking with a candidate index.
    Indexed,
}

/// Trait for blocking checkers
pub trait BlockingChecker: Send + Sync + std::fmt::Debug {
    /// The kind of this strategy — allocation-free, so adaptive wrappers can
    /// test their inner strategy without `Debug`-formatting it.
    ///
    /// Deliberately a *required* method: a silent default would let a new
    /// checker answer the wrong kind and silently break the adaptation logic.
    fn kind(&self) -> StrategyKind;

    /// Check if a node is blocked by another node
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId>;

    /// Update blocking information after node changes
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()>;

    /// Get the blocking signature for a node
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel>;

    /// Debug counter: total number of signature recomputations performed by
    /// this checker (0 for checkers without a signature cache).
    fn signature_recomputations(&self) -> u64 {
        0
    }
}

/// Factory for creating blocking checkers
pub struct BlockingStrategy;

impl BlockingStrategy {
    /// Create a blocking checker based on the configured strategy.
    ///
    /// The executor invokes this factory once per tableau run and reuses the
    /// resulting checker across iterations so that strategy-local state (e.g.
    /// signature caches, pairwise comparison sets) persists between updates.
    pub fn create_checker(strategy: ConfigBlockingStrategy) -> Result<Box<dyn BlockingChecker>> {
        match strategy {
            ConfigBlockingStrategy::Anywhere => Ok(Box::new(AnywhereBlocking::new())),
            ConfigBlockingStrategy::Ancestor => Ok(Box::new(AncestorBlocking::new())),
            ConfigBlockingStrategy::Pairwise => Ok(Box::new(PairwiseBlocking::new())),
            ConfigBlockingStrategy::Dynamic => Ok(Box::new(DynamicBlocking::new())),
            ConfigBlockingStrategy::Single => Ok(Box::new(SingleBlocking::new())),
            ConfigBlockingStrategy::Core => Ok(Box::new(CoreBlocking::new())),
            ConfigBlockingStrategy::Optimal => Ok(Box::new(OptimalBlocking::new())),
            ConfigBlockingStrategy::Indexed => Ok(Box::new(IndexedAnywhereBlocking::new())),
        }
    }
}

/// Anywhere blocking strategy (subset-based: an earlier node whose label is a
/// subset of the node's label blocks it).
#[derive(Debug)]
pub struct AnywhereBlocking {
    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl Default for AnywhereBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl AnywhereBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: SignatureCache::default(),
        }
    }
}

impl BlockingChecker for AnywhereBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Anywhere
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        // Nominal/root nodes are never blocked; Phase E: they are never
        // blockers either (see `may_be_blocker`).
        first_blocker(&self.cache, node, nodes, |id| id < node.id)
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        refresh_all(&mut self.cache, nodes);

        // Update blocking information for all nodes
        for i in 0..nodes.len() {
            let blocker = first_blocker(&self.cache, &nodes[i], nodes, |id| id < nodes[i].id);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

/// Ancestor blocking strategy (subset-based, but only against ancestors)
#[derive(Debug)]
pub struct AncestorBlocking {
    /// Parent relationships between nodes — rebuilt from
    /// `TableauNode::role_successors` on every [`Self::update_blocking`] call
    /// (the map used to stay empty forever, so this strategy never blocked).
    parent_map: HashMap<NodeId, NodeId>,

    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl Default for AncestorBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl AncestorBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            parent_map: HashMap::new(),
            cache: SignatureCache::default(),
        }
    }

    /// Derive the parent forest from the current role-successor structure.
    /// A successor's parent is the first (lowest-id) node listing it; the map
    /// is rebuilt from scratch so it stays correct across backtracking,
    /// merges and node-list truncation.
    fn rebuild_parent_map(&mut self, nodes: &[TableauNode]) {
        self.parent_map.clear();
        for node in nodes.iter() {
            for successors in node.role_successors.values() {
                for &successor in successors {
                    if successor < nodes.len() && successor != node.id {
                        self.parent_map.entry(successor).or_insert(node.id);
                    }
                }
            }
        }
    }
}

impl BlockingChecker for AncestorBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Ancestor
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        let ancestors = ancestor_set(node.id, &self.parent_map);
        if ancestors.is_empty() {
            return None;
        }
        first_blocker(&self.cache, node, nodes, |id| ancestors.contains(&id))
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        self.rebuild_parent_map(nodes);
        refresh_all(&mut self.cache, nodes);

        for i in 0..nodes.len() {
            let ancestors = ancestor_set(nodes[i].id, &self.parent_map);
            let blocker = if ancestors.is_empty() {
                None
            } else {
                first_blocker(&self.cache, &nodes[i], nodes, |id| ancestors.contains(&id))
            };
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

/// Pairwise blocking strategy
#[derive(Debug)]
pub struct PairwiseBlocking {
    /// Pairs of nodes that have been compared
    compared_pairs: HashSet<(NodeId, NodeId)>,

    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl Default for PairwiseBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl PairwiseBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            compared_pairs: HashSet::new(),
            cache: SignatureCache::default(),
        }
    }
}

impl BlockingChecker for PairwiseBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Pairwise
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        let compared = &self.compared_pairs;
        first_blocker(&self.cache, node, nodes, |id| {
            !compared.contains(&(node.id, id))
        })
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        refresh_all(&mut self.cache, nodes);

        for i in 0..nodes.len() {
            let blocker = first_blocker(&self.cache, &nodes[i], nodes, |id| {
                !self.compared_pairs.contains(&(nodes[i].id, id))
            });
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

/// Dynamic blocking strategy that adapts based on tableau characteristics
#[derive(Debug)]
pub struct DynamicBlocking {
    /// Current strategy being used
    current_strategy: Box<dyn BlockingChecker>,

    /// Statistics for adaptation
    stats: BlockingStatistics,
}

#[derive(Debug, Default)]
struct BlockingStatistics {
    blocked_nodes: usize,
    #[allow(dead_code)]
    total_comparisons: usize,
    false_positives: usize,
}

impl Default for DynamicBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_strategy: Box::new(AnywhereBlocking::new()),
            stats: BlockingStatistics::default(),
        }
    }

    /// Adapt the blocking strategy based on performance
    fn adapt_strategy(&mut self) {
        // Simple adaptation logic - in practice this would be more sophisticated
        if self.stats.false_positives > self.stats.blocked_nodes / 2 {
            // Too many false positives, switch to more precise strategy
            if !self.is_ancestor_blocking() {
                self.current_strategy = Box::new(AncestorBlocking::new());
            }
        }
    }

    fn is_ancestor_blocking(&self) -> bool {
        // Phase C: enum kind test instead of `format!("{:?}", …)` — formatting
        // a strategy renders its whole signature cache and dominated the
        // `update_blocking` pass in the consumer profile.
        self.current_strategy.kind() == StrategyKind::Ancestor
    }
}

impl BlockingChecker for DynamicBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Dynamic
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        self.current_strategy.is_blocked(node, nodes)
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        let result = self.current_strategy.update_blocking(nodes);

        // Update statistics
        self.stats.blocked_nodes = nodes.iter().filter(|n| n.blocking_info.is_blocked).count();

        // Adapt strategy if needed
        self.adapt_strategy();

        result
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        self.current_strategy.get_signature(node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.current_strategy.signature_recomputations()
    }
}

/// Check if one signature subsumes another (every concept of `sig1` appears in
/// `sig2`).
///
/// Thin wrapper over the cached [`NodeSignature`] representation; the
/// semantics are identical to the historical linear scan (`sig2.contains`).
/// Kept for the unit tests (and API parity with the pre-Phase-A helpers).
#[allow(dead_code)] // exercised by the module's unit tests
fn signatures_subsume(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> bool {
    let index = NodeSignature::build(sig2.iter());
    sig1.iter().all(|c| index.contains_label(c))
}

/// More sophisticated signature comparison (same length and same members).
///
/// Thin wrapper over [`NodeSignature`]; semantics identical to the historical
/// `len` check plus `sig2.contains` scan.
#[allow(dead_code)] // exercised by the module's unit tests
fn signatures_equal(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> bool {
    if sig1.len() != sig2.len() {
        return false;
    }
    let index = NodeSignature::build(sig2.iter());
    sig1.iter().all(|c| index.contains_label(c))
}

/// Compute signature similarity (for dynamic blocking)
#[allow(dead_code)]
fn signature_similarity(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> f64 {
    if sig1.is_empty() && sig2.is_empty() {
        return 1.0;
    }

    let intersection_size = sig1.iter().filter(|concept| sig2.contains(concept)).count();
    let union_size = sig1.len() + sig2.len() - intersection_size;

    if union_size == 0 {
        1.0
    } else {
        intersection_size as f64 / union_size as f64
    }
}

/// Single blocking strategy (HermiT-style) — blocks on *identical* signatures
#[derive(Debug)]
pub struct SingleBlocking {
    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl Default for SingleBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl SingleBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: SignatureCache::default(),
        }
    }
}

impl BlockingChecker for SingleBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Single
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        if !may_be_blocked(node) {
            return None;
        }
        let node_signature = signature_of(&self.cache, node);

        // Single blocking: check only against nodes with exactly the same signature
        for other in nodes {
            // Phase E: Root/Nominal nodes are never blockers.
            if other.id != node.id && other.id < node.id && may_be_blocker(other) {
                let other_signature = signature_of(&self.cache, other);
                if other_signature.equals(&node_signature) {
                    return Some(other.id);
                }
            }
        }

        None
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        refresh_all(&mut self.cache, nodes);

        // Update blocking information for all nodes
        for i in 0..nodes.len() {
            let blocker = if !may_be_blocked(&nodes[i]) {
                None
            } else {
                let node_signature = signature_of(&self.cache, &nodes[i]);
                let mut found = None;
                for other in nodes.iter() {
                    // Phase E: Root/Nominal nodes are never blockers.
                    if other.id != nodes[i].id && other.id < nodes[i].id && may_be_blocker(other) {
                        let other_signature = signature_of(&self.cache, other);
                        if other_signature.equals(&node_signature) {
                            found = Some(other.id);
                            break;
                        }
                    }
                }
                found
            };
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

/// Core blocking strategy (HermiT-style).
///
/// The "core" filter (atomic concepts only) is currently the identity on this
/// [`ConceptLabel`] representation, so core blocking coincides with subset
/// anywhere blocking — which is exactly what keeps it terminating.
#[derive(Debug)]
pub struct CoreBlocking {
    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl Default for CoreBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: SignatureCache::default(),
        }
    }
}

impl BlockingChecker for CoreBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Core
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        first_blocker(&self.cache, node, nodes, |id| id < node.id)
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        refresh_all(&mut self.cache, nodes);

        // Update blocking information for all nodes
        for i in 0..nodes.len() {
            let blocker = first_blocker(&self.cache, &nodes[i], nodes, |id| id < nodes[i].id);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

/// Optimal blocking strategy (HermiT-style) - combines multiple strategies
#[derive(Debug)]
pub struct OptimalBlocking {
    /// Current active strategy
    current_strategy: Box<dyn BlockingChecker>,
    /// Performance statistics
    statistics: OptimalBlockingStats,
}

#[derive(Debug, Default)]
struct OptimalBlockingStats {
    nodes_blocked: usize,
    blocking_checks: usize,
    #[allow(dead_code)]
    cache_hits: usize,
}

impl Default for OptimalBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimalBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // The fast path must stay *subset-based* (anywhere blocking): only
            // subset blocking guarantees that an unblocked chain of labels
            // strictly grows within the finite subexpression set.
            current_strategy: Box::new(AnywhereBlocking::new()),
            statistics: OptimalBlockingStats::default(),
        }
    }

    /// Adapt the blocking strategy based on performance
    fn adapt_strategy(&mut self) {
        // Simple adaptation: if too many checks with few blocks, switch to a
        // more precise strategy.
        //
        // Phase A: the previous switch target was `SingleBlocking`
        // (equality-based). Equality blocking is strictly *weaker* than subset
        // blocking and cannot guarantee termination on its own — an ever-growing
        // label chain never reaches equality — so adapting to it could turn a
        // terminating run into a non-terminating one. The switch now targets
        // `CoreBlocking`, which keeps the subset-based anywhere relation
        // (today's "core" filter is the identity), preserving the adaptation
        // behaviour without risking non-termination.
        if self.statistics.blocking_checks > 1000
            && self.statistics.nodes_blocked < self.statistics.blocking_checks / 10
            && !self.is_core_blocking()
        {
            self.current_strategy = Box::new(CoreBlocking::new());
        }
    }

    fn is_core_blocking(&self) -> bool {
        // Phase C: enum kind test instead of `format!("{:?}", …)` (see
        // `DynamicBlocking::is_ancestor_blocking`). The adaptation thresholds
        // above are unchanged.
        self.current_strategy.kind() == StrategyKind::Core
    }
}

impl BlockingChecker for OptimalBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Optimal
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        self.current_strategy.is_blocked(node, nodes)
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        let result = self.current_strategy.update_blocking(nodes);

        // Update statistics
        self.statistics.nodes_blocked = nodes.iter().filter(|n| n.blocking_info.is_blocked).count();
        self.statistics.blocking_checks += nodes.len();

        // Adapt strategy if needed
        self.adapt_strategy();

        result
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        self.current_strategy.get_signature(node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.current_strategy.signature_recomputations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tableau::{NodeStatus, NodeType, node::BlockingInfo};
    use std::collections::HashMap;

    fn create_test_node(id: NodeId, concepts: Vec<&str>) -> TableauNode {
        let concept_set: HashSet<ConceptLabel> = concepts
            .into_iter()
            .map(|c| ConceptLabel::Atomic(c.to_string()))
            .collect();
        let mut node = TableauNode {
            id,
            concepts: concept_set,
            node_type: NodeType::Individual,
            blocking_info: BlockingInfo::default(),
            concept_dependencies: HashMap::new(),
            role_successors: HashMap::new(),
            status: NodeStatus::default(),
            label_version: 0,
        };
        // Simulate the insertions that produced the label set.
        node.touch_labels();
        node
    }

    #[test]
    fn test_anywhere_blocking() {
        let blocking = AnywhereBlocking::new();

        let node1 = create_test_node(0, vec!["A", "B"]);
        let node2 = create_test_node(1, vec!["A", "B", "C"]);
        let node3 = create_test_node(2, vec!["A", "B"]);

        let nodes = vec![node1, node2, node3];

        // Node 3 should be blocked by node 1
        let blocked = blocking.is_blocked(&nodes[2], &nodes);
        assert_eq!(blocked, Some(0));
    }

    #[test]
    fn test_signature_subsumption() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];

        let sig2 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
            ConceptLabel::Atomic("C".to_string()),
        ];

        assert!(signatures_subsume(&sig1, &sig2));
        assert!(!signatures_subsume(&sig2, &sig1));
    }

    #[test]
    fn test_signature_equality() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];

        let sig2 = vec![
            ConceptLabel::Atomic("B".to_string()),
            ConceptLabel::Atomic("A".to_string()),
        ];

        assert!(signatures_equal(&sig1, &sig2));
    }

    #[test]
    fn test_signature_similarity() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];

        let sig2 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("C".to_string()),
        ];

        let similarity = signature_similarity(&sig1, &sig2);
        assert!((similarity - 0.333).abs() < 0.01); // 1/3 similarity
    }

    // ── Phase A: signature caching ────────────────────────────────────────────

    /// Phase C: every strategy the factory can build must report the same kind
    /// the historical `Debug`-substring test derived (the adaptive wrappers
    /// used that test to pick their inner strategy).
    #[test]
    fn strategy_kind_matches_debug_substring_identity() {
        use crate::config::BlockingStrategy as Cfg;
        let cases = [
            (Cfg::Anywhere, StrategyKind::Anywhere),
            (Cfg::Ancestor, StrategyKind::Ancestor),
            (Cfg::Pairwise, StrategyKind::Pairwise),
            (Cfg::Dynamic, StrategyKind::Dynamic),
            (Cfg::Single, StrategyKind::Single),
            (Cfg::Core, StrategyKind::Core),
            (Cfg::Optimal, StrategyKind::Optimal),
            (Cfg::Indexed, StrategyKind::Indexed),
        ];
        for (cfg, expected) in cases {
            let checker = BlockingStrategy::create_checker(cfg).unwrap();
            assert_eq!(checker.kind(), expected);
            // The old test was `format!("{:?}", checker).contains(name)`; the
            // `Debug` of each concrete checker still starts with its type name.
            let debug = format!("{checker:?}");
            assert!(
                debug.contains(&format!("{expected:?}")),
                "{debug} does not contain {expected:?}"
            );
        }
    }

    /// Phase C: the adaptive wrappers must identify their inner strategy via
    /// `kind()` with exactly the same decisions the `Debug`-substring test made
    /// (`Anywhere` → adapt once, then stable at the switch target).
    #[test]
    fn adaptive_wrappers_identify_inner_strategy_without_formatting() {
        let mut dyn_strategy = DynamicBlocking::new();
        assert!(!dyn_strategy.is_ancestor_blocking());
        dyn_strategy.current_strategy = Box::new(AncestorBlocking::new());
        assert!(dyn_strategy.is_ancestor_blocking());
        dyn_strategy.current_strategy = Box::new(CoreBlocking::new());
        assert!(!dyn_strategy.is_ancestor_blocking());

        let mut optimal = OptimalBlocking::new();
        assert!(!optimal.is_core_blocking());
        optimal.current_strategy = Box::new(CoreBlocking::new());
        assert!(optimal.is_core_blocking());
        optimal.current_strategy = Box::new(AnywhereBlocking::new());
        assert!(!optimal.is_core_blocking());
    }

    /// The hash-sorted representation must agree with a brute-force reference
    /// implementation (the historical linear `contains` scan) on subset and
    /// equality decisions, including hash-collision safety (label equality is
    /// consulted whenever hashes match).
    #[test]
    fn node_signature_matches_brute_force() {
        let a = create_test_node(0, vec!["A", "B", "C"]);
        let b = create_test_node(1, vec!["A", "B"]);
        let c = create_test_node(2, vec!["A", "B", "D"]);

        let sa = NodeSignature::of_node(&a);
        let sb = NodeSignature::of_node(&b);
        let sc = NodeSignature::of_node(&c);

        // {A,B} ⊆ {A,B,C}
        assert!(sb.subsumes(&sa));
        assert!(!sa.subsumes(&sb));
        // {A,B,C} ⊄ {A,B,D} (C missing) and vice versa
        assert!(!sa.subsumes(&sc));
        assert!(!sc.subsumes(&sa));
        // equality
        assert!(sa.equals(&NodeSignature::of_node(&a)));
        assert!(!sa.equals(&sb));
        // wrapper helpers agree with the reference semantics
        let va: Vec<ConceptLabel> = a.concepts.iter().cloned().collect();
        let vb: Vec<ConceptLabel> = b.concepts.iter().cloned().collect();
        assert!(signatures_subsume(&vb, &va));
        assert!(!signatures_subsume(&va, &vb));
        assert!(signatures_equal(&va, &va.clone()));
        assert!(!signatures_equal(&va, &vb));
    }

    /// Repeated `update_blocking` passes over an unchanged node set must not
    /// recompute any signature, and a label mutation must recompute exactly
    /// the mutated node's signature.
    #[test]
    fn signature_cache_recomputes_only_on_label_change() {
        let mut nodes = vec![
            create_test_node(0, vec!["A", "B"]),
            create_test_node(1, vec!["A", "B", "C"]),
            create_test_node(2, vec!["A", "B"]),
        ];

        let mut blocking = AnywhereBlocking::new();
        blocking.update_blocking(&mut nodes).unwrap();
        let after_first = blocking.signature_recomputations();
        assert_eq!(after_first, 3, "one recomputation per node on first pass");

        // Unchanged labels → no recomputation, same blocking decisions.
        let decisions_before: Vec<_> = nodes
            .iter()
            .map(|n| (n.blocking_info.is_blocked, n.blocking_info.blocker))
            .collect();
        blocking.update_blocking(&mut nodes).unwrap();
        assert_eq!(
            blocking.signature_recomputations(),
            after_first,
            "unchanged labels must not trigger recomputation"
        );
        let decisions_after: Vec<_> = nodes
            .iter()
            .map(|n| (n.blocking_info.is_blocked, n.blocking_info.blocker))
            .collect();
        assert_eq!(decisions_before, decisions_after);

        // Mutate node 2's labels → exactly one more recomputation.
        nodes[2].add_concept(ConceptLabel::Atomic("D".to_string()));
        assert_eq!(nodes[2].label_version, 2);
        blocking.update_blocking(&mut nodes).unwrap();
        assert_eq!(
            blocking.signature_recomputations(),
            after_first + 1,
            "only the mutated node's signature may be recomputed"
        );
        // {A,B,D} no longer equal to node 0 {A,B} but node 0 ⊆ node 2 still.
        assert_eq!(nodes[2].blocking_info.blocker, Some(0));
    }

    /// Phase E: nominal and root nodes are never *marked* blocked **and never
    /// serve as blockers** — the O(n²) candidate scan skips them outright. A
    /// generated node whose only subset candidate is a nominal therefore stays
    /// unblocked, while a node carrying a `{a}` label is never blocked either.
    #[test]
    fn nominal_and_root_nodes_are_never_used_as_blockers() {
        let mut nominal = create_test_node(0, vec!["A"]);
        nominal.node_type = NodeType::Nominal;
        let successor = create_test_node(1, vec!["A", "B"]);

        let mut nodes = vec![nominal, successor];
        let mut blocking = AnywhereBlocking::new();
        blocking.update_blocking(&mut nodes).unwrap();

        // {A} ⊆ {A,B}, but the nominal node is not a legitimate witness: the
        // successor stays unblocked (Phase E blocker candidacy filter).
        assert_eq!(nodes[1].blocking_info.blocker, None);
        assert!(!nodes[1].blocking_info.is_blocked);
        assert!(!nodes[0].blocking_info.is_blocked);
        assert_eq!(nodes[0].blocking_info.blocker, None);

        // Direct query: a nominal node is never reported blocked.
        assert_eq!(blocking.is_blocked(&nodes[0], &nodes), None);

        // A node carrying a `{a}` nominal *label* is never blocked either —
        // even when an earlier node's label is a subset of its own.
        let mut labelled = create_test_node(1, vec!["A", "B"]);
        labelled.add_concept(ConceptLabel::Nominal("http://ex.org/a".to_string()));
        let labelled_nominal = create_test_node(0, vec!["A"]);
        let pair = vec![labelled_nominal, labelled];
        assert_eq!(blocking.is_blocked(&pair[1], &pair), None);

        // Root nodes are equally excluded on the blocker side: a root whose
        // label is a subset must not block a later node.
        let mut root = create_test_node(0, vec!["A"]);
        root.node_type = NodeType::Root;
        let later = create_test_node(1, vec!["A", "B"]);
        let pair = vec![root, later];
        assert_eq!(blocking.is_blocked(&pair[1], &pair), None);

        // …while an eligible earlier *generated* node still blocks it.
        let earlier = create_test_node(0, vec!["A"]);
        let later = create_test_node(1, vec!["A", "B"]);
        let pair = vec![earlier, later];
        assert_eq!(blocking.is_blocked(&pair[1], &pair), Some(0));
    }
}

/// Hash-based blocker candidate index for O(1) blocker lookup
#[derive(Debug)]
pub struct BlockerCandidateIndex {
    /// Maps signature hashes to node IDs
    signature_index: HashMap<u64, Vec<NodeId>>,

    /// Maps node IDs to their signature hashes
    node_signatures: HashMap<NodeId, u64>,

    /// Cached signatures for quick retrieval
    signature_cache: HashMap<NodeId, Vec<ConceptLabel>>,
}

impl BlockerCandidateIndex {
    /// Create a new blocker candidate index
    #[must_use]
    pub fn new() -> Self {
        Self {
            signature_index: HashMap::new(),
            node_signatures: HashMap::new(),
            signature_cache: HashMap::new(),
        }
    }

    /// Compute a hash for a signature (Phase E: [`FastHasher`](crate::core::fast_hash)
    /// instead of SipHash — this runs for every indexed node per pass).
    fn compute_signature_hash(signature: &[ConceptLabel]) -> u64 {
        let mut hasher = crate::core::fast_hash::FastHasher::new();

        // Sort signature for deterministic hashing — use hash keys to avoid allocations.
        let mut sorted_sig: Vec<_> = signature.iter().collect();
        sorted_sig.sort_unstable_by_key(|c| concept_sort_key(c));

        for concept in sorted_sig {
            concept.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Index a node's signature
    pub fn index_node(&mut self, node_id: NodeId, signature: Vec<ConceptLabel>) {
        let hash = Self::compute_signature_hash(&signature);

        // Add to signature index
        self.signature_index.entry(hash).or_default().push(node_id);

        // Store node's signature hash
        self.node_signatures.insert(node_id, hash);

        // Cache the signature
        self.signature_cache.insert(node_id, signature);
    }

    /// Find potential blocker candidates for a node signature
    #[must_use]
    pub fn find_blocker_candidates(&self, signature: &[ConceptLabel]) -> Vec<NodeId> {
        let hash = Self::compute_signature_hash(signature);

        // Exact match candidates
        let mut candidates = self.signature_index.get(&hash).cloned().unwrap_or_default();

        // For robustness, also check signatures with small variations
        // (in case of hash collisions or near-matches)
        for (&other_hash, node_ids) in &self.signature_index {
            if other_hash != hash {
                // Simple similarity check: XOR the hashes and check if they're close
                let diff = (hash ^ other_hash).count_ones();
                if diff <= 8 {
                    // Allow up to 8 bit differences
                    candidates.extend(node_ids.iter().copied());
                }
            }
        }

        candidates
    }

    /// Get the cached signature for a node
    #[must_use]
    pub fn get_signature(&self, node_id: NodeId) -> Option<&Vec<ConceptLabel>> {
        self.signature_cache.get(&node_id)
    }

    /// Remove a node from the index
    pub fn remove_node(&mut self, node_id: NodeId) {
        if let Some(hash) = self.node_signatures.remove(&node_id)
            && let Some(node_ids) = self.signature_index.get_mut(&hash)
        {
            node_ids.retain(|&id| id != node_id);
            if node_ids.is_empty() {
                self.signature_index.remove(&hash);
            }
        }
        self.signature_cache.remove(&node_id);
    }

    /// Clear the entire index
    pub fn clear(&mut self) {
        self.signature_index.clear();
        self.node_signatures.clear();
        self.signature_cache.clear();
    }

    /// Get the number of indexed nodes
    #[must_use]
    pub fn size(&self) -> usize {
        self.node_signatures.len()
    }

    /// Check if a node is indexed
    #[must_use]
    pub fn contains(&self, node_id: NodeId) -> bool {
        self.node_signatures.contains_key(&node_id)
    }
}

impl Default for BlockerCandidateIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized anywhere blocking using hash-based index.
///
/// Phase A: blocking decisions use the exact subset scan over cached
/// signatures (identical relation to [`AnywhereBlocking`]); the candidate
/// index is rebuilt each pass for the public [`BlockerCandidateIndex`]
/// API — the old hash-*proximity* candidate heuristic could miss subset
/// blockers whose hashes are unrelated, which is exactly the kind of
/// under-blocking that risks non-termination.
#[derive(Debug)]
pub struct IndexedAnywhereBlocking {
    /// Blocker candidate index
    index: BlockerCandidateIndex,

    /// Cache of node signatures keyed by `label_version`
    cache: SignatureCache,
}

impl IndexedAnywhereBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self {
            index: BlockerCandidateIndex::new(),
            cache: SignatureCache::default(),
        }
    }
}

impl Default for IndexedAnywhereBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockingChecker for IndexedAnywhereBlocking {
    fn kind(&self) -> StrategyKind {
        StrategyKind::Indexed
    }

    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        first_blocker(&self.cache, node, nodes, |id| id < node.id)
    }

    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        refresh_all(&mut self.cache, nodes);

        // Rebuild index (kept for the `BlockerCandidateIndex` API/tests).
        self.index.clear();
        for node in nodes.iter() {
            let signature = signature_vec(&self.cache, node);
            self.index.index_node(node.id, signature);
        }

        // Update blocking information
        for i in 0..nodes.len() {
            let blocker = first_blocker(&self.cache, &nodes[i], nodes, |id| id < nodes[i].id);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }

        Ok(())
    }

    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        signature_vec(&self.cache, node)
    }

    fn signature_recomputations(&self) -> u64 {
        self.cache.recomputations
    }
}

#[cfg(test)]
mod blocker_index_tests {
    use super::*;

    #[test]
    fn test_blocker_index_basic() {
        let mut index = BlockerCandidateIndex::new();

        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];

        index.index_node(1, sig1.clone());

        assert_eq!(index.size(), 1);
        assert!(index.contains(1));

        let candidates = index.find_blocker_candidates(&sig1);
        assert!(candidates.contains(&1));
    }

    #[test]
    fn test_blocker_index_removal() {
        let mut index = BlockerCandidateIndex::new();

        let sig1 = vec![ConceptLabel::Atomic("A".to_string())];
        index.index_node(1, sig1);

        assert_eq!(index.size(), 1);

        index.remove_node(1);

        assert_eq!(index.size(), 0);
        assert!(!index.contains(1));
    }

    #[test]
    fn test_indexed_anywhere_blocking() {
        let mut blocking = IndexedAnywhereBlocking::new();

        // Test with empty nodes
        let nodes = vec![];
        assert!(blocking.update_blocking(&mut nodes.clone()).is_ok());
    }
}
