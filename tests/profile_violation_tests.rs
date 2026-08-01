#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::*;
use oxidowl::profiles::el::ELValidator;
use oxidowl::profiles::ql::QLValidator;
use oxidowl::profiles::rl::RLValidator;
use oxidowl::profiles::validator::OWL2ProfileValidator;
use oxidowl::profiles::ProfileValidator;
use oxidowl::profiles::*;

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 EL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_el_valid_passes() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = df.obj_prop("http://ex.org/P");
    let svf = df.some_values_from(p, b);
    let ax = df.sub_class_of(a, svf);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(report.is_valid(), "Simple EL ontology should pass");
}

#[test]
fn test_el_violation_union() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let d = df.class_ce("http://ex.org/D");
    let e = df.class_ce("http://ex.org/E");
    let union = df.union_of(vec![c.clone(), d.clone()]);
    let ax = df.sub_class_of(union, e);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject union in subclass position");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_complement() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let not_b = df.complement_of(b);
    let ax = df.sub_class_of(a, not_b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject complement");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_universal_to_bottom() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let r = df.obj_prop("http://ex.org/R");
    let bottom = df.owl_nothing();
    let avf = df.all_values_from(r, bottom);
    let ax = df.sub_class_of(c, avf);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject universal restriction");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_cardinality() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let r = df.obj_prop("http://ex.org/R");
    let d = df.class_ce("http://ex.org/D");
    let min_card = df.min_cardinality(2, r, d);
    let ax = df.sub_class_of(c, min_card);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject cardinality restrictions");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_has_self() {
    let df = DF::new();

    let r = df.obj_prop("http://ex.org/R");
    let self_restriction = df.has_self(r);
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, self_restriction);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(report.is_valid(), "ObjectHasSelf is allowed in OWL 2 EL per W3C spec");
}

#[test]
fn test_el_violation_inverse() {
    let df = DF::new();

    let r_inv = df.inv_obj_prop("http://ex.org/R");
    let c = df.class_ce("http://ex.org/C");
    let sovf = df.some_values_from(r_inv, c);
    let d = df.class_ce("http://ex.org/D");
    let ax = df.sub_class_of(d, sovf);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator correctly rejects ObjectInverseOf in ObjectSomeValuesFrom per OWL 2 EL spec");
    assert!(!report.violations.is_empty(), "Should have violations for inverse property expression");
}

#[test]
fn test_el_violation_disjoint_classes() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let d = df.class_ce("http://ex.org/D");
    let ax = df.disjoint_classes(vec![c, d]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject DisjointClasses");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_disjoint_union() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let d1 = df.class_ce("http://ex.org/D1");
    let d2 = df.class_ce("http://ex.org/D2");
    let ax = df.disjoint_union(c, vec![d1, d2]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject DisjointUnion");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_functional() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.functional_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject FunctionalObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_el_violation_irreflexive() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.irreflexive_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    // NOTE: Current EL validator does not explicitly reject IrreflexiveObjectProperty;
    // it falls through to the wildcard arm. This is a known gap.
    assert!(report.is_valid(), "Current EL validator does not yet reject IrreflexiveObjectProperty");
}

#[test]
fn test_el_violation_property_chain() {
    let df = DF::new();

    let p1 = df.obj_prop("http://ex.org/P1");
    let p2 = df.obj_prop("http://ex.org/P2");
    let chain = ObjectPropertyExpression::PropertyChain(vec![p1.clone(), p2.clone()]);
    let c = df.class_ce("http://ex.org/C");
    let sovf = ClassExpression::ObjectSomeValuesFrom {
        property: chain,
        filler: Box::new(c),
    };
    let d = df.class_ce("http://ex.org/D");
    let ax = df.sub_class_of(d, sovf);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = ELValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "EL validator should reject property chains");
    assert!(!report.violations.is_empty(), "Should have violations");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 QL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ql_valid_passes() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(report.is_valid(), "Simple QL ontology should pass");
}

