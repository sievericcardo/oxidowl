//! OWL 2 DL Properties
//! 
//! This module implements OWL 2 DL object properties, data properties, and annotation properties
//! following the OWL 2 specification structure.

use crate::{Error, Result};
use crate::ontology::{ObjectPropertyExpression, ObjectProperty};
use std::collections::{HashMap, HashSet};

/// Data Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataProperty {
    pub iri: crate::ontology::IRI,
}

/// Annotation Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnnotationProperty {
    pub iri: crate::ontology::IRI,
}

/// Role (interface for Object and Data Properties in tableau reasoning)
##[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    ObjectProperty(crate::ontology::ObjectPropertyExpression),
    DataProperty(DataProperty),
}

impl ObjectProperty {
    /// Create a new Object Property
    pub fn new(iri: crate::ontology::IRI) -> Result<Self> {
        Ok(Self { iri: iri.to_url()? })
    }

    /// Create the top object property (owl:topObjectProperty)
    pub fn top() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#topObjectProperty".to_string())).unwrap()
    }

    /// Create the bottom object property (owl:bottomObjectProperty)
    pub fn bottom() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#bottomObjectProperty".to_string())).unwrap()
    }
}

impl DataProperty {
    /// Create a new Data Property
    pub fn new(iri: crate::ontology::IRI) -> Self {
        Self { iri }
    }

    /// Create the top data property (owl:topDataProperty)
    pub fn top() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#topDataProperty".to_string())).unwrap()
    }

    /// Create the bottom data property (owl:bottomDataProperty)
    pub fn bottom() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#bottomDataProperty".to_string())).unwrap()
    }
}

impl AnnotationProperty {
    /// Create a new Annotation Property
    pub fn new(iri: crate::ontology::IRI) -> Self {
        Self { iri }
    }

    /// Create the top annotation property (owl:topAnnotationProperty)
    pub fn top() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#topAnnotationProperty".to_string())).unwrap()
    }

    /// Create the bottom annotation property (owl:bottomAnnotationProperty)
    pub fn bottom() -> Self {
        Self::new(crate::ontology::IRI::from("http://www.w3.org/2002/07/owl#bottomAnnotationProperty".to_string())).unwrap()
    }
}

impl ObjectPropertyExpression {
    /// Create an object property expression
    pub fn property(property: ObjectProperty) -> Self {
        ObjectPropertyExpression::ObjectProperty(property)
    }

    /// Create an inverse object property expression
    pub fn inverse(property: ObjectProperty) -> Self {
        ObjectPropertyExpression::InverseObjectProperty(property)
    }

    /// Create a property chain expression
    pub fn property_chain(properties: Vec<ObjectProperty>) -> Result<Self> {
        if properties.is_empty() {
            return Err(Error::InvalidPropertyChain("Property chain cannot be empty".to_string()));
        } else if properties.len() == 1 {
            return Err(crate::Error::ontology_parsing("Property chain must contain at least 2 properties"));
        }
        Ok(ObjectPropertyExpression::PropertyChain(properties))
    }

    /// Check if the expression is a simple property expression
    pub fn is_simple(&self) -> bool {
        matches!(self, ObjectPropertyExpression::ObjectProperty(_))
    }

    /// Check if the expression is an inverse property expression
    pub fn is_inverse(&self) -> bool {
        matches!(self, ObjectPropertyExpression::InverseObjectProperty(_))
    }

    /// Check if the expression is a property chain
    pub fn is_property_chain(&self) -> bool {
        matches!(self, ObjectPropertyExpression::PropertyChain(_))
    }

    /// Check if the expression is a simple property
    pub fn is_simple_property(&self) -> bool {
        !self.is_property_chain()
    }

    /// Get the length of the property chain if it is a property chain expression
    pub fn chain_length(&self) -> usize {
        match self {
            ObjectPropertyExpression::PropertyChain(chain) => chain.len(),
            _ => 1,
        }
    }

    /// Get the object property if the expression is a simple property
    pub fn as_object_property(&self) -> Option<&ObjectProperty> {
        if let ObjectPropertyExpression::ObjectProperty(property) = self {
            Some(property)
        } else {
            None
        }
    }

