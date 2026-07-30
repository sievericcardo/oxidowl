use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

/// Strip axiom IDs for semantic comparison. Returns a canonical form
/// that ignores ID values but preserves structural content.
fn strip_axiom_id(axiom: &Axiom) -> Axiom {
    let mut ax = axiom.clone();
    match &mut ax {
        Axiom::Declaration(a) => a.id = 0,
        Axiom::SubClassOf(a) => a.id = 0,
        Axiom::EquivalentClasses(a) => a.id = 0,
        Axiom::DisjointClasses(a) => a.id = 0,
        Axiom::DisjointUnion(a) => a.id = 0,
        Axiom::SubObjectPropertyOf(a) => a.id = 0,
        Axiom::EquivalentObjectProperties(a) => a.id = 0,
        Axiom::DisjointObjectProperties(a) => a.id = 0,
        Axiom::InverseObjectProperties(a) => a.id = 0,
        Axiom::ObjectPropertyDomain(a) => a.id = 0,
        Axiom::ObjectPropertyRange(a) => a.id = 0,
        Axiom::FunctionalObjectProperty(a) => a.id = 0,
        Axiom::InverseFunctionalObjectProperty(a) => a.id = 0,
        Axiom::ReflexiveObjectProperty(a) => a.id = 0,
        Axiom::IrreflexiveObjectProperty(a) => a.id = 0,
        Axiom::SymmetricObjectProperty(a) => a.id = 0,
        Axiom::AsymmetricObjectProperty(a) => a.id = 0,
        Axiom::TransitiveObjectProperty(a) => a.id = 0,
        Axiom::SubDataPropertyOf(a) => a.id = 0,
        Axiom::EquivalentDataProperties(a) => a.id = 0,
        Axiom::DisjointDataProperties(a) => a.id = 0,
        Axiom::DataPropertyDomain(a) => a.id = 0,
        Axiom::DataPropertyRange(a) => a.id = 0,
        Axiom::FunctionalDataProperty(a) => a.id = 0,
        Axiom::SameIndividual(a) => a.id = 0,
        Axiom::DifferentIndividuals(a) => a.id = 0,
        Axiom::ClassAssertion(a) => a.id = 0,
        Axiom::ObjectPropertyAssertion(a) => a.id = 0,
        Axiom::DataPropertyAssertion(a) => a.id = 0,
        Axiom::NegativeObjectPropertyAssertion(a) => a.id = 0,
        Axiom::NegativeDataPropertyAssertion(a) => a.id = 0,
        Axiom::AnnotationAssertion(a) => a.id = 0,
        Axiom::SubAnnotationPropertyOf(a) => a.id = 0,
        Axiom::AnnotationPropertyDomain(a) => a.id = 0,
        Axiom::AnnotationPropertyRange(a) => a.id = 0,
        Axiom::Rule(a) => a.id = 0,
        Axiom::HasKey(a) => a.id = 0,
        Axiom::DatatypeDefinition(a) => a.id = 0,
    }
    ax
}

/// Assert that two ontologies contain semantically equivalent axiom sets
/// (comparing structural content while ignoring axiom IDs).
pub fn assert_ontologies_axiom_equal(ont1: &Ontology, ont2: &Ontology) {
    let axioms1: Vec<_> = ont1.axioms().iter().map(strip_axiom_id).collect();
    let axioms2: Vec<_> = ont2.axioms().iter().map(strip_axiom_id).collect();

    let only_in_1: Vec<_> = axioms1
        .iter()
        .filter(|a| !axioms2.contains(a))
        .collect();
    let only_in_2: Vec<_> = axioms2
        .iter()
        .filter(|a| !axioms1.contains(a))
        .collect();

    assert!(
        only_in_1.is_empty() && only_in_2.is_empty(),
        "Ontologies differ:\n  Only in first ({}): {:?}\n  Only in second ({}): {:?}",
        only_in_1.len(),
        only_in_1,
        only_in_2.len(),
        only_in_2
    );
}

/// Assert that two ontologies have the same number of axioms.
pub fn assert_axiom_count_equal(ont1: &Ontology, ont2: &Ontology) {
    let c1 = ont1.axioms().len();
    let c2 = ont2.axioms().len();
    assert_eq!(
        c1, c2,
        "Axiom count mismatch: {c1} != {c2}"
    );
}

/// Assert that ontology `inner` is a subset of ontology `outer`.
pub fn assert_ontology_contains(outer: &Ontology, inner: &Ontology) {
    for ax in inner.axioms() {
        assert!(
            outer.axioms().contains(ax),
            "Ontology missing axiom: {ax:?}"
        );
    }
}

/// Assert that `ontology` contains a specific axiom.
pub fn assert_contains_axiom(ontology: &Ontology, axiom: &Axiom) {
    assert!(
        ontology.axioms().contains(axiom),
        "Expected axiom not found: {axiom:?}\nOntology has: {:?}",
        ontology.axioms()
    );
}

/// Assert that `ontology` does NOT contain a specific axiom.
pub fn assert_not_contains_axiom(ontology: &Ontology, axiom: &Axiom) {
    assert!(
        !ontology.axioms().contains(axiom),
        "Unexpected axiom found: {axiom:?}"
    );
}

/// Assert that two ontologies have the same signature.
pub fn assert_signature_equal(ont1: &Ontology, ont2: &Ontology) {
    let sig1 = ont1.signature().unwrap_or_default();
    let sig2 = ont2.signature().unwrap_or_default();

    assert_eq!(
        sig1.classes.len(),
        sig2.classes.len(),
        "Class count in signature differs"
    );
    assert_eq!(
        sig1.object_properties.len(),
        sig2.object_properties.len(),
        "Object property count differs"
    );
    assert_eq!(
        sig1.data_properties.len(),
        sig2.data_properties.len(),
        "Data property count differs"
    );
    assert_eq!(
        sig1.individuals.len(),
        sig2.individuals.len(),
        "Individual count differs"
    );
}

/// Assert that the IRI is present in an ontology's set of axioms
/// (checking through the axiom list for any axiom referencing the IRI).
pub fn assert_ontology_mentions_iri(ontology: &Ontology, iri: &IRI) {
    let iri_str = iri.as_str();
    let found = ontology.axioms().iter().any(|ax| {
        format!("{ax:?}").contains(iri_str)
    });
    assert!(found, "Ontology does not mention IRI {iri}");
}

/// Assert that a class expression exists in the ontology (as a class
/// or within any axiom).
pub fn assert_contains_class(ontology: &Ontology, class: &Class) {
    let found = ontology.axioms().iter().any(|ax| {
        let debug = format!("{ax:?}");
        debug.contains(class.iri.as_str())
    });
    assert!(
        found,
        "Class {} not found in ontology axioms",
        class.iri.as_str()
    );
}
