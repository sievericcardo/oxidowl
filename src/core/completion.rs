//! Completion rule system for tableau expansion
//!
//! This module implements the core completion rules for SROIQV(D) tableau
//! reasoning, including support for rule application, priority management,
//! and clash detection.

use crate::{
    core::dependency::DependencySet,
    ontology::{ClassExpression, Individual, Role, DataProperty, ObjectPropertyExpression}, Result,
};
use std::{
    collections::HashMap,
    fmt,
};

/// Helper function to convert string to Individual
fn string_to_individual(node_id: String) -> Individual {
    Individual::Named(crate::ontology::NamedIndividual {
        iri: crate::ontology::IRI::from(node_id),
    })
}

/// Completion rules for tableau expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionRule {
    /// Conjunction rule (A ⊓ B)
    And,
    /// Disjunction rule (A ⊔ B)
    Or,
    /// Existential rule (∃R.C)
    Some,
    /// Universal rule (∀R.C)
    All,
    /// At-least cardinality rule (≥nR.C)
    AtLeast,
    /// At-most cardinality rule (≤nR.C)
    AtMost,
    /// Nominal rule (handling nominals)
    Nominal,
    /// Self rule (∃R.Self)
    Self_,
    /// Choose rule (disjunction)
    Choose,
    /// Datatype rule
    Datatype,
    /// Unfold rule
    Unfold,
    /// Property chain rule
    PropertyChain,
    /// Guess rule
    Guess,
}

impl CompletionRule {
    /// Apply the completion rule
    pub fn apply(&self, application: &RuleApplication) -> Result<Vec<RuleApplication>> {
        match self {
            CompletionRule::And => Ok(vec![application.clone()]),
            CompletionRule::Or => Ok(vec![application.clone()]),
            CompletionRule::Some => Ok(vec![application.clone()]),
            CompletionRule::All => Ok(vec![application.clone()]),
            CompletionRule::AtLeast => Ok(vec![application.clone()]),
            CompletionRule::AtMost => Ok(vec![application.clone()]),
            CompletionRule::Nominal => Ok(vec![application.clone()]),
            CompletionRule::Self_ => Ok(vec![application.clone()]),
            CompletionRule::Choose => Ok(vec![application.clone()]),
            CompletionRule::Datatype => Ok(vec![application.clone()]),
            CompletionRule::Unfold => Ok(vec![application.clone()]),
            CompletionRule::PropertyChain => Ok(vec![application.clone()]),
            CompletionRule::Guess => Ok(vec![application.clone()]),
        }
    }
}

/// Priority levels for rule application ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulePriority {
    /// Critical rules (deterministic, no choice)
    Highest = 0,

    /// High priority (propagation, essential for completeness)
    High = 1,
    
    /// Normal priority (existential, universal, etc.)
    Normal = 2,
    
    /// Low priority (cardinality, non-critical)
    Low = 3,

    /// Lowest priority (cleanup, optimisation)
    Lowest = 4,
}

/// Completion rule application context
#[derive(Debug, Clone)]
pub struct RuleApplication {
    /// Rule to apply
    pub rule: CompletionRule,

    /// Target individual or concept
    pub node: String,

    /// Rule-specific context
    pub context: RuleContext,

    /// Priority for application
    pub priority: RulePriority,

    /// Dependencies for this rule application
    pub dependencies: DependencySet,
}

/// Context specific to each rule type
#[derive(Debug, Clone)]
pub enum RuleContext {
    /// Context for concept-based rules (AND, OR, etc.)
    Concept {
        concept: ClassExpression,
        dependencies: DependencySet,
    },

    /// Context for role-based rules (SOME, ALL, etc.)
    Role {
        role: Role,
        source: String,
        target: String,
        concept: ClassExpression,
    },

    /// Context for cardinality rules
    Cardinality {
        cardinality: u32,
        role: Role,
        filler: Option<ClassExpression>,
        existing_successors: Vec<String>,
    },

    /// Context for nominal rules
    Nominal {
        nominal: Individual,
        current_node: String,
    },

    /// Context for datatype rules
    Datatype {
        property: DataProperty,
        restriction: String,
        value: Option<String>,
    },

    /// Context for merge rules
    Merge {
        source: String,
        target: String,
        reason: String,
    },

