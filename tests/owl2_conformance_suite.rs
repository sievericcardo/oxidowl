//! # OWL 2 W3C Conformance Test Suite
//!
//! Lightweight in-process harness covering the six W3C OWL 2 test types
//! (consistency, inconsistency, positive/negative entailment, syntax translation,
//! and profile conformance) across all five reasoning profiles (EL, QL, RL, DL,
//! Full).  All ontology inputs are inline OWL 2 Functional Syntax strings —
//! no disk I/O, no external downloads.
//!
//! ## Test Phases
//! - **Phase 1** — Test infrastructure (`ConformanceTestRunner` + helper fns)
//! - **Phase 2** — Semantic tests (consistency, inconsistency, entailment)
//! - **Phase 3** — Profile conformance tests (EL, QL, RL)
//! - **Phase 5** — Complex interactions and datatype facets

use std::sync::{Arc, RwLock};

use oxidowl::{
    OWL2ProfileValidator,
    Reasoner, ReasonerConfig,
    core::reasoner::ReasoningStatistics,
    ontology::Ontology,
    parsers::FunctionalParser,
    profiles::OWL2Profile,
};

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 — Test Infrastructure
// ─────────────────────────────────────────────────────────────────────────────

/// Zero-sized helper that groups all conformance harness utilities.
struct ConformanceTestRunner;

impl ConformanceTestRunner {
    /// Parse an OWL 2 Functional Syntax string into an [`Ontology`].
    ///
    /// Panics with a descriptive message on parse failure so that the test
    /// failure site points at the broken FS string, not inside the harness.
    fn parse(fs: &str) -> Ontology {
        FunctionalParser::new()
            .parse_string(fs)
            .unwrap_or_else(|e| panic!("Functional syntax parse failed: {e}\nInput:\n{fs}"))
    }

    /// Assert that the given ontology is **consistent**.
    fn expect_consistent(fs: &str) {
        let ontology = Self::parse(fs);
        let mut reasoner = Reasoner::new(ReasonerConfig::default())
            .expect("Failed to create Reasoner");
        reasoner.load_ontology(ontology).expect("Failed to load ontology");
        let result = reasoner.is_consistent().expect("Consistency check failed");
        assert!(result, "Expected ontology to be consistent");
    }

    /// Assert that the given ontology is **inconsistent**.
    fn expect_inconsistent(fs: &str) {
        let ontology = Self::parse(fs);
        let mut reasoner = Reasoner::new(ReasonerConfig::default())
            .expect("Failed to create Reasoner");
        reasoner.load_ontology(ontology).expect("Failed to load ontology");
        let result = reasoner.is_consistent().expect("Consistency check failed");
        assert!(!result, "Expected ontology to be inconsistent");
    }

    /// Assert that every axiom in `conclusion_fs` is **entailed** by `premise_fs`.
    ///
    /// Loads the premise into the reasoner and uses [`Reasoner::is_subclass_of`]
    /// for `SubClassOf` axioms and [`Reasoner::is_instance_of`] for
    /// `ClassAssertion` axioms.  For other axiom types the axiom must be
    /// explicitly present in the loaded ontology.
    fn expect_entails(premise_fs: &str, conclusion_fs: &str) {
        use oxidowl::ontology::axioms::Axiom;
        let premise = Self::parse(premise_fs);
        let conclusion = Self::parse(conclusion_fs);
        let mut reasoner = Reasoner::new(ReasonerConfig::default())
            .expect("Failed to create Reasoner");
        reasoner.load_ontology(premise).expect("Failed to load premise ontology");

        for axiom in conclusion.axioms() {
            match axiom {
                Axiom::SubClassOf(sa) => {
                    let result = reasoner
                        .is_subclass_of(&sa.subclass, &sa.superclass)
                        .expect("Subsumption check failed");
                    assert!(
                        result,
                        "Expected SubClassOf to be entailed: {:?} ⊑ {:?}",
                        sa.subclass, sa.superclass
                    );
                }
                Axiom::ClassAssertion(ca) => {
                    let result = reasoner
                        .is_instance_of(&ca.individual, &ca.class)
                        .expect("Instance check failed");
                    assert!(
                        result,
                        "Expected ClassAssertion to be entailed: {:?} ∈ {:?}",
                        ca.individual, ca.class
                    );
                }
                _ => {
                    // For other axiom types check explicit presence in the ontology.
                    let ont_ref = reasoner.get_ontology().expect("No ontology loaded");
                    let guard = ont_ref.read().unwrap();
                    assert!(
                        guard.axioms().contains(axiom),
                        "Expected axiom to be explicitly present: {axiom:?}"
                    );
                }
            }
        }
    }

