//! N3 tests (B5 Slice 2 — real superset of Turtle, docs/PLAN.md).
//!
//! Locks `src/parsers/rio/n3.rs` (oxttl N3 engine + shared RDF→OWL mapping):
//!
//! 1. **Ground-triple round-trip equality** — serialize → parse must return
//!    the identical axiom set (declarations, SubClassOf, assertions,
//!    annotations, EquivalentClasses) plus the ontology IRI and prefixes.
//! 2. **Turtle-syntax superset** — `a` shorthand (Part 1b), `;` / `,`
//!    predicate-object lists, `@prefix`, blank-node property lists and
//!    collections all parse to the expected axioms.
//! 3. **Rules** — `{ ?x a :A } => { ?x a :B }` maps to an
//!    [`Axiom::Rule`] (SWRL) with the documented variable minting;
//!    `<=` swaps the sides; ground implications use individuals.
//! 4. **Lossless preservation** — implications that are not SWRL-expressible
//!    become `log:implies` ontology annotations, standalone formulae become
//!    `log:Formula` annotations, and `@forAll`/`@forSome` become
//!    `log:forAll`/`log:forSome` annotations; all of them survive a
//!    serialize → parse cycle unchanged.
//! 5. **N3 verbs** — `!`/`^` paths (including the oxttl predicate-path
//!    repair), `is … of`, `has`, and RDF lists in class expressions.

#[allow(dead_code)]
#[path = "helpers/mod.rs"]
mod helpers;

use helpers::assertions::assert_ontologies_axiom_equal;
use helpers::df::DF;
use oxidowl::ontology::*;
use oxidowl::parsers::rio::n3;
use oxidowl::parsers::save_to_string;

const EX: &str = "http://example.org/";
const OWL_ONTOLOGY_IRI: &str = "http://example.org/onto";

const LOG_IMPLIES: &str = "http://www.w3.org/2000/10/swap/log#implies";
const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";
const LOG_FOR_ALL: &str = "http://www.w3.org/2000/10/swap/log#forAll";
const LOG_FOR_SOME: &str = "http://www.w3.org/2000/10/swap/log#forSome";

/// Serialize as N3 and parse the result back with the N3 parser.
fn n3_roundtrip(ont: &Ontology) -> Ontology {
    let text = save_to_string(ont, OntologyFormat::N3)
        .unwrap_or_else(|e| panic!("N3 serialization failed: {e}"));
    n3::parse(&text).unwrap_or_else(|e| panic!("N3 reparse failed: {e}:\n{text}"))
}

/// Parse an N3 document, panicking with context on failure.
fn parse_n3(doc: &str) -> Ontology {
    n3::parse(doc).unwrap_or_else(|e| panic!("N3 parse failed: {e}"))
}

/// Ontology annotations with the given property IRI (order-preserving).
fn preserved<'a>(ont: &'a Ontology, property: &str) -> Vec<&'a AnnotationValue> {
    ont.annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == property)
        .map(|a| &a.value)
        .collect()
}

fn literal_text(value: &AnnotationValue) -> &str {
    match value {
        AnnotationValue::Literal(lit) => lit.value.as_str(),
        other => panic!("expected a literal annotation value, got {other:?}"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Ground-triple round-trip equality
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn ground_triple_roundtrip_equality() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));
    let c = df.class_ce(format!("{EX}C"));
    let d = df.class_ce(format!("{EX}D"));
    let p = df.obj_prop(format!("{EX}p"));
    let dp = df.data_prop(format!("{EX}age"));
    let i = df.named(format!("{EX}i"));
    let j = df.named(format!("{EX}j"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.class_assertion(c.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(42)),
        df.equivalent_classes(vec![c.clone(), d.clone()]),
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new(&format!("{EX}A"))),
            property: df.annotation_property(format!("{EX}ann")),
            value: AnnotationValue::Literal(df.literal("hello")),
            annotations: vec![],
        }),
    ]);
    df.auto_declare(&mut ont);
    ont.set_iri(IRI::new(OWL_ONTOLOGY_IRI));
    ont.prefixes.insert("ex".to_owned(), IRI::new(EX));

    let reloaded = n3_roundtrip(&ont);
    assert_ontologies_axiom_equal(&ont, &reloaded);

    // Ontology identity and prefixes survive as well.
    assert_eq!(
        reloaded.get_iri().map(ToString::to_string).as_deref(),
        Some(OWL_ONTOLOGY_IRI),
        "ontology IRI must round-trip"
    );
    assert_eq!(
        reloaded.prefixes.get("ex").map(IRI::as_str),
        Some(EX),
        "prefix declarations must round-trip"
    );

    // Every required kind is individually present after the round-trip.
    for kind in [
        AxiomType::Declaration,
        AxiomType::SubClassOf,
        AxiomType::ClassAssertion,
        AxiomType::ObjectPropertyAssertion,
        AxiomType::DataPropertyAssertion,
        AxiomType::EquivalentClasses,
        AxiomType::AnnotationAssertion,
    ] {
        assert!(
            reloaded.axioms().iter().any(|ax| ax.axiom_type() == kind),
            "{kind:?} lost in the N3 round-trip; axioms: {:?}",
            reloaded.axioms()
        );
    }

    // Byte-oriented entry point must agree with the string one.
    let text = save_to_string(&ont, OntologyFormat::N3).expect("serialize");
    let from_bytes = n3::parse_bytes(text.as_bytes()).expect("parse_bytes");
    assert_ontologies_axiom_equal(&ont, &from_bytes);
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Turtle-syntax superset: `a`, `;`, `,`, prefixes, blank nodes, lists
// ═════════════════════════════════════════════════════════════════════════════

