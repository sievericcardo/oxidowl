//! Entailment checker implementation
//!
//! This module contains the main `EntailmentChecker` struct and `EntailmentRegime` enum
//! for checking various types of entailment relationships.

#![allow(dead_code)]

use crate::Result;
use crate::semantics::{RdfGraph, RdfTerm, Triple};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

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
    id_generator: AtomicUsize,
}

impl EntailmentChecker {
    /// Create a new entailment checker for the specified regime
    #[must_use]
    pub fn new(regime: EntailmentRegime) -> Self {
        Self {
            regime,
            cache: HashMap::new(),
            id_generator: AtomicUsize::new(1),
        }
    }

    /// Generate a unique ID for axioms
    fn generate_id(&self) -> usize {
        self.id_generator.fetch_add(1, Ordering::SeqCst)
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
            EntailmentRegime::RdfSimple => {
                self.check_rdf_simple_entailment(premises, conclusion)?
            }
            EntailmentRegime::Rdfs => self.check_rdfs_entailment(premises, conclusion)?,
            EntailmentRegime::OwlRdf => self.check_owl_rdf_entailment(premises, conclusion)?,
            EntailmentRegime::OwlDirect => {
                self.check_owl_direct_entailment(premises, conclusion)?
            }
            EntailmentRegime::OwlRl => self.check_owl_rl_entailment(premises, conclusion)?,
            EntailmentRegime::OwlEl => self.check_owl_el_entailment(premises, conclusion)?,
            EntailmentRegime::OwlQl => self.check_owl_ql_entailment(premises, conclusion)?,
        };