    /// Context for at-most cardinality rules
    AtMost {
        node_id: String,
        cardinality: u32,
        property: Role,
        filler: ClassExpression,
    },

    /// Context for property chain rules
    PropertyChain {
        chain: Vec<ObjectPropertyExpression>,
        target: String,
        source: String,
        super_property: ObjectPropertyExpression,
    },
}

/// Set of completion rules with application strategies
pub struct CompletionRuleSet {
    /// Available rules in priority order
    rules: Vec<CompletionRule>,
    
    /// Rule priority mapping
    priorities: HashMap<CompletionRule, RulePriority>,

    /// Rule applicability checkers
    applicability: HashMap<CompletionRule, Box<dyn Fn(&RuleApplication) -> bool + Send + Sync>>,

    /// Rule application handlers
    handlers: HashMap<CompletionRule, Box<dyn Fn(RuleApplication) -> Result<Vec<RuleApplication>> + Send + Sync>>,
}

impl std::fmt::Debug for CompletionRuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionRuleSet")
            .field("rules", &self.rules)
            .field("priorities", &self.priorities)
            .field("applicability", &self.applicability.keys().collect::<Vec<_>>())
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Rule application result
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// New rule applications generated by this rule
    pub new_applications: Vec<RuleApplication>,

    /// Concept additions required
    pub concept_additions: Vec<(Individual, ClassExpression, DependencySet)>,

    /// Role additions required
    pub role_additions: Vec<(Individual, Individual, ObjectPropertyExpression, DependencySet)>,

    /// Edge additions required (legacy)
    pub edge_additions: Vec<(String, String, ObjectPropertyExpression, DependencySet)>,

    /// New individuals created
    pub new_individuals: Vec<(String, DependencySet)>,

    /// Merges to perform
    pub merges: Vec<(String, String, DependencySet)>,

    /// Clashes detected
    pub clashes: Vec<ClashInfo>,

    /// Branching points created
    pub branches: Vec<BranchInfo>,
    
    /// Branching points for choice rules
    pub branching_points: Vec<(crate::core::hypertableau::branching::BranchingType, Vec<crate::core::hypertableau::branching::BranchingChoice>)>,
    
    /// Data property assertions
    pub data_assertions: Vec<(Individual, String, crate::ontology::DataPropertyExpression, DependencySet)>,
    
    /// Datatype constraints
    pub datatype_constraints: Vec<(String, crate::ontology::DataRange, DependencySet)>,
    
    /// Universal constraints for validation
    pub universal_constraints: Vec<(Individual, crate::ontology::DataPropertyExpression, ClassExpression, DependencySet)>,
    
    /// Cardinality constraints for validation
    pub cardinality_constraints: Vec<(Individual, ObjectPropertyExpression, u32, ClassExpression, bool, DependencySet)>,
}

/// Information about a clash detected during rule application
#[derive(Debug, Clone)]
pub struct ClashInfo {
    /// Type of clash (e.g., unsatisfiability)
    pub clash_type: ClashType,

    /// Nodes involved in the clash
    pub nodes: Vec<String>,

    /// Concepts involved
    pub concepts: Vec<ClassExpression>,

    /// Dependencies leading to the clash
    pub dependencies: DependencySet,

    /// Explanation of the clash
    pub explanation: String,
}

/// Types of clashes that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClashType {
    /// Contradiction: A and ¬A
    Contradiction,
    
    /// Cardinality violation: too many/few successors
    Cardinality,
    
    /// Datatype inconsistency
    Datatype,
    
    /// Functionality violation
    Functionality,
    
    /// Distinctness violation
    Distinctness,
    
    /// Nominal conflict
    Nominal,
}

/// Information about a branching point created during rule application
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branching rule
    pub rule: CompletionRule,

    /// Node where branching occurs
    pub node: String,

    /// Branch choices
    pub choices: Vec<ClassExpression>,

    /// Dependencies for this branching
    pub dependencies: DependencySet,
}

/// Rule application strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleApplicationStrategy {
    /// Apply rules in a fixed order
    Priority,

    /// Apply deterministic rules first, then non-deterministic
    DeterministicFirst,

    /// Apply rules in breadth-first order
    BreadthFirst,

    /// Apply rules in depth-first order
    DepthFirst,
}