/// Part 1b: N3 (the Turtle superset) must honour the `a` → `rdf:type`
/// shorthand exactly like the Turtle parser (Slice 0 fix), together with
/// `;` / `,` lists, `@prefix` declarations and blank-node property lists.
#[test]
fn a_shorthand_semicolon_comma_prefixes_and_blank_nodes() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         ex:A a owl:Class .\n\
         ex:B a owl:Class .\n\
         ex:i a ex:A, ex:B ;\n\
             ex:p ex:j ;\n\
             ex:label \"hi\" .\n"
    );
    let ont = parse_n3(&doc);

    // `a` shorthand → declarations and class assertions.
    assert!(
        ont.axioms().iter().any(|ax| matches!(
            ax,
            Axiom::Declaration(d) if matches!(&d.entity, Entity::Class(c)
                if c.as_str() == format!("{EX}A"))
        )),
        "`ex:A a owl:Class` must declare class A; axioms: {:?}",
        ont.axioms()
    );
    let class_assertions: Vec<_> = ont
        .axioms()
        .iter()
        .filter_map(|ax| match ax {
            Axiom::ClassAssertion(ca) => Some(ca.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        class_assertions.len(),
        2,
        "`ex:i a ex:A, ex:B` must produce two class assertions: {class_assertions:?}"
    );

    // `;`-separated predicate list.
    assert!(
        ont.axioms().iter().any(|ax| matches!(
            ax,
            Axiom::ObjectPropertyAssertion(o)
                if o.property == df_obj_prop(&format!("{EX}p"))
                    && matches!(&o.target, Individual::Named(n) if n.iri.as_str() == format!("{EX}j"))
        )),
        "`ex:p ex:j` after `;` must produce an object property assertion"
    );
    // Literal object → data property assertion.
    assert!(
        ont.axioms().iter().any(|ax| matches!(
            ax,
            Axiom::DataPropertyAssertion(d)
                if d.value.value == "hi"
        )),
        "`ex:label \"hi\"` must produce a data property assertion"
    );

    // Blank-node property lists reconstruct OWL restrictions.
    let blank_doc = format!(
        "@prefix ex: <{EX}> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         ex:A owl:equivalentClass [ a owl:Restriction ;\n\
             owl:onProperty ex:p ;\n\
             owl:someValuesFrom ex:B ] .\n"
    );
    let blank_ont = parse_n3(&blank_doc);
    assert!(
        blank_ont.axioms().iter().any(|ax| matches!(
            ax,
            Axiom::EquivalentClasses(e) if e.classes.iter().any(
                |c| matches!(c, ClassExpression::ObjectSomeValuesFrom { .. })
            )
        )),
        "blank-node restriction must reconstruct ObjectSomeValuesFrom; axioms: {:?}",
        blank_ont.axioms()
    );

    // Collections (RDF lists) in list-valued axioms.
    let list_doc = format!(
        "@prefix ex: <{EX}> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         ex:A a owl:Class .\n\
         ex:B a owl:Class .\n\
         ex:C a owl:Class .\n\
         ex:A owl:equivalentClass [ owl:unionOf ( ex:B ex:C ) ] .\n"
    );
    let list_ont = parse_n3(&list_doc);
    assert!(
        list_ont.axioms().iter().any(|ax| matches!(
            ax,
            Axiom::EquivalentClasses(e) if e.classes.iter().any(
                |c| matches!(c, ClassExpression::ObjectUnionOf(members) if members.len() == 2)
            )
        )),
        "`owl:unionOf ( … )` list must reconstruct a union; axioms: {:?}",
        list_ont.axioms()
    );
}

/// Helper: build an `ObjectPropertyExpression` for comparison.
fn df_obj_prop(iri: &str) -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: IRI::new(iri) })
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Implications → SWRL rules
// ═════════════════════════════════════════════════════════════════════════════