    /// Get the inverse object property if the expression is an inverse property
    pub fn as_inverse_object_property(&self) -> &ObjectPropertyExpression {
        match self {
            ObjectPropertyExpression::ObjectProperty(property) => {
                ObjectPropertyExpression::InverseObjectProperty(property.clone())
            }
            ObjectPropertyExpression::InverseObjectProperty(property) => {
                ObjectPropertyExpression::ObjectProperty(property.clone())
            }
            ObjectPropertyExpression::PropertyChain(_) => {
                // Inverse of a property chain is the reverse chain with each property inverted
                let inverse_chain: Vec<ObjectPropertyExpression> = chain
                    .iter()
                    .rev()
                    .map(|p| p.get_inverse())
                    .collect();
                ObjectPropertyExpression::PropertyChain(inverse_chain)
            }
        }
    }

    /// Get the simple object property at the core of this expression
    /// For property chains, this returns the first property in the chain
    pub fn get_named_property(&self) -> &ObjectProperty {
        match self {
            ObjectPropertyExpression::ObjectProperty(property) => property,
            ObjectPropertyExpression::InverseObjectProperty(property) => property,
            ObjectPropertyExpression::PropertyChain(chain) => {
                // Return the first property in the chain
                if let Some(first_property) = chain.first() {
                    first_property.get_named_property()
                } else {
                    // This should not happen in a well-formed property chain
                    panic!("Empty property chain")
                }
            }
        }
    }

    /// Simplify the property expression
    pub fn simplify(&self) -> ObjectPropertyExpression {
        match self {
            ObjectPropertyExpression::ObjectProperty(property) => ObjectPropertyExpression::ObjectProperty(property.clone()),
            ObjectPropertyExpression::InverseObjectProperty(property) => ObjectPropertyExpression::InverseObjectProperty(property.clone()),
            ObjectPropertyExpression::PropertyChain(chain) => {
                // Simplify each property in the chain
                let simplified_chain: Vec<ObjectProperty> = chain
                    .iter()
                    .map(|p| p.simlify())
                    .collect();
                ObjectPropertyExpression::PropertyChain(simplified_chain)
            }
        }
    }
}

impl Role {
    /// Create a new role for an object property expression
    pub fn new_object_property(property: ObjectPropertyExpression) -> Self {
        Role::ObjectProperty(property)
    }

    /// Create a new role for a data property
    pub fn new_data_property(property: DataProperty) -> Self {
        Role::DataProperty(property)
    }


    /// Check if the role is an object property
    pub fn is_object_property(&self) -> bool {
        matches!(self, Role::ObjectProperty(_))
    }

    /// Check if the role is a data property
    pub fn is_data_property(&self) -> bool {
        matches!(self, Role::DataProperty(_))
    }

    /// Get the object property expression if the role is an object property
    pub fn as_object_property(&self) -> Option<&ObjectPropertyExpression> {
        if let Role::ObjectProperty(property) = self {
            Some(property)
        } else {
            None
        }
    }

    /// Get the data property if the role is a data property
    pub fn as_data_property(&self) -> Option<&DataProperty> {
        if let Role::DataProperty(property) = self {
            Some(property)
        } else {
            None
        }
    }
}

/// Object property characteristics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectPropertyCharacteristics {
    pub functional: bool,
    pub inverse_functional: bool,
    pub symmetric: bool,
    pub asymmetric: bool,
    pub reflexive: bool,
    pub irreflexive: bool,
    pub transitive: bool,
}

impl ObjectPropertyCharacteristics {
    /// Create a new set of object property characteristics
    pub fn new() -> Self {
        Self {
            functional: false,
            inverse_functional: false,
            symmetric: false,
            asymmetric: false,
            reflexive: false,
            irreflexive: false,
            transitive: false,
        }
    }

    /// Set the functional characteristic
    pub fn set_functional(&mut self, value: bool) {
        self.functional = value;
    }