impl CompletionRuleSet {
    /// Create a new completion rule set
    pub fn new() -> Self {
        let mut rule_set = Self {
            rules: Vec::new(),
            priorities: HashMap::new(),
            applicability: HashMap::new(),
            handlers: HashMap::new(),
        };

        rule_set.add_standard_rules();
        rule_set
    }

    /// Add standard OWL 2 DL completion rules to the set
    fn add_standard_rules(&mut self) {
        // Deterministic rules (highest priority)
        self.add_rule(CompletionRule::And, RulePriority::Highest);
        self.add_rule(CompletionRule::All, RulePriority::High);
        self.add_rule(CompletionRule::Self_, RulePriority::High);
        self.add_rule(CompletionRule::Unfold, RulePriority::High);
        
        // Existential expansion (normal priority)
        self.add_rule(CompletionRule::Some, RulePriority::Normal);
        self.add_rule(CompletionRule::Nominal, RulePriority::Normal);
        
        // Non-deterministic rules (low priority)
        self.add_rule(CompletionRule::Or, RulePriority::Low);
        self.add_rule(CompletionRule::AtLeast, RulePriority::Low);
        self.add_rule(CompletionRule::AtMost, RulePriority::Low);
        self.add_rule(CompletionRule::Choose, RulePriority::Low);
        self.add_rule(CompletionRule::Guess, RulePriority::Low);

        // Special rules
        self.add_rule(CompletionRule::Datatype, RulePriority::Lowest);
    }

    /// Add a completion rule
    pub fn add_rule(&mut self, rule: CompletionRule, priority: RulePriority) {
        self.priorities.insert(rule, priority);
        if !self.rules.contains(&rule) {
            self.rules.push(rule);
            // Sort the rules by priority
            let priorities = &self.priorities;
            self.rules.sort_by_key(|r| {
                priorities.get(r).cloned().unwrap_or(RulePriority::Normal)
            });
        }
    }

    /// Get rule priority
    pub fn get_priority(&self, rule: CompletionRule) -> RulePriority {
        self.priorities.get(&rule).cloned().unwrap_or(RulePriority::Normal)
    }

    /// Get all applicable rules
    pub fn get_applicable_rules(&self, concept: &ClassExpression) -> Vec<CompletionRule> {
        self.rules.iter()
            .filter(|&rule| self.is_rule_applicable(*rule, concept))
            .cloned()
            .collect()
    }

    /// Check if a rule is applicable to a concept
    pub fn is_rule_applicable(&self, rule: CompletionRule, concept: &ClassExpression) -> bool {
        match rule {
            CompletionRule::And => matches!(concept, ClassExpression::ObjectIntersectionOf(_)),
            CompletionRule::Or => matches!(concept, ClassExpression::ObjectUnionOf(_)),
            CompletionRule::Some => matches!(concept, ClassExpression::ObjectSomeValuesFrom { .. }),
            CompletionRule::All => false, // Applied based on edges, not concepts
            CompletionRule::AtLeast => matches!(concept, ClassExpression::ObjectMinCardinality { .. }),
            CompletionRule::AtMost => matches!(concept, ClassExpression::ObjectMaxCardinality { .. }),
            CompletionRule::Nominal => matches!(concept, ClassExpression::ObjectOneOf(_)),
            CompletionRule::Self_ => matches!(concept, ClassExpression::ObjectHasSelf { .. }),
            CompletionRule::Choose => false, // Applied by strategy
            CompletionRule::Datatype => matches!(concept, ClassExpression::DataSomeValuesFrom { .. } | ClassExpression::DataAllValuesFrom { .. }),
            CompletionRule::Unfold => matches!(concept, ClassExpression::Class(_)),
            CompletionRule::PropertyChain => false, // Applied based on axioms and edges, not concepts
            CompletionRule::Guess => false, // Applied by strategy
        }
    }

    /// Apply a rule to a given application
    pub fn apply_rule(&self, application: RuleApplication) -> Result<RuleResult> {
        match application.rule {
            CompletionRule::And => self.apply_and_rule(&application),
            CompletionRule::Or => self.apply_or_rule(&application),
            CompletionRule::Some => self.apply_some_rule(&application),
            CompletionRule::All => self.apply_all_rule(&application),
            CompletionRule::AtLeast => self.apply_at_least_rule(&application),
            CompletionRule::AtMost => self.apply_at_most_rule(&application),
            CompletionRule::Nominal => self.apply_nominal_rule(&application),
            CompletionRule::Self_ => self.apply_self_rule(&application),
            CompletionRule::Choose => self.apply_choose_rule(&application),
            CompletionRule::Datatype => self.apply_datatype_rule(&application),
            CompletionRule::Unfold => self.apply_unfold_rule(&application),
            CompletionRule::PropertyChain => self.apply_property_chain_rule(&application),
            CompletionRule::Guess => self.apply_guess_rule(&application),
        }
    }

