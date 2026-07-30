#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// Object Property Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn object_property_creation() {
    let prop = ObjectProperty { iri: IRI::new("http://ex.org/P") };
    assert_eq!(prop.iri.as_str(), "http://ex.org/P");
}

#[test]
fn object_property_expression() {
    let prop = ObjectProperty { iri: IRI::new("http://ex.org/P") };
    let ope = ObjectPropertyExpression::ObjectProperty(prop.clone());
    assert!(matches!(ope, ObjectPropertyExpression::ObjectProperty(_)));
    assert_eq!(ope.iri().unwrap().as_str(), "http://ex.org/P");
}

#[test]
fn inverse_object_property() {
    let prop = ObjectProperty { iri: IRI::new("http://ex.org/P") };
    let inv = ObjectPropertyExpression::InverseObjectProperty(prop.clone());
    assert!(inv.is_inverse());
    assert_eq!(inv.get_named_property().iri.as_str(), "http://ex.org/P");
}

#[test]
fn property_chain() {
    let p = ObjectProperty { iri: IRI::new("http://ex.org/P") };
    let q = ObjectProperty { iri: IRI::new("http://ex.org/Q") };
    let chain = ObjectPropertyExpression::PropertyChain(vec![
        ObjectPropertyExpression::ObjectProperty(p),
        ObjectPropertyExpression::ObjectProperty(q),
    ]);
    assert!(chain.is_property_chain());
    assert_eq!(chain.chain_length(), 2);
}

#[test]
fn object_property_simple_check() {
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    assert!(p.is_simple_property());
    let inv = ObjectPropertyExpression::InverseObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    assert!(inv.is_inverse());
}

// ── Object Property Axioms ────────────────────────────────────────────────

#[test]
fn sub_object_property_of() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.sub_object_property_of(p, q);
    match &ax {
        Axiom::SubObjectPropertyOf(a) => {
            assert!(!a.sub_property.is_inverse());
            assert!(!a.super_property.is_inverse());
        }
        _ => panic!("Expected SubObjectPropertyOf"),
    }
}

#[test]
fn equivalent_object_properties() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.equivalent_object_properties(vec![p, q]);
    match &ax {
        Axiom::EquivalentObjectProperties(a) => assert_eq!(a.properties.len(), 2),
        _ => panic!("Expected EquivalentObjectProperties"),
    }
}

#[test]
fn disjoint_object_properties() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.disjoint_object_properties(vec![p, q]);
    match &ax {
        Axiom::DisjointObjectProperties(a) => assert_eq!(a.properties.len(), 2),
        _ => panic!("Expected DisjointObjectProperties"),
    }
}

#[test]
fn all_object_property_characteristics() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    assert!(matches!(df.functional_object_property(p.clone()), Axiom::FunctionalObjectProperty(_)));
    assert!(matches!(df.transitive_object_property(p.clone()), Axiom::TransitiveObjectProperty(_)));
    assert!(matches!(df.symmetric_object_property(p.clone()), Axiom::SymmetricObjectProperty(_)));
    assert!(matches!(df.asymmetric_object_property(p.clone()), Axiom::AsymmetricObjectProperty(_)));
    assert!(matches!(df.reflexive_object_property(p.clone()), Axiom::ReflexiveObjectProperty(_)));
    assert!(matches!(df.irreflexive_object_property(p.clone()), Axiom::IrreflexiveObjectProperty(_)));
    assert!(matches!(df.inverse_functional_object_property(p), Axiom::InverseFunctionalObjectProperty(_)));
}

#[test]
fn object_property_domain_and_range() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let c = df.class_ce("http://ex.org/C");
    let domain = df.object_property_domain(p.clone(), c.clone());
    let range = df.object_property_range(p, c);
    assert!(matches!(domain, Axiom::ObjectPropertyDomain(_)));
    assert!(matches!(range, Axiom::ObjectPropertyRange(_)));
}

// ══════════════════════════════════════════════════════════════════════════════
// Data Property Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn data_property_creation() {
    let dp = DataProperty { iri: IRI::new("http://ex.org/dp") };
    assert_eq!(dp.iri.as_str(), "http://ex.org/dp");
}

#[test]
fn data_property_expression() {
    let dp = DataProperty { iri: IRI::new("http://ex.org/dp") };
    let dpe = DataPropertyExpression::DataProperty(dp);
    assert!(matches!(dpe, DataPropertyExpression::DataProperty(_)));
}

#[test]
fn sub_data_property_of() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    let ax = df.sub_data_property_of(dp1, dp2);
    assert!(matches!(ax, Axiom::SubDataPropertyOf(_)));
}

#[test]
fn equivalent_data_properties() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    let ax = df.equivalent_data_properties(vec![dp1, dp2]);
    match &ax {
        Axiom::EquivalentDataProperties(a) => assert_eq!(a.properties.len(), 2),
        _ => panic!("Expected EquivalentDataProperties"),
    }
}

#[test]
fn disjoint_data_properties() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    let ax = df.disjoint_data_properties(vec![dp1, dp2]);
    match &ax {
        Axiom::DisjointDataProperties(a) => assert_eq!(a.properties.len(), 2),
        _ => panic!("Expected DisjointDataProperties"),
    }
}

#[test]
fn functional_data_property() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let ax = df.functional_data_property(dp);
    assert!(matches!(ax, Axiom::FunctionalDataProperty(_)));
}

#[test]
fn data_property_domain_and_range() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let c = df.class_ce("http://ex.org/C");
    let domain = df.data_property_domain(dp.clone(), c);
    let range = df.data_property_range(dp, df.data_range_integer());
    assert!(matches!(domain, Axiom::DataPropertyDomain(_)));
    assert!(matches!(range, Axiom::DataPropertyRange(_)));
}
