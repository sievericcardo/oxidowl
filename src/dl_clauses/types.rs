//! Core types for DL clause generation and representation

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// A DL clause with head and body atoms
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DLClause {
    /// Head atoms (conclusions)
    pub head: Vec<DLAtom>,
    /// Body atoms (conditions)
    pub body: Vec<DLAtom>,
    /// Variables used in the clause
    pub variables: HashSet<String>,
    /// Clause identifier
    pub id: String,
}

/// An atomic formula in DL clauses
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DLAtom {
    /// Predicate name
    pub predicate: String,
    /// Arguments (variables or constants)
    pub arguments: Vec<String>,
    /// Whether this is a positive or negative atom
    pub is_positive: bool,
    /// Additional constraints or annotations
    pub constraints: Vec<String>,
}

/// Result of DL clause generation
#[derive(Debug, Clone)]
pub struct DLClauseSet {
    /// Deterministic DL clauses (Horn clauses)
    pub deterministic_clauses: Vec<DLClause>,
    /// Disjunctive DL clauses (multiple heads)
    pub disjunctive_clauses: Vec<DLClause>,
    /// ABox facts (ground assertions)
    pub abox_facts: Vec<DLAtom>,
    /// Prefixes used in the ontology
    pub prefixes: HashMap<String, String>,
    /// Statistics about the clause set
    pub statistics: DLClauseStatistics,
}

/// Statistics about DL clauses
#[derive(Debug, Clone, Default)]
pub struct DLClauseStatistics {
    pub deterministic_clause_count: usize,
    pub disjunctive_clause_count: usize,
    pub disjunction_count: usize,
    pub positive_fact_count: usize,
    pub negative_fact_count: usize,
}

impl DLAtom {
    /// Create a new positive atomic formula
    pub fn new(predicate: String, arguments: Vec<String>) -> Self {
        Self {
            predicate,
            arguments,
            is_positive: true,
            constraints: Vec::new(),
        }
    }

    /// Create a new negative atomic formula
    pub fn new_negative(predicate: String, arguments: Vec<String>) -> Self {
        Self {
            predicate,
            arguments,
            is_positive: false,
            constraints: Vec::new(),
        }
    }

    /// Create an atom with specified negation
    pub fn with_negation(mut self, negate: bool) -> Self {
        self.is_positive = !negate;
        self
    }

    /// Add a constraint to this atom
    pub fn with_constraint(mut self, constraint: String) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Create a concept assertion C(x)
    pub fn concept_assertion(concept: &str, individual: &str) -> Self {
        Self::new(concept.to_string(), vec![individual.to_string()])
    }

    /// Create a role assertion R(x, y)
    pub fn role_assertion(role: &str, subject: &str, object: &str) -> Self {
        Self::new(
            role.to_string(),
            vec![subject.to_string(), object.to_string()],
        )
    }

    /// Create a datatype property assertion P(x, v)
    pub fn datatype_assertion(property: &str, subject: &str, value: &str) -> Self {
        Self::new(
            property.to_string(),
            vec![subject.to_string(), value.to_string()],
        )
    }

    /// Create an atLeast cardinality atom - HermiT style
    pub fn at_least_cardinality(cardinality: u32, property: &str, range: &str, subject: &str) -> Self {
        Self::new(
            format!("atLeast({},{},{})", cardinality, property, range),
            vec![subject.to_string()],
        )
    }

    /// Create an atMost cardinality atom - HermiT style
    pub fn at_most_cardinality(cardinality: u32, property: &str, range: &str, subject: &str) -> Self {
        Self::new(
            format!("atMost({},{},{})", cardinality, property, range),
            vec![subject.to_string()],
        )
    }

    /// Create an equality constraint atom - HermiT style
    pub fn equality_constraint(var1: &str, var2: &str) -> Self {
        Self::new(
            format!("[{} == {}]", var1, var2),
            vec![],
        )
    }

    /// Create a datatype restriction atom - HermiT style
    pub fn datatype_restriction(datatype: &str, restrictions: &[String], variable: &str) -> Self {
        let restriction_str = if restrictions.is_empty() {
            datatype.to_string()
        } else {
            format!("{}[{}]", datatype, restrictions.join(","))
        };
        Self::new(restriction_str, vec![variable.to_string()])
    }

    /// Create a nominal atom - HermiT style
    pub fn nominal(value: &str, variable: &str) -> Self {
        Self::new(
            format!("{{{}}}", value),
            vec![variable.to_string()],
        )
    }
}