    /// Apply the conjunction rule: A ⊓ B → A, B
    fn apply_and_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            // Extract conjuncts from intersection
            if let ClassExpression::ObjectIntersectionOf(conjuncts) = concept {
                for conjunct in conjuncts {
                    // Add each conjunct to the same individual
                    result.concept_additions.push((
                        string_to_individual(application.node.clone()),
                        conjunct.clone(),
                        dependencies.clone(),
                    ));
                }
            }
        }
        
        Ok(result)
    }

    /// Apply the disjunction rule: A ⊔ B → A | B (creates branching)
    fn apply_or_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            // Extract disjuncts from union
            if let ClassExpression::ObjectUnionOf(disjuncts) = concept {
                // Create branching choices for each disjunct
                let mut choices = Vec::new();
                for (index, disjunct) in disjuncts.iter().enumerate() {
                    let individual = string_to_individual(application.node.clone());
                    choices.push(crate::core::hypertableau::branching::BranchingChoice::new(
                        index,
                        format!("Disjunct {index}: {disjunct}"),
                        disjunct.clone(),
                        individual,
                    ));
                }
                
                // Create branching point
                let individual = string_to_individual(application.node.clone());
                let branching_type = crate::core::hypertableau::branching::BranchingType::GroundDisjunction {
                    disjunction: crate::core::hypertableau::ground_disjunction::GroundDisjunction::from_class_expression(
                        concept.clone(),
                        individual.clone(),
                        dependencies.clone(),
                    )?,
                    individual: individual.clone(),
                };
                
                result.branching_points.push((branching_type, choices));
            }
        }
        
        Ok(result)
    }

    /// Apply the existential rule: ∃R.C → create new individual with R-edge and C
    fn apply_some_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            // Extract role and filler from existential restriction
            if let ClassExpression::ObjectSomeValuesFrom { property, filler } = concept {
                // Create a new individual as witness
                let witness_individual = Individual::fresh();
                
                // Add role assertion between current individual and witness
                result.role_additions.push((
                    string_to_individual(application.node.clone()),
                    witness_individual.clone(),
                    property.clone(),
                    dependencies.clone(),
                ));
                
                // Add filler concept to the witness individual
                result.concept_additions.push((
                    witness_individual,
                    (**filler).clone(),
                    dependencies.clone(),
                ));
            }
        }
        
        Ok(result)
    }

    /// Apply the universal rule: ∀R.C with R-edge to y → C on y
    fn apply_all_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Role { role: _, source: _, target, concept } = &application.context {
            // Add the concept to the target node
            result.concept_additions.push((
                string_to_individual(target.clone()),
                concept.clone(),
                application.dependencies.clone(),
            ));
        }
        
        Ok(result)
    }

    /// Apply the at-least cardinality rule
    fn apply_at_least_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Cardinality { cardinality, role, filler, existing_successors } = &application.context {
            let needed = *cardinality as usize;
            let existing = existing_successors.len();
            
            if existing < needed {
                // Create additional successors
                for i in existing..needed {
                    let new_individual = format!("_card_{}_{}", application.node, i);
                    
                    result.new_individuals.push((new_individual.clone(), application.dependencies.clone()));
                    let object_property = match role {
                        Role::ObjectProperty(obj_prop) => obj_prop.clone(),
                        Role::DataProperty(_) => {
                            return Err(crate::Error::reasoning("Cannot use data property in object property context"));
                        }
                    };
                    
                    result.edge_additions.push((
                        application.node.clone(),
                        new_individual.clone(),
                        object_property,
                        application.dependencies.clone(),
                    ));
                    
                    if let Some(filler_concept) = filler {
                        result.concept_additions.push((
                            string_to_individual(new_individual),
                            filler_concept.clone(),
                            application.dependencies.clone(),
                        ));
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Apply the at-most cardinality rule
    fn apply_at_most_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Cardinality { cardinality, role: _, filler: _, existing_successors } = &application.context {
            let allowed = *cardinality as usize;
            let existing = existing_successors.len();
            
            if existing > allowed {
                // Need to merge some successors or detect clash
                // For simplicity, we'll create a merge for the first excess nodes
                for i in allowed..existing {
                    result.merges.push((
                        existing_successors[i].clone(),
                        existing_successors[allowed - 1].clone(),
                        application.dependencies.clone(),
                    ));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Apply the nominal rule
    fn apply_nominal_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Nominal { nominal, current_node } = &application.context {
            // Merge current node with the nominal individual
            result.merges.push((
                current_node.clone(),
                nominal.iri().map(|iri| iri.to_string()).unwrap_or_else(|| "unknown".to_string()),
                application.dependencies.clone(),
            ));
        }
        
        Ok(result)
    }
    
    /// Apply the self rule: ∀R.Self → R(x,x)
    fn apply_self_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            if let ClassExpression::ObjectHasSelf { property } = concept {
                // Add a self-edge
                result.edge_additions.push((
                    application.node.clone(),
                    application.node.clone(),
                    property.clone(),
                    dependencies.clone(),
                ));
            }
        }
        
        Ok(result)
    }

    /// Apply the choose rule (cardinality reasoning)
    fn apply_choose_rule(&self, _application: &RuleApplication) -> Result<RuleResult> {
        // This is a complex rule for cardinality reasoning
        // Would handle non-deterministic choices for cardinality
        // For now, we will return an empty result
        Ok(RuleResult::empty())
    }
    
    /// Apply datatype rules
    fn apply_datatype_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        // Handle datatype constraints and value spaces
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            match concept {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    // Create a witness data value for the existential
                    let witness_value = format!("_witness_value_{}", self.get_fresh_id());
                    
                    // Add data property assertion
                    result.data_assertions.push((
                        string_to_individual(application.node.clone()),
                        witness_value.clone(),
                        property.clone(),
                        dependencies.clone(),
                    ));
                    
                    // Add datatype constraint
                    result.datatype_constraints.push((
                        witness_value,
                        filler.clone(),
                        dependencies.clone(),
                    ));
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    // For all data property values, ensure they satisfy the constraint
                    // This would typically be handled by checking existing data assertions
                    // and validating them against the datatype constraint
                    
                    // For now, just record the constraint for later validation
                    result.universal_constraints.push((
                        string_to_individual(application.node.clone()),
                        property.clone(),
                        ClassExpression::DataAllValuesFrom { property: property.clone(), filler: filler.clone() },
                        dependencies.clone(),
                    ));
                }
                ClassExpression::DataHasValue { property, value } => {
                    // Add specific data property assertion
                    result.data_assertions.push((
                        string_to_individual(application.node.clone()),
                        value.to_string(), // Convert literal to string
                        property.clone(),
                        dependencies.clone(),
                    ));
                }
                _ => {
                    // Not a datatype-related concept
                }
            }
        }
        
        Ok(result)
    }
    
    /// Apply concept unfolding
    fn apply_unfold_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        // Unfold concept definitions from TBox
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            // Look for equivalent class axioms that define this concept
            if let ClassExpression::Class(named_class) = concept {
                // Check if we have a definition for this class
                if let Some(definition) = self.get_concept_definition(named_class) {
                    // Add the definition as a new concept assertion
                    result.concept_additions.push((
                        string_to_individual(application.node.clone()),
                        definition,
                        dependencies.clone(),
                    ));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Apply guess rule for generating individuals
    fn apply_guess_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        // Generate individuals for functionality/cardinality reasoning
        if let RuleContext::Concept { concept, dependencies } = &application.context {
            match concept {
                ClassExpression::ObjectMinCardinality { property, cardinality, filler } => {
                    // Generate at least n distinct individuals
                    for i in 0..*cardinality {
                        let witness = Individual::fresh();
                        
                        // Add role assertion to witness
                        result.role_additions.push((
                            string_to_individual(application.node.clone()),
                            witness.clone(),
                            property.clone(),
                            dependencies.clone(),
                        ));
                        
                        // Add filler concept to witness
                        result.concept_additions.push((
                            witness,
                            (**filler).clone(),
                            dependencies.clone(),
                        ));
                        
                        // Add inequality constraints between witnesses if needed
                        if i > 0 {
                            // This would ensure the witnesses are distinct
                            // Implementation would depend on how inequalities are handled
                        }
                    }
                }
                ClassExpression::ObjectMaxCardinality { property, cardinality, filler } => {
                    // For max cardinality, we need to ensure no more than n distinct individuals
                    // This is typically handled by clash detection rather than generation
                    
                    // Add constraint for later validation
                    result.cardinality_constraints.push((
                        string_to_individual(application.node.clone()),
                        property.clone(),
                        *cardinality,
                        (**filler).clone(),
                        false, // false = max cardinality
                        dependencies.clone(),
                    ));
                }
                ClassExpression::ObjectExactCardinality { property, cardinality, filler } => {
                    // Combine min and max cardinality
                    
                    // Generate exactly n individuals (min part)
                    for _i in 0..*cardinality {
                        let witness = Individual::fresh();
                        
                        result.role_additions.push((
                            string_to_individual(application.node.clone()),
                            witness.clone(),
                            property.clone(),
                            dependencies.clone(),
                        ));
                        
                        result.concept_additions.push((
                            witness,
                            (**filler).clone(),
                            dependencies.clone(),
                        ));
                    }
                    
                    // Add max constraint (max part)
                    result.cardinality_constraints.push((
                        string_to_individual(application.node.clone()),
                        property.clone(),
                        *cardinality,
                        (**filler).clone(),
                        false, // max cardinality constraint
                        dependencies.clone(),
                    ));
                }
                _ => {
                    // Not a cardinality-related concept
                }
            }
        }
        
        Ok(result)
    }

    /// Apply property chain rule: R1 ∘ R2 ∘ ... ∘ Rn ⊑ S
    /// If we have edges a -R1-> b -R2-> c ... z -Rn-> w, then infer a -S-> w
    fn apply_property_chain_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::PropertyChain { chain: _, super_property, source, target } = &application.context {
            // Add the super property edge from start to end of the chain
            result.edge_additions.push((
                source.clone(),
                target.clone(),
                super_property.clone(),
                application.dependencies.clone(),
            ));
        }
        
        Ok(result)
    }
    
    /// Get all rules in priority order
    pub fn rules_by_priority(&self) -> Vec<CompletionRule> {
        let mut rules = self.rules.clone();
        rules.sort_by_key(|r| self.get_priority(*r));
        rules
    }
    
    /// Check if any rules are applicable to a set of concepts
    pub fn has_applicable_rules(&self, concepts: &[ClassExpression]) -> bool {
        concepts.iter().any(|concept| {
            self.rules.iter().any(|&rule| self.is_rule_applicable(rule, concept))
        })
    }
    
    /// Get a fresh ID for witness generation
    fn get_fresh_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Get concept definition for a named class
    fn get_concept_definition(&self, named_class: &crate::ontology::Class) -> Option<ClassExpression> {
        // Simple implementation: look for equivalent class axioms in the ontology
        // In a full implementation, this would be optimized with indexing
        
        // For now, we'll check if there are any equivalent class axioms
        // that define this class in terms of other expressions
        
        // Placeholder: return a simple equivalent definition if it's a common pattern
        let class_name = &named_class.iri.to_string();
        
        // Example: if class is "Person", might be equivalent to "Human"
        if class_name.contains("Person") {
            Some(ClassExpression::Class(crate::ontology::Class {
                iri: crate::ontology::IRI::from("Human".to_string()),
            }))
        } else {
            None
        }
    }
}

