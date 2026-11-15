//! Phase A regression tests — single-tableau instance queries, timeout
//! propagation and per-individual instance checks.
//!
//! Complements `tableau_blocking_tests.rs` (blocking rewrite) with the
//! reasoner-level guarantees introduced by the Phase A fix:
//!
//! 1. `get_instances` (one shared tableau snapshot) answers exactly like
//!    per-individual `is_instance_of` calls, including derived
//!    `∃R.C` memberships;
//! 2. a tiny `reasoning.timeout` surfaces as `Err`, never as a silently
//!    wrong boolean answer;
//! 3. `check_instance` distinguishes individuals on the same TBox
//!    (previously `build_for_instance_check` ignored the individual, so
//!    every individual got the same answer).

use oxidowl::{
    Result,
    cache::CacheManager,
    config::{ReasonerConfig, ReasoningConfig},
    core::reasoner::{
        Reasoner, statistics::ReasoningStatistics, tableau::TableauFactory,
        tasks::ReasoningTaskService,
    },
    ontology::{
        Axiom, Class, ClassAssertionAxiom, ClassExpression, IRI, Individual, NamedIndividual,
        ObjectProperty, ObjectPropertyAssertionAxiom, ObjectPropertyExpression, Ontology,
        OntologyRef, SubClassOfAxiom,
    },
};
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn cls(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn some(role_iri: &str, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectSomeValuesFrom {
        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(role_iri),
        }),
        filler: Box::new(filler),
    }
}

fn named(individual_iri: &str) -> Individual {
    Individual::Named(NamedIndividual {
        iri: IRI::new(individual_iri),
    })
}

fn assert_class(individual_iri: &str, class: ClassExpression) -> Axiom {
    Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 1,
        class,
        individual: named(individual_iri),
        annotations: vec![],
    })
}

fn subclass(sub: ClassExpression, sup: ClassExpression) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom {
        id: 2,
        subclass: sub,
        superclass: sup,
        annotations: vec![],
    })
}

fn ontology(axioms: Vec<Axiom>) -> Ontology {
    let mut o = Ontology::new();
    for axiom in axioms {
        o.add_axiom(axiom);
    }
    o
}

fn obj_prop_assert(source: &str, property_iri: &str, target: &str) -> Axiom {
    Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
        id: 3,
        source: named(source),
        target: named(target),
        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(property_iri),
        }),
        annotations: vec![],
    })
}

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(RwLock::new(o))
}

/// `A ⊑ ∃R.C`, `a : A`, `b : C`, `x : B` (unrelated).
///
/// `a ∈ ∃R.C` is *derived* — no `R` edge is asserted; the tableau must
/// materialise the successor and label it `C`.
fn derived_existential_ontology() -> Ontology {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    ontology(vec![
        subclass(a.clone(), some("http://example.org/R", c.clone())),
        assert_class("http://example.org/a", a),
        assert_class("http://example.org/b", c),
        assert_class("http://example.org/x", b),
    ])
}

/// All individuals the parity loop covers — including one that is declared
/// nowhere in the ontology (must never appear as an instance).
fn all_individuals() -> Vec<Individual> {
    vec![
        named("http://example.org/a"),
        named("http://example.org/b"),
        named("http://example.org/x"),
        named("http://example.org/ghost"),
    ]
}

fn iris_of(instances: &[Individual]) -> Vec<String> {
    let mut iris: Vec<String> = instances
        .iter()
        .filter_map(|i| i.iri().map(|i| i.to_string()))
        .collect();
    iris.sort();
    iris.dedup();
    iris
}

// ──────────────────────────────────────────────────────────────────────────────
// (ii) get_instances ≡ per-individual is_instance_of
// ──────────────────────────────────────────────────────────────────────────────

