//! RDF Simple Entailment Implementation
//!
//! This module implements RDF simple entailment as defined in:
//! https://www.w3.org/TR/rdf11-mt/#simple-entailment

use super::{RdfGraph, RdfTerm, SemanticInterpretation, Triple};
use crate::Result;
use std::collections::{HashMap, HashSet};

/// RDF Simple Interpretation
///
/// Implements the formal semantics for RDF simple entailment
/// according to the RDF 1.1 Model Theory specification.
#[derive(Debug, Clone)]
pub struct RdfSimpleInterpretation {
    /// Domain of interpretation - the set of all resources
    domain: HashSet<String>,
    /// Property interpretation mapping
    property_interpretation: HashMap<String, HashSet<(String, String)>>,
    /// Resource interpretation mapping
    resource_interpretation: HashMap<String, String>,
    /// Literal interpretation mapping  
    literal_interpretation: HashMap<String, String>,
}

impl RdfSimpleInterpretation {
    /// Create a new RDF simple interpretation
    pub fn new() -> Self {
        Self {
            domain: HashSet::new(),
            property_interpretation: HashMap::new(),
            resource_interpretation: HashMap::new(),
            literal_interpretation: HashMap::new(),
        }
    }

    /// Add a resource to the domain
    pub fn add_resource(&mut self, resource: String) {
        self.domain.insert(resource);
    }

    /// Set property interpretation
    pub fn set_property_interpretation(
        &mut self,
        property: String,
        pairs: HashSet<(String, String)>,
    ) {
        self.property_interpretation.insert(property, pairs);
    }

    /// Set resource interpretation
    pub fn set_resource_interpretation(&mut self, iri: String, resource: String) {
        self.resource_interpretation.insert(iri, resource);
    }

    /// Interpret an RDF term in this interpretation
    fn interpret_rdf_term(&self, term: &RdfTerm) -> Option<String> {
        match term {
            RdfTerm::Iri(iri) => {
                // IRIs are interpreted as resources in the domain
                self.resource_interpretation
                    .get(&iri.to_string())
                    .cloned()
                    .or_else(|| Some(iri.to_string())) // Default to self-interpretation
            }
            RdfTerm::BlankNode(id) => {
                // Blank nodes are interpreted as resources in the domain
                self.resource_interpretation.get(id).cloned()
            }
            RdfTerm::Literal {
                value,
                datatype,
                language,
            } => {
                // Literals are interpreted according to their datatype
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
                    .or_else(|| Some(literal_key)) // Default interpretation
            }
        }
    }

    /// Check if a triple is satisfied by this interpretation
    fn satisfies_triple(&self, triple: &Triple) -> bool {
        let subject_interp = self.interpret_rdf_term(&triple.subject);
        let predicate_interp = self.interpret_rdf_term(&triple.predicate);
        let object_interp = self.interpret_rdf_term(&triple.object);

        if let (Some(s), Some(p), Some(o)) = (subject_interp, predicate_interp, object_interp) {
            // Check if the property interpretation contains the (subject, object) pair
            if let Some(prop_pairs) = self.property_interpretation.get(&p) {
                prop_pairs.contains(&(s, o))
            } else {
                // If property is not explicitly interpreted, assume it's satisfied
                // This is a simplification - in practice we'd need more sophisticated handling
                true
            }
        } else {
            false
        }
    }
}

impl Default for RdfSimpleInterpretation {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticInterpretation for RdfSimpleInterpretation {
    fn satisfies(&self, graph: &RdfGraph) -> bool {
        // An interpretation satisfies a graph if it satisfies all triples in the graph
        graph
            .triples()
            .iter()
            .all(|triple| self.satisfies_triple(triple))
    }

    fn interpret_term(&self, term: &RdfTerm) -> Option<String> {
        self.interpret_rdf_term(term)
    }

    fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // Simple entailment: premises entail conclusion if every interpretation
        // that satisfies the premises also satisfies the conclusion

        // For now, we use a simplified check: conclusion is subset of premises
        // A proper implementation would need to handle blank node renaming
        conclusion
            .triples()
            .iter()
            .all(|triple| premises.contains_triple(triple))
    }
}

/// RDF Simple Entailment Engine
///
/// Implements RDF simple entailment reasoning according to the RDF 1.1 specification.
#[derive(Debug)]
pub struct RdfSimpleEntailment {
    /// Base graph
    base_graph: RdfGraph,
    /// Derived facts
    derived_graph: RdfGraph,
}

impl RdfSimpleEntailment {
    /// Create a new RDF simple entailment engine
    pub fn new(base_graph: RdfGraph) -> Self {
        Self {
            base_graph,
            derived_graph: RdfGraph::new(),
        }
    }

