//! Ontology Walker — depth-first traversal of ontology structure.

pub mod merge;

use crate::ontology::{
    Annotation, ClassExpression, DataPropertyExpression, DataRange, Individual,
    Literal, ObjectPropertyExpression, Ontology,
};
use crate::ontology::axioms::Axiom;

// ── OWLObjectVisitor ─────────────────────────────────────────────────────────

/// Visitor trait for OWL objects encountered during walking.
pub trait OWLObjectVisitor {
    fn visit_axiom(&mut self, _axiom: &Axiom) {}
    fn visit_class_expression(&mut self, _expr: &ClassExpression) {}
    fn visit_data_range(&mut self, _range: &DataRange) {}
    fn visit_individual(&mut self, _ind: &Individual) {}
    fn visit_literal(&mut self, _lit: &Literal) {}
    fn visit_annotation(&mut self, _ann: &Annotation) {}
    fn visit_iri(&mut self, _iri: &crate::ontology::IRI) {}
    fn visit_ope(&mut self, _ope: &ObjectPropertyExpression) {}
    fn visit_dpe(&mut self, _dpe: &DataPropertyExpression) {}
}

// ── OntologyWalker ───────────────────────────────────────────────────────────

/// Depth-first walker that traverses every OWL object in an ontology.
pub struct OntologyWalker<V: OWLObjectVisitor> {
    visitor: V,
}

impl<V: OWLObjectVisitor> OntologyWalker<V> {
    #[must_use]
    pub fn new(visitor: V) -> Self { Self { visitor } }

    pub fn walk_ontology(&mut self, ontology: &Ontology) {
        for axiom in ontology.axioms() { self.walk_axiom(axiom); }
        for ann in &ontology.annotations { self.walk_annotation(ann); }
    }

    pub fn walk_axiom(&mut self, axiom: &Axiom) {
        self.visitor.visit_axiom(axiom);
        match axiom {
            Axiom::SubClassOf(a) => { self.walk_ce(&a.subclass); self.walk_ce(&a.superclass); }
            Axiom::EquivalentClasses(a) => { for c in &a.classes { self.walk_ce(c); } }
            Axiom::DisjointClasses(a) => { for c in &a.classes { self.walk_ce(c); } }
            Axiom::DisjointUnion(a) => { self.walk_ce(&a.class); for c in &a.disjoint_classes { self.walk_ce(c); } }
            Axiom::SubObjectPropertyOf(a) => { self.walk_ope(&a.sub_property); self.walk_ope(&a.super_property); }
            Axiom::EquivalentObjectProperties(a) => { for p in &a.properties { self.walk_ope(p); } }
            Axiom::DisjointObjectProperties(a) => { for p in &a.properties { self.walk_ope(p); } }
            Axiom::InverseObjectProperties(a) => { self.walk_ope(&a.property1); self.walk_ope(&a.property2); }
            Axiom::ObjectPropertyDomain(a) => { self.walk_ope(&a.property); self.walk_ce(&a.domain); }
            Axiom::ObjectPropertyRange(a) => { self.walk_ope(&a.property); self.walk_ce(&a.range); }
            Axiom::FunctionalObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::InverseFunctionalObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::ReflexiveObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::IrreflexiveObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::SymmetricObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::AsymmetricObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::TransitiveObjectProperty(a) => { self.walk_ope(&a.property); }
            Axiom::SubDataPropertyOf(a) => { self.walk_dpe(&a.sub_property); self.walk_dpe(&a.super_property); }
            Axiom::EquivalentDataProperties(a) => { for p in &a.properties { self.walk_dpe(p); } }
            Axiom::DisjointDataProperties(a) => { for p in &a.properties { self.walk_dpe(p); } }
            Axiom::DataPropertyDomain(a) => { self.walk_dpe(&a.property); self.walk_ce(&a.domain); }
            Axiom::DataPropertyRange(a) => { self.walk_dpe(&a.property); self.walk_dr(&a.range); }
            Axiom::FunctionalDataProperty(a) => { self.walk_dpe(&a.property); }
            Axiom::ClassAssertion(a) => { self.walk_ce(&a.class); self.walk_ind(&a.individual); }
            Axiom::ObjectPropertyAssertion(a) => { self.walk_ope(&a.property); self.walk_ind(&a.source); self.walk_ind(&a.target); }
            Axiom::DataPropertyAssertion(a) => { self.walk_dpe(&a.property); self.walk_ind(&a.individual); self.walk_lit(&a.value); }
            Axiom::NegativeObjectPropertyAssertion(a) => { self.walk_ope(&a.property); self.walk_ind(&a.source); self.walk_ind(&a.target); }
            Axiom::NegativeDataPropertyAssertion(a) => { self.walk_dpe(&a.property); self.walk_ind(&a.individual); self.walk_lit(&a.value); }
            Axiom::SameIndividual(a) => { for i in &a.individuals { self.walk_ind(i); } }
            Axiom::DifferentIndividuals(a) => { for i in &a.individuals { self.walk_ind(i); } }
            Axiom::HasKey(a) => { self.walk_ce(&a.class); for p in &a.object_properties { self.walk_ope(p); } for p in &a.data_properties { self.walk_dpe(p); } }
            _ => {}
        }
    }