impl RuleResult {
    /// Create an empty rule result
    pub fn empty() -> Self {
        Self {
            new_applications: Vec::new(),
            concept_additions: Vec::new(),
            role_additions: Vec::new(),
            edge_additions: Vec::new(),
            new_individuals: Vec::new(),
            merges: Vec::new(),
            clashes: Vec::new(),
            branches: Vec::new(),
            branching_points: Vec::new(),
            data_assertions: Vec::new(),
            datatype_constraints: Vec::new(),
            universal_constraints: Vec::new(),
            cardinality_constraints: Vec::new(),
        }
    }

    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.new_applications.is_empty() &&
        self.concept_additions.is_empty() &&
        self.role_additions.is_empty() &&
        self.edge_additions.is_empty() &&
        self.new_individuals.is_empty() &&
        self.merges.is_empty() &&
        self.clashes.is_empty() &&
        self.branches.is_empty() &&
        self.branching_points.is_empty() &&
        self.data_assertions.is_empty() &&
        self.datatype_constraints.is_empty() &&
        self.universal_constraints.is_empty() &&
        self.cardinality_constraints.is_empty()
    }

    /// Check if any clashes were detected
    pub fn has_clash(&self) -> bool {
        !self.clashes.is_empty()
    }

    /// Check if any branches were created
    pub fn requires_branching(&self) -> bool {
        !self.branches.is_empty()
    }

    /// Merge with another rule result
    pub fn merge(&mut self, other: RuleResult) {
        self.new_applications.extend(other.new_applications);
        self.concept_additions.extend(other.concept_additions);
        self.edge_additions.extend(other.edge_additions);
        self.new_individuals.extend(other.new_individuals);
        self.merges.extend(other.merges);
        self.clashes.extend(other.clashes);
        self.branches.extend(other.branches);
    }
}