/// The chosen representation for the canonical N3 rule: an [`Axiom::Rule`]
/// whose variables are minted as `urn:oxidowl:n3var:<name>`.
#[test]
fn simple_implication_maps_to_swrl_rule() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         {{ ?x a ex:A }} => {{ ?x a ex:B }} .\n"
    );
    let ont = parse_n3(&doc);

    let rules: Vec<_> = ont
        .axioms()
        .iter()
        .filter_map(|ax| match ax {
            Axiom::Rule(r) => Some(r.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        rules.len(),
        1,
        "exactly one rule; axioms: {:?}",
        ont.axioms()
    );

    let rule = &rules[0].rule;
    let var_x = SWRLVariable::new(IRI::new("urn:oxidowl:n3var:x"));
    assert_eq!(
        rule.body,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}A")))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        }],
        "body must be `?x a ex:A`"
    );
    assert_eq!(
        rule.head,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}B")))),
            argument: SWRLIArgument::Variable(var_x),
        }],
        "head must be `?x a ex:B`"
    );
    assert!(
        rule.is_safe(),
        "head variables are a subset of body variables"
    );

    // Round-trip: serialize → parse reproduces the same rule axiom set.
    let reloaded = n3_roundtrip(&ont);
    assert_ontologies_axiom_equal(&ont, &reloaded);
    assert!(
        reloaded
            .axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::Rule(_))),
        "rule must survive the N3 round-trip"
    );
}

/// `A <= B` means “B implies A”: the rule sides must be swapped relative to
/// the `=>` spelling.
#[test]
fn backward_implication_le_swaps_rule_sides() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         {{ ?x a ex:A }} <= {{ ?x a ex:B }} .\n"
    );
    let ont = parse_n3(&doc);
    let rule = ont
        .axioms()
        .iter()
        .find_map(|ax| match ax {
            Axiom::Rule(r) => Some(r.clone()),
            _ => None,
        })
        .expect("`<=` must produce a rule");
    let var_x = SWRLVariable::new(IRI::new("urn:oxidowl:n3var:x"));
    // Body of `<=` is the RIGHT-hand formula.
    assert_eq!(
        rule.rule.body,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}B")))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        }],
        "body must come from the right-hand side of `<=`"
    );
    assert_eq!(
        rule.rule.head,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}A")))),
            argument: SWRLIArgument::Variable(var_x),
        }],
        "head must come from the left-hand side of `<=`"
    );
}

/// Ground (variable-free) implications still map to SWRL — with individual
/// arguments instead of variables.
#[test]
fn ground_implication_maps_to_rule_with_individuals() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         {{ ex:i a ex:A }} => {{ ex:i a ex:B }} .\n"
    );
    let ont = parse_n3(&doc);
    let rule = ont
        .axioms()
        .iter()
        .find_map(|ax| match ax {
            Axiom::Rule(r) => Some(r.clone()),
            _ => None,
        })
        .expect("ground implication must produce a rule");
    assert_eq!(
        rule.rule.body,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}A")))),
            argument: SWRLIArgument::Individual(Individual::Named(NamedIndividual {
                iri: IRI::new(&format!("{EX}i")),
            })),
        }],
    );
    assert_eq!(
        rule.rule.head,
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(&format!("{EX}B")))),
            argument: SWRLIArgument::Individual(Individual::Named(NamedIndividual {
                iri: IRI::new(&format!("{EX}i")),
            })),
        }],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Lossless preservation of non-SWRL N3 constructs
// ═════════════════════════════════════════════════════════════════════════════

/// A rule body containing RDF-list structure is not SWRL-expressible: it must
/// be preserved as a `log:implies` annotation carrying the canonical N3 text
/// — and that annotation must survive a serialize → parse cycle unchanged.
#[test]
fn unmappable_implication_preserved_as_log_implies_annotation() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         {{ ?x ex:member ( ex:one ex:two ) }} => {{ ?x a ex:B }} .\n"
    );
    let ont = parse_n3(&doc);
    assert!(
        !ont.axioms().iter().any(|ax| matches!(ax, Axiom::Rule(_))),
        "list-structured bodies are not SWRL-expressible"
    );
    let values = preserved(&ont, LOG_IMPLIES);
    assert_eq!(values.len(), 1, "exactly one log:implies annotation");
    let text = literal_text(values[0]);
    assert!(
        text.starts_with("{ ") && text.contains(" => { ") && text.ends_with(" }"),
        "annotation must hold canonical `{{ … }} => {{ … }}` N3 text, got: {text}"
    );
    assert!(
        text.contains("rdf-syntax-ns#first"),
        "the list structure must be inside the preserved text: {text}"
    );

    // Round-trip: the annotation text is re-emitted verbatim.
    let reloaded = n3_roundtrip(&ont);
    assert_eq!(
        ont.annotations, reloaded.annotations,
        "preserved log:implies annotations must be idempotent"
    );
    assert_ontologies_axiom_equal(&ont, &reloaded);
}

