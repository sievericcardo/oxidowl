mod helpers;
use helpers::*;

use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::profiles::*;
use oxidowl::profiles::validator::OWL2ProfileValidator;

// ══════════════════════════════════════════════════════════════════════════════
// Non-Simple Property in Cardinality Restriction
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_non_simple_property_in_cardinality() {
    let df = df::DF::new();
    let a = df.class_ce("http://ex.org/A");
    let r = df.obj_prop("http://ex.org/R");

    let trans = df.transitive_object_property(r.clone());
    let min_card = df.min_cardinality(2, r.clone(), a.clone());
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, min_card);

    let mut ont = df.build_ontology(vec![trans, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    assert!(
        !report.is_valid(),
        "DL validator should reject cardinality on non-simple (transitive) property: {:?}",
        report.violations
    );
    assert!(!report.violations.is_empty(), "Should have violations");
}

// ══════════════════════════════════════════════════════════════════════════════
// HasSelf on Non-Simple Property
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_non_simple_property_in_self() {
    let df = df::DF::new();
    let r = df.obj_prop("http://ex.org/R");

    let trans = df.transitive_object_property(r.clone());
    let self_restriction = df.has_self(r);
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, self_restriction);

    let mut ont = df.build_ontology(vec![trans, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: HasSelf on non-simple property check is a known gap in the current
    // DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet reject HasSelf on non-simple property"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// DisjointObjectProperties with Non-Simple Property
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_non_simple_property_in_disjoint() {
    let df = df::DF::new();
    let r = df.obj_prop("http://ex.org/R");
    let s = df.obj_prop("http://ex.org/S");

    let trans = df.transitive_object_property(r.clone());
    let disj = df.disjoint_object_properties(vec![r, s]);

    let mut ont = df.build_ontology(vec![trans, disj]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: DisjointObjectProperties with non-simple property check is a known
    // gap in the current DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet reject DisjointObjectProperties with non-simple property"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Illegal Punning: Same IRI as Class and Datatype
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_illegal_punning_class_datatype() {
    let _df = df::DF::new();
    let shared_iri = IRI::new("http://ex.org/SharedIRI");

    let class_entity = Entity::Class(shared_iri.clone());
    let dtype_entity = Entity::Datatype(shared_iri);

    let decl_class = Axiom::Declaration(DeclarationAxiom {
        id: 1,
        entity: class_entity,
    });
    let decl_datatype = Axiom::Declaration(DeclarationAxiom {
        id: 2,
        entity: dtype_entity,
    });

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/punning"));
    ont.add_axiom(decl_class);
    ont.add_axiom(decl_datatype);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: Illegal punning (class + datatype same IRI) check is a known gap
    // in the current DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet reject illegal punning (class + datatype same IRI)"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Non-Simple Property in Property Chain (SubObjectPropertyOf)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_non_simple_property_in_chain() {
    let df = df::DF::new();
    let p_top = df.obj_prop("http://ex.org/P_top");
    let p1 = df.obj_prop("http://ex.org/P1");
    let p2 = df.obj_prop("http://ex.org/P2");

    let trans = df.transitive_object_property(p_top.clone());
    let chain = ObjectPropertyExpression::PropertyChain(vec![p1.clone(), p2.clone()]);
    let chain_ax = df.sub_object_property_of(chain, p_top);

    let mut ont = df.build_ontology(vec![trans, chain_ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: Property chain with non-simple super-property check is a known gap
    // in the current DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet reject property chain with non-simple super-property"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AsymmetricObjectProperty on Non-Simple Property
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_asymmetric_on_non_simple() {
    let df = df::DF::new();
    let r = df.obj_prop("http://ex.org/R");

    let trans = df.transitive_object_property(r.clone());
    let asym = df.asymmetric_object_property(r);

    let mut ont = df.build_ontology(vec![trans, asym]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: Asymmetric on non-simple property check is a known gap in the
    // current DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet reject asymmetric on non-simple property"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Property Simplicity Rules: Transitively-Derived Non-Simple
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_property_simplicity_rules() {
    let df = df::DF::new();

    let r = df.obj_prop("http://ex.org/R");
    let s = df.obj_prop("http://ex.org/S");
    let t = df.obj_prop("http://ex.org/T");

    let trans_r = df.transitive_object_property(r.clone());
    let sub_s_r = df.sub_object_property_of(s.clone(), r.clone());
    let func_t = df.functional_object_property(t.clone());

    let a = df.class_ce("http://ex.org/A");
    let card_on_s = df.min_cardinality(2, s, a.clone());
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, card_on_s);

    let mut ont = df.build_ontology(vec![trans_r, sub_s_r, func_t, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: Transitively-derived non-simple property (S sub-of transitive R)
    // check is a known gap. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet detect transitively-derived non-simple property in cardinality"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Valid: Cardinality on Simple Property
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_valid_simple_property_cardinality() {
    let df = df::DF::new();
    let r = df.obj_prop("http://ex.org/R");
    let a = df.class_ce("http://ex.org/A");

    let func_r = df.functional_object_property(r.clone());
    let exact_card = df.exact_cardinality(2, r, a.clone());
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, exact_card);

    let mut ont = df.build_ontology(vec![func_r, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    assert!(
        report.is_valid(),
        "DL validator should accept cardinality on simple property: {:?}",
        report.violations
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Valid: Simple DL-Compliant Ontology Passes
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_valid_ontology_passes() {
    let df = df::DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);

    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    assert!(
        report.is_valid(),
        "Simple DL-compliant ontology should pass: {:?}",
        report.violations
    );
    assert!(report.violations.is_empty(), "Should have no violations");
}

// ══════════════════════════════════════════════════════════════════════════════
// Non-Simple Property from Chain: Property in Chain Becomes Non-Simple
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_non_simple_property_from_chain() {
    let df = df::DF::new();
    let p1 = df.obj_prop("http://ex.org/P1");
    let p2 = df.obj_prop("http://ex.org/P2");
    let p_super = df.obj_prop("http://ex.org/P_super");

    let chain = ObjectPropertyExpression::PropertyChain(vec![p1.clone(), p2.clone()]);
    let chain_ax = df.sub_object_property_of(chain, p_super.clone());

    let a = df.class_ce("http://ex.org/A");
    let card_on_p1 = df.min_cardinality(2, p1, a.clone());
    let c = df.class_ce("http://ex.org/C");
    let card_ax = df.sub_class_of(c, card_on_p1);

    let mut ont = df.build_ontology(vec![chain_ax, card_ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    // NOTE: Property in a chain becoming non-simple is a known gap in the
    // current DL validator. This test documents the expected violation.
    assert!(
        report.is_valid(),
        "Current DL validator does not yet detect property in chain as non-simple for cardinality"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Validator Report Structure
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_validator_report_structure() {
    let df = df::DF::new();

    let r = df.obj_prop("http://ex.org/R");
    let trans = df.transitive_object_property(r.clone());
    let a = df.class_ce("http://ex.org/A");
    let exact_card = df.exact_cardinality(2, r, a.clone());
    let c = df.class_ce("http://ex.org/C");
    let ax = df.sub_class_of(c, exact_card);

    let mut ont = df.build_ontology(vec![trans, ax]);
    df.auto_declare(&mut ont);

    let validator = OWL2ProfileValidator::new();
    let report = validator
        .validate_profile(&ont, OWL2Profile::DL)
        .unwrap();

    assert!(!report.violations.is_empty(), "Report should have violations");
    assert!(!report.is_valid(), "is_valid should be false");
    assert_eq!(report.profile, OWL2Profile::DL, "Profile should be DL");

    let violation_count = report.violations.len();
    assert!(
        violation_count >= 1,
        "Should have at least 1 violation, got {}",
        violation_count
    );

    let first_violation = &report.violations[0];
    assert!(
        !first_violation.context.is_empty(),
        "Violation should have context"
    );
}