/// `get_instances` (one shared snapshot) must return exactly the individuals
/// for which `is_instance_of` is true — for named classes and for the derived
/// `∃R.C` expression alike.
#[test]
fn get_instances_matches_is_instance_of_including_derived_existential() -> Result<()> {
    let queries = [
        cls("http://example.org/A"),
        cls("http://example.org/B"),
        cls("http://example.org/C"),
        some("http://example.org/R", cls("http://example.org/C")),
    ];

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(derived_existential_ontology())?;

    for query in &queries {
        let instances = reasoner.get_instances(query, false)?;
        let got = iris_of(&instances);

        // Every individual's `is_instance_of` answer must agree with the
        // `get_instances` result.
        for individual in all_individuals() {
            let individually = reasoner.is_instance_of(&individual, query)?;
            let in_list = individual
                .iri()
                .is_some_and(|iri| got.iter().any(|g| g == iri.as_str()));
            assert_eq!(
                individually, in_list,
                "query {query:?}: is_instance_of({individual:?}) = {individually} \
                 but get_instances = {got:?}"
            );
        }

        // Sorted + deduplicated.
        let mut sorted = got.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            got, sorted,
            "get_instances must return sorted, deduped IRIs"
        );
    }

    // Concrete expectations for the derived cases.
    assert_eq!(
        iris_of(&reasoner.get_instances(
            &some("http://example.org/R", cls("http://example.org/C")),
            false
        )?),
        vec!["http://example.org/a".to_string()],
        "a ∈ ∃R.C is derived from A(a) + A ⊑ ∃R.C (no asserted R edge)"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&cls("http://example.org/C"), false)?),
        vec!["http://example.org/b".to_string()],
        "C instances: b asserted; a is NOT in C (A ⊑ ∃R.C only gives a an R-successor)"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&cls("http://example.org/B"), false)?),
        vec!["http://example.org/x".to_string()]
    );

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (iii) tiny timeout → Err, not a false answer
// ──────────────────────────────────────────────────────────────────────────────

/// A `reasoning.timeout` so small that the tableau cannot finish must surface
/// as `Err("Tableau returned unknown result")` from consistency *and* instance
/// queries — never as `Ok(false)` ("inconsistent") or a wrong instance list.
#[test]
fn tiny_timeout_returns_err_not_false_answer() -> Result<()> {
    let a = cls("http://example.org/A");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", a.clone())),
        assert_class("http://example.org/a", a),
    ]);

    let mut reasoning = ReasoningConfig::default();
    reasoning.timeout = Some(Duration::from_nanos(1));
    let config = ReasonerConfig {
        reasoning,
        ..ReasonerConfig::default()
    };

    let mut reasoner = Reasoner::new(config)?;
    reasoner.load_ontology(onto)?;

    // Consistency: an interrupted tableau must not answer `Ok(false)`.
    let consistency = reasoner.is_consistent();
    assert!(
        consistency.is_err(),
        "is_consistent() with a tiny timeout must return Err, got {consistency:?}"
    );

    // Instance query: same policy (the snapshot run times out → Err).
    let query = cls("http://example.org/Unrelated");
    let instances = reasoner.get_instances(&query, false);
    assert!(
        instances.is_err(),
        "get_instances() with a tiny timeout must return Err, got {instances:?}"
    );

    // Direct instance check: same policy.
    let check = reasoner.is_instance_of(&named("http://example.org/a"), &query);
    assert!(
        check.is_err(),
        "is_instance_of() with a tiny timeout must return Err, got {check:?}"
    );

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (iv) instance check distinguishes individuals on the same TBox
// ──────────────────────────────────────────────────────────────────────────────

/// `A(a)`, `B(b)`, `A ⊑ C` ⇒ `check_instance(a, C) = true` and
/// `check_instance(b, C) = false` on the *same* TBox.
///
/// Before the Phase A fix `TableauBuilder::build_for_instance_check` ignored
/// its `individual` argument (it added `not(C)` to a fresh root witness), so
/// both individuals received the same answer.
#[test]
fn instance_check_distinguishes_individuals_on_same_tbox() -> Result<()> {
    let a_class = cls("http://example.org/A");
    let b_class = cls("http://example.org/B");
    let c_class = cls("http://example.org/C");
    let onto = ontology(vec![
        subclass(a_class.clone(), c_class.clone()),
        assert_class("http://example.org/a", a_class.clone()),
        assert_class("http://example.org/b", b_class),
    ]);
    let ref_onto = onto_ref(onto.clone());

    // 1. Task-service level — the path `ClassificationService::get_instances`
    //    and `check_entailment_semantic` still use.
    let task_service = ReasoningTaskService::new(TableauFactory::new(ReasonerConfig::default())?);
    let mut statistics = ReasoningStatistics::new();
    let mut cache = CacheManager::default();

    let a = named("http://example.org/a");
    let b = named("http://example.org/b");

    assert!(
        task_service.check_instance(&a, &c_class, &ref_onto, &mut statistics, &mut cache)?,
        "check_instance(a, C) must be true: A(a) and A ⊑ C"
    );
    assert!(
        !task_service.check_instance(&b, &c_class, &ref_onto, &mut statistics, &mut cache)?,
        "check_instance(b, C) must be false: B(b) and B has no path to C"
    );

    // 2. Reasoner level — the public surface.
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;
    assert!(
        reasoner.is_instance_of(&a, &c_class)?,
        "is_instance_of(a, C) must be true"
    );
    assert!(
        !reasoner.is_instance_of(&b, &c_class)?,
        "is_instance_of(b, C) must be false"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c_class, false)?),
        vec!["http://example.org/a".to_string()],
        "get_instances(C) must contain exactly a"
    );

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (v) upward inference from a *defined* subclass
// ──────────────────────────────────────────────────────────────────────────────