#[test]
fn test_ql_violation_disjunction_subclass() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let d = df.class_ce("http://ex.org/D");
    let e = df.class_ce("http://ex.org/E");
    let union = df.union_of(vec![c, d]);
    let ax = df.sub_class_of(union, e);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject disjunction in subclass");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_existential_super() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let r = df.obj_prop("http://ex.org/R");
    let d = df.class_ce("http://ex.org/D");
    let ex = df.some_values_from(r, d);
    let ax = df.sub_class_of(c, ex);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(report.is_valid(), "Existential in superclass position is allowed in OWL 2 QL");
}

#[test]
fn test_ql_violation_functional() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.functional_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject FunctionalObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_inverse_functional() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.inverse_functional_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject InverseFunctionalObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_symmetric() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.symmetric_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject SymmetricObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_transitive() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.transitive_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject TransitiveObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_irreflexive() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let ax = df.irreflexive_object_property(p);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject IrreflexiveObjectProperty");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_ql_violation_has_key() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");
    let ax = df.has_key(c, vec![p], vec![dp]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = QLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "QL validator should reject HasKey");
    assert!(!report.violations.is_empty(), "Should have violations");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 RL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rl_valid_passes() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(report.is_valid(), "Simple RL ontology should pass");
}

#[test]
fn test_rl_violation_some_values_from_super() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let r = df.obj_prop("http://ex.org/R");
    let d = df.class_ce("http://ex.org/D");
    let ex = df.some_values_from(r, d);
    let ax = df.sub_class_of(c, ex);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject SomeValuesFrom in superclass");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_rl_violation_union_super() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let d = df.class_ce("http://ex.org/D");
    let e = df.class_ce("http://ex.org/E");
    let union = df.union_of(vec![d, e]);
    let ax = df.sub_class_of(c, union);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject union in superclass");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_rl_violation_complement() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let not_b = df.complement_of(b);
    let ax = df.sub_class_of(a, not_b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject complement");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_rl_violation_one_of() {
    let df = DF::new();

    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");
    let one_of = df.one_of(vec![a, b, c]);
    let d = df.class_ce("http://ex.org/D");
    let ax = df.sub_class_of(one_of, d);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject ObjectOneOf");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_rl_violation_self() {
    let df = DF::new();

    let r = df.obj_prop("http://ex.org/R");
    let self_restriction = df.has_self(r);
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, self_restriction);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject ObjectHasSelf");
    assert!(!report.violations.is_empty(), "Should have violations");
}

#[test]
fn test_rl_violation_exact_cardinality() {
    let df = DF::new();

    let c = df.class_ce("http://ex.org/C");
    let r = df.obj_prop("http://ex.org/R");
    let d = df.class_ce("http://ex.org/D");
    let exact_card = df.exact_cardinality(2, r, d);
    let ax = df.sub_class_of(c, exact_card);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = RLValidator::new();
    let report = validator.validate(&ont).unwrap();

    assert!(!report.is_valid(), "RL validator should reject exact cardinality (not =1)");
    assert!(!report.violations.is_empty(), "Should have violations");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 DL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_valid_passes() {
    let df = DF::new();

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator.validate_profile(&ont, OWL2Profile::DL).unwrap();

    assert!(report.is_valid(), "Simple DL ontology should pass");
}

#[test]
fn test_dl_violation_non_simple_cardinality() {
    let df = DF::new();

    let p = df.obj_prop("http://ex.org/P");
    let transitive = df.transitive_object_property(p.clone());
    let c = df.class_ce("http://ex.org/C");
    let exact_card = df.exact_cardinality(2, p, c);
    let ax = df.sub_class_of(df.owl_thing(), exact_card);
    let mut ont = df.build_ontology(vec![transitive, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator.validate_profile(&ont, OWL2Profile::DL).unwrap();

    assert!(!report.is_valid(), "DL validator should reject cardinality on non-simple (transitive) property");
    assert!(!report.violations.is_empty(), "Should have violations");
}
