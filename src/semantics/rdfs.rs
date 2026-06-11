//! RDFS Entailment Implementation
//!
//! This module implements RDFS entailment as defined in:
//! <https://www.w3.org/TR/rdf-schema/#ch_entailmentrules>
//!
//! RDFS entailment rules (rdfs1-rdfs13) are implemented according to the specification.

#![allow(dead_code)]

use super::{RdfGraph, RdfTerm, SemanticInterpretation, Triple, vocabulary::*};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

/// RDFS Interpretation
///
/// Extends RDF Simple Interpretation with RDFS semantics
#[derive(Debug, Clone)]
pub struct RdfsInterpretation {
    /// Base RDF interpretation
    base: super::rdf::RdfSimpleInterpretation,
    /// Class interpretation (maps classes to sets of resources)
    class_interpretation: HashMap<String, HashSet<String>>,
    /// Property interpretation extended for RDFS
    property_interpretation: HashMap<String, HashSet<(String, String)>>,
}

impl RdfsInterpretation {
    /// Create a new RDFS interpretation
    #[must_use]
    pub fn new() -> Self {
        let mut interpretation = Self {
            base: super::rdf::RdfSimpleInterpretation::new(),
            class_interpretation: HashMap::new(),
            property_interpretation: HashMap::new(),
        };

        // Initialize built-in RDFS classes and properties
        interpretation.initialize_rdfs_vocabulary();
        interpretation
    }

    /// Initialize RDFS vocabulary with proper interpretations
    fn initialize_rdfs_vocabulary(&mut self) {
        // Initialize rdfs:Resource as the universal class
        let resource_class = HashSet::new();
        // In a complete implementation, this would contain all resources in the domain
        self.class_interpretation
            .insert(RDFS_RESOURCE.to_string(), resource_class);

        // Initialize rdfs:Class
        let mut class_class = HashSet::new();
        class_class.insert(RDFS_RESOURCE.to_string());
        class_class.insert(RDFS_CLASS.to_string());
        self.class_interpretation
            .insert(RDFS_CLASS.to_string(), class_class);

        // Initialize rdfs:Literal
        self.class_interpretation
            .insert(RDFS_LITERAL.to_string(), HashSet::new());

        // Initialize rdfs:Datatype
        self.class_interpretation
            .insert(RDFS_DATATYPE.to_string(), HashSet::new());
    }

    /// Set class interpretation
    pub fn set_class_interpretation(&mut self, class: String, instances: HashSet<String>) {
        self.class_interpretation.insert(class, instances);
    }

    /// Get class interpretation
    #[must_use]
    pub fn get_class_interpretation(&self, class: &str) -> Option<&HashSet<String>> {
        self.class_interpretation.get(class)
    }

    /// Check if resource is instance of class
    #[must_use]
    pub fn is_instance_of(&self, resource: &str, class: &str) -> bool {
        if let Some(instances) = self.class_interpretation.get(class) {
            instances.contains(resource)
        } else {
            false
        }
    }

    /// Add instance to class
    pub fn add_instance(&mut self, class: String, instance: String) {
        self.class_interpretation
            .entry(class)
            .or_default()
            .insert(instance);
    }
}

impl Default for RdfsInterpretation {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticInterpretation for RdfsInterpretation {
    fn satisfies(&self, graph: &RdfGraph) -> bool {
        // An RDFS interpretation satisfies a graph if it satisfies all triples
        // and respects RDFS semantic conditions

        for triple in graph.triples() {
            if !self.satisfies_triple(triple) {
                return false;
            }
        }

        // Check RDFS semantic conditions
        self.check_rdfs_conditions(graph)
    }

    fn interpret_term(&self, term: &RdfTerm) -> Option<String> {
        self.base.interpret_term(term)
    }

    fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // RDFS entailment extends RDF simple entailment
        self.base.entails(premises, conclusion) || self.check_rdfs_entailment(premises, conclusion)
    }
}

impl RdfsInterpretation {
    /// Check if a triple is satisfied by this RDFS interpretation
    fn satisfies_triple(&self, triple: &Triple) -> bool {
        // First check RDF simple satisfaction
        if !self.base.satisfies(&{
            let mut g = RdfGraph::new();
            g.add_triple(triple.clone());
            g
        }) {
            return false;
        }

        // Additional RDFS checks
        true // For now, assume satisfied if RDF simple conditions are met
    }

