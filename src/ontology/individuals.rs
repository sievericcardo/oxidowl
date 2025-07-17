//! OWL 2 DL Individuals
//! 
//! This module implements OWL 2 DL individuals (named and anonymous)
//! following the OWL 2 specification structure.

use crate::Error;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Identifiers for OWL 2 DL individuals.
pub type IndividualId = u64;

/// Represents an OWL 2 DL individual.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Individual {
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
        Individual::Named(NamedIndividual { iri })
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
            Individual::Anonymous(anon) => format!("_:{}", anon.id),
        }
    }
}

impl fmt::Display for Individual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Individual::Named(named) => write!(f, "{}", named.iri),
            Individual::Anonymous(anon) => write!(f, "_:{}", anon.id),
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

/// Store for managing individuals and their assertions.
#[derive(Debug, Clone)]
pub struct IndividualStore {
    /// Map of named individuals by their identifiers.
    named_individuals: HashMap<crate::ontology::IRI, NamedIndividual>,

    /// Map of anonymous individuals by their unique identifiers.
    anonymous_individuals: HashMap<String, AnonymousIndividual>,

    /// List of individual assertions.
    assertions: Vec<IndividualAssertion>,

    /// Next unique identifier for anonymous individuals.
    next_anon_id: u64,
}

impl IndividualStore {
    pub fn new() -> Self {
        Self {
            named_individuals: HashMap::new(),
            anonymous_individuals: HashMap::new(),
            assertions: Vec::new(),
            next_anon_id: 0,
        }
    }

    /// Add a named individual to the store.
    pub fn add_named_individual(&mut self, individual: NamedIndividual) -> &NamedIndividual {
        let iri = individual.iri.clone();
        self.named_individuals.entry(iri).or_insert(individual)
    }

    /// Get a named individual by its IRI.
    pub fn get_named_individual(&self, iri: &crate::ontology::IRI) -> Option<&NamedIndividual> {
        self.named_individuals.get(iri)
    }

    /// Get or create a named individual by its IRI.
    pub fn get_or_create_named_individual(&mut self, iri: crate::ontology::IRI) -> &NamedIndividual {
        if !self.named_individuals.contains_key(&iri) {
            self.add_named_individual(NamedIndividual { iri: iri.clone() });
        }
        self.named_individuals.get(&iri).expect("Named individual should exist")
    }

    /// Add an anonymous individual to the store.
    pub fn add_anonymous_individual(&mut self, individual: AnonymousIndividual) -> &AnonymousIndividual {
        let id = individual.id.clone();
        self.anonymous_individuals.entry(id).or_insert(individual)
    }

    /// Get an anonymous individual by its unique identifier.
    pub fn get_anonymous_individual(&self, id: &str) -> Option<&AnonymousIndividual> {
        self.anonymous_individuals.get(id)
    }

    /// Create a new anonymous individual with a generated unique identifier.
    pub fn create_anonymous_individual(&mut self) -> &AnonymousIndividual {
        let id = AnonymousIndividual::generate_unique_id();
        let individual = AnonymousIndividual::new(id);
        self.add_anonymous_individual(individual)
    }

    /// Add an individual assertion to the store.
    pub fn add_assertion(&mut self, assertion: IndividualAssertion) {
        // Validate the assertion before adding it
        match &assertion {
            IndividualAssertion::ClassAssertion { individual, class } => {
                if !self.is_valid_class_assertion(individual, class) {
                    return Err(Error::InvalidAssertion { message: "Invalid class assertion".to_string() });
                }
            }
            IndividualAssertion::ObjectPropertyAssertion { subject, object, property } => {
                if !self.is_valid_object_property_assertion(subject, object, property) {
                    return Err(Error::InvalidAssertion { message: "Invalid object property assertion".to_string() });
                }
            }
            IndividualAssertion::DataPropertyAssertion { subject, value, property } => {
                if !self.is_valid_data_property_assertion(subject, value, property) {
                    return Err(Error::InvalidAssertion { message: "Invalid data property assertion".to_string() });
                }
            }
            _ => {}
        }

        for individual in assertion.individuals() {
            if individual.is_anonymous() {
                self.add_anonymous_individual(individual.anonymous_id().unwrap().clone());
            } else {
                self.add_named_individual(individual.named_iri().unwrap().clone());
            }
        }

        self.assertions.push(assertion);
    }