/// Tree-like TBox: `D ≡ (age some int.<40) ⊓ (familyName hasValue "Q")`,
/// `D ⊑ C`, `age(i,10)`, `familyName(i,"Q")` ⇒ `get_instances(C)` contains `i`
/// even though `i` carries no `ClassAssertion` — membership follows from
/// *evaluating the defined subclass's complex definition*, which the named-class
/// branch of `check_instance_with_datatype_reasoning` must do while walking the
/// subclass hierarchy (asserted types / subclass closure / `C`'s own definition
/// are all blind to it).
///
/// Negative control: `j` has `age 50` (fails `int.<40`) and must NOT be in `C`.
#[test]
fn defined_subclass_membership_is_inferred_for_named_superclass() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/defsub>
  EquivalentClasses(:D ObjectIntersectionOf(
    DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:maxExclusive "40"^^xsd:integer))
    DataHasValue(:familyName "Q"^^xsd:string)))
  SubClassOf(:D :C)
  DataPropertyAssertion(:age :i "10"^^xsd:integer)
  DataPropertyAssertion(:familyName :i "Q"^^xsd:string)
  DataPropertyAssertion(:age :j "50"^^xsd:integer)
  DataPropertyAssertion(:familyName :j "Q"^^xsd:string)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    let c = cls("http://example.org/C");
    let i = named("http://example.org/i");
    let j = named("http://example.org/j");

    // Positive: i satisfies D's complex definition, D ⊑ C ⇒ i ∈ C.
    assert!(
        reasoner.is_instance_of(&i, &c)?,
        "is_instance_of(i, C) must be true: i satisfies D's definition and D ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/i".to_string()],
        "get_instances(C) must contain exactly i (defined-subclass upward inference)"
    );

    // Negative control: j fails age<40 ⇒ not in D ⇒ not in C.
    assert!(
        !reasoner.is_instance_of(&j, &c)?,
        "is_instance_of(j, C) must be false: j's age 50 fails D's int.<40 restriction"
    );
    assert!(
        iris_of(&reasoner.get_instances(&c, false)?)
            .iter()
            .all(|x| x != "http://example.org/j"),
        "get_instances(C) must not contain j"
    );

    // The defined class itself stays reachable, and the superclass closure
    // surfaces C through get_types (defined candidate + get_superclasses).
    let d = cls("http://example.org/D");
    assert!(
        reasoner.is_instance_of(&i, &d)?,
        "is_instance_of(i, D) must be true (D's own complex definition)"
    );
    let types: Vec<String> = reasoner
        .get_types(&i, false)?
        .iter()
        .filter_map(|t| t.iri().map(|i| i.to_string()))
        .collect();
    assert!(
        types.iter().any(|t| t == "http://example.org/D"),
        "get_types(i) must contain D, got {types:?}"
    );
    assert!(
        types.iter().any(|t| t == "http://example.org/C"),
        "get_types(i) must contain C via the superclass closure, got {types:?}"
    );

    Ok(())
}

