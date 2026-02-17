//! Interpretation Framework for Semantic Models
//!
//! This module provides a framework for creating and working with
//! interpretations according to RDF, RDFS, and OWL 2 semantics.

use super::{RdfGraph, RdfTerm, SemanticInterpretation, Triple};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

/// Abstract interpretation structure
///
/// Provides a general framework for semantic interpretations
/// that can be specialized for different logics (RDF, RDFS, OWL).
#[derive(Debug, Clone)]
pub struct Interpretation {
    /// Domain of interpretation (universe of discourse)
    domain: HashSet<String>,
    /// Interpretation of IRIs to domain elements
    iri_interpretation: HashMap<String, String>,
    /// Interpretation of blank nodes to domain elements
    blank_node_interpretation: HashMap<String, String>,
    /// Interpretation of properties as binary relations
    property_interpretation: HashMap<String, HashSet<(String, String)>>,
    /// Interpretation of classes as sets of domain elements
    class_interpretation: HashMap<String, HashSet<String>>,
    /// Interpretation of literals to domain elements
    literal_interpretation: HashMap<String, String>,
    /// Interpretation of datatypes
    datatype_interpretation: HashMap<String, HashSet<String>>,
}

impl Interpretation {
    /// Create a new empty interpretation
    pub fn new() -> Self {
        Self {
            domain: HashSet::new(),
            iri_interpretation: HashMap::new(),
            blank_node_interpretation: HashMap::new(),
            property_interpretation: HashMap::new(),
            class_interpretation: HashMap::new(),
            literal_interpretation: HashMap::new(),
            datatype_interpretation: HashMap::new(),
        }
    }

    /// Set the domain of interpretation
    pub fn set_domain(&mut self, domain: HashSet<String>) {
        self.domain = domain;
    }

    /// Get the domain of interpretation
    pub fn domain(&self) -> &HashSet<String> {
        &self.domain
    }

    /// Add an element to the domain
    pub fn add_to_domain(&mut self, element: String) {
        self.domain.insert(element);
    }

    /// Set interpretation for an IRI
    pub fn set_iri_interpretation(&mut self, iri: String, domain_element: String) {
        self.iri_interpretation.insert(iri, domain_element);
    }

    /// Get interpretation for an IRI
    pub fn get_iri_interpretation(&self, iri: &str) -> Option<&String> {
        self.iri_interpretation.get(iri)
    }

    /// Set interpretation for a blank node
    pub fn set_blank_node_interpretation(&mut self, blank_node: String, domain_element: String) {
        self.blank_node_interpretation
            .insert(blank_node, domain_element);
    }

    /// Get interpretation for a blank node
    pub fn get_blank_node_interpretation(&self, blank_node: &str) -> Option<&String> {
        self.blank_node_interpretation.get(blank_node)
    }

    /// Set property interpretation
    pub fn set_property_interpretation(
        &mut self,
        property: String,
        relations: HashSet<(String, String)>,
    ) {
        self.property_interpretation.insert(property, relations);
    }

    /// Get property interpretation
    pub fn get_property_interpretation(
        &self,
        property: &str,
    ) -> Option<&HashSet<(String, String)>> {
        self.property_interpretation.get(property)
    }

    /// Add a relation to a property interpretation
    pub fn add_property_relation(&mut self, property: String, subject: String, object: String) {
        self.property_interpretation
            .entry(property)
            .or_default()
            .insert((subject, object));
    }

    /// Set class interpretation
    pub fn set_class_interpretation(&mut self, class: String, instances: HashSet<String>) {
        self.class_interpretation.insert(class, instances);
    }

    /// Get class interpretation
    pub fn get_class_interpretation(&self, class: &str) -> Option<&HashSet<String>> {
        self.class_interpretation.get(class)
    }

    /// Add an instance to a class interpretation
    pub fn add_class_instance(&mut self, class: String, instance: String) {
        self.class_interpretation
            .entry(class)
            .or_default()
            .insert(instance);
    }

    /// Set literal interpretation
    pub fn set_literal_interpretation(&mut self, literal: String, domain_element: String) {
        self.literal_interpretation.insert(literal, domain_element);
    }

    /// Get literal interpretation
    pub fn get_literal_interpretation(&self, literal: &str) -> Option<&String> {
        self.literal_interpretation.get(literal)
    }

