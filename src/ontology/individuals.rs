//! OWL 2 DL Individuals
//! 
//! This module implements OWL 2 DL individuals (named and anonymous)
//! following the OWL 2 specification structure.

use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

/// Identifiers for OWL 2 DL individuals.
pub type IndividualId = u64;

/// Represents an OWL 2 DL individual.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Individual {
    /// Named individual identifier.
    Named(NamedIndividual),

    /// Anonymous individual identifier.
    Anonymous(AnonymousIndividual),
}

/// Represents a named OWL 2 DL individual with an IRI
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedIndividual {
    /// IRI of the named individual.
    pub iri: crate::ontology::IRI,
}

/// Represents an anonymous OWL 2 DL individual.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnonymousIndividual {
    pub id: String,
}

impl Individual {
    /// Creates a new named individual from an IRI.
    pub fn named(iri: crate::ontology::IRI) -> Self {
        Individual::Named(NamedIndividual { iri )
    }

    /// Creates a new anonymous individual with a unique identifier.
    pub fn anonymous(id: String) -> Self {
        Individual::Anonymous(AnonymousIndividual { id })
    }

    /// Check if the individual is named.
    pub fn is_named(&self) -> bool {
        matches!(self, Individual::Named(_))
    }

    /// Check if the individual is anonymous.
    pub fn is_anonymous(&self) -> bool {
        matches!(self, Individual::Anonymous(_))
    }

    /// Get the IRI of a named individual.
    pub fn named_iri(&self) -> Option<&NamedIndividual> {
        if let Individual::Named(named) = self {
            Some(named)
        } else {
            None
        }
    }

    /// Get the ID of an anonymous individual.
    pub fn anonymous_id(&self) -> Option<&AnonymousIndividual> {
        if let Individual::Anonymous(anon) = self {
            Some(anon)
        } else {
            None
        }
    }

    /// Get a string representation of the individual.
    pub fn to_string(&self) -> String {
        match self {
            Individual::Named(named) => named.iri.to_string(),
            Individual::Anonymous(anon) => format!("_:{}", anonymous.id),
        }
    }
}

impl NamedIndividual {
    /// Creates a new named individual from an IRI.
    pub fn new(iri: crate::ontology::IRI) -> Self {
        Self { iri }
    }
}

impl AnonymousIndividual {
    /// Creates a new anonymous individual with a unique identifier.
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Generate a unique identifier for an anonymous individual.
    pub fn generate_unique_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        format!("anon_:{}", COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a new anonymous individual with a unique identifier.
    pub fn new_unique() -> Self {
        Self::new(Self::generate_unique_id())
    }
}

/// Individual assertion for ABox reasoning
#[derive(Debug, Clone, PartialEq)]
pub enum IndividualAssertion {
    /// Class assertion: the individual is an instance of a class.
    ClassAssertion {
        individual: Individual,
        class: crate::ontology::ClassExpression,
    },

    /// Object property assertion: the individual has a relationship with another individual.
    ObjectPropertyAssertion {
        subject: Individual,
        property: crate::ontology::ObjectPropertyExpression,
        object: Individual,
    },

    /// Negative object property assertion: the individual does not have a relationship with another individual.
    NegativeObjectPropertyAssertion {
        subject: Individual,
        property: crate::ontology::ObjectPropertyExpression,
        object: Individual,
    },

    /// Data property assertion: the individual has a relationship with a data value.
    DataPropertyAssertion {
        subject: Individual,
        property: crate::ontology::DataPropertyExpression,
        value: crate::ontology::Literal,
    },

    /// Negative data property assertion: the individual does not have a relationship with a data value.
    NegativeDataPropertyAssertion {
        subject: Individual,
        property: crate::ontology::DataPropertyExpression,
        value: crate::ontology::Literal,
    },

    /// Same individuals assertion: the individual is equivalent to another individual.
    SameIndividuals {
        individuals: Vec<Individual>,
    },

    /// Different individuals assertion: the individual is not equivalent to another individual.
    DifferentIndividuals {
        individuals: Vec<Individual>,
    },

    /// Annotation assertion: the individual has an annotation.
    AnnotationAssertion {
        individual: Individual,
        property: crate::ontology::AnnotationPropertyExpression,
        value: crate::ontology::Literal,
    },
}

impl IndividualAssertion {
    /// Get the individual involved in the assertion.
    pub fn individuals(&self) -> HashSet<Individual> {
        let mut individuals = HashSet::new();

        match self {
            IndividualAssertion::ClassAssertion { individual, .. } => {
                individuals.insert(individual.clone());
            }
            IndividualAssertion::ObjectPropertyAssertion { subject, object, .. } |
            IndividualAssertion::NegativeObjectPropertyAssertion { subject, object, .. } => {
                individuals.insert(subject.clone());
                individuals.insert(object.clone());
            }
            IndividualAssertion::DataPropertyAssertion { subject, .. } |
            IndividualAssertion::NegativeDataPropertyAssertion { subject, .. } => {
                individuals.insert(subject.clone());
            }
            IndividualAssertion::SameIndividuals { individuals: inds } |
            IndividualAssertion::DifferentIndividuals { individuals: inds } => {
                individuals.extend(inds.iter().cloned());
            }
            IndividualAssertion::AnnotationAssertion { individual, .. } => {
                individuals.insert(individual.clone());
            }
        }

        individuals
    }

    /// Check if the assertion is positive (i.e., it asserts a relationship).
    pub fn is_positive(&self) -> bool {
        !matches!(
            self,
            IndividualAssertion::NegativeObjectPropertyAssertion { .. }
                | IndividualAssertion::NegativeDataPropertyAssertion { .. }
        )
    }

    /// Check if the assertion is negative (i.e., it denies a relationship).
    pub fn is_negative(&self) -> bool {
        !self.is_positive()
    }
}

