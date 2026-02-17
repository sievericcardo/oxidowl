//! Conjunctive query data structures and types
//!
//! This module defines the core types for representing and processing
//! conjunctive queries over OWL 2 DL ontologies.

use crate::ontology::{
    ClassExpression, DataPropertyExpression, IRI, Individual, Literal, ObjectPropertyExpression,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// A variable in a conjunctive query
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryVariable {
    pub name: String,
    pub var_type: VariableType,
}

/// Types of variables that can appear in queries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariableType {
    /// Individual variable (can be bound to named individuals)
    Individual,
    /// Class variable (can be bound to classes)
    Class,
    /// Object property variable
    ObjectProperty,
    /// Data property variable  
    DataProperty,
    /// Literal variable (for data values)
    Literal,
}

/// An atom in a conjunctive query
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryAtom {
    /// Class atom: C(x) where C is a class expression and x is an individual variable
    ClassAtom {
        variable: QueryVariable,
        class_expression: ClassExpression,
    },
    /// Object property atom: R(x, y) where R is an object property and x, y are individual variables
    ObjectPropertyAtom {
        subject: QueryVariable,
        property: ObjectPropertyExpression,
        object: QueryVariable,
    },
    /// Data property atom: P(x, v) where P is a data property, x is individual, v is literal
    DataPropertyAtom {
        subject: QueryVariable,
        property: DataPropertyExpression,
        literal: QueryVariable,
    },
    /// Same individual atom: x = y
    SameIndividualAtom {
        left: QueryVariable,
        right: QueryVariable,
    },
    /// Different individuals atom: x ≠ y  
    DifferentIndividualsAtom {
        left: QueryVariable,
        right: QueryVariable,
    },
    /// Concrete individual atom: x = `individual_iri`
    ConcreteIndividualAtom {
        variable: QueryVariable,
        individual: Individual,
    },
    /// Concrete literal atom: v = `literal_value`
    ConcreteLiteralAtom {
        variable: QueryVariable,
        literal: Literal,
    },
}

/// A conjunctive query
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConjunctiveQuery {
    /// Variables appearing in the query head (answer variables)
    pub answer_variables: Vec<QueryVariable>,
    /// Atoms in the query body
    pub body_atoms: Vec<QueryAtom>,
    /// Variable bindings and constraints
    pub constraints: QueryConstraints,
    /// Query metadata
    pub metadata: QueryMetadata,
}

/// Constraints and filters applied to query variables
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryConstraints {
    /// Variables that must be distinct (stored as Vec of pairs for Hash compatibility)
    pub distinct_variables: Vec<Vec<QueryVariable>>,
    /// Type constraints for variables (stored as Vec of pairs for Hash compatibility)
    pub type_constraints: Vec<(QueryVariable, Vec<ClassExpression>)>,
    /// Value range constraints for literal variables (stored as Vec of pairs for Hash compatibility)
    pub value_constraints: Vec<(QueryVariable, ValueConstraint)>,
}

/// Constraints on literal values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueConstraint {
    /// Exact value match
    ExactValue(Literal),
    /// Value in a set of allowed values
    ValueSet(Vec<Literal>),
    /// Numeric range constraint (using i64 for Hash compatibility)
    NumericRange { min: Option<i64>, max: Option<i64> },
    /// String pattern constraint (regex)
    StringPattern(String),
    /// Datatype constraint
    DatatypeConstraint(IRI),
}

/// Query metadata and execution hints
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query name or identifier
    pub name: Option<String>,
    /// Expected result size hint
    pub expected_result_size: Option<usize>,
    /// Optimization preferences
    pub optimization_hints: OptimizationHints,
    /// Query provenance information
    pub source: Option<String>,
}

/// Hints for query optimization
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptimizationHints {
    /// Prefer specific optimization strategy
    pub strategy: Option<OptimizationStrategy>,
    /// Maximum time allowed for query execution (ms)
    pub timeout: Option<u64>,
    /// Use cached results if available
    pub use_cache: bool,
    /// Prefer approximation over exact results
    pub allow_approximation: bool,
}

