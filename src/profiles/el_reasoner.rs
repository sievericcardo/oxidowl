//! OWL 2 EL Profile Reasoner
//!
//! This module implements polynomial-time reasoning for the OWL 2 EL profile
//! using completion rules and optimized data structures.
//! 
//! Features concurrent classification for improved performance on multi-core systems.

use crate::{
    Error, Result,
    config::ReasonerConfig,
    ontology::{Axiom, ClassExpression, Individual, ObjectPropertyExpression, Ontology},
    core::reasoner::ClassificationResult,
    explanation::{ExplanationService, Explanation},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, RwLock, Mutex},
    time::Instant,
};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// EL profile reasoner with polynomial-time algorithms
#[derive(Debug)]
pub struct ELReasoner {
    /// Normalized axioms
    normalized_axioms: Vec<ELAxiom>,
    /// Concept subsumption hierarchy
    concept_hierarchy: ConceptHierarchy,
    /// Role hierarchy
    role_hierarchy: RoleHierarchy,
    /// Completion rules engine
    completion_engine: CompletionEngine,
    /// Configuration
    config: ReasonerConfig,
    /// Explanation service
    explanation_service: Option<Arc<ExplanationService>>,
    /// Statistics
    statistics: ELStatistics,
}

impl ELReasoner {
    /// Create a new EL reasoner
    pub fn new(config: ReasonerConfig) -> Self {
        let explanation_service = if config.reasoning.enable_explanations {
            Some(Arc::new(ExplanationService::new()))
        } else {
            None
        };

        Self {
            normalized_axioms: Vec::new(),
            concept_hierarchy: ConceptHierarchy::new(),
            role_hierarchy: RoleHierarchy::new(),
            completion_engine: CompletionEngine::new(),
            config,
            explanation_service,
            statistics: ELStatistics::default(),
        }
    }

    /// Initialize with an ontology
    pub fn initialize(&mut self, ontology: &Ontology) -> Result<()> {
        let start = Instant::now();
        
        // Step 1: Normalize ontology to EL normal form
        self.normalize_ontology(ontology)?;
        
        // Step 2: Extract role hierarchy
        self.extract_role_hierarchy()?;
        
        // Step 3: Initialize completion engine
        self.completion_engine.initialize(&self.normalized_axioms, &self.role_hierarchy)?;
        
        self.statistics.initialization_time = start.elapsed();
        Ok(())
    }

    /// Perform EL classification with optional concurrent processing
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        let start = Instant::now();
        
        // Apply completion rules until fixpoint
        self.completion_engine.run_to_completion()?;
        
        // Build concept hierarchy from completion results
        // Use concurrent classification if enabled and rayon is available
        #[cfg(feature = "rayon")]
        if self.config.performance.enable_parallel_expansion {
            self.build_concept_hierarchy_concurrent()?;
        } else {
            self.build_concept_hierarchy()?;
        }
        
        #[cfg(not(feature = "rayon"))]
        self.build_concept_hierarchy()?;
        
        let classification_time = start.elapsed();
        self.statistics.classification_time = classification_time;
        
        // Convert hierarchy to ClassificationResult format
        let hierarchy_map = self.convert_to_class_expression_hierarchy();
        
