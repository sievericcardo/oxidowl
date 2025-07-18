//! Completion rule system for tableau expansion
//!
//! This module implements the core completion rules for SROIQV(D) tableau
//! reasoning, based on the rule systems from Konclude, HermiT, and Pellet.

use crate::{
    core::dependency::{DependencySet, DependencyTracker, DependencyType},
    ontology::{ClassExpression, Individual, Role, DataProperty, ObjectPropert            CompletionRule::Some => self.apply_some_rule(&application),
            CompletionRule::All => self.apply_all_rule(&application),
            CompletionRule::AtLeast => self.apply_at_least_rule(&application),
            CompletionRule::AtMost => self.apply_at_most_rule(&application),
            CompletionRule::Nominal => self.apply_nominal_rule(&application),
            CompletionRule::Self_ => self.apply_self_rule(&application),
            CompletionRule::Choose => self.apply_choose_rule(&application),
            CompletionRule::Datatype => self.apply_datatype_rule(&application),
            CompletionRule::Unfold => self.apply_unfold_rule(&application),
            CompletionRule::PropertyChain => self.apply_property_chain_rule(&application),
            CompletionRule::Guess => self.apply_guess_rule(&application),},
    Error, Result
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
};

/// Completion rule types for tableau expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionRule {
    /// Conjunction rule: A ⊓ B → A, B
    And,
    
    /// Disjunction rule: A ⊔ B → A | B (creates branching)
    Or,
    
    /// Existential rule: ∃R.C → create new individual with R-edge and C
    Some,
    
    /// Universal rule: ∀R.C with R-edge to y → C on y
    All,
    
    /// At-least cardinality: ≥n R.C → create at least n R-successors with C
    AtLeast,
    
    /// At-most cardinality: ≤n R.C → merge or block excess successors
    AtMost,
    
    /// Nominal rule: {a} → merge with individual a
    Nominal,
    
    /// Self rule: ∀R.Self → R(x,x)
    Self_,
    
    /// Choose rule: handle non-deterministic cardinality choices
    Choose,
    
    /// Datatype rule: handle datatype restrictions
    Datatype,
    
    /// Unfolding rule: unfold concept definitions
    Unfold,
    
    /// Property chain rule: R1 ∘ R2 ∘ ... ∘ Rn ⊑ S → propagate S edges
    PropertyChain,
    
    /// Guess rule: generate individuals for functionality/cardinality
    Guess,
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
    pub concept_additions: Vec<(String, ClassExpression, DependencySet)>,

    /// Edge additions required
    pub edge_additions: Vec<(String, String, ObjectPropertyExpression, DependencySet)>,

    /// New individuals created
    pub new_individuals: Vec<(String, DependencySet)>,

    /// Merges to perform
    pub merges: Vec<(String, String, DependencySet)>,

    /// Clashes detected
    pub clashes: Vec<ClashInfo>,

    /// Branching points created
    pub branches: Vec<BranchInfo>,
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
            .filter(|&&rule| self.is_rule_applicable(*rule, concept))
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
            CompletionRule::And => self.apply_and_rule(application),
            CompletionRule::Or => self.apply_or_rule(application),
            CompletionRule::Some => self.apply_some_rule(application),
            CompletionRule::All => self.apply_all_rule(application),
            CompletionRule::AtLeast => self.apply_at_least_rule(application),
            CompletionRule::AtMost => self.apply_at_most_rule(application),
            CompletionRule::Nominal => self.apply_nominal_rule(application),
            CompletionRule::Self_ => self.apply_self_rule(application),
            CompletionRule::Choose => self.apply_choose_rule(application),
            CompletionRule::Datatype => self.apply_datatype_rule(application),
            CompletionRule::Unfold => self.apply_unfold_rule(application),
            CompletionRule::PropertyChain => self.apply_property_chain_rule(application),
            CompletionRule::Guess => self.apply_guess_rule(application),
        }
    }

    /// Apply the conjunction rule: A ⊓ B → A, B
    fn apply_and_rule(&self, application: RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Concept { concept, dependencies } = application.context {
            if let ClassExpression::ObjectIntersectionOf(conjuncts) = concept {
                for operand in conjuncts {
                    result.concept_additions.push((
                        application.node.clone(),
                        operand,
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
            if let ClassExpression::ObjectUnionOf(operands) = concept {
                // Create a branching point
                result.branches.push(BranchInfo {
                    rule: CompletionRule::Or,
                    node: application.node.clone(),
                    choices: operands.clone(),
                    dependencies: dependencies.clone(),
                });
            }
        }

        Ok(result)
    }

    /// Apply the existential rule: ∃R.C → create new individual with R-edge and C
    fn apply_some_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Concept { concept, dependencies } = &application.context {
            if let ClassExpression::ObjectSomeValuesFrom { property, filler } = concept {
                // Generate a fresh individual name
                let uuid_str = uuid::Uuid::new_v4().to_string();
                let new_individual = format!("_gen_{}", &uuid_str[..8]);
                
                // Create the new individual
                result.new_individuals.push((new_individual.clone(), dependencies.clone()));
                
                // Add the edge
                result.edge_additions.push((
                    application.node.clone(),
                    new_individual.clone(),
                    property.clone(),
                    dependencies.clone(),
                ));
                
                // Add the filler concept to the new individual
                result.concept_additions.push((
                    new_individual,
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

        if let RuleContext::Role { role: _, source: _, target: _, concept } = &application.context {
            // Add the concept to the target node
            result.concept_additions.push((
                to_node.clone(),
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
                            new_individual,
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
                nominal.iri().to_string(),
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
    fn apply_datatype_rule(&self, _application: &RuleApplication) -> Result<RuleResult> {
        // TODO: Datatype reasoning implementation
        // Would handle datatype constraints and value spaces
        Ok(RuleResult::empty())
    }
    
    /// Apply concept unfolding
    fn apply_unfold_rule(&self, _application: &RuleApplication) -> Result<RuleResult> {
        // TODO: Unfold concept definitions from TBox
        // Would typically expand definitions into simpler forms
        Ok(RuleResult::empty())
    }
    
    /// Apply guess rule for generating individuals
    fn apply_guess_rule(&self, _application: &RuleApplication) -> Result<RuleResult> {
        // TODO: Generate individuals for functionality/cardinality reasoning
        Ok(RuleResult::empty())
    }

    /// Apply property chain rule: R1 ∘ R2 ∘ ... ∘ Rn ⊑ S
    /// If we have edges a -R1-> b -R2-> c ... z -Rn-> w, then infer a -S-> w
    fn apply_property_chain_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();
        
        if let RuleContext::PropertyChain { chain, super_property, source, target } = &application.context {
            // Add the super property edge from start to end of the chain
            result.edge_additions.push((
                from_node.clone(),
                to_node.clone(),
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
}

impl RuleResult {
    /// Create an empty rule result
    pub fn empty() -> Self {
        Self {
            new_applications: Vec::new(),
            concept_additions: Vec::new(),
            edge_additions: Vec::new(),
            new_individuals: Vec::new(),
            merges: Vec::new(),
            clashes: Vec::new(),
            branches: Vec::new(),
        }
    }

    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.new_applications.is_empty() &&
        self.concept_additions.is_empty() &&
        self.edge_additions.is_empty() &&
        self.new_individuals.is_empty() &&
        self.merges.is_empty() &&
        self.clashes.is_empty() &&
        self.branches.is_empty()
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