/// Query optimization strategies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// Standard tableau-based reasoning
    Tableau,
    /// OWL 2 QL query rewriting
    QLRewriting,
    /// Hybrid approach combining multiple strategies
    Hybrid,
    /// Custom strategy with specific parameters (stored as Vec of pairs for Hash compatibility)
    Custom(Vec<(String, String)>),
}

impl QueryVariable {
    /// Create a new variable with default Individual type (for backward compatibility)
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::Individual,
        }
    }

    /// Create a new individual variable
    pub fn individual(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::Individual,
        }
    }

    /// Create a new class variable
    pub fn class(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::Class,
        }
    }

    /// Create a new object property variable
    pub fn object_property(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::ObjectProperty,
        }
    }

    /// Create a new data property variable
    pub fn data_property(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::DataProperty,
        }
    }

    /// Create a new literal variable
    pub fn literal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::Literal,
        }
    }

    /// Check if this variable can be bound to individuals
    #[must_use]
    pub fn is_individual(&self) -> bool {
        matches!(self.var_type, VariableType::Individual)
    }

    /// Check if this variable can be bound to classes
    #[must_use]
    pub fn is_class(&self) -> bool {
        matches!(self.var_type, VariableType::Class)
    }

    /// Check if this variable can be bound to literals
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self.var_type, VariableType::Literal)
    }
}

impl ConjunctiveQuery {
    /// Create a new empty conjunctive query
    #[must_use]
    pub fn new() -> Self {
        Self {
            answer_variables: Vec::new(),
            body_atoms: Vec::new(),
            constraints: QueryConstraints::default(),
            metadata: QueryMetadata::default(),
        }
    }

    /// Add an answer variable to the query
    pub fn add_answer_variable(&mut self, variable: QueryVariable) -> &mut Self {
        self.answer_variables.push(variable);
        self
    }

    /// Add a body atom to the query
    pub fn add_body_atom(&mut self, atom: QueryAtom) -> &mut Self {
        self.body_atoms.push(atom);
        self
    }

    /// Get all variables appearing in the query
    #[must_use]
    pub fn get_all_variables(&self) -> HashSet<QueryVariable> {
        let mut variables = HashSet::new();

        // Add answer variables
        variables.extend(self.answer_variables.iter().cloned());

        // Add variables from body atoms
        for atom in &self.body_atoms {
            match atom {
                QueryAtom::ClassAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
                QueryAtom::ObjectPropertyAtom {
                    subject, object, ..
                } => {
                    variables.insert(subject.clone());
                    variables.insert(object.clone());
                }
                QueryAtom::DataPropertyAtom {
                    subject, literal, ..
                } => {
                    variables.insert(subject.clone());
                    variables.insert(literal.clone());
                }
                QueryAtom::SameIndividualAtom { left, right }
                | QueryAtom::DifferentIndividualsAtom { left, right } => {
                    variables.insert(left.clone());
                    variables.insert(right.clone());
                }
                QueryAtom::ConcreteIndividualAtom { variable, .. }
                | QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
            }
        }

        variables
    }