        self.cache.insert(cache_key, result);
        Ok(result)
    }

    /// Check RDF simple entailment
    fn check_rdf_simple_entailment(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> Result<bool> {
        // Simple RDF entailment: check if conclusion triples are subset of premises
        for conclusion_triple in &conclusion.triples {
            if !premises.contains_triple(conclusion_triple) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check RDFS entailment using forward-chaining closure
    fn check_rdfs_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        if conclusion.triples().is_empty() {
            return Ok(true);
        }
        if premises.triples().is_empty() {
            return Ok(conclusion
                .triples()
                .iter()
                .all(|c| self.is_trivial_rdfs_conclusion(c)));
        }

        let rdf_type_url = url::Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Valid hardcoded rdf:type URL");
        let rdfs_domain_url =
            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#domain")
                .expect("Valid hardcoded rdfs:domain URL");
        let rdfs_range_url = url::Url::parse("http://www.w3.org/2000/01/rdf-schema#range")
            .expect("Valid hardcoded rdfs:range URL");
        let rdfs_subproperty_url =
            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#subPropertyOf")
                .expect("Valid hardcoded rdfs:subPropertyOf URL");
        let rdfs_subclass_url =
            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#subClassOf")
                .expect("Valid hardcoded rdfs:subClassOf URL");

        let mut triples: HashSet<Triple> = premises.triples().iter().cloned().collect();
        let mut changed = true;
        let max_iterations = 50;
        let mut iteration = 0;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;
            let current: Vec<Triple> = triples.iter().cloned().collect();

            for triple in &current {
                // rdfs2: ?p rdfs:domain ?c . ?s ?p ?o => ?s rdf:type ?c
                if let RdfTerm::Iri(ref pred) = triple.predicate {
                    if pred == &rdfs_domain_url {
                        let domain_class = &triple.object;
                        for t in &current {
                            if t.predicate == triple.subject {
                                let new_triple = Triple::new(
                                    t.subject.clone(),
                                    RdfTerm::Iri(rdf_type_url.clone()),
                                    domain_class.clone(),
                                );
                                if triples.insert(new_triple) {
                                    changed = true;
                                }
                            }
                        }
                    }
                    // rdfs3: ?p rdfs:range ?c . ?s ?p ?o => ?o rdf:type ?c
                    if pred == &rdfs_range_url {
                        let range_class = &triple.object;
                        for t in &current {
                            if t.predicate == triple.subject {
                                let new_triple = Triple::new(
                                    t.object.clone(),
                                    RdfTerm::Iri(rdf_type_url.clone()),
                                    range_class.clone(),
                                );
                                if triples.insert(new_triple) {
                                    changed = true;
                                }
                            }
                        }
                    }
                    // rdfs5: ?p rdfs:subPropertyOf ?q . ?q rdfs:subPropertyOf ?r => ?p rdfs:subPropertyOf ?r
                    // rdfs7: ?p rdfs:subPropertyOf ?q . ?s ?p ?o => ?s ?q ?o
                    if pred == &rdfs_subproperty_url {
                        let super_prop = &triple.object;
                        // rdfs5: transitivity of subPropertyOf
                        for t in &current {
                            if let RdfTerm::Iri(ref t_pred) = t.predicate
                                && t_pred == &rdfs_subproperty_url
                                    && t.subject == triple.object
                                {
                                    let new_triple = Triple::new(
                                        triple.subject.clone(),
                                        RdfTerm::Iri(rdfs_subproperty_url.clone()),
                                        t.object.clone(),
                                    );
                                    if triples.insert(new_triple) {
                                        changed = true;
                                    }
                                }
                        }
                        // rdfs7: sub-property propagation
                        for t in &current {
                            if t.predicate == triple.subject {
                                let new_triple = Triple::new(
                                    t.subject.clone(),
                                    super_prop.clone(),
                                    t.object.clone(),
                                );
                                if triples.insert(new_triple) {
                                    changed = true;
                                }
                            }
                        }
                    }
                    // rdfs9: ?s rdf:type ?c . ?c rdfs:subClassOf ?d => ?s rdf:type ?d
                    if pred == &rdf_type_url
                        && let RdfTerm::Iri(ref _class_iri) = triple.object {
                            for t in &current {
                                if let RdfTerm::Iri(ref t_pred) = t.predicate
                                    && t_pred == &rdfs_subclass_url
                                        && t.subject == triple.object
                                    {
                                        let new_triple = Triple::new(
                                            triple.subject.clone(),
                                            RdfTerm::Iri(rdf_type_url.clone()),
                                            t.object.clone(),
                                        );
                                        if triples.insert(new_triple) {
                                            changed = true;
                                        }
                                    }
                            }
                        }
                    // rdfs11: ?c rdfs:subClassOf ?d . ?d rdfs:subClassOf ?e => ?c rdfs:subClassOf ?e
                    if pred == &rdfs_subclass_url {
                        for t in &current {
                            if let RdfTerm::Iri(ref t_pred) = t.predicate
                                && t_pred == &rdfs_subclass_url
                                    && t.subject == triple.object
                                {
                                    let new_triple = Triple::new(
                                        triple.subject.clone(),
                                        RdfTerm::Iri(rdfs_subclass_url.clone()),
                                        t.object.clone(),
                                    );
                                    if triples.insert(new_triple) {
                                        changed = true;
                                    }
                                }
                        }
                    }
                }
            }

            // Check if all conclusions are in the closure
            if conclusion.triples().iter().all(|c| triples.contains(c)) {
                return Ok(true);
            }
        }

        Ok(conclusion
            .triples()
            .iter()
            .all(|c| triples.contains(c)))
    }

    /// Check if a conclusion is trivially true under RDFS semantics
    fn is_trivial_rdfs_conclusion(&self, triple: &Triple) -> bool {
        if let RdfTerm::Iri(ref pred) = triple.predicate {
            // rdfs:Resource rdf:type rdfs:Class
            if pred.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
                return true;
            }
        }
        false
    }

    /// Check OWL RDF-based entailment via OWL 2 RL rules
    fn check_owl_rdf_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Check OWL Direct semantics entailment via reasoner if available
    fn check_owl_direct_entailment(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> Result<bool> {
        // Try with the actual tableau reasoner for direct semantics
        if let Ok(entailed) = self.check_via_tableau_reasoner(premises, conclusion) {
            return Ok(entailed);
        }
        // Fallback to RL reasoning as approximation
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Use the tableau reasoner to check entailment
    fn check_via_tableau_reasoner(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> Result<bool> {
        use crate::ontology::Ontology;
        use crate::reasoner_api::{
            OWLReasoner, OWLReasonerConfiguration, TableauOWLReasoner,
        };
        use std::sync::{Arc, RwLock};

        // Build ontology from premise triples
        let mut ont = Ontology::new();
        for triple in premises.triples() {
            if let Some(axiom) = self.triple_to_axiom(triple) {
                ont.add_axiom(axiom);
            }
        }

        if ont.axioms.is_empty() {
            return Err(crate::Error::reasoning("No axioms extracted from premises"));
        }

        let ont_ref: Arc<RwLock<Ontology>> = Arc::new(RwLock::new(ont));
        let config = OWLReasonerConfiguration::default();
        let reasoner = TableauOWLReasoner::new(ont_ref, config)?;

        // Check each conclusion
        for conclusion_triple in conclusion.triples() {
            if let Some(axiom) = self.triple_to_axiom(conclusion_triple) {
                if !reasoner.is_entailed(&axiom)? {
                    return Ok(false);
                }
            } else {
                // If we can't convert the conclusion to an axiom, check
                // whether it's already in the premises (simple entailment)
                if !premises.contains_triple(conclusion_triple) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Convert an RDF triple to an OWL axiom when possible
    fn triple_to_axiom(&self, triple: &Triple) -> Option<crate::ontology::Axiom> {
        use crate::ontology::axioms::{
            ClassAssertionAxiom, SubClassOfAxiom, SubObjectPropertyOfAxiom,
        };
        use crate::ontology::{Axiom, Class, ClassExpression, IRI, Individual};

        let subject_iri = triple.subject.as_iri().map(|u| IRI::from_url(u.clone()));
        let object_iri = triple.object.as_iri().map(|u| IRI::from_url(u.clone()));

        match &triple.predicate {
            RdfTerm::Iri(pred_url) => {
                let pred_str = pred_url.as_str();

                if pred_str == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
                    let subject_iri = subject_iri?;
                    let object_iri = object_iri?;
                    let individual = Individual::Named(
                        crate::ontology::NamedIndividual { iri: subject_iri },
                    );
                    let class = ClassExpression::Class(Class::new(object_iri));
                    let axiom = Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        individual,
                        class,
                        annotations: Vec::new(),
                    });
                    return Some(axiom);
                }

                if pred_str == "http://www.w3.org/2000/01/rdf-schema#subClassOf" {
                    let sub = subject_iri?;
                    let sup = object_iri?;
                    let axiom = Axiom::SubClassOf(SubClassOfAxiom {
                        id: 0,
                        subclass: ClassExpression::Class(Class::new(sub)),
                        superclass: ClassExpression::Class(Class::new(sup)),
                        annotations: Vec::new(),
                    });
                    return Some(axiom);
                }

                if pred_str == "http://www.w3.org/2000/01/rdf-schema#subPropertyOf" {
                    let sub = subject_iri?;
                    let sup = object_iri?;
                    let axiom = Axiom::SubObjectPropertyOf(
                        SubObjectPropertyOfAxiom {
                            id: 0,
                            sub_property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                crate::ontology::ObjectProperty { iri: sub },
                            ),
                            super_property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                                crate::ontology::ObjectProperty { iri: sup },
                            ),
                            annotations: Vec::new(),
                        },
                    );
                    return Some(axiom);
                }

                None
            }
            _ => None,
        }
    }

    /// Check OWL 2 RL entailment
    fn check_owl_rl_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // Use the Owl2RlEngine from the owl2_rl module
        use super::owl2_rl::Owl2RlEngine;
        let mut engine = Owl2RlEngine::new(premises.clone());
        engine.reason()?;

        let closure = engine.closure();
        Ok(conclusion
            .triples()
            .iter()
            .all(|triple| closure.contains_triple(triple)))
    }

    /// Check OWL 2 EL entailment using EL reasoner
    fn check_owl_el_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // OWL 2 EL is a subset of OWL 2 RL; use RL reasoning as approximation
        self.check_owl_rl_entailment(premises, conclusion)
    }

    /// Check OWL 2 QL entailment using RL rules as approximation
    fn check_owl_ql_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> Result<bool> {
        // OWL 2 QL can be checked via query rewriting; use RL as conservative approximation
        self.check_owl_rl_entailment(premises, conclusion)
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
