//! Completion rule system for tableau expansion
//!
//! This module implements the core completion rules for SROIQV(D) tableau
//! reasoning, including support for rule application, priority management,
//! and clash detection.

use crate::{
    Result,
    core::dependency::DependencySet,
    ontology::{ClassExpression, DataProperty, Individual, ObjectPropertyExpression, Role},
};
use std::{collections::HashMap, fmt, sync::Arc};

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

/// Strategy for applying completion rules
#[derive(Debug, Clone)]
pub struct CompletionStrategy {
    /// Rule priority mapping
    pub rule_priorities: HashMap<CompletionRule, RulePriority>,
    /// Enable/disable certain rules
    pub enabled_rules: HashMap<CompletionRule, bool>,
}

impl Default for CompletionStrategy {
    fn default() -> Self {
        let mut rule_priorities = HashMap::with_capacity(13);
        rule_priorities.insert(CompletionRule::And, RulePriority::High);
        rule_priorities.insert(CompletionRule::All, RulePriority::High);
        rule_priorities.insert(CompletionRule::Some, RulePriority::Normal);
        rule_priorities.insert(CompletionRule::Or, RulePriority::Low);
        rule_priorities.insert(CompletionRule::Choose, RulePriority::Low);

        let mut enabled_rules = HashMap::with_capacity(13);
        for rule in [
            CompletionRule::And,
            CompletionRule::Or,
            CompletionRule::Some,
            CompletionRule::All,
            CompletionRule::AtLeast,
            CompletionRule::AtMost,
            CompletionRule::Nominal,
            CompletionRule::Self_,
            CompletionRule::Choose,
            CompletionRule::Datatype,
            CompletionRule::Unfold,
            CompletionRule::PropertyChain,
            CompletionRule::Guess,
        ] {
            enabled_rules.insert(rule, true);
        }

        Self {
            rule_priorities,
            enabled_rules,
        }
    }
}

impl CompletionRule {
    // Removed redundant apply() method - use apply_rule() on CompletionRuleSet instead
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
    pub dependencies: Arc<DependencySet>,
}

/// Context specific to each rule type
#[derive(Debug, Clone)]
pub enum RuleContext {
    /// Context for concept-based rules (AND, OR, etc.)
    Concept {
        concept: ClassExpression,
        dependencies: Arc<DependencySet>,
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
    handlers: HashMap<
        CompletionRule,
        Box<dyn Fn(RuleApplication) -> Result<Vec<RuleApplication>> + Send + Sync>,
    >,
}

impl std::fmt::Debug for CompletionRuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionRuleSet")
            .field("rules", &self.rules)
            .field("priorities", &self.priorities)
            .field(
                "applicability",
                &self.applicability.keys().collect::<Vec<_>>(),
            )
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Rule application result
#[derive(Debug, Clone, Default)]
pub struct RuleResult {
    /// New rule applications generated by this rule
    pub new_applications: Vec<RuleApplication>,

    /// Concept additions required
    pub concept_additions: Vec<(Individual, ClassExpression, Arc<DependencySet>)>,

    /// Role additions required
    pub role_additions: Vec<(
        Individual,
        Individual,
        ObjectPropertyExpression,
        Arc<DependencySet>,
    )>,

    /// Edge additions required (legacy)
    pub edge_additions: Vec<(String, String, ObjectPropertyExpression, Arc<DependencySet>)>,

    /// New individuals created
    pub new_individuals: Vec<(String, Arc<DependencySet>)>,

    /// Merges to perform
    pub merges: Vec<(String, String, Arc<DependencySet>)>,

    /// Clashes detected
    pub clashes: Vec<ClashInfo>,

    /// Branching points created
    pub branches: Vec<BranchInfo>,

    /// Branching points for choice rules (simplified from HyperTableau)
    pub branching_points: Vec<(String, Vec<String>)>,

