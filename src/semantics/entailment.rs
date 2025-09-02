//! Entailment Relations Implementation
//!
//! This module implements various entailment relations for RDF, RDFS, and OWL
//! according to the W3C specifications.

use super::{RdfGraph, RdfTerm, Triple, owl2::Owl2ReasoningEngine};
use crate::{Error, Result, ontology::{Axiom, Ontology}};
use std::collections::{HashMap, HashSet};
use itertools::Itertools;

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

    /// Filter axioms of a specific type from the ontology
    fn get_class_assertions<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ClassAssertionAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::ClassAssertion(ca) = axiom {
                Some(ca)
            } else {
                None
            }
        })
    }

    fn get_object_property_assertions<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ObjectPropertyAssertionAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(opa) = axiom {
                Some(opa)
            } else {
                None
            }
        })
    }

    fn get_data_property_assertions<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::DataPropertyAssertionAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::DataPropertyAssertion(dpa) = axiom {
                Some(dpa)
            } else {
                None
            }
        })
    }

    fn get_subclass_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubClassOfAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::SubClassOf(sco) = axiom {
                Some(sco)
            } else {
                None
            }
        })
    }

    fn get_disjoint_classes_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::DisjointClassesAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::DisjointClasses(dc) = axiom {
                Some(dc)
            } else {
                None
            }
        })
    }

    fn get_equivalent_classes_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::EquivalentClassesAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::EquivalentClasses(ec) = axiom {
                Some(ec)
            } else {
                None
            }
        })
    }

    fn get_sub_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubObjectPropertyOfAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::SubObjectPropertyOf(sop) = axiom {
                Some(sop)
            } else {
                None
            }
        })
    }

    fn get_equivalent_object_properties_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::EquivalentObjectPropertiesAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::EquivalentObjectProperties(eop) = axiom {
                Some(eop)
            } else {
                None
            }
        })
    }

    fn get_disjoint_object_properties_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::DisjointObjectPropertiesAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::DisjointObjectProperties(dop) = axiom {
                Some(dop)
            } else {
                None
            }
        })
    }

    fn get_inverse_object_properties_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::InverseObjectPropertiesAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::InverseObjectProperties(iop) = axiom {
                Some(iop)
            } else {
                None
            }
        })
    }

    fn get_same_individual_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SameIndividualAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::SameIndividual(si) = axiom {
                Some(si)
            } else {
                None
            }
        })
    }

    fn get_different_individuals_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::DifferentIndividualsAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::DifferentIndividuals(di) = axiom {
                Some(di)
            } else {
                None
            }
        })
    }

    // Add more filtering methods for all axiom types
    fn get_sub_data_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubDataPropertyOfAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::SubDataPropertyOf(sdp) = axiom {
                Some(sdp)
            } else {
                None
            }
        })
    }

    fn get_functional_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::FunctionalObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::FunctionalObjectProperty(fop) = axiom {
                Some(fop)
            } else {
                None
            }
        })
    }

    fn get_inverse_functional_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::InverseFunctionalObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::InverseFunctionalObjectProperty(ifop) = axiom {
                Some(ifop)
            } else {
                None
            }
        })
    }

    fn get_reflexive_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ReflexiveObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::ReflexiveObjectProperty(rop) = axiom {
                Some(rop)
            } else {
                None
            }
        })
    }

    fn get_irreflexive_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::IrreflexiveObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::IrreflexiveObjectProperty(irop) = axiom {
                Some(irop)
            } else {
                None
            }
        })
    }

    fn get_symmetric_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SymmetricObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::SymmetricObjectProperty(sop) = axiom {
                Some(sop)
            } else {
                None
            }
        })
    }

    fn get_asymmetric_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::AsymmetricObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::AsymmetricObjectProperty(asop) = axiom {
                Some(asop)
            } else {
                None
            }
        })
    }

    fn get_transitive_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::TransitiveObjectPropertyAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::TransitiveObjectProperty(top) = axiom {
                Some(top)
            } else {
                None
            }
        })
    }

    fn get_object_property_domain_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ObjectPropertyDomainAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::ObjectPropertyDomain(opd) = axiom {
                Some(opd)
            } else {
                None
            }
        })
    }

    fn get_object_property_range_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ObjectPropertyRangeAxiom> + 'a {
        ontology.axioms.iter().filter_map(|axiom| {
            if let crate::ontology::axioms::Axiom::ObjectPropertyRange(opr) = axiom {
                Some(opr)
            } else {
                None
            }
        })
    }

    // TODO: Add SubPropertyChainOfAxiom support when it's implemented
    // fn get_sub_property_chain_of_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubPropertyChainOfAxiom> + 'a {
    //     ontology.axioms.iter().filter_map(|axiom| {
    //         if let crate::ontology::axioms::Axiom::SubPropertyChainOf(spco) = axiom {
    //             Some(spco)
    //         } else {
    //             None
    //         }
    //     })
    // }

    // Helper methods for checking relationships and properties
    fn is_individual_in_class(&self, _ontology: &Ontology, _individual: &crate::ontology::Individual, _class: &crate::ontology::ClassExpression) -> Result<bool> {
        // TODO: Implement proper class membership checking
        Ok(false)
    }

    fn property_sets_overlap<T>(&self, _set1: &[T], _set2: &[T]) -> bool 
    where T: PartialEq {
        // TODO: Implement proper set overlap checking
        false
    }

    fn axiom_sets_overlap<T>(&self, _set1: &[T], _set2: &[T]) -> bool 
    where T: PartialEq {
        // TODO: Implement proper axiom set overlap checking
        false
    }

    fn individual_sets_overlap(&self, _set1: &[crate::ontology::Individual], _set2: &[crate::ontology::Individual]) -> bool {
        // TODO: Implement proper individual set overlap checking
        false
    }

    fn is_property_subproperty_by_transitivity(&self, _ontology: &Ontology, _sub: &crate::ontology::ObjectPropertyExpression, _super_: &crate::ontology::ObjectPropertyExpression) -> Result<bool> {
        // TODO: Implement transitivity checking
        Ok(false)
    }

    fn is_data_property_subproperty_by_transitivity(&self, _ontology: &Ontology, _sub: &crate::ontology::DataPropertyExpression, _super_: &crate::ontology::DataPropertyExpression) -> Result<bool> {
        // TODO: Implement data property transitivity checking
        Ok(false)
    }

    fn are_individuals_same(&self, _ontology: &Ontology, _ind1: &crate::ontology::Individual, _ind2: &crate::ontology::Individual) -> Result<bool> {
        // TODO: Implement individual equality checking
        Ok(false)
    }

    fn is_class_subclass_of(&self, _ontology: &Ontology, _sub: &crate::ontology::ClassExpression, _super_: &crate::ontology::ClassExpression) -> Result<bool> {
        // TODO: Implement class subsumption checking
        Ok(false)
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
    
    /// Convert RDF graph to OWL axioms (comprehensive implementation)
    fn rdf_graph_to_owl_axioms(&self, graph: &RdfGraph) -> Result<Vec<crate::ontology::Axiom>> {
        let mut axioms = Vec::new();
        
        // Extract class assertions (rdf:type statements)
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") {
                if let Ok(axiom) = self.create_class_assertion_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract subclass relationships (rdfs:subClassOf)
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/2000/01/rdf-schema#subClassOf") {
                if let Ok(axiom) = self.create_subclass_axiom_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract object property assertions
        for triple in &graph.triples {
            if self.is_object_property_predicate(&triple.predicate) {
                if let Ok(axiom) = self.create_object_property_assertion_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract data property assertions
        for triple in &graph.triples {
            if self.is_data_property_predicate(&triple.predicate) {
                if let Ok(axiom) = self.create_data_property_assertion_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract property domain/range axioms
        for triple in &graph.triples {
            match triple.predicate.as_str() {
                Some("http://www.w3.org/2000/01/rdf-schema#domain") => {
                    if let Ok(axiom) = self.create_property_domain_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2000/01/rdf-schema#range") => {
                    if let Ok(axiom) = self.create_property_range_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                _ => {}
            }
        }
        
        // Extract OWL-specific constructs
        for triple in &graph.triples {
            match triple.predicate.as_str() {
                Some("http://www.w3.org/2002/07/owl#equivalentClass") => {
                    if let Ok(axiom) = self.create_equivalent_classes_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#disjointWith") => {
                    if let Ok(axiom) = self.create_disjoint_classes_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#equivalentProperty") => {
                    if let Ok(axiom) = self.create_equivalent_properties_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#inverseOf") => {
                    if let Ok(axiom) = self.create_inverse_object_properties_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#sameAs") => {
                    if let Ok(axiom) = self.create_same_individual_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#differentFrom") => {
                    if let Ok(axiom) = self.create_different_individuals_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                _ => {}
            }
        }
        
        // Extract cardinality restrictions
        for triple in &graph.triples {
            if self.is_cardinality_restriction(&triple.predicate) {
                if let Ok(axiom) = self.create_cardinality_axiom_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        // Extract complex class expressions (intersections, unions, complements)
        for triple in &graph.triples {
            match triple.predicate.as_str() {
                Some("http://www.w3.org/2002/07/owl#intersectionOf") => {
                    if let Ok(axiom) = self.create_intersection_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#unionOf") => {
                    if let Ok(axiom) = self.create_union_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                Some("http://www.w3.org/2002/07/owl#complementOf") => {
                    if let Ok(axiom) = self.create_complement_axiom_from_triple(triple) {
                        axioms.push(axiom);
                    }
                }
                _ => {}
            }
        }
        
        // Extract property chains
        for triple in &graph.triples {
            if triple.predicate.as_str() == Some("http://www.w3.org/2002/07/owl#propertyChainAxiom") {
                if let Ok(axiom) = self.create_property_chain_axiom_from_triple(triple) {
                    axioms.push(axiom);
                }
            }
        }
        
        Ok(axioms)
    }
    
    /// Check if a predicate represents an object property
    fn is_object_property_predicate(&self, predicate: &RdfTerm) -> bool {
        // Check against known object properties in the ontology
        // This is a simplified check - would need proper reasoning in full implementation
        if let Some(iri) = predicate.as_str() {
            // Skip RDF/RDFS/OWL built-in properties
            !iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#") &&
            !iri.starts_with("http://www.w3.org/2000/01/rdf-schema#") &&
            !iri.starts_with("http://www.w3.org/2002/07/owl#")
        } else {
            false
        }
    }
    
    /// Check if a predicate represents a data property
    fn is_data_property_predicate(&self, predicate: &RdfTerm) -> bool {
        // Similar to object properties but for data properties
        // Would need proper reasoning to distinguish in full implementation
        false // Simplified for now
    }
    
    /// Check if a predicate represents a cardinality restriction
    fn is_cardinality_restriction(&self, predicate: &RdfTerm) -> bool {
        if let Some(iri) = predicate.as_str() {
            matches!(iri,
                "http://www.w3.org/2002/07/owl#minCardinality" |
                "http://www.w3.org/2002/07/owl#maxCardinality" |
                "http://www.w3.org/2002/07/owl#cardinality" |
                "http://www.w3.org/2002/07/owl#minQualifiedCardinality" |
                "http://www.w3.org/2002/07/owl#maxQualifiedCardinality" |
                "http://www.w3.org/2002/07/owl#qualifiedCardinality"
            )
        } else {
            false
        }
    }
    
    /// Create class assertion axiom from RDF triple
    pub fn create_class_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
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
    pub fn create_subclass_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
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

    /// Create object property assertion axiom from RDF triple
    pub fn create_object_property_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject = self.rdf_term_to_individual(&triple.subject)?;
        let object = self.rdf_term_to_individual(&triple.object)?;
        let property = self.rdf_term_to_object_property_expression(&triple.predicate)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyAssertion(
            crate::ontology::axioms::ObjectPropertyAssertionAxiom {
                id: 0,
                source: subject,
                target: object,
                property,
                annotations: vec![],
            }
        ))
    }

    /// Create data property assertion axiom from RDF triple
    pub fn create_data_property_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject = self.rdf_term_to_individual(&triple.subject)?;
        let target = self.rdf_term_to_literal(&triple.object)?;
        let property = self.rdf_term_to_data_property_expression(&triple.predicate)?;
        
        Ok(crate::ontology::Axiom::DataPropertyAssertion(
            crate::ontology::axioms::DataPropertyAssertionAxiom {
                id: 0,
                individual: subject,
                property,
                value: target,
                annotations: vec![],
            }
        ))
    }

    /// Create property domain axiom from RDF triple
    pub fn create_property_domain_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let property = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let domain = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyDomain(
            crate::ontology::axioms::ObjectPropertyDomainAxiom {
                id: 0,
                property,
                domain,
                annotations: vec![],
            }
        ))
    }

    /// Create property range axiom from RDF triple
    pub fn create_property_range_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let property = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let range = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyRange(
            crate::ontology::axioms::ObjectPropertyRangeAxiom {
                id: 0,
                property,
                range,
                annotations: vec![],
            }
        ))
    }

    /// Create equivalent classes axiom from RDF triple
    pub fn create_equivalent_classes_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let class1 = self.rdf_term_to_class_expression(&triple.subject)?;
        let class2 = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::EquivalentClasses(
            crate::ontology::axioms::EquivalentClassesAxiom {
                id: 0,
                classes: vec![class1, class2],
                annotations: vec![],
            }
        ))
    }

    /// Create disjoint classes axiom from RDF triple
    pub fn create_disjoint_classes_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let class1 = self.rdf_term_to_class_expression(&triple.subject)?;
        let class2 = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::DisjointClasses(
            crate::ontology::axioms::DisjointClassesAxiom {
                id: 0,
                classes: vec![class1, class2],
                annotations: vec![],
            }
        ))
    }

    /// Create equivalent properties axiom from RDF triple
    pub fn create_equivalent_properties_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let prop1 = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let prop2 = self.rdf_term_to_object_property_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::EquivalentObjectProperties(
            crate::ontology::axioms::EquivalentObjectPropertiesAxiom {
                id: 0,
                properties: vec![prop1, prop2],
                annotations: vec![],
            }
        ))
    }

    /// Create inverse object properties axiom from RDF triple
    pub fn create_inverse_object_properties_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let prop1 = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let prop2 = self.rdf_term_to_object_property_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::InverseObjectProperties(
            crate::ontology::axioms::InverseObjectPropertiesAxiom {
                id: 0,
                property1: prop1,
                property2: prop2,
                annotations: vec![],
            }
        ))
    }

    /// Create same individual axiom from RDF triple
    pub fn create_same_individual_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let ind1 = self.rdf_term_to_individual(&triple.subject)?;
        let ind2 = self.rdf_term_to_individual(&triple.object)?;
        
        Ok(crate::ontology::Axiom::SameIndividual(
            crate::ontology::axioms::SameIndividualAxiom {
                id: 0,
                individuals: vec![ind1, ind2],
                annotations: vec![],
            }
        ))
    }

    /// Create different individuals axiom from RDF triple
    pub fn create_different_individuals_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let ind1 = self.rdf_term_to_individual(&triple.subject)?;
        let ind2 = self.rdf_term_to_individual(&triple.object)?;
        
        Ok(crate::ontology::Axiom::DifferentIndividuals(
            crate::ontology::axioms::DifferentIndividualsAxiom {
                id: 0,
                individuals: vec![ind1, ind2],
                annotations: vec![],
            }
        ))
    }

    /// Create cardinality axiom from RDF triple (placeholder)
    pub fn create_cardinality_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        Err(Error::ontology_parsing("Cardinality axiom creation not implemented"))
    }

    /// Create intersection axiom from RDF triple (placeholder)
    pub fn create_intersection_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        Err(Error::ontology_parsing("Intersection axiom creation not implemented"))
    }

    /// Create union axiom from RDF triple (placeholder)
    pub fn create_union_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        Err(Error::ontology_parsing("Union axiom creation not implemented"))
    }

    /// Create complement axiom from RDF triple (placeholder)
    pub fn create_complement_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        Err(Error::ontology_parsing("Complement axiom creation not implemented"))
    }

    /// Create property chain axiom from RDF triple (placeholder)
    pub fn create_property_chain_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        Err(Error::ontology_parsing("Property chain axiom creation not implemented"))
    }
    
    /// Check if an axiom is entailed by an ontology (comprehensive check)
    fn is_axiom_entailed(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::Axiom) -> Result<bool> {
        // First check for exact axiom match
        for ont_axiom in ontology.axioms() {
            if ont_axiom == axiom {
                return Ok(true);
            }
        }
        
        // Check for entailments based on axiom type
        match axiom {
            crate::ontology::Axiom::ClassAssertion(assertion) => {
                self.check_class_assertion_entailment(ontology, assertion)
            }
            crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                self.check_object_property_assertion_entailment(ontology, assertion)
            }
            crate::ontology::Axiom::DataPropertyAssertion(assertion) => {
                self.check_data_property_assertion_entailment(ontology, assertion)
            }
            crate::ontology::Axiom::SubClassOf(axiom) => {
                self.check_subclass_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::EquivalentClasses(axiom) => {
                self.check_equivalent_classes_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::DisjointClasses(axiom) => {
                self.check_disjoint_classes_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::SubObjectPropertyOf(axiom) => {
                self.check_sub_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::EquivalentObjectProperties(axiom) => {
                self.check_equivalent_object_properties_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::DisjointObjectProperties(axiom) => {
                self.check_disjoint_object_properties_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::InverseObjectProperties(axiom) => {
                self.check_inverse_object_properties_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::ObjectPropertyDomain(axiom) => {
                self.check_object_property_domain_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::ObjectPropertyRange(axiom) => {
                self.check_object_property_range_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::FunctionalObjectProperty(axiom) => {
                self.check_functional_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::InverseFunctionalObjectProperty(axiom) => {
                self.check_inverse_functional_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::ReflexiveObjectProperty(axiom) => {
                self.check_reflexive_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::IrreflexiveObjectProperty(axiom) => {
                self.check_irreflexive_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::SymmetricObjectProperty(axiom) => {
                self.check_symmetric_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::AsymmetricObjectProperty(axiom) => {
                self.check_asymmetric_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::TransitiveObjectProperty(axiom) => {
                self.check_transitive_object_property_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::SameIndividual(axiom) => {
                self.check_same_individual_entailment(ontology, axiom)
            }
            crate::ontology::Axiom::DifferentIndividuals(axiom) => {
                self.check_different_individuals_entailment(ontology, axiom)
            }
            _ => Ok(false), // Other axiom types not implemented yet
        }
    }
    
    /// Check class assertion entailment
    fn check_class_assertion_entailment(&self, ontology: &crate::ontology::Ontology, assertion: &crate::ontology::axioms::ClassAssertionAxiom) -> Result<bool> {
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
        
        // Check through equivalences
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::EquivalentClasses(equiv) = ont_axiom {
                for class_expr in &equiv.classes {
                    if self.are_class_expressions_equivalent(class_expr, &assertion.class, ontology)? {
                        // Check if individual is member of any equivalent class
                        for other_class in &equiv.classes {
                            if class_expr != other_class {
                                for check_axiom in ontology.axioms() {
                                    if let crate::ontology::Axiom::ClassAssertion(check_assertion) = check_axiom {
                                        if check_assertion.individual == assertion.individual &&
                                           self.are_class_expressions_equivalent(other_class, &check_assertion.class, ontology)? {
                                            return Ok(true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check object property assertion entailment
    fn check_object_property_assertion_entailment(&self, ontology: &crate::ontology::Ontology, assertion: &crate::ontology::axioms::ObjectPropertyAssertionAxiom) -> Result<bool> {
        // Check through sub-property relationships
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::ObjectPropertyAssertion(ont_assertion) = ont_axiom {
                if ont_assertion.source == assertion.source && ont_assertion.target == assertion.target {
                    if self.is_sub_object_property_in_ontology(&ont_assertion.property, &assertion.property, ontology)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        // Check through property equivalences
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::EquivalentObjectProperties(equiv) = ont_axiom {
                for prop_expr in &equiv.properties {
                    if self.are_object_property_expressions_equivalent(prop_expr, &assertion.property, ontology)? {
                        for other_prop in &equiv.properties {
                            if prop_expr != other_prop {
                                for check_axiom in ontology.axioms() {
                                    if let crate::ontology::Axiom::ObjectPropertyAssertion(check_assertion) = check_axiom {
                                        if check_assertion.source == assertion.source &&
                                           check_assertion.target == assertion.target &&
                                           self.are_object_property_expressions_equivalent(other_prop, &check_assertion.property, ontology)? {
                                            return Ok(true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check through inverse properties
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::InverseObjectProperties(inverse) = ont_axiom {
                if self.are_object_property_expressions_equivalent(&inverse.property1, &assertion.property, ontology)? {
                    // Check for assertion with swapped subject/object using second property
                    for check_axiom in ontology.axioms() {
                        if let crate::ontology::Axiom::ObjectPropertyAssertion(check_assertion) = check_axiom {
                            if check_assertion.source == assertion.target &&
                               check_assertion.target == assertion.source &&
                               self.are_object_property_expressions_equivalent(&inverse.property2, &check_assertion.property, ontology)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check data property assertion entailment  
    fn check_data_property_assertion_entailment(&self, ontology: &crate::ontology::Ontology, assertion: &crate::ontology::axioms::DataPropertyAssertionAxiom) -> Result<bool> {
        // Check through sub-property relationships
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::DataPropertyAssertion(ont_assertion) = ont_axiom {
                if ont_assertion.individual == assertion.individual && ont_assertion.value == assertion.value {
                    if self.is_sub_data_property_in_ontology(&ont_assertion.property, &assertion.property, ontology)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check subclass entailment
    fn check_subclass_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::SubClassOfAxiom) -> Result<bool> {
        // Direct transitivity check
        if self.is_subclass_in_ontology(&axiom.subclass, &axiom.superclass, ontology)? {
            return Ok(true);
        }
        
        // Check through equivalences
        for ont_axiom in ontology.axioms() {
            if let crate::ontology::Axiom::EquivalentClasses(equiv) = ont_axiom {
                for class_expr in &equiv.classes {
                    if self.are_class_expressions_equivalent(class_expr, &axiom.subclass, ontology)? {
                        for other_class in &equiv.classes {
                            if class_expr != other_class {
                                if self.is_subclass_in_ontology(other_class, &axiom.superclass, ontology)? {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check equivalent classes entailment
    fn check_equivalent_classes_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::EquivalentClassesAxiom) -> Result<bool> {
        // Check if all pairs are equivalent through transitivity
        for (i, class1) in axiom.classes.iter().enumerate() {
            for (j, class2) in axiom.classes.iter().enumerate() {
                if i != j && !self.are_class_expressions_equivalent(class1, class2, ontology)? {
                    return Ok(false);
                }
            }
        }
        
        Ok(true)
    }
    
    /// Helper methods for checking equivalences and subsumptions
    fn is_subclass_in_ontology(&self, sub: &crate::ontology::ClassExpression, super_: &crate::ontology::ClassExpression, ontology: &crate::ontology::Ontology) -> Result<bool> {
        // Check direct subclass relationships
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                if self.are_class_expressions_equivalent(&subclass_axiom.subclass, sub, ontology)? &&
                   self.are_class_expressions_equivalent(&subclass_axiom.superclass, super_, ontology)? {
                    return Ok(true);
                }
            }
        }
        
        // Check transitivity (simplified - would need full reasoning)
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                if self.are_class_expressions_equivalent(&subclass_axiom.subclass, sub, ontology)? {
                    if self.is_subclass_in_ontology(&subclass_axiom.superclass, super_, ontology)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Additional helper methods would be implemented similarly...
    fn check_disjoint_classes_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::DisjointClassesAxiom) -> Result<bool> {
        // Check if any individuals are asserted to be members of multiple disjoint classes
        for class_pair in axiom.classes.iter().combinations(2) {
            let class1 = &class_pair[0];
            let class2 = &class_pair[1];
            
            // Check if any individual is in both classes
            for (iri, individual) in &ontology.individuals() {
                let in_class1 = self.is_individual_in_class(ontology, individual, class1)?;
                let in_class2 = self.is_individual_in_class(ontology, individual, class2)?;
                
                if in_class1 && in_class2 {
                    return Ok(true); // Entailment violated (inconsistency)
                }
            }
        }
        
        // Check if classes are already known to be disjoint
        for existing_axiom in self.get_disjoint_classes_axioms(ontology) {
            if self.axiom_sets_overlap(&axiom.classes, &existing_axiom.classes) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn check_sub_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::SubObjectPropertyOfAxiom) -> Result<bool> {
        // Check if this sub-property relationship is already implied by existing axioms
        
        // Direct check - if axiom already exists
        for existing_axiom in self.get_sub_object_property_axioms(ontology) {
            if existing_axiom.sub_property == axiom.sub_property && 
               existing_axiom.super_property == axiom.super_property {
                return Ok(true);
            }
        }
        
        // Transitive check - if there's a chain that implies this relationship
        if self.is_property_subproperty_by_transitivity(ontology, &axiom.sub_property, &axiom.super_property)? {
            return Ok(true);
        }
        
        // TODO: Check property chain implications when SubPropertyChainOfAxiom is implemented
        // for chain_axiom in self.get_sub_property_chain_of_axioms(ontology) {
        //     if chain_axiom.super_property == axiom.super_property {
        //         // Check if the sub_property can be derived from the chain
        //         if chain_axiom.property_chain.len() == 1 && 
        //            chain_axiom.property_chain[0] == axiom.sub_property {
        //             return Ok(true);
        //         }
        //     }
        // }
        
        Ok(false)
    }
    
    fn check_equivalent_object_properties_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::EquivalentObjectPropertiesAxiom) -> Result<bool> {
        if axiom.properties.len() < 2 {
            return Ok(false);
        }
        
        // Check if equivalence is already established
        for existing_axiom in self.get_equivalent_object_properties_axioms(ontology) {
            if self.property_sets_overlap(&axiom.properties, &existing_axiom.properties) {
                return Ok(true);
            }
        }
        
        // Check if equivalence can be derived from sub-property relationships
        // If A ⊆ B and B ⊆ A, then A ≡ B
        for property_pair in axiom.properties.iter().combinations(2) {
            let prop1 = &property_pair[0];
            let prop2 = &property_pair[1];
            
            let prop1_sub_prop2 = self.check_sub_object_property_entailment(ontology, 
                &crate::ontology::axioms::SubObjectPropertyOfAxiom {
                    id: 0, // Using 0 as default axiom ID
                    sub_property: (*prop1).clone(),
                    super_property: (*prop2).clone(),
                    annotations: Vec::new(),
                })?;
            
            let prop2_sub_prop1 = self.check_sub_object_property_entailment(ontology,
                &crate::ontology::axioms::SubObjectPropertyOfAxiom {
                    id: 0, // Using 0 as default axiom ID
                    sub_property: (*prop2).clone(),
                    super_property: (*prop1).clone(),
                    annotations: Vec::new(),
                })?;
            
            if prop1_sub_prop2 && prop2_sub_prop1 {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn check_disjoint_object_properties_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::DisjointObjectPropertiesAxiom) -> Result<bool> {
        // Check if any property assertion violates disjointness
        for prop_pair in axiom.properties.iter().combinations(2) {
            let prop1 = &prop_pair[0];
            let prop2 = &prop_pair[1];
            
            // Check if any individual pair has both properties asserted
            for assertion in self.get_object_property_assertions(ontology) {
                if assertion.property == **prop1 {
                    // Check if the same individual pair also has prop2
                    for other_assertion in self.get_object_property_assertions(ontology) {
                        if other_assertion.property == **prop2 &&
                           other_assertion.source == assertion.source &&
                           other_assertion.target == assertion.target {
                            return Ok(true); // Violation found
                        }
                    }
                }
            }
        }
        
        // Check if disjointness is already established
        for existing_axiom in self.get_disjoint_object_properties_axioms(ontology) {
            if self.property_sets_overlap(&axiom.properties, &existing_axiom.properties) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn check_inverse_object_properties_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::InverseObjectPropertiesAxiom) -> Result<bool> {
        // Check if inverse relationship is already established
        for existing_axiom in self.get_inverse_object_properties_axioms(ontology) {
            if (existing_axiom.property1 == axiom.property1 && existing_axiom.property2 == axiom.property2) ||
               (existing_axiom.property1 == axiom.property2 && existing_axiom.property2 == axiom.property1) {
                return Ok(true);
            }
        }
        
        // Check if inverse relationship can be derived from property assertions
        // If (a, b) ∈ P and (b, a) ∈ Q for all instances, then P ≡ Q⁻¹
        let mut forward_pairs = std::collections::HashSet::new();
        let mut inverse_pairs = std::collections::HashSet::new();
        
        // Collect all assertions for both properties
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property1 {
                forward_pairs.insert((assertion.source.clone(), assertion.target.clone()));
            } else if assertion.property == axiom.property2 {
                inverse_pairs.insert((assertion.target.clone(), assertion.source.clone()));
            }
        }
        
        // Check if forward and inverse pairs match (simplified check)
        if !forward_pairs.is_empty() && forward_pairs == inverse_pairs {
            return Ok(true);
        }
        
        Ok(false)
    }
    
    fn check_object_property_domain_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::ObjectPropertyDomainAxiom) -> Result<bool> {
        // Check if domain constraint is already established
        for existing_axiom in self.get_object_property_domain_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                // Check if the new domain is subsumed by existing domain
                if self.is_class_subclass_of(ontology, &axiom.domain, &existing_axiom.domain)? {
                    return Ok(true);
                }
            }
        }
        
        // Check if domain constraint is implied by property assertions
        // If all subjects of property P are in class C, then domain(P) ⊆ C
        let mut subjects_in_domain = true;
        let mut has_assertions = false;
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                has_assertions = true;
                if !self.is_individual_in_class(ontology, &assertion.source, &axiom.domain)? {
                    subjects_in_domain = false;
                    break;
                }
            }
        }
        
        Ok(has_assertions && subjects_in_domain)
    }
    
    fn check_object_property_range_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::ObjectPropertyRangeAxiom) -> Result<bool> {
        // Check if range constraint is already established
        for existing_axiom in self.get_object_property_range_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                // Check if the new range is subsumed by existing range
                if self.is_class_subclass_of(ontology, &axiom.range, &existing_axiom.range)? {
                    return Ok(true);
                }
            }
        }
        
        // Check if range constraint is implied by property assertions
        // If all objects of property P are in class C, then range(P) ⊆ C
        let mut objects_in_range = true;
        let mut has_assertions = false;
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                has_assertions = true;
                if !self.is_individual_in_class(ontology, &assertion.target, &axiom.range)? {
                    objects_in_range = false;
                    break;
                }
            }
        }
        
        Ok(has_assertions && objects_in_range)
    }
    
    fn check_functional_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::FunctionalObjectPropertyAxiom) -> Result<bool> {
        // Check if functionality is already established
        for existing_axiom in self.get_functional_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if functionality is implied by property assertions
        // A property is functional if each subject has at most one object
        let mut subject_objects = std::collections::HashMap::new();
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                let entry = subject_objects.entry(assertion.source.clone()).or_insert_with(Vec::new);
                if !entry.contains(&assertion.target) {
                    entry.push(assertion.target.clone());
                }
                
                // If any subject has more than one distinct object, not functional
                if entry.len() > 1 {
                    return Ok(false);
                }
            }
        }
        
        // If we only have at most one object per subject, it's functionally consistent
        Ok(!subject_objects.is_empty())
    }
    
    fn check_inverse_functional_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::InverseFunctionalObjectPropertyAxiom) -> Result<bool> {
        // Check if inverse functionality is already established
        for existing_axiom in self.get_inverse_functional_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if inverse functionality is implied by property assertions
        // A property is inverse functional if each object has at most one subject
        let mut object_subjects = std::collections::HashMap::new();
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                let entry = object_subjects.entry(assertion.target.clone()).or_insert_with(Vec::new);
                if !entry.contains(&assertion.source) {
                    entry.push(assertion.source.clone());
                }
                
                // If any object has more than one distinct subject, not inverse functional
                if entry.len() > 1 {
                    return Ok(false);
                }
            }
        }
        
        // If we only have at most one subject per object, it's inverse functionally consistent
        Ok(!object_subjects.is_empty())
    }
    
    fn check_reflexive_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::ReflexiveObjectPropertyAxiom) -> Result<bool> {
        // Check if reflexivity is already established
        for existing_axiom in self.get_reflexive_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if reflexivity is implied by property assertions
        // A property is reflexive if for all individuals x in its domain, (x,x) is asserted
        let mut domain_individuals = std::collections::HashSet::new();
        let mut reflexive_pairs = std::collections::HashSet::new();
        
        // Collect domain individuals and reflexive pairs
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                domain_individuals.insert(assertion.source.clone());
                if assertion.source == assertion.target {
                    reflexive_pairs.insert(assertion.source.clone());
                }
            }
        }
        
        // Check if all domain individuals have reflexive pairs
        Ok(!domain_individuals.is_empty() && 
           domain_individuals.iter().all(|ind| reflexive_pairs.contains(ind)))
    }
    
    fn check_irreflexive_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::IrreflexiveObjectPropertyAxiom) -> Result<bool> {
        // Check if irreflexivity is already established
        for existing_axiom in self.get_irreflexive_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if irreflexivity is violated by property assertions
        // A property is irreflexive if no individual has (x,x) asserted
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property && assertion.source == assertion.target {
                return Ok(false); // Irreflexivity violated
            }
        }
        
        // If no reflexive pairs found, irreflexivity is consistent
        Ok(true)
    }
    
    fn check_symmetric_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::SymmetricObjectPropertyAxiom) -> Result<bool> {
        // Check if symmetry is already established
        for existing_axiom in self.get_symmetric_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if symmetry is implied by property assertions
        // A property is symmetric if whenever (x,y) is asserted, (y,x) is also asserted
        let mut forward_pairs = std::collections::HashSet::new();
        let mut reverse_pairs = std::collections::HashSet::new();
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                forward_pairs.insert((assertion.source.clone(), assertion.target.clone()));
                reverse_pairs.insert((assertion.target.clone(), assertion.source.clone()));
            }
        }
        
        // Check if every forward pair has a corresponding reverse pair
        Ok(!forward_pairs.is_empty() && 
           forward_pairs.iter().all(|(x, y)| reverse_pairs.contains(&(x.clone(), y.clone()))))
    }
    
    fn check_asymmetric_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::AsymmetricObjectPropertyAxiom) -> Result<bool> {
        // Check if asymmetry is already established
        for existing_axiom in self.get_asymmetric_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if asymmetry is violated by property assertions
        // A property is asymmetric if whenever (x,y) is asserted, (y,x) is not asserted
        let mut pairs = std::collections::HashSet::new();
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                let pair = (assertion.source.clone(), assertion.target.clone());
                let reverse_pair = (assertion.target.clone(), assertion.source.clone());
                
                if pairs.contains(&reverse_pair) {
                    return Ok(false); // Asymmetry violated
                }
                pairs.insert(pair);
            }
        }
        
        // If no symmetric pairs found, asymmetry is consistent
        Ok(true)
    }
    
    fn check_transitive_object_property_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::TransitiveObjectPropertyAxiom) -> Result<bool> {
        // Check if transitivity is already established
        for existing_axiom in self.get_transitive_object_property_axioms(ontology) {
            if existing_axiom.property == axiom.property {
                return Ok(true);
            }
        }
        
        // Check if transitivity is implied by property assertions
        // A property is transitive if whenever (x,y) and (y,z) are asserted, (x,z) is also asserted
        let mut pairs = std::collections::HashSet::new();
        
        for assertion in self.get_object_property_assertions(ontology) {
            if assertion.property == axiom.property {
                pairs.insert((assertion.source.clone(), assertion.target.clone()));
            }
        }
        
        // Check transitivity closure
        for (x, y) in &pairs {
            for (y2, z) in &pairs {
                if y == y2 {
                    // If we have (x,y) and (y,z), we should have (x,z)
                    if x != z && !pairs.contains(&(x.clone(), z.clone())) {
                        return Ok(false); // Transitivity incomplete
                    }
                }
            }
        }
        
        Ok(!pairs.is_empty())
    }
    
    fn check_same_individual_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::SameIndividualAxiom) -> Result<bool> {
        // Check if individual sameness is already established
        for existing_axiom in self.get_same_individual_axioms(ontology) {
            if self.individual_sets_overlap(&axiom.individuals, &existing_axiom.individuals) {
                return Ok(true);
            }
        }
        
        // In OWL, sameness is symmetric and transitive
        // Check if any pair in the axiom is already known to be the same
        for i in 0..axiom.individuals.len() {
            for j in (i+1)..axiom.individuals.len() {
                if self.are_individuals_same(ontology, &axiom.individuals[i], &axiom.individuals[j])? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    fn check_different_individuals_entailment(&self, ontology: &crate::ontology::Ontology, axiom: &crate::ontology::axioms::DifferentIndividualsAxiom) -> Result<bool> {
        // Check if individual difference is already established
        for existing_axiom in self.get_different_individuals_axioms(ontology) {
            if self.individual_sets_overlap(&axiom.individuals, &existing_axiom.individuals) {
                return Ok(true);
            }
        }
        
        // Check if any individuals in the axiom are asserted to be the same (contradiction)
        for i in 0..axiom.individuals.len() {
            for j in (i+1)..axiom.individuals.len() {
                if self.are_individuals_same(ontology, &axiom.individuals[i], &axiom.individuals[j])? {
                    return Ok(false); // Contradiction found
                }
            }
        }
        
        Ok(false)
    }
    
    fn are_class_expressions_equivalent(&self, expr1: &crate::ontology::ClassExpression, expr2: &crate::ontology::ClassExpression, ontology: &crate::ontology::Ontology) -> Result<bool> {
        // Simple structural equivalence check first
        if expr1 == expr2 {
            return Ok(true);
        }
        
        // Check if equivalence is established via axioms
        match (expr1, expr2) {
            (crate::ontology::ClassExpression::Class(c1), crate::ontology::ClassExpression::Class(c2)) => {
                // Check equivalent classes axioms
                for axiom in self.get_equivalent_classes_axioms(ontology) {
                    if axiom.classes.contains(&crate::ontology::ClassExpression::Class(c1.clone())) && 
                       axiom.classes.contains(&crate::ontology::ClassExpression::Class(c2.clone())) {
                        return Ok(true);
                    }
                }
            }
            _ => {
                // For complex expressions, would need full reasoning
                // This is a simplified check
            }
        }
        
        Ok(false)
    }
    
    fn are_object_property_expressions_equivalent(&self, expr1: &crate::ontology::ObjectPropertyExpression, expr2: &crate::ontology::ObjectPropertyExpression, ontology: &crate::ontology::Ontology) -> Result<bool> {
        // Simple structural equivalence check first
        if expr1 == expr2 {
            return Ok(true);
        }
        
        // Check if equivalence is established via axioms
        match (expr1, expr2) {
            (crate::ontology::ObjectPropertyExpression::ObjectProperty(p1), 
             crate::ontology::ObjectPropertyExpression::ObjectProperty(p2)) => {
                // Check equivalent object properties axioms
                for axiom in self.get_equivalent_object_properties_axioms(ontology) {
                    if axiom.properties.contains(&crate::ontology::ObjectPropertyExpression::ObjectProperty(p1.clone())) && 
                       axiom.properties.contains(&crate::ontology::ObjectPropertyExpression::ObjectProperty(p2.clone())) {
                        return Ok(true);
                    }
                }
            }
            _ => {
                // For inverse properties and complex expressions, would need more logic
            }
        }
        
        Ok(false)
    }
    
    fn is_sub_object_property_in_ontology(&self, sub: &crate::ontology::ObjectPropertyExpression, super_: &crate::ontology::ObjectPropertyExpression, ontology: &crate::ontology::Ontology) -> Result<bool> {
        // Direct check in sub-property axioms
        for axiom in self.get_sub_object_property_axioms(ontology) {
            if axiom.sub_property == *sub && axiom.super_property == *super_ {
                return Ok(true);
            }
        }
        
        // Check transitivity
        self.is_property_subproperty_by_transitivity(ontology, sub, super_)
    }
    
    fn is_sub_data_property_in_ontology(&self, sub: &crate::ontology::DataPropertyExpression, super_: &crate::ontology::DataPropertyExpression, ontology: &crate::ontology::Ontology) -> Result<bool> {
        // Direct check in sub-property axioms
        for axiom in self.get_sub_data_property_axioms(ontology) {
            if axiom.sub_property == *sub && axiom.super_property == *super_ {
                return Ok(true);
            }
        }
        
        // Check transitivity
        self.is_data_property_subproperty_by_transitivity(ontology, sub, super_)
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

    /// Helper RDF term conversion methods
    fn rdf_term_to_individual(&self, term: &RdfTerm) -> Result<crate::ontology::Individual> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::Individual::Named(
                crate::ontology::individuals::NamedIndividual {
                    iri: crate::ontology::IRI::from(iri.clone()),
                }
            )),
            RdfTerm::BlankNode(id) => Ok(crate::ontology::Individual::Anonymous(
                crate::ontology::individuals::AnonymousIndividual {
                    id: id.clone(),
                }
            )),
            _ => Err(Error::reasoning("Invalid individual term")),
        }
    }
    
    fn rdf_term_to_class_expression(&self, term: &RdfTerm) -> Result<crate::ontology::ClassExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::ClassExpression::Class(
                crate::ontology::concepts::Class::new(crate::ontology::IRI::from(iri.clone()))
            )),
            _ => Err(Error::reasoning("Complex class expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_object_property_expression(&self, term: &RdfTerm) -> Result<crate::ontology::ObjectPropertyExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::ObjectPropertyExpression::ObjectProperty(
                crate::ontology::ObjectProperty::new(crate::ontology::IRI::new(&iri.to_string()))?
            )),
            _ => Err(Error::reasoning("Complex object property expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_data_property_expression(&self, term: &RdfTerm) -> Result<crate::ontology::DataPropertyExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::DataPropertyExpression::DataProperty(
                crate::ontology::DataProperty { iri: crate::ontology::IRI::from(iri.clone()) }
            )),
            _ => Err(Error::reasoning("Complex data property expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_literal(&self, term: &RdfTerm) -> Result<crate::ontology::Literal> {
        match term {
            RdfTerm::Literal { value, datatype, language } => {
                let datatype_iri = datatype.as_ref()
                    .map(|dt| crate::ontology::IRI::from(dt.clone()))
                    .unwrap_or_else(|| crate::ontology::IRI::from(url::Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap()));
                
                Ok(crate::ontology::Literal {
                    value: value.clone(),
                    datatype: datatype.clone(),
                    language: language.clone(),
                })
            }
            _ => Err(Error::reasoning("Invalid literal term")),
        }
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
    
    /// Parse an RDF list into a vector of terms (comprehensive implementation)
    fn parse_rdf_list(&self, list_term: &RdfTerm, graph: &RdfGraph) -> Option<Vec<RdfTerm>> {
        // Check if this is rdf:nil (empty list)
        if self.is_iri(list_term, "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil") {
            return Some(Vec::new());
        }
        
        let mut elements = Vec::new();
        let mut current_node = list_term.clone();
        let mut visited = std::collections::HashSet::new();
        
        loop {
            // Prevent infinite loops
            if visited.contains(&current_node) {
                return None; // Circular list
            }
            visited.insert(current_node.clone());
            
            // Check if we've reached rdf:nil
            if self.is_iri(&current_node, "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil") {
                break;
            }
            
            // Find rdf:first for this node
            let mut first_found = false;
            for triple in &graph.triples {
                if triple.subject == current_node &&
                   self.is_iri(&triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#first") {
                    elements.push(triple.object.clone());
                    first_found = true;
                    break;
                }
            }
            
            if !first_found {
                return None; // Malformed list - no rdf:first
            }
            
            // Find rdf:rest for this node
            let mut rest_found = false;
            for triple in &graph.triples {
                if triple.subject == current_node &&
                   self.is_iri(&triple.predicate, "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest") {
                    current_node = triple.object.clone();
                    rest_found = true;
                    break;
                }
            }
            
            if !rest_found {
                return None; // Malformed list - no rdf:rest
            }
        }
        
        Some(elements)
    }
    
    /// Parse RDF list starting from a specific predicate
    fn parse_rdf_list_from_predicate(&self, subject: &RdfTerm, predicate_iri: &str, graph: &RdfGraph) -> Option<Vec<RdfTerm>> {
        for triple in &graph.triples {
            if triple.subject == *subject && self.is_iri(&triple.predicate, predicate_iri) {
                return self.parse_rdf_list(&triple.object, graph);
            }
        }
        None
    }
    
    /// Create helper methods for various axiom creation from triples
    pub fn create_object_property_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject = self.rdf_term_to_individual(&triple.subject)?;
        let object = self.rdf_term_to_individual(&triple.object)?;
        let property = self.rdf_term_to_object_property_expression(&triple.predicate)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyAssertion(
            crate::ontology::axioms::ObjectPropertyAssertionAxiom {
                id: 0, // TODO: proper ID generation
                source: subject,
                property,
                target: object,
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_data_property_assertion_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let subject = self.rdf_term_to_individual(&triple.subject)?;
        let target = self.rdf_term_to_literal(&triple.object)?;
        let property = self.rdf_term_to_data_property_expression(&triple.predicate)?;
        
        Ok(crate::ontology::Axiom::DataPropertyAssertion(
            crate::ontology::axioms::DataPropertyAssertionAxiom {
                id: 0, // TODO: proper ID generation
                individual: subject,
                property,
                value: target,
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_property_domain_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let property = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let domain = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyDomain(
            crate::ontology::axioms::ObjectPropertyDomainAxiom {
                id: 0, // TODO: proper ID generation
                property,
                domain,
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_property_range_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let property = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let range = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::ObjectPropertyRange(
            crate::ontology::axioms::ObjectPropertyRangeAxiom {
                id: 0, // TODO: proper ID generation
                property,
                range,
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_equivalent_classes_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let class1 = self.rdf_term_to_class_expression(&triple.subject)?;
        let class2 = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::EquivalentClasses(
            crate::ontology::axioms::EquivalentClassesAxiom {
                id: 0, // TODO: proper ID generation
                classes: vec![class1, class2],
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_disjoint_classes_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let class1 = self.rdf_term_to_class_expression(&triple.subject)?;
        let class2 = self.rdf_term_to_class_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::DisjointClasses(
            crate::ontology::axioms::DisjointClassesAxiom {
                id: 0, // TODO: proper ID generation
                classes: vec![class1, class2],
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_equivalent_properties_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let prop1 = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let prop2 = self.rdf_term_to_object_property_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::EquivalentObjectProperties(
            crate::ontology::axioms::EquivalentObjectPropertiesAxiom {
                id: 0, // TODO: proper ID generation
                properties: vec![prop1, prop2],
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_inverse_object_properties_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let prop1 = self.rdf_term_to_object_property_expression(&triple.subject)?;
        let prop2 = self.rdf_term_to_object_property_expression(&triple.object)?;
        
        Ok(crate::ontology::Axiom::InverseObjectProperties(
            crate::ontology::axioms::InverseObjectPropertiesAxiom {
                id: 0, // TODO: proper ID generation
                property1: prop1,
                property2: prop2,
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_same_individual_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let ind1 = self.rdf_term_to_individual(&triple.subject)?;
        let ind2 = self.rdf_term_to_individual(&triple.object)?;
        
        Ok(crate::ontology::Axiom::SameIndividual(
            crate::ontology::axioms::SameIndividualAxiom {
                id: 0, // TODO: proper ID generation
                individuals: vec![ind1, ind2],
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_different_individuals_axiom_from_triple(&self, triple: &Triple) -> Result<crate::ontology::Axiom> {
        let ind1 = self.rdf_term_to_individual(&triple.subject)?;
        let ind2 = self.rdf_term_to_individual(&triple.object)?;
        
        Ok(crate::ontology::Axiom::DifferentIndividuals(
            crate::ontology::axioms::DifferentIndividualsAxiom {
                id: 0, // TODO: proper ID generation
                individuals: vec![ind1, ind2],
                annotations: vec![],
            }
        ))
    }
    
    pub fn create_cardinality_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        // Placeholder for cardinality restriction parsing
        Err(Error::reasoning("Cardinality axiom parsing not implemented"))
    }
    
    pub fn create_intersection_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        // Placeholder for intersection parsing
        Err(Error::reasoning("Intersection axiom parsing not implemented"))
    }
    
    pub fn create_union_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        // Placeholder for union parsing
        Err(Error::reasoning("Union axiom parsing not implemented"))
    }
    
    pub fn create_complement_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        // Placeholder for complement parsing
        Err(Error::reasoning("Complement axiom parsing not implemented"))
    }
    
    pub fn create_property_chain_axiom_from_triple(&self, _triple: &Triple) -> Result<crate::ontology::Axiom> {
        // Placeholder for property chain parsing
        Err(Error::reasoning("Property chain axiom parsing not implemented"))
    }
    
    /// Helper RDF term conversion methods for Owl2RlEngine
    fn rdf_term_to_individual(&self, term: &RdfTerm) -> Result<crate::ontology::Individual> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::Individual::Named(
                crate::ontology::individuals::NamedIndividual {
                    iri: crate::ontology::IRI::from(iri.clone()),
                }
            )),
            RdfTerm::BlankNode(id) => Ok(crate::ontology::Individual::Anonymous(
                crate::ontology::individuals::AnonymousIndividual {
                    id: id.clone(),
                }
            )),
            _ => Err(Error::reasoning("Invalid individual term")),
        }
    }
    
    fn rdf_term_to_class_expression(&self, term: &RdfTerm) -> Result<crate::ontology::ClassExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::ClassExpression::Class(
                crate::ontology::concepts::Class::new(crate::ontology::IRI::from(iri.clone()))
            )),
            _ => Err(Error::reasoning("Complex class expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_object_property_expression(&self, term: &RdfTerm) -> Result<crate::ontology::ObjectPropertyExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::ObjectPropertyExpression::ObjectProperty(
                crate::ontology::ObjectProperty::new(crate::ontology::IRI::new(&iri.to_string()))?
            )),
            _ => Err(Error::reasoning("Complex object property expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_data_property_expression(&self, term: &RdfTerm) -> Result<crate::ontology::DataPropertyExpression> {
        match term {
            RdfTerm::Iri(iri) => Ok(crate::ontology::DataPropertyExpression::DataProperty(
                crate::ontology::DataProperty { iri: crate::ontology::IRI::from(iri.clone()) }
            )),
            _ => Err(Error::reasoning("Complex data property expressions not supported in simple conversion")),
        }
    }
    
    fn rdf_term_to_literal(&self, term: &RdfTerm) -> Result<crate::ontology::Literal> {
        match term {
            RdfTerm::Literal { value, datatype, language } => {
                let datatype_iri = datatype.as_ref()
                    .map(|dt| crate::ontology::IRI::from(dt.clone()))
                    .unwrap_or_else(|| crate::ontology::IRI::from(url::Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap()));
                
                Ok(crate::ontology::Literal {
                    value: value.clone(),
                    datatype: datatype.clone(),
                    language: language.clone(),
                })
            }
            _ => Err(Error::reasoning("Invalid literal term")),
        }
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
    
    /// Helper method to check if two sets of axioms overlap
    fn axiom_sets_overlap(&self, set1: &[crate::ontology::ClassExpression], set2: &[crate::ontology::ClassExpression]) -> bool {
        set1.iter().any(|item| set2.contains(item))
    }
    
    /// Helper method to check if two property sets overlap
    fn property_sets_overlap(&self, set1: &[crate::ontology::ObjectPropertyExpression], set2: &[crate::ontology::ObjectPropertyExpression]) -> bool {
        set1.iter().any(|item| set2.contains(item))
    }
    
    /// Helper method to check if two individual sets overlap
    fn individual_sets_overlap(&self, set1: &[crate::ontology::Individual], set2: &[crate::ontology::Individual]) -> bool {
        set1.iter().any(|item| set2.contains(item))
    }
    
    /// Helper method to check if an individual is in a class
    fn is_individual_in_class(&self, ontology: &crate::ontology::Ontology, individual: &crate::ontology::Individual, class: &crate::ontology::ClassExpression) -> Result<bool> {
        // Check class assertions
        for assertion in self.get_class_assertions(ontology) {
            if assertion.individual == *individual {
                if assertion.class == *class {
                    return Ok(true);
                }
                
                // Check if the asserted class is a subclass of the target class
                if self.is_class_subclass_of(ontology, &assertion.class, class)? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Helper method to check if one class is a subclass of another
    fn is_class_subclass_of(&self, ontology: &crate::ontology::Ontology, sub: &crate::ontology::ClassExpression, super_: &crate::ontology::ClassExpression) -> Result<bool> {
        // Direct subclass check
        for axiom in self.get_subclass_of_axioms(ontology) {
            if axiom.subclass == *sub && axiom.superclass == *super_ {
                return Ok(true);
            }
        }
        
        // Check transitivity
        for axiom in self.get_subclass_of_axioms(ontology) {
            if axiom.subclass == *sub {
                if self.is_class_subclass_of(ontology, &axiom.superclass, super_)? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Helper method to check property sub-property relationship by transitivity
    fn is_property_subproperty_by_transitivity(&self, ontology: &crate::ontology::Ontology, sub: &crate::ontology::ObjectPropertyExpression, super_: &crate::ontology::ObjectPropertyExpression) -> Result<bool> {
        // Check transitivity through intermediate properties
        for axiom in self.get_sub_object_property_axioms(ontology) {
            if axiom.sub_property == *sub {
                // Check if this intermediate property is a sub-property of the target
                if self.is_sub_object_property_in_ontology(&axiom.super_property, super_, ontology)? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Helper method to check data property sub-property relationship by transitivity
    fn is_data_property_subproperty_by_transitivity(&self, ontology: &crate::ontology::Ontology, sub: &crate::ontology::DataPropertyExpression, super_: &crate::ontology::DataPropertyExpression) -> Result<bool> {
        // Check transitivity through intermediate properties
        for axiom in self.get_sub_data_property_axioms(ontology) {
            if axiom.sub_property == *sub {
                // Check if this intermediate property is a sub-property of the target
                if self.is_sub_data_property_in_ontology(&axiom.super_property, super_, ontology)? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Helper method to check if two individuals are the same
    fn are_individuals_same(&self, ontology: &crate::ontology::Ontology, ind1: &crate::ontology::Individual, ind2: &crate::ontology::Individual) -> Result<bool> {
        if ind1 == ind2 {
            return Ok(true);
        }
        
        // Check same individual axioms
        for axiom in self.get_same_individual_axioms(ontology) {
            if axiom.individuals.contains(ind1) && axiom.individuals.contains(ind2) {
                return Ok(true);
            }
        }
        
        // Check transitivity - if ind1 == x and x == ind2, then ind1 == ind2
        for axiom in self.get_same_individual_axioms(ontology) {
            if axiom.individuals.contains(ind1) {
                for other_ind in &axiom.individuals {
                    if other_ind != ind1 && self.are_individuals_same(ontology, other_ind, ind2)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    // Methods that should be delegated to the base EntailmentChecker
    fn get_class_assertions<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::ClassAssertionAxiom> + 'a {
        // For now, return an empty iterator - this would need proper implementation
        std::iter::empty()
    }

    fn get_subclass_of_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubClassOfAxiom> + 'a {
        // For now, return an empty iterator - this would need proper implementation
        std::iter::empty()
    }

    fn get_sub_object_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubObjectPropertyOfAxiom> + 'a {
        // For now, return an empty iterator - this would need proper implementation
        std::iter::empty()
    }

    fn get_sub_data_property_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SubDataPropertyOfAxiom> + 'a {
        // For now, return an empty iterator - this would need proper implementation
        std::iter::empty()
    }

    fn is_sub_object_property_in_ontology(&self, _super_property: &crate::ontology::ObjectPropertyExpression, _sub_property: &crate::ontology::ObjectPropertyExpression, _ontology: &Ontology) -> Result<bool> {
        // For now, return false - this would need proper implementation
        Ok(false)
    }

    fn get_same_individual_axioms<'a>(&self, ontology: &'a Ontology) -> impl Iterator<Item = &'a crate::ontology::axioms::SameIndividualAxiom> + 'a {
        // For now, return an empty iterator - this would need proper implementation
        std::iter::empty()
    }

    fn is_sub_data_property_in_ontology(&self, _super_property: &crate::ontology::DataPropertyExpression, _sub_property: &crate::ontology::DataPropertyExpression, _ontology: &Ontology) -> Result<bool> {
        // For now, return false - this would need proper implementation
        Ok(false)
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