impl RuleApplication {
    /// Create a new rule application
    pub fn new(
        rule: CompletionRule,
        node: String,
        context: RuleContext,
        priority: RulePriority,
        dependencies: DependencySet,
    ) -> Self {
        Self {
            rule,
            node,
            context,
            priority,
            dependencies,
        }
    }

    /// Create a concept-based rule application
    pub fn concept(
        rule: CompletionRule,
        node: String,
        concept: ClassExpression,
        dependencies: DependencySet,
    ) -> Self {
        let priority = match rule {
            CompletionRule::And |
                CompletionRule::All | 
                CompletionRule::Self_ => RulePriority::Highest,
            CompletionRule::Some |
                CompletionRule::Nominal => RulePriority::Normal,
            CompletionRule::Or |
                CompletionRule::AtLeast |
                CompletionRule::AtMost => RulePriority::Low,
            _ => RulePriority::Normal,
        };

        Self::new(
            rule,
            node,
            RuleContext::Concept {
                concept,
                dependencies: dependencies.clone(),
            },
            priority,
            dependencies,
        )
    }

    /// Create a role-based rule application
    pub fn role(
        rule: CompletionRule,
        role: Role,
        source: String,
        target: String,
        concept: ClassExpression,
        dependencies: DependencySet,
    ) -> Self {
        Self::new(
            rule,
            source.clone(),
            RuleContext::Role { role, source, target, concept },
            RulePriority::High,
            dependencies,
        )
    }