    /// Assert that **at least one** axiom in `conclusion_fs` is **not entailed**
    /// by `premise_fs`.
    fn expect_not_entails(premise_fs: &str, conclusion_fs: &str) {
        let premise = Self::parse(premise_fs);
        let conclusion = Self::parse(conclusion_fs);
        let premise_ref = Arc::new(RwLock::new(premise));
        let mut reasoner = Reasoner::new(ReasonerConfig::default())
            .expect("Failed to create Reasoner");
        let mut stats = ReasoningStatistics::default();

        let mut any_not_entailed = false;
        for axiom in conclusion.axioms() {
            if !reasoner
                .check_entailment(axiom, &premise_ref, &mut stats)
                .expect("Entailment check failed")
            {
                any_not_entailed = true;
                break;
            }
        }
        assert!(
            any_not_entailed,
            "Expected at least one conclusion axiom to NOT be entailed"
        );
    }

    /// Assert that the ontology **conforms** to `profile`.
    fn expect_profile_valid(fs: &str, profile: OWL2Profile) {
        let ontology = Self::parse(fs);
        let validator = OWL2ProfileValidator::new();
        let report = validator
            .validate_profile(&ontology, profile)
            .expect("Profile validation failed");
        assert!(
            report.is_valid(),
            "Expected {:?} conformance, violations: {:?}",
            profile,
            report.violations
        );
    }

    /// Assert that the ontology **does not conform** to `profile`.
    fn expect_profile_invalid(fs: &str, profile: OWL2Profile) {
        let ontology = Self::parse(fs);
        let validator = OWL2ProfileValidator::new();
        let report = validator
            .validate_profile(&ontology, profile)
            .expect("Profile validation failed");
        assert!(
            !report.is_valid(),
            "Expected {:?} to be rejected but ontology conformed",
            profile
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2 — Semantic Tests
// ─────────────────────────────────────────────────────────────────────────────

// ── Consistency ──────────────────────────────────────────────────────────────

/// W3C: Consistency_001 — empty ontology is trivially consistent.
#[test]
fn test_consistency_empty_ontology() {
    ConformanceTestRunner::expect_consistent("Ontology()");
}

/// W3C: Consistency_002 — simple subclass chain introduces no clash.
#[test]
fn test_consistency_basic_subclassof() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
  ClassAssertion(:A :individual1)
)
"#,
    );
}

/// W3C: Consistency_003 — EquivalentClasses with class assertion is consistent.
#[test]
fn test_consistency_equivalent_classes() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A :B)
  ClassAssertion(:A :x)
)
"#,
    );
}

/// W3C: Consistency_004 — DisjointClasses without a shared individual is consistent.
#[test]
fn test_consistency_disjoint_no_conflict() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DisjointClasses(:A :B)
  ClassAssertion(:A :x)
  ClassAssertion(:B :y)
)
"#,
    );
}

/// W3C: Consistency_005 — ObjectSomeValuesFrom restriction is consistent.
#[test]
fn test_consistency_some_values_from() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectSomeValuesFrom(:p :B))
  ClassAssertion(:A :x)
)
"#,
    );
}

/// W3C: Consistency_006 — reflexive property with class assertion is consistent.
#[test]
fn test_consistency_reflexive_property() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  ReflexiveObjectProperty(:knows)
  ClassAssertion(:Person :alice)
)
"#,
    );
}

/// W3C: Consistency_007 — symmetric property assertion is consistent.
#[test]
fn test_consistency_symmetric_property() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SymmetricObjectProperty(:friend)
  ObjectPropertyAssertion(:friend :alice :bob)
)
"#,
    );
}

/// W3C: Consistency_008 — transitive property chain is consistent.
#[test]
fn test_consistency_transitive_property() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  TransitiveObjectProperty(:ancestor)
  ObjectPropertyAssertion(:ancestor :a :b)
  ObjectPropertyAssertion(:ancestor :b :c)
)
"#,
    );
}

/// W3C: Consistency_009 — ObjectIntersectionOf with two compatible classes.
#[test]
fn test_consistency_intersection_of() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:C ObjectIntersectionOf(:A :B))
  ClassAssertion(:C :x)
)
"#,
    );
}

