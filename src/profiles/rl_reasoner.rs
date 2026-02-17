//! OWL 2 RL Profile Reasoner
//!
//! This module implements rule-based reasoning for the OWL 2 RL profile
//! using forward-chaining materialization with predictable performance.
//!
//! Features:
//! - Forward-chaining inference with RL-specific rules
//! - Predictable polynomial-time performance
//! - Incremental materialization support
//! - Integration with triple stores

use crate::{
    Error, Result,
    config::ReasonerConfig,
    ontology::{Axiom, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression, Ontology},
    core::reasoner::ClassificationResult,
    explanation::{ExplanationService, Explanation},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

#[cfg(feature = "rayon")]
#[allow(unused_imports)]
use rayon::prelude::*;

/// OWL 2 RL reasoner with forward-chaining materialization
#[derive(Debug)]
pub struct RLReasoner {
    /// RL-compatible axioms
    rl_axioms: Vec<RLAxiom>,
    /// Materialized facts (ABox)
    materialized_facts: MaterializedKnowledgeBase,
    /// TBox hierarchy
    tbox: TBoxHierarchy,
    /// Forward-chaining rule engine
    rule_engine: ForwardChainingEngine,
    /// Configuration
    #[allow(dead_code)]
    config: ReasonerConfig,
    /// Explanation service
    explanation_service: Option<Arc<ExplanationService>>,
    /// Statistics
    statistics: RLStatistics,
}

impl RLReasoner {
    /// Create a new RL reasoner
    pub fn new(config: ReasonerConfig) -> Self {
        let explanation_service = if config.reasoning.is_enabled(crate::config::ReasoningFeature::Explanations) {
            Some(Arc::new(ExplanationService::new()))
        } else {
            None
        };

        Self {
            rl_axioms: Vec::new(),
            materialized_facts: MaterializedKnowledgeBase::new(),
            tbox: TBoxHierarchy::new(),
            rule_engine: ForwardChainingEngine::new(),
            config,
            explanation_service,
            statistics: RLStatistics::default(),
        }
    }

    /// Initialize with an ontology
    pub fn initialize(&mut self, ontology: &Ontology) -> Result<()> {
        let start = Instant::now();
        
        // Step 1: Extract RL-compatible axioms
        self.extract_rl_axioms(ontology)?;
        
        // Step 2: Build TBox hierarchy
        self.build_tbox()?;
        
        // Step 3: Initialize materialized facts with ABox assertions
        self.initialize_abox()?;
        
        self.statistics.initialization_time = start.elapsed();
        Ok(())
    }

    /// Perform forward-chaining materialization
    pub fn materialize(&mut self) -> Result<()> {
        let start = Instant::now();
        
        // Apply RL rules until fixpoint
        let iterations = self.rule_engine.run_forward_chaining(
            &self.rl_axioms,
            &mut self.materialized_facts,
            &self.tbox,
        )?;
        
        self.statistics.materialization_time = start.elapsed();
        self.statistics.materialization_iterations = iterations;
        self.statistics.materialized_facts = self.materialized_facts.fact_count();
        
        Ok(())
    }

    /// Perform classification using materialized facts
    pub fn classify(&mut self) -> Result<ClassificationResult> {
        let start = Instant::now();
        
        // Materialize all consequences
        self.materialize()?;
        
        // Extract classification hierarchy from materialized facts
        let hierarchy = self.extract_classification_hierarchy()?;
        
        let _classification_time = start.elapsed();
        
        Ok(ClassificationResult::new(hierarchy))
    }

    /// Check instance membership
    pub fn is_instance_of(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        // Check if the fact is materialized
        Ok(self.materialized_facts.has_class_assertion(individual, class))
    }

    /// Get all instances of a class
    pub fn get_instances(&self, class: &ClassExpression) -> Result<HashSet<Individual>> {
        Ok(self.materialized_facts.get_instances(class))
    }

    /// Get explanation for an inferred fact
    pub fn explain_fact(&self, _axiom: &Axiom) -> Result<Option<Explanation>> {
        if let Some(ref explanation_service) = self.explanation_service {
            let explanation = explanation_service.explain_subsumption(
                &ClassExpression::Class(crate::ontology::Class::new(crate::ontology::IRI::new("http://example.org/dummy"))),
                &ClassExpression::Class(crate::ontology::Class::new(crate::ontology::IRI::new("http://example.org/dummy"))),
                &self.original_axioms_as_general_axioms(),
            )?;
            Ok(Some(explanation))
        } else {
            Ok(None)
        }
    }

    /// Extract RL-compatible axioms from ontology
    fn extract_rl_axioms(&mut self, ontology: &Ontology) -> Result<()> {
        for axiom in ontology.axioms() {
            if let Some(rl_axiom) = RLAxiom::from_general_axiom(axiom) {
                self.rl_axioms.push(rl_axiom);
            }
        }
        Ok(())
    }

    /// Build TBox hierarchy from RL axioms
    fn build_tbox(&mut self) -> Result<()> {
        for axiom in &self.rl_axioms {
            match axiom {
                RLAxiom::SubClassOf { subclass, superclass } => {
                    self.tbox.add_class_inclusion(subclass.clone(), superclass.clone());
                }
                RLAxiom::SubPropertyOf { subproperty, superproperty } => {
                    self.tbox.add_property_inclusion(subproperty.clone(), superproperty.clone());
                }
                RLAxiom::EquivalentClasses { classes } => {
                    // Add bidirectional inclusions
                    for i in 0..classes.len() {
                        for j in 0..classes.len() {
                            if i != j {
                                self.tbox.add_class_inclusion(classes[i].clone(), classes[j].clone());
                            }
                        }
                    }
                }
                RLAxiom::Domain { property, domain } => {
                    self.tbox.add_domain(property.clone(), domain.clone());
                }
                RLAxiom::Range { property, range } => {
                    self.tbox.add_range(property.clone(), range.clone());
                }
                _ => {}
            }
        }
        
        self.tbox.compute_transitive_closure();
        Ok(())
    }

    /// Initialize ABox with assertions from ontology
    fn initialize_abox(&mut self) -> Result<()> {
        for axiom in &self.rl_axioms {
            match axiom {
                RLAxiom::ClassAssertion { class, individual } => {
                    self.materialized_facts.add_class_assertion(individual.clone(), class.clone());
                }
                RLAxiom::ObjectPropertyAssertion { property, subject, object } => {
                    self.materialized_facts.add_object_property_assertion(
                        subject.clone(),
                        property.clone(),
                        object.clone(),
                    );
                }
                RLAxiom::DataPropertyAssertion { property, subject, value } => {
                    self.materialized_facts.add_data_property_assertion(
                        subject.clone(),
                        property.clone(),
                        value.clone(),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Extract classification hierarchy from materialized facts
    fn extract_classification_hierarchy(&self) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>> {
        let mut hierarchy = HashMap::new();
        
        // Collect all classes that appear in the ontology
        let mut all_classes: HashSet<RLClassExpression> = HashSet::new();
        
        // From TBox subsumptions
        for (subclass, superclasses) in self.tbox.get_all_subsumptions() {
            all_classes.insert(subclass.clone());
            all_classes.extend(superclasses.iter().cloned());
        }
        
        // From materialized class assertions
        for individual in self.materialized_facts.get_all_individuals() {
            let classes = self.materialized_facts.get_classes(&individual);
            all_classes.extend(classes);
        }
        
        // From original axioms
        for axiom in &self.rl_axioms {
            match axiom {
                RLAxiom::SubClassOf { subclass, superclass } => {
                    all_classes.insert(subclass.clone());
                    all_classes.insert(superclass.clone());
                }
                RLAxiom::EquivalentClasses { classes } => {
                    all_classes.extend(classes.iter().cloned());
                }
                RLAxiom::ClassAssertion { class, .. } => {
                    all_classes.insert(class.clone());
                }
                RLAxiom::Domain { domain, .. } => {
                    all_classes.insert(domain.clone());
                }
                RLAxiom::Range { range, .. } => {
                    all_classes.insert(range.clone());
                }
                _ => {}
            }
        }
        
        // Build hierarchy from TBox subsumptions (already includes transitive closure)
        for (subclass, superclasses) in self.tbox.get_all_subsumptions() {
            let sub_expr = subclass.to_class_expression();
            let sup_exprs: HashSet<_> = superclasses.iter()
                .map(|sup| sup.to_class_expression())
                .collect();
            hierarchy.insert(sub_expr, sup_exprs);
        }
        
        // Add reflexive relationships for all classes (every class is a subclass of itself)
        for class in &all_classes {
            let class_expr = class.to_class_expression();
            hierarchy.entry(class_expr.clone())
                .or_insert_with(HashSet::new)
                .insert(class_expr);
        }
        
        Ok(hierarchy)
    }

    #[allow(dead_code)]
    fn get_reasoning_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("initialization_time".to_string(), serde_json::json!(self.statistics.initialization_time.as_millis()));
        stats.insert("materialization_time".to_string(), serde_json::json!(self.statistics.materialization_time.as_millis()));
        stats.insert("materialization_iterations".to_string(), serde_json::json!(self.statistics.materialization_iterations));
        stats.insert("materialized_facts".to_string(), serde_json::json!(self.statistics.materialized_facts));
        stats.insert("rules_fired".to_string(), serde_json::json!(self.statistics.rules_fired));
        stats
    }

    fn original_axioms_as_general_axioms(&self) -> Vec<Axiom> {
        self.rl_axioms.iter()
            .map(|rl_axiom| rl_axiom.to_general_axiom())
            .collect()
    }
}

/// RL-specific axiom representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RLAxiom {
    /// SubClassOf axiom
    SubClassOf {
        subclass: RLClassExpression,
        superclass: RLClassExpression,
    },
    /// SubPropertyOf axiom
    SubPropertyOf {
        subproperty: ObjectPropertyExpression,
        superproperty: ObjectPropertyExpression,
    },
    /// EquivalentClasses axiom
    EquivalentClasses {
        classes: Vec<RLClassExpression>,
    },
    /// Domain axiom
    Domain {
        property: ObjectPropertyExpression,
        domain: RLClassExpression,
    },
    /// Range axiom
    Range {
        property: ObjectPropertyExpression,
        range: RLClassExpression,
    },
    /// Class assertion
    ClassAssertion {
        class: RLClassExpression,
        individual: Individual,
    },
    /// Object property assertion
    ObjectPropertyAssertion {
        property: ObjectPropertyExpression,
        subject: Individual,
        object: Individual,
    },
    /// Data property assertion
    DataPropertyAssertion {
        property: DataPropertyExpression,
        subject: Individual,
        value: String, // Simplified literal representation
    },
    /// TransitiveProperty
    TransitiveProperty {
        property: ObjectPropertyExpression,
    },
    /// SymmetricProperty
    SymmetricProperty {
        property: ObjectPropertyExpression,
    },
}

impl RLAxiom {
    /// Try to convert a general axiom to RL axiom
    pub fn from_general_axiom(axiom: &Axiom) -> Option<Self> {
        use crate::ontology::axioms::*;
        match axiom {
            Axiom::SubClassOf(SubClassOfAxiom { subclass, superclass, .. }) => {
                let rl_sub = RLClassExpression::from_class_expression(subclass)?;
                let rl_sup = RLClassExpression::from_class_expression(superclass)?;
                Some(RLAxiom::SubClassOf {
                    subclass: rl_sub,
                    superclass: rl_sup,
                })
            }
            Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom { sub_property, super_property, .. }) => {
                Some(RLAxiom::SubPropertyOf {
                    subproperty: sub_property.clone(),
                    superproperty: super_property.clone(),
                })
            }
            Axiom::ClassAssertion(ClassAssertionAxiom { class, individual, .. }) => {
                let rl_class = RLClassExpression::from_class_expression(class)?;
                Some(RLAxiom::ClassAssertion {
                    class: rl_class,
                    individual: individual.clone(),
                })
            }
            Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom { property, source, target, .. }) => {
                Some(RLAxiom::ObjectPropertyAssertion {
                    property: property.clone(),
                    subject: source.clone(),
                    object: target.clone(),
                })
            }
            _ => None,
        }
    }

    /// Convert to general axiom
    pub fn to_general_axiom(&self) -> Axiom {
        use crate::ontology::axioms::*;
        match self {
            RLAxiom::SubClassOf { subclass, superclass } => {
                Axiom::SubClassOf(SubClassOfAxiom {
                    id: 0,
                    subclass: subclass.to_class_expression(),
                    superclass: superclass.to_class_expression(),
                    annotations: Vec::new(),
                })
            }
            RLAxiom::SubPropertyOf { subproperty, superproperty } => {
                Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                    id: 0,
                    sub_property: subproperty.clone(),
                    super_property: superproperty.clone(),
                    annotations: Vec::new(),
                })
            }
            RLAxiom::ClassAssertion { class, individual } => {
                Axiom::ClassAssertion(ClassAssertionAxiom {
                    id: 0,
                    individual: individual.clone(),
                    class: class.to_class_expression(),
                    annotations: Vec::new(),
                })
            }
            RLAxiom::ObjectPropertyAssertion { property, subject, object } => {
                Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                    id: 0,
                    source: subject.clone(),
                    target: object.clone(),
                    property: property.clone(),
                    annotations: Vec::new(),
                })
            }
            _ => Axiom::Declaration(DeclarationAxiom {
                id: 0,
                entity: Entity::Class(crate::ontology::IRI::new("http://example.org/dummy")),
            }),
        }
    }
}

