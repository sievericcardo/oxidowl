//! RDF Simple Entailment Implementation
//!
//! This module implements RDF simple entailment as defined in:
//! https://www.w3.org/TR/rdf11-mt/#simple-entailment

#![allow(dead_code)]

use super::{RdfGraph, RdfTerm, SemanticInterpretation, Triple};
use crate::Result;
use std::collections::{HashMap, HashSet};

/// RDF Simple Interpretation
///
/// Implements the formal semantics for RDF simple entailment
/// according to the RDF 1.1 Model Theory specification.
/// Extended to support RDF-star quoted triples when enabled.
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
    /// RDF 1.1 compatibility mode - disables RDF-star features
    rdf11_mode: bool,
    /// Quoted triple interpretation mapping (I_QTP: QuotedTriple -> Resource)
    /// Maps quoted triples to resources in the domain
    quoted_triple_interpretation: HashMap<String, String>,
}

impl RdfSimpleInterpretation {
    /// Create a new RDF simple interpretation  
    pub fn new() -> Self {
        Self {
            domain: HashSet::new(),
            property_interpretation: HashMap::new(),
            resource_interpretation: HashMap::new(),
            literal_interpretation: HashMap::new(),
            rdf11_mode: false,
            quoted_triple_interpretation: HashMap::new(),
        }
    }

    /// Create a new RDF simple interpretation with RDF 1.1 compatibility mode
    pub fn new_rdf11() -> Self {
        Self {
            domain: HashSet::new(),
            property_interpretation: HashMap::new(),
            resource_interpretation: HashMap::new(),
            literal_interpretation: HashMap::new(),
            rdf11_mode: true,
            quoted_triple_interpretation: HashMap::new(),
        }
    }

    /// Enable or disable RDF 1.1 compatibility mode
    pub fn set_rdf11_mode(&mut self, enabled: bool) {
        self.rdf11_mode = enabled;
    }

    /// Check if RDF 1.1 compatibility mode is enabled
    pub fn is_rdf11_mode(&self) -> bool {
        self.rdf11_mode
    }

    /// Add a resource to the domain
    pub fn add_resource(&mut self, resource: String) {
        self.domain.insert(resource);
    }