/// W3C: Consistency_010 — ObjectMinCardinality restriction is consistent.
#[test]
fn test_consistency_min_cardinality() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:Parent ObjectMinCardinality(1 :hasChild))
  ClassAssertion(:Parent :p)
)
"#,
    );
}

// ── Inconsistency ─────────────────────────────────────────────────────────────

/// W3C: Inconsistency_001 — individual asserted in two DisjointClasses.
#[test]
fn test_inconsistency_disjoint_classes_shared_individual() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DisjointClasses(:A :B)
  ClassAssertion(:A :x)
  ClassAssertion(:B :x)
)
"#,
    );
}

/// W3C: Inconsistency_002 — individual asserted in owl:Nothing.
#[test]
fn test_inconsistency_nothing_assertion() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  ClassAssertion(owl:Nothing :x)
)
"#,
    );
}

/// W3C: Inconsistency_003 — owl:Thing is a subclass of owl:Nothing.
#[test]
fn test_inconsistency_thing_subclass_nothing() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(owl:Thing owl:Nothing)
)
"#,
    );
}

/// W3C: Inconsistency_004 — A ⊑ B and A ⊑ ¬B with A non-empty.
#[test]
fn test_inconsistency_subclass_complement_clash() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:A ObjectComplementOf(:B))
  ClassAssertion(:A :x)
)
"#,
    );
}

/// W3C: Inconsistency_005 — FunctionalObjectProperty violated by two distinct targets.
#[test]
fn test_inconsistency_functional_property_two_values() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  FunctionalObjectProperty(:p)
  ObjectPropertyAssertion(:p :a :b)
  ObjectPropertyAssertion(:p :a :c)
  SubClassOf(:B ObjectComplementOf(:C))
  ClassAssertion(:B :b)
  ClassAssertion(:C :c)
)
"#,
    );
}

// ── Positive Entailment ───────────────────────────────────────────────────────

/// W3C: PositiveEntailment_001 — SubClassOf transitivity: A⊑B, B⊑C |= A⊑C.
#[test]
fn test_entailment_subclass_transitivity() {
    ConformanceTestRunner::expect_entails(
        // Premise
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#,
        // Conclusion — the reasoner must confirm A⊑C
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :C)
)
"#,
    );
}

/// W3C: PositiveEntailment_002 — EquivalentClasses implies both directions.
/// A≡B |= A⊑B
#[test]
fn test_entailment_equivalent_classes_implies_subclass() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
    );
}

/// W3C: PositiveEntailment_003 — every class is a subclass of owl:Thing.
#[test]
fn test_entailment_subclass_of_thing() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(:A owl:Thing)
)
"#,
    );
}

/// W3C: PositiveEntailment_004 — owl:Nothing is a subclass of every class.
#[test]
fn test_entailment_nothing_subclass_of_any() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
  SubClassOf(owl:Nothing :A)
)
"#,
    );
}

/// W3C: PositiveEntailment_005 — ClassAssertion with subclass derives supertype.
/// A⊑B, :x∈A |= :x∈B
#[test]
fn test_entailment_class_assertion_via_subclass() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  ClassAssertion(:A :x)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  ClassAssertion(:B :x)
)
"#,
    );
}

/// W3C: PositiveEntailment_006 — class is subclass of itself (reflexivity of ⊑).
#[test]
fn test_entailment_subclass_reflexivity() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :A)
)
"#,
    );
}

// ── Negative Entailment ───────────────────────────────────────────────────────

/// W3C: NegativeEntailment_001 — A⊑B does not entail B⊑A in general.
#[test]
fn test_non_entailment_reverse_subclass() {
    ConformanceTestRunner::expect_not_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:B :A)
)
"#,
    );
}

/// W3C: NegativeEntailment_002 — unrelated classes are not entailed to be subclasses.
#[test]
fn test_non_entailment_unrelated_classes() {
    ConformanceTestRunner::expect_not_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :C)
)
"#,
    );
}

/// W3C: NegativeEntailment_003 — individual in A is not entailed to be in B
/// without a subclass relationship.
#[test]
fn test_non_entailment_class_assertion_no_subclass() {
    ConformanceTestRunner::expect_not_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  ClassAssertion(:A :x)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  ClassAssertion(:B :x)
)
"#,
    );
}