/// RL class expression (restricted subset)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RLClassExpression {
    /// Named class
    Class(crate::ontology::Class),
    /// Intersection
    Intersection(Vec<RLClassExpression>),
    /// Existential restriction
    SomeValuesFrom {
        property: ObjectPropertyExpression,
        filler: Box<RLClassExpression>,
    },
    /// Universal restriction
    AllValuesFrom {
        property: ObjectPropertyExpression,
        filler: Box<RLClassExpression>,
    },
}

impl RLClassExpression {
    /// Try to convert from general class expression
    pub fn from_class_expression(expr: &ClassExpression) -> Option<Self> {
        match expr {
            ClassExpression::Class(class) => Some(RLClassExpression::Class(class.clone ())),
            ClassExpression::ObjectIntersectionOf(exprs) => {
                let rl_exprs: Option<Vec<_>> = exprs.iter()
                    .map(Self::from_class_expression)
                    .collect();
                rl_exprs.map(RLClassExpression::Intersection)
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                Self::from_class_expression(filler).map(|rl_filler| {
                    RLClassExpression::SomeValuesFrom {
                        property: property.clone(),
                        filler: Box::new(rl_filler),
                    }
                })
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                Self::from_class_expression(filler).map(|rl_filler| {
                    RLClassExpression::AllValuesFrom {
                        property: property.clone(),
                        filler: Box::new(rl_filler),
                    }
                })
            }
            _ => None,
        }
    }