/// A standalone `{ … }` formula (not an implication side) is quoted as a
/// `log:Formula` annotation and round-trips unchanged.
#[test]
fn standalone_formula_preserved_as_log_formula_annotation() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         ex:i ex:p ex:j .\n\
         {{ ex:k ex:q ex:v }} .\n"
    );
    let ont = parse_n3(&doc);
    // The ground statement is a normal axiom…
    assert!(
        ont.axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::ObjectPropertyAssertion(_))),
        "ground triples map to axioms"
    );
    // …and the formula is quoted, not dropped.
    let values = preserved(&ont, LOG_FORMULA);
    assert_eq!(
        values.len(),
        1,
        "one log:Formula annotation; {:?}",
        ont.annotations
    );
    assert_eq!(
        literal_text(values[0]),
        format!("{{ <{EX}k> <{EX}q> <{EX}v> . }}"),
        "formula text must be quoted losslessly"
    );

    let reloaded = n3_roundtrip(&ont);
    assert_eq!(
        ont.annotations, reloaded.annotations,
        "log:Formula annotations must be idempotent"
    );
    assert_ontologies_axiom_equal(&ont, &reloaded);
}

/// `@forAll` / `@forSome` quantifier statements are extracted before parsing
/// (oxttl does not support them) and preserved as `log:forAll` /
/// `log:forSome` annotations — round-trip stable.
#[test]
fn forall_and_forsome_preserved_as_annotations() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         @forAll ex:x ex:y .\n\
         @forSome ?v .\n\
         ex:i ex:p ex:j .\n"
    );
    let ont = parse_n3(&doc);
    let forall = preserved(&ont, LOG_FOR_ALL);
    let forsome = preserved(&ont, LOG_FOR_SOME);
    assert_eq!(forall.len(), 1, "@forAll statement: {:?}", ont.annotations);
    assert_eq!(literal_text(forall[0]), "@forAll ex:x ex:y");
    assert_eq!(
        forsome.len(),
        1,
        "@forSome statement: {:?}",
        ont.annotations
    );
    assert_eq!(literal_text(forsome[0]), "@forSome ?v");
    // The surrounding ground statement still maps normally.
    assert!(
        ont.axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::ObjectPropertyAssertion(_)))
    );

    let reloaded = n3_roundtrip(&ont);
    assert_eq!(
        ont.annotations, reloaded.annotations,
        "quantifier annotations must be idempotent"
    );
    assert_ontologies_axiom_equal(&ont, &reloaded);
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. N3 verbs beyond Turtle: paths, `is … of`, `has`
// ═════════════════════════════════════════════════════════════════════════════

/// `!` / `^` paths (with the predicate-position repair), `is … of` and `has`
/// all parse into property assertions.
#[test]
fn paths_is_of_and_has_n3_verbs() {
    let doc = format!(
        "@prefix ex: <{EX}> .\n\
         ex:a ex:p!ex:q ex:b .\n\
         ex:c ex:p^ex:q ex:d .\n\
         ex:e is ex:p of ex:f .\n\
         ex:g has ex:p ex:h .\n"
    );
    let ont = parse_n3(&doc);
    let assertions: Vec<_> = ont
        .axioms()
        .iter()
        .filter_map(|ax| match ax {
            Axiom::ObjectPropertyAssertion(o) => Some(o.clone()),
            _ => None,
        })
        .collect();
    // Two paths × two steps = 4 assertions bridged by anonymous
    // intermediates, plus `is…of` and `has` = 6 total.
    assert_eq!(
        assertions.len(),
        6,
        "paths + is…of + has → 6 assertions: {assertions:?}"
    );
    // `ex:e is ex:p of ex:f` inverts: ex:f ex:p ex:e.
    assert!(
        assertions.iter().any(|o| {
            matches!(&o.source, Individual::Named(n) if n.iri.as_str() == format!("{EX}f"))
                && matches!(&o.target, Individual::Named(n) if n.iri.as_str() == format!("{EX}e"))
        }),
        "`e is p of f` must invert to `f p e`"
    );
    // `ex:g has ex:p ex:h` stays forward.
    assert!(
        assertions.iter().any(|o| {
            matches!(&o.source, Individual::Named(n) if n.iri.as_str() == format!("{EX}g"))
                && matches!(&o.target, Individual::Named(n) if n.iri.as_str() == format!("{EX}h"))
        }),
        "`g has p h` must stay `g p h`"
    );
}
