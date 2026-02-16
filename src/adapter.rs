//! Adapter for converting between horned-owl and oxidowl representations
//!
//! This module provides bidirectional conversion between the horned-owl OWL 2 model
//! and oxidowl's extended model that supports RDF-star (RDF 1.2) features.
//!
//! # RDF-star Support
//!
//! The adapter handles quoted triples (RDF-star) through automatic reification when
//! converting to horned-owl (which doesn't support RDF-star natively). This ensures
//! compatibility with existing OWL 2 tools while preserving the semantics of quoted triples.
//!
//! ## Reification
//!
//! When converting a quoted triple `<< :s :p :o >>` to RDF 1.1, it becomes:
//!
//! ```text
//! _:bn rdf:type rdf:Statement .
//! _:bn rdf:subject :s .
//! _:bn rdf:predicate :p .
//! _:bn rdf:object :o .
//! ```
//!
//! This allows RDF-star statements to be represented in RDF 1.1-compatible form.
//!
//! ## RDF Compatibility Modes
//!
//! The adapter supports two modes:
//!
//! - **RDF 1.1 Mode (default)**: Automatically reifies quoted triples for maximum compatibility
//! - **RDF 1.2 Mode**: Preserves quoted triples natively (requires RDF-star-aware tools)
//!
//! # Example Usage
//!
//! ```rust
//! use oxidowl::adapter::{HornedOwlAdapter};
//! use oxidowl::semantics::{RdfTerm, Triple};
//!
//! # fn example() -> oxidowl::Result<()> {
//! // Create adapter in RDF 1.1 compatibility mode
//! let mut adapter = HornedOwlAdapter::new();
//!
//! // Create a quoted triple: << :alice :knows :bob >>
//! let alice = RdfTerm::iri("http://example.org/alice")?;
//! let knows = RdfTerm::iri("http://example.org/knows")?;
//! let bob = RdfTerm::iri("http://example.org/bob")?;
//!
//! let inner_triple = Triple::new(alice, knows, bob);
//! let quoted_triple = RdfTerm::QuotedTriple(Box::new(inner_triple));
//!
//! // Convert to RDF 1.1 (automatically reified)
//! let (reified_term, reification_triples) = adapter.reify_rdf_term(&quoted_triple)?;
//!
//! // reified_term is now a blank node
//! // reification_triples contains 4 triples expressing the reification
//!
//! // Create a statement about the quoted triple:
//! // << :alice :knows :bob >> :certainty 0.95
//! let certainty = RdfTerm::iri("http://example.org/certainty")?;
//! let value = RdfTerm::Literal {
//!     value: "0.95".to_string(),
//!     datatype: None,
//!     language: None,
//!     direction: None,
//! };
//!
//! let meta_triple = Triple::new(quoted_triple, certainty, value);
//!
//! // Convert to RDF 1.1
//! let rdf11_triples = adapter.convert_triple_to_rdf11(&meta_triple)?;
//!
//! // rdf11_triples now contains:
//! // 1. _:bn :certainty 0.95  (the main triple with blank node)
//! // 2-5. Four reification triples
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Nested Quoted Triples
//!
//! The adapter supports arbitrary nesting of quoted triples. For example:
//!
//! ```text
//! << << :alice :knows :bob >> :certainty 0.95 >> :source :survey2023
//! ```
//!
//! This is recursively reified, with each level creating its own blank node and
//! reification triples.
//!
//! # Future Work
//!
//! - **Dereification**: Converting RDF 1.1 reification patterns back to quoted triples
//! - **Performance optimization**: Caching reification patterns for repeated conversions
//! - **Full horned-owl integration**: Complete bidirectional conversion of complex axioms

use crate::Result;
use crate::ontology::{Class, IRI, axioms::AxiomId};
use crate::semantics::{RdfTerm, Triple};
use horned_owl::model::MutableOntology;
use std::collections::HashMap;

#[cfg(test)]
use url::Url;