    /// Set datatype interpretation
    pub fn set_datatype_interpretation(&mut self, datatype: String, value_space: HashSet<String>) {
        self.datatype_interpretation.insert(datatype, value_space);
    }

    /// Get datatype interpretation
    pub fn get_datatype_interpretation(&self, datatype: &str) -> Option<&HashSet<String>> {
        self.datatype_interpretation.get(datatype)
    }

    /// Interpret an RDF term in this interpretation
    pub fn interpret_rdf_term(&self, term: &RdfTerm) -> Option<String> {
        match term {
            RdfTerm::Iri(iri) => {
                self.iri_interpretation
                    .get(&iri.to_string())
                    .cloned()
                    .or_else(|| Some(iri.to_string())) // Default to self-interpretation
            }
            RdfTerm::BlankNode(id) => self.blank_node_interpretation.get(id).cloned(),
            RdfTerm::Literal {
                value,
                datatype,
                language,
                ..
            } => {
                let literal_key = if let Some(dt) = datatype {
                    format!("{}^^{}", value, dt)
                } else if let Some(lang) = language {
                    format!("{}@{}", value, lang)
                } else {
                    value.clone()
                };

                self.literal_interpretation
                    .get(&literal_key)
                    .cloned()
                    .or(Some(literal_key)) // Default interpretation
            }
            RdfTerm::QuotedTriple(triple) => {
                // RDF-star: quoted triples interpreted as resources
                let triple_id = format!("<<{}>>", triple);
                Some(triple_id)
            }
        }
    }

    /// Check if a triple is satisfied by this interpretation
    pub fn satisfies_triple(&self, triple: &Triple) -> bool {
        let subject_interp = self.interpret_rdf_term(&triple.subject);
        let predicate_interp = self.interpret_rdf_term(&triple.predicate);
        let object_interp = self.interpret_rdf_term(&triple.object);

        if let (Some(s), Some(p), Some(o)) = (subject_interp, predicate_interp, object_interp) {
            // Check if the property interpretation contains the (subject, object) pair
            if let Some(prop_relations) = self.property_interpretation.get(&p) {
                prop_relations.contains(&(s, o))
            } else {
                // If property is not explicitly interpreted, use default behavior
                // In a complete implementation, this might involve type checking
                true
            }
        } else {
            false
        }
    }