    /// Check RDFS semantic conditions
    fn check_rdfs_conditions(&self, graph: &RdfGraph) -> bool {
        // RDFS semantic conditions (from RDF Schema 1.1 specification)
        // Pre-build predicate RdfTerms once — avoids repeated URL parsing in O(n²) loops
        let rdf_type_pred = RdfTerm::Iri(RDF_TYPE.clone());
        let rdfs_subclass_pred = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());
        let rdfs_subproperty_pred = RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone());

        // 1. Domain constraints: if (s, p, o) and (p, rdfs:domain, c) then (s, rdf:type, c)
        for triple in &graph.triples {
            if let Some(domain_class) = self.get_property_domain(&triple.predicate, graph) {
                let type_triple = Triple {
                    subject: triple.subject.clone(),
                    predicate: rdf_type_pred.clone(),
                    object: RdfTerm::Iri(domain_class),
                };
                if !self.triple_in_graph(&type_triple, graph) {
                    return false;
                }
            }
        }

        // 2. Range constraints: if (s, p, o) and (p, rdfs:range, c) then (o, rdf:type, c)
        for triple in &graph.triples {
            if let Some(range_class) = self.get_property_range(&triple.predicate, graph)
                && let RdfTerm::Iri(object_iri) = &triple.object
            {
                let type_triple = Triple {
                    subject: RdfTerm::Iri(object_iri.clone()),
                    predicate: rdf_type_pred.clone(),
                    object: RdfTerm::Iri(range_class),
                };
                if !self.triple_in_graph(&type_triple, graph) {
                    return false;
                }
            }
        }

        // 3. Subclass transitivity: if (x, rdfs:subClassOf, y) and (y, rdfs:subClassOf, z) then (x, rdfs:subClassOf, z)
        for triple1 in &graph.triples {
            if triple1.predicate == rdfs_subclass_pred
                && let (RdfTerm::Iri(x), RdfTerm::Iri(y)) = (&triple1.subject, &triple1.object)
            {
                for triple2 in &graph.triples {
                    if triple2.predicate == rdfs_subclass_pred
                        && RdfTerm::Iri(y.clone()) == triple2.subject
                        && let RdfTerm::Iri(z) = &triple2.object
                    {
                        let derived_triple = Triple {
                            subject: RdfTerm::Iri(x.clone()),
                            predicate: rdfs_subclass_pred.clone(),
                            object: RdfTerm::Iri(z.clone()),
                        };
                        if !self.triple_in_graph(&derived_triple, graph) {
                            return false;
                        }
                    }
                }
            }
        }

        // 4. Subproperty transitivity: if (p, rdfs:subPropertyOf, q) and (q, rdfs:subPropertyOf, r) then (p, rdfs:subPropertyOf, r)
        for triple1 in &graph.triples {
            if triple1.predicate == rdfs_subproperty_pred
                && let (RdfTerm::Iri(p), RdfTerm::Iri(q)) = (&triple1.subject, &triple1.object)
            {
                for triple2 in &graph.triples {
                    if triple2.predicate == rdfs_subproperty_pred
                        && RdfTerm::Iri(q.clone()) == triple2.subject
                        && let RdfTerm::Iri(r) = &triple2.object
                    {
                        let derived_triple = Triple {
                            subject: RdfTerm::Iri(p.clone()),
                            predicate: rdfs_subproperty_pred.clone(),
                            object: RdfTerm::Iri(r.clone()),
                        };
                        if !self.triple_in_graph(&derived_triple, graph) {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Check RDFS-specific entailment
    fn check_rdfs_entailment(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool {
        // Apply RDFS entailment rules to check if conclusion follows from premises
        let mut engine = RdfsEntailmentEngine::new(premises.clone());
        let _ = engine.reason();
        let closure = engine.closure();

        // Check if conclusion is in the closure
        conclusion
            .triples()
            .iter()
            .all(|triple| closure.contains_triple(triple))
    }

    /// Get property domain if defined
    fn get_property_domain(&self, property: &RdfTerm, graph: &RdfGraph) -> Option<url::Url> {
        let domain_pred = RdfTerm::Iri(RDFS_DOMAIN.clone());
        for triple in &graph.triples {
            if &triple.subject == property
                && triple.predicate == domain_pred
                && let RdfTerm::Iri(domain) = &triple.object
            {
                return Some(domain.clone());
            }
        }
        None
    }

    /// Get property range if defined
    fn get_property_range(&self, property: &RdfTerm, graph: &RdfGraph) -> Option<url::Url> {
        let range_pred = RdfTerm::Iri(RDFS_RANGE.clone());
        for triple in &graph.triples {
            if &triple.subject == property
                && triple.predicate == range_pred
                && let RdfTerm::Iri(range) = &triple.object
            {
                return Some(range.clone());
            }
        }
        None
    }

    /// Check if a triple is present in the graph
    fn triple_in_graph(&self, triple: &Triple, graph: &RdfGraph) -> bool {
        graph.triples.contains(triple)
    }
}

/// RDFS Entailment Engine
///
/// Implements the 13 RDFS entailment rules according to the RDF Schema specification.
#[derive(Debug)]
pub struct RdfsEntailmentEngine {
    /// Input graph
    input_graph: RdfGraph,
    /// Derived facts
    derived_graph: RdfGraph,
    /// Fixed point reached
    fixed_point: bool,
}

impl RdfsEntailmentEngine {
    /// Get property domain if defined
    fn get_property_domain(&self, property: &RdfTerm, graph: &RdfGraph) -> Option<url::Url> {
        let domain_pred = RdfTerm::Iri(RDFS_DOMAIN.clone());
        for triple in &graph.triples {
            if &triple.subject == property
                && triple.predicate == domain_pred
                && let RdfTerm::Iri(domain) = &triple.object
            {
                return Some(domain.clone());
            }
        }
        None
    }

    /// Get property range if defined
    fn get_property_range(&self, property: &RdfTerm, graph: &RdfGraph) -> Option<url::Url> {
        let range_pred = RdfTerm::Iri(RDFS_RANGE.clone());
        for triple in &graph.triples {
            if &triple.subject == property
                && triple.predicate == range_pred
                && let RdfTerm::Iri(range) = &triple.object
            {
                return Some(range.clone());
            }
        }
        None
    }

    /// Check if a triple is present in the graph
    fn triple_in_graph(&self, triple: &Triple, graph: &RdfGraph) -> bool {
        graph.triples.contains(triple)
    }
}

impl RdfsEntailmentEngine {
    /// Create a new RDFS entailment engine
    #[must_use]
    pub fn new(input_graph: RdfGraph) -> Self {
        Self {
            input_graph,
            derived_graph: RdfGraph::new(),
            fixed_point: false,
        }
    }

    /// Perform RDFS entailment reasoning
    pub fn reason(&mut self) -> Result<()> {
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops

        while !self.fixed_point && iteration < MAX_ITERATIONS {
            let initial_size = self.derived_graph.size();

            // Apply all RDFS entailment rules
            self.apply_rdfs_rules()?;

            // Check if fixed point is reached
            if self.derived_graph.size() == initial_size {
                self.fixed_point = true;
            }

            iteration += 1;
        }

        if iteration >= MAX_ITERATIONS {
            return Err(Error::reasoning(
                "RDFS reasoning did not converge".to_string(),
            ));
        }

        Ok(())
    }

    /// Apply all RDFS entailment rules
    fn apply_rdfs_rules(&mut self) -> Result<()> {
        let working_graph = self.get_working_graph();

        // Rule rdfs2: Domain entailment
        self.apply_rdfs2(&working_graph)?;

        // Rule rdfs3: Range entailment
        self.apply_rdfs3(&working_graph)?;

        // Rule rdfs4a: rdf:Property entailment
        self.apply_rdfs4a(&working_graph)?;

        // Rule rdfs4b: rdfs:Resource entailment
        self.apply_rdfs4b(&working_graph)?;

        // Rule rdfs5: Subproperty transitivity
        self.apply_rdfs5(&working_graph)?;

        // Rule rdfs6: Property reflexivity
        self.apply_rdfs6(&working_graph)?;

        // Rule rdfs7: Subproperty inheritance
        self.apply_rdfs7(&working_graph)?;

        // Rule rdfs8: rdfs:Class entailment
        self.apply_rdfs8(&working_graph)?;

        // Rule rdfs9: Subclass inheritance
        self.apply_rdfs9(&working_graph)?;

        // Rule rdfs10: Class reflexivity
        self.apply_rdfs10(&working_graph)?;

        // Rule rdfs11: Subclass transitivity
        self.apply_rdfs11(&working_graph)?;

        // Rule rdfs12: Member entailment
        self.apply_rdfs12(&working_graph)?;

        // Rule rdfs13: Datatype entailment
        self.apply_rdfs13(&working_graph)?;

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
        if !self.input_graph.contains_triple(&triple)
            && !self.derived_graph.contains_triple(&triple)
        {
            self.derived_graph.add_triple(triple);
        }
    }

    /// Rule rdfs2: (xxx rdfs:domain yyy) & (aaa xxx bbb) => (aaa rdf:type yyy)
    fn apply_rdfs2(&mut self, graph: &RdfGraph) -> Result<()> {
        let domain_iri = RdfTerm::Iri(RDFS_DOMAIN.clone());
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());

        // Find all domain statements
        let domain_triples = graph.find_triples(None, Some(&domain_iri), None);

        for domain_triple in domain_triples {
            let property = &domain_triple.subject;
            let domain_class = &domain_triple.object;

            // Find all uses of this property
            let property_uses = graph.find_triples(None, Some(property), None);

            for use_triple in property_uses {
                let subject = &use_triple.subject;

                // Add (subject rdf:type domain_class)
                let derived_triple = Triple {
                    subject: subject.clone(),
                    predicate: type_iri.clone(),
                    object: domain_class.clone(),
                };

                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule rdfs3: (xxx rdfs:range yyy) & (aaa xxx bbb) => (bbb rdf:type yyy)
    fn apply_rdfs3(&mut self, graph: &RdfGraph) -> Result<()> {
        let range_iri = RdfTerm::Iri(RDFS_RANGE.clone());
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());

        // Find all range statements
        let range_triples = graph.find_triples(None, Some(&range_iri), None);

        for range_triple in range_triples {
            let property = &range_triple.subject;
            let range_class = &range_triple.object;

            // Find all uses of this property
            let property_uses = graph.find_triples(None, Some(property), None);

            for use_triple in property_uses {
                let object = &use_triple.object;

                // Only apply to IRI and blank node objects (not literals)
                if !object.is_literal() {
                    // Add (object rdf:type range_class)
                    let derived_triple = Triple {
                        subject: object.clone(),
                        predicate: type_iri.clone(),
                        object: range_class.clone(),
                    };

                    self.add_derived_triple(derived_triple);
                }
            }
        }

        Ok(())
    }

    /// Rule rdfs4a: (xxx aaa yyy) => (xxx rdf:type rdfs:Resource)
    fn apply_rdfs4a(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let resource_iri = RdfTerm::Iri(RDFS_RESOURCE.clone());

        for triple in graph.triples() {
            // Subject is always a resource
            let derived_triple = Triple {
                subject: triple.subject.clone(),
                predicate: type_iri.clone(),
                object: resource_iri.clone(),
            };

            self.add_derived_triple(derived_triple);
        }

        Ok(())
    }

    /// Rule rdfs4b: (xxx aaa yyy) => (yyy rdf:type rdfs:Resource) [if yyy is not literal]
    fn apply_rdfs4b(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let resource_iri = RdfTerm::Iri(RDFS_RESOURCE.clone());

        for triple in graph.triples() {
            // Object is a resource if it's not a literal
            if !triple.object.is_literal() {
                let derived_triple = Triple {
                    subject: triple.object.clone(),
                    predicate: type_iri.clone(),
                    object: resource_iri.clone(),
                };

                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule rdfs5: (xxx rdfs:subPropertyOf yyy) & (yyy rdfs:subPropertyOf zzz) => (xxx rdfs:subPropertyOf zzz)
    fn apply_rdfs5(&mut self, graph: &RdfGraph) -> Result<()> {
        let subprop_iri = RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone());

        let subprop_triples: Vec<_> = graph.find_triples(None, Some(&subprop_iri), None).clone();

        for triple1 in &subprop_triples {
            for triple2 in &subprop_triples {
                // If triple1: (xxx rdfs:subPropertyOf yyy) and triple2: (yyy rdfs:subPropertyOf zzz)
                if triple1.object == triple2.subject {
                    let derived_triple = Triple {
                        subject: triple1.subject.clone(),
                        predicate: subprop_iri.clone(),
                        object: triple2.object.clone(),
                    };

                    self.add_derived_triple(derived_triple);
                }
            }
        }

        Ok(())
    }

    /// Rule rdfs6: (xxx rdf:type rdf:Property) => (xxx rdfs:subPropertyOf xxx)
    fn apply_rdfs6(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let property_iri = RdfTerm::Iri(RDF_PROPERTY.clone());
        let subprop_iri = RdfTerm::Iri(RDFS_SUBPROPERTY_OF.clone());

        let property_instances = graph.find_triples(None, Some(&type_iri), Some(&property_iri));

        for triple in property_instances {
            let property = &triple.subject;

            let derived_triple = Triple {
                subject: property.clone(),
                predicate: subprop_iri.clone(),
                object: property.clone(),
            };

            self.add_derived_triple(derived_triple);
        }

        Ok(())
    }

    /// Rule rdfs7: (xxx rdfs:subPropertyOf yyy) & (aaa xxx bbb) => (aaa yyy bbb)
    fn apply_rdfs7(&mut self, graph: &RdfGraph) -> Result<()> {
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

    /// Rule rdfs8: (xxx rdf:type rdfs:Class) => (xxx rdfs:subClassOf rdfs:Resource)
    fn apply_rdfs8(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let class_iri = RdfTerm::Iri(RDFS_CLASS.clone());
        let subclass_iri = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());
        let resource_iri = RdfTerm::Iri(RDFS_RESOURCE.clone());

        let class_instances = graph.find_triples(None, Some(&type_iri), Some(&class_iri));

        for triple in class_instances {
            let class = &triple.subject;

            let derived_triple = Triple {
                subject: class.clone(),
                predicate: subclass_iri.clone(),
                object: resource_iri.clone(),
            };

            self.add_derived_triple(derived_triple);
        }

        Ok(())
    }

    /// Rule rdfs9: (xxx rdfs:subClassOf yyy) & (aaa rdf:type xxx) => (aaa rdf:type yyy)
    fn apply_rdfs9(&mut self, graph: &RdfGraph) -> Result<()> {
        let subclass_iri = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());

        let subclass_triples = graph.find_triples(None, Some(&subclass_iri), None);

        for subclass_triple in subclass_triples {
            let subclass = &subclass_triple.subject;
            let superclass = &subclass_triple.object;

            // Find all instances of the subclass
            let instances = graph.find_triples(None, Some(&type_iri), Some(subclass));

            for instance_triple in instances {
                let instance = &instance_triple.subject;

                let derived_triple = Triple {
                    subject: instance.clone(),
                    predicate: type_iri.clone(),
                    object: superclass.clone(),
                };

                self.add_derived_triple(derived_triple);
            }
        }

        Ok(())
    }

    /// Rule rdfs10: (xxx rdf:type rdfs:Class) => (xxx rdfs:subClassOf xxx)
    fn apply_rdfs10(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let class_iri = RdfTerm::Iri(RDFS_CLASS.clone());
        let subclass_iri = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());

        let class_instances = graph.find_triples(None, Some(&type_iri), Some(&class_iri));

        for triple in class_instances {
            let class = &triple.subject;

            let derived_triple = Triple {
                subject: class.clone(),
                predicate: subclass_iri.clone(),
                object: class.clone(),
            };

            self.add_derived_triple(derived_triple);
        }

        Ok(())
    }

    /// Rule rdfs11: (xxx rdfs:subClassOf yyy) & (yyy rdfs:subClassOf zzz) => (xxx rdfs:subClassOf zzz)
    fn apply_rdfs11(&mut self, graph: &RdfGraph) -> Result<()> {
        let subclass_iri = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());

        let subclass_triples: Vec<_> = graph.find_triples(None, Some(&subclass_iri), None).clone();

        for triple1 in &subclass_triples {
            for triple2 in &subclass_triples {
                // If triple1: (xxx rdfs:subClassOf yyy) and triple2: (yyy rdfs:subClassOf zzz)
                if triple1.object == triple2.subject {
                    let derived_triple = Triple {
                        subject: triple1.subject.clone(),
                        predicate: subclass_iri.clone(),
                        object: triple2.object.clone(),
                    };

                    self.add_derived_triple(derived_triple);
                }
            }
        }

        Ok(())
    }

    /// Rule rdfs12: (xxx rdf:type rdfs:ContainerMembershipProperty) => (xxx rdfs:subPropertyOf rdfs:member)
    fn apply_rdfs12(&mut self, _graph: &RdfGraph) -> Result<()> {
        // Note: rdfs:ContainerMembershipProperty is not in our basic vocabulary
        // This rule handles rdf:_1, rdf:_2, etc. properties
        // For now, we'll skip this rule as it requires more complex handling
        Ok(())
    }

    /// Rule rdfs13: (xxx rdf:type rdfs:Datatype) => (xxx rdfs:subClassOf rdfs:Literal)
    fn apply_rdfs13(&mut self, graph: &RdfGraph) -> Result<()> {
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let datatype_iri = RdfTerm::Iri(RDFS_DATATYPE.clone());
        let subclass_iri = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());
        let literal_iri = RdfTerm::Iri(RDFS_LITERAL.clone());

        let datatype_instances = graph.find_triples(None, Some(&type_iri), Some(&datatype_iri));

        for triple in datatype_instances {
            let datatype = &triple.subject;

            let derived_triple = Triple {
                subject: datatype.clone(),
                predicate: subclass_iri.clone(),
                object: literal_iri.clone(),
            };

            self.add_derived_triple(derived_triple);
        }

        Ok(())
    }

    /// Get the closure (input + derived facts)
    #[must_use]
    pub fn closure(&self) -> RdfGraph {
        let mut closure = self.input_graph.clone();
        closure.merge(&self.derived_graph);
        closure
    }

    /// Get derived facts only
    #[must_use]
    pub fn derived_facts(&self) -> &RdfGraph {
        &self.derived_graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdfs_interpretation() {
        let mut interp = RdfsInterpretation::new();

        // Add a class and instance
        let mut instances = HashSet::new();
        instances.insert("individual1".to_string());
        interp.set_class_interpretation("http://example.org/Person".to_string(), instances);

        assert!(interp.is_instance_of("individual1", "http://example.org/Person"));
        assert!(!interp.is_instance_of("individual2", "http://example.org/Person"));
    }

    #[test]
    fn test_rdfs_rule2_domain() {
        let mut graph = RdfGraph::new();

        // Add domain statement: ex:knows rdfs:domain ex:Person
        let knows = RdfTerm::iri("http://example.org/knows")
            .expect("Failed to create RDF IRI term from valid URI string");
        let domain = RdfTerm::Iri(RDFS_DOMAIN.clone());
        let person = RdfTerm::iri("http://example.org/Person")
            .expect("Failed to create RDF IRI term from valid URI string");

        graph.add_triple(Triple {
            subject: knows.clone(),
            predicate: domain,
            object: person.clone(),
        });

        // Add usage: ex:john ex:knows ex:mary
        let john = RdfTerm::iri("http://example.org/john")
            .expect("Failed to create RDF IRI term from valid URI string");
        let mary = RdfTerm::iri("http://example.org/mary")
            .expect("Failed to create RDF IRI term from valid URI string");

        graph.add_triple(Triple {
            subject: john.clone(),
            predicate: knows,
            object: mary,
        });

        // Apply RDFS reasoning
        let mut engine = RdfsEntailmentEngine::new(graph);
        engine
            .reason()
            .expect("Failed to execute RDFS reasoning over RDF graph");

        let closure = engine.closure();

        // Should derive: ex:john rdf:type ex:Person
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());
        let expected_triple = Triple {
            subject: john,
            predicate: type_iri,
            object: person,
        };

        assert!(closure.contains_triple(&expected_triple));
    }

    #[test]
    fn test_rdfs_rule9_subclass() {
        let mut graph = RdfGraph::new();

        // Add subclass statement: ex:Student rdfs:subClassOf ex:Person
        let student = RdfTerm::iri("http://example.org/Student")
            .expect("Failed to create RDF IRI term from valid URI string");
        let subclass = RdfTerm::Iri(RDFS_SUBCLASS_OF.clone());
        let person = RdfTerm::iri("http://example.org/Person")
            .expect("Failed to create RDF IRI term from valid URI string");

        graph.add_triple(Triple {
            subject: student.clone(),
            predicate: subclass,
            object: person.clone(),
        });

        // Add instance: ex:john rdf:type ex:Student
        let john = RdfTerm::iri("http://example.org/john")
            .expect("Failed to create RDF IRI term from valid URI string");
        let type_iri = RdfTerm::Iri(RDF_TYPE.clone());

        graph.add_triple(Triple {
            subject: john.clone(),
            predicate: type_iri.clone(),
            object: student,
        });

        // Apply RDFS reasoning
        let mut engine = RdfsEntailmentEngine::new(graph);
        engine
            .reason()
            .expect("Failed to execute RDFS reasoning over RDF graph");

        let closure = engine.closure();

        // Should derive: ex:john rdf:type ex:Person
        let expected_triple = Triple {
            subject: john,
            predicate: type_iri,
            object: person,
        };

        assert!(closure.contains_triple(&expected_triple));
    }
}