    /// Set the inverse functional characteristic
    pub fn set_inverse_functional(&mut self, value: bool) {
        self.inverse_functional = value;
    }

    /// Set the symmetric characteristic
    pub fn set_symmetric(&mut self, value: bool) {
        self.symmetric = value;
    }

    /// Set the asymmetric characteristic
    pub fn set_asymmetric(&mut self, value: bool) {
        self.asymmetric = value;
    }

    /// Set the reflexive characteristic
    pub fn set_reflexive(&mut self, value: bool) {
        self.reflexive = value;
    }

    /// Set the irreflexive characteristic
    pub fn set_irreflexive(&mut self, value: bool) {
        self.irreflexive = value;
    }

    /// Set the transitive characteristic
    pub fn set_transitive(&mut self, value: bool) {
        self.transitive = value;
    }

    /// Check if the characteristics are consistent
    pub fn is_consistent(&self) -> bool {
        // Check for contradictions
        if self.functional && self.inverse_functional {
            return false; // Cannot be both functional and inverse functional
        }
        if self.symmetric && self.asymmetric {
            return false; // Cannot be both symmetric and asymmetric
        }
        if self.reflexive && self.irreflexive {
            return false; // Cannot be both reflexive and irreflexive
        }
        true
    }
}

impl Default for ObjectPropertyCharacteristics {
    fn default() -> Self {
        Self::new()
    }
}


/// Data property characteristics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPropertyCharacteristics {
    pub functional: bool,
}

impl DataPropertyCharacteristics {
    /// Create a new set of data property characteristics
    pub fn new() -> Self {
        Self {
            functional: false,
        }
    }

    /// Set the functional characteristic
    pub fn set_functional(&mut self, value: bool) {
        self.functional = value;
    }
}

/// Property hierarchy
#[derive(Debug, Clone)]
pub struct ObjectPropertyHierarchy {
    properties: HashMap<crate::ontology::IRI, ObjectProperty>,
    sub_properties: HashMap<crate::ontology::IRI, HashSet<crate::ontology::IRI>>,
    super_properties: HashMap<crate::ontology::IRI, HashSet<crate::ontology::IRI>>,
    characteristics: HashMap<crate::ontology::IRI, ObjectPropertyCharacteristics>,
    equivalent_properties: HashMap<crate::ontology::IRI, HashSet<crate::ontology::IRI>>,
    disjoint_properties: HashMap<crate::ontology::IRI, HashSet<crate::ontology::IRI>>,
    inverse_properties: HashMap<crate::ontology::IRI, crate::ontology::IRI>,
    domains: HashMap<crate::ontology::IRI, HashSet<crate::ontology::ClassExpression>>,
    ranges: HashMap<crate::ontology::IRI, HashSet<crate::ontology::ClassExpression>>,
}

impl ObjectPropertyHierarchy {
    /// Create a new empty object property hierarchy
    pub fn new() -> Self {
        let mut hierarchy = Self {
            properties: HashMap::new(),
            sub_properties: HashMap::new(),
            super_properties: HashMap::new(),
            characteristics: HashMap::new(),
            equivalent_properties: HashMap::new(),
            disjoint_properties: HashMap::new(),
            inverse_properties: HashMap::new(),
            domains: HashMap::new(),
            ranges: HashMap::new(),
        };

        // Add built-in properties
        hierarchy.add_property(ObjectProperty::top().expect("Built-in top property should be valid"));
        hierarchy.add_property(ObjectProperty::bottom().expect("Built-in bottom property should be valid"));
    }

    /// Add an object property to the hierarchy
    pub fn add_property(&mut self, property: ObjectProperty) -> &ObjectProperty {
        let iri = crate::ontology::IRI::new(property.iri.to_string());
        self.properties.entry(iri.clone()).or_insert_with(|| {
            self.characteristics.insert(iri.clone(), ObjectPropertyCharacteristics::new());
            property
        })
    }

    pub fn get_property(&self, iri: &crate::ontology::IRI) -> Option<&ObjectProperty> {
        self.properties.get(iri)
    }