    fn walk_ce(&mut self, expr: &ClassExpression) {
        self.visitor.visit_class_expression(expr);
        match expr {
            ClassExpression::Class(cls) => { self.visitor.visit_iri(&cls.iri); }
            ClassExpression::ObjectIntersectionOf(ops) | ClassExpression::ObjectUnionOf(ops) => { for op in ops { self.walk_ce(op); } }
            ClassExpression::ObjectComplementOf(inner) => { self.walk_ce(inner); }
            ClassExpression::ObjectSomeValuesFrom { property, filler } | ClassExpression::ObjectAllValuesFrom { property, filler } => { self.walk_ope(property); self.walk_ce(filler); }
            ClassExpression::ObjectHasValue { property, value } => { self.walk_ope(property); self.walk_ind(value); }
            ClassExpression::ObjectHasSelf { property } => { self.walk_ope(property); }
            ClassExpression::ObjectMinCardinality { property, filler, .. } | ClassExpression::ObjectMaxCardinality { property, filler, .. } | ClassExpression::ObjectExactCardinality { property, filler, .. } => { self.walk_ope(property); self.walk_ce(filler); }
            ClassExpression::ObjectOneOf(inds) => { for ind in inds { self.walk_ind(ind); } }
            ClassExpression::DataSomeValuesFrom { property, filler } | ClassExpression::DataAllValuesFrom { property, filler } => { self.walk_dpe(property); self.walk_dr(filler); }
            ClassExpression::DataHasValue { property, value } => { self.walk_dpe(property); self.walk_lit(value); }
            ClassExpression::DataMinCardinality { property, filler, .. } | ClassExpression::DataMaxCardinality { property, filler, .. } | ClassExpression::DataExactCardinality { property, filler, .. } => { self.walk_dpe(property); self.walk_dr(filler); }
        }
    }

    fn walk_ope(&mut self, ope: &ObjectPropertyExpression) {
        self.visitor.visit_ope(ope);
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => { self.visitor.visit_iri(&p.iri); }
            ObjectPropertyExpression::InverseObjectProperty(p) => { self.visitor.visit_iri(&p.iri); }
            ObjectPropertyExpression::PropertyChain(chain) => { for p in chain { self.walk_ope(p); } }
        }
    }

    fn walk_dpe(&mut self, dpe: &DataPropertyExpression) {
        self.visitor.visit_dpe(dpe);
        match dpe {
            DataPropertyExpression::DataProperty(p) => { self.visitor.visit_iri(&p.iri); }
        }
    }

    fn walk_ind(&mut self, ind: &Individual) {
        self.visitor.visit_individual(ind);
        match ind {
            Individual::Named(n) => { self.visitor.visit_iri(&n.iri); }
            Individual::Anonymous(_) => {}
        }
    }

    fn walk_lit(&mut self, lit: &Literal) {
        self.visitor.visit_literal(lit);
    }

    fn walk_dr(&mut self, dr: &DataRange) {
        self.visitor.visit_data_range(dr);
        match dr {
            DataRange::Datatype(iri) => { self.visitor.visit_iri(iri); }
            DataRange::DataIntersectionOf(rs) | DataRange::DataUnionOf(rs) => { for r in rs { self.walk_dr(r); } }
            DataRange::DataComplementOf(r) => { self.walk_dr(r); }
            DataRange::DataOneOf(lits) => { for l in lits { self.walk_lit(l); } }
            DataRange::DatatypeRestriction { datatype, .. } => { self.visitor.visit_iri(datatype); }
        }
    }

    fn walk_annotation(&mut self, ann: &Annotation) {
        self.visitor.visit_annotation(ann);
        self.visitor.visit_iri(&ann.property.iri);
        match &ann.value {
            crate::ontology::AnnotationValue::IRI(iri) => self.visitor.visit_iri(iri),
            crate::ontology::AnnotationValue::Literal(lit) => self.walk_lit(lit),
            crate::ontology::AnnotationValue::AnonymousIndividual(a) => self.visitor.visit_iri(&crate::ontology::IRI::new(&a.id)),
        }
    }

    pub fn into_visitor(self) -> V { self.visitor }
}

// ── StructureWalker ──────────────────────────────────────────────────────────

/// A walker that tracks which axiom is currently being visited.
pub struct StructureWalker<'a, V: OWLObjectVisitor> {
    walker: OntologyWalker<V>,
    current_axiom: Option<&'a Axiom>,
}

impl<'a, V: OWLObjectVisitor> StructureWalker<'a, V> {
    #[must_use]
    pub fn new(visitor: V) -> Self {
        Self { walker: OntologyWalker::new(visitor), current_axiom: None }
    }

    pub fn walk_ontology(&mut self, ontology: &'a Ontology) {
        for axiom in ontology.axioms() {
            self.current_axiom = Some(axiom);
            self.walker.walk_axiom(axiom);
        }
    }

    /// Get the axiom currently being visited.
    #[must_use]
    pub fn get_current_axiom(&self) -> Option<&'a Axiom> {
        self.current_axiom
    }
}