        Ok(ClassificationResult::new(hierarchy_map))
    }
    
    /// Build concept hierarchy using concurrent processing
    #[cfg(feature = "rayon")]
    fn build_concept_hierarchy_concurrent(&mut self) -> Result<()> {
        let subsumptions = self.completion_engine.get_all_subsumptions();
        
        // Group subsumptions by concept to enable parallel processing
        let subsumption_map = Arc::new(Mutex::new(HashMap::new()));
        
        // Process subsumptions in parallel
        subsumptions.par_iter().for_each(|(sub, sup)| {
            let mut map = subsumption_map.lock().unwrap();
            map.entry(sub.clone())
                .or_insert_with(HashSet::new)
                .insert(sup.clone());
        });
        
        // Build hierarchy from the collected subsumptions
        let final_map = Arc::try_unwrap(subsumption_map)
            .map(|mutex| mutex.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone());
        self.concept_hierarchy = ConceptHierarchy::from_subsumption_map(final_map);
        
        Ok(())
    }
    
    /// Convert EL concept hierarchy to ClassExpression hierarchy
    fn convert_to_class_expression_hierarchy(&self) -> HashMap<ClassExpression, HashSet<ClassExpression>> {
        let mut result = HashMap::new();
        
        for (sub, sups) in self.concept_hierarchy.get_all_subsumptions() {
            let sub_expr = sub.to_class_expression();
            let sup_exprs: HashSet<_> = sups.iter()
                .map(|sup| sup.to_class_expression())
                .collect();
            result.insert(sub_expr, sup_exprs);
        }
        
        result
    }

    /// Check if a subsumption holds
    pub fn is_subsumed(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        // Convert to EL concepts and check in hierarchy
        let sub_concept = self.to_el_concept(subclass)?;
        let super_concept = self.to_el_concept(superclass)?;
        
        Ok(self.concept_hierarchy.is_subsumed(&sub_concept, &super_concept))
    }

    /// Get explanation for a subsumption
    pub fn explain_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<Option<Explanation>> {
        if let Some(ref explanation_service) = self.explanation_service {
            let explanation = explanation_service.explain_subsumption(
                subclass,
                superclass,
                &self.original_axioms_as_general_axioms(),
            )?;
            Ok(Some(explanation))
        } else {
            Ok(None)
        }
    }

    /// Check if the ontology is consistent (always true for EL)
    pub fn is_consistent(&self) -> bool {
        // EL ontologies are always consistent
        true
    }

    /// Check if a class is satisfiable
    pub fn is_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        // In EL, check if class subsumes bottom
        let bottom = ClassExpression::ObjectIntersectionOf(vec![]);
        let is_bottom_subsumed = self.is_subsumed(&bottom, class)?;
        Ok(!is_bottom_subsumed)
    }

    /// Get reasoning statistics
    pub fn get_reasoning_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("initialization_time".to_string(), serde_json::json!(self.statistics.initialization_time.as_millis()));
        stats.insert("classification_time".to_string(), serde_json::json!(self.statistics.classification_time.as_millis()));
        stats.insert("completion_steps".to_string(), serde_json::json!(self.statistics.completion_steps));
        stats.insert("memory_usage".to_string(), serde_json::json!(self.statistics.memory_usage));
        stats
    }

    // Private methods

    fn normalize_ontology(&mut self, ontology: &Ontology) -> Result<()> {
        let normalizer = ELNormalizer::new();
        self.normalized_axioms = normalizer.normalize_axioms(&ontology.axioms)?;
        Ok(())
    }

    fn extract_role_hierarchy(&mut self) -> Result<()> {
        for axiom in &self.normalized_axioms {
            match axiom {
                ELAxiom::RoleInclusion { sub_role, super_role } => {
                    self.role_hierarchy.add_inclusion(sub_role.clone(), super_role.clone());
                }
                _ => {}
            }
        }
        self.role_hierarchy.compute_transitive_closure();
        Ok(())
    }

    fn build_concept_hierarchy(&mut self) -> Result<()> {
        let subsumptions = self.completion_engine.get_all_subsumptions();
        self.concept_hierarchy = ConceptHierarchy::from_subsumptions(subsumptions);
        Ok(())
    }

    fn to_el_concept(&self, class_expr: &ClassExpression) -> Result<ELConcept> {
        match class_expr {
            ClassExpression::Class(class) => Ok(ELConcept::Atomic(class.clone())),
            ClassExpression::ObjectIntersectionOf(classes) => {
                let el_concepts: Result<Vec<_>> = classes.iter()
                    .map(|c| self.to_el_concept(c))
                    .collect();
                Ok(ELConcept::Conjunction(el_concepts?))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let el_concept = self.to_el_concept(filler)?;
                Ok(ELConcept::Existential {
                    role: property.clone(),
                    filler: Box::new(el_concept),
                })
            }
            _ => Err(Error::unsupported(format!("Class expression not supported in EL: {:?}", class_expr))),
        }
    }

    fn original_axioms_as_general_axioms(&self) -> Vec<Axiom> {
        // Convert EL axioms back to general axioms for explanation
        self.normalized_axioms
            .iter()
            .map(|el_axiom| el_axiom.to_general_axiom())
            .collect()
    }
}

