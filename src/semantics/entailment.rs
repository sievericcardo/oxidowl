//! Entailment Relations Implementation
//!
//! This module implements various entailment relations for RDF, RDFS, and OWL
//! according to the W3C specifications.

use super::{RdfGraph, RdfTerm, Triple, owl2::Owl2ReasoningEngine};
use crate::{Error, Result, ontology::Axiom};
use std::collections::{HashMap, HashSet};

/// Types of entailment regimes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntailmentRegime {
    /// Simple RDF entailment
    RdfSimple,
    /// RDFS entailment
    Rdfs,
    /// OWL 2 RDF-Based semantics
    OwlRdf,
    /// OWL 2 Direct semantics
    OwlDirect,
    /// OWL 2 RL profile
    OwlRl,
    /// OWL 2 EL profile
    OwlEl,
    /// OWL 2 QL profile
    OwlQl,
}

/// Entailment checker for different regimes
#[derive(Debug)]
pub struct EntailmentChecker {
    regime: EntailmentRegime,
    cache: HashMap<(String, String), bool>,
}

impl EntailmentChecker {
    /// Create a new entailment checker for the specified regime
    pub fn new(regime: EntailmentRegime) -> Self {
        Self {
            regime,
            cache: HashMap::new(),
        }
    }

    /// Check if premises entail conclusion under the current regime
    pub fn entails(&mut self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        let cache_key = (
            format!("{:?}", premises.triples()),
            format!("{:?}", conclusion.triples()),
        );

        if let Some(&result) = self.cache.get(&cache_key) {
            return Ok(result);
        }

        let result = match self.regime {
            EntailmentRegime::RdfSimple => self.check_rdf_simple_entailment(premises, conclusion)?,
            EntailmentRegime::Rdfs => self.check_rdfs_entailment(premises, conclusion)?,
            EntailmentRegime::OwlRdf => self.check_owl_rdf_entailment(premises, conclusion)?,
            EntailmentRegime::OwlDirect => self.check_owl_direct_entailment(premises, conclusion)?,
            EntailmentRegime::OwlRl => self.check_owl_rl_entailment(premises, conclusion)?,
            EntailmentRegime::OwlEl => self.check_owl_el_entailment(premises, conclusion)?,
            EntailmentRegime::OwlQl => self.check_owl_ql_entailment(premises, conclusion)?,
        };

        self.cache.insert(cache_key, result);
        Ok(result)
    }

    /// Check RDF simple entailment
    fn check_rdf_simple_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        use super::rdf::RdfSimpleEntailment;
        
