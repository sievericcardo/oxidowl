//! Oxigraph-backed SPARQL store for ABox classification and reasoning
//!
//! This module provides an in-process Oxigraph store that mirrors the ontology
//! as standard OWL/RDF triples, enabling full SPARQL 1.1 query and update
//! support including the SPARQL-based ABox classification rules.

#![cfg(feature = "sparql-store")]

use crate::{
    Error, Result,
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
        Ontology,
    },
    profiles::rl_reasoner::MaterializedKnowledgeBase,
};
use oxigraph::{
    model::{BlankNode, GraphName, Literal as OxLiteral, NamedNode, NamedOrBlankNode, Quad, Term},
    sparql::{QueryResults, SparqlEvaluator},
    store::Store,
};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

// ─── RDF / OWL vocabulary constants ─────────────────────────────────────────
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const RDFS_SUBPROP_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
const OWL_EQUIV_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const OWL_ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
const OWL_HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
const OWL_INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const OWL_ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";
const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const OWL_INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_TRANSITIVE: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
const OWL_SYMMETRIC: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
const OWL_REFLEXIVE: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";
const OWL_FUNCTIONAL: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
const OWL_INVERSE_FUNCTIONAL: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
#[allow(unused)]
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// Blank node ID counter for unique generation
static BLANK_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn fresh_blank() -> BlankNode {
    let id = BLANK_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    BlankNode::new(format!("b{id}")).expect("valid blank node id")
}

fn named(iri: &str) -> NamedNode {
    NamedNode::new(iri).unwrap_or_else(|_| {
        NamedNode::new("http://www.w3.org/2002/07/owl#Thing").expect("fallback IRI")
    })
}

fn default_quad(s: impl Into<NamedOrBlankNode>, p: NamedNode, o: impl Into<Term>) -> Quad {
    Quad::new(s.into(), p, o.into(), GraphName::DefaultGraph)
}

// ─── OxigraphStore ──────────────────────────────────────────────────────────

/// In-process Oxigraph store providing full SPARQL 1.1 over ontology + materialized facts
pub struct OxigraphStore {
    store: Store,
}

impl std::fmt::Debug for OxigraphStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OxigraphStore").finish_non_exhaustive()
    }
}

impl OxigraphStore {
    /// Create a new empty store
    pub fn new() -> Result<Self> {
        let store = Store::new().map_err(|e| Error::reasoning(format!("Oxigraph init: {e}")))?;
        Ok(Self { store })
    }

    /// Load all ontology axioms as RDF triples into the store
    pub fn load_from_ontology(&self, ontology: &Ontology) -> Result<()> {
        for axiom in ontology.axioms() {
            self.load_axiom(axiom)?;
        }
        Ok(())
    }

