//! Integration tests for RBox (role-axiom) enforcement in the active tableau
//! (workstream A5).
//!
//! Covers: transitivity, symmetry, asymmetry, irreflexivity, reflexivity, disjoint
//! roles, functional / inverse-functional merging, role inclusions, complex role
//! inclusions (RIAs of length > 2), negative object property assertions, HasKey,
//! and the non-simple-property interplay (no crash).

mod helpers;

use helpers::df::DF;
use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{ConceptLabel, Tableau, TableauState},
    },
    ontology::{ObjectPropertyExpression, Ontology},
};
use std::sync::Arc;
use std::time::Duration;

const EX: &str = "http://ex.org/";

fn config_with_timeout(timeout_ms: u64) -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: BlockingStrategy::Anywhere,
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

fn build_tableau(onto: Ontology, timeout_ms: u64) -> Result<Tableau> {
    Tableau::from_ontology(Arc::new(onto), config_with_timeout(timeout_ms))
}

fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(oxidowl::config::ReasonerConfig {
        reasoning: config_with_timeout(5000),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;
    Ok(r)
}

/// Node id of the first node carrying `ConceptLabel::Nominal(name)`.
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

/// Whether the tableau has an edge `a --role--> b` (by individual IRI).
fn has_edge(tableau: &Tableau, a: &str, b: &str, role: &str) -> bool {
    let (Some(src), Some(tgt)) = (nominal_node_id(tableau, a), nominal_node_id(tableau, b)) else {
        return false;
    };
    tableau
        .edges()
        .iter()
        .any(|e| e.from == src && e.to == tgt && e.role.to_string() == role)
}

// ──────────────────────────────────────────────────────────────────────────────
// Transitivity: TransitiveObjectProperty(R), R(a,b), R(b,c) ⊨ R(a,c)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_transitivity_entails_direct_edge() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.transitive_object_property(r.clone()),
        df.object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}b")),
            df.named(&format!("{EX}c")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}a"),
            &format!("{EX}c"),
            &r.to_string()
        ),
        "transitivity must derive R(a,c) from R(a,b) ∧ R(b,c)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Symmetry: SymmetricObjectProperty(S), S(a,b) ⊨ S(b,a)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_symmetry_derives_reverse_edge() -> Result<()> {
    let df = DF::new();
    let s = df.obj_prop(&format!("{EX}S"));
    let mut ont = df.build_ontology(vec![
        df.symmetric_object_property(s.clone()),
        df.object_property_assertion(
            s.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}b"),
            &format!("{EX}a"),
            &s.to_string()
        ),
        "symmetry must derive S(b,a) from S(a,b)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Asymmetry: AsymmetricObjectProperty(A), A(a,b) ∧ A(b,a) ⇒ inconsistent
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_asymmetry_clash() -> Result<()> {
    let df = DF::new();
    let a = df.obj_prop(&format!("{EX}A"));
    let mut ont = df.build_ontology(vec![
        df.asymmetric_object_property(a.clone()),
        df.object_property_assertion(
            a.clone(),
            df.named(&format!("{EX}x")),
            df.named(&format!("{EX}y")),
        ),
        df.object_property_assertion(
            a.clone(),
            df.named(&format!("{EX}y")),
            df.named(&format!("{EX}x")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        !r.is_consistent()?,
        "asymmetric property with both directions must be inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Unsatisfiable,
        "tableau must report Unsatisfiable for an asymmetric violation"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Irreflexivity: IrreflexiveObjectProperty(I), I(a,a) ⇒ inconsistent
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_irreflexivity_clash() -> Result<()> {
    let df = DF::new();
    let i = df.obj_prop(&format!("{EX}I"));
    let mut ont = df.build_ontology(vec![
        df.irreflexive_object_property(i.clone()),
        df.object_property_assertion(
            i.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}a")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        !r.is_consistent()?,
        "irreflexive property with a self-loop must be inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Unsatisfiable);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Reflexivity: ReflexiveObjectProperty(RF) ⇒ RF(a,a) for every individual
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_reflexivity_derives_self_loops() -> Result<()> {
    let df = DF::new();
    let rf = df.obj_prop(&format!("{EX}RF"));
    let mut ont = df.build_ontology(vec![
        df.reflexive_object_property(rf.clone()),
        // A named individual a so there is an individual witness in the tableau.
        df.class_assertion(df.owl_thing(), df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}a"),
            &format!("{EX}a"),
            &rf.to_string()
        ),
        "reflexive property must add a self-loop RF(a,a)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Disjoint roles: DisjointObjectProperties(P,Q), P(a,b) ∧ Q(a,b) ⇒ inconsistent
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_disjoint_roles_clash() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let q = df.obj_prop(&format!("{EX}Q"));
    let mut ont = df.build_ontology(vec![
        df.disjoint_object_properties(vec![p.clone(), q.clone()]),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            q.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        !r.is_consistent()?,
        "disjoint object properties sharing an edge must be inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Unsatisfiable);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Functional: FunctionalObjectProperty(P), P(a,b) ∧ P(a,c) ⇒ b sameAs c
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_functional_merges_successors() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let mut ont = df.build_ontology(vec![
        df.functional_object_property(p.clone()),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}c")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "functional P(a,b) ∧ P(a,c) is consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    // The two successors must be merged into a single node (both nominals on one node).
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal b");
    let c_id = nominal_node_id(&tableau, &format!("{EX}c")).expect("nominal c");
    assert_eq!(
        b_id, c_id,
        "functional property must merge the two P-successors of a"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Inverse-functional: InverseFunctional(P), P(a,c) ∧ P(b,c) ⇒ a sameAs b
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_inverse_functional_merges_sources() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let mut ont = df.build_ontology(vec![
        df.inverse_functional_object_property(p.clone()),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}c")),
        ),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}b")),
            df.named(&format!("{EX}c")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "inverse-functional P(a,c) ∧ P(b,c) is consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal b");
    assert_eq!(
        a_id, b_id,
        "inverse-functional property must merge the two P-sources of c"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Role inclusion: SubObjectPropertyOf(R,S), R(a,b) ⇒ S(a,b)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_role_inclusion_derives_super_edge() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let s = df.obj_prop(&format!("{EX}S"));
    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}a"),
            &format!("{EX}b"),
            &s.to_string()
        ),
        "SubObjectPropertyOf(R,S) with R(a,b) must derive S(a,b)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Complex role inclusion (RIA) of length 3:
// SubObjectPropertyOf(R∘S∘T, U), R(a,b), S(b,c), T(c,d) ⇒ U(a,d)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_ria_length_three_composes_path() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let s = df.obj_prop(&format!("{EX}S"));
    let t = df.obj_prop(&format!("{EX}T"));
    let u = df.obj_prop(&format!("{EX}U"));
    let chain = ObjectPropertyExpression::PropertyChain(vec![r.clone(), s.clone(), t.clone()]);
    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(chain, u.clone()),
        df.object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            s.clone(),
            df.named(&format!("{EX}b")),
            df.named(&format!("{EX}c")),
        ),
        df.object_property_assertion(
            t.clone(),
            df.named(&format!("{EX}c")),
            df.named(&format!("{EX}d")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}a"),
            &format!("{EX}d"),
            &u.to_string()
        ),
        "R∘S∘T ⊑ U with R(a,b),S(b,c),T(c,d) must derive U(a,d)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Negative assertion: NegativeObjectPropertyAssertion(R,a,b) + R(a,b) ⇒ inconsistent
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_negative_object_property_assertion_clash() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.negative_object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r2 = reasoner_for(ont.clone())?;
    assert!(
        !r2.is_consistent()?,
        "¬R(a,b) together with R(a,b) must be inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Unsatisfiable);
    Ok(())
}

/// ¬R(a,b) must also introduce a ≠ b, so a reflexive-ish / functional-ish pattern cannot
/// collapse a and b without a clash. Concretely: ¬R(a,b) + SameIndividual(a,b) is
/// inconsistent (a = b would force R(a,b) for any reflexive superproperty, but more
/// directly the inequality a ≠ b conflicts with the merge).
#[test]
fn rbox_negative_assertion_conflicts_with_same_individual() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.negative_object_property_assertion(
            r.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.same_individual(vec![
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ]),
    ]);
    df.auto_declare(&mut ont);

    // SameIndividual(a,b) merges the nodes, then the inequality a ≠ b is a clash.
    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Unsatisfiable,
        "¬R(a,b) + SameIndividual(a,b) must be inconsistent (a ≠ b forced)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// HasKey: HasKey(C,[P]), a:C, b:C, P(a,v) ∧ P(b,v) ⇒ a sameAs b
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_has_key_merges_individuals() -> Result<()> {
    let df = DF::new();
    let c = df.class_ce(&format!("{EX}C"));
    let p = df.obj_prop(&format!("{EX}P"));
    let v = df.named(&format!("{EX}v"));
    let mut ont = df.build_ontology(vec![
        df.has_key(c.clone(), vec![p.clone()], vec![]),
        df.class_assertion(c.clone(), df.named(&format!("{EX}a"))),
        df.class_assertion(c.clone(), df.named(&format!("{EX}b"))),
        df.object_property_assertion(p.clone(), df.named(&format!("{EX}a")), v.clone()),
        df.object_property_assertion(p.clone(), df.named(&format!("{EX}b")), v.clone()),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(r.is_consistent()?, "HasKey ontology is consistent");

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal b");
    assert_eq!(
        a_id, b_id,
        "HasKey(C,[P]) with a:C, b:C, P(a,v), P(b,v) must merge a and b"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Non-simple interplay: cardinality restriction on a transitive (non-simple) property
// must not crash the reasoner. Full non-simple rejection is the DL validator's job (B3).
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_cardinality_on_transitive_property_does_not_crash() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let c = df.class_ce(&format!("{EX}C"));
    let atleast2 = df.min_cardinality(2, r.clone(), df.owl_thing());
    let mut ont = df.build_ontology(vec![
        df.transitive_object_property(r.clone()),
        df.sub_class_of(c.clone(), atleast2),
        df.class_assertion(c.clone(), df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r2 = reasoner_for(ont.clone())?;
    // The reasoner must terminate and return a Boolean — either is acceptable for A5,
    // the important property is that it does not crash or time out.
    let _ = r2.is_consistent()?;

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert!(
        matches!(
            state,
            TableauState::Satisfiable | TableauState::Unsatisfiable
        ),
        "reasoner must not crash on a cardinality restriction over a non-simple property"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// SameIndividual + DifferentIndividuals: the inequality contradicts the merge.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_same_individual_conflicts_with_different_individuals() -> Result<()> {
    let df = DF::new();
    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ]),
        df.different_individuals(vec![
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ]),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        !r.is_consistent()?,
        "SameIndividual(a,b) + DifferentIndividuals(a,b) must be inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Unsatisfiable);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Functional merge vs. DifferentIndividuals: functionality forces b = c, the ontology
// asserts b ≠ c ⇒ inconsistent.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_functional_merge_conflicts_with_different_individuals() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let mut ont = df.build_ontology(vec![
        df.functional_object_property(p.clone()),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}c")),
        ),
        df.different_individuals(vec![
            df.named(&format!("{EX}b")),
            df.named(&format!("{EX}c")),
        ]),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        !r.is_consistent()?,
        "FunctionalObjectProperty forcing b=c with DifferentIndividuals(b,c) is inconsistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Unsatisfiable);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// EquivalentObjectProperties: P ≡ Q derives Q(a,b) from P(a,b) and vice versa.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn rbox_equivalent_object_properties_both_directions() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let q = df.obj_prop(&format!("{EX}Q"));
    let mut ont = df.build_ontology(vec![
        df.equivalent_object_properties(vec![p.clone(), q.clone()]),
        df.object_property_assertion(
            p.clone(),
            df.named(&format!("{EX}a")),
            df.named(&format!("{EX}b")),
        ),
        df.object_property_assertion(
            q.clone(),
            df.named(&format!("{EX}b")),
            df.named(&format!("{EX}a")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}a"),
            &format!("{EX}b"),
            &q.to_string()
        ),
        "EquivalentObjectProperties(P,Q) must derive Q(a,b) from P(a,b)"
    );
    assert!(
        has_edge(
            &tableau,
            &format!("{EX}b"),
            &format!("{EX}a"),
            &p.to_string()
        ),
        "EquivalentObjectProperties(P,Q) must derive P(b,a) from Q(b,a)"
    );
    Ok(())
}