/// EL-specific axiom representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ELAxiom {
    /// Concept inclusion: A ⊑ B
    ConceptInclusion {
        lhs: ELConcept,
        rhs: ELConcept,
    },
    /// Role inclusion: r ⊑ s
    RoleInclusion {
        sub_role: ObjectPropertyExpression,
        super_role: ObjectPropertyExpression,
    },
    /// Concept assertion: A(a)
    ConceptAssertion {
        concept: ELConcept,
        individual: Individual,
    },
    /// Role assertion: r(a, b)
    RoleAssertion {
        role: ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    },
}

impl ELAxiom {
    /// Convert back to general axiom for explanations
    pub fn to_general_axiom(&self) -> Axiom {
        use crate::ontology::axioms::*;
        match self {
            ELAxiom::ConceptInclusion { lhs, rhs } => {
                Axiom::SubClassOf(SubClassOfAxiom {
                    id: 0,
                    subclass: lhs.to_class_expression(),
                    superclass: rhs.to_class_expression(),
                    annotations: Vec::new(),
                })
            }
            ELAxiom::RoleInclusion { sub_role, super_role } => {
                Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                    id: 0,
                    sub_property: sub_role.clone(),
                    super_property: super_role.clone(),
                    annotations: Vec::new(),
                })
            }
            ELAxiom::ConceptAssertion { concept, individual } => {
                Axiom::ClassAssertion(ClassAssertionAxiom {
                    id: 0,
                    individual: individual.clone(),
                    class: concept.to_class_expression(),
                    annotations: Vec::new(),
                })
            }
            ELAxiom::RoleAssertion { role, source, target } => {
                Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                    id: 0,
                    source: source.clone(),
                    target: target.clone(),
                    property: role.clone(),
                    annotations: Vec::new(),
                })
            }
        }
    }
}

/// EL concept representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ELConcept {
    /// Atomic concept
    Atomic(crate::ontology::Class),
    /// Top concept
    Top,
    /// Conjunction of concepts
    Conjunction(Vec<ELConcept>),
    /// Existential restriction
    Existential {
        role: ObjectPropertyExpression,
        filler: Box<ELConcept>,
    },
}

impl ELConcept {
    /// Convert to general class expression
    pub fn to_class_expression(&self) -> ClassExpression {
        match self {
            ELConcept::Atomic(class) => ClassExpression::Class(class.clone()),
            ELConcept::Top => ClassExpression::Class(crate::ontology::Class::new(crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing"))),
            ELConcept::Conjunction(concepts) => {
                let class_exprs: Vec<_> = concepts.iter()
                    .map(|c| c.to_class_expression())
                    .collect();
                ClassExpression::ObjectIntersectionOf(class_exprs)
            }
            ELConcept::Existential { role, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: role.clone(),
                    filler: Box::new(filler.to_class_expression()),
                }
            }
        }
    }

    /// Check if this concept is atomic
    pub fn is_atomic(&self) -> bool {
        matches!(self, ELConcept::Atomic(_))
    }

    /// Get all atomic concepts in this concept
    pub fn get_atomic_concepts(&self) -> HashSet<crate::ontology::Class> {
        let mut atoms = HashSet::new();
        self.collect_atomic_concepts(&mut atoms);
        atoms
    }

    fn collect_atomic_concepts(&self, atoms: &mut HashSet<crate::ontology::Class>) {
        match self {
            ELConcept::Atomic(class) => {
                atoms.insert(class.clone());
            }
            ELConcept::Conjunction(concepts) => {
                for concept in concepts {
                    concept.collect_atomic_concepts(atoms);
                }
            }
            ELConcept::Existential { filler, .. } => {
                filler.collect_atomic_concepts(atoms);
            }
            ELConcept::Top => {}
        }
    }
}

/// EL normalizer for converting general axioms to EL normal form
#[derive(Debug)]
pub struct ELNormalizer;

impl ELNormalizer {
    /// Create a new normalizer
    pub fn new() -> Self {
        Self
    }