    /// Data property assertions
    pub data_assertions: Vec<(
        Individual,
        String,
        crate::ontology::DataPropertyExpression,
        Arc<DependencySet>,
    )>,

    /// Datatype constraints
    pub datatype_constraints: Vec<(String, crate::ontology::DataRange, Arc<DependencySet>)>,

    /// Universal constraints for validation
    pub universal_constraints: Vec<(
        Individual,
        crate::ontology::DataPropertyExpression,
        ClassExpression,
        Arc<DependencySet>,
    )>,

    /// Cardinality constraints for validation
    pub cardinality_constraints: Vec<(
        Individual,
        ObjectPropertyExpression,
        u32,
        ClassExpression,
        bool,
        Arc<DependencySet>,
    )>,
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
    pub dependencies: Arc<DependencySet>,

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
    pub dependencies: Arc<DependencySet>,
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
    #[must_use]
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
            self.rules
                .sort_by_key(|r| priorities.get(r).copied().unwrap_or(RulePriority::Normal));
        }
    }

    /// Get rule priority
    #[must_use]
    pub fn get_priority(&self, rule: CompletionRule) -> RulePriority {
        self.priorities
            .get(&rule)
            .copied()
            .unwrap_or(RulePriority::Normal)
    }

    /// Get all applicable rules
    #[must_use]
    pub fn get_applicable_rules(&self, concept: &ClassExpression) -> Vec<CompletionRule> {
        self.rules
            .iter()
            .filter(|&rule| self.is_rule_applicable(*rule, concept))
            .copied()
            .collect()
    }

    /// Check if a rule is applicable to a concept
    #[must_use]
    pub fn is_rule_applicable(&self, rule: CompletionRule, concept: &ClassExpression) -> bool {
        match rule {
            CompletionRule::And => matches!(concept, ClassExpression::ObjectIntersectionOf(_)),
            CompletionRule::Or => matches!(concept, ClassExpression::ObjectUnionOf(_)),
            CompletionRule::Some => matches!(concept, ClassExpression::ObjectSomeValuesFrom { .. }),
            CompletionRule::All => false, // Applied based on edges, not concepts
            CompletionRule::AtLeast => {
                matches!(concept, ClassExpression::ObjectMinCardinality { .. })
            }
            CompletionRule::AtMost => {
                matches!(concept, ClassExpression::ObjectMaxCardinality { .. })
            }
            CompletionRule::Nominal => matches!(concept, ClassExpression::ObjectOneOf(_)),
            CompletionRule::Self_ => matches!(concept, ClassExpression::ObjectHasSelf { .. }),
            CompletionRule::Choose => false, // Applied by strategy
            CompletionRule::Datatype => matches!(
                concept,
                ClassExpression::DataSomeValuesFrom { .. }
                    | ClassExpression::DataAllValuesFrom { .. }
            ),
            CompletionRule::Unfold => matches!(concept, ClassExpression::Class(_)),
            CompletionRule::PropertyChain => false, // Applied based on axioms and edges, not concepts
            CompletionRule::Guess => false,         // Applied by strategy
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

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            // Extract conjuncts from intersection
            if let ClassExpression::ObjectIntersectionOf(conjuncts) = concept {
                let individual = string_to_individual(application.node.clone());
                for conjunct in conjuncts {
                    // Add each conjunct to the same individual
                    result.concept_additions.push((
                        individual.clone(),
                        conjunct.clone(),
                        Arc::clone(dependencies),
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Apply the disjunction rule: A ⊔ B → A | B (creates branching)
    fn apply_or_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            // Extract disjuncts from union
            if let ClassExpression::ObjectUnionOf(disjuncts) = concept {
                // Create branching choices for each disjunct (simplified)
                let mut choices = Vec::with_capacity(disjuncts.len());
                for i in 0..disjuncts.len() {
                    choices.push(format!("Disjunct {i}"));
                }

                // Create simple branching point
                result.branching_points.push((String::from("GroundDisjunction"), choices));
            }
        }

        Ok(result)
    }

    /// Apply the existential rule: ∃R.C → create new individual with R-edge and C
    fn apply_some_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            // Extract role and filler from existential restriction
            if let ClassExpression::ObjectSomeValuesFrom { property, filler } = concept {
                // Create a new individual as witness
                let witness_individual = Individual::fresh();
                let source_individual = string_to_individual(application.node.clone());

                // Add role assertion between current individual and witness
                result.role_additions.push((
                    source_individual,
                    witness_individual.clone(),
                    property.clone(),
                    Arc::clone(dependencies),
                ));

                // Add filler concept to the witness individual
                result.concept_additions.push((
                    witness_individual,
                    (**filler).clone(),
                    Arc::clone(dependencies),
                ));
            }
        }

        Ok(result)
    }

    /// Apply the universal rule: ∀R.C with R-edge to y → C on y
    fn apply_all_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Role {
            role: _,
            source: _,
            target,
            concept,
        } = &application.context
        {
            // Add the concept to the target node
            result.concept_additions.push((
                string_to_individual(target.clone()),
                concept.clone(),
                Arc::clone(&application.dependencies),
            ));
        }

        Ok(result)
    }

    /// Apply the at-least cardinality rule
    fn apply_at_least_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Cardinality {
            cardinality,
            role,
            filler,
            existing_successors,
        } = &application.context
        {
            let needed = *cardinality as usize;
            let existing = existing_successors.len();

            if existing < needed {
                // Extract object property once before the loop
                let object_property = match role {
                    Role::ObjectProperty(obj_prop) => obj_prop.clone(),
                    Role::DataProperty(_) => {
                        return Err(crate::Error::reasoning(
                            "Cannot use data property in object property context",
                        ));
                    }
                };
                
                let deps = application.dependencies.clone();
                
                // Create additional successors
                for i in existing..needed {
                    let new_individual = format!("_card_{}_{}", application.node, i);

                    result
                        .new_individuals
                        .push((new_individual.clone(), Arc::clone(&deps)));

                    result.edge_additions.push((
                        application.node.clone(),
                        new_individual.clone(),
                        object_property.clone(),
                        Arc::clone(&deps),
                    ));

                    if let Some(filler_concept) = filler {
                        result.concept_additions.push((
                            string_to_individual(new_individual),
                            filler_concept.clone(),
                            Arc::clone(&deps),
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

        if let RuleContext::Cardinality {
            cardinality,
            role: _,
            filler: _,
            existing_successors,
        } = &application.context
        {
            let allowed = *cardinality as usize;
            let existing = existing_successors.len();

            if existing > allowed {
                // Need to merge some successors or detect clash
                // For simplicity, we'll create a merge for the first excess nodes
                let deps = Arc::clone(&application.dependencies);
                let target = existing_successors[allowed - 1].clone();
                
                for i in allowed..existing {
                    result.merges.push((
                        existing_successors[i].clone(),
                        target.clone(),
                        Arc::clone(&deps),
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Apply the nominal rule
    fn apply_nominal_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Nominal {
            nominal,
            current_node,
        } = &application.context
        {
            // Merge current node with the nominal individual
            result.merges.push((
                current_node.clone(),
                nominal
                    .iri()
                    .map(|iri| iri.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                Arc::clone(&application.dependencies),
            ));
        }

        Ok(result)
    }

    /// Apply the self rule: ∀R.Self → R(x,x)
    fn apply_self_rule(&self, application: &RuleApplication) -> Result<RuleResult> {
        let mut result = RuleResult::empty();

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            if let ClassExpression::ObjectHasSelf { property } = concept {
                // Add a self-edge
                result.edge_additions.push((
                    application.node.clone(),
                    application.node.clone(),
                    property.clone(),
                    Arc::clone(dependencies),
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
        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            let individual = string_to_individual(application.node.clone());
            
            match concept {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    // Create a witness data value for the existential
                    let witness_value = format!("_witness_value_{}", self.get_fresh_id());

                    // Add data property assertion
                    result.data_assertions.push((
                        individual,
                        witness_value.clone(),
                        property.clone(),
                        Arc::clone(dependencies),
                    ));

                    // Add datatype constraint
                    result.datatype_constraints.push((
                        witness_value,
                        filler.clone(),
                        Arc::clone(dependencies),
                    ));
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    // For all data property values, ensure they satisfy the constraint
                    // This would typically be handled by checking existing data assertions
                    // and validating them against the datatype constraint

                    // For now, just record the constraint for later validation
                    result.universal_constraints.push((
                        individual,
                        property.clone(),
                        ClassExpression::DataAllValuesFrom {
                            property: property.clone(),
                            filler: filler.clone(),
                        },
                        Arc::clone(dependencies),
                    ));
                }
                ClassExpression::DataHasValue { property, value } => {
                    // Add specific data property assertion
                    result.data_assertions.push((
                        individual,
                        value.value.clone(), // Use value directly
                        property.clone(),
                        Arc::clone(dependencies),
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
        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            // Look for equivalent class axioms that define this concept
            if let ClassExpression::Class(named_class) = concept {
                // Check if we have a definition for this class
                if let Some(definition) = self.get_concept_definition(named_class) {
                    // Add the definition as a new concept assertion
                    result.concept_additions.push((
                        string_to_individual(application.node.clone()),
                        definition,
                        Arc::clone(dependencies),
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
        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &application.context
        {
            let source_individual = string_to_individual(application.node.clone());
            
            match concept {
                ClassExpression::ObjectMinCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    // Generate at least n distinct individuals
                    for _i in 0..*cardinality {
                        let witness = Individual::fresh();

                        // Add role assertion to witness
                        result.role_additions.push((
                            source_individual.clone(),
                            witness.clone(),
                            property.clone(),
                            Arc::clone(dependencies),
                        ));

                        // Add filler concept to witness
                        result.concept_additions.push((
                            witness,
                            (**filler).clone(),
                            Arc::clone(dependencies),
                        ));

                        // Add inequality constraints between witnesses if needed
                        // (Implementation would depend on how inequalities are handled)
                    }
                }
                ClassExpression::ObjectMaxCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    // For max cardinality, we need to ensure no more than n distinct individuals
                    // This is typically handled by clash detection rather than generation

                    // Add constraint for later validation
                    result.cardinality_constraints.push((
                        source_individual,
                        property.clone(),
                        *cardinality,
                        (**filler).clone(),
                        false, // false = max cardinality
                        Arc::clone(dependencies),
                    ));
                }
                ClassExpression::ObjectExactCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    // Combine min and max cardinality

                    // Generate exactly n individuals (min part)
                    for _i in 0..*cardinality {
                        let witness = Individual::fresh();

                        result.role_additions.push((
                            source_individual.clone(),
                            witness.clone(),
                            property.clone(),
                            Arc::clone(dependencies),
                        ));

                        result.concept_additions.push((
                            witness,
                            (**filler).clone(),
                            Arc::clone(dependencies),
                        ));
                    }

                    // Add max constraint (max part)
                    result.cardinality_constraints.push((
                        source_individual,
                        property.clone(),
                        *cardinality,
                        (**filler).clone(),
                        false, // max cardinality constraint
                        Arc::clone(dependencies),
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

        if let RuleContext::PropertyChain {
            chain: _,
            super_property,
            source,
            target,
        } = &application.context
        {
            // Add the super property edge from start to end of the chain
            result.edge_additions.push((
                source.clone(),
                target.clone(),
                super_property.clone(),
                Arc::clone(&application.dependencies),
            ));
        }

        Ok(result)
    }

    /// Get all rules in priority order
    #[must_use]
    pub fn rules_by_priority(&self) -> Vec<CompletionRule> {
        let mut rules = self.rules.clone();
        rules.sort_by_key(|r| self.get_priority(*r));
        rules
    }

    /// Check if any rules are applicable to a set of concepts
    #[must_use]
    pub fn has_applicable_rules(&self, concepts: &[ClassExpression]) -> bool {
        concepts.iter().any(|concept| {
            self.rules
                .iter()
                .any(|&rule| self.is_rule_applicable(rule, concept))
        })
    }

    /// Get a fresh ID for witness generation
    fn get_fresh_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Get concept definition for a named class
    fn get_concept_definition(
        &self,
        named_class: &crate::ontology::Class,
    ) -> Option<ClassExpression> {
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
    /// Create an empty rule result (uses Default)
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a rule result with pre-allocated capacity
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            new_applications: Vec::with_capacity(capacity),
            concept_additions: Vec::with_capacity(capacity),
            role_additions: Vec::with_capacity(capacity),
            edge_additions: Vec::with_capacity(capacity),
            new_individuals: Vec::with_capacity(capacity),
            merges: Vec::with_capacity(capacity),
            clashes: Vec::with_capacity(capacity / 4),  // Clashes are less common
            branches: Vec::with_capacity(capacity / 4),  // Branches are less common
            branching_points: Vec::with_capacity(capacity / 4),
            data_assertions: Vec::with_capacity(capacity / 2),
            datatype_constraints: Vec::with_capacity(capacity / 2),
            universal_constraints: Vec::with_capacity(capacity / 2),
            cardinality_constraints: Vec::with_capacity(capacity / 2),
        }
    }

    /// Check if the result is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.new_applications.is_empty()
            && self.concept_additions.is_empty()
            && self.role_additions.is_empty()
            && self.edge_additions.is_empty()
            && self.new_individuals.is_empty()
            && self.merges.is_empty()
            && self.clashes.is_empty()
            && self.branches.is_empty()
            && self.branching_points.is_empty()
            && self.data_assertions.is_empty()
            && self.datatype_constraints.is_empty()
            && self.universal_constraints.is_empty()
            && self.cardinality_constraints.is_empty()
    }

    /// Check if any clashes were detected
    #[must_use]
    pub fn has_clash(&self) -> bool {
        !self.clashes.is_empty()
    }

    /// Check if any branches were created
    #[must_use]
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
    #[must_use]
    pub fn new(
        rule: CompletionRule,
        node: String,
        context: RuleContext,
        priority: RulePriority,
        dependencies: Arc<DependencySet>,
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
    #[must_use]
    pub fn concept(
        rule: CompletionRule,
        node: String,
        concept: ClassExpression,
        dependencies: Arc<DependencySet>,
    ) -> Self {
        let priority = match rule {
            CompletionRule::And | CompletionRule::All | CompletionRule::Self_ => {
                RulePriority::Highest
            }
            CompletionRule::Some | CompletionRule::Nominal => RulePriority::Normal,
            CompletionRule::Or | CompletionRule::AtLeast | CompletionRule::AtMost => {
                RulePriority::Low
            }
            _ => RulePriority::Normal,
        };

        Self::new(
            rule,
            node,
            RuleContext::Concept {
                concept,
                dependencies: Arc::clone(&dependencies),
            },
            priority,
            dependencies,
        )
    }

    /// Create a role-based rule application
    #[must_use]
    pub fn role(
        rule: CompletionRule,
        role: Role,
        source: String,
        target: String,
        concept: ClassExpression,
        dependencies: Arc<DependencySet>,
    ) -> Self {
        Self::new(
            rule,
            source.clone(),
            RuleContext::Role {
                role,
                source,
                target,
                concept,
            },
            RulePriority::High,
            dependencies,
        )
    }

    /// Create a property chain rule application
    #[must_use]
    pub fn property_chain(
        chain: Vec<ObjectPropertyExpression>,
        target: String,
        source: String,
        super_property: ObjectPropertyExpression,
        dependencies: Arc<DependencySet>,
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
