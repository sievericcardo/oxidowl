//! OWL 2 DL Concepts and Class Expressions
//!
//! This module implements OWL 2 DL class expressions and concept representation
//! following the OWL 2 specification structure.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Identifier for concepts
pub type ConceptId = u64;

/// Named OWL classes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Class {
    pub iri: crate::ontology::IRI,
}

impl Class {
    #[must_use]
    pub fn new(iri: crate::ontology::IRI) -> Self {
        Self { iri }
    }

    #[must_use]
    pub fn thing() -> Self {
        Self::new(crate::ontology::IRI::new(
            "http://www.w3.org/2002/07/owl#Thing",
        ))
    }

    #[must_use]
    pub fn nothing() -> Self {
        Self::new(crate::ontology::IRI::new(
            "http://www.w3.org/2002/07/owl#Nothing",
        ))
    }

    #[must_use]
    pub fn is_thing(&self) -> bool {
        self.iri == crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing")
    }

    #[must_use]
    pub fn is_nothing(&self) -> bool {
        self.iri == crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing")
    }
}

/// OWL 2 DL Class expressions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassExpression {
    /// Named class
    Class(Class),

    /// Intersection of class expressions (`ObjectIntersectionOf`)
    ObjectIntersectionOf(Vec<ClassExpression>),

    /// Union of class expressions (`ObjectUnionOf`)
    ObjectUnionOf(Vec<ClassExpression>),

    /// Enumeration of individuals (`ObjectOneOf`)
    ObjectOneOf(Vec<crate::ontology::Individual>),

    /// Object property restriction (`ObjectSomeValuesFrom`)
    ObjectSomeValuesFrom {
        property: crate::ontology::ObjectPropertyExpression,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (`ObjectAllValuesFrom`)
    ObjectAllValuesFrom {
        property: crate::ontology::ObjectPropertyExpression,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (`ObjectHasValue`)
    ObjectHasValue {
        property: crate::ontology::ObjectPropertyExpression,
        value: crate::ontology::Individual,
    },

    /// Object property restriction (`ObjectHasSelf`)
    ObjectHasSelf {
        property: crate::ontology::ObjectPropertyExpression,
    },

    /// Object property restriction (`ObjectMinCardinality`)
    ObjectMinCardinality {
        property: crate::ontology::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (`ObjectMaxCardinality`)
    ObjectMaxCardinality {
        property: crate::ontology::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (`ObjectExactCardinality`)
    ObjectExactCardinality {
        property: crate::ontology::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Data property restriction (`DataSomeValuesFrom`)
    DataSomeValuesFrom {
        property: crate::ontology::DataPropertyExpression,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (`DataAllValuesFrom`)
    DataAllValuesFrom {
        property: crate::ontology::DataPropertyExpression,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (`DataHasValue`)
    DataHasValue {
        property: crate::ontology::DataPropertyExpression,
        value: crate::ontology::Literal,
    },

    /// Data property restriction (`DataMinCardinality`)
    DataMinCardinality {
        property: crate::ontology::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (`DataMaxCardinality`)
    DataMaxCardinality {
        property: crate::ontology::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (`DataExactCardinality`)
    DataExactCardinality {
        property: crate::ontology::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Negation of a class expression (`ObjectComplementOf`)
    ObjectComplementOf(Box<ClassExpression>),
}

impl ClassExpression {
    /// Create a class expression from a named class
    #[must_use]
    pub fn class(iri: crate::ontology::IRI) -> Self {
        ClassExpression::Class(Class::new(iri))
    }

    /// Create the OWL Thing class expression
    #[must_use]
    pub fn thing() -> Self {
        ClassExpression::Class(Class::thing())
    }

    /// Create the OWL Nothing class expression
    #[must_use]
    pub fn nothing() -> Self {
        ClassExpression::Class(Class::nothing())
    }

    /// Create an intersection of class expressions
    #[must_use]
    pub fn intersection_of(expressions: Vec<ClassExpression>) -> Self {
        if expressions.is_empty() {
            Self::thing() // Intersection of nothing is Thing
        } else if expressions.len() == 1 {
            expressions
                .into_iter()
                .next()
                .expect("Vector has exactly one element as verified by length check")
        } else {
            ClassExpression::ObjectIntersectionOf(expressions)
        }
    }

    /// Create a union of class expressions
    #[must_use]
    pub fn union_of(expressions: Vec<ClassExpression>) -> Self {
        if expressions.is_empty() {
            Self::nothing() // Union of nothing is Nothing
        } else if expressions.len() == 1 {
            expressions
                .into_iter()
                .next()
                .expect("Vector has exactly one element as verified by length check")
        } else {
            ClassExpression::ObjectUnionOf(expressions)
        }
    }

    /// Create a complement of a class expression
    #[must_use]
    pub fn complement_of(expression: ClassExpression) -> Self {
        ClassExpression::ObjectComplementOf(Box::new(expression))
    }

    /// Create an existential restriction (some values from)
    #[must_use]
    pub fn some_values_from(
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> Self {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Create a universal restriction (all values from)
    #[must_use]
    pub fn all_values_from(
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> Self {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Check if this class is a named class
    #[must_use]
    pub fn is_named_class(&self) -> bool {
        matches!(self, ClassExpression::Class(_))
    }

    /// Check if this class ia a complex class expression
    #[must_use]
    pub fn is_complex_class_expression(&self) -> bool {
        !self.is_named_class()
    }

    /// Get the named class IRI if this is a named class
    #[must_use]
    pub fn as_class(&self) -> Option<&Class> {
        if let ClassExpression::Class(class) = self {
            Some(class)
        } else {
            None
        }
    }

    /// Get the IRI if this is a named class
    #[must_use]
    pub fn iri(&self) -> Option<&crate::ontology::IRI> {
        match self {
            ClassExpression::Class(class) => Some(&class.iri),
            _ => None,
        }
    }

    /// Get all named classes referenced in this class expression
    #[must_use]
    pub fn signature(&self) -> HashSet<Class> {
        let mut signature = HashSet::new();
        self.collect_classes(&mut signature);
        signature
    }

    fn collect_classes(&self, signature: &mut HashSet<Class>) {
        match self {
            ClassExpression::Class(class) => {
                signature.insert(class.clone());
            }
            ClassExpression::ObjectIntersectionOf(expressions)
            | ClassExpression::ObjectUnionOf(expressions) => {
                for expr in expressions {
                    expr.collect_classes(signature);
                }
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. }
            | ClassExpression::ObjectAllValuesFrom { filler, .. }
            | ClassExpression::ObjectMinCardinality { filler, .. }
            | ClassExpression::ObjectMaxCardinality { filler, .. }
            | ClassExpression::ObjectExactCardinality { filler, .. } => {
                filler.collect_classes(signature);
            }
            ClassExpression::ObjectHasValue { .. } => {
                // ObjectHasValue has an individual value, not a class expression
                // No classes to collect from individuals
            }
            ClassExpression::DataSomeValuesFrom { filler, .. }
            | ClassExpression::DataAllValuesFrom { filler, .. }
            | ClassExpression::DataMinCardinality { filler, .. }
            | ClassExpression::DataMaxCardinality { filler, .. }
            | ClassExpression::DataExactCardinality { filler, .. } => {
                // Data ranges do not contain named classes
            }
            ClassExpression::ObjectComplementOf(expr) => {
                expr.collect_classes(signature);
            }
            _ => {} // Other expressions do not contain named classes
        }
    }

    /// Compute the negation normal form (NNF) of this class expression
    #[must_use]
    pub fn to_nnf(&self) -> ClassExpression {
        self.to_nnf_helper(false)
    }

    fn to_nnf_helper(&self, negated: bool) -> ClassExpression {
        match self {
            ClassExpression::ObjectComplementOf(expr) => {
                // Negate the inner expression
                expr.to_nnf_helper(!negated)
            }

            ClassExpression::ObjectIntersectionOf(expressions) => {
                let nnf_expressions: Vec<_> = expressions
                    .iter()
                    .map(|e| e.to_nnf_helper(negated))
                    .collect();
                if negated {
                    ClassExpression::ObjectUnionOf(nnf_expressions)
                } else {
                    ClassExpression::ObjectIntersectionOf(nnf_expressions)
                }
            }

            ClassExpression::ObjectUnionOf(expressions) => {
                let nnf_expressions: Vec<_> = expressions
                    .iter()
                    .map(|e| e.to_nnf_helper(negated))
                    .collect();
                if negated {
                    ClassExpression::ObjectIntersectionOf(nnf_expressions)
                } else {
                    ClassExpression::ObjectUnionOf(nnf_expressions)
                }
            }

            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                if negated {
                    // Negation of some values from becomes all values from
                    ClassExpression::ObjectAllValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(true)),
                    }
                } else {
                    ClassExpression::ObjectSomeValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(false)),
                    }
                }
            }

            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                if negated {
                    // Negation of all values from becomes some values from
                    ClassExpression::ObjectSomeValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(true)),
                    }
                } else {
                    ClassExpression::ObjectAllValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(false)),
                    }
                }
            }

            // For other expressions, wrap in complement if negated
            _ => {
                if negated {
                    ClassExpression::ObjectComplementOf(Box::new(self.clone()))
                } else {
                    self.clone()
                }
            }
        }
    }

    /// Check if this class expression is in negation normal form (NNF)
    #[must_use]
    pub fn is_nnf(&self) -> bool {
        match self {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectIntersectionOf(expressions)
            | ClassExpression::ObjectUnionOf(expressions) => {
                expressions.iter().all(ClassExpression::is_nnf)
            }
            ClassExpression::ObjectComplementOf(expr) => {
                // NNF does not allow negation of complex expressions
                expr.is_named_class()
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. }
            | ClassExpression::ObjectAllValuesFrom { filler, .. } => filler.is_nnf(),
            ClassExpression::ObjectMinCardinality { filler, .. }
            | ClassExpression::ObjectMaxCardinality { filler, .. }
            | ClassExpression::ObjectExactCardinality { filler, .. } => {
                // Cardinality restrictions are in NNF if filler is in NNF
                filler.is_nnf()
            }
            ClassExpression::DataSomeValuesFrom { filler, .. }
            | ClassExpression::DataAllValuesFrom { filler, .. }
            | ClassExpression::DataMinCardinality { filler, .. }
            | ClassExpression::DataMaxCardinality { filler, .. }
            | ClassExpression::DataExactCardinality { filler, .. } => {
                // Data ranges do not contain negations
                true
            }
            _ => {
                // Other expressions do not contain negations
                true
            }
        }
    }

    /// Simplify the class expression by applying logic rules
    pub fn simplify(&self) -> crate::Result<ClassExpression> {
        match self {
            ClassExpression::ObjectIntersectionOf(expressions) => {
                let simplified: Vec<_> = expressions
                    .iter()
                    .map(ClassExpression::simplify)
                    .collect::<crate::Result<Vec<_>>>()?;

                // Remove duplicates and empty expressions
                let mut unique_exprs = Vec::with_capacity(simplified.len());
                let mut has_nothing = false;

                for expr in simplified {
                    if let ClassExpression::Class(class) = &expr {
                        if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                            has_nothing = true;
                            break; // Nothing dominates intersection
                        } else if class.iri.as_str() != "http://www.w3.org/2002/07/owl#Thing" {
                            continue; // Ignore Thing in intersection
                        }
                    }

                    if !unique_exprs.contains(&expr) {
                        unique_exprs.push(expr);
                    }
                }

                if has_nothing {
                    Ok(Self::nothing())
                } else if unique_exprs.is_empty() {
                    Ok(Self::thing()) // Empty intersection is Thing
                } else if unique_exprs.len() == 1 {
                    Ok(unique_exprs
                        .into_iter()
                        .next()
                        .expect("Vector has exactly one element as verified by length check"))
                } else {
                    Ok(ClassExpression::ObjectIntersectionOf(unique_exprs))
                }
            }

            ClassExpression::ObjectUnionOf(expressions) => {
                let simplified: Vec<_> = expressions
                    .iter()
                    .map(ClassExpression::simplify)
                    .collect::<Result<Vec<_>, _>>()?;

                // Remove duplicates and empty expressions
                let mut unique_exprs = Vec::with_capacity(simplified.len());
                let mut has_thing = false;

                for expr in simplified {
                    if let ClassExpression::Class(class) = &expr {
                        if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                            has_thing = true;
                            break; // Thing dominates union
                        } else if class.iri.as_str() != "http://www.w3.org/2002/07/owl#Nothing" {
                            continue; // Ignore Nothing in union
                        }
                    }

                    if !unique_exprs.contains(&expr) {
                        unique_exprs.push(expr);
                    }
                }

                if has_thing {
                    Ok(Self::thing())
                } else if unique_exprs.is_empty() {
                    Ok(Self::nothing()) // Empty union is Nothing
                } else if unique_exprs.len() == 1 {
                    Ok(unique_exprs
                        .into_iter()
                        .next()
                        .expect("Vector has exactly one element as verified by length check"))
                } else {
                    Ok(ClassExpression::ObjectUnionOf(unique_exprs))
                }
            }

            ClassExpression::ObjectComplementOf(expr) => {
                let simplified = expr.simplify()?;

                if let ClassExpression::ObjectComplementOf(inner) = simplified {
                    // Double negation elimination
                    Ok(inner.as_ref().clone())
                } else if let ClassExpression::Class(class) = &simplified {
                    if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                        // Complement of Thing is Nothing
                        Ok(ClassExpression::nothing())
                    } else if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                        // Complement of Nothing is Thing
                        Ok(ClassExpression::thing())
                    } else {
                        // Complement of a named class remains unchanged
                        Ok(ClassExpression::ObjectComplementOf(Box::new(simplified)))
                    }
                } else {
                    // Other expressions remain unchanged
                    Ok(ClassExpression::ObjectComplementOf(Box::new(simplified)))
                }
            }

            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let simplified_filler = filler.simplify()?;
                Ok(ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(simplified_filler),
                })
            }

            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let simplified_filler = filler.simplify()?;
                Ok(ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(simplified_filler),
                })
            }

            ClassExpression::ObjectHasValue { property, value } => {
                // Has value restrictions do not simplify further
                Ok(ClassExpression::ObjectHasValue {
                    property: property.clone(),
                    value: value.clone(),
                })
            }

            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::ObjectMinCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(filler.simplify()?),
            }),

            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::ObjectMaxCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(filler.simplify()?),
            }),

            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::ObjectExactCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(filler.simplify()?),
            }),

            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // Data ranges do not simplify further
                Ok(ClassExpression::DataSomeValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                })
            }

            ClassExpression::DataAllValuesFrom { property, filler } => {
                // Data ranges do not simplify further
                Ok(ClassExpression::DataAllValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                })
            }

            ClassExpression::DataHasValue { property, value } => {
                // Has value restrictions do not simplify further
                Ok(ClassExpression::DataHasValue {
                    property: property.clone(),
                    value: value.clone(),
                })
            }

            ClassExpression::DataMinCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::DataMinCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            }),

            ClassExpression::DataMaxCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::DataMaxCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            }),

            ClassExpression::DataExactCardinality {
                property,
                cardinality,
                filler,
            } => Ok(ClassExpression::DataExactCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            }),

            _ => {
                // Other expressions remain unchanged
                Ok(self.clone())
            }
        }
    }
}