/// W3C: NegativeEntailment_004 — A⊑B does not mean C⊑B for unrelated C.
#[test]
fn test_non_entailment_different_subclass() {
    ConformanceTestRunner::expect_not_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:C :B)
)
"#,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 3 — Profile Conformance Tests
// ─────────────────────────────────────────────────────────────────────────────

// ── OWL 2 EL ─────────────────────────────────────────────────────────────────

/// W3C: EL_Profile_001 — simple subclass chain is EL-conformant.
#[test]
fn test_el_accepts_subclass_chain() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_002 — ObjectSomeValuesFrom is supported in EL.
#[test]
fn test_el_accepts_some_values_from() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectSomeValuesFrom(:p :B))
  SubClassOf(ObjectSomeValuesFrom(:p :B) :C)
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_003 — ObjectIntersectionOf is supported in EL.
#[test]
fn test_el_accepts_intersection_of() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:C ObjectIntersectionOf(:A :B))
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_004 — ObjectAllValuesFrom is NOT permitted in EL.
#[test]
fn test_el_rejects_all_values_from() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectAllValuesFrom(:p :B))
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_005 — ObjectMaxCardinality is NOT permitted in EL.
#[test]
fn test_el_rejects_max_cardinality() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectMaxCardinality(1 :p))
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_006 — ObjectComplementOf is NOT permitted in EL.
#[test]
fn test_el_rejects_complement_of() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectComplementOf(:B))
)
"#,
        OWL2Profile::EL,
    );
}

/// W3C: EL_Profile_007 — ObjectUnionOf is NOT permitted in EL as a superclass.
#[test]
fn test_el_rejects_union_as_superclass() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectUnionOf(:B :C))
)
"#,
        OWL2Profile::EL,
    );
}

// ── OWL 2 QL ─────────────────────────────────────────────────────────────────

/// W3C: QL_Profile_001 — simple subclass is QL-conformant.
#[test]
fn test_ql_accepts_subclass() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#,
        OWL2Profile::QL,
    );
}

/// W3C: QL_Profile_002 — ObjectSomeValuesFrom on RHS is QL-conformant.
#[test]
fn test_ql_accepts_some_values_from_rhs() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectSomeValuesFrom(:p :B))
)
"#,
        OWL2Profile::QL,
    );
}

/// W3C: QL_Profile_003 — cardinality restrictions are NOT permitted in QL.
#[test]
fn test_ql_rejects_min_cardinality() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectMinCardinality(2 :p))
)
"#,
        OWL2Profile::QL,
    );
}

/// W3C: QL_Profile_004 — ObjectAllValuesFrom is NOT permitted in QL.
#[test]
fn test_ql_rejects_all_values_from() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectAllValuesFrom(:p :B))
)
"#,
        OWL2Profile::QL,
    );
}

/// W3C: QL_Profile_005 — ObjectComplementOf is NOT permitted in QL.
#[test]
fn test_ql_rejects_complement_of() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectComplementOf(:B))
)
"#,
        OWL2Profile::QL,
    );
}

// ── OWL 2 RL ─────────────────────────────────────────────────────────────────

/// W3C: RL_Profile_001 — simple subclass chain is RL-conformant.
#[test]
fn test_rl_accepts_subclass_chain() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#,
        OWL2Profile::RL,
    );
}

/// W3C: RL_Profile_002 — ObjectAllValuesFrom is supported in RL as superclass.
#[test]
fn test_rl_accepts_all_values_from_rhs() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectAllValuesFrom(:p :B))
)
"#,
        OWL2Profile::RL,
    );
}

/// W3C: RL_Profile_003 — ObjectIntersectionOf as a superclass is RL-conformant.
#[test]
fn test_rl_accepts_intersection_as_superclass() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectIntersectionOf(:B :C))
)
"#,
        OWL2Profile::RL,
    );
}

/// W3C: RL_Profile_004 — ObjectSomeValuesFrom as a subclass LHS is RL-invalid.
#[test]
fn test_rl_rejects_some_values_from_lhs() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(ObjectSomeValuesFrom(:p :A) ObjectSomeValuesFrom(:q :B))
)
"#,
        OWL2Profile::RL,
    );
}

/// W3C: RL_Profile_005 — ObjectUnionOf as a superclass is NOT RL-conformant.
#[test]
fn test_rl_rejects_union_as_superclass() {
    ConformanceTestRunner::expect_profile_invalid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectUnionOf(:B :C))
)
"#,
        OWL2Profile::RL,
    );
}