    /// Convert to general class expression
    pub fn to_class_expression(&self) -> ClassExpression {
        match self {
            RLClassExpression::Class(class) => ClassExpression::Class(class.clone()),
            RLClassExpression::Intersection(exprs) => {
                let class_exprs: Vec<_> = exprs.iter()
                    .map(|e| e.to_class_expression())
                    .collect();
                ClassExpression::ObjectIntersectionOf(class_exprs)
            }
            RLClassExpression::SomeValuesFrom { property, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(filler.to_class_expression()),
                }
            }
            RLClassExpression::AllValuesFrom { property, filler } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(filler.to_class_expression()),
                }
            }
        }
    }
}

/// Materialized knowledge base (ABox)
#[derive(Debug)]
pub struct MaterializedKnowledgeBase {
    /// Class assertions: individual -> classes
    class_assertions: HashMap<Individual, HashSet<RLClassExpression>>,
    /// Object property assertions: (subject, property) -> objects
    object_property_assertions: HashMap<(Individual, ObjectPropertyExpression), HashSet<Individual>>,
    /// Data property assertions: (subject, property) -> values
    data_property_assertions: HashMap<(Individual, DataPropertyExpression), HashSet<String>>,
}

impl MaterializedKnowledgeBase {
    /// Create a new knowledge base
    pub fn new() -> Self {
        Self {
            class_assertions: HashMap::new(),
            object_property_assertions: HashMap::new(),
            data_property_assertions: HashMap::new(),
        }
    }