/// Trait for components that support RDF-star features
/// 
/// This trait marks components that can handle quoted triples and other RDF-star
/// constructs natively. Components that don't implement this trait should receive
/// RDF 1.1-compatible data (with quoted triples reified).
pub trait RdfStarCapable {
    /// Returns true if this component supports RDF-star natively
    fn supports_rdf_star(&self) -> bool;
    
    /// Returns the maximum nesting depth supported (0 = no nesting)
    fn max_nesting_depth(&self) -> usize;
}

/// Adapter for converting between horned-owl and oxidowl representations
/// 
/// This adapter handles bidirectional conversion between horned-owl's OWL 2 model
/// and oxidowl's extended model that supports RDF-star features. When converting
/// to horned-owl (which doesn't support RDF-star), quoted triples are automatically
/// reified using standard RDF reification vocabulary.
pub struct HornedOwlAdapter {
    iri_cache: HashMap<String, IRI>,
    axiom_counter: u64,
    /// Counter for generating unique blank node identifiers during reification
    blank_node_counter: u64,
    /// If true, automatically reify quoted triples when converting to horned-owl
    /// If false, attempt to preserve quoted triples (may fail if target doesn't support)
    rdf11_mode: bool,
}

impl Default for HornedOwlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl HornedOwlAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            iri_cache: HashMap::new(),
            axiom_counter: 0,
            blank_node_counter: 0,
            rdf11_mode: true, // Default to RDF 1.1 compatibility mode
        }
    }

    /// Create a new adapter with specific RDF mode
    /// 
    /// # Arguments
    /// * `rdf11_mode` - If true, automatically reify quoted triples for RDF 1.1 compatibility
    #[must_use]
    pub fn with_rdf_mode(rdf11_mode: bool) -> Self {
        Self {
            iri_cache: HashMap::new(),
            axiom_counter: 0,
            blank_node_counter: 0,
            rdf11_mode,
        }
    }

    /// Set RDF compatibility mode
    pub fn set_rdf11_mode(&mut self, enabled: bool) {
        self.rdf11_mode = enabled;
    }

    /// Check if adapter is in RDF 1.1 compatibility mode
    pub fn is_rdf11_mode(&self) -> bool {
        self.rdf11_mode
    }

    /// Generate a unique blank node identifier for reification
    fn next_blank_node_id(&mut self) -> String {
        self.blank_node_counter += 1;
        format!("_:reify{}", self.blank_node_counter)
    }

    fn next_axiom_id(&mut self) -> AxiomId {
        self.axiom_counter += 1;
        self.axiom_counter
    }

    /// Convert horned-owl IRI to oxidowl IRI
    pub fn convert_iri(&mut self, horned_iri: &horned_owl::model::IRI<String>) -> Result<IRI> {
        let iri_string = horned_iri.to_string();

        // Check cache first
        if let Some(cached_iri) = self.iri_cache.get(&iri_string) {
            return Ok(cached_iri.clone());
        }

        let oxidowl_iri = IRI::new(&iri_string);
        self.iri_cache.insert(iri_string, oxidowl_iri.clone());
        Ok(oxidowl_iri)
    }

    /// Convert horned-owl Class to oxidowl Class
    pub fn convert_class(
        &mut self,
        horned_class: &horned_owl::model::Class<String>,
    ) -> Result<Class> {
        let iri = self.convert_iri(&horned_class.0)?;
        Ok(Class::new(iri))
    }

    /// Convert oxidowl RdfTerm to a set of RDF 1.1-compatible triples
    /// 
    /// This method handles RDF-star quoted triples by reifying them according to
    /// the RDF reification vocabulary. Each quoted triple becomes a blank node
    /// with rdf:type rdf:Statement and rdf:subject, rdf:predicate, rdf:object properties.
    /// 
    /// # Arguments
    /// * `term` - The RdfTerm to convert (may contain nested quoted triples)
    /// 
    /// # Returns
    /// A tuple of (converted_term, additional_triples) where:
    /// - converted_term is the RDF 1.1-compatible term (quoted triples become blank nodes)
    /// - additional_triples are the reification triples to add to the graph
    pub fn reify_rdf_term(&mut self, term: &RdfTerm) -> Result<(RdfTerm, Vec<Triple>)> {
        match term {
            RdfTerm::QuotedTriple(triple) => {
                // Generate a blank node to represent this quoted triple
                let blank_node_id = self.next_blank_node_id();
                let blank_node = RdfTerm::BlankNode(blank_node_id.clone());
                
                // Recursively reify the subject, predicate, and object
                let (reified_subject, mut subject_triples) = self.reify_rdf_term(&triple.subject)?;
                let (reified_predicate, mut predicate_triples) = self.reify_rdf_term(&triple.predicate)?;
                let (reified_object, mut object_triples) = self.reify_rdf_term(&triple.object)?;
                
                // Create reification triples using RDF vocabulary
                let rdf_type = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
                let rdf_statement = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement")?;
                let rdf_subject = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#subject")?;
                let rdf_predicate = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate")?;
                let rdf_object = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#object")?;
                
                let mut reification_triples = vec![
                    // _:bn rdf:type rdf:Statement
                    Triple::new(blank_node.clone(), rdf_type, rdf_statement),
                    // _:bn rdf:subject <subject>
                    Triple::new(blank_node.clone(), rdf_subject, reified_subject),
                    // _:bn rdf:predicate <predicate>
                    Triple::new(blank_node.clone(), rdf_predicate, reified_predicate),
                    // _:bn rdf:object <object>
                    Triple::new(blank_node.clone(), rdf_object, reified_object),
                ];
                
                // Add any triples from recursive reification
                reification_triples.append(&mut subject_triples);
                reification_triples.append(&mut predicate_triples);
                reification_triples.append(&mut object_triples);
                
                Ok((blank_node, reification_triples))
            }
            // Non-quoted terms are returned as-is
            _ => Ok((term.clone(), vec![])),
        }
    }

    /// Convert an oxidowl Triple to RDF 1.1-compatible triples
    /// 
    /// If the triple contains quoted triples, they are reified according to
    /// RDF reification vocabulary. The returned vector contains the main triple
    /// (with quoted triples replaced by blank nodes) plus all reification triples.
    /// 
    /// # Arguments
    /// * `triple` - The triple to convert
    /// 
    /// # Returns
    /// A vector of RDF 1.1-compatible triples
    pub fn convert_triple_to_rdf11(&mut self, triple: &Triple) -> Result<Vec<Triple>> {
        if !self.rdf11_mode {
            // In RDF 1.2 mode, return the triple as-is
            return Ok(vec![triple.clone()]);
        }
        
        // Reify each position if it contains quoted triples
        let (reified_subject, mut subject_triples) = self.reify_rdf_term(&triple.subject)?;
        let (reified_predicate, mut predicate_triples) = self.reify_rdf_term(&triple.predicate)?;
        let (reified_object, mut object_triples) = self.reify_rdf_term(&triple.object)?;
        
        // Create the main triple with reified terms
        let main_triple = Triple::new(reified_subject, reified_predicate, reified_object);
        
        // Combine all triples
        let mut all_triples = vec![main_triple];
        all_triples.append(&mut subject_triples);
        all_triples.append(&mut predicate_triples);
        all_triples.append(&mut object_triples);
        
        Ok(all_triples)
    }

    /// Detect reified triples in RDF 1.1 data and convert to quoted triples
    /// 
    /// This is the inverse of reify_rdf_term. It scans for patterns like:
    /// ```
    /// _:bn rdf:type rdf:Statement .
    /// _:bn rdf:subject <s> .
    /// _:bn rdf:predicate <p> .
    /// _:bn rdf:object <o> .
    /// ```
    /// 
    /// And converts them back to `<< <s> <p> <o> >>` quoted triple syntax.
    /// 
    /// # Arguments
    /// * `triples` - Set of triples that may contain reification patterns
    /// 
    /// # Returns
    /// A new set of triples with reifications converted to quoted triples
    pub fn dereify_triples(&self, triples: &[Triple]) -> Result<Vec<Triple>> {
        use std::collections::{HashMap, HashSet};
        
        // RDF vocabulary IRIs - convert to Url for comparison
        let rdf_type_term = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
        let rdf_statement_term = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement")?;
        let rdf_subject_term = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#subject")?;
        let rdf_predicate_term = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate")?;
        let rdf_object_term = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#object")?;
        
        // Extract Urls for comparison
        let (rdf_type, rdf_statement, rdf_subject, rdf_predicate, rdf_object) = 
            match (&rdf_type_term, &rdf_statement_term, &rdf_subject_term, &rdf_predicate_term, &rdf_object_term) {
                (RdfTerm::Iri(t), RdfTerm::Iri(s), RdfTerm::Iri(sub), RdfTerm::Iri(pred), RdfTerm::Iri(obj)) => {
                    (t, s, sub, pred, obj)
                }
                _ => unreachable!("RdfTerm::iri always returns Iri variant"),
            };
        
        // Step 1: Find all blank nodes that are rdf:Statements
        let mut statement_nodes = HashSet::new();
        for triple in triples {
            if let (RdfTerm::BlankNode(bn), RdfTerm::Iri(pred), RdfTerm::Iri(obj)) = 
                (&triple.subject, &triple.predicate, &triple.object) {
                if pred == rdf_type && obj == rdf_statement {
                    statement_nodes.insert(bn.clone());
                }
            }
        }
        
        // Step 2: For each statement node, find its subject/predicate/object
        let mut reifications = HashMap::new();
        for bn in &statement_nodes {
            let mut subj = None;
            let mut pred = None;
            let mut obj = None;
            
            for triple in triples {
                if let RdfTerm::BlankNode(triple_bn) = &triple.subject {
                    if triple_bn == bn {
                        match &triple.predicate {
                            RdfTerm::Iri(iri) if iri == rdf_subject => {
                                subj = Some(triple.object.clone());
                            }
                            RdfTerm::Iri(iri) if iri == rdf_predicate => {
                                pred = Some(triple.object.clone());
                            }
                            RdfTerm::Iri(iri) if iri == rdf_object => {
                                obj = Some(triple.object.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            // If we found all three components, create a quoted triple
            if let (Some(s), Some(p), Some(o)) = (subj, pred, obj) {
                let quoted_triple = RdfTerm::QuotedTriple(Box::new(Triple::new(s, p, o)));
                reifications.insert(bn.clone(), quoted_triple);
            }
        }
        
        // Step 3: Remove reification pattern triples and replace blank node references
        let triples_to_skip: HashSet<_> = triples.iter()
            .filter(|triple| {
                // Skip if it's part of a reification pattern
                if let RdfTerm::BlankNode(bn) = &triple.subject {
                    if statement_nodes.contains(bn) {
                        if let RdfTerm::Iri(pred) = &triple.predicate {
                            return pred == rdf_type 
                                || pred == rdf_subject 
                                || pred == rdf_predicate 
                                || pred == rdf_object;
                        }
                    }
                }
                false
            })
            .collect();
        
        // Replace blank node references with quoted triples
        let mut result = Vec::new();
        for triple in triples {
            if !triples_to_skip.contains(triple) {
                let new_subject = match &triple.subject {
                    RdfTerm::BlankNode(bn) if reifications.contains_key(bn) => {
                        reifications[bn].clone()
                    }
                    _ => triple.subject.clone(),
                };
                
                let new_predicate = match &triple.predicate {
                    RdfTerm::BlankNode(bn) if reifications.contains_key(bn) => {
                        reifications[bn].clone()
                    }
                    _ => triple.predicate.clone(),
                };
                
                let new_object = match &triple.object {
                    RdfTerm::BlankNode(bn) if reifications.contains_key(bn) => {
                        reifications[bn].clone()
                    }
                    _ => triple.object.clone(),
                };
                
                result.push(Triple::new(new_subject, new_predicate, new_object));
            }
        }
        
        Ok(result)
    }

    /// Convert horned-owl ontology to oxidowl ontology (basic conversion)
    pub fn convert_basic_ontology<A>(
        &mut self,
        horned_ontology: &dyn std::fmt::Debug,
    ) -> Result<crate::ontology::Ontology>
    where
        A: Clone + std::fmt::Display + std::hash::Hash + Eq,
    {
        // Create a basic oxidowl ontology for now
        let mut oxidowl_ontology = crate::ontology::Ontology::new();

        // Implement basic conversion from horned-owl to oxidowl
        // Since horned-owl API is complex and we're passed a debug trait object,
        // we'll create a minimal ontology with basic structure

        // Set a default IRI if none exists
        if oxidowl_ontology.get_iri().is_none() {
            let default_iri = crate::ontology::IRI::new("http://example.org/converted-ontology");
            oxidowl_ontology.set_ontology_iri(Some(default_iri));
        }

        // Log the conversion attempt
        log::debug!("Converting horned-owl ontology: {:?}", horned_ontology);

        // For now, return the basic ontology structure
        // In a full implementation, this would parse the horned-owl structure
        // and convert axioms, entities, etc.
        Ok(oxidowl_ontology)
    }

    /// Convert horned-owl ontology with SWRL rules support
    pub fn convert_ontology_with_swrl<A>(
        &mut self,
        horned_ontology: &dyn std::fmt::Debug,
    ) -> Result<crate::ontology::Ontology>
    where
        A: Clone + std::fmt::Display + std::hash::Hash + Eq,
    {
        // Start with basic conversion
        let mut ontology = self.convert_basic_ontology::<A>(horned_ontology)?;

        // Add basic SWRL rule support structure
        // For now, we'll ensure the ontology can handle SWRL rules
        log::debug!(
            "Converting ontology with SWRL support: {:?}",
            horned_ontology
        );

        // In a full implementation, this would:
        // 1. Extract SWRL rules from horned-owl ontology
        // 2. Convert them to oxidowl SWRL representation
        // 3. Add them to the ontology

        // For now, we add a comment indicating SWRL support was attempted
        let swrl_comment = crate::ontology::Annotation {
            property: crate::ontology::AnnotationProperty {
                iri: crate::ontology::IRI::new("http://www.w3.org/2000/01/rdf-schema#comment"),
            },
            value: crate::ontology::AnnotationValue::Literal(crate::ontology::Literal::new(
                "SWRL rules conversion attempted".to_string(),
            )),
        };
        ontology.annotations.push(swrl_comment);

        Ok(ontology)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests are commented out due to horned-owl API instability
    // They will be re-enabled when the horned-owl API becomes more stable

    #[test]
    fn test_adapter_creation() {
        let adapter = HornedOwlAdapter::new();
        // Test that adapter can be created successfully
        assert_eq!(adapter.axiom_counter, 0);
    }

    #[test]
    fn test_basic_conversion() {
        let mut adapter = HornedOwlAdapter::new();

        // Create a mock debug object for testing
        let mock_ontology = "Mock horned-owl ontology";

        // Test basic conversion
        let result = adapter.convert_basic_ontology::<String>(&mock_ontology);
        assert!(result.is_ok());

        let ontology = result.expect("Failed to complete operation successfully");
        assert!(ontology.get_iri().is_some());
    }

    #[test]
    fn test_rdf11_mode_default() {
        let adapter = HornedOwlAdapter::new();
        assert!(adapter.is_rdf11_mode(), "Adapter should default to RDF 1.1 mode");
    }

    #[test]
    fn test_rdf_mode_configuration() {
        let mut adapter = HornedOwlAdapter::with_rdf_mode(false);
        assert!(!adapter.is_rdf11_mode(), "Adapter should be in RDF 1.2 mode");
        
        adapter.set_rdf11_mode(true);
        assert!(adapter.is_rdf11_mode(), "Adapter should switch to RDF 1.1 mode");
    }

    #[test]
    fn test_reify_simple_quoted_triple() {
        let mut adapter = HornedOwlAdapter::new();
        
        // Create a simple quoted triple: << :alice :knows :bob >>
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        
        let inner_triple = Triple::new(alice, knows, bob);
        let quoted_term = RdfTerm::QuotedTriple(Box::new(inner_triple));
        
        // Reify the quoted triple
        let result = adapter.reify_rdf_term(&quoted_term);
        assert!(result.is_ok(), "Reification should succeed");
        
        let (reified_term, reification_triples) = result.unwrap();
        
        // Should return a blank node
        assert!(matches!(reified_term, RdfTerm::BlankNode(_)), "Reified term should be a blank node");
        
        // Should have 4 reification triples: type, subject, predicate, object
        assert_eq!(reification_triples.len(), 4, "Should have 4 reification triples");
        
        // Check for rdf:type rdf:Statement triple
        let has_type_triple = reification_triples.iter().any(|t| {
            if let RdfTerm::Iri(pred_url) = &t.predicate {
                pred_url.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
            } else {
                false
            }
        });
        assert!(has_type_triple, "Should have rdf:type triple");
    }

    #[test]
    fn test_reify_nested_quoted_triple() {
        let mut adapter = HornedOwlAdapter::new();
        
        // Create nested quoted triple: << << :a :b :c >> :d :e >>
        let a = RdfTerm::iri("http://example.org/a").unwrap();
        let b = RdfTerm::iri("http://example.org/b").unwrap();
        let c = RdfTerm::iri("http://example.org/c").unwrap();
        let d = RdfTerm::iri("http://example.org/d").unwrap();
        let e = RdfTerm::iri("http://example.org/e").unwrap();
        
        let inner_triple = Triple::new(a, b, c);
        let nested_term = RdfTerm::QuotedTriple(Box::new(inner_triple));
        let outer_triple = Triple::new(nested_term, d, e);
        let outer_quoted = RdfTerm::QuotedTriple(Box::new(outer_triple));
        
        // Reify the nested quoted triple
        let result = adapter.reify_rdf_term(&outer_quoted);
        assert!(result.is_ok(), "Nested reification should succeed");
        
        let (reified_term, reification_triples) = result.unwrap();
        
        // Should return a blank node
        assert!(matches!(reified_term, RdfTerm::BlankNode(_)));
        
        // Should have 8 triples total: 4 for outer + 4 for inner
        assert_eq!(reification_triples.len(), 8, "Should have 8 reification triples for nested structure");
    }

    #[test]
    fn test_convert_triple_to_rdf11_without_quoted_triples() {
        let mut adapter = HornedOwlAdapter::new();
        
        // Create a simple RDF triple with no quoted triples
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        
        let triple = Triple::new(alice, knows, bob);
        
        // Convert to RDF 1.1
        let result = adapter.convert_triple_to_rdf11(&triple);
        assert!(result.is_ok());
        
        let triples = result.unwrap();
        
        // Should return just the original triple
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0], triple);
    }

    #[test]
    fn test_convert_triple_to_rdf11_with_quoted_triple_subject() {
        let mut adapter = HornedOwlAdapter::new();
        
        // Create: << :alice :knows :bob >> :certainty 0.95
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        let certainty = RdfTerm::iri("http://example.org/certainty").unwrap();
        let value = RdfTerm::Literal {
            value: "0.95".to_string(),
            datatype: Some(Url::parse("http://www.w3.org/2001/XMLSchema#decimal").unwrap()),
            language: None,
            direction: None,
        };
        
        let inner_triple = Triple::new(alice, knows, bob);
        let quoted_subject = RdfTerm::QuotedTriple(Box::new(inner_triple));
        let main_triple = Triple::new(quoted_subject, certainty, value);
        
        // Convert to RDF 1.1
        let result = adapter.convert_triple_to_rdf11(&main_triple);
        assert!(result.is_ok());
        
        let triples = result.unwrap();
        
        // Should have 5 triples: main triple + 4 reification triples
        assert_eq!(triples.len(), 5);
        
        // First triple should have blank node as subject
        assert!(matches!(triples[0].subject, RdfTerm::BlankNode(_)));
    }

    #[test]
    fn test_convert_triple_rdf12_mode() {
        let mut adapter = HornedOwlAdapter::with_rdf_mode(false); // RDF 1.2 mode
        
        // Create a quoted triple
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        
        let inner_triple = Triple::new(alice, knows, bob);
        let quoted_term = RdfTerm::QuotedTriple(Box::new(inner_triple.clone()));
        let certainty = RdfTerm::iri("http://example.org/certainty").unwrap();
        let value = RdfTerm::Literal {
            value: "0.95".to_string(),
            datatype: None,
            language: None,
            direction: None,
        };
        
        let triple = Triple::new(quoted_term, certainty, value);
        
        // Convert in RDF 1.2 mode (should preserve quoted triples)
        let result = adapter.convert_triple_to_rdf11(&triple);
        assert!(result.is_ok());
        
        let triples = result.unwrap();
        
        // Should return just the original triple without reification
        assert_eq!(triples.len(), 1);
        assert!(matches!(triples[0].subject, RdfTerm::QuotedTriple(_)));
    }

    #[test]
    fn test_blank_node_counter() {
        let mut adapter = HornedOwlAdapter::new();
        
        let id1 = adapter.next_blank_node_id();
        let id2 = adapter.next_blank_node_id();
        let id3 = adapter.next_blank_node_id();
        
        assert_eq!(id1, "_:reify1");
        assert_eq!(id2, "_:reify2");
        assert_eq!(id3, "_:reify3");
    }

    #[test]
    fn test_reify_non_quoted_term() {
        let mut adapter = HornedOwlAdapter::new();
        
        // Test with IRI
        let iri_term = RdfTerm::iri("http://example.org/test").unwrap();
        let result = adapter.reify_rdf_term(&iri_term);
        assert!(result.is_ok());
        
        let (reified, triples) = result.unwrap();
        assert_eq!(reified, iri_term); // Should be unchanged
        assert_eq!(triples.len(), 0); // No additional triples
        
        // Test with blank node
        let blank = RdfTerm::BlankNode("_:b1".to_string());
        let result = adapter.reify_rdf_term(&blank);
        assert!(result.is_ok());
        
        let (reified, triples) = result.unwrap();
        assert_eq!(reified, blank);
        assert_eq!(triples.len(), 0);
        
        // Test with literal
        let literal = RdfTerm::Literal {
            value: "test".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: None,
        };
        let result = adapter.reify_rdf_term(&literal);
        assert!(result.is_ok());
        
        let (reified, triples) = result.unwrap();
        assert_eq!(reified, literal);
        assert_eq!(triples.len(), 0);
    }

    #[test]
    fn test_dereify_triples_passthrough() {
        let adapter = HornedOwlAdapter::new();
        
        // Create some simple triples
        let alice = RdfTerm::iri("http://example.org/alice").unwrap();
        let knows = RdfTerm::iri("http://example.org/knows").unwrap();
        let bob = RdfTerm::iri("http://example.org/bob").unwrap();
        
        let triple = Triple::new(alice, knows, bob);
        let triples = vec![triple.clone()];
        
        // Dereify (currently just passes through)
        let result = adapter.dereify_triples(&triples);
        assert!(result.is_ok());
        
        let dereified = result.unwrap();
        assert_eq!(dereified.len(), 1);
        assert_eq!(dereified[0], triple);
    }

    /*
    // TODO: Re-enable these tests when horned-owl API is more stable
    #[test]
    fn test_iri_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        // Use proper IRI constructor - build IRI from string
        let horned_iri = horned_owl::model::IRI::from("http://example.org/test".to_string());
        let oxidowl_iri = adapter.convert_iri(&horned_iri).expect("Failed to convert Horned OWL IRI to OxidOwl IRI");
        assert_eq!(oxidowl_iri.as_str(), "http://example.org/test");
    }

    #[test]
    fn test_class_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        let horned_iri = horned_owl::model::IRI::from("http://example.org/Person".to_string());
        let horned_class = horned_owl::model::Class(horned_iri);
        let oxidowl_class = adapter.convert_class(&horned_class).expect("Failed to convert Horned OWL class to OxidOwl class");
        assert_eq!(oxidowl_class.iri.as_str(), "http://example.org/Person");
    }
    */
}