    pub fn add_sub_property(&mut setf, sub: &crate::ontology::IRI, super_prop: &crate::ontology::IRI){
        sub.sub_properties
            .entry(super_prop.clone())
            .or_insert_with(HashSet::new)
            .insert(sub.clone());
        self.super_properties
            .entry(sub.clone())
            .or_insert_with(HashSet::new)
            .insert(super_prop.clone());
    }

    pub fn get_sub_properties(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::IRI>> {
        self.sub_properties.get(iri)
    }

    pub fn get_super_properties(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::IRI>> {
        self.super_properties.get(iri)
    }

    pub fn get_characteristics(&self, iri: &crate::ontology::IRI) -> Option<&ObjectPropertyCharacteristics> {
        self.characteristics.get(iri)
    }

    pub fn add_equivalent_property(&mut self, property: &crate::ontology::IRI, equivalent: &crate::ontology::IRI) {
        self.equivalent_properties
            .entry(property.clone())
            .or_insert_with(HashSet::new)
            .insert(equivalent.clone());
        self.equivalent_properties
            .entry(equivalent.clone())
            .or_insert_with(HashSet::new)
            .insert(property.clone());
    }

    pub fn get_equivalent_properties(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::IRI>> {
        self.equivalent_properties.get(iri)
    }

    pub fn add_disjoint_property(&mut self, property: &crate::ontology::IRI, disjoint: &crate::ontology::IRI) {
        self.disjoint_properties
            .entry(property.clone())
            .or_insert_with(HashSet::new)
            .insert(disjoint.clone());
        self.disjoint_properties
            .entry(disjoint.clone())
            .or_insert_with(HashSet::new)
            .insert(property.clone());
    }

    pub fn get_disjoint_properties(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::IRI>> {
        self.disjoint_properties.get(iri)
    }

    pub fn set_inverse_functional(&mut self, property: &crate::ontology::IRI, inverse: &crate::ontology::IRI) {
        self.inverse_properties.insert(property.clone(), inverse.clone());
        self.inverse_properties.insert(inverse.clone(), property.clone());
    }

    pub fn get_inverse_property(&self, iri: &crate::ontology::IRI) -> Option<&crate::ontology::IRI> {
        self.inverse_properties.get(iri)
    }

    pub fn add_domain(&mut self, property: &crate::ontology::IRI, domain: crate::ontology::ClassExpression) {
        self.domains
            .entry(property.clone())
            .or_insert_with(HashSet::new)
            .insert(domain);
    }

    pub fn get_domains(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::ClassExpression>> {
        self.domains.get(iri)
    }

    pub fn add_range(&mut self, property: &crate::ontology::IRI, range: crate::ontology::ClassExpression) {
        self.ranges
            .entry(property.clone())
            .or_insert_with(HashSet::new)
            .insert(range);
    }

    pub fn get_ranges(&self, iri: &crate::ontology::IRI) -> Option<&HashSet<crate::ontology::ClassExpression>> {
        self.ranges.get(iri)
    }

    pub fn all_properties(&self) -> impl Iterator<Item = &ObjectProperty> {
        self.properties.values()
    }

    pub fn is_functional(&self, property: &crate::ontology::IRI) -> bool {
        self.chainistics
            .get(property)
            .map(|c| c.functional)
            .unwrap_or(false)
    }

    pub fn is_inverse_functional(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.inverse_functional)
            .unwrap_or(false)
    }

    pub fn is_symmetric(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.symmetric)
            .unwrap_or(false)
    }

    pub fn is_asymmetric(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.asymmetric)
            .unwrap_or(false)
    }

    pub fn is_reflexive(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.reflexive)
            .unwrap_or(false)
    }

    pub fn is_irreflexive(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.irreflexive)
            .unwrap_or(false)
    }

    pub fn is_transitive(&self, property: &crate::ontology::IRI) -> bool {
        self.characteristics
            .get(property)
            .map(|c| c.transitive)
            .unwrap_or(false)
    }
}

impl Default for ObjectPropertyHierarchy {
    fn default() -> Self {
        Self::new()
    }
}