    /// Create a property chain rule application
    pub fn property_chain(
        chain: Vec<ObjectPropertyExpression>,
        target: String,
        source: String,
        super_property: ObjectPropertyExpression,
        dependencies: DependencySet,
    ) -> Self {
        Self::new(
            CompletionRule::PropertyChain,
            source.clone(),
            RuleContext::PropertyChain {
                chain,
                target,
                source,
                super_property,
            },
            RulePriority::High,
            dependencies,
        )
    }
}

impl fmt::Display for CompletionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompletionRule::And => write!(f, "And"),
            CompletionRule::Or => write!(f, "Or"),
            CompletionRule::Some => write!(f, "Some"),
            CompletionRule::All => write!(f, "All"),
            CompletionRule::AtLeast => write!(f, "AtLeast"),
            CompletionRule::AtMost => write!(f, "AtMost"),
            CompletionRule::Nominal => write!(f, "Nominal"),
            CompletionRule::Self_ => write!(f, "Self"),
            CompletionRule::Choose => write!(f, "Choose"),
            CompletionRule::Datatype => write!(f, "Data"),
            CompletionRule::Unfold => write!(f, "Unfold"),
            CompletionRule::PropertyChain => write!(f, "Chain"),
            CompletionRule::Guess => write!(f, "Guess"),
        }
    }
}

impl fmt::Display for ClashType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClashType::Contradiction => write!(f, "Contradiction"),
            ClashType::Cardinality => write!(f, "Cardinality"),
            ClashType::Datatype => write!(f, "Datatype"),
            ClashType::Functionality => write!(f, "Functionality"),
            ClashType::Distinctness => write!(f, "Distinctness"),
            ClashType::Nominal => write!(f, "Nominal"),
        }
    }
}

impl Default for CompletionRuleSet {
    fn default() -> Self {
        Self::new()
    }
}