impl fmt::Display for DLAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.is_positive { "" } else { "not(" };
        let suffix = if self.is_positive { "" } else { ")" };

        let constraint_str = if self.constraints.is_empty() {
            String::new()
        } else {
            format!("@{}", self.constraints.join("@"))
        };

        if self.arguments.is_empty() {
            write!(f, "{prefix}{}{}{suffix}", self.predicate, constraint_str)
        } else if self.arguments.len() == 1 {
            write!(
                f,
                "{prefix}{}({}){}{suffix}",
                self.predicate, self.arguments[0], constraint_str
            )
        } else {
            write!(
                f,
                "{prefix}{}({}){}{suffix}",
                self.predicate,
                self.arguments.join(","),
                constraint_str
            )
        }
    }
}

impl DLClause {
    /// Create a new DL clause
    pub fn new(head: Vec<DLAtom>, body: Vec<DLAtom>, id: String) -> Self {
        let mut variables = HashSet::new();

        // Collect variables from head and body
        for atom in &head {
            for arg in &atom.arguments {
                if arg.chars().next().map_or(false, |c| c.is_uppercase()) {
                    variables.insert(arg.clone());
                }
            }
        }
        for atom in &body {
            for arg in &atom.arguments {
                if arg.chars().next().map_or(false, |c| c.is_uppercase()) {
                    variables.insert(arg.clone());
                }
            }
        }

        Self {
            head,
            body,
            variables,
            id,
        }
    }

    /// Check if this is a deterministic clause (at most one head atom)
    pub fn is_deterministic(&self) -> bool {
        self.head.len() <= 1
    }

    /// Check if this is a disjunctive clause (multiple head atoms)
    pub fn is_disjunctive(&self) -> bool {
        self.head.len() > 1
    }

    /// Check if this is a fact (no body atoms)
    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }

    /// Check if this is a constraint (no head atoms)
    pub fn is_constraint(&self) -> bool {
        self.head.is_empty()
    }
}

impl fmt::Display for DLClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.head.is_empty() {
            // Constraint (contradiction)
            write!(
                f,
                ": - {}",
                self.body
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else if self.head.len() == 1 {
            // Deterministic clause
            if self.body.is_empty() {
                // Fact
                write!(f, "{}", self.head[0])
            } else {
                // Rule
                write!(
                    f,
                    "{} :- {}",
                    self.head[0],
                    self.body
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        } else {
            // Disjunctive clause
            if self.body.is_empty() {
                // Disjunctive fact
                write!(
                    f,
                    "{}",
                    self.head
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(" v ")
                )
            } else {
                // Disjunctive rule
                write!(
                    f,
                    "{} :- {}",
                    self.head
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(" v "),
                    self.body
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl DLClauseSet {
    /// Create a new empty clause set
    pub fn new() -> Self {
        Self {
            deterministic_clauses: Vec::new(),
            disjunctive_clauses: Vec::new(),
            abox_facts: Vec::new(),
            prefixes: HashMap::new(),
            statistics: DLClauseStatistics::default(),
        }
    }

    /// Add a clause to the appropriate collection
    pub fn add_clause(&mut self, clause: DLClause) {
        if clause.is_disjunctive() {
            self.disjunctive_clauses.push(clause);
        } else {
            self.deterministic_clauses.push(clause);
        }
        self.update_statistics();
    }

    /// Add multiple clauses
    pub fn add_clauses(&mut self, clauses: Vec<DLClause>) {
        for clause in clauses {
            self.add_clause(clause);
        }
    }

    /// Add an ABox fact
    pub fn add_fact(&mut self, fact: DLAtom) {
        self.abox_facts.push(fact);
        self.update_statistics();
    }

    /// Add a prefix mapping
    pub fn add_prefix(&mut self, prefix: String, namespace: String) {
        self.prefixes.insert(prefix, namespace);
    }

    /// Update internal statistics
    pub fn update_statistics(&mut self) {
        self.statistics.deterministic_clause_count = self.deterministic_clauses.len();
        self.statistics.disjunctive_clause_count = self.disjunctive_clauses.len();
        self.statistics.disjunction_count = self
            .disjunctive_clauses
            .iter()
            .map(|c| c.head.len().saturating_sub(1))
            .sum();
        self.statistics.positive_fact_count =
            self.abox_facts.iter().filter(|f| f.is_positive).count();
        self.statistics.negative_fact_count =
            self.abox_facts.iter().filter(|f| !f.is_positive).count();
    }

    /// Get total number of clauses
    pub fn total_clauses(&self) -> usize {
        self.deterministic_clauses.len() + self.disjunctive_clauses.len()
    }
}

impl Default for DLClauseSet {
    fn default() -> Self {
        Self::new()
    }
}
