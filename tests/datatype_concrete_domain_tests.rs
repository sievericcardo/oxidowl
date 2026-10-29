//! Concrete-domain (datatype) reasoning tests — workstream A4.
//!
//! These integration tests verify the tableau's `V` of SROIQV(D):
//!
//! 1. `DataSomeValuesFrom` over an **empty** range (contradictory facets, disjoint
//!    datatype intersection) produces a datatype clash → the class is unsatisfiable and
//!    an asserted instance makes the ontology inconsistent.
//! 2. Facet enforcement in the value-space check: `pattern`, `length`, `minLength`,
//!    `maxLength`, `totalDigits`, `fractionDigits`, `langRange`, and the ordering facets.
//! 3. Datatype subtyping (`"5"^^xsd:int` satisfies the `xsd:integer` range).
//! 4. `DataHasValue` value equality (`a P "x"` ⇒ `a ∈ A ≡ ∃P.{x}`).
//! 5. `DataMinCardinality` / `DataMaxCardinality` clash on a real individual.
//! 6. Datatype disjointness (`xsd:int ⊓ xsd:string` is empty).
//! 7. `DatatypeDefinition`: a literal of a user-defined datatype is validated against
//!    its defining range (ill-typed literals are inconsistent).
//! 8. `owl:real` / `owl:rational` basic value space.
//! 9. Concrete literal generation: `∃P.xsd:int` mints a real literal on the data
//!    successor in the tableau.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{ConceptLabel, NodeType, Tableau, TableauState},
    },
    ontology::{
        ClassExpression, DataRange, Literal, Ontology, axioms::Axiom,
        datatypes::DatatypeDefinitionAxiom,
    },
};
use std::sync::Arc;
use std::time::Duration;

const EX: &str = "http://ex.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn config_with_timeout(strategy: BlockingStrategy, timeout_ms: u64) -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: strategy,
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

fn build_tableau(onto: Ontology, timeout_ms: u64) -> Result<Tableau> {
    Tableau::from_ontology(
        Arc::new(onto),
        config_with_timeout(BlockingStrategy::Anywhere, timeout_ms),
    )
}

fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: config_with_timeout(BlockingStrategy::Anywhere, 5000),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;
    Ok(r)
}

fn xsd_iri(local: &str) -> oxidowl::ontology::IRI {
    oxidowl::ontology::IRI::new(&format!("{XSD}{local}"))
}

fn xsd_range(local: &str) -> DataRange {
    DataRange::Datatype(xsd_iri(local))
}

/// The data successors of a node for a canonical role name that carry a concrete
/// `DataValue` label.
fn data_value_successors(tableau: &Tableau, node_id: usize, role: &str) -> Vec<Literal> {
    tableau.nodes()[node_id]
        .role_successors
        .get(role)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|succ| {
            tableau.nodes().get(succ).and_then(|n| {
                n.concepts.iter().find_map(|c| match c {
                    ConceptLabel::DataValue(lit) => Some(lit.clone()),
                    _ => None,
                })
            })
        })
        .collect()
}

/// Find the node id carrying the nominal label `{name}`.
fn nominal_node_id(tableau: &Tableau, name: &str) -> Option<usize> {
    tableau
        .nodes()
        .iter()
        .find(|n| {
            n.concepts
                .iter()
                .any(|c| matches!(c, ConceptLabel::Nominal(x) if x == name))
        })
        .map(|n| n.id)
}