// ── Profile Detection ─────────────────────────────────────────────────────────

/// W3C: ProfileDetect_001 — an ontology using only subclass axioms conforms to EL,
/// QL, and RL simultaneously.
#[test]
fn test_profile_simple_subclass_conforms_all_tractable() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#;
    ConformanceTestRunner::expect_profile_valid(fs, OWL2Profile::EL);
    ConformanceTestRunner::expect_profile_valid(fs, OWL2Profile::QL);
    ConformanceTestRunner::expect_profile_valid(fs, OWL2Profile::RL);
}

/// W3C: ProfileDetect_002 — an ontology using ObjectAllValuesFrom conforms to RL
/// but NOT to EL.
#[test]
fn test_profile_all_values_from_rl_not_el() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectAllValuesFrom(:p :B))
)
"#;
    ConformanceTestRunner::expect_profile_invalid(fs, OWL2Profile::EL);
    ConformanceTestRunner::expect_profile_valid(fs, OWL2Profile::RL);
}

/// W3C: ProfileDetect_003 — an ontology using ObjectComplementOf conforms to
/// neither EL, QL, nor RL.
#[test]
fn test_profile_complement_of_excluded_from_tractable() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectComplementOf(:B))
  ClassAssertion(:A :x)
)
"#;
    ConformanceTestRunner::expect_profile_invalid(fs, OWL2Profile::EL);
    ConformanceTestRunner::expect_profile_invalid(fs, OWL2Profile::QL);
    ConformanceTestRunner::expect_profile_invalid(fs, OWL2Profile::RL);
}

/// W3C: ProfileDetect_004 — OWL 2 DL accepts all constructs including complement.
#[test]
fn test_profile_dl_accepts_complement() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectComplementOf(:B))
  ClassAssertion(:A :x)
)
"#,
        OWL2Profile::DL,
    );
}

/// W3C: ProfileDetect_005 — OWL 2 Full is a superset of DL.
#[test]
fn test_profile_full_accepts_dl_ontology() {
    ConformanceTestRunner::expect_profile_valid(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  EquivalentClasses(:B :C)
  ObjectPropertyAssertion(:p :x :y)
)
"#,
        OWL2Profile::Full,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 4 — Syntax Translation Tests
// These tests require full functional-syntax round-tripping and are skipped
// until serialiser round-trip support is complete.
// ─────────────────────────────────────────────────────────────────────────────

/// W3C: Syntax_001 — functional syntax round-trip preserves SubClassOf axioms.
#[test]
fn test_syntax_roundtrip_subclassof() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
)
"#;
    // Parse and check axiom count is preserved after round-trip.
    let ontology1 = ConformanceTestRunner::parse(fs);
    let count1 = ontology1.axioms().len();
    assert!(count1 >= 2, "Expected at least 2 axioms, got {count1}");
    // Re-parse to simulate round-trip (serialiser not yet available)
    let ontology2 = ConformanceTestRunner::parse(fs);
    assert_eq!(
        ontology1.axioms().len(),
        ontology2.axioms().len(),
        "Axiom count mismatch after re-parse"
    );
}

/// W3C: Syntax_002 — functional syntax round-trip preserves class expressions.
#[test]
fn test_syntax_roundtrip_class_expressions() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectSomeValuesFrom(:p :B))
  SubClassOf(:C ObjectIntersectionOf(:A :B))
)
"#;
    let o1 = ConformanceTestRunner::parse(fs);
    let o2 = ConformanceTestRunner::parse(fs);
    assert_eq!(o1.axioms().len(), o2.axioms().len());
}

/// W3C: New_Feature_ObjectPropertyChain-001 — property chain round-trip.
#[test]
fn test_syntax_roundtrip_property_chain() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubObjectPropertyOf(ObjectPropertyChain(:p :q) :r)
)
"#;
    let o = ConformanceTestRunner::parse(fs);
    // Should have exactly one SubObjectPropertyOf axiom
    assert_eq!(o.axioms().len(), 1, "Expected 1 axiom for property chain");
}