/// Concept store for managing named classes and class expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptStore {
    classes: HashMap<crate::ontology::IRI, Class>,
    expressions: HashMap<ConceptId, ClassExpression>,
    next_id: ConceptId,
}

impl ConceptStore {
    #[must_use]
    pub fn new() -> Self {
        let mut store = Self {
            classes: HashMap::new(),
            expressions: HashMap::new(),
            next_id: 0,
        };

        // Built-in classes
        store.add_class(Class::thing());
        store.add_class(Class::nothing());

        store
    }

    pub fn add_class(&mut self, class: Class) -> &Class {
        let iri = class.iri.clone();
        self.classes.entry(iri).or_insert(class)
    }

    #[must_use]
    pub fn get_class(&self, iri: &crate::ontology::IRI) -> Option<&Class> {
        self.classes.get(iri)
    }

    pub fn get_or_create_class(&mut self, iri: crate::ontology::IRI) -> &Class {
        if !self.classes.contains_key(&iri) {
            let class = Class::new(iri.clone());
            self.classes.insert(iri.clone(), class);
        }
        &self.classes[&iri]
    }

    pub fn add_expression(&mut self, expression: ClassExpression) -> ConceptId {
        let id = self.next_id;
        self.expressions.insert(id, expression);
        self.next_id += 1;
        id
    }