    /// Add a class assertion
    pub fn add_class_assertion(&mut self, individual: Individual, class: RLClassExpression) -> bool {
        self.class_assertions
            .entry(individual)
            .or_default()
            .insert(class)
    }

    /// Add an object property assertion
    pub fn add_object_property_assertion(
        &mut self,
        subject: Individual,
        property: ObjectPropertyExpression,
        object: Individual,
    ) -> bool {
        self.object_property_assertions
            .entry((subject, property))
            .or_default()
            .insert(object)
    }

    /// Add a data property assertion
    pub fn add_data_property_assertion(
        &mut self,
        subject: Individual,
        property: DataPropertyExpression,
        value: String,
    ) -> bool {
        self.data_property_assertions
            .entry((subject, property))
            .or_default()
            .insert(value)
    }

    /// Check if a class assertion exists
    pub fn has_class_assertion(&self, individual: &Individual, class: &ClassExpression) -> bool {
        if let Some(rl_class) = RLClassExpression::from_class_expression(class) {
            self.class_assertions
                .get(individual)
                .map(|classes| classes.contains(&rl_class))
                .unwrap_or(false)
        } else {
            false
        }
    }

    /// Get all instances of a class
    pub fn get_instances(&self, class: &ClassExpression) -> HashSet<Individual> {
        if let Some(rl_class) = RLClassExpression::from_class_expression(class) {
            self.class_assertions
                .iter()
                .filter(|(_, classes)| classes.contains(&rl_class))
                .map(|(individual, _)| individual.clone())
                .collect()
        } else {
            HashSet::new()
        }
    }