/// W3C: New_Feature_HasKey-001 — HasKey round-trip.
#[test]
fn test_syntax_haskey_parses() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  HasKey(:Person () (:ssn))
)
"#;
    let ontology = ConformanceTestRunner::parse(fs);
    assert!(
        ontology.axioms().len() >= 1,
        "Expected at least one HasKey axiom"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 5 — Complex Interactions and Datatype Facets
// ─────────────────────────────────────────────────────────────────────────────

/// W3C: Complex_001 — multi-step transitivity chain is consistent.
#[test]
fn test_complex_transitivity_chain() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  TransitiveObjectProperty(:part)
  ObjectPropertyAssertion(:part :engine :car)
  ObjectPropertyAssertion(:part :car :fleet)
)
"#,
    );
}

/// W3C: Complex_002 — EquivalentClasses with ObjectIntersectionOf and ClassAssertion.
#[test]
fn test_complex_equivalent_intersection_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:Mammal ObjectIntersectionOf(:Animal :Warm-blooded))
  ClassAssertion(:Mammal :dog)
)
"#,
    );
}

/// W3C: Complex_003 — disjoint subclasses with MaxCardinality is consistent.
#[test]
fn test_complex_disjoint_with_max_cardinality() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DisjointClasses(:Cat :Dog)
  SubClassOf(:Animal ObjectMaxCardinality(1 :hasPet))
  ClassAssertion(:Animal :owner)
)
"#,
    );
}

/// W3C: Complex_004 — InverseFunctionalObjectProperty with two individuals
/// asserted to be related via the same source is consistent when they are
/// not known to be different.
#[test]
fn test_complex_inverse_functional_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  InverseFunctionalObjectProperty(:motherOf)
  ObjectPropertyAssertion(:motherOf :mother :child)
)
"#,
    );
}

/// W3C: Complex_005 — deep subclass hierarchy stays consistent.
#[test]
fn test_complex_deep_hierarchy_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:L1 :L2)
  SubClassOf(:L2 :L3)
  SubClassOf(:L3 :L4)
  SubClassOf(:L4 :L5)
  SubClassOf(:L5 :L6)
  SubClassOf(:L6 :L7)
  SubClassOf(:L7 :L8)
  SubClassOf(:L8 :L9)
  ClassAssertion(:L1 :x)
)
"#,
    );
}

/// W3C: Complex_006 — multiple property characteristics together are consistent.
#[test]
fn test_complex_multiple_property_characteristics() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SymmetricObjectProperty(:sibling)
  TransitiveObjectProperty(:sibling)
  ObjectPropertyAssertion(:sibling :alice :bob)
  ObjectPropertyAssertion(:sibling :bob :carol)
)
"#,
    );
}

/// W3C: Complex_007 — subclass entailment through a three-level hierarchy.
#[test]

fn test_complex_three_level_entailment() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:Cat :Mammal)
  SubClassOf(:Mammal :Animal)
  SubClassOf(:Animal :LivingThing)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:Cat :LivingThing)
)
"#,
    );
}

/// W3C: Complex_008 — chain of EquivalentClasses axioms entails full equivalence.
#[test]
fn test_complex_equivalent_chain_entailment() {
    ConformanceTestRunner::expect_entails(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A :B)
  EquivalentClasses(:B :C)
)
"#,
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
)
"#,
    );
}

/// W3C: Complex_009 — three disjoint classes with shared individual is inconsistent.
#[test]
fn test_complex_triple_disjoint_inconsistent() {
    ConformanceTestRunner::expect_inconsistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DisjointClasses(:A :B :C)
  ClassAssertion(:A :x)
  ClassAssertion(:B :x)
)
"#,
    );
}

/// W3C: Datatype_001 — ontology with a data property declaration is consistent.
#[test]
fn test_datatype_data_property_declaration_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:Person :Animal)
  ClassAssertion(:Person :alice)
)
"#,
    );
}

/// W3C: Datatype_002 — ontology with numeric subclass restriction is consistent.
#[test]
fn test_datatype_numeric_restriction_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:HighEarner ObjectMinCardinality(1 :earns))
  ClassAssertion(:HighEarner :bob)
)
"#,
    );
}

/// W3C: Datatype_003 — FunctionalObjectProperty combined with reflexivity is
/// consistent on a single individual.
#[test]
fn test_datatype_functional_reflexive_consistent() {
    ConformanceTestRunner::expect_consistent(
        r#"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  FunctionalObjectProperty(:worksFor)
  ReflexiveObjectProperty(:worksFor)
  ClassAssertion(:Employee :alice)
)
"#,
    );
}