    /// Load materialized ABox facts into the store
    pub fn load_materialized_facts(&self, kb: &MaterializedKnowledgeBase) -> Result<()> {
        let rdf_type = named(RDF_TYPE);
        // Class assertions
        for (individual, classes) in kb.class_assertion_iter() {
            if let Some(ind_iri) = individual_iri(individual) {
                let ind_node = named(&ind_iri);
                for class in classes {
                    let class_term = rl_class_expr_to_term(class, &self.store)?;
                    if is_named_term(&class_term) {
                        self.insert(default_quad(ind_node.clone(), rdf_type.clone(), class_term))?;
                    }
                }
            }
        }
        // Object property assertions
        for ((subject, property), objects) in kb.object_property_assertion_iter() {
            if let (Some(subj_iri), Some(prop_iri)) =
                (individual_iri(subject), obj_prop_iri(property))
            {
                let subj_node = named(&subj_iri);
                let prop_node = named(&prop_iri);
                for obj in objects {
                    if let Some(obj_iri) = individual_iri(obj) {
                        self.insert(default_quad(subj_node.clone(), prop_node.clone(), named(&obj_iri)))?;
                    }
                }
            }
        }
        // Same-as assertions
        for (left, rights) in kb.same_as_iter() {
            if let Some(left_iri) = individual_iri(left) {
                let left_node = named(&left_iri);
                let same_as = named(OWL_SAME_AS);
                for right in rights {
                    if let Some(right_iri) = individual_iri(right) {
                        self.insert(default_quad(left_node.clone(), same_as.clone(), named(&right_iri)))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Execute a SPARQL SELECT/ASK/CONSTRUCT/DESCRIBE query, returning JSON results
    pub fn execute_query(&self, sparql: &str) -> Result<String> {
        let results = SparqlEvaluator::new()
            .parse_query(sparql)
            .map_err(|e| Error::Sparql { message: format!("SPARQL parse error: {e}") })?
            .on_store(&self.store)
            .execute()
            .map_err(|e| Error::Sparql { message: format!("SPARQL execute error: {e}") })?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut bindings: Vec<serde_json::Value> = Vec::new();
                let mut vars: Vec<String> = Vec::new();
                for sol in solutions {
                    let sol = sol
                        .map_err(|e| Error::Sparql { message: e.to_string() })?;
                    if vars.is_empty() {
                        vars = sol.iter().map(|(v, _)| v.as_str().to_string()).collect();
                    }
                    let mut binding = serde_json::Map::new();
                    for (var, term) in sol.iter() {
                        binding.insert(var.as_str().to_string(), term_to_json(term));
                    }
                    bindings.push(serde_json::Value::Object(binding));
                }
                Ok(serde_json::json!({
                    "head": { "vars": vars },
                    "results": { "bindings": bindings }
                })
                .to_string())
            }
            QueryResults::Boolean(b) => Ok(serde_json::json!({
                "head": {},
                "boolean": b
            })
            .to_string()),
            QueryResults::Graph(graph) => {
                let mut triples = Vec::new();
                for triple in graph {
                    let t = triple.map_err(|e| Error::Sparql { message: e.to_string() })?;
                    triples.push(serde_json::json!({
                        "subject": term_to_json(&t.subject.into()),
                        "predicate": term_to_json(&t.predicate.into()),
                        "object": term_to_json(&t.object),
                    }));
                }
                Ok(serde_json::json!({
                    "head": {},
                    "results": { "bindings": triples }
                })
                .to_string())
            }
        }
    }

    /// Execute a SPARQL UPDATE (INSERT WHERE / DELETE WHERE), returns count of new triples (estimated)
    pub fn execute_update(&self, sparql: &str) -> Result<usize> {
        let before = self.store.len().unwrap_or(0);
        self.store
            .update(sparql)
            .map_err(|e| Error::Sparql { message: format!("SPARQL update error: {e}") })?;
        let after = self.store.len().unwrap_or(0);
        Ok(after.saturating_sub(before))
    }

    /// Insert a single quad into the store
    pub fn insert(&self, quad: Quad) -> Result<()> {
        self.store
            .insert(&quad)
            .map_err(|e| Error::Sparql { message: format!("Store insert error: {e}") })
    }

    /// Number of quads in the store
    pub fn len(&self) -> usize {
        self.store.len().unwrap_or(0)
    }

    /// Return true if the store is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ─── Private helpers ────────────────────────────────────────────────────

    fn load_axiom(&self, axiom: &Axiom) -> Result<()> {
        use crate::ontology::axioms::*;
        match axiom {
            // ABox
            Axiom::ClassAssertion(ax) => self.load_class_assertion(ax),
            Axiom::ObjectPropertyAssertion(ax) => self.load_obj_prop_assertion(ax),
            Axiom::DataPropertyAssertion(ax) => self.load_data_prop_assertion(ax),
            Axiom::SameIndividual(ax) => self.load_same_individual(ax),

            // TBox – class axioms
            Axiom::SubClassOf(ax) => self.load_subclass_of(ax),
            Axiom::EquivalentClasses(ax) => self.load_equivalent_classes(ax),
            Axiom::DisjointClasses(ax) => self.load_disjoint_classes(ax),

            // TBox – property axioms
            Axiom::ObjectPropertyDomain(ax) => self.load_obj_domain(ax),
            Axiom::ObjectPropertyRange(ax) => self.load_obj_range(ax),
            Axiom::SubObjectPropertyOf(ax) => self.load_sub_obj_prop(ax),
            Axiom::InverseObjectProperties(ax) => self.load_inverse_props(ax),
            Axiom::TransitiveObjectProperty(ax) => self.load_transitive(ax),
            Axiom::SymmetricObjectProperty(ax) => self.load_symmetric(ax),
            Axiom::ReflexiveObjectProperty(ax) => self.load_reflexive(ax),
            Axiom::FunctionalObjectProperty(ax) => self.load_functional(ax),
            Axiom::InverseFunctionalObjectProperty(ax) => self.load_inverse_functional(ax),

            _ => Ok(()), // other axiom types not yet needed
        }
    }

    fn load_class_assertion(&self, ax: &crate::ontology::axioms::ClassAssertionAxiom) -> Result<()> {
        if let Some(ind_iri) = individual_iri(&ax.individual) {
            let ind = named(&ind_iri);
            let class_term = self.class_expr_to_term(&ax.class)?;
            self.insert(default_quad(ind, named(RDF_TYPE), class_term))
        } else {
            Ok(())
        }
    }

    fn load_obj_prop_assertion(
        &self,
        ax: &crate::ontology::axioms::ObjectPropertyAssertionAxiom,
    ) -> Result<()> {
        if let (Some(src), Some(tgt), Some(prop)) = (
            individual_iri(&ax.source),
            individual_iri(&ax.target),
            obj_prop_iri_from_expr(&ax.property),
        ) {
            self.insert(default_quad(named(&src), named(&prop), named(&tgt)))
        } else {
            Ok(())
        }
    }

    fn load_data_prop_assertion(
        &self,
        ax: &crate::ontology::axioms::DataPropertyAssertionAxiom,
    ) -> Result<()> {
        use crate::ontology::DataPropertyExpression;
        if let Some(ind_iri) = individual_iri(&ax.individual) {
            let DataPropertyExpression::DataProperty(dp) = &ax.property;
            let prop = named(dp.iri.as_str());
            let lit = literal_from_oxidowl(&ax.value);
            self.insert(default_quad(named(&ind_iri), prop, lit))
        } else {
            Ok(())
        }
    }

    fn load_same_individual(&self, ax: &crate::ontology::axioms::SameIndividualAxiom) -> Result<()> {
        let same_as = named(OWL_SAME_AS);
        let iris: Vec<_> = ax.individuals.iter().filter_map(individual_iri).collect();
        for i in 0..iris.len() {
            for j in 0..iris.len() {
                if i != j {
                    self.insert(default_quad(named(&iris[i]), same_as.clone(), named(&iris[j])))?;
                }
            }
        }
        Ok(())
    }

    fn load_subclass_of(&self, ax: &crate::ontology::axioms::SubClassOfAxiom) -> Result<()> {
        let sub = self.class_expr_to_term(&ax.subclass)?;
        let sup = self.class_expr_to_term(&ax.superclass)?;
        if let Some(sub_node) = term_to_named_or_blank(&sub) {
            self.insert(default_quad(sub_node, named(RDFS_SUBCLASS_OF), sup))
        } else {
            Ok(())
        }
    }

    fn load_equivalent_classes(
        &self,
        ax: &crate::ontology::axioms::EquivalentClassesAxiom,
    ) -> Result<()> {
        let terms: Vec<Term> = ax
            .classes
            .iter()
            .map(|c| self.class_expr_to_term(c))
            .collect::<Result<_>>()?;
        let equiv = named(OWL_EQUIV_CLASS);
        for i in 0..terms.len() {
            for j in 0..terms.len() {
                if i != j {
                    if let Some(left) = term_to_named_or_blank(&terms[i]) {
                        self.insert(default_quad(left, equiv.clone(), terms[j].clone()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn load_disjoint_classes(
        &self,
        ax: &crate::ontology::axioms::DisjointClassesAxiom,
    ) -> Result<()> {
        let disjoint = named("http://www.w3.org/2002/07/owl#disjointWith");
        let terms: Vec<Term> = ax
            .classes
            .iter()
            .map(|c| self.class_expr_to_term(c))
            .collect::<Result<_>>()?;
        for i in 0..terms.len() {
            for j in (i + 1)..terms.len() {
                if let Some(left) = term_to_named_or_blank(&terms[i]) {
                    self.insert(default_quad(left, disjoint.clone(), terms[j].clone()))?;
                }
            }
        }
        Ok(())
    }

    fn load_obj_domain(&self, ax: &crate::ontology::axioms::ObjectPropertyDomainAxiom) -> Result<()> {
        if let Some(prop_iri) = obj_prop_iri_from_expr(&ax.property) {
            let domain_term = self.class_expr_to_term(&ax.domain)?;
            self.insert(default_quad(named(&prop_iri), named(RDFS_DOMAIN), domain_term))
        } else {
            Ok(())
        }
    }

    fn load_obj_range(&self, ax: &crate::ontology::axioms::ObjectPropertyRangeAxiom) -> Result<()> {
        if let Some(prop_iri) = obj_prop_iri_from_expr(&ax.property) {
            let range_term = self.class_expr_to_term(&ax.range)?;
            self.insert(default_quad(named(&prop_iri), named(RDFS_RANGE), range_term))
        } else {
            Ok(())
        }
    }

    fn load_sub_obj_prop(&self, ax: &crate::ontology::axioms::SubObjectPropertyOfAxiom) -> Result<()> {
        if let (Some(sub_iri), Some(sup_iri)) = (
            obj_prop_iri_from_expr(&ax.sub_property),
            obj_prop_iri_from_expr(&ax.super_property),
        ) {
            self.insert(default_quad(named(&sub_iri), named(RDFS_SUBPROP_OF), named(&sup_iri)))
        } else {
            Ok(())
        }
    }

    fn load_inverse_props(
        &self,
        ax: &crate::ontology::axioms::InverseObjectPropertiesAxiom,
    ) -> Result<()> {
        if let (Some(p1), Some(p2)) = (
            obj_prop_iri_from_expr(&ax.property1),
            obj_prop_iri_from_expr(&ax.property2),
        ) {
            let inv = named(OWL_INVERSE_OF);
            self.insert(default_quad(named(&p1), inv.clone(), named(&p2)))?;
            self.insert(default_quad(named(&p2), inv, named(&p1)))
        } else {
            Ok(())
        }
    }

    fn load_transitive(&self, ax: &crate::ontology::axioms::TransitiveObjectPropertyAxiom) -> Result<()> {
        if let Some(iri) = obj_prop_iri_from_expr(&ax.property) {
            self.insert(default_quad(named(&iri), named(RDF_TYPE), named(OWL_TRANSITIVE)))
        } else {
            Ok(())
        }
    }

    fn load_symmetric(&self, ax: &crate::ontology::axioms::SymmetricObjectPropertyAxiom) -> Result<()> {
        if let Some(iri) = obj_prop_iri_from_expr(&ax.property) {
            self.insert(default_quad(named(&iri), named(RDF_TYPE), named(OWL_SYMMETRIC)))
        } else {
            Ok(())
        }
    }

    fn load_reflexive(&self, ax: &crate::ontology::axioms::ReflexiveObjectPropertyAxiom) -> Result<()> {
        if let Some(iri) = obj_prop_iri_from_expr(&ax.property) {
            self.insert(default_quad(named(&iri), named(RDF_TYPE), named(OWL_REFLEXIVE)))
        } else {
            Ok(())
        }
    }

    fn load_functional(&self, ax: &crate::ontology::axioms::FunctionalObjectPropertyAxiom) -> Result<()> {
        if let Some(iri) = obj_prop_iri_from_expr(&ax.property) {
            self.insert(default_quad(named(&iri), named(RDF_TYPE), named(OWL_FUNCTIONAL)))
        } else {
            Ok(())
        }
    }

    fn load_inverse_functional(
        &self,
        ax: &crate::ontology::axioms::InverseFunctionalObjectPropertyAxiom,
    ) -> Result<()> {
        if let Some(iri) = obj_prop_iri_from_expr(&ax.property) {
            self.insert(default_quad(named(&iri), named(RDF_TYPE), named(OWL_INVERSE_FUNCTIONAL)))
        } else {
            Ok(())
        }
    }

    /// Convert a ClassExpression to an Oxigraph Term, inserting blank-node triples as needed
    fn class_expr_to_term(&self, expr: &ClassExpression) -> Result<Term> {
        match expr {
            ClassExpression::Class(c) => Ok(Term::NamedNode(named(c.iri.as_str()))),

            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let bn = fresh_blank();
                self.insert(default_quad(bn.clone(), named(RDF_TYPE), named(OWL_RESTRICTION)))?;
                if let Some(prop_iri) = obj_prop_iri_from_expr(property) {
                    self.insert(default_quad(bn.clone(), named(OWL_ON_PROPERTY), named(&prop_iri)))?;
                }
                let filler_term = self.class_expr_to_term(filler)?;
                self.insert(default_quad(bn.clone(), named(OWL_SOME_VALUES_FROM), filler_term))?;
                Ok(Term::BlankNode(bn))
            }

            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let bn = fresh_blank();
                self.insert(default_quad(bn.clone(), named(RDF_TYPE), named(OWL_RESTRICTION)))?;
                if let Some(prop_iri) = obj_prop_iri_from_expr(property) {
                    self.insert(default_quad(bn.clone(), named(OWL_ON_PROPERTY), named(&prop_iri)))?;
                }
                let filler_term = self.class_expr_to_term(filler)?;
                self.insert(default_quad(bn.clone(), named(OWL_ALL_VALUES_FROM), filler_term))?;
                Ok(Term::BlankNode(bn))
            }

            ClassExpression::ObjectHasValue { property, value } => {
                let bn = fresh_blank();
                self.insert(default_quad(bn.clone(), named(RDF_TYPE), named(OWL_RESTRICTION)))?;
                if let Some(prop_iri) = obj_prop_iri_from_expr(property) {
                    self.insert(default_quad(bn.clone(), named(OWL_ON_PROPERTY), named(&prop_iri)))?;
                }
                if let Some(val_iri) = individual_iri(value) {
                    self.insert(default_quad(bn.clone(), named(OWL_HAS_VALUE), named(&val_iri)))?;
                }
                Ok(Term::BlankNode(bn))
            }

            ClassExpression::ObjectIntersectionOf(exprs) => {
                let bn = fresh_blank();
                self.insert(default_quad(bn.clone(), named(RDF_TYPE), named(OWL_CLASS)))?;
                let list_head = self.build_rdf_list_class(exprs)?;
                self.insert(default_quad(bn.clone(), named(OWL_INTERSECTION_OF), list_head))?;
                Ok(Term::BlankNode(bn))
            }

            ClassExpression::ObjectOneOf(individuals) => {
                let bn = fresh_blank();
                self.insert(default_quad(bn.clone(), named(RDF_TYPE), named(OWL_CLASS)))?;
                let list_head = self.build_rdf_list_individuals(individuals)?;
                self.insert(default_quad(bn.clone(), named(OWL_ONE_OF), list_head))?;
                Ok(Term::BlankNode(bn))
            }

            _ => {
                // For other complex expressions, use a fresh blank node as placeholder
                Ok(Term::BlankNode(fresh_blank()))
            }
        }
    }

    /// Build an RDF list of class expressions, returning the head Term
    fn build_rdf_list_class(&self, exprs: &[ClassExpression]) -> Result<Term> {
        if exprs.is_empty() {
            return Ok(Term::NamedNode(named(RDF_NIL)));
        }
        let head = fresh_blank();
        let head_term = Term::BlankNode(head.clone());
        let first_term = self.class_expr_to_term(&exprs[0])?;
        self.insert(default_quad(head.clone(), named(RDF_FIRST), first_term))?;
        let rest = self.build_rdf_list_class(&exprs[1..])?;
        self.insert(default_quad(head, named(RDF_REST), rest))?;
        Ok(head_term)
    }

    /// Build an RDF list of individuals, returning the head Term
    fn build_rdf_list_individuals(&self, individuals: &[Individual]) -> Result<Term> {
        if individuals.is_empty() {
            return Ok(Term::NamedNode(named(RDF_NIL)));
        }
        let head = fresh_blank();
        let head_term = Term::BlankNode(head.clone());
        if let Some(ind_iri) = individual_iri(&individuals[0]) {
            self.insert(default_quad(head.clone(), named(RDF_FIRST), named(&ind_iri)))?;
        }
        let rest = self.build_rdf_list_individuals(&individuals[1..])?;
        self.insert(default_quad(head, named(RDF_REST), rest))?;
        Ok(head_term)
    }
}

// ─── Free-function helpers ───────────────────────────────────────────────────

/// Convert a materialized RLClassExpression to a Term (for loading into store)
fn rl_class_expr_to_term(
    class: &crate::profiles::rl_reasoner::RLClassExpression,
    _store: &Store,
) -> Result<Term> {
    use crate::profiles::rl_reasoner::RLClassExpression;
    match class {
        RLClassExpression::Class(c) => Ok(Term::NamedNode(named(c.iri.as_str()))),
        // Other variants converted to blank nodes (simplified for materialized facts)
        _ => Ok(Term::BlankNode(fresh_blank())),
    }
}

fn is_named_term(t: &Term) -> bool {
    matches!(t, Term::NamedNode(_))
}

fn term_to_named_or_blank(t: &Term) -> Option<NamedOrBlankNode> {
    match t {
        Term::NamedNode(n) => Some(NamedOrBlankNode::NamedNode(n.clone())),
        Term::BlankNode(b) => Some(NamedOrBlankNode::BlankNode(b.clone())),
        Term::Literal(_) => None,
    }
}

/// Extract IRI string from Individual (named only)
pub fn individual_iri(ind: &Individual) -> Option<String> {
    match ind {
        Individual::Named(named) => Some(named.iri.to_string()),
        Individual::Anonymous(_) => None,
    }
}

/// Extract IRI string from ObjectPropertyExpression (simple property only)
pub fn obj_prop_iri_from_expr(expr: &ObjectPropertyExpression) -> Option<String> {
    match expr {
        ObjectPropertyExpression::ObjectProperty(p) => Some(p.iri.to_string()),
        ObjectPropertyExpression::InverseObjectProperty(p) => {
            // Inverse: still return the property IRI (caller handles the flip)
            Some(p.iri.to_string())
        }
        ObjectPropertyExpression::PropertyChain(_) => None, // chains don't have a single IRI
    }
}

/// Extract IRI string from DataPropertyExpression
pub fn data_prop_iri(expr: &DataPropertyExpression) -> Option<String> {
    match expr {
        DataPropertyExpression::DataProperty(p) => Some(p.iri.to_string()),
    }
}

/// Like obj_prop_iri_from_expr but only for simple ObjectProperty (not inverse)
pub fn obj_prop_iri(expr: &ObjectPropertyExpression) -> Option<String> {
    if let ObjectPropertyExpression::ObjectProperty(p) = expr {
        Some(p.iri.to_string())
    } else {
        None
    }
}

/// Convert an oxidowl Literal to an Oxigraph Literal
fn literal_from_oxidowl(lit: &crate::ontology::Literal) -> Term {
    let value = &lit.value;
    let datatype_iri = if value.parse::<i64>().is_ok() {
        XSD_INTEGER
    } else if value.parse::<f64>().is_ok() {
        XSD_DOUBLE
    } else if value == "true" || value == "false" {
        XSD_BOOLEAN
    } else {
        XSD_STRING
    };
    let dt = named(datatype_iri);
    Term::Literal(OxLiteral::new_typed_literal(value, dt))
}

/// Convert an Oxigraph Term to JSON for SPARQL results
fn term_to_json(term: &Term) -> serde_json::Value {
    match term {
        Term::NamedNode(n) => serde_json::json!({
            "type": "uri",
            "value": n.as_str()
        }),
        Term::BlankNode(b) => serde_json::json!({
            "type": "bnode",
            "value": b.as_str()
        }),
        Term::Literal(l) => {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), serde_json::json!("literal"));
            obj.insert("value".to_string(), serde_json::json!(l.value()));
            if let Some(lang) = l.language() {
                obj.insert("xml:lang".to_string(), serde_json::json!(lang));
            } else {
                obj.insert("datatype".to_string(), serde_json::json!(l.datatype().as_str()));
            }
            serde_json::Value::Object(obj)
        }
    }
}