    /// Perform simple entailment reasoning
    pub fn reason(&mut self) -> Result<()> {
        // For RDF simple entailment, no additional inferences are made
        // All entailments are already explicit in the graph

        // Copy base graph to derived graph
        self.derived_graph = self.base_graph.clone();

        Ok(())
    }

    /// Get the closure (base + derived facts)
    pub fn closure(&self) -> RdfGraph {
        let mut closure = self.base_graph.clone();
        closure.merge(&self.derived_graph);
        closure
    }

    /// Check if premises entail conclusion
    pub fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // Check simple graph entailment
        self.check_simple_entailment(premises, conclusion)
    }

    /// Check simple entailment (subset relationship with blank node handling)
    fn check_simple_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // RDF Simple Entailment: conclusion is entailed by premises if there exists
        // a mapping from blank nodes in conclusion to terms in premises such that
        // when applied, all triples in conclusion appear in premises

        if conclusion.triples().is_empty() {
            return true; // Empty graph is entailed by any graph
        }

        // Collect blank nodes in conclusion
        let mut conclusion_blanks = HashSet::new();
        for triple in conclusion.triples() {
            if let RdfTerm::BlankNode(ref id) = triple.subject {
                conclusion_blanks.insert(id.clone());
            }
            if let RdfTerm::BlankNode(ref id) = triple.object {
                conclusion_blanks.insert(id.clone());
            }
        }

        // If no blank nodes, check direct containment
        if conclusion_blanks.is_empty() {
            return conclusion
                .triples()
                .iter()
                .all(|triple| premises.contains_triple(triple));
        }

        // Try to find a consistent mapping for blank nodes
        let blank_list: Vec<String> = conclusion_blanks.into_iter().collect();
        let mut mapping = HashMap::new();

        self.find_entailment_mapping(premises, conclusion, &mut mapping, &blank_list, 0)
    }

    /// Find a consistent mapping for blank nodes that makes conclusion entailed by premises
    fn find_entailment_mapping(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
        mapping: &mut HashMap<String, RdfTerm>,
        blanks: &[String],
        index: usize,
    ) -> bool {
        if index >= blanks.len() {
            // All blank nodes mapped, check if conclusion matches premises under this mapping
            return self.check_mapping_validity(premises, conclusion, mapping);
        }

        let blank = &blanks[index];

        // Try mapping this blank node to each term that appears in premises
        let mut candidate_terms = HashSet::new();

        for triple in premises.triples() {
            candidate_terms.insert(triple.subject.clone());
            candidate_terms.insert(triple.object.clone());
        }

        for candidate in candidate_terms {
            mapping.insert(blank.clone(), candidate);

            if self.find_entailment_mapping(premises, conclusion, mapping, blanks, index + 1) {
                return true;
            }

            mapping.remove(blank);
        }

        false
    }

    /// Check if conclusion matches premises under the given blank node mapping
    fn check_mapping_validity(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
        mapping: &HashMap<String, RdfTerm>,
    ) -> bool {
        for triple in conclusion.triples() {
            let mapped_triple = self.apply_mapping_to_triple(triple, mapping);
            if !premises.contains_triple(&mapped_triple) {
                return false;
            }
        }
        true
    }

    /// Apply blank node mapping to a triple
    fn apply_mapping_to_triple(
        &self,
        triple: &Triple,
        mapping: &HashMap<String, RdfTerm>,
    ) -> Triple {
        let subject = if let RdfTerm::BlankNode(ref id) = triple.subject {
            mapping
                .get(id)
                .cloned()
                .unwrap_or_else(|| triple.subject.clone())
        } else {
            triple.subject.clone()
        };

        let predicate = if let RdfTerm::BlankNode(ref id) = triple.predicate {
            mapping
                .get(id)
                .cloned()
                .unwrap_or_else(|| triple.predicate.clone())
        } else {
            triple.predicate.clone()
        };

        let object = if let RdfTerm::BlankNode(ref id) = triple.object {
            mapping
                .get(id)
                .cloned()
                .unwrap_or_else(|| triple.object.clone())
        } else {
            triple.object.clone()
        };

        Triple {
            subject,
            predicate,
            object,
        }
    }

    /// Check if a triple (possibly with blank nodes) matches any triple in the graph
    fn triple_matches_in_graph(&self, pattern: &Triple, graph: &RdfGraph) -> bool {
        // Check for exact match first
        if graph.contains_triple(pattern) {
            return true;
        }

        // If pattern contains blank nodes, check for compatible matches
        for graph_triple in graph.triples() {
            if self.triples_compatible(pattern, graph_triple) {
                return true;
            }
        }

        false
    }

    /// Check if two triples are compatible (same structure, blank nodes can match anything)
    fn triples_compatible(&self, pattern: &Triple, candidate: &Triple) -> bool {
        self.terms_compatible(&pattern.subject, &candidate.subject)
            && self.terms_compatible(&pattern.predicate, &candidate.predicate)
            && self.terms_compatible(&pattern.object, &candidate.object)
    }

    /// Check if two RDF terms are compatible for matching
    fn terms_compatible(&self, pattern_term: &RdfTerm, candidate_term: &RdfTerm) -> bool {
        match (pattern_term, candidate_term) {
            // Blank nodes in pattern can match any term
            (RdfTerm::BlankNode(_), _) => true,
            // Non-blank nodes must match exactly
            (term1, term2) => term1 == term2,
        }
    }
}