    /// Normalize axioms to EL normal form
    pub fn normalize_axioms(&self, axioms: &[Axiom]) -> Result<Vec<ELAxiom>> {
        use crate::ontology::axioms::*;
        let mut el_axioms = Vec::with_capacity(axioms.len());
        
        for axiom in axioms {
            match axiom {
                Axiom::SubClassOf(SubClassOfAxiom { subclass, superclass, .. }) => {
                    let el_sub = self.normalize_class_expression(subclass)?;
                    let el_sup = self.normalize_class_expression(superclass)?;
                    el_axioms.push(ELAxiom::ConceptInclusion { lhs: el_sub, rhs: el_sup });
                }
                Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom { sub_property, super_property, .. }) => {
                    el_axioms.push(ELAxiom::RoleInclusion { 
                        sub_role: sub_property.clone(), 
                        super_role: super_property.clone() 
                    });
                }
                Axiom::ClassAssertion(ClassAssertionAxiom { individual, class, .. }) => {
                    let el_concept = self.normalize_class_expression(class)?;
                    el_axioms.push(ELAxiom::ConceptAssertion { 
                        concept: el_concept, 
                        individual: individual.clone() 
                    });
                }
                Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom { source, target, property, .. }) => {
                    el_axioms.push(ELAxiom::RoleAssertion {
                        role: property.clone(),
                        source: source.clone(),
                        target: target.clone(),
                    });
                }
                _ => {
                    // Skip non-EL axioms
                    continue;
                }
            }
        }
        
        Ok(el_axioms)
    }

    fn normalize_class_expression(&self, class_expr: &ClassExpression) -> Result<ELConcept> {
        match class_expr {
            ClassExpression::Class(class) => Ok(ELConcept::Atomic(class.clone())),
            ClassExpression::ObjectIntersectionOf(classes) => {
                let el_concepts: Result<Vec<_>> = classes.iter()
                    .map(|c| self.normalize_class_expression(c))
                    .collect();
                Ok(ELConcept::Conjunction(el_concepts?))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let el_filler = self.normalize_class_expression(filler)?;
                Ok(ELConcept::Existential {
                    role: property.clone(),
                    filler: Box::new(el_filler),
                })
            }
            _ => Err(Error::unsupported(format!("Not supported in EL: {:?}", class_expr))),
        }
    }
}

/// Concept hierarchy for EL reasoning
#[derive(Debug)]
pub struct ConceptHierarchy {
    /// Direct subsumption relationships
    subsumptions: HashMap<ELConcept, HashSet<ELConcept>>,
    /// Transitive closure of subsumptions
    transitive_subsumptions: HashMap<ELConcept, HashSet<ELConcept>>,
}

impl ConceptHierarchy {
    /// Create a new concept hierarchy
    pub fn new() -> Self {
        Self {
            subsumptions: HashMap::new(),
            transitive_subsumptions: HashMap::new(),
        }
    }

    /// Build hierarchy from subsumption pairs
    pub fn from_subsumptions(subsumptions: Vec<(ELConcept, ELConcept)>) -> Self {
        let mut hierarchy = Self::new();
        
        for (sub, sup) in subsumptions {
            hierarchy.add_subsumption(sub, sup);
        }
        
        hierarchy.compute_transitive_closure();
        hierarchy
    }
    
    /// Build hierarchy from a pre-computed subsumption map (used by concurrent classification)
    pub fn from_subsumption_map(subsumptions: HashMap<ELConcept, HashSet<ELConcept>>) -> Self {
        let mut hierarchy = Self {
            subsumptions,
            transitive_subsumptions: HashMap::new(),
        };
        
        hierarchy.compute_transitive_closure();
        hierarchy
    }

    /// Add a subsumption relationship
    pub fn add_subsumption(&mut self, sub: ELConcept, sup: ELConcept) {
        self.subsumptions.entry(sub).or_insert_with(HashSet::new).insert(sup);
    }

    /// Check if one concept subsumes another
    pub fn is_subsumed(&self, sub: &ELConcept, sup: &ELConcept) -> bool {
        self.transitive_subsumptions
            .get(sub)
            .map(|sups| sups.contains(sup))
            .unwrap_or(false)
    }