// ──────────────────────────────────────────────────────────────────────────────
// 1. Contradictory facets → empty range → clash
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃P.(xsd:int [minInclusive 10, maxInclusive 5])` is an empty data range.
/// `A(a)` forces a P-successor in it → the ontology is inconsistent.
#[test]
fn datatype_contradictory_facets_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let empty_range = df.data_range_restriction(
        &xsd_iri("int"),
        vec![
            (
                xsd_iri("minInclusive"),
                df.typed_literal("10", &format!("{XSD}int")),
            ),
            (
                xsd_iri("maxInclusive"),
                df.typed_literal("5", &format!("{XSD}int")),
            ),
        ],
    );
    let some = df.data_some_values_from(p, empty_range);
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "A ≡ ∃P.(int [minInclusive 10, maxInclusive 5]) with A(a) must be inconsistent"
    );
    Ok(())
}

/// The same empty range without any instance of `A` keeps the ontology consistent
/// (root-level `TBox` encoding must not mint a spurious clash).
#[test]
fn datatype_empty_range_tbox_only_is_consistent() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let empty_range = df.data_range_restriction(
        &xsd_iri("int"),
        vec![
            (
                xsd_iri("minInclusive"),
                df.typed_literal("10", &format!("{XSD}int")),
            ),
            (
                xsd_iri("maxInclusive"),
                df.typed_literal("5", &format!("{XSD}int")),
            ),
        ],
    );
    let mut ont = df.build_ontology(vec![df.equivalent_classes(vec![
        a_cls.clone(),
        df.data_some_values_from(p, empty_range),
    ])]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        r.is_consistent()?,
        "TBox-only A ≡ ∃P.(empty) with no instance of A must be consistent"
    );
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ≡ ∃P.(empty) must make A unsatisfiable"
    );
    Ok(())
}

/// `A ≡ ∃P.(xsd:int ⊓ xsd:string)` is empty (disjoint datatypes) → `A(a)` is inconsistent.
#[test]
fn datatype_disjointness_intersection_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let disjoint = df.data_range_intersection_of(vec![xsd_range("int"), xsd_range("string")]);
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), df.data_some_values_from(p, disjoint)]),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "A ≡ ∃P.(xsd:int ⊓ xsd:string) with A(a) must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 2. Facet enforcement (pattern / length / ordering)
// ──────────────────────────────────────────────────────────────────────────────

/// `a P "123"` (xsd:string) makes `a ∈ A ≡ ∃P.(xsd:string [pattern "[0-9]{3}"])`; `"abc"` does not.
#[test]
fn datatype_pattern_facet_instance_check() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let pattern_range = df.data_range_restriction(
        &xsd_iri("string"),
        vec![(xsd_iri("pattern"), df.literal("[0-9]{3}"))],
    );
    let some = df.data_some_values_from(p.clone(), pattern_range);

    // "123" matches the pattern → a ∈ A.
    let mut ok_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some.clone()]),
        df.data_property_assertion(
            p.clone(),
            df.named(format!("{EX}a")),
            df.typed_literal("123", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(
        r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?,
        "\"123\" must satisfy pattern [0-9]{{3}}"
    );

    // "abc" does not match → a ∉ A.
    let mut bad_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("abc", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(
        !r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?,
        "\"abc\" must NOT satisfy pattern [0-9]{{3}}"
    );
    Ok(())
}

/// `xsd:string [length 3]`: "abc" satisfies, "abcd" does not.
#[test]
fn datatype_length_facet_instance_check() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let len_range = df.data_range_restriction(
        &xsd_iri("string"),
        vec![(
            xsd_iri("length"),
            df.typed_literal("3", &format!("{XSD}integer")),
        )],
    );
    let some = df.data_some_values_from(p.clone(), len_range);

    let mut ok_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some.clone()]),
        df.data_property_assertion(
            p.clone(),
            df.named(format!("{EX}a")),
            df.typed_literal("abc", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);

    let mut bad_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("abcd", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(!r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);
    Ok(())
}

/// `xsd:int [minInclusive 1, maxInclusive 10]`: 7 in, 42 out.
#[test]
fn datatype_ordering_facets_instance_check() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let range = df.data_range_restriction(
        &xsd_iri("int"),
        vec![
            (
                xsd_iri("minInclusive"),
                df.typed_literal("1", &format!("{XSD}int")),
            ),
            (
                xsd_iri("maxInclusive"),
                df.typed_literal("10", &format!("{XSD}int")),
            ),
        ],
    );
    let some = df.data_some_values_from(p.clone(), range);

    let mut ok_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some.clone()]),
        df.data_property_assertion(
            p.clone(),
            df.named(format!("{EX}a")),
            df.typed_literal("7", &format!("{XSD}int")),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);

    let mut bad_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("42", &format!("{XSD}int")),
        ),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(!r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 3. Datatype subtyping
// ──────────────────────────────────────────────────────────────────────────────

/// `"5"^^xsd:int` lies in the value space of `xsd:integer` (int ⊑ integer ⊑ decimal).
#[test]
fn datatype_subtyping_int_satisfies_integer_range() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let some = df.data_some_values_from(p.clone(), xsd_range("integer"));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("5", &format!("{XSD}int")),
        ),
    ]);
    df.auto_declare(&mut ont);
    let r = reasoner_for(ont)?;
    assert!(
        r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?,
        "\"5\"^^xsd:int must satisfy the xsd:integer data range"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 4. DataHasValue value equality
// ──────────────────────────────────────────────────────────────────────────────

/// `a P "x"` (xsd:string) ⇒ `a ∈ A ≡ DataHasValue(P, "x")`.
#[test]
fn datatype_has_value_instance_membership() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let has_value = df.data_has_value(p.clone(), df.typed_literal("x", &format!("{XSD}string")));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), has_value]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("x", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ont);
    let r = reasoner_for(ont)?;
    assert!(
        r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?,
        "a P \"x\" must make a an instance of A ≡ ∃P.{{\"x\"}}"
    );
    Ok(())
}

/// `A ≡ DataHasValue(P, "x")` and `A(a)` unfolds the value restriction in the tableau:
/// the a node gets a data successor carrying the concrete literal `"x"^^xsd:string`.
#[test]
fn datatype_has_value_mints_concrete_successor_in_tableau() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let has_value = df.data_has_value(p.clone(), df.typed_literal("x", &format!("{XSD}string")));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), has_value]),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let values = data_value_successors(&tableau, a_id, &format!("{EX}P"));
    assert_eq!(
        values.len(),
        1,
        "DataHasValue must create exactly one data successor"
    );
    assert_eq!(
        values[0].value, "x",
        "data successor must carry the value \"x\""
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 5. Data cardinality
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ DataMinCardinality 2 P xsd:int`, `A ⊑ DataMaxCardinality 1 P xsd:int` and
/// `A(a)` are together inconsistent (two distinct ints needed, at most one allowed).
#[test]
fn datatype_min_two_max_one_unsatisfiable_with_instance() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let int_range = xsd_range("int");

    let data_min = ClassExpression::DataMinCardinality {
        property: p.clone(),
        cardinality: 2,
        filler: int_range.clone(),
    };
    let data_max = ClassExpression::DataMaxCardinality {
        property: p.clone(),
        cardinality: 1,
        filler: int_range,
    };
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), data_min),
        df.sub_class_of(a_cls.clone(), data_max),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "A(a) with ≥2 P xsd:int and ≤1 P xsd:int must be inconsistent"
    );
    Ok(())
}