/// Blank Node Mapping for entailment checking
#[derive(Debug, Clone)]
pub struct BlankNodeMapping {
    mapping: HashMap<String, String>,
}

impl BlankNodeMapping {
    /// Create a new blank node mapping
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Add a mapping from conclusion blank node to premise blank node
    pub fn add_mapping(&mut self, conclusion_blank: String, premise_blank: String) {
        self.mapping.insert(conclusion_blank, premise_blank);
    }

    /// Get mapping for a blank node
    pub fn get_mapping(&self, blank_node: &str) -> Option<&String> {
        self.mapping.get(blank_node)
    }

    /// Apply mapping to an RDF term
    pub fn apply_to_term(&self, term: &RdfTerm) -> RdfTerm {
        match term {
            RdfTerm::BlankNode(id) => {
                if let Some(mapped_id) = self.mapping.get(id) {
                    RdfTerm::BlankNode(mapped_id.clone())
                } else {
                    term.clone()
                }
            }
            _ => term.clone(),
        }
    }

    /// Apply mapping to a triple
    pub fn apply_to_triple(&self, triple: &Triple) -> Triple {
        Triple {
            subject: self.apply_to_term(&triple.subject),
            predicate: self.apply_to_term(&triple.predicate),
            object: self.apply_to_term(&triple.object),
        }
    }
}

impl Default for BlankNodeMapping {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantics::RdfTerm;

    #[test]
    fn test_simple_interpretation() {
        let mut interp = RdfSimpleInterpretation::new();

        // Add resources to domain
        interp.add_resource("resource1".to_string());
        interp.add_resource("resource2".to_string());

        // Set property interpretation
        let mut prop_pairs = HashSet::new();
        prop_pairs.insert(("resource1".to_string(), "resource2".to_string()));
        interp.set_property_interpretation("http://example.org/knows".to_string(), prop_pairs);

        // Create a triple
        let subject = RdfTerm::iri("http://example.org/person1")
            .expect("Failed to create RDF IRI term from valid URI string");
        let predicate = RdfTerm::iri("http://example.org/knows")
            .expect("Failed to create RDF IRI term from valid URI string");
        let object = RdfTerm::iri("http://example.org/person2")
            .expect("Failed to create RDF IRI term from valid URI string");

        let triple = Triple {
            subject,
            predicate,
            object,
        };

        // Create graph with the triple
        let mut graph = RdfGraph::new();
        graph.add_triple(triple);

        // Set resource interpretations
        interp.set_resource_interpretation(
            "http://example.org/person1".to_string(),
            "resource1".to_string(),
        );
        interp.set_resource_interpretation(
            "http://example.org/person2".to_string(),
            "resource2".to_string(),
        );

        // Check if interpretation satisfies the graph
        assert!(interp.satisfies(&graph));
    }

    #[test]
    fn test_simple_entailment() {
        let mut premises = RdfGraph::new();
        let mut conclusion = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject")
            .expect("Failed to create RDF IRI term from valid URI string");
        let predicate = RdfTerm::iri("http://example.org/predicate")
            .expect("Failed to create RDF IRI term from valid URI string");
        let object = RdfTerm::literal("object");

        let triple = Triple {
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
        };

        premises.add_triple(triple.clone());
        conclusion.add_triple(triple);

        let entailment = RdfSimpleEntailment::new(premises.clone());
        assert!(entailment.entails(&premises, &conclusion));
    }

    #[test]
    fn test_blank_node_mapping() {
        let mut mapping = BlankNodeMapping::new();
        mapping.add_mapping("_:b1".to_string(), "_:x1".to_string());

        let blank_term = RdfTerm::blank_node("_:b1");
        let mapped_term = mapping.apply_to_term(&blank_term);

        if let RdfTerm::BlankNode(id) = mapped_term {
            assert_eq!(id, "_:x1");
        } else {
            panic!("Expected blank node");
        }
    }
}
