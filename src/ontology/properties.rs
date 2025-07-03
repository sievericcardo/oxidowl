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