    /// Compute transitive closure of subsumptions
    pub fn compute_transitive_closure(&mut self) {
        // Floyd-Warshall algorithm for transitive closure
        let concepts: Vec<_> = self.subsumptions.keys().cloned().collect();
        
        // Initialize with direct subsumptions
        for (sub, sups) in &self.subsumptions {
            self.transitive_subsumptions.insert(sub.clone(), sups.clone());
        }
        
        // Compute transitive closure
        for k in &concepts {
            for i in &concepts {
                for j in &concepts {
                    if self.is_in_transitive(i, k) && self.is_in_transitive(k, j) {
                        self.transitive_subsumptions
                            .entry(i.clone())
                            .or_insert_with(HashSet::new)
                            .insert(j.clone());
                    }
                }
            }
        }
    }

    fn is_in_transitive(&self, sub: &ELConcept, sup: &ELConcept) -> bool {
        self.transitive_subsumptions
            .get(sub)
            .map(|sups| sups.contains(sup))
            .unwrap_or(false)
    }

    /// Convert to classification hierarchy format
    pub fn to_classification_hierarchy(&self) -> HashMap<String, Vec<String>> {
        let mut hierarchy = HashMap::with_capacity(self.transitive_subsumptions.len());
        
        for (sub, sups) in &self.transitive_subsumptions {
            let sub_name = format!("{:?}", sub);
            let sup_names: Vec<_> = sups.iter()
                .map(|sup| format!("{:?}", sup))
                .collect();
            hierarchy.insert(sub_name, sup_names);
        }
        
        hierarchy
    }
    
    /// Get all subsumptions
    pub fn get_all_subsumptions(&self) -> &HashMap<ELConcept, HashSet<ELConcept>> {
        &self.transitive_subsumptions
    }
}

/// Role hierarchy for EL reasoning
#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    /// Direct role inclusions
    inclusions: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    /// Transitive closure
    transitive_inclusions: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
}

impl RoleHierarchy {
    /// Create a new role hierarchy
    pub fn new() -> Self {
        Self {
            inclusions: HashMap::new(),
            transitive_inclusions: HashMap::new(),
        }
    }

    /// Add a role inclusion
    pub fn add_inclusion(&mut self, sub_role: ObjectPropertyExpression, super_role: ObjectPropertyExpression) {
        self.inclusions.entry(sub_role).or_insert_with(HashSet::new).insert(super_role);
    }

    /// Compute transitive closure of role inclusions
    pub fn compute_transitive_closure(&mut self) {
        // Similar to concept hierarchy transitive closure
        let roles: Vec<_> = self.inclusions.keys().cloned().collect();
        
        // Initialize with direct inclusions
        self.transitive_inclusions = self.inclusions.clone();
        
        // Compute transitive closure
        for k in &roles {
            for i in &roles {
                for j in &roles {
                    if self.is_role_subsumed(i, k) && self.is_role_subsumed(k, j) {
                        self.transitive_inclusions
                            .entry(i.clone())
                            .or_insert_with(HashSet::new)
                            .insert(j.clone());
                    }
                }
            }
        }
    }

    /// Check if one role subsumes another
    pub fn is_role_subsumed(&self, sub_role: &ObjectPropertyExpression, super_role: &ObjectPropertyExpression) -> bool {
        self.transitive_inclusions
            .get(sub_role)
            .map(|sups| sups.contains(super_role))
            .unwrap_or(false)
    }

    /// Get all super-roles of a given role
    pub fn get_super_roles(&self, role: &ObjectPropertyExpression) -> HashSet<ObjectPropertyExpression> {
        self.transitive_inclusions
            .get(role)
            .cloned()
            .unwrap_or_default()
    }
}

/// Completion engine for EL reasoning
#[derive(Debug)]
pub struct CompletionEngine {
    /// Current state of completion
    state: CompletionState,
    /// Completion rules
    rules: Vec<Box<dyn CompletionRule>>,
    /// Queue of pending inferences
    queue: VecDeque<Inference>,
    /// Statistics
    completion_steps: usize,
}