    /// Set guided triple interpretation (I_QTP)
    /// Maps a quoted triple to a resource in the interpretation domain
    pub fn set_quoted_triple_interpretation(&mut self, triple_id: String, resource: String) {
        if !self.rdf11_mode {
            self.quoted_triple_interpretation.insert(triple_id, resource);
        }
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
                ..
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
            RdfTerm::QuotedTriple(triple) => {
                if self.rdf11_mode {
                    // In RDF 1.1 mode, quoted triples are not supported
                    None
                } else {
                    // RDF-star: use the dedicated interpretation function
                    self.interpret_quoted_triple(triple)
                }
            }
        }
    }

    /// Interpret a quoted triple (I_QTP: QuotedTriple -> Resource)
    ///
    /// In RDF-star semantics, a quoted triple denotes a unique resource
    /// that represents the triple. This enables referential transparency:
    /// the same quoted triple always refers to the same resource.
    pub fn interpret_quoted_triple(&self, triple: &Box<Triple>) -> Option<String> {
        if self.rdf11_mode {
            return None;
        }

        // Create a canonical identifier for the quoted triple 
        // This ensures referential transparency: same triple = same resource
        let triple_id = format!(
            "<<{} {} {}>>",
            self.term_to_canonical_form(&triple.subject),
            self.term_to_canonical_form(&triple.predicate),
            self.term_to_canonical_form(&triple.object)
        );

        // Check if we have an explicit interpretation for this quoted triple
        self.quoted_triple_interpretation
            .get(&triple_id)
            .cloned()
            .or_else(|| {
                // Default: the quoted triple denotes itself as a resource
                Some(triple_id)
            })
    }

    /// Convert an RDF term to its canonical form for quoted triple identification
    fn term_to_canonical_form(&self, term: &RdfTerm) -> String {
        match term {
            RdfTerm::Iri(iri) => iri.to_string(),
            RdfTerm::BlankNode(id) => format!("_:{}", id),
            RdfTerm::Literal {
                value,
                datatype,
                language,
                ..
            } => {
                if let Some(dt) = datatype {
                    format!("\"{}\"^^<{}>", value, dt)
                } else if let Some(lang) = language {
                    format!("\"{}\"@{}", value, lang)
                } else {
                    format!("\"{}\"", value)
                }
            }
            RdfTerm::QuotedTriple(triple) => {
                // Recursive canonical form for nested quoted triples
                format!(
                    "<<{} {} {}>>",
                    self.term_to_canonical_form(&triple.subject),
                    self.term_to_canonical_form(&triple.predicate),
                    self.term_to_canonical_form(&triple.object)
                )
            }
        }
    }

    /// Check if a triple is satisfied by this interpretation
    fn satisfies_triple(&self, triple: &Triple) -> bool {
        // Handle quoted triples in subject or object position (RDF-star)
        if !self.rdf11_mode {
            // In RDF-star mode, check if this triple contains quoted triples 
            // and handle them with proper referential transparency
            if matches!(triple.subject, RdfTerm::QuotedTriple(_))
                || matches!(triple.object, RdfTerm::QuotedTriple(_))
            {
                return self.satisfies_triple_with_quoted_terms(triple);
            }
        }

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

    /// Check satisfiability of triples containing quoted triples (RDF-star semantics)
    ///
    /// Implements referential transparency: a quoted triple <<s p o>> is treated as
    /// a denoting term referring to a resource, not as an assertion of s p o.
    fn satisfies_triple_with_quoted_terms(&self, triple: &Triple) -> bool {
        // Interpret all terms including quoted triples as resources
        let subject_interp = self.interpret_rdf_term(&triple.subject);
        let predicate_interp = self.interpret_rdf_term(&triple.predicate);
        let object_interp = self.interpret_rdf_term(&triple.object);

        if let (Some(s), Some(p), Some(o)) = (subject_interp, predicate_interp, object_interp) {
            // Check the property interpretation
            if let Some(prop_pairs) = self.property_interpretation.get(&p) {
                prop_pairs.contains(&(s, o))
            } else {
                // Default: assume satisfied if not explicitly contradicted
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
/// Extended to support RDF-star entailment rules when enabled.
#[derive(Debug)]
pub struct RdfSimpleEntailment {
    /// Base graph
    base_graph: RdfGraph,
    /// Derived facts
    derived_graph: RdfGraph,
    /// RDF 1.1 compatibility mode - disables RDF-star features
    rdf11_mode: bool,
    /// Enable quoted triple entailment: if << s p o >> is asserted, infer s p o
    /// This is configurable because it may not be desired in all use cases
    enable_quoted_triple_entailment: bool,
}

impl RdfSimpleEntailment {
    /// Create a new RDF simple entailment engine
    pub fn new(base_graph: RdfGraph) -> Self {
        Self {
            base_graph,
            derived_graph: RdfGraph::new(),
            rdf11_mode: false,
            enable_quoted_triple_entailment: false,
        }
    }

    /// Create a new RDF simple entailment engine with RDF 1.1 compatibility
    pub fn new_rdf11(base_graph: RdfGraph) -> Self {
        Self {
            base_graph,
            derived_graph: RdfGraph::new(),
            rdf11_mode: true,
            enable_quoted_triple_entailment: false,
        }
    }

    /// Enable or disable RDF 1.1 compatibility mode
    pub fn set_rdf11_mode(&mut self, enabled: bool) {
        self.rdf11_mode = enabled;
    }

    /// Enable or disable quoted triple entailment
    /// When enabled: << s p o >> entails s p o
    pub fn set_quoted_triple_entailment(&mut self, enabled: bool) {
        self.enable_quoted_triple_entailment = enabled && !self.rdf11_mode;
    }

    /// Perform simple entailment reasoning
    pub fn reason(&mut self) -> Result<()> {
        // Copy base graph to derived graph
        self.derived_graph = self.base_graph.clone();

        // If RDF-star mode and quoted triple entailment is enabled, 
        // add derived triples from quoted triples
        if !self.rdf11_mode && self.enable_quoted_triple_entailment {
            self.derive_from_quoted_triples()?;
        }

        Ok(())
    }

    /// Derive triples from quoted triples
    /// Rule: if << s p o >> appears in the graph (as subject or object),
    /// then s p o is entailed (subject to configuration)
    fn derive_from_quoted_triples(&mut self) -> Result<()> {
        let mut derived = Vec::new();

        // Extract all quoted triples from the base graph
        for triple in self.base_graph.triples() {
            // Check subject position
            if let RdfTerm::QuotedTriple(quoted) = &triple.subject {
                derived.push((**quoted).clone());
            }

            // Check object position
            if let RdfTerm::QuotedTriple(quoted) = &triple.object {
                derived.push((**quoted).clone());
            }

            // Recursively extract from nested quoted triples
            self.extract_nested_quoted_triples(&triple.subject, &mut derived);
            self.extract_nested_quoted_triples(&triple.object, &mut derived);
        }

        // Add derived triples to the derived graph
        for triple in derived {
            self.derived_graph.add_triple(triple);
        }

        Ok(())
    }

    /// Recursively extract quoted triples from nested structures
    fn extract_nested_quoted_triples(&self, term: &RdfTerm, result: &mut Vec<Triple>) {
        if let RdfTerm::QuotedTriple(quoted) = term {
            result.push((**quoted).clone());

            // Recursively extract from the quoted triple's terms
            self.extract_nested_quoted_triples(&quoted.subject, result);
            self.extract_nested_quoted_triples(&quoted.object, result);
        }
    }

    /// Get the closure (base + derived facts)
    pub fn closure(&self) -> RdfGraph {
        let mut closure = self.base_graph.clone();
        closure.merge(&self.derived_graph);
        closure
    }

    /// Check if premises entail conclusion
    pub fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // In RDF-star mode with quoted triple entailment, check both regular
        // and quoted triple entailment
        if !self.rdf11_mode && self.enable_quoted_triple_entailment {
            // Check if conclusion can be derived from premises considering quoted triples
            self.check_entailment_with_quoted_triples(premises, conclusion)
        } else {
            // Standard RDF simple entailment
            self.check_simple_entailment(premises, conclusion)
        }
    }

    /// Check entailment specifically for quoted triples (RDF-star)
    ///
    /// Returns true if the premises entail the conclusion considering:
    /// 1. Regular graph entailment
    /// 2. Quoted triple entailment: if << s p o >> is in premises, then s p o is entailed
    pub fn entails_quoted(&self, premises: &RdfGraph, quoted_triple: &Triple) -> bool {
        if self.rdf11_mode {
            // In RDF 1.1 mode, no quoted triple entailment
            return false;
        }

        // Check if the quoted triple itself appears in premises
        if premises.contains_triple(quoted_triple) {
            return true;
        }

        // Check if << s p o >> appears as a term in premises
        let quoted_term = RdfTerm::QuotedTriple(Box::new(quoted_triple.clone()));

        for triple in premises.triples() {
            // Check if quoted triple appears in subject or object position
            if triple.subject == quoted_term || triple.object == quoted_term {
                // If the quoted triple is mentioned and entailment is enabled,
                // then the triple itself is entailed
                if self.enable_quoted_triple_entailment {
                    return true;
                }
            }

            // Check nested quoted triples
            if self.contains_quoted_triple_term(&triple.subject, quoted_triple)
                || self.contains_quoted_triple_term(&triple.object, quoted_triple)
            {
                if self.enable_quoted_triple_entailment {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a term contains a specific quoted triple (handles nesting)
    fn contains_quoted_triple_term(&self, term: &RdfTerm, target: &Triple) -> bool {
        match term {
            RdfTerm::QuotedTriple(quoted) =>  {
                if **quoted == *target {
                    true
                } else {
                    // Recursively check nested quoted triples
                    self.contains_quoted_triple_term(&quoted.subject, target)
                        || self.contains_quoted_triple_term(&quoted.object, target)
                }
            }
            _ => false,
        }
    }

    /// Check entailment considering quoted triple semantics
    fn check_entailment_with_quoted_triples(
        &self,
        premises: &RdfGraph,
        conclusion: &RdfGraph,
    ) -> bool {
        // First, expand premises with derived facts from quoted triples
        let mut expanded_premises = premises.clone();

        if self.enable_quoted_triple_entailment {
            // Extract and add all quoted triples
            for triple in premises.triples() {
                if let RdfTerm::QuotedTriple(quoted) = &triple.subject {
                    expanded_premises.add_triple((**quoted).clone());
                }
                if let RdfTerm::QuotedTriple(quoted) = &triple.object {
                    expanded_premises.add_triple((**quoted).clone());
                }
            }
        }

        // Now check regular entailment against expanded premises
        self.check_simple_entailment(&expanded_premises, conclusion)
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

    #[test]
    fn test_quoted_triple_interpretation() {
        let mut interp = RdfSimpleInterpretation::new();

        // Create a quoted triple
        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        };

        let quoted_term = RdfTerm::QuotedTriple(Box::new(inner_triple));

        // Interpret the quoted triple - should return a resource identifier
        let interpretation = interp.interpret_rdf_term(&quoted_term);
        assert!(interpretation.is_some());

        // The same quoted triple should always have the same interpretation (referential transparency)
        let interpretation2 = interp.interpret_rdf_term(&quoted_term);
        assert_eq!(interpretation, interpretation2);
    }

    #[test]
    fn test_quoted_triple_referential_transparency() {
        let interp = RdfSimpleInterpretation::new();

        // Create two identical quoted triples
        let inner_triple1 = Box::new(Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        });

        let inner_triple2 = Box::new(Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        });

        // Both should have identical interpretations (referential transparency)
        let interp1 = interp.interpret_quoted_triple(&inner_triple1);
        let interp2 = interp.interpret_quoted_triple(&inner_triple2);

        assert_eq!(interp1, interp2);
    }

    #[test]
    fn test_rdf11_mode_disables_quoted_triples() {
        let mut interp = RdfSimpleInterpretation::new_rdf11();

        assert!(interp.is_rdf11_mode());

        // Create a quoted triple
        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        };

        let quoted_term = RdfTerm::QuotedTriple(Box::new(inner_triple));

        // In RDF 1.1 mode, quoted triples should not be interpretable
        let interpretation = interp.interpret_rdf_term(&quoted_term);
        assert!(interpretation.is_none());

        // Setting quoted triple interpretation should have no effect in RDF 1.1 mode
        interp.set_quoted_triple_interpretation(
            "<<test>>".to_string(),
            "resource1".to_string(),
        );
        assert!(interp.quoted_triple_interpretation.is_empty());
    }

    #[test]
    fn test_nested_quoted_triples() {
        let interp = RdfSimpleInterpretation::new();

        // Create a nested quoted triple: << << :a :b :c >> :certainty "high" >>
        let inner_inner = Box::new(Triple {
            subject: RdfTerm::iri("http://example.org/a").unwrap(),
            predicate: RdfTerm::iri("http://example.org/b").unwrap(),
            object: RdfTerm::iri("http://example.org/c").unwrap(),
        });

        let inner = Box::new(Triple {
            subject: RdfTerm::QuotedTriple(inner_inner),
            predicate: RdfTerm::iri("http://example.org/certainty").unwrap(),
            object: RdfTerm::literal("high"),
        });

        // Should be able to interpret nested quoted triples
        let interpretation = interp.interpret_quoted_triple(&inner);
        assert!(interpretation.is_some());

        // Check canonical form includes nesting
        let canonical = interpretation.unwrap();
        assert!(canonical.contains("<<"));
        assert!(canonical.contains(">>"));
    }

    #[test]
    fn test_satisfies_with_quoted_triple() {
        let mut interp = RdfSimpleInterpretation::new();

        // Create a triple with a quoted triple in subject position
        let inner_triple = Box::new(Triple {
            subject: RdfTerm::iri("http://example.org/doc1").unwrap(),
            predicate: RdfTerm::iri("http://example.org/author").unwrap(),
            object: RdfTerm::literal("Smith"),
        });

        let triple = Triple {
            subject: RdfTerm::QuotedTriple(inner_triple.clone()),
            predicate: RdfTerm::iri("http://example.org/source").unwrap(),
            object: RdfTerm::iri("http://example.org/archive23").unwrap(),
        };

        let mut graph = RdfGraph::new();
        graph.add_triple(triple);

        // Set up interpretation mappings
        let quoted_resource = interp.interpret_quoted_triple(&inner_triple).unwrap();
        let source_pred = "http://example.org/source";
        let archive_resource = "http://example.org/archive23";

        let mut prop_pairs = HashSet::new();
        prop_pairs.insert((quoted_resource.clone(), archive_resource.to_string()));
        interp.set_property_interpretation(source_pred.to_string(), prop_pairs);

        // Graph should be satisfied by the interpretation
        assert!(interp.satisfies(&graph));
    }

    #[test]
    fn test_rdf11_mode_toggle() {
        let mut interp = RdfSimpleInterpretation::new();

        assert!(!interp.is_rdf11_mode());

        // Enable RDF 1.1 mode
        interp.set_rdf11_mode(true);
        assert!(interp.is_rdf11_mode());

        // Disable RDF 1.1 mode
        interp.set_rdf11_mode(false);
        assert!(!interp.is_rdf11_mode());
    }

    #[test]
    fn test_quoted_triple_entailment_basic() {
        let mut premises = RdfGraph::new();

        // Create a triple with a quoted triple: << :alice :knows :bob >> :certainty "high"
        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        };

        let outer_triple = Triple {
            subject: RdfTerm::QuotedTriple(Box::new(inner_triple.clone())),
            predicate: RdfTerm::iri("http://example.org/certainty").unwrap(),
            object: RdfTerm::literal("high"),
        };

        premises.add_triple(outer_triple);

        // Create entailment engine with quoted triple entailment enabled
        let mut entailment = RdfSimpleEntailment::new(premises.clone());
        entailment.set_quoted_triple_entailment(true);

        // The quoted triple << :alice :knows :bob >> should entail :alice :knows :bob
        assert!(entailment.entails_quoted(&premises, &inner_triple));
    }

    #[test]
    fn test_quoted_triple_entailment_reasoning() {
        let mut premises = RdfGraph::new();

        // Add: << :doc1 :author "Smith" >> :source :archive23
        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/doc1").unwrap(),
            predicate: RdfTerm::iri("http://example.org/author").unwrap(),
            object: RdfTerm::literal("Smith"),
        };

        premises.add_triple(Triple {
            subject: RdfTerm::QuotedTriple(Box::new(inner_triple.clone())),
            predicate: RdfTerm::iri("http://example.org/source").unwrap(),
            object: RdfTerm::iri("http://example.org/archive23").unwrap(),
        });

        // Create entailment engine and enable quoted triple entailment
        let mut entailment = RdfSimpleEntailment::new(premises.clone());
        entailment.set_quoted_triple_entailment(true);
        entailment.reason().unwrap();

        // The closure should contain both the original triple and the derived triple
        let closure = entailment.closure();

        // Check that the inner triple is derived
        assert!(closure.contains_triple(&inner_triple));
    }

    #[test]
    fn test_rdf11_mode_no_quoted_entailment() {
        let mut premises = RdfGraph::new();

        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        };

        premises.add_triple(Triple {
            subject: RdfTerm::QuotedTriple(Box::new(inner_triple.clone())),
            predicate: RdfTerm::iri("http://example.org/certainty").unwrap(),
            object: RdfTerm::literal("high"),
        });

        // Create RDF 1.1 mode entailment engine
        let entailment = RdfSimpleEntailment::new_rdf11(premises.clone());

        // In RDF 1.1 mode, quoted triple entailment should not work
        assert!(!entailment.entails_quoted(&premises, &inner_triple));
    }

    #[test]
    fn test_nested_quoted_triple_entailment() {
        let mut premises = RdfGraph::new();

        // Create nested: << << :a :b :c >> :d :e >> :f :g
        let inner_inner = Triple {
            subject: RdfTerm::iri("http://example.org/a").unwrap(),
            predicate: RdfTerm::iri("http://example.org/b").unwrap(),
            object: RdfTerm::iri("http://example.org/c").unwrap(),
        };

        let inner = Box::new(Triple {
            subject: RdfTerm::QuotedTriple(Box::new(inner_inner.clone())),
            predicate: RdfTerm::iri("http://example.org/d").unwrap(),
            object: RdfTerm::iri("http://example.org/e").unwrap(),
        });

        premises.add_triple(Triple {
            subject: RdfTerm::QuotedTriple(inner),
            predicate: RdfTerm::iri("http://example.org/f").unwrap(),
            object: RdfTerm::iri("http://example.org/g").unwrap(),
        });

        // Create entailment engine with quoted triple entailment enabled
        let mut entailment = RdfSimpleEntailment::new(premises.clone());
        entailment.set_quoted_triple_entailment(true);
        entailment.reason().unwrap();

        // The innermost triple should be entailed
        let closure = entailment.closure();
        assert!(closure.contains_triple(&inner_inner));
    }

    #[test]
    fn test_entailment_without_quoted_triple_flag() {
        let mut premises = RdfGraph::new();

        let inner_triple = Triple {
            subject: RdfTerm::iri("http://example.org/alice").unwrap(),
            predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
            object: RdfTerm::iri("http://example.org/bob").unwrap(),
        };

        premises.add_triple(Triple {
            subject: RdfTerm::QuotedTriple(Box::new(inner_triple.clone())),
            predicate: RdfTerm::iri("http://example.org/certainty").unwrap(),
            object: RdfTerm::literal("high"),
        });

        // Create entailment engine WITHOUT enabling quoted triple entailment
        let mut entailment = RdfSimpleEntailment::new(premises.clone());
        entailment.reason().unwrap();

        // The inner triple should NOT be derived
        let closure = entailment.closure();
        assert!(!closure.contains_triple(&inner_triple));
    }
}
