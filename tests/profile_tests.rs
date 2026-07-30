#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::ELValidator;
use oxidowl::profiles::ql::QLValidator;
use oxidowl::profiles::rl::RLValidator;
use oxidowl::profiles::ProfileValidator;

/// Helper: validate ontology against a specific profile validator.
fn validate_el(axioms: &[Axiom]) -> bool {
    let df = DF::new();
    let onto = df.build_ontology(axioms.to_vec());
    let validator = ELValidator::new();
    validator.validate(&onto).map(|r| r.is_valid()).unwrap_or(false)
}

fn validate_ql(axioms: &[Axiom]) -> bool {
    let df = DF::new();
    let onto = df.build_ontology(axioms.to_vec());
    let validator = QLValidator::new();
    validator.validate(&onto).map(|r| r.is_valid()).unwrap_or(false)
}

fn validate_rl(axioms: &[Axiom]) -> bool {
    let df = DF::new();
    let onto = df.build_ontology(axioms.to_vec());
    let validator = RLValidator::new();
    validator.validate(&onto).map(|r| r.is_valid()).unwrap_or(false)
}

/// Simple ontology that should pass all profiles.
fn el_valid_ontology() -> Vec<Axiom> {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = df.obj_prop("http://ex.org/P");
    let svf = df.some_values_from(p, b);
    vec![df.sub_class_of(a, svf)]
}

fn union_ontology() -> Vec<Axiom> {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let union = df.union_of(vec![b, c]);
    vec![df.sub_class_of(a, union)]
}

fn complement_ontology() -> Vec<Axiom> {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let not_b = df.complement_of(b);
    vec![df.sub_class_of(a, not_b)]
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 EL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn profile_el_valid() {
    assert!(validate_el(&el_valid_ontology()), "EL-valid ontology should pass");
}

#[test]
fn profile_el_union_violation() {
    assert!(!validate_el(&union_ontology()), "Union should not pass EL");
}

#[test]
fn profile_el_complement_violation() {
    assert!(!validate_el(&complement_ontology()), "Complement should not pass EL");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 QL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn profile_ql_simple_subclass_valid() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    assert!(validate_ql(&[df.sub_class_of(a, b)]), "Simple SubClassOf should pass QL");
}

#[test]
fn profile_ql_transitive_property_violation() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    assert!(!validate_ql(&[df.transitive_object_property(p)]),
        "TransitiveObjectProperty should NOT pass QL");
}

#[test]
fn profile_ql_symmetric_property_violation() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    assert!(!validate_ql(&[df.symmetric_object_property(p)]),
        "SymmetricObjectProperty should NOT pass QL");
}

#[test]
fn profile_ql_functional_property_violation() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    assert!(!validate_ql(&[df.functional_object_property(p)]),
        "FunctionalObjectProperty should NOT pass QL");
}

#[test]
fn profile_ql_irreflexive_property_violation() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    assert!(!validate_ql(&[df.irreflexive_object_property(p)]),
        "IrreflexiveObjectProperty should NOT pass QL");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: OWL 2 RL
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn profile_rl_class_assertion_valid() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    assert!(validate_rl(&[df.class_assertion(a, i)]), "Simple class assertion should pass RL");
}

#[test]
fn profile_rl_complement_violation() {
    assert!(!validate_rl(&complement_ontology()), "Complement should NOT pass RL");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: Punning Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn punning_class_and_individual_allowed_in_el() {
    let df = DF::new();
    let iri = IRI::new("http://ex.org/Punned");
    let axioms = vec![
        df.declaration_axiom(Entity::Class(iri.clone())),
        df.declaration_axiom(Entity::NamedIndividual(iri.clone())),
    ];
    // Punning class+individual is allowed in EL
    assert!(validate_el(&axioms));
}

#[test]
fn punning_class_and_datatype_not_allowed_in_el() {
    let df = DF::new();
    let iri = IRI::new("http://ex.org/Punned");
    let axioms = vec![
        df.declaration_axiom(Entity::Class(iri.clone())),
        df.declaration_axiom(Entity::Datatype(iri.clone())),
    ];
    // Punning class+datatype is NOT allowed in EL
    assert!(!validate_el(&axioms));
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: HasKey Violations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn profile_ql_has_key_violation() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");
    let axioms = vec![df.has_key(c, vec![p], vec![dp])];
    assert!(!validate_ql(&axioms), "HasKey should NOT pass QL");
}

#[test]
fn profile_rl_has_key_violation() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");
    let axioms = vec![df.has_key(c, vec![p], vec![dp])];
    assert!(!validate_rl(&axioms), "HasKey should NOT pass RL");
}

// ══════════════════════════════════════════════════════════════════════════════
// Profile: DisjointUnion Violation Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn profile_el_disjoint_union_violation() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let d1 = df.class_ce("http://ex.org/D1");
    let d2 = df.class_ce("http://ex.org/D2");
    let axioms = vec![df.disjoint_union(c, vec![d1, d2])];
    assert!(!validate_el(&axioms), "DisjointUnion should NOT pass EL");
}