impl CompletionEngine {
    /// Create a new completion engine
    pub fn new() -> Self {
        Self {
            state: CompletionState::new(),
            rules: vec![
                Box::new(SubsumptionRule),
                Box::new(ConjunctionRule),
                Box::new(ExistentialRule),
                Box::new(RoleChainRule),
            ],
            queue: VecDeque::new(),
            completion_steps: 0,
        }
    }

    /// Initialize with axioms and role hierarchy
    pub fn initialize(&mut self, axioms: &[ELAxiom], role_hierarchy: &RoleHierarchy) -> Result<()> {
        self.state.initialize(axioms, role_hierarchy)?;
        
        // Add initial inferences to queue
        for axiom in axioms {
            match axiom {
                ELAxiom::ConceptInclusion { lhs, rhs } => {
                    self.queue.push_back(Inference::Subsumption {
                        sub: lhs.clone(),
                        sup: rhs.clone(),
                    });
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    /// Run completion to fixpoint
    pub fn run_to_completion(&mut self) -> Result<()> {
        while let Some(inference) = self.queue.pop_front() {
            self.completion_steps += 1;
            
            // Apply all applicable rules
            for rule in &self.rules {
                let new_inferences = rule.apply(&inference, &mut self.state)?;
                for new_inf in new_inferences {
                    if !self.state.has_inference(&new_inf) {
                        self.queue.push_back(new_inf);
                    }
                }
            }
            
            // Add inference to state
            self.state.add_inference(inference);
        }
        
        Ok(())
    }

    /// Get all computed subsumptions
    pub fn get_all_subsumptions(&self) -> Vec<(ELConcept, ELConcept)> {
        self.state.get_all_subsumptions()
    }
}

/// Completion state for EL reasoning
#[derive(Debug)]
pub struct CompletionState {
    /// All computed subsumptions
    subsumptions: HashSet<(ELConcept, ELConcept)>,
    /// Existential fillers for each concept-role pair
    existential_fillers: HashMap<(ELConcept, ObjectPropertyExpression), HashSet<ELConcept>>,
    /// Role hierarchy reference
    role_hierarchy: Option<RoleHierarchy>,
}

impl CompletionState {
    /// Create new completion state
    pub fn new() -> Self {
        Self {
            subsumptions: HashSet::new(),
            existential_fillers: HashMap::new(),
            role_hierarchy: None,
        }
    }

    /// Initialize state with axioms
    pub fn initialize(&mut self, axioms: &[ELAxiom], role_hierarchy: &RoleHierarchy) -> Result<()> {
        self.role_hierarchy = Some(role_hierarchy.clone());
        
        // Initialize with direct subsumptions from axioms
        for axiom in axioms {
            if let ELAxiom::ConceptInclusion { lhs, rhs } = axiom {
                self.subsumptions.insert((lhs.clone(), rhs.clone()));
            }
        }
        
        Ok(())
    }

    /// Check if an inference is already known
    pub fn has_inference(&self, inference: &Inference) -> bool {
        match inference {
            Inference::Subsumption { sub, sup } => {
                self.subsumptions.contains(&(sub.clone(), sup.clone()))
            }
        }
    }

    /// Add an inference to the state
    pub fn add_inference(&mut self, inference: Inference) {
        match inference {
            Inference::Subsumption { sub, sup } => {
                self.subsumptions.insert((sub, sup));
            }
        }
    }

    /// Get all subsumptions
    pub fn get_all_subsumptions(&self) -> Vec<(ELConcept, ELConcept)> {
        self.subsumptions.iter().cloned().collect()
    }

    /// Check if a subsumption holds
    pub fn has_subsumption(&self, sub: &ELConcept, sup: &ELConcept) -> bool {
        self.subsumptions.contains(&(sub.clone(), sup.clone()))
    }
}

/// Types of inferences in EL completion
#[derive(Debug, Clone)]
pub enum Inference {
    /// Subsumption inference
    Subsumption {
        sub: ELConcept,
        sup: ELConcept,
    },
}

/// Trait for completion rules
pub trait CompletionRule: std::fmt::Debug + Send + Sync {
    /// Apply the rule to an inference and return new inferences
    fn apply(&self, inference: &Inference, state: &mut CompletionState) -> Result<Vec<Inference>>;
}

/// Subsumption rule: A ⊑ B, B ⊑ C ⟹ A ⊑ C
#[derive(Debug)]
pub struct SubsumptionRule;

impl CompletionRule for SubsumptionRule {
    fn apply(&self, inference: &Inference, state: &mut CompletionState) -> Result<Vec<Inference>> {
        let mut new_inferences = Vec::new();
        
        if let Inference::Subsumption { sub, sup } = inference {
            // Find all concepts that sup subsumes
            for (existing_sub, existing_sup) in &state.subsumptions {
                if existing_sub == sup {
                    // sub ⊑ sup, sup ⊑ existing_sup ⟹ sub ⊑ existing_sup
                    new_inferences.push(Inference::Subsumption {
                        sub: sub.clone(),
                        sup: existing_sup.clone(),
                    });
                }
                if existing_sup == sub {
                    // existing_sub ⊑ sub, sub ⊑ sup ⟹ existing_sub ⊑ sup
                    new_inferences.push(Inference::Subsumption {
                        sub: existing_sub.clone(),
                        sup: sup.clone(),
                    });
                }
            }
        }
        
        Ok(new_inferences)
    }
}

/// Conjunction rule: A ⊑ B ∩ C ⟹ A ⊑ B, A ⊑ C
#[derive(Debug)]
pub struct ConjunctionRule;

impl CompletionRule for ConjunctionRule {
    fn apply(&self, inference: &Inference, _state: &mut CompletionState) -> Result<Vec<Inference>> {
        let mut new_inferences = Vec::new();
        
        if let Inference::Subsumption { sub, sup } = inference {
            if let ELConcept::Conjunction(concepts) = sup {
                for concept in concepts {
                    new_inferences.push(Inference::Subsumption {
                        sub: sub.clone(),
                        sup: concept.clone(),
                    });
                }
            }
        }
        
        Ok(new_inferences)
    }
}

/// Existential rule: A ⊑ ∃r.B, B ⊑ C ⟹ A ⊑ ∃r.C
#[derive(Debug)]
pub struct ExistentialRule;

impl CompletionRule for ExistentialRule {
    fn apply(&self, inference: &Inference, state: &mut CompletionState) -> Result<Vec<Inference>> {
        let mut new_inferences = Vec::new();
        
        if let Inference::Subsumption { sub: filler_sub, sup: filler_sup } = inference {
            // Look for existential restrictions with this filler
            for (existing_sub, existing_sup) in &state.subsumptions {
                if let ELConcept::Existential { role, filler } = existing_sup {
                    if filler.as_ref() == filler_sub {
                        // existing_sub ⊑ ∃role.filler_sub, filler_sub ⊑ filler_sup ⟹ existing_sub ⊑ ∃role.filler_sup
                        new_inferences.push(Inference::Subsumption {
                            sub: existing_sub.clone(),
                            sup: ELConcept::Existential {
                                role: role.clone(),
                                filler: Box::new(filler_sup.clone()),
                            },
                        });
                    }
                }
            }
        }
        
        Ok(new_inferences)
    }
}

/// Role chain rule: A ⊑ ∃r.B, B ⊑ ∃s.C, r ∘ s ⊑ t ⟹ A ⊑ ∃t.C
#[derive(Debug)]
pub struct RoleChainRule;

impl CompletionRule for RoleChainRule {
    fn apply(&self, _inference: &Inference, _state: &mut CompletionState) -> Result<Vec<Inference>> {
        // Simplified implementation - would need role chain support
        Ok(Vec::new())
    }
}

/// Statistics for EL reasoning
#[derive(Debug, Default)]
pub struct ELStatistics {
    /// Time spent on initialization
    pub initialization_time: std::time::Duration,
    /// Time spent on classification
    pub classification_time: std::time::Duration,
    /// Number of completion steps
    pub completion_steps: usize,
    /// Memory usage in bytes
    pub memory_usage: usize,
}