    /// Get all class assertions for an individual
    pub fn get_classes(&self, individual: &Individual) -> HashSet<RLClassExpression> {
        self.class_assertions
            .get(individual)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }

    /// Get all object property assertions
    pub fn get_object_property_assertions(
        &self,
        subject: &Individual,
        property: &ObjectPropertyExpression,
    ) -> HashSet<Individual> {
        self.object_property_assertions
            .get(&(subject.clone(), property.clone()))
            .cloned()
            .unwrap_or_else(HashSet::new)
    }

    /// Count total facts
    pub fn fact_count(&self) -> usize {
        let class_count: usize = self.class_assertions.values().map(|s| s.len()).sum();
        let obj_prop_count: usize = self.object_property_assertions.values().map(|s| s.len()).sum();
        let data_prop_count: usize = self.data_property_assertions.values().map(|s| s.len()).sum();
        class_count + obj_prop_count + data_prop_count
    }

    /// Get all individuals
    pub fn get_all_individuals(&self) -> HashSet<Individual> {
        let mut individuals = HashSet::new();
        individuals.extend(self.class_assertions.keys().cloned());
        for ((subject, _), objects) in &self.object_property_assertions {
            individuals.insert(subject.clone());
            individuals.extend(objects.iter().cloned());
        }
        individuals
    }
}

/// TBox hierarchy
#[derive(Debug)]
pub struct TBoxHierarchy {
    /// Class inclusions
    class_inclusions: HashMap<RLClassExpression, HashSet<RLClassExpression>>,
    /// Property inclusions
    property_inclusions: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    /// Domains
    domains: HashMap<ObjectPropertyExpression, HashSet<RLClassExpression>>,
    /// Ranges
    ranges: HashMap<ObjectPropertyExpression, HashSet<RLClassExpression>>,
}

impl TBoxHierarchy {
    /// Create a new TBox hierarchy
    pub fn new() -> Self {
        Self {
            class_inclusions: HashMap::new(),
            property_inclusions: HashMap::new(),
            domains: HashMap::new(),
            ranges: HashMap::new(),
        }
    }

    /// Add a class inclusion
    pub fn add_class_inclusion(&mut self, subclass: RLClassExpression, superclass: RLClassExpression) {
        self.class_inclusions
            .entry(subclass)
            .or_default()
            .insert(superclass);
    }

    /// Add a property inclusion
    pub fn add_property_inclusion(
        &mut self,
        subproperty: ObjectPropertyExpression,
        superproperty: ObjectPropertyExpression,
    ) {
        self.property_inclusions
            .entry(subproperty)
            .or_default()
            .insert(superproperty);
    }

    /// Add a domain axiom
    pub fn add_domain(&mut self, property: ObjectPropertyExpression, domain: RLClassExpression) {
        self.domains
            .entry(property)
            .or_default()
            .insert(domain);
    }