    /// Check if this interpretation is well-formed
    pub fn is_well_formed(&self) -> Result<bool> {
        // Check that all interpretations map to domain elements
        for domain_element in self.iri_interpretation.values() {
            if !self.domain.contains(domain_element) {
                return Ok(false);
            }
        }

        for domain_element in self.blank_node_interpretation.values() {
            if !self.domain.contains(domain_element) {
                return Ok(false);
            }
        }

        // Check property interpretations
        for relations in self.property_interpretation.values() {
            for (subject, object) in relations {
                if !self.domain.contains(subject) || !self.domain.contains(object) {
                    return Ok(false);
                }
            }
        }

        // Check class interpretations
        for instances in self.class_interpretation.values() {
            for instance in instances {
                if !self.domain.contains(instance) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Create a model-theoretic interpretation from an RDF graph
    pub fn from_rdf_graph(graph: &RdfGraph) -> Result<Self> {
        let mut interpretation = Self::new();

        // Extract all terms from the graph
        let mut all_terms = HashSet::new();
        for triple in graph.triples() {
            all_terms.insert(&triple.subject);
            all_terms.insert(&triple.predicate);
            all_terms.insert(&triple.object);
        }

        // Build domain from non-literal terms
        let mut domain = HashSet::new();
        let mut term_counter = 0;

        for term in &all_terms {
            if !term.is_literal() {
                let domain_element = format!("d{}", term_counter);
                domain.insert(domain_element.clone());

                match term {
                    RdfTerm::Iri(iri) => {
                        interpretation.set_iri_interpretation(iri.to_string(), domain_element);
                    }
                    RdfTerm::BlankNode(id) => {
                        interpretation.set_blank_node_interpretation(id.clone(), domain_element);
                    }
                    _ => {}
                }

                term_counter += 1;
            }
        }

        interpretation.set_domain(domain);

        // Interpret properties based on triples
        for triple in graph.triples() {
            if let (Some(s), Some(p), Some(o)) = (
                interpretation.interpret_rdf_term(&triple.subject),
                interpretation.interpret_rdf_term(&triple.predicate),
                interpretation.interpret_rdf_term(&triple.object),
            ) {
                interpretation.add_property_relation(p, s, o);
            }
        }

        Ok(interpretation)
    }

    /// Merge another interpretation into this one
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        // Merge domains
        self.domain.extend(other.domain.iter().cloned());

        // Merge IRI interpretations (checking for conflicts)
        for (iri, interpretation) in &other.iri_interpretation {
            if let Some(existing) = self.iri_interpretation.get(iri) {
                if existing != interpretation {
                    return Err(Error::reasoning(format!(
                        "Conflicting IRI interpretation for {}",
                        iri
                    )));
                }
            } else {
                self.iri_interpretation
                    .insert(iri.clone(), interpretation.clone());
            }
        }

        // Merge blank node interpretations
        for (blank_node, interpretation) in &other.blank_node_interpretation {
            self.blank_node_interpretation
                .insert(blank_node.clone(), interpretation.clone());
        }

        // Merge property interpretations
        for (property, relations) in &other.property_interpretation {
            self.property_interpretation
                .entry(property.clone())
                .or_default()
                .extend(relations.iter().cloned());
        }

        // Merge class interpretations
        for (class, instances) in &other.class_interpretation {
            self.class_interpretation
                .entry(class.clone())
                .or_default()
                .extend(instances.iter().cloned());
        }

        // Merge literal interpretations
        for (literal, interpretation) in &other.literal_interpretation {
            self.literal_interpretation
                .insert(literal.clone(), interpretation.clone());
        }

        // Merge datatype interpretations
        for (datatype, value_space) in &other.datatype_interpretation {
            self.datatype_interpretation
                .entry(datatype.clone())
                .or_default()
                .extend(value_space.iter().cloned());
        }

        Ok(())
    }
}

impl Default for Interpretation {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticInterpretation for Interpretation {
    fn satisfies(&self, graph: &RdfGraph) -> bool {
        // An interpretation satisfies a graph if it satisfies all triples
        graph
            .triples()
            .iter()
            .all(|triple| self.satisfies_triple(triple))
    }

    fn interpret_term(&self, term: &RdfTerm) -> Option<String> {
        self.interpret_rdf_term(term)
    }

    fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // For model-theoretic entailment: premises entail conclusion if every
        // interpretation that satisfies premises also satisfies conclusion
        if self.satisfies(premises) {
            self.satisfies(conclusion)
        } else {
            true // Vacuously true if premises are not satisfied
        }
    }
}

/// Interpretation builder for constructing interpretations step by step
#[derive(Debug)]
pub struct InterpretationBuilder {
    interpretation: Interpretation,
}

impl InterpretationBuilder {
    /// Create a new interpretation builder
    pub fn new() -> Self {
        Self {
            interpretation: Interpretation::new(),
        }
    }

    /// Set the domain
    pub fn with_domain(mut self, domain: HashSet<String>) -> Self {
        self.interpretation.set_domain(domain);
        self
    }

    /// Add an IRI interpretation
    pub fn with_iri(mut self, iri: String, domain_element: String) -> Self {
        self.interpretation
            .set_iri_interpretation(iri, domain_element);
        self
    }

    /// Add a property interpretation
    pub fn with_property(mut self, property: String, relations: HashSet<(String, String)>) -> Self {
        self.interpretation
            .set_property_interpretation(property, relations);
        self
    }

    /// Add a class interpretation
    pub fn with_class(mut self, class: String, instances: HashSet<String>) -> Self {
        self.interpretation
            .set_class_interpretation(class, instances);
        self
    }

    /// Build the interpretation
    pub fn build(self) -> Result<Interpretation> {
        // Validate the interpretation
        if !self.interpretation.is_well_formed()? {
            return Err(Error::reasoning(
                "Interpretation is not well-formed".to_string(),
            ));
        }

        Ok(self.interpretation)
    }
}

impl Default for InterpretationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpretation factory for creating standard interpretations
pub struct InterpretationFactory;

impl InterpretationFactory {
    /// Create a minimal interpretation for RDF
    pub fn create_minimal_rdf_interpretation() -> Interpretation {
        let mut interpretation = Interpretation::new();

        // Add minimal domain
        let mut domain = HashSet::new();
        domain.insert("resource1".to_string());
        interpretation.set_domain(domain);

        interpretation
    }

    /// Create a standard RDFS interpretation
    pub fn create_rdfs_interpretation() -> Interpretation {
        let mut interpretation = Self::create_minimal_rdf_interpretation();

        // Add RDFS vocabulary interpretations
        use super::vocabulary::*;

        let mut rdfs_resource_instances = HashSet::new();
        rdfs_resource_instances.insert("resource1".to_string());
        interpretation.set_class_interpretation(RDFS_RESOURCE.to_string(), rdfs_resource_instances);

        let rdfs_class_instances = HashSet::new();
        interpretation.set_class_interpretation(RDFS_CLASS.to_string(), rdfs_class_instances);

        interpretation
    }

    /// Create a standard OWL interpretation
    pub fn create_owl_interpretation() -> Interpretation {
        let mut interpretation = Self::create_rdfs_interpretation();

        // Add OWL vocabulary interpretations
        use super::vocabulary::*;

        // owl:Thing contains all domain elements
        interpretation
            .set_class_interpretation(OWL_THING.to_string(), interpretation.domain().clone());

        // owl:Nothing is empty
        interpretation.set_class_interpretation(OWL_NOTHING.to_string(), HashSet::new());

        interpretation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpretation_creation() {
        let mut interpretation = Interpretation::new();

        let mut domain = HashSet::new();
        domain.insert("d1".to_string());
        domain.insert("d2".to_string());
        interpretation.set_domain(domain);

        interpretation
            .set_iri_interpretation("http://example.org/test".to_string(), "d1".to_string());

        assert_eq!(interpretation.domain().len(), 2);
        assert_eq!(
            interpretation.get_iri_interpretation("http://example.org/test"),
            Some(&"d1".to_string())
        );
    }

    #[test]
    fn test_interpretation_builder() {
        let mut domain = HashSet::new();
        domain.insert("d1".to_string());

        let interpretation = InterpretationBuilder::new()
            .with_domain(domain)
            .with_iri("http://example.org/test".to_string(), "d1".to_string())
            .build()
            .expect("Failed to build RDF interpretation from builder");

        assert!(
            interpretation
                .is_well_formed()
                .expect("Failed to check if RDF interpretation is well-formed")
        );
    }

    #[test]
    fn test_triple_satisfaction() {
        let mut interpretation = Interpretation::new();

        let mut domain = HashSet::new();
        domain.insert("d1".to_string());
        domain.insert("d2".to_string());
        interpretation.set_domain(domain);

        interpretation
            .set_iri_interpretation("http://example.org/subject".to_string(), "d1".to_string());
        interpretation.set_iri_interpretation(
            "http://example.org/predicate".to_string(),
            "pred".to_string(),
        );
        interpretation
            .set_iri_interpretation("http://example.org/object".to_string(), "d2".to_string());

        let mut relations = HashSet::new();
        relations.insert(("d1".to_string(), "d2".to_string()));
        interpretation.set_property_interpretation("pred".to_string(), relations);

        let subject = RdfTerm::iri("http://example.org/subject")
            .expect("Failed to create RDF IRI term from valid URI string");
        let predicate = RdfTerm::iri("http://example.org/predicate")
            .expect("Failed to create RDF IRI term from valid URI string");
        let object = RdfTerm::iri("http://example.org/object")
            .expect("Failed to create RDF IRI term from valid URI string");

        let triple = Triple {
            subject,
            predicate,
            object,
        };

        assert!(interpretation.satisfies_triple(&triple));
    }

    #[test]
    fn test_interpretation_factory() {
        let rdf_interp = InterpretationFactory::create_minimal_rdf_interpretation();
        assert!(!rdf_interp.domain().is_empty());

        let rdfs_interp = InterpretationFactory::create_rdfs_interpretation();
        assert!(
            rdfs_interp
                .get_class_interpretation(&super::super::vocabulary::RDFS_RESOURCE.to_string())
                .is_some()
        );

        let owl_interp = InterpretationFactory::create_owl_interpretation();
        assert!(
            owl_interp
                .get_class_interpretation(&super::super::vocabulary::OWL_THING.to_string())
                .is_some()
        );
    }
}
