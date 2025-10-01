//! OWL 2 RL rule engine implementation
//!
//! This module implements the OWL 2 RL profile rules as defined in:
//! https://www.w3.org/TR/owl2-profiles/\#OWL_2_RL

use crate::semantics::{RdfGraph, RdfTerm, Triple};
use crate::{Error, Result};

/// OWL 2 RL Rule Engine
///
/// Implements the OWL 2 RL profile rules for rule-based reasoning on RDF graphs.
#[derive(Debug)]
pub struct Owl2RlEngine {
    graph: RdfGraph,
    closure: RdfGraph,
    rule_applications: usize,
}

impl Owl2RlEngine {
    /// Create a new OWL 2 RL engine with the given RDF graph
    pub fn new(graph: RdfGraph) -> Self {
        let closure = graph.clone();
        Self {
            graph,
            closure,
            rule_applications: 0,
        }
    }

    /// Run the rule engine to completion
    pub fn reason(&mut self) -> Result<()> {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops

        while changed && iterations < MAX_ITERATIONS {
            let initial_size = self.closure.triples.len();

            // Apply OWL 2 RL rules
            self.apply_subclass_rules()?;
            self.apply_property_rules()?;
            self.apply_domain_range_rules()?;

            changed = self.closure.triples.len() > initial_size;
            iterations += 1;
        }

        if iterations >= MAX_ITERATIONS {
            return Err(Error::reasoning(
                "Rule application reached maximum iterations",
            ));
        }

        Ok(())
    }

    /// Get the closure (all derived triples)
    pub fn closure(&self) -> &RdfGraph {
        &self.closure
    }

    /// Get the number of rule applications
    pub fn rule_applications(&self) -> usize {
        self.rule_applications
    }

    /// Apply subclass transitivity rules
    fn apply_subclass_rules(&mut self) -> Result<()> {
        let subclass_pred = RdfTerm::iri("http://www.w3.org/2000/01/rdf-schema#subClassOf")?;

        let triples: Vec<_> = self.closure.triples.iter().cloned().collect();
        let mut new_triples = Vec::new();

        // Transitivity: if A subClassOf B and B subClassOf C, then A subClassOf C
        for triple1 in &triples {
            if triple1.predicate == subclass_pred {
                for triple2 in &triples {
                    if triple2.predicate == subclass_pred && triple1.object == triple2.subject {
                        let transitive_triple = Triple {
                            subject: triple1.subject.clone(),
                            predicate: subclass_pred.clone(),
                            object: triple2.object.clone(),
                        };
                        if !self.closure.contains_triple(&transitive_triple) {
                            new_triples.push(transitive_triple);
                        }
                    }
                }
            }
        }

        // Add new triples
        for triple in new_triples {
            self.closure.triples.insert(triple);
            self.rule_applications += 1;
        }

        Ok(())
    }

    /// Apply property hierarchy rules
    fn apply_property_rules(&mut self) -> Result<()> {
        let subprop_pred = RdfTerm::iri("http://www.w3.org/2000/01/rdf-schema#subPropertyOf")?;

        let triples: Vec<_> = self.closure.triples.iter().cloned().collect();
        let mut new_triples = Vec::new();

        // Sub-property implication: if P subPropertyOf Q and (x,y) in P, then (x,y) in Q
        for subprop_triple in &triples {
            if subprop_triple.predicate == subprop_pred {
                for prop_triple in &triples {
                    if prop_triple.predicate == subprop_triple.subject {
                        let implied_triple = Triple {
                            subject: prop_triple.subject.clone(),
                            predicate: subprop_triple.object.clone(),
                            object: prop_triple.object.clone(),
                        };
                        if !self.closure.contains_triple(&implied_triple) {
                            new_triples.push(implied_triple);
                        }
                    }
                }
            }
        }

        // Add new triples
        for triple in new_triples {
            self.closure.triples.insert(triple);
            self.rule_applications += 1;
        }

        Ok(())
    }

    /// Apply domain and range rules
    fn apply_domain_range_rules(&mut self) -> Result<()> {
        let domain_pred = RdfTerm::iri("http://www.w3.org/2000/01/rdf-schema#domain")?;
        let range_pred = RdfTerm::iri("http://www.w3.org/2000/01/rdf-schema#range")?;
        let type_pred = RdfTerm::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;

        let triples: Vec<_> = self.closure.triples.iter().cloned().collect();
        let mut new_triples = Vec::new();

        // Domain rule: if P has domain C and (x,y) in P, then x : C
        for domain_triple in &triples {
            if domain_triple.predicate == domain_pred {
                for prop_triple in &triples {
                    if prop_triple.predicate == domain_triple.subject {
                        let type_triple = Triple {
                            subject: prop_triple.subject.clone(),
                            predicate: type_pred.clone(),
                            object: domain_triple.object.clone(),
                        };
                        if !self.closure.contains_triple(&type_triple) {
                            new_triples.push(type_triple);
                        }
                    }
                }
            }
        }

        // Range rule: if P has range C and (x,y) in P, then y : C
        for range_triple in &triples {
            if range_triple.predicate == range_pred {
                for prop_triple in &triples {
                    if prop_triple.predicate == range_triple.subject {
                        let type_triple = Triple {
                            subject: prop_triple.object.clone(),
                            predicate: type_pred.clone(),
                            object: range_triple.object.clone(),
                        };
                        if !self.closure.contains_triple(&type_triple) {
                            new_triples.push(type_triple);
                        }
                    }
                }
            }
        }

        // Add new triples
        for triple in new_triples {
            self.closure.triples.insert(triple);
            self.rule_applications += 1;
        }

        Ok(())
    }
}