        let engine = RdfSimpleEntailment::new(premises.clone());
        Ok(engine.entails(premises, conclusion))
    }

    /// Check RDFS entailment
    fn check_rdfs_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // TODO: Re-enable when rdfs module is fixed
        // let mut engine = RdfsEntailmentEngine::new(premises.clone());
        // engine.reason()?;
        // let closure = engine.closure();
        
        // Temporary fallback: basic triple containment check
        for triple in conclusion.triples() {
            if !premises.contains_triple(triple) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check OWL RDF-based entailment
    fn check_owl_rdf_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Enhanced OWL RDF-based semantics entailment
        // This combines RDFS entailment with OWL-specific semantics rules
        
        // First check RDFS entailment as a foundation
        let rdfs_entails = self.check_rdfs_entailment(premises, conclusion)?;
        if rdfs_entails {
            return Ok(true);
        }
        
        // Apply OWL RDF-based semantics rules
        let mut working_premises = premises.clone();
        
        // Apply OWL RDF semantics rules iteratively
        loop {
            let initial_size = working_premises.triples.len();
            
            // Apply OWL class semantics
            self.apply_owl_class_semantics(&mut working_premises)?;
            
            // Apply OWL property semantics
            self.apply_owl_property_semantics(&mut working_premises)?;
            
            // Apply OWL individual semantics
            self.apply_owl_individual_semantics(&mut working_premises)?;
            
            // Apply OWL restriction semantics
            self.apply_owl_restriction_semantics(&mut working_premises)?;
            
            // Check if we reached a fixed point
            if working_premises.triples.len() == initial_size {
                break;
            }
        }
        
        // Check if conclusion is now entailed
        for conclusion_triple in &conclusion.triples {
            if !working_premises.contains_triple(conclusion_triple) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Apply OWL class semantics rules
    fn apply_owl_class_semantics(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // owl:equivalentClass semantics
        let equiv_triples: Vec<_> = graph.find_triples(
            None, 
            Some(&RdfTerm::Iri(OWL_EQUIVALENT_CLASS.clone())), 
            None
        ).into_iter().cloned().collect();
        
        for triple in equiv_triples {
            // If A owl:equivalentClass B, then A rdfs:subClassOf B and B rdfs:subClassOf A
            let subclass_triple1 = Triple {
                subject: triple.subject.clone(),
                predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                object: triple.object.clone(),
            };
            let subclass_triple2 = Triple {
                subject: triple.object,
                predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                object: triple.subject,
            };
            
            graph.add_triple(subclass_triple1);
            graph.add_triple(subclass_triple2);
        }
        
        Ok(())
    }
    
    /// Apply OWL property semantics rules
    fn apply_owl_property_semantics(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // owl:equivalentProperty semantics
        let equiv_prop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(OWL_EQUIVALENT_PROPERTY.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple in equiv_prop_triples {
            // If P owl:equivalentProperty Q, then P rdfs:subPropertyOf Q and Q rdfs:subPropertyOf P
            let subprop_triple1 = Triple {
                subject: triple.subject.clone(),
                predicate: RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone()),
                object: triple.object.clone(),
            };
            let subprop_triple2 = Triple {
                subject: triple.object,
                predicate: RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone()),
                object: triple.subject,
            };
            
            graph.add_triple(subprop_triple1);
            graph.add_triple(subprop_triple2);
        }
        
        Ok(())
    }
    
    /// Apply OWL individual semantics rules
    fn apply_owl_individual_semantics(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // owl:sameAs semantics (symmetry and transitivity)
        let same_as_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(OWL_SAME_AS.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple in same_as_triples {
            // Symmetry: if x owl:sameAs y, then y owl:sameAs x
            let symmetric_triple = Triple {
                subject: triple.object.clone(),
                predicate: RdfTerm::Iri(OWL_SAME_AS.clone()),
                object: triple.subject.clone(),
            };
            graph.add_triple(symmetric_triple);
            
            // Reflexivity: x owl:sameAs x
            let reflexive_triple1 = Triple {
                subject: triple.subject.clone(),
                predicate: RdfTerm::Iri(OWL_SAME_AS.clone()),
                object: triple.subject.clone(),
            };
            let reflexive_triple2 = Triple {
                subject: triple.object.clone(),
                predicate: RdfTerm::Iri(OWL_SAME_AS.clone()),
                object: triple.object.clone(),
            };
            graph.add_triple(reflexive_triple1);
            graph.add_triple(reflexive_triple2);
        }
        
        Ok(())
    }
    
    /// Apply OWL restriction semantics rules
    fn apply_owl_restriction_semantics(&self, graph: &mut RdfGraph) -> Result<()> {
        // Apply basic OWL restriction reasoning
        let mut derived_triples = Vec::new();
        
        // Find existential restrictions (someValuesFrom)
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/2002/07/owl#someValuesFrom") {
                // Apply existential restriction reasoning
                self.apply_existential_restriction(triple, graph, &mut derived_triples)?;
            }
            
            if triple.predicate.as_str() == Some("http://www.w3.org/2002/07/owl#allValuesFrom") {
                // Apply universal restriction reasoning
                self.apply_universal_restriction(triple, graph, &mut derived_triples)?;
            }
        }
        
        // Add derived triples
        graph.triples.extend(derived_triples);
        Ok(())
    }
    
    /// Apply existential restriction reasoning
    fn apply_existential_restriction(
        &self,
        _restriction_triple: &Triple,
        _graph: &RdfGraph,
        _derived_triples: &mut Vec<Triple>,
    ) -> Result<()> {
        // For now, implement basic existential restriction handling
        // Full implementation would require complex reasoning
        Ok(())
    }
    
    /// Apply universal restriction reasoning  
    fn apply_universal_restriction(
        &self,
        _restriction_triple: &Triple,
        _graph: &RdfGraph,
        _derived_triples: &mut Vec<Triple>,
    ) -> Result<()> {
        // For now, implement basic universal restriction handling
        // Full implementation would require complex reasoning
        Ok(())
    }

    /// Check OWL Direct semantics entailment
    fn check_owl_direct_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Convert RDF graphs to OWL axioms and use direct semantics
        let premise_axioms = self.rdf_graph_to_owl_axioms(premises)?;
        let conclusion_axioms = self.rdf_graph_to_owl_axioms(conclusion)?;
        
        // Create a temporary ontology with premise axioms
        let mut ontology = crate::ontology::Ontology::new();
        for axiom in premise_axioms {
            ontology.add_axiom(axiom);
        }
        
        // Check if each conclusion axiom is entailed
        for conclusion_axiom in conclusion_axioms {
            if !self.is_axiom_entailed(&ontology, &conclusion_axiom)? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Convert RDF graph to OWL axioms (simplified implementation)
    fn rdf_graph_to_owl_axioms(&self, graph: &RdfGraph) -> Result<Vec<crate::ontology::Axiom>> {
        let mut axioms = Vec::new();
        
        // Extract class assertions
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") {
                if let Ok(axiom) = self.create_class_assertion_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract subclass relationships
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/2000/01/rdf-schema#subClassOf") {
                if let Ok(axiom) = self.create_subclass_axiom_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        Ok(axioms)
    }
    
    /// Create class assertion axiom from RDF triple
    fn create_class_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject_str = triple.subject.as_str().ok_or_else(|| 
            Error::ontology_parsing("Invalid subject term in RDF triple"))?;
        let individual = crate::ontology::Individual::named(
            crate::ontology::IRI::new(subject_str)
        );
        let object_str = triple.object.as_str().ok_or_else(|| 
            Error::ontology_parsing("Invalid object term in RDF triple"))?;
            
        let class = crate::ontology::ClassExpression::Class(
            crate::ontology::Class::new(crate::ontology::IRI::new(object_str))
        );
        
        Ok(crate::ontology::Axiom::ClassAssertion(
            crate::ontology::ClassAssertionAxiom {
                id: 0,
                class,
                individual,
                annotations: Vec::new(),
            }
        ))
    }
    
    /// Create subclass axiom from RDF triple
    fn create_subclass_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject_str = triple.subject.as_str().ok_or_else(|| 
            Error::ontology_parsing("Invalid subject term in RDF triple"))?;
        let subclass = crate::ontology::ClassExpression::Class(
            crate::ontology::Class::new(crate::ontology::IRI::new(subject_str))
        );
        let object_str = triple.object.as_str().ok_or_else(|| 
            Error::ontology_parsing("Invalid object term in RDF triple"))?;
            
        let superclass = crate::ontology::ClassExpression::Class(
            crate::ontology::Class::new(crate::ontology::IRI::new(object_str))
        );
        
        Ok(crate::ontology::Axiom::SubClassOf(
            crate::ontology::SubClassOfAxiom {
                id: 0,
                subclass,
                superclass,
                annotations: Vec::new(),
            }
        ))
    }
    
    /// Check if an axiom is entailed by an ontology (simplified check)
    fn is_axiom_entailed(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::Axiom) -> Result<bool> {
        // Simple check - look for exact axiom match or obvious entailments
        for ont_axiom in ontology.axioms() {
            if ont_axiom == axiom {
                return Ok(true);
            }
        }
        
        // Check for basic entailments based on axiom type
        match axiom {
            crate::ontology::Axiom::ClassAssertion(assertion) => {
                // Check if individual has a subclass of the asserted class
                for ont_axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::ClassAssertion(ont_assertion) = ont_axiom {
                        if ont_assertion.individual == assertion.individual {
                            // Check if ont_assertion.class is subclass of assertion.class
                            if self.is_subclass_in_ontology(&ont_assertion.class, &assertion.class, ontology)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
            _ => {} // Handle other axiom types as needed
        }
        
        Ok(false)
    }
    
    /// Check if one class is a subclass of another in the ontology
    fn is_subclass_in_ontology(
        &self,
        subclass: &crate::ontology::ClassExpression,
        superclass: &crate::ontology::ClassExpression,
        ontology: &crate::ontology::Ontology,
    ) -> Result<bool> {
        if subclass == superclass {
            return Ok(true);
        }
        
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(sub_axiom) = axiom {
                if sub_axiom.subclass == *subclass && sub_axiom.superclass == *superclass {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }

    /// Check OWL 2 RL entailment
    fn check_owl_rl_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // OWL 2 RL uses rule-based reasoning on RDF graphs
        let mut engine = Owl2RlEngine::new(premises.clone());
        engine.reason()?;
        
        let closure = engine.closure();
        Ok(conclusion.triples().iter().all(|triple| closure.contains_triple(triple)))
    }

    /// Check OWL 2 EL entailment  
    fn check_owl_el_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Enhanced OWL 2 EL profile reasoning
        // EL allows: conjunction, existential quantification, nominals, and limited universal quantification
        
        let mut working_premises = premises.clone();
        
        // Apply EL-specific completion rules
        loop {
            let initial_size = working_premises.triples.len();
            
            // EL completion rules
            self.apply_el_subclass_rules(&mut working_premises)?;
            self.apply_el_intersection_rules(&mut working_premises)?;
            self.apply_el_existential_rules(&mut working_premises)?;
            self.apply_el_nominal_rules(&mut working_premises)?;
            
            // Check for fixed point
            if working_premises.triples.len() == initial_size {
                break;
            }
        }
        
        // Check if conclusion is entailed
        for conclusion_triple in &conclusion.triples {
            if !working_premises.contains_triple(conclusion_triple) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Check OWL 2 QL entailment
    fn check_owl_ql_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Enhanced OWL 2 QL profile reasoning
        // QL allows: subclass axioms, some property restrictions, and query answering optimizations
        
        let mut working_premises = premises.clone();
        
        // Apply QL-specific completion rules  
        loop {
            let initial_size = working_premises.triples.len();
            
            // QL completion rules
            self.apply_ql_subclass_rules(&mut working_premises)?;
            self.apply_ql_property_rules(&mut working_premises)?;
            self.apply_ql_domain_range_rules(&mut working_premises)?;
            
            // Check for fixed point
            if working_premises.triples.len() == initial_size {
                break;
            }
        }
        
        // Check if conclusion is entailed
        for conclusion_triple in &conclusion.triples {
            if !working_premises.contains_triple(conclusion_triple) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Apply EL subclass completion rules
    fn apply_el_subclass_rules(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // Basic subclass transitivity (already handled in RDFS)
        // EL-specific: if A ⊑ B and B ⊑ C, then A ⊑ C
        let subclass_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBCLASS_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple1 in &subclass_triples {
            for triple2 in &subclass_triples {
                if triple1.object == triple2.subject {
                    let transitive_triple = Triple {
                        subject: triple1.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                        object: triple2.object.clone(),
                    };
                    graph.add_triple(transitive_triple);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply EL intersection completion rules
    fn apply_el_intersection_rules(&self, _graph: &mut RdfGraph) -> Result<()> {
        // EL intersection rules: if A ⊑ B and A ⊑ C, then A ⊑ B ⊓ C
        // This requires parsing intersection constructs from RDF
        // Simplified implementation for now
        Ok(())
    }
    
    /// Apply EL existential completion rules
    fn apply_el_existential_rules(&self, _graph: &mut RdfGraph) -> Result<()> {
        // EL existential rules: if A ⊑ ∃R.B and (x,y) ∈ R and x ∈ A, then y ∈ B
        // This requires parsing existential restrictions from RDF
        // Simplified implementation for now
        Ok(())
    }
    
    /// Apply EL nominal completion rules
    fn apply_el_nominal_rules(&self, _graph: &mut RdfGraph) -> Result<()> {
        // EL nominal rules: handling of nominals {a}
        // Simplified implementation for now
        Ok(())
    }
    
    /// Apply QL subclass completion rules
    fn apply_ql_subclass_rules(&self, graph: &mut RdfGraph) -> Result<()> {
        // QL subclass rules are the same as basic RDFS subclass transitivity
        self.apply_el_subclass_rules(graph)
    }
    
    /// Apply QL property completion rules
    fn apply_ql_property_rules(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // QL property hierarchy rules
        let subprop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        // Apply subproperty implications
        for subprop_triple in subprop_triples {
            if let (RdfTerm::Iri(sub_prop), RdfTerm::Iri(super_prop)) = (&subprop_triple.subject, &subprop_triple.object) {
                // If P ⊑ Q and (x,y) ∈ P, then (x,y) ∈ Q
                let prop_assertions: Vec<_> = graph.find_triples(
                    None,
                    Some(&RdfTerm::Iri(sub_prop.clone())),
                    None
                ).into_iter().cloned().collect();
                
                for prop_assertion in prop_assertions {
                    let derived_triple = Triple {
                        subject: prop_assertion.subject,
                        predicate: RdfTerm::Iri(super_prop.clone()),
                        object: prop_assertion.object,
                    };
                    graph.add_triple(derived_triple);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply QL domain and range completion rules
    fn apply_ql_domain_range_rules(&self, graph: &mut RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // Domain rules: if P has domain C and (x,y) ∈ P, then x ∈ C
        let domain_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_DOMAIN.clone())),
            None
        ).into_iter().cloned().collect();
        
        for domain_triple in domain_triples {
            if let RdfTerm::Iri(property) = &domain_triple.subject {
                let prop_assertions: Vec<_> = graph.find_triples(
                    None,
                    Some(&RdfTerm::Iri(property.clone())),
                    None
                ).into_iter().cloned().collect();
                
                for prop_assertion in prop_assertions {
                    let type_triple = Triple {
                        subject: prop_assertion.subject,
                        predicate: RdfTerm::Iri(RDF_TYPE.clone()),
                        object: domain_triple.object.clone(),
                    };
                    graph.add_triple(type_triple);
                }
            }
        }
        
        // Range rules: if P has range C and (x,y) ∈ P, then y ∈ C
        let range_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_RANGE.clone())),
            None
        ).into_iter().cloned().collect();
        
        for range_triple in range_triples {
            if let RdfTerm::Iri(property) = &range_triple.subject {
                let prop_assertions: Vec<_> = graph.find_triples(
                    None,
                    Some(&RdfTerm::Iri(property.clone())),
                    None
                ).into_iter().cloned().collect();
                
                for prop_assertion in prop_assertions {
                    let type_triple = Triple {
                        subject: prop_assertion.object,
                        predicate: RdfTerm::Iri(RDF_TYPE.clone()),
                        object: range_triple.object.clone(),
                    };
                    graph.add_triple(type_triple);
                }
            }
        }
        
        Ok(())
    }

    /// Clear the entailment cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the current entailment regime
    pub fn regime(&self) -> EntailmentRegime {
        self.regime
    }

    /// Set the entailment regime
    pub fn set_regime(&mut self, regime: EntailmentRegime) {
        self.regime = regime;
        self.clear_cache(); // Clear cache when regime changes
    }
}

/// OWL 2 RL Rule Engine
///
/// Implements the OWL 2 RL profile rules as defined in:
/// https://www.w3.org/TR/owl2-profiles/#OWL_2_RL
#[derive(Debug)]
pub struct Owl2RlEngine {
    input_graph: RdfGraph,
    derived_graph: RdfGraph,
    fixed_point: bool,
}

impl Owl2RlEngine {
    /// Create a new OWL 2 RL engine
    pub fn new(input_graph: RdfGraph) -> Self {
        Self {
            input_graph,
            derived_graph: RdfGraph::new(),
            fixed_point: false,
        }
    }

    /// Perform OWL 2 RL reasoning
    pub fn reason(&mut self) -> Result<()> {
        // TODO: Re-enable when rdfs module is fixed
        // First apply RDFS reasoning as a base
        // let mut rdfs_engine = RdfsEntailmentEngine::new(self.input_graph.clone());
        // rdfs_engine.reason()?;
        // self.derived_graph = rdfs_engine.derived_facts().clone();
        
        // Temporary: start with input graph
        self.derived_graph = self.input_graph.clone();
        
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 1000;

        while !self.fixed_point && iteration < MAX_ITERATIONS {
            let initial_size = self.derived_graph.size();
            
            // Apply OWL 2 RL rules
            self.apply_owl_rl_rules()?;
            
            // Check if fixed point is reached
            if self.derived_graph.size() == initial_size {
                self.fixed_point = true;
            }
            
            iteration += 1;
        }

        if iteration >= MAX_ITERATIONS {
            return Err(Error::reasoning("OWL 2 RL reasoning did not converge".to_string()));
        }

        Ok(())
    }

    /// Apply OWL 2 RL inference rules
    fn apply_owl_rl_rules(&mut self) -> Result<()> {
        let working_graph = self.get_working_graph();

        // Apply class-related rules
        self.apply_cls_rules(&working_graph)?;
        
        // Apply property-related rules
        self.apply_prp_rules(&working_graph)?;
        
        // Apply schema rules
        self.apply_scm_rules(&working_graph)?;

        Ok(())
    }

    /// Get working graph (input + derived)
    fn get_working_graph(&self) -> RdfGraph {
        let mut working = self.input_graph.clone();
        working.merge(&self.derived_graph);
        working
    }

    /// Add derived triple if not already present
    fn add_derived_triple(&mut self, triple: Triple) {
        if !self.input_graph.contains_triple(&triple) && !self.derived_graph.contains_triple(&triple) {
            self.derived_graph.add_triple(triple);
        }
    }

    /// Apply class-related OWL 2 RL rules (cls-*)
    fn apply_cls_rules(&mut self, graph: &RdfGraph) -> Result<()> {
        // cls-thing: Every individual is an instance of owl:Thing
        self.apply_cls_thing(graph)?;
        
        // cls-nothing: owl:Nothing has no instances
        self.apply_cls_nothing(graph)?;
        
        // cls-int1: Intersection reasoning
        self.apply_cls_intersection(graph)?;
        
        // cls-uni: Union reasoning
        self.apply_cls_union(graph)?;
        
        // cls-com: Complement reasoning
        self.apply_cls_complement(graph)?;
        
        // cls-svf1: Some values from reasoning
        self.apply_cls_some_values_from(graph)?;
        
        // cls-avf: All values from reasoning
        self.apply_cls_all_values_from(graph)?;
        
        // cls-hv1: Has value reasoning
        self.apply_cls_has_value(graph)?;
        
        // cls-maxc1: Max cardinality reasoning
        self.apply_cls_max_cardinality(graph)?;

        Ok(())
    }

    /// Apply property-related OWL 2 RL rules (prp-*)
    fn apply_prp_rules(&mut self, graph: &RdfGraph) -> Result<()> {
        // prp-fp: Functional property reasoning
        self.apply_prp_functional(graph)?;
        
        // prp-ifp: Inverse functional property reasoning
        self.apply_prp_inverse_functional(graph)?;
        
        // prp-symp: Symmetric property reasoning
        self.apply_prp_symmetric(graph)?;
        
        // prp-trp: Transitive property reasoning
        self.apply_prp_transitive(graph)?;
        
        // prp-spo1: Subproperty reasoning
        self.apply_prp_subproperty(graph)?;
        
        // prp-eqp1: Equivalent property reasoning
        self.apply_prp_equivalent_property(graph)?;
        
        // prp-inv1: Inverse property reasoning
        self.apply_prp_inverse_property(graph)?;

        Ok(())
    }

    /// Apply schema-related OWL 2 RL rules (scm-*)
    fn apply_scm_rules(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced schema rules for class and property hierarchies
        
        // scm-cls: Class hierarchy rules
        self.apply_scm_class_hierarchy(graph)?;
        
        // scm-spo: Subproperty rules  
        self.apply_scm_subproperty_hierarchy(graph)?;
        
        // scm-eqc: Equivalent class rules
        self.apply_scm_equivalent_classes(graph)?;
        
        // scm-eqp: Equivalent property rules
        self.apply_scm_equivalent_properties(graph)?;
        
        // scm-dom: Domain inheritance rules
        self.apply_scm_domain_inheritance(graph)?;
        
        // scm-rng: Range inheritance rules
        self.apply_scm_range_inheritance(graph)?;
        
        Ok(())
    }
    
    /// Apply class hierarchy schema rules (scm-cls)
    fn apply_scm_class_hierarchy(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-cls: If C1 rdfs:subClassOf C2 and C2 rdfs:subClassOf C3, then C1 rdfs:subClassOf C3
        let subclass_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBCLASS_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple1 in &subclass_triples {
            for triple2 in &subclass_triples {
                if triple1.object == triple2.subject {
                    let transitive_triple = Triple {
                        subject: triple1.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                        object: triple2.object.clone(),
                    };
                    self.add_derived_triple(transitive_triple);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply subproperty hierarchy schema rules (scm-spo)
    fn apply_scm_subproperty_hierarchy(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-spo: If P1 rdfs:subPropertyOf P2 and P2 rdfs:subPropertyOf P3, then P1 rdfs:subPropertyOf P3
        let subprop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple1 in &subprop_triples {
            for triple2 in &subprop_triples {
                if triple1.object == triple2.subject {
                    let transitive_triple = Triple {
                        subject: triple1.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone()),
                        object: triple2.object.clone(),
                    };
                    self.add_derived_triple(transitive_triple);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply equivalent class schema rules (scm-eqc)
    fn apply_scm_equivalent_classes(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-eqc1: If C1 owl:equivalentClass C2, then C1 rdfs:subClassOf C2
        // scm-eqc2: If C1 owl:equivalentClass C2, then C2 rdfs:subClassOf C1
        let equiv_class_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(OWL_EQUIVALENT_CLASS.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple in equiv_class_triples {
            let subclass_triple1 = Triple {
                subject: triple.subject.clone(),
                predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                object: triple.object.clone(),
            };
            let subclass_triple2 = Triple {
                subject: triple.object,
                predicate: RdfTerm::Iri(RDFS_SUBCLASS_OF.clone()),
                object: triple.subject,
            };
            
            self.add_derived_triple(subclass_triple1);
            self.add_derived_triple(subclass_triple2);
        }
        
        Ok(())
    }
    
    /// Apply equivalent property schema rules (scm-eqp)
    fn apply_scm_equivalent_properties(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-eqp1: If P1 owl:equivalentProperty P2, then P1 rdfs:subPropertyOf P2
        // scm-eqp2: If P1 owl:equivalentProperty P2, then P2 rdfs:subPropertyOf P1
        let equiv_prop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(OWL_EQUIVALENT_PROPERTY.clone())),
            None
        ).into_iter().cloned().collect();
        
        for triple in equiv_prop_triples {
            let subprop_triple1 = Triple {
                subject: triple.subject.clone(),
                predicate: RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone()),
                object: triple.object.clone(),
            };
            let subprop_triple2 = Triple {
                subject: triple.object,
                predicate: RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone()),
                object: triple.subject,
            };
            
            self.add_derived_triple(subprop_triple1);
            self.add_derived_triple(subprop_triple2);
        }
        
        Ok(())
    }
    
    /// Apply domain inheritance schema rules (scm-dom)
    fn apply_scm_domain_inheritance(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-dom1: If P rdfs:domain C1 and C1 rdfs:subClassOf C2, then P rdfs:domain C2
        // scm-dom2: If P1 rdfs:subPropertyOf P2 and P2 rdfs:domain C, then P1 rdfs:domain C
        
        // Rule scm-dom1
        let domain_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_DOMAIN.clone())),
            None
        ).into_iter().cloned().collect();
        
        let subclass_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBCLASS_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for domain_triple in &domain_triples {
            for subclass_triple in &subclass_triples {
                if domain_triple.object == subclass_triple.subject {
                    let derived_domain_triple = Triple {
                        subject: domain_triple.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_DOMAIN.clone()),
                        object: subclass_triple.object.clone(),
                    };
                    self.add_derived_triple(derived_domain_triple);
                }
            }
        }
        
        // Rule scm-dom2
        let subprop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for subprop_triple in &subprop_triples {
            for domain_triple in &domain_triples {
                if subprop_triple.object == domain_triple.subject {
                    let derived_domain_triple = Triple {
                        subject: subprop_triple.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_DOMAIN.clone()),
                        object: domain_triple.object.clone(),
                    };
                    self.add_derived_triple(derived_domain_triple);
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply range inheritance schema rules (scm-rng)
    fn apply_scm_range_inheritance(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        // scm-rng1: If P rdfs:range C1 and C1 rdfs:subClassOf C2, then P rdfs:range C2
        // scm-rng2: If P1 rdfs:subPropertyOf P2 and P2 rdfs:range C, then P1 rdfs:range C
        
        // Rule scm-rng1
        let range_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_RANGE.clone())),
            None
        ).into_iter().cloned().collect();
        
        let subclass_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBCLASS_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for range_triple in &range_triples {
            for subclass_triple in &subclass_triples {
                if range_triple.object == subclass_triple.subject {
                    let derived_range_triple = Triple {
                        subject: range_triple.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_RANGE.clone()),
                        object: subclass_triple.object.clone(),
                    };
                    self.add_derived_triple(derived_range_triple);
                }
            }
        }
        
        // Rule scm-rng2
        let subprop_triples: Vec<_> = graph.find_triples(
            None,
            Some(&RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone())),
            None
        ).into_iter().cloned().collect();
        
        for subprop_triple in &subprop_triples {
            for range_triple in &range_triples {
                if subprop_triple.object == range_triple.subject {
                    let derived_range_triple = Triple {
                        subject: subprop_triple.subject.clone(),
                        predicate: RdfTerm::Iri(RDFS_RANGE.clone()),
                        object: range_triple.object.clone(),
                    };
                    self.add_derived_triple(derived_range_triple);
                }
            }
        }
        
        Ok(())
    }

    /// Rule cls-thing: Every individual is an instance of owl:Thing
    fn apply_cls_thing(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let thing_iri = RdfTerm::Iri(OWL_THING.clone());

        // For every individual mentioned in the graph, add rdf:type owl:Thing
        let individuals = graph.subjects();
        
        for individual in individuals {
            if !individual.is_literal() {
                let derived_triple = Triple {
                    subject: individual.clone(),
                    predicate: type_iri.clone(),
                    object: thing_iri.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule cls-nothing: owl:Nothing has no instances
    fn apply_cls_nothing(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let nothing_iri = RdfTerm::Iri(OWL_NOTHING.clone());

        // Check for inconsistency: if anything is typed as owl:Nothing, we have a contradiction
        let nothing_instances = graph.find_triples(None, Some(&type_iri), Some(&nothing_iri));
        
        if !nothing_instances.is_empty() {
            // This indicates an inconsistency - for now, we'll just note it
            // In a complete implementation, this would trigger inconsistency handling
        }

        Ok(())
    }

    /// Rule cls-int1: Intersection reasoning
    fn apply_cls_intersection(&mut self, graph: &RdfGraph) -> Result<()> {
        // If C owl:intersectionOf (C1 ... Cn) and x rdf:type C1, ..., x rdf:type Cn
        // then x rdf:type C
        
        // Find intersection definitions
        for triple in &graph.triples {
            if self.is_iri(&triple.predicate, "http://www.w3.org/2002/07/owl#intersectionOf") {
                if let RdfTerm::Iri(class_iri) = &triple.subject {
                    // Parse the intersection list (simplified)
                    if let Some(class_list) = self.parse_rdf_list(&triple.object, graph) {
                        // Find individuals that are instances of all classes in the intersection
                        for individual_triple in &graph.triples {
                            if self.is_iri(&individual_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") {
                                let individual = &individual_triple.subject;
                                
                                // Check if individual is instance of all classes in intersection
                                let is_instance_of_all = class_list.iter().all(|class_term| {
                                    if let Ok(type_predicate) = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") {
                                        let type_triple = Triple {
                                            subject: individual.clone(),
                                            predicate: type_predicate,
                                            object: class_term.clone(),
                                        };
                                        graph.contains_triple(&type_triple)
                                    } else {
                                        false
                                    }
                                });
                                
                                if is_instance_of_all && !class_list.is_empty() {
                                    if let Ok(type_predicate) = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") {
                                        let new_triple = Triple {
                                            subject: individual.clone(),
                                            predicate: type_predicate,
                                            object: RdfTerm::Iri(class_iri.clone()),
                                        };
                                        self.add_derived_triple(new_triple);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Rule cls-uni: Union reasoning
    fn apply_cls_union(&mut self, graph: &RdfGraph) -> Result<()> {
        // If C owl:unionOf (C1 ... Cn) and x rdf:type Ci for some i
        // then x rdf:type C
        
        for triple in &graph.triples {
            if self.is_iri(&triple.predicate, "http://www.w3.org/2002/07/owl#unionOf") {
                if let RdfTerm::Iri(union_class) = &triple.subject {
                    if let Some(class_list) = self.parse_rdf_list(&triple.object, graph) {
                        // For each class in the union
                        for class_term in class_list {
                            // Find individuals that are instances of this class
                            for type_triple in &graph.triples {
                                if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") &&
                                   type_triple.object == class_term {
                                    // Add that they are also instances of the union class
                                    let new_triple = Triple {
                                        subject: type_triple.subject.clone(),
                                        predicate: RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?,
                                        object: RdfTerm::Iri(union_class.clone()),
                                    };
                                    self.add_derived_triple(new_triple);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Rule cls-com: Complement reasoning
    fn apply_cls_complement(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced complement reasoning - C owl:complementOf D
        // If x rdf:type C and C owl:complementOf D, then x cannot be of type D
        // If x rdf:type D and C owl:complementOf D, then x cannot be of type C
        
        for triple in &graph.triples {
            if self.is_iri(&triple.predicate, "http://www.w3.org/2002/07/owl#complementOf") {
                let class_c = &triple.subject;
                let class_d = &triple.object;
                
                // Find all instances of C
                for instance_triple in &graph.triples {
                    if self.is_iri(&instance_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") 
                        && instance_triple.object == *class_c {
                        let instance = &instance_triple.subject;
                        
                        // Check if instance is also asserted to be of type D
                        for type_triple in &graph.triples {
                            if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                                && type_triple.subject == *instance
                                && type_triple.object == *class_d {
                                // Inconsistency detected - instance cannot be both C and complementOf(C)
                                return Err(Error::reasoning(
                                    &format!("Inconsistency: Individual {} is asserted to be both {} and its complement {}", 
                                    instance, class_c, class_d)
                                ));
                            }
                        }
                    }
                }
                
                // Find all instances of D and ensure they're not also instances of C
                for instance_triple in &graph.triples {
                    if self.is_iri(&instance_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") 
                        && instance_triple.object == *class_d {
                        let instance = &instance_triple.subject;
                        
                        // Check if instance is also asserted to be of type C
                        for type_triple in &graph.triples {
                            if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                                && type_triple.subject == *instance
                                && type_triple.object == *class_c {
                                // Inconsistency detected
                                return Err(Error::reasoning(
                                    &format!("Inconsistency: Individual {} is asserted to be both {} and its complement {}", 
                                    instance, class_d, class_c)
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Rule cls-svf1: Some values from reasoning
    fn apply_cls_some_values_from(&mut self, graph: &RdfGraph) -> Result<()> {
        // If C owl:someValuesFrom D and C owl:onProperty P and x rdf:type C
        // then there exists y such that x P y and y rdf:type D
        
        for restriction_triple in &graph.triples {
            if self.is_iri(&restriction_triple.predicate, "http://www.w3.org/2002/07/owl#someValuesFrom") {
                let restriction = &restriction_triple.subject;
                let target_class = &restriction_triple.object;
                
                // Find the property for this restriction
                if let Some(property) = self.find_restriction_property(restriction, graph) {
                    // Find individuals that are instances of this restriction
                    for type_triple in &graph.triples {
                        if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") &&
                           type_triple.object == *restriction {
                            let individual = &type_triple.subject;
                            
                            // Check if there's already a suitable property assertion
                            let has_suitable_assertion = graph.triples.iter().any(|t| {
                                t.subject == *individual &&
                                t.predicate == RdfTerm::Iri(property.to_url().unwrap()) &&
                                // Check if the object is of the target class
                                graph.triples.iter().any(|type_t| {
                                    type_t.subject == t.object &&
                                    self.is_iri(&type_t.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") &&
                                    type_t.object == *target_class
                                })
                            });
                            
                            if !has_suitable_assertion {
                                // In a complete implementation, we would create a witness
                                // For now, we just note that this constraint exists
                                // This is typically handled by tableau reasoners
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Rule cls-avf: All values from reasoning
    fn apply_cls_all_values_from(&mut self, graph: &RdfGraph) -> Result<()> {
        // If C owl:allValuesFrom D and C owl:onProperty P and x rdf:type C and x P y
        // then y rdf:type D
        
        for restriction_triple in &graph.triples {
            if self.is_iri(&restriction_triple.predicate, "http://www.w3.org/2002/07/owl#allValuesFrom") {
                let restriction = &restriction_triple.subject;
                let target_class = &restriction_triple.object;
                
                // Find the property for this restriction
                if let Some(property) = self.find_restriction_property(restriction, graph) {
                    // Find individuals that are instances of this restriction
                    for type_triple in &graph.triples {
                        if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") &&
                           type_triple.object == *restriction {
                            let individual = &type_triple.subject;
                            
                            // Find all property assertions for this individual
                            for prop_triple in &graph.triples {
                                if prop_triple.subject == *individual &&
                                   prop_triple.predicate == RdfTerm::Iri(property.to_url().unwrap()) {
                                    // The object must be of the target class
                                    let new_triple = Triple {
                                        subject: prop_triple.object.clone(),
                                        predicate: RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?,
                                        object: target_class.clone(),
                                    };
                                    self.add_derived_triple(new_triple);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Rule cls-hv1: Has value reasoning
    fn apply_cls_has_value(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced has value reasoning - C owl:hasValue a and C owl:onProperty P
        // If x rdf:type C, then x P a
        // If x P a and C owl:hasValue a and C owl:onProperty P, then x rdf:type C
        
        for restriction_triple in &graph.triples {
            if self.is_iri(&restriction_triple.predicate, "http://www.w3.org/2002/07/owl#hasValue") {
                let restriction = &restriction_triple.subject;
                let value = &restriction_triple.object;
                
                // Find the property for this restriction
                if let Some(property) = self.find_on_property(graph, restriction) {
                    // Rule 1: If x rdf:type C, then x P a
                    for type_triple in &graph.triples {
                        if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                            && type_triple.object == *restriction {
                            let individual = &type_triple.subject;
                            
                            // Check if x P a already exists
                            let has_property_assertion = graph.triples.iter().any(|t| {
                                t.subject == *individual && 
                                t.predicate == property && 
                                t.object == *value
                            });
                            
                            if !has_property_assertion {
                                // Add the inferred property assertion: x P a
                                self.derived_graph.add_triple(Triple {
                                    subject: individual.clone(),
                                    predicate: property.clone(),
                                    object: value.clone()
                                });
                            }
                        }
                    }
                    
                    // Rule 2: If x P a and C owl:hasValue a and C owl:onProperty P, then x rdf:type C
                    for property_triple in &graph.triples {
                        if property_triple.predicate == property && property_triple.object == *value {
                            let individual = &property_triple.subject;
                            
                            // Check if x rdf:type C already exists
                            let has_type_assertion = graph.triples.iter().any(|t| {
                                self.is_iri(&t.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type") &&
                                t.subject == *individual && 
                                t.object == *restriction
                            });
                            
                            if !has_type_assertion {
                                // Add the inferred type assertion: x rdf:type C
                                self.derived_graph.add_triple(Triple {
                                    subject: individual.clone(),
                                    predicate: RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?,
                                    object: restriction.clone()
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Rule cls-maxc1: Max cardinality reasoning
    fn apply_cls_max_cardinality(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced max cardinality reasoning - C owl:maxCardinality n and C owl:onProperty P
        // If x rdf:type C and x has more than n distinct P-successors, then inconsistency
        // If x rdf:type C and x has exactly n distinct P-successors y1...yn, 
        // and x P z for some z not in {y1...yn}, then z = yi for some i
        
        for restriction_triple in &graph.triples {
            if self.is_iri(&restriction_triple.predicate, "http://www.w3.org/2002/07/owl#maxCardinality") {
                let restriction = &restriction_triple.subject;
                
                // Extract cardinality value
                if let Some(cardinality) = self.extract_cardinality_value(&restriction_triple.object) {
                    // Find the property for this restriction
                    if let Some(property) = self.find_on_property(graph, restriction) {
                        // Find all individuals of this restriction type
                        for type_triple in &graph.triples {
                            if self.is_iri(&type_triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                                && type_triple.object == *restriction {
                                let individual = &type_triple.subject;
                                
                                // Collect all property values for this individual
                                let mut property_values = Vec::new();
                                for prop_triple in &graph.triples {
                                    if prop_triple.subject == *individual && prop_triple.predicate == property {
                                        if !property_values.contains(&prop_triple.object) {
                                            property_values.push(prop_triple.object.clone());
                                        }
                                    }
                                }
                                
                                // Check cardinality constraint
                                if property_values.len() > cardinality {
                                    // Too many distinct values - inconsistency or need for sameAs inference
                                    if cardinality == 0 {
                                        return Err(Error::reasoning(
                                            &format!("Max cardinality violation: Individual {} has {} values for property {}, but max cardinality is 0", 
                                            individual, property_values.len(), property)
                                        ));
                                    } else {
                                        // Infer that some values must be the same (closed world assumption)
                                        // For simplicity, we'll merge the excess values
                                        for i in cardinality..property_values.len() {
                                            let excess_value = &property_values[i];
                                            let canonical_value = &property_values[0];
                                            
                                            if excess_value != canonical_value {
                                                // Add sameAs assertion
                                                self.derived_graph.add_triple(Triple {
                                                    subject: excess_value.clone(),
                                                    predicate: RdfTerm::iri("http://www.w3.org/2002/07/owl#sameAs")?,
                                                    object: canonical_value.clone()
                                                });
                                            }
                                        }
                                    }
                                }
                                
                                // For maxCardinality 1 (functional-like), ensure uniqueness
                                if cardinality == 1 && property_values.len() > 1 {
                                    // All values must be the same
                                    let canonical_value = &property_values[0];
                                    for value in &property_values[1..] {
                                        if value != canonical_value {
                                            self.derived_graph.add_triple(Triple {
                                                subject: value.clone(),
                                                predicate: RdfTerm::iri("http://www.w3.org/2002/07/owl#sameAs")?,
                                                object: canonical_value.clone()
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Rule prp-fp: Functional property reasoning
    fn apply_prp_functional(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let functional_iri = RdfTerm::Iri(OWL_FUNCTIONAL_PROPERTY.clone());
        let same_as_iri = RdfTerm::Iri(OWL_SAME_AS.clone());

        // Find all functional properties
        let functional_props = graph.find_triples(None, Some(&type_iri), Some(&functional_iri));
        
        for func_prop_triple in functional_props {
            let property = &func_prop_triple.subject;
            
            // Find all uses of this property
            let property_uses = graph.find_triples(None, Some(property), None);
            
            // Group by subject
            let mut subject_objects: HashMap<&RdfTerm, Vec<&RdfTerm>> = HashMap::new();
            for use_triple in property_uses {
                subject_objects
                    .entry(&use_triple.subject)
                    .or_insert_with(Vec::new)
                    .push(&use_triple.object);
            }
            
            // For each subject with multiple objects, add sameAs statements
            for (_, objects) in subject_objects {
                if objects.len() > 1 {
                    for i in 0..objects.len() {
                        for j in i + 1..objects.len() {
                            let derived_triple = Triple {
                                subject: objects[i].clone(),
                                predicate: same_as_iri.clone(),
                                object: objects[j].clone(),
                            };
                            
                            self.add_derived_triple(derived_triple);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Rule prp-ifp: Inverse functional property reasoning
    fn apply_prp_inverse_functional(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced inverse functional property reasoning
        // If P is inverse functional and (x P y) and (z P y), then x = z
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let inv_functional_iri = RdfTerm::Iri(OWL_INVERSE_FUNCTIONAL_PROPERTY.clone());
        
        // Find all inverse functional properties
        let inv_functional_props = graph.find_triples(None, Some(&type_iri), Some(&inv_functional_iri));
        
        for inv_func_prop_triple in inv_functional_props {
            let property = &inv_func_prop_triple.subject;
            
            // Group all property assertions by their object (range)
            let mut object_subjects: std::collections::HashMap<RdfTerm, Vec<RdfTerm>> = 
                std::collections::HashMap::new();
            
            for triple in &graph.triples {
                if triple.predicate == *property {
                    object_subjects
                        .entry(triple.object.clone())
                        .or_insert_with(Vec::new)
                        .push(triple.subject.clone());
                }
            }
            
            // For each object that has multiple subjects, infer that the subjects are the same
            for (_, subjects) in object_subjects {
                if subjects.len() > 1 {
                    // All subjects must be the same individual
                    for i in 0..subjects.len() {
                        for j in i + 1..subjects.len() {
                            if subjects[i] != subjects[j] {
                                // Add sameAs assertion
                                let derived_triple = Triple {
                                    subject: subjects[i].clone(),
                                    predicate: RdfTerm::iri("http://www.w3.org/2002/07/owl#sameAs")?,
                                    object: subjects[j].clone(),
                                };
                                
                                self.add_derived_triple(derived_triple);
                                
                                // Also add the symmetric assertion
                                let symmetric_triple = Triple {
                                    subject: subjects[j].clone(),
                                    predicate: RdfTerm::iri("http://www.w3.org/2002/07/owl#sameAs")?,
                                    object: subjects[i].clone(),
                                };
                                
                                self.add_derived_triple(symmetric_triple);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Rule prp-symp: Symmetric property reasoning
    fn apply_prp_symmetric(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let symmetric_iri = RdfTerm::Iri(OWL_SYMMETRIC_PROPERTY.clone());

        // Find all symmetric properties
        let symmetric_props = graph.find_triples(None, Some(&type_iri), Some(&symmetric_iri));
        
        for sym_prop_triple in symmetric_props {
            let property = &sym_prop_triple.subject;
            
            // Find all uses of this property
            let property_uses = graph.find_triples(None, Some(property), None);
            
            // For each (x P y), add (y P x)
            for use_triple in property_uses {
                let derived_triple = Triple {
                    subject: use_triple.object.clone(),
                    predicate: property.clone(),
                    object: use_triple.subject.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule prp-trp: Transitive property reasoning
    fn apply_prp_transitive(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let transitive_iri = RdfTerm::Iri(OWL_TRANSITIVE_PROPERTY.clone());

        // Find all transitive properties
        let transitive_props = graph.find_triples(None, Some(&type_iri), Some(&transitive_iri));
        
        for trans_prop_triple in transitive_props {
            let property = &trans_prop_triple.subject;
            
            // Find all uses of this property
            let property_uses: Vec<_> = graph.find_triples(None, Some(property), None).into_iter().cloned().collect();
            
            // For each pair (x P y) and (y P z), add (x P z)
            for use1 in &property_uses {
                for use2 in &property_uses {
                    if use1.object == use2.subject {
                        let derived_triple = Triple {
                            subject: use1.subject.clone(),
                            predicate: property.clone(),
                            object: use2.object.clone(),
                        };
                        
                        self.add_derived_triple(derived_triple);
                    }
                }
            }
        }

        Ok(())
    }

    /// Rule prp-spo1: Subproperty reasoning
    fn apply_prp_subproperty(&mut self, graph: &RdfGraph) -> Result<()> {
        use super::vocabulary::*;
        
        let subprop_iri = RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone());

        let subprop_triples = graph.find_triples(None, Some(&subprop_iri), None);
        
        for subprop_triple in subprop_triples {
            let subproperty = &subprop_triple.subject;
            let superproperty = &subprop_triple.object;
            
            // Find all uses of the subproperty
            let subprop_uses = graph.find_triples(None, Some(subproperty), None);
            
            for use_triple in subprop_uses {
                let derived_triple = Triple {
                    subject: use_triple.subject.clone(),
                    predicate: superproperty.clone(),
                    object: use_triple.object.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule prp-eqp1: Equivalent property reasoning
    fn apply_prp_equivalent_property(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced equivalent property reasoning
        // If P owl:equivalentProperty Q and (x P y), then (x Q y)
        // If P owl:equivalentProperty Q and (x Q y), then (x P y)
        use super::vocabulary::*;
        
        let equiv_prop_iri = RdfTerm::Iri(OWL_EQUIVALENT_PROPERTY.clone());
        
        // Find all equivalent property assertions
        let equiv_assertions = graph.find_triples(None, Some(&equiv_prop_iri), None);
        
        for equiv_triple in equiv_assertions {
            let property1 = &equiv_triple.subject;
            let property2 = &equiv_triple.object;
            
            // Rule 1: If (x P y), then (x Q y)
            let property1_uses = graph.find_triples(None, Some(property1), None);
            for use_triple in property1_uses {
                let derived_triple = Triple {
                    subject: use_triple.subject.clone(),
                    predicate: property2.clone(),
                    object: use_triple.object.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
            
            // Rule 2: If (x Q y), then (x P y)
            let property2_uses = graph.find_triples(None, Some(property2), None);
            for use_triple in property2_uses {
                let derived_triple = Triple {
                    subject: use_triple.subject.clone(),
                    predicate: property1.clone(),
                    object: use_triple.object.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
            
            // Also ensure reflexivity for transitivity of equivalence
            // P owl:equivalentProperty P
            let reflexive_triple1 = Triple {
                subject: property1.clone(),
                predicate: equiv_prop_iri.clone(),
                object: property1.clone(),
            };
            self.add_derived_triple(reflexive_triple1);
            
            let reflexive_triple2 = Triple {
                subject: property2.clone(),
                predicate: equiv_prop_iri.clone(),
                object: property2.clone(),
            };
            self.add_derived_triple(reflexive_triple2);
            
            // Ensure symmetry: if P equiv Q, then Q equiv P
            let symmetric_triple = Triple {
                subject: property2.clone(),
                predicate: equiv_prop_iri.clone(),
                object: property1.clone(),
            };
            self.add_derived_triple(symmetric_triple);
        }
        
        Ok(())
    }

    /// Rule prp-inv1: Inverse property reasoning
    fn apply_prp_inverse_property(&mut self, graph: &RdfGraph) -> Result<()> {
        // Enhanced inverse property reasoning
        // If P owl:inverseOf Q and (x P y), then (y Q x)
        // If P owl:inverseOf Q and (y Q x), then (x P y)
        use super::vocabulary::*;
        
        let inverse_iri = RdfTerm::Iri(OWL_INVERSE_OF.clone());
        
        // Find all inverse property assertions
        let inverse_assertions = graph.find_triples(None, Some(&inverse_iri), None);
        
        for inverse_triple in inverse_assertions {
            let property1 = &inverse_triple.subject;
            let property2 = &inverse_triple.object;
            
            // Rule 1: If (x P y), then (y Q x)
            let property1_uses = graph.find_triples(None, Some(property1), None);
            for use_triple in property1_uses {
                let derived_triple = Triple {
                    subject: use_triple.object.clone(),
                    predicate: property2.clone(),
                    object: use_triple.subject.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
            
            // Rule 2: If (y Q x), then (x P y)
            let property2_uses = graph.find_triples(None, Some(property2), None);
            for use_triple in property2_uses {
                let derived_triple = Triple {
                    subject: use_triple.object.clone(),
                    predicate: property1.clone(),
                    object: use_triple.subject.clone(),
                };
                
                self.add_derived_triple(derived_triple);
            }
            
            // Ensure symmetry: if P inverseOf Q, then Q inverseOf P
            let symmetric_triple = Triple {
                subject: property2.clone(),
                predicate: inverse_iri.clone(),
                object: property1.clone(),
            };
            self.add_derived_triple(symmetric_triple);
        }
        
        Ok(())
    }

    /// Get the closure (input + derived facts)
    pub fn closure(&self) -> RdfGraph {
        let mut closure = self.input_graph.clone();
        closure.merge(&self.derived_graph);
        closure
    }
    
    /// Helper method to check if a term is a specific IRI
    fn is_iri(&self, term: &RdfTerm, iri_str: &str) -> bool {
        match term {
            RdfTerm::Iri(iri) => iri.as_str() == iri_str,
            _ => false,
        }
    }
    
    /// Find the onProperty value for a restriction
    fn find_on_property(&self, graph: &RdfGraph, restriction: &RdfTerm) -> Option<RdfTerm> {
        for triple in &graph.triples {
            if triple.subject == *restriction 
                && self.is_iri(&triple.predicate, "http://www.w3.org/2002/07/owl#onProperty") {
                return Some(triple.object.clone());
            }
        }
        None
    }
    
    /// Extract cardinality value from an RDF term
    fn extract_cardinality_value(&self, term: &RdfTerm) -> Option<usize> {
        match term {
            RdfTerm::Literal { value, .. } => {
                // Try to parse as integer
                if let Ok(parsed_value) = value.parse::<usize>() {
                    Some(parsed_value)
                } else {
                    None
                }
            },
            RdfTerm::Iri(iri) => {
                // Handle specific IRIs for common cardinalities
                match iri.as_str() {
                    "http://www.w3.org/2002/07/owl#zero" => Some(0),
                    "http://www.w3.org/2002/07/owl#one" => Some(1),
                    _ => None,
                }
            },
            _ => None,
        }
    }
    
    /// Parse an RDF list into a vector of terms (simplified implementation)
    fn parse_rdf_list(&self, list_term: &RdfTerm, graph: &RdfGraph) -> Option<Vec<RdfTerm>> {
        // This is a simplified implementation that doesn't fully parse RDF lists
        // A complete implementation would follow rdf:first/rdf:rest chains
        
        // For now, return None to indicate that list parsing is not fully implemented
        // This should be enhanced to properly parse RDF lists following the RDF specification
        None
    }
    
    /// Find the property associated with a restriction
    fn find_restriction_property(&self, restriction: &RdfTerm, graph: &RdfGraph) -> Option<crate::ontology::IRI> {
        for triple in &graph.triples {
            if triple.subject == *restriction &&
               self.is_iri(&triple.predicate, "http://www.w3.org/2002/07/owl#onProperty") {
                if let RdfTerm::Iri(prop_iri) = &triple.object {
                    return Some(crate::ontology::IRI::from(prop_iri.clone()));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entailment_checker_regimes() {
        let mut checker = EntailmentChecker::new(EntailmentRegime::RdfSimple);
        assert_eq!(checker.regime(), EntailmentRegime::RdfSimple);
        
        checker.set_regime(EntailmentRegime::Rdfs);
        assert_eq!(checker.regime(), EntailmentRegime::Rdfs);
    }

    #[test]
    fn test_rdfs_entailment() {
        let mut premises = RdfGraph::new();
        let conclusion = RdfGraph::new();
        
        // Add some RDFS reasoning scenario
        // This is a simplified test
        
        let mut checker = EntailmentChecker::new(EntailmentRegime::Rdfs);
        let result = checker.entails(&premises, &conclusion).unwrap();
        
        // Empty conclusion should be entailed by any premises
        assert!(result);
    }

    #[test]
    fn test_owl_rl_engine() {
        let graph = RdfGraph::new();
        let mut engine = Owl2RlEngine::new(graph);
        
        // Test that reasoning completes without error
        let result = engine.reason();
        assert!(result.is_ok());
    }
}