    #[must_use]
    pub fn get_expression(&self, id: ConceptId) -> Option<&ClassExpression> {
        self.expressions.get(&id)
    }

    pub fn all_classes(&self) -> impl Iterator<Item = &Class> {
        self.classes.values()
    }

    pub fn all_expressions(&self) -> impl Iterator<Item = &ClassExpression> {
        self.expressions.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len() + self.expressions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.expressions.is_empty()
    }
}

impl Default for ConceptStore {
    fn default() -> Self {
        Self::new()
    }
}

use std::fmt;

impl fmt::Display for ClassExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClassExpression::Class(class) => write!(f, "{}", class.iri.as_str()),
            ClassExpression::ObjectIntersectionOf(classes) => {
                write!(f, "(")?;
                for (i, class) in classes.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊓ ")?;
                    }
                    write!(f, "{class}")?;
                }
                write!(f, ")")
            }
            ClassExpression::ObjectUnionOf(classes) => {
                write!(f, "(")?;
                for (i, class) in classes.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊔ ")?;
                    }
                    write!(f, "{class}")?;
                }
                write!(f, ")")
            }
            ClassExpression::ObjectComplementOf(class) => {
                write!(f, "¬{class}")
            }
            ClassExpression::ObjectOneOf(individuals) => {
                write!(f, "{{")?;
                for (i, individual) in individuals.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{individual}")?;
                }
                write!(f, "}}")
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                write!(f, "∃{property}.{filler}")
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                write!(f, "∀{property}.{filler}")
            }
            ClassExpression::ObjectHasValue { property, value } => {
                write!(f, "∃{property}.{{{value}}}")
            }
            ClassExpression::ObjectHasSelf { property } => {
                write!(f, "∃{property}.Self")
            }
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "≥{cardinality} {property}.{filler}")
            }
            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "≤{cardinality} {property}.{filler}")
            }
            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "={cardinality} {property}.{filler}")
            }
            // Data property restrictions
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                write!(f, "∃{property}.{filler}")
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                write!(f, "∀{property}.{filler}")
            }
            ClassExpression::DataHasValue { property, value } => {
                write!(f, "∃{property}.{{{value}}}")
            }
            ClassExpression::DataMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "≥{cardinality} {property}.{filler}")
            }
            ClassExpression::DataMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "≤{cardinality} {property}.{filler}")
            }
            ClassExpression::DataExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                write!(f, "={cardinality} {property}.{filler}")
            }
        }
    }
}