    /// Get all assertions
    pub fn assertions(&self) -> &Vec<IndividualAssertion> {
        &self.assertions
    }

    /// Get assertions by specific individual
    pub fn assertions_for_individual(&self, individual: &Individual) -> Vec<&IndividualAssertion> {
        self.assertions
            .iter()
            .filter(|a| a.individuals()
            .contains(individual))
            .collect()
    }

    /// Get class assertions for a specific individual
    pub fn class_assertions_for_individual(&self, individual: &Individual) -> Vec<&IndividualAssertion> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::ClassAssertion { individual: ind, class: _ } = assertion {
                    if ind == individual {
                        Some(assertion)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get object property assertions where the individual is the subject
    pub fn object_property_assertions_for_subject(&self, individual: &Individual) -> Vec<(&crate::ontology::ObjectPropertyExpression, &Individual)> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::ObjectPropertyAssertion { subject: sub, property: _, object: _ } = assertion {
                    if sub == individual {
                        Some((assertion.property(), assertion.object()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get data property assertions where the individual is the object
    pub fn data_property_assertions_for_subject(&self, individual: &Individual) -> Vec<(&crate::ontology::DataPropertyExpression, &crate::ontology::Literal)> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::DataPropertyAssertion { subject: sub, property: _, value: _ } = assertion {
                    if sub == individual {
                        Some((assertion.property(), assertion.value()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get data property assertions for a specific individual
    pub fn data_property_assertions_for_individual(&self, individual: &Individual) -> Vec<(&crate::ontology::DataProperty, &crate::ontology::Literal)> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::DataPropertyAssertion { subject: sub, property, value } = assertion {
                    if sub == individual {
                        Some((property, value))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get individuals that are explicitly stated to be the same
    pub fn same_individuals(&self) -> Vec<&Individual> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::SameIndividuals { individuals } = assertion {
                    if individuals.contains(&self.individual) {
                        Some(individuals
                            .iter()
                            .filter(|&ind| ind != &self.individual)
                            .collect::<Vec<&Individual>>())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// Get individuals that are stated to be different
    pub fn different_individuals(&self) -> Vec<&Individual> {
        self.assertions
            .iter()
            .filter_map(|assertion| {
                if let IndividualAssertion::DifferentIndividuals { individuals } = assertion {
                    if individuals.contains(&self.individual) {
                        Some(individuals
                            .iter()
                            .filter(|&ind| ind != &self.individual)
                            .collect::<Vec<_>>())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// Get all named individuals
    pub fn named_individuals(&self) -> impl Iterator<Item = &NamedIndividual> {
        self.named_individuals.values()
    }

    /// Get all anonymous individuals
    pub fn anonymous_individuals(&self) -> impl Iterator<Item = &AnonymousIndividual> {
        self.anonymous_individuals.values()
    }

    /// Get all individuals (both named and anonymous)
    pub fn all_individuals(&self) -> Vec<Individual> {
        let mut individuals = Vec::new();
        for named in self.named_individuals.values() {
            individuals.push(Individual::Named(named.clone()));
        }
        for anon in self.anonymous_individuals.values() {
            individuals.push(Individual::Anonymous(anon.clone()));
        }
        individuals
    }

    /// Check if an individual is known
    pub fn is_known_individual(&self, individual: &Individual) -> bool {
        match individual {
            Individual::Named(named) => self.named_individuals.contains_key(&named.iri),
            Individual::Anonymous(anon) => self.anonymous_individuals.contains_key(&anon.id),
        }
    }

    /// Get the number of named individuals
    pub fn named_individual_count(&self) -> usize {
        self.named_individuals.len()
    }

    /// Check if the store contains any named individuals
    pub fn has_named_individuals(&self) -> bool {
        !self.named_individuals.is_empty()
    }

    /// Clear all individuals and assertions from the store
    pub fn clear(&mut self) {
        self.named_individuals.clear();
        self.anonymous_individuals.clear();
        self.assertions.clear();
        self.next_anon_id = 0;
    }
}

impl Default for IndividualStore {
    fn default() -> Self {
        Self::new()
    }
}
