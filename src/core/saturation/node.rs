//! Saturation node data structures

use crate::ontology::{ClassExpression, IRI};
use crate::core::persistent_collections::ConceptSet;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Status of a saturation node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SaturationStatus {
    /// Node is completely saturated (all deterministic consequences computed)
    Complete,

    /// Node is partially saturated (some consequences computed)
    Partial,

    /// Node contains non-deterministic constructs that cannot be fully saturated
    NonDeterministic,

    /// Node requires full tableau expansion due to complexity
    RequiresFullTableau,

    /// Node saturation is in progress
    InProgress,

    /// Node has not been processed yet
    Unprocessed,
}

/// A node in the saturation graph representing a concept with its saturated consequences
#[derive(Debug, Clone)]
pub struct SaturationNode {
    /// The primary concept this node represents
    pub concept: ClassExpression,

    /// All concepts that are saturated into this node (deterministic consequences)
    pub saturated_concepts: ConceptSet,

    /// Direct subsumers discovered through saturation
    pub direct_subsumers: ConceptSet,

    /// All subsumers (transitive closure)
    pub all_subsumers: ConceptSet,

    /// Existential restrictions that must hold
    pub existentials: Vec<ExistentialRestriction>,

    /// Universal restrictions that must hold
    pub universals: Vec<UniversalRestriction>,

    /// Current status of this saturation node
    pub status: SaturationStatus,

    /// Number of non-deterministic branches encountered
    pub branch_count: usize,

    /// Number of saturation iterations performed
    pub iteration_count: usize,

    /// Hash signature for quick comparison
    pub signature: u64,

    /// Whether this node represents an inconsistent concept
    pub is_inconsistent: bool,

    /// Cached subsumption relationships
    subsumption_cache: HashMap<ClassExpression, bool>,
}

/// Represents an existential restriction ∃R.C
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExistentialRestriction {
    pub property: IRI,
    pub filler: ClassExpression,
}

/// Represents a universal restriction ∀R.C
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniversalRestriction {
    pub property: IRI,
    pub filler: ClassExpression,
}

impl SaturationNode {
    /// Create a new saturation node for a concept
    pub fn new(concept: ClassExpression) -> Self {
        let mut saturated_concepts = ConceptSet::new();
        saturated_concepts.insert(concept.clone());

        let signature = Self::compute_signature(&saturated_concepts);

        Self {
            concept,
            saturated_concepts,
            direct_subsumers: ConceptSet::new(),
            all_subsumers: ConceptSet::new(),
            existentials: Vec::new(),
            universals: Vec::new(),
            status: SaturationStatus::Unprocessed,
            branch_count: 0,
            iteration_count: 0,
            signature,
            is_inconsistent: false,
            subsumption_cache: HashMap::new(),
        }
    }

    /// Add a saturated concept to this node
    pub fn add_saturated_concept(&mut self, concept: ClassExpression) -> bool {
        if !self.saturated_concepts.contains(&concept) {
            self.saturated_concepts = self.saturated_concepts.update(concept);
            self.signature = Self::compute_signature(&self.saturated_concepts);
            true
        } else {
            false
        }
    }

    /// Add multiple saturated concepts
    pub fn add_saturated_concepts(&mut self, concepts: impl IntoIterator<Item = ClassExpression>) {
        let mut changed = false;
        for concept in concepts {
            if !self.saturated_concepts.contains(&concept) {
                self.saturated_concepts = self.saturated_concepts.update(concept);
                changed = true;
            }
        }
        if changed {
            self.signature = Self::compute_signature(&self.saturated_concepts);
        }
    }

    /// Add a direct subsumer
    pub fn add_direct_subsumer(&mut self, subsumer: ClassExpression) {
        self.direct_subsumers.insert(subsumer);
    }

    /// Add an existential restriction
    pub fn add_existential(&mut self, property: IRI, filler: ClassExpression) {
        let restriction = ExistentialRestriction { property, filler };
        if !self.existentials.contains(&restriction) {
            self.existentials.push(restriction);
        }
    }

    /// Add a universal restriction
    pub fn add_universal(&mut self, property: IRI, filler: ClassExpression) {
        let restriction = UniversalRestriction { property, filler };
        if !self.universals.contains(&restriction) {
            self.universals.push(restriction);
        }
    }

    /// Mark this node as having encountered a non-deterministic branch
    pub fn increment_branch_count(&mut self) {
        self.branch_count += 1;
    }

    /// Check if this node should be marked as RequiresFullTableau
    pub fn should_require_tableau(&self, max_branches: usize) -> bool {
        self.branch_count > max_branches
    }

    /// Update the status based on branch count and configuration
    pub fn update_status(&mut self, max_branches: usize) {
        if self.is_inconsistent {
            self.status = SaturationStatus::Complete;
        } else if self.should_require_tableau(max_branches) {
            self.status = SaturationStatus::RequiresFullTableau;
        } else if self.branch_count > 0 {
            self.status = SaturationStatus::NonDeterministic;
        }
    }

    /// Mark this node as inconsistent (bottom)
    pub fn mark_inconsistent(&mut self) {
        self.is_inconsistent = true;
        self.status = SaturationStatus::Complete;
    }

    /// Check if a concept is subsumed by this node's saturated concepts
    pub fn subsumes(&self, concept: &ClassExpression) -> bool {
        self.saturated_concepts.contains(concept)
    }

    /// Get a cached subsumption result
    pub fn get_cached_subsumption(&self, concept: &ClassExpression) -> Option<bool> {
        self.subsumption_cache.get(concept).copied()
    }

    /// Cache a subsumption result
    pub fn cache_subsumption(&mut self, concept: ClassExpression, result: bool) {
        self.subsumption_cache.insert(concept, result);
    }

    /// Compute a hash signature for a set of concepts
    fn compute_signature(concepts: &ConceptSet) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        
        // Sort concepts for deterministic hashing
        let mut sorted_concepts: Vec<_> = concepts.iter().collect();
        sorted_concepts.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        
        for concept in sorted_concepts {
            format!("{:?}", concept).hash(&mut hasher);
        }
        
        hasher.finish()
    }

    /// Get the signature hash
    pub fn get_signature(&self) -> u64 {
        self.signature
    }

    /// Check if this node is complete
    pub fn is_complete(&self) -> bool {
        self.status == SaturationStatus::Complete
    }

    /// Check if this node requires full tableau expansion
    pub fn requires_full_tableau(&self) -> bool {
        self.status == SaturationStatus::RequiresFullTableau
    }
}

impl PartialEq for SaturationNode {
    fn eq(&self, other: &Self) -> bool {
        self.signature == other.signature
    }
}

impl Eq for SaturationNode {}

impl Hash for SaturationNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.signature.hash(state);
    }
}