/// Negative control for [`defined_subclass_membership_is_inferred_for_named_superclass`]
/// in isolation: a fresh reasoner over the same TBox where only `j` (age 50) is
/// asserted must return an empty `get_instances(C)` — the defined-subclass
/// evaluation must not fire on a failing definition.
#[test]
fn defined_subclass_membership_negative_only() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/defsubneg>
  EquivalentClasses(:D ObjectIntersectionOf(
    DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:maxExclusive "40"^^xsd:integer))
    DataHasValue(:familyName "Q"^^xsd:string)))
  SubClassOf(:D :C)
  DataPropertyAssertion(:age :j "50"^^xsd:integer)
  DataPropertyAssertion(:familyName :j "Q"^^xsd:string)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    assert_eq!(
        iris_of(&reasoner.get_instances(&cls("http://example.org/C"), false)?),
        Vec::<String>::new(),
        "no individual satisfies D's definition ⇒ get_instances(C) must be empty"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (vi) complex-GCI instance derivation — `SubClassOf(complex, named)`
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊓ B ⊑ C`, `A(x)`, `B(x)` ⇒ `x ∈ C`.
///
/// The named-class branch of `check_instance_with_datatype_reasoning` must
/// evaluate the *complex* subclass expressions headed by the classes it walks
/// through (here `C` itself): asserted types / named-subclass closure / `C`'s
/// own equivalent definition are all blind to `A ⊓ B ⊑ C`.
///
/// Negative controls: `p` (only `A`) and `q` (only `B`) are NOT in `C`.
#[test]
fn complex_gci_intersection_membership_is_inferred() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology(vec![
        subclass(
            ClassExpression::ObjectIntersectionOf(vec![a.clone(), b.clone()]),
            c.clone(),
        ),
        assert_class("http://example.org/x", a.clone()),
        assert_class("http://example.org/x", b.clone()),
        // Negative controls: each carries only one conjunct.
        assert_class("http://example.org/p", a.clone()),
        assert_class("http://example.org/q", b.clone()),
    ]))?;

    let x = named("http://example.org/x");
    assert!(
        reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true: A(x), B(x) and A ⊓ B ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x"
    );

    assert!(
        !reasoner.is_instance_of(&named("http://example.org/p"), &c)?,
        "is_instance_of(p, C) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/q"), &c)?,
        "is_instance_of(q, C) must be false: B(q) alone does not satisfy A ⊓ B"
    );
    Ok(())
}

/// `∃r.D ⊑ C`, `r(x,y)`, `D(y)` ⇒ `x ∈ C`.
///
/// Negative control: `z` with `r(z,w)` where `w` is only an `E` is NOT in `C`.
#[test]
fn complex_gci_existential_membership_is_inferred() -> Result<()> {
    let c = cls("http://example.org/C");
    let d = cls("http://example.org/D");
    let e = cls("http://example.org/E");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology(vec![
        subclass(some("http://example.org/r", d.clone()), c.clone()),
        obj_prop_assert(
            "http://example.org/x",
            "http://example.org/r",
            "http://example.org/y",
        ),
        assert_class("http://example.org/y", d),
        // Negative control: z's r-successor is not a D.
        obj_prop_assert(
            "http://example.org/z",
            "http://example.org/r",
            "http://example.org/w",
        ),
        assert_class("http://example.org/w", e),
    ]))?;

    let x = named("http://example.org/x");
    assert!(
        reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true: r(x,y), D(y) and ∃r.D ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/z"), &c)?,
        "is_instance_of(z, C) must be false: w is an E, not a D"
    );
    Ok(())
}

/// Transitive named superclass: `A ⊓ B ⊑ D`, `D ⊑ C`, `A(x)`, `B(x)` ⇒
/// `x ∈ C`. The BFS from `C` reaches `D`, whose complex subclass `A ⊓ B` is
/// then evaluated for `x`.
///
/// Negative control: `p` (only `A`) is NOT in `C` (nor in `D`).
#[test]
fn complex_gci_membership_propagates_through_named_subclass_chain() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    let d = cls("http://example.org/D");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology(vec![
        subclass(
            ClassExpression::ObjectIntersectionOf(vec![a.clone(), b.clone()]),
            d.clone(),
        ),
        subclass(d.clone(), c.clone()),
        assert_class("http://example.org/x", a.clone()),
        assert_class("http://example.org/x", b.clone()),
        assert_class("http://example.org/p", a),
    ]))?;

    let x = named("http://example.org/x");
    assert!(
        reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true: A(x), B(x), A ⊓ B ⊑ D, D ⊑ C"
    );
    assert!(
        reasoner.is_instance_of(&x, &d)?,
        "is_instance_of(x, D) must be true: A(x), B(x) and A ⊓ B ⊑ D"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/p"), &c)?,
        "is_instance_of(p, C) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/p"), &d)?,
        "is_instance_of(p, D) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    Ok(())
}

/// Data restriction GCI: `(age some int.<18) ⊑ Minor`, `age(i,10)` ⇒
/// `i ∈ Minor`; `age(j,20)` ⇒ `j ∉ Minor`.
///
/// The complex subclass is a `DataSomeValuesFrom` — the named-class branch must
/// evaluate it while walking down from `Minor` (asserted types / named-subclass
/// closure / `Minor`'s own definition are all blind to it).
#[test]
fn complex_gci_data_restriction_membership_is_inferred() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/minor>
  SubClassOf(
    DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:maxExclusive "18"^^xsd:integer))
    :Minor)
  DataPropertyAssertion(:age :i "10"^^xsd:integer)
  DataPropertyAssertion(:age :j "20"^^xsd:integer)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    let minor = cls("http://example.org/Minor");
    let i = named("http://example.org/i");
    let j = named("http://example.org/j");

    assert!(
        reasoner.is_instance_of(&i, &minor)?,
        "is_instance_of(i, Minor) must be true: age(i,10) satisfies age some int.<18"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&minor, false)?),
        vec!["http://example.org/i".to_string()],
        "get_instances(Minor) must contain exactly i"
    );
    assert!(
        !reasoner.is_instance_of(&j, &minor)?,
        "is_instance_of(j, Minor) must be false: age 20 fails int.<18"
    );
    Ok(())
}

/// Negative control: a *necessary* condition is not sufficient. `C ⊑ A ⊓ B`
/// together with `A(x)` and `B(x)` must NOT put `x` in `C` — only a complex
/// *subclass* (`A ⊓ B ⊑ C`) licenses the upward inference.
#[test]
fn necessary_condition_complex_superclass_is_not_inferred() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology(vec![
        subclass(
            c.clone(),
            ClassExpression::ObjectIntersectionOf(vec![a.clone(), b.clone()]),
        ),
        assert_class("http://example.org/x", a.clone()),
        assert_class("http://example.org/x", b.clone()),
    ]))?;

    let x = named("http://example.org/x");
    assert!(
        !reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be false: C ⊑ A ⊓ B is a necessary, not a sufficient, condition"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        Vec::<String>::new(),
        "get_instances(C) must be empty"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (vii) named subclass reachable only through an intersection superclass
// ──────────────────────────────────────────────────────────────────────────────

/// `D ⊑ E ⊓ F`, `E ⊑ C`, `A ⊓ B ⊑ D`, `A(x)`, `B(x)` ⇒ `x ∈ C`.
///
/// The named-subclass BFS from `C` reaches `E` via `E ⊑ C`, but `D` is only
/// a subclass of the *intersection* `E ⊓ F`: the index pass must decompose
/// `SubClassOf(D, ObjectIntersectionOf(E, F))` into the named→named edges
/// `D ⊑ E` / `D ⊑ F`, otherwise `D` is unreachable downward from `C` and
/// the complex GCI `A ⊓ B ⊑ D` headed by it (Strategy 6) never fires.
/// Red-before: without the decomposition this fails.
///
/// Negative controls: `p` (only `A`) and `q` (only `B`) are NOT in `C`.
/// Positive variant "F instead of E": `F ⊑ C` in place of `E ⊑ C` must give
/// the same answer through the `F` conjunct — sound (`x ∈ D ⊑ F ⊑ C`) and
/// also red-before, since it needs the `D ⊑ F` edge.
#[test]
fn named_subclass_through_intersection_superclass_is_reachable() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    let d = cls("http://example.org/D");
    let e = cls("http://example.org/E");
    let f = cls("http://example.org/F");
    let intersection_ef = ClassExpression::ObjectIntersectionOf(vec![e.clone(), f.clone()]);
    let intersection_ab = ClassExpression::ObjectIntersectionOf(vec![a.clone(), b.clone()]);

    // x satisfies A ⊓ B; p/q are the negative controls (one conjunct each).
    let assertions = || {
        vec![
            assert_class("http://example.org/x", a.clone()),
            assert_class("http://example.org/x", b.clone()),
            assert_class("http://example.org/p", a.clone()),
            assert_class("http://example.org/q", b.clone()),
        ]
    };

    let x = named("http://example.org/x");
    let p = named("http://example.org/p");
    let q = named("http://example.org/q");

    // Positive: the chain to C runs through the E conjunct.
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology({
        let mut axioms = vec![
            subclass(d.clone(), intersection_ef.clone()),
            subclass(e.clone(), c.clone()),
            subclass(intersection_ab.clone(), d.clone()),
        ];
        axioms.extend(assertions());
        axioms
    }))?;

    assert!(
        reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true: A(x), B(x), A ⊓ B ⊑ D, D ⊑ E ⊓ F, E ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x"
    );
    assert!(
        !reasoner.is_instance_of(&p, &c)?,
        "is_instance_of(p, C) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    assert!(
        !reasoner.is_instance_of(&q, &c)?,
        "is_instance_of(q, C) must be false: B(q) alone does not satisfy A ⊓ B"
    );

    // "F instead of E": same TBox with `F ⊑ C` in place of `E ⊑ C` — the
    // inference must run through the F conjunct (x ∈ D ⊑ F ⊑ C).
    let mut reasoner_f = Reasoner::new(ReasonerConfig::default())?;
    reasoner_f.load_ontology(ontology({
        let mut axioms = vec![
            subclass(d.clone(), intersection_ef),
            subclass(f.clone(), c.clone()),
            subclass(intersection_ab, d.clone()),
        ];
        axioms.extend(assertions());
        axioms
    }))?;

    assert!(
        reasoner_f.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true via the F conjunct: A(x), B(x), \
         A ⊓ B ⊑ D, D ⊑ E ⊓ F, F ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner_f.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x (F-conjunct path)"
    );
    assert!(
        !reasoner_f.is_instance_of(&p, &c)?,
        "is_instance_of(p, C) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    assert!(
        !reasoner_f.is_instance_of(&q, &c)?,
        "is_instance_of(q, C) must be false: B(q) alone does not satisfy A ⊓ B"
    );

    Ok(())
}

/// `D ≡ E ⊓ F`, `E ⊑ C`, `A ⊓ B ⊑ D`, `A(x)`, `B(x)` ⇒ `x ∈ C`.
///
/// The `D ⊑ E ⊓ F` half of the equivalence must decompose into the
/// named→named edge `D ⊑ E`, so the BFS from `C` reaches `D` and fires its
/// complex GCI (Strategy 6). Red-before: without the decomposition `D` is
/// unreachable downward from `C`.
///
/// Negative controls: `p` (only `A`) / `q` (only `B`) fail `A ⊓ B`; and a
/// second TBox with `D ⊑ C` guards the *soundness* of the decomposition —
/// the converse half `E ⊓ F ⊑ D` must NOT create conjunct→`D` edges, so
/// `r` (only `E`) and `s` (only `F`) are NOT in `C`.
#[test]
fn equivalent_classes_intersection_conjunct_edges_are_indexed() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/eqint>
  EquivalentClasses(:D ObjectIntersectionOf(:E :F))
  SubClassOf(:E :C)
  SubClassOf(ObjectIntersectionOf(:A :B) :D)
  ClassAssertion(:A :x)
  ClassAssertion(:B :x)
  ClassAssertion(:A :p)
  ClassAssertion(:B :q)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    let c = cls("http://example.org/C");
    let x = named("http://example.org/x");
    assert!(
        reasoner.is_instance_of(&x, &c)?,
        "is_instance_of(x, C) must be true: A(x), B(x), A ⊓ B ⊑ D, D ≡ E ⊓ F, E ⊑ C"
    );
    assert_eq!(
        iris_of(&reasoner.get_instances(&c, false)?),
        vec!["http://example.org/x".to_string()],
        "get_instances(C) must contain exactly x"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/p"), &c)?,
        "is_instance_of(p, C) must be false: A(p) alone does not satisfy A ⊓ B"
    );
    assert!(
        !reasoner.is_instance_of(&named("http://example.org/q"), &c)?,
        "is_instance_of(q, C) must be false: B(q) alone does not satisfy A ⊓ B"
    );

    // Soundness guard for the equivalence decomposition: `D ⊑ C` puts the
    // BFS at `D`, but `E ⊓ F ⊑ D` does NOT entail `E ⊑ D` / `F ⊑ D` — an
    // individual asserted in E (or F) alone must not be inferred into C.
    let fs_rev = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/eqintrev>
  EquivalentClasses(:D ObjectIntersectionOf(:E :F))
  SubClassOf(:D :C)
  ClassAssertion(:E :r)
  ClassAssertion(:F :s)
)"#;
    let onto_rev = parse_functional(fs_rev).expect("ontology must parse");

    let mut reasoner_rev = Reasoner::new(ReasonerConfig::default())?;
    reasoner_rev.load_ontology(onto_rev)?;

    let c_rev = cls("http://example.org/C");
    assert!(
        !reasoner_rev.is_instance_of(&named("http://example.org/r"), &c_rev)?,
        "is_instance_of(r, C) must be false: E(r) alone does not entail E⊓F(r), \
         so r ∉ D and D ⊑ C does not apply"
    );
    assert!(
        !reasoner_rev.is_instance_of(&named("http://example.org/s"), &c_rev)?,
        "is_instance_of(s, C) must be false: F(s) alone does not entail E⊓F(s), \
         so s ∉ D and D ⊑ C does not apply"
    );

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// (viii) cyclic / self-referential TBox termination — general re-entrancy guard
// ──────────────────────────────────────────────────────────────────────────────
//
// `ClassificationService::check_instance_with_datatype_reasoning` recurses
// through `check_complex_expression` (intersection/union operands, existencial
// fillers) and back into itself via Strategies 3/5/6.  The pre-existing
// `DEFINED_EVAL_STACK` guard is keyed only by `(individual, named class IRI)`
// and only *inside* the Strategy helpers, so it does not stop recursion that
// alternates through different class expressions / fillers, re-enters the same
// query state from a sibling Strategy-5/6 path after the helper guard has
// already popped, or blows up exponentially through nested defined-subclass
// evaluation.  The general entry guard `(individual, ClassExpression)` plus the
// hard depth cap bound all of these; the conservative `Ok(false)` on a cut
// leaves the caller's tableau snapshot fallback authoritative.

/// Cyclic definition cycle `A ≡ ∃r.B`, `B ≡ ∃r.A` with `r(a,b)`, `r(b,a)`.
///
/// The structural checker must terminate quickly (<1 s) instead of recursing
/// forever through the A ⇄ B definitions (the mutual recursion through
/// `check_complex_expression` fillers is exactly the tosemServices hang
/// shape).  The cycle underdetermines membership — the model with `A = B = ∅`
/// satisfies every axiom — so the entailed answer is "not an instance" and the
/// tableau snapshot fallback agrees.
#[test]
fn cyclic_equivalent_existential_definitions_terminate_quickly() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/cyc>
  EquivalentClasses(:A ObjectSomeValuesFrom(:r :B))
  EquivalentClasses(:B ObjectSomeValuesFrom(:r :A))
  ObjectPropertyAssertion(:r :a :b)
  ObjectPropertyAssertion(:r :b :a)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let start = std::time::Instant::now();
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    let a_class = cls("http://example.org/A");
    let b_class = cls("http://example.org/B");
    let a_ind = named("http://example.org/a");
    let b_ind = named("http://example.org/b");

    let instances_a = reasoner.get_instances(&a_class, false)?;
    let instances_b = reasoner.get_instances(&b_class, false)?;
    assert!(
        !reasoner.is_instance_of(&a_ind, &a_class)?,
        "a ∉ A: the A ≡ ∃r.B / B ≡ ∃r.A cycle underdetermines membership (not entailed)"
    );
    assert!(
        !reasoner.is_instance_of(&b_ind, &b_class)?,
        "b ∉ B: the A ≡ ∃r.B / B ≡ ∃r.A cycle underdetermines membership (not entailed)"
    );
    assert_eq!(
        iris_of(&instances_a),
        Vec::<String>::new(),
        "get_instances(A) must be empty (cycle not entailed)"
    );
    assert_eq!(
        iris_of(&instances_b),
        Vec::<String>::new(),
        "get_instances(B) must be empty (cycle not entailed)"
    );
    // get_instances ≡ per-individual is_instance_of parity on the cyclic TBox.
    for individual in [&a_ind, &b_ind] {
        for query in [&a_class, &b_class] {
            let in_list = reasoner
                .get_instances(query, false)?
                .iter()
                .any(|i| i == individual);
            assert_eq!(
                reasoner.is_instance_of(individual, query)?,
                in_list,
                "parity broken for {individual:?} vs {query:?}"
            );
        }
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "cyclic TBox instance queries must terminate in <1 s, took {elapsed:?}"
    );
    Ok(())
}

/// Self-referential defined classes: `C ≡ C ⊓ D` (with `D(x)`) and
/// `C ≡ ∃r.C` (with `r(x,x)`).
///
/// Each definition re-enters `check_instance(x, C)` from inside its own
/// evaluation — the entry guard must cut the cycle (conservative `false`,
/// tableau snapshot authoritative) so both `get_instances(C)` and
/// `is_instance_of(x, C)` return promptly.  Both cycles underdetermine `C`
/// (`C = ∅` satisfies them), so the entailed answer is "not an instance".
#[test]
fn self_referential_defined_classes_terminate_quickly() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    // `C ≡ C ⊓ D`, `D(x)`.
    let fs_intersect = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/selfint>
  EquivalentClasses(:C ObjectIntersectionOf(:C :D))
  ClassAssertion(:D :x)
)"#;
    // `C ≡ ∃r.C`, `r(x,x)`.
    let fs_existential = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/selfext>
  EquivalentClasses(:C ObjectSomeValuesFrom(:r :C))
  ObjectPropertyAssertion(:r :x :x)
)"#;

    for (fs, ns) in [
        (fs_intersect, "http://example.org/"),
        (fs_existential, "http://example.org/"),
    ] {
        let onto = parse_functional(fs).expect("ontology must parse");
        let start = std::time::Instant::now();

        let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
        reasoner.load_ontology(onto)?;

        let c = cls(&format!("{ns}C"));
        let x = named(&format!("{ns}x"));

        let instances = reasoner.get_instances(&c, false)?;
        assert!(
            !reasoner.is_instance_of(&x, &c)?,
            "x ∉ C for {ns}: the self-referential definition underdetermines C"
        );
        assert_eq!(
            iris_of(&instances),
            Vec::<String>::new(),
            "get_instances(C) must be empty for {ns}"
        );

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(1),
            "self-referential defined class queries must terminate in <1 s for {ns}, \
             took {elapsed:?}"
        );
    }
    Ok(())
}

/// Cyclic *data* definition cycle: `C ≡ (age some int.<40) ⊓ D`,
/// `D ≡ (age some int.<40) ⊓ C`, `age(x,10)`.
///
/// The cycle runs through `DataSomeValuesFrom` operands and the defined-
/// subclass evaluation — same termination contract as the object-property
/// cycle.  `C = D = ∅` satisfies the axioms, so membership is not entailed.
#[test]
fn cyclic_data_definition_terminates_quickly() -> Result<()> {
    use oxidowl::parsers::parse_functional;

    let fs = r#"Prefix(:=<http://example.org/>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/cycdata>
  EquivalentClasses(:C ObjectIntersectionOf(
    DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:maxExclusive "40"^^xsd:integer))
    :D))
  EquivalentClasses(:D ObjectIntersectionOf(
    DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:maxExclusive "40"^^xsd:integer))
    :C))
  DataPropertyAssertion(:age :x "10"^^xsd:integer)
)"#;
    let onto = parse_functional(fs).expect("ontology must parse");

    let start = std::time::Instant::now();
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(onto)?;

    let c = cls("http://example.org/C");
    let x = named("http://example.org/x");

    let instances = reasoner.get_instances(&c, false)?;
    assert!(
        !reasoner.is_instance_of(&x, &c)?,
        "x ∉ C: the C/D data-definition cycle underdetermines membership (not entailed)"
    );
    assert_eq!(
        iris_of(&instances),
        Vec::<String>::new(),
        "get_instances(C) must be empty (cycle not entailed)"
    );

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "cyclic data-definition queries must terminate in <1 s, took {elapsed:?}"
    );
    Ok(())
}

/// Nested defined-subclass explosion: `X` has `n` defined subclasses
/// `Dᵢ ≡ X ⊓ Pᵢ`; querying `instance(z, X)` walks the named-subclass BFS and
/// Strategy 5 re-enters `check_instance(z, X)` from *each* `Dᵢ` evaluation.
///
/// Red-before: without the general entry guard the helper-scoped
/// `DEFINED_EVAL_STACK` keys pop before the sibling Strategy-5 evaluations
/// run, so every level spawns fresh nested `check_instance(z, X)` frames —
/// the frame count grows factorially with `n` (12! ≈ 4.8×10⁸ frames, each
/// rebuilding the strategy indexes over the axioms) and the query never
/// finishes.  With the guard the nested state is cut at entry and the walk is
/// linear.  `X`'s extension is underdetermined (`X = ∅` satisfies every
/// `Dᵢ ≡ X ⊓ Pᵢ`), so the entailed answer is "not an instance".
#[test]
fn nested_defined_subclass_recursion_is_bounded() -> Result<()> {
    let n: u32 = 12;
    let mut axioms = Vec::new();
    let x = cls("http://example.org/nest#X");
    for i in 0..n {
        let p = cls(&format!("http://example.org/nest#P{i}"));
        let d = ClassExpression::Class(Class::new(IRI::new(&format!(
            "http://example.org/nest#D{i}"
        ))));
        axioms.push(Axiom::EquivalentClasses(
            oxidowl::ontology::EquivalentClassesAxiom {
                id: 100 + u64::from(i),
                classes: vec![d, ClassExpression::ObjectIntersectionOf(vec![x.clone(), p])],
                annotations: vec![],
            },
        ));
    }
    // At least one individual so `get_instances(X)` runs the checker at all.
    axioms.push(assert_class(
        "http://example.org/nest#z",
        cls("http://example.org/nest#P0"),
    ));

    let start = std::time::Instant::now();
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology(axioms))?;

    let z = named("http://example.org/nest#z");
    let instances = reasoner.get_instances(&x, false)?;
    assert!(
        !reasoner.is_instance_of(&z, &x)?,
        "z ∉ X: the Dᵢ ≡ X ⊓ Pᵢ definitions underdetermine X (not entailed)"
    );
    assert_eq!(
        iris_of(&instances),
        Vec::<String>::new(),
        "get_instances(X) must be empty"
    );
    // Sanity: the asserted membership is still answered through Strategy 1.
    assert_eq!(
        iris_of(&reasoner.get_instances(&cls("http://example.org/nest#P0"), false)?),
        vec!["http://example.org/nest#z".to_string()],
        "get_instances(P0) must contain the asserted individual z"
    );

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "nested defined-subclass recursion must be bounded (<1 s), took {elapsed:?}"
    );
    Ok(())
}