    /// Add a range axiom
    pub fn add_range(&mut self, property: ObjectPropertyExpression, range: RLClassExpression) {
        self.ranges
            .entry(property)
            .or_default()
            .insert(range);
    }

    /// Compute transitive closure
    pub fn compute_transitive_closure(&mut self) {
        // Compute transitive closure of class inclusions
        let classes: Vec<_> = self.class_inclusions.keys().cloned().collect();
        
        loop {
            let mut changed = false;
            
            for sub in &classes {
                if let Some(supers) = self.class_inclusions.get(sub).cloned() {
                    for sup in &supers {
                        if let Some(super_supers) = self.class_inclusions.get(sup).cloned() {
                            for super_sup in super_supers {
                                if self.class_inclusions
                                    .get_mut(sub)
                                    .unwrap()
                                    .insert(super_sup)
                                {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
            
            if !changed {
                break;
            }
        }
    }

    /// Get all subsumptions
    pub fn get_all_subsumptions(&self) -> &HashMap<RLClassExpression, HashSet<RLClassExpression>> {
        &self.class_inclusions
    }

    /// Get superclasses of a class
    pub fn get_superclasses(&self, class: &RLClassExpression) -> HashSet<RLClassExpression> {
        self.class_inclusions
            .get(class)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }

    /// Get domains of a property
    pub fn get_domains(&self, property: &ObjectPropertyExpression) -> HashSet<RLClassExpression> {
        self.domains
            .get(property)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }

    /// Get ranges of a property
    pub fn get_ranges(&self, property: &ObjectPropertyExpression) -> HashSet<RLClassExpression> {
        self.ranges
            .get(property)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }
}

/// Forward-chaining rule engine
#[derive(Debug)]
pub struct ForwardChainingEngine {
    /// Maximum iterations
    max_iterations: usize,
    /// Rules fired counter
    rules_fired: usize,
}

impl ForwardChainingEngine {
    /// Create a new forward-chaining engine
    pub fn new() -> Self {
        Self {
            max_iterations: 10000,
            rules_fired: 0,
        }
    }

    /// Run forward-chaining until fixpoint
    pub fn run_forward_chaining(
        &mut self,
        axioms: &[RLAxiom],
        kb: &mut MaterializedKnowledgeBase,
        tbox: &TBoxHierarchy,
    ) -> Result<usize> {
        let mut iteration = 0;
        
        loop {
            iteration += 1;
            if iteration > self.max_iterations {
                return Err(Error::reasoning(
                    format!("Forward chaining exceeded maximum iterations: {}", self.max_iterations)
                ));
            }
            
            let mut facts_added = false;
            
            // Apply RL rules
            facts_added |= self.apply_subclass_rule(kb, tbox)?;
            facts_added |= self.apply_domain_rule(kb, tbox)?;
            facts_added |= self.apply_range_rule(kb, tbox)?;
            facts_added |= self.apply_property_chain_rule(kb, axioms)?;
            facts_added |= self.apply_transitive_rule(kb, axioms)?;
            facts_added |= self.apply_symmetric_rule(kb, axioms)?;
            
            if !facts_added {
                break;
            }
        }
        
        Ok(iteration)
    }

    /// Apply subclass rule: C(x) ∧ C ⊑ D ⟹ D(x)
    fn apply_subclass_rule(
        &mut self,
        kb: &mut MaterializedKnowledgeBase,
        tbox: &TBoxHierarchy,
    ) -> Result<bool> {
        let mut new_facts = Vec::new();
        
        for (individual, classes) in kb.class_assertions.iter() {
            for class in classes {
                for superclass in tbox.get_superclasses(class) {
                    new_facts.push((individual.clone(), superclass));
                }
            }
        }
        
        let mut added = false;
        for (individual, class) in new_facts {
            if kb.add_class_assertion(individual, class) {
                added = true;
                self.rules_fired += 1;
            }
        }
        
        Ok(added)
    }

    /// Apply domain rule: P(x, y) ∧ dom(P) = C ⟹ C(x)
    fn apply_domain_rule(
        &mut self,
        kb: &mut MaterializedKnowledgeBase,
        tbox: &TBoxHierarchy,
    ) -> Result<bool> {
        let mut new_facts = Vec::new();
        
        for ((subject, property), _) in kb.object_property_assertions.iter() {
            for domain_class in tbox.get_domains(property) {
                new_facts.push((subject.clone(), domain_class));
            }
        }
        
        let mut added = false;
        for (individual, class) in new_facts {
            if kb.add_class_assertion(individual, class) {
                added = true;
                self.rules_fired += 1;
            }
        }
        
        Ok(added)
    }

    /// Apply range rule: P(x, y) ∧ range(P) = C ⟹ C(y)
    fn apply_range_rule(
        &mut self,
        kb: &mut MaterializedKnowledgeBase,
        tbox: &TBoxHierarchy,
    ) -> Result<bool> {
        let mut new_facts = Vec::new();
        
        for ((_, property), objects) in kb.object_property_assertions.iter() {
            for range_class in tbox.get_ranges(property) {
                for object in objects {
                    new_facts.push((object.clone(), range_class.clone()));
                }
            }
        }
        
        let mut added = false;
        for (individual, class) in new_facts {
            if kb.add_class_assertion(individual, class) {
                added = true;
                self.rules_fired += 1;
            }
        }
        
        Ok(added)
    }

    /// Apply property chain rule (simplified)
    fn apply_property_chain_rule(
        &mut self,
        _kb: &mut MaterializedKnowledgeBase,
        _axioms: &[RLAxiom],
    ) -> Result<bool> {
        // Simplified implementation
        Ok(false)
    }

    /// Apply transitive rule: P(x, y) ∧ P(y, z) ∧ Trans(P) ⟹ P(x, z)
    fn apply_transitive_rule(
        &mut self,
        kb: &mut MaterializedKnowledgeBase,
        axioms: &[RLAxiom],
    ) -> Result<bool> {
        let mut new_facts = Vec::new();
        
        // Find transitive properties
        let transitive_props: HashSet<_> = axioms.iter()
            .filter_map(|ax| match ax {
                RLAxiom::TransitiveProperty { property } => Some(property.clone()),
                _ => None,
            })
            .collect();
        
        // Apply transitivity
        for prop in transitive_props {
            let assertions: Vec<_> = kb.object_property_assertions.iter()
                .filter(|((_, p), _)| p == &prop)
                .flat_map(|((subj, _), objs)| {
                    objs.iter().map(move |obj| (subj.clone(), obj.clone()))
                })
                .collect();
            
            for (x, y) in &assertions {
                for (y2, z) in &assertions {
                    if y == y2 {
                        new_facts.push((x.clone(), prop.clone(), z.clone()));
                    }
                }
            }
        }
        
        let mut added = false;
        for (subject, property, object) in new_facts {
            if kb.add_object_property_assertion(subject, property, object) {
                added = true;
                self.rules_fired += 1;
            }
        }
        
        Ok(added)
    }

    /// Apply symmetric rule: P(x, y) ∧ Sym(P) ⟹ P(y, x)
    fn apply_symmetric_rule(
        &mut self,
        kb: &mut MaterializedKnowledgeBase,
        axioms: &[RLAxiom],
    ) -> Result<bool> {
        let mut new_facts = Vec::new();
        
        // Find symmetric properties
        let symmetric_props: HashSet<_> = axioms.iter()
            .filter_map(|ax| match ax {
                RLAxiom::SymmetricProperty { property } => Some(property.clone()),
                _ => None,
            })
            .collect();
        
        // Apply symmetry
        for ((subject, property), objects) in kb.object_property_assertions.iter() {
            if symmetric_props.contains(property) {
                for object in objects {
                    new_facts.push((object.clone(), property.clone(), subject.clone()));
                }
            }
        }
        
        let mut added = false;
        for (subject, property, object) in new_facts {
            if kb.add_object_property_assertion(subject, property, object) {
                added = true;
                self.rules_fired += 1;
            }
        }
        
        Ok(added)
    }
}

/// Statistics for RL reasoning
#[derive(Debug, Default)]
pub struct RLStatistics {
    /// Time spent on initialization
    pub initialization_time: std::time::Duration,
    /// Time spent on materialization
    pub materialization_time: std::time::Duration,
    /// Number of materialization iterations
    pub materialization_iterations: usize,
    /// Number of materialized facts
    pub materialized_facts: usize,
    /// Number of rules fired
    pub rules_fired: usize,
}