/// `A ⊑ DataMinCardinality 2 P xsd:int` mints two *distinct* concrete integer literals.
#[test]
fn datatype_min_two_mints_distinct_concrete_values() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let data_min = ClassExpression::DataMinCardinality {
        property: p.clone(),
        cardinality: 2,
        filler: xsd_range("int"),
    };
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), data_min),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let values = data_value_successors(&tableau, a_id, &format!("{EX}P"));
    assert_eq!(
        values.len(),
        2,
        "≥2 P xsd:int must create two data successors"
    );
    let mut lex: Vec<String> = values.iter().map(|l| l.value.clone()).collect();
    lex.sort();
    lex.dedup();
    assert_eq!(
        lex.len(),
        2,
        "the two minted integer literals must be genuinely distinct; got {lex:?}"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 6. Datatype disjointness
// ──────────────────────────────────────────────────────────────────────────────

/// `a P "x"^^xsd:string` with `A ≡ ∀P.xsd:int` and `A(a)` is inconsistent: the string
/// successor's concrete value is outside the xsd:int value space.
#[test]
fn datatype_all_values_from_clashes_on_disjoint_value() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let all = df.data_all_values_from(p.clone(), xsd_range("int"));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), all]),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("x", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "A(a), A ≡ ∀P.xsd:int and a P \"x\"^^xsd:string must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 7. DatatypeDefinition
// ──────────────────────────────────────────────────────────────────────────────

/// `DatatypeDefinition(:AdultAge, xsd:int [minInclusive 18])`.
fn adult_age_restriction(df: &DF) -> DataRange {
    df.data_range_restriction(
        &xsd_iri("int"),
        vec![(
            xsd_iri("minInclusive"),
            df.typed_literal("18", &format!("{XSD}int")),
        )],
    )
}

/// `a P "15"^^AdultAge` with `AdultAge` := xsd:int [minInclusive 18] is ill-typed →
/// inconsistent. `"20"^^AdultAge` is well-typed → consistent.
#[test]
fn datatype_definition_adult_age_ill_typed_inconsistent() -> Result<()> {
    let df = DF::new();
    let adult_age_iri = oxidowl::ontology::IRI::new(&format!("{EX}AdultAge"));
    let p = df.data_prop(format!("{EX}P"));

    let mut bad_ont = df.build_ontology(vec![
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom::new(
            df.next_id(),
            adult_age_iri.clone(),
            adult_age_restriction(&df),
            vec![],
        )),
        df.data_property_assertion(
            p.clone(),
            df.named(format!("{EX}a")),
            df.typed_literal("15", adult_age_iri.as_str()),
        ),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(
        !r.is_consistent()?,
        "a P \"15\"^^AdultAge with AdultAge ≥ 18 must be inconsistent"
    );

    let mut ok_ont = df.build_ontology(vec![
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom::new(
            df.next_id(),
            adult_age_iri.clone(),
            adult_age_restriction(&df),
            vec![],
        )),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("20", adult_age_iri.as_str()),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(
        r.is_consistent()?,
        "a P \"20\"^^AdultAge with AdultAge ≥ 18 must be consistent"
    );
    Ok(())
}

/// The `DatatypeDefinition` must be surfaced to the tableau: a data value of datatype
/// `AdultAge` is constrained by the defining range even through a `TBox` `∃P.AdultAge`.
#[test]
fn datatype_definition_generation_respects_definition() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let adult_age_iri = oxidowl::ontology::IRI::new(&format!("{EX}AdultAge"));
    let p = df.data_prop(format!("{EX}P"));

    let mut ont = df.build_ontology(vec![
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom::new(
            df.next_id(),
            adult_age_iri.clone(),
            adult_age_restriction(&df),
            vec![],
        )),
        df.equivalent_classes(vec![
            a_cls.clone(),
            df.data_some_values_from(p.clone(), DataRange::Datatype(adult_age_iri)),
        ]),
        df.class_assertion(a_cls, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    // A(a) with A ≡ ∃P.AdultAge must be satisfiable (the minted literal must satisfy
    // the definition's minInclusive 18 constraint).
    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A(a) with A ≡ ∃P.AdultAge must be consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let values = data_value_successors(&tableau, a_id, &format!("{EX}P"));
    assert_eq!(values.len(), 1, "∃P.AdultAge must mint a data successor");
    // The minted value must satisfy the defining range (int ≥ 18).
    let ok = values[0].value.parse::<i64>().is_ok_and(|v| v >= 18);
    assert!(
        ok,
        "minted AdultAge literal must satisfy minInclusive 18; got {:?}",
        values[0].value
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 8. owl:real / owl:rational value space
// ──────────────────────────────────────────────────────────────────────────────

/// `"1.5"^^xsd:decimal` is in the `owl:real` and `owl:rational` value spaces; `"x"^^xsd:string`
/// is not in `owl:real`.
#[test]
fn datatype_owl_real_rational_value_space() -> Result<()> {
    let df = DF::new();

    // decimal literal in owl:real range.
    let real_range = DataRange::Datatype(oxidowl::ontology::IRI::new(&format!("{OWL}real")));
    let mut ont_real = df.build_ontology(vec![
        df.equivalent_classes(vec![
            df.class_ce(format!("{EX}A")),
            df.data_some_values_from(df.data_prop(format!("{EX}P")), real_range),
        ]),
        df.data_property_assertion(
            df.data_prop(format!("{EX}P")),
            df.named(format!("{EX}a")),
            df.typed_literal("1.5", &format!("{XSD}decimal")),
        ),
    ]);
    df.auto_declare(&mut ont_real);
    let r = reasoner_for(ont_real)?;
    assert!(
        r.is_instance_of(&df.named(format!("{EX}a")), &df.class_ce(format!("{EX}A")))?,
        "\"1.5\"^^xsd:decimal must be in the owl:real value space"
    );

    // rational range also accepts decimal.
    let rational_range =
        DataRange::Datatype(oxidowl::ontology::IRI::new(&format!("{OWL}rational")));
    let mut ont_rat = df.build_ontology(vec![
        df.equivalent_classes(vec![
            df.class_ce(format!("{EX}B")),
            df.data_some_values_from(df.data_prop(format!("{EX}P")), rational_range),
        ]),
        df.data_property_assertion(
            df.data_prop(format!("{EX}P")),
            df.named(format!("{EX}b")),
            df.typed_literal("1.5", &format!("{XSD}decimal")),
        ),
    ]);
    df.auto_declare(&mut ont_rat);
    let r = reasoner_for(ont_rat)?;
    assert!(
        r.is_instance_of(&df.named(format!("{EX}b")), &df.class_ce(format!("{EX}B")))?,
        "\"1.5\"^^xsd:decimal must be in the owl:rational value space"
    );

    // string is NOT in owl:real.
    let mut ont_str = df.build_ontology(vec![
        df.equivalent_classes(vec![
            df.class_ce(format!("{EX}C")),
            df.data_some_values_from(
                df.data_prop(format!("{EX}P")),
                DataRange::Datatype(oxidowl::ontology::IRI::new(&format!("{OWL}real"))),
            ),
        ]),
        df.data_property_assertion(
            df.data_prop(format!("{EX}P")),
            df.named(format!("{EX}c")),
            df.typed_literal("x", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ont_str);
    let r = reasoner_for(ont_str)?;
    assert!(
        !r.is_instance_of(&df.named(format!("{EX}c")), &df.class_ce(format!("{EX}C")))?,
        "\"x\"^^xsd:string must NOT be in the owl:real value space"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 9. Concrete literal generation in the tableau
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃P.xsd:int`, `A(a)` mints a real data successor carrying an xsd:int literal.
#[test]
fn datatype_some_values_from_mints_concrete_literal() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![
            a_cls.clone(),
            df.data_some_values_from(p.clone(), xsd_range("int")),
        ]),
        df.class_assertion(a_cls, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let values = data_value_successors(&tableau, a_id, &format!("{EX}P"));
    assert_eq!(
        values.len(),
        1,
        "∃P.xsd:int must mint exactly one data successor"
    );
    let datatype = values[0]
        .datatype
        .as_ref()
        .map(url::Url::to_string)
        .unwrap_or_default();
    assert_eq!(
        datatype,
        format!("{XSD}int"),
        "minted literal must be typed xsd:int"
    );
    assert!(
        values[0].value.parse::<i32>().is_ok(),
        "minted literal must be a valid int lexical form; got {:?}",
        values[0].value
    );
    Ok(())
}

/// The minted data successors are labelled with the range witness too (used by ≤-rule
/// counting when value-space matching is unavailable).
#[test]
fn datatype_successors_carry_range_witness_marker() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(
            a_cls.clone(),
            ClassExpression::DataMinCardinality {
                property: p.clone(),
                cardinality: 2,
                filler: xsd_range("int"),
            },
        ),
        df.class_assertion(a_cls, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let succs: Vec<usize> = tableau.nodes()[a_id]
        .role_successors
        .get(format!("{EX}P").as_str())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();
    assert_eq!(
        succs.len(),
        2,
        "≥2 P xsd:int must create two data successors"
    );
    for s in succs {
        let node = &tableau.nodes()[s];
        assert_eq!(node.node_type, NodeType::Generated);
        assert!(
            node.concepts
                .iter()
                .any(|c| matches!(c, ConceptLabel::DataValue(_))),
            "each data successor must carry a concrete DataValue"
        );
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// DataOneOf
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃P.{ "x", "y" }` — `a P "y"` puts a in A; `a P "z"` does not.
#[test]
fn datatype_one_of_instance_check() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let one_of = df.data_range_one_of(vec![
        df.typed_literal("x", &format!("{XSD}string")),
        df.typed_literal("y", &format!("{XSD}string")),
    ]);
    let some = df.data_some_values_from(p.clone(), one_of);

    let mut ok_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some.clone()]),
        df.data_property_assertion(
            p.clone(),
            df.named(format!("{EX}a")),
            df.typed_literal("y", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);

    let mut bad_ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), some]),
        df.data_property_assertion(
            p,
            df.named(format!("{EX}a")),
            df.typed_literal("z", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(!r.is_instance_of(&df.named(format!("{EX}a")), &a_cls)?);
    Ok(())
}

/// `DatatypeDefinition` clause generation: the DL clause generator must emit a clause for
/// `DatatypeDefinition(AdultAge, xsd:int [minInclusive 18])` (previously ignored).
#[test]
fn datatype_definition_generates_dl_clause() -> Result<()> {
    use oxidowl::dl_clauses::DLClauseGenerator;
    let df = DF::new();
    let adult_age_iri = oxidowl::ontology::IRI::new(&format!("{EX}AdultAge"));
    let mut ont = df.build_ontology(vec![Axiom::DatatypeDefinition(
        DatatypeDefinitionAxiom::new(
            df.next_id(),
            adult_age_iri,
            adult_age_restriction(&df),
            vec![],
        ),
    )]);
    df.auto_declare(&mut ont);

    let mut generator = DLClauseGenerator::new();
    let set = generator.generate_clauses(&ont)?;
    let total = set.deterministic_clauses.len() + set.disjunctive_clauses.len();
    assert!(
        total >= 1,
        "DatatypeDefinition must produce at least one DL clause; got {total}"
    );
    Ok(())
}