    /// Check if the query is well-formed
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        // Check that all answer variables appear in the body
        let body_variables = self.get_body_variables();
        self.answer_variables
            .iter()
            .all(|var| body_variables.contains(var))
    }

    /// Get variables that appear only in the query body
    #[must_use]
    pub fn get_body_variables(&self) -> HashSet<QueryVariable> {
        let mut variables = HashSet::new();

        for atom in &self.body_atoms {
            match atom {
                QueryAtom::ClassAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
                QueryAtom::ObjectPropertyAtom {
                    subject, object, ..
                } => {
                    variables.insert(subject.clone());
                    variables.insert(object.clone());
                }
                QueryAtom::DataPropertyAtom {
                    subject, literal, ..
                } => {
                    variables.insert(subject.clone());
                    variables.insert(literal.clone());
                }
                QueryAtom::SameIndividualAtom { left, right }
                | QueryAtom::DifferentIndividualsAtom { left, right } => {
                    variables.insert(left.clone());
                    variables.insert(right.clone());
                }
                QueryAtom::ConcreteIndividualAtom { variable, .. }
                | QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
            }
        }

        variables
    }

    /// Get the complexity score of this query (higher = more complex)
    #[must_use]
    pub fn complexity_score(&self) -> u32 {
        let mut score = 0;

        // Base score from number of atoms
        score += self.body_atoms.len() as u32;

        // Additional score for complex class expressions
        for atom in &self.body_atoms {
            if let QueryAtom::ClassAtom {
                class_expression, ..
            } = atom
            {
                score += Self::class_expression_complexity(class_expression);
            }
        }

        // Score for constraints
        score += self.constraints.distinct_variables.len() as u32;
        score += self.constraints.type_constraints.len() as u32;
        score += self.constraints.value_constraints.len() as u32;

        score
    }

    fn class_expression_complexity(expr: &ClassExpression) -> u32 {
        match expr {
            ClassExpression::Class(_) => 1,
            ClassExpression::ObjectIntersectionOf(exprs)
            | ClassExpression::ObjectUnionOf(exprs) => {
                1 + exprs
                    .iter()
                    .map(Self::class_expression_complexity)
                    .sum::<u32>()
            }
            ClassExpression::ObjectComplementOf(expr) => {
                2 + Self::class_expression_complexity(expr)
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. }
            | ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                2 + Self::class_expression_complexity(filler)
            }
            ClassExpression::ObjectHasValue { .. } => 2,
            ClassExpression::ObjectHasSelf { .. } => 2,
            ClassExpression::ObjectMinCardinality { filler, .. }
            | ClassExpression::ObjectMaxCardinality { filler, .. }
            | ClassExpression::ObjectExactCardinality { filler, .. } => {
                3 + Self::class_expression_complexity(filler.as_ref())
            }
            ClassExpression::DataSomeValuesFrom { .. }
            | ClassExpression::DataAllValuesFrom { .. } => 2,
            ClassExpression::DataHasValue { .. } => 2,
            ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. } => 3,
            ClassExpression::ObjectOneOf(_) => 2,
        }
    }
}

impl Default for ConjunctiveQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for QueryVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?{}", self.name)
    }
}

impl fmt::Display for QueryAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryAtom::ClassAtom {
                variable,
                class_expression,
            } => {
                write!(f, "{class_expression}({variable})")
            }
            QueryAtom::ObjectPropertyAtom {
                subject,
                property,
                object,
            } => {
                write!(f, "{property}({subject}, {object})")
            }
            QueryAtom::DataPropertyAtom {
                subject,
                property,
                literal,
            } => {
                write!(f, "{property}({subject}, {literal})")
            }
            QueryAtom::SameIndividualAtom { left, right } => {
                write!(f, "{left} = {right}")
            }
            QueryAtom::DifferentIndividualsAtom { left, right } => {
                write!(f, "{left} ≠ {right}")
            }
            QueryAtom::ConcreteIndividualAtom {
                variable,
                individual,
            } => {
                write!(f, "{variable} = {individual}")
            }
            QueryAtom::ConcreteLiteralAtom { variable, literal } => {
                write!(f, "{variable} = {literal}")
            }
        }
    }
}

impl fmt::Display for ConjunctiveQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SELECT ")?;

        if self.answer_variables.is_empty() {
            write!(f, "*")?;
        } else {
            for (i, var) in self.answer_variables.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{var}")?;
            }
        }

        write!(f, " WHERE {{ ")?;

        for (i, atom) in self.body_atoms.iter().enumerate() {
            if i > 0 {
                write!(f, " . ")?;
            }
            write!(f, "{atom}")?;
        }

        write!(f, " }}")
    }
}

impl Eq for ValueConstraint {}

impl std::hash::Hash for ConjunctiveQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.answer_variables.hash(state);
        self.body_atoms.hash(state);
        // Skip constraints and metadata for hashing to avoid complex Hash requirements
    }
}
