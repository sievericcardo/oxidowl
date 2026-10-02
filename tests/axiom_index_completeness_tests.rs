#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::datatypes::DatatypeDefinitionAxiom;
use oxidowl::ontology::*;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn xsd_iri(local: &str) -> IRI {
    IRI::new(&format!("{XSD}{local}"))
}

/// (a) Every logical axiom type added to an ontology is retrievable through
/// the incremental `AxiomIndex` maintained by `Ontology::add_axiom`.
#[test]
fn test_every_axiom_type_retrievable_via_index() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = df.obj_prop("http://ex.org/p");
    let q = df.obj_prop("http://ex.org/q");
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let integer = xsd_iri("integer");

    let dt_def_range = DataRange::DataComplementOf(Box::new(DataRange::Datatype(integer)));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.equivalent_classes(vec![a.clone(), df.class_ce("http://ex.org/A2")]),
        df.disjoint_classes(vec![a.clone(), df.class_ce("http://ex.org/D1")]),
        df.disjoint_union(
            b.clone(),
            vec![
                df.class_ce("http://ex.org/U1"),
                df.class_ce("http://ex.org/U2"),
            ],
        ),
        df.sub_object_property_of(p.clone(), q.clone()),
        df.equivalent_object_properties(vec![p.clone(), df.obj_prop("http://ex.org/p2")]),
        df.disjoint_object_properties(vec![p.clone(), df.obj_prop("http://ex.org/p3")]),
        df.inverse_object_properties(p.clone(), q.clone()),
        df.object_property_domain(p.clone(), a.clone()),
        df.object_property_range(p.clone(), b.clone()),
        df.functional_object_property(p.clone()),
        df.inverse_functional_object_property(p.clone()),
        df.reflexive_object_property(p.clone()),
        df.irreflexive_object_property(p.clone()),
        df.symmetric_object_property(p.clone()),
        df.asymmetric_object_property(p.clone()),
        df.transitive_object_property(p.clone()),
        df.sub_data_property_of(dp.clone(), df.data_prop("http://ex.org/dp2")),
        df.equivalent_data_properties(vec![dp.clone(), df.data_prop("http://ex.org/dp3")]),
        df.disjoint_data_properties(vec![dp.clone(), df.data_prop("http://ex.org/dp4")]),
        df.data_property_domain(dp.clone(), a.clone()),
        df.data_property_range(dp.clone(), df.data_range_integer()),
        df.functional_data_property(dp.clone()),
        df.same_individual(vec![i.clone(), df.named("http://ex.org/i2")]),
        df.different_individuals(vec![j.clone(), df.named("http://ex.org/j2")]),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(42)),
        df.negative_object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.negative_data_property_assertion(dp.clone(), i.clone(), df.int_literal(7)),
        df.has_key(a.clone(), vec![p.clone()], vec![dp.clone()]),
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom {
            id: df.next_id(),
            datatype: IRI::new("http://ex.org/customDT"),
            data_range: dt_def_range,
            annotations: vec![],
        }),
    ]);

    let var_x = SWRLVariable::new(IRI::new("http://ex.org/var#x"));
    let rule = SWRLRule::new(
        vec![SWRLAtom::ClassAtom {
            predicate: a.clone(),
            argument: SWRLIArgument::Variable(var_x.clone()),
        }],
        vec![SWRLAtom::ObjectPropertyAtom {
            predicate: p.clone(),
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_x.clone()),
        }],
    );
    ont.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(df.next_id(), rule)));

    let index = ont.axiom_index();

    // Every logical axiom type must be retrievable.
    for t in [
        AxiomType::SubClassOf,
        AxiomType::EquivalentClasses,
        AxiomType::DisjointClasses,
        AxiomType::DisjointUnion,
        AxiomType::SubObjectPropertyOf,
        AxiomType::EquivalentObjectProperties,
        AxiomType::DisjointObjectProperties,
        AxiomType::InverseObjectProperties,
        AxiomType::ObjectPropertyDomain,
        AxiomType::ObjectPropertyRange,
        AxiomType::FunctionalObjectProperty,
        AxiomType::InverseFunctionalObjectProperty,
        AxiomType::ReflexiveObjectProperty,
        AxiomType::IrreflexiveObjectProperty,
        AxiomType::SymmetricObjectProperty,
        AxiomType::AsymmetricObjectProperty,
        AxiomType::TransitiveObjectProperty,
        AxiomType::SubDataPropertyOf,
        AxiomType::EquivalentDataProperties,
        AxiomType::DisjointDataProperties,
        AxiomType::DataPropertyDomain,
        AxiomType::DataPropertyRange,
        AxiomType::FunctionalDataProperty,
        AxiomType::SameIndividual,
        AxiomType::DifferentIndividuals,
        AxiomType::ClassAssertion,
        AxiomType::ObjectPropertyAssertion,
        AxiomType::DataPropertyAssertion,
        AxiomType::NegativeObjectPropertyAssertion,
        AxiomType::NegativeDataPropertyAssertion,
        AxiomType::HasKey,
        AxiomType::DatatypeDefinition,
        AxiomType::Rule,
    ] {
        assert!(
            !index.get(t.clone()).is_empty(),
            "AxiomIndex should contain axioms of type {t:?}"
        );
    }

    // Non-logical axiom types are indexed too.
    assert_eq!(index.get(AxiomType::Declaration).len(), 0); // none added here
    assert!(index.get(AxiomType::DatatypeDefinition).len() == 1);

    // Typed query helpers work.
    assert_eq!(index.sub_object_property_axioms().len(), 1);
    assert_eq!(index.same_individual_axioms().len(), 1);
    assert_eq!(index.datatype_definition_axioms().len(), 1);
    assert_eq!(index.rule_axioms().len(), 1);
    assert_eq!(index.transitive_object_property_axioms().len(), 1);
    assert_eq!(index.negative_object_property_assertion_axioms().len(), 1);

    // The incremental index agrees with a freshly built index.
    let rebuilt = ont.build_index();
    for t in [
        AxiomType::SubClassOf,
        AxiomType::EquivalentClasses,
        AxiomType::SameIndividual,
        AxiomType::DatatypeDefinition,
        AxiomType::Rule,
    ] {
        assert_eq!(
            index.get(t.clone()).len(),
            rebuilt.get(t.clone()).len(),
            "incremental index diverged from rebuilt index for {t:?}"
        );
    }
}

/// (a2) `remove_axiom` removes the axiom from the incremental index too.
#[test]
fn test_remove_axiom_updates_index() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let sub = df.sub_class_of(a.clone(), b.clone());
    let p = df.obj_prop("http://ex.org/p");

    let mut ont = df.build_ontology(vec![sub.clone(), df.transitive_object_property(p.clone())]);

    assert_eq!(ont.axiom_index().get(AxiomType::SubClassOf).len(), 1);
    assert_eq!(
        ont.axiom_index()
            .get(AxiomType::TransitiveObjectProperty)
            .len(),
        1
    );

    ont.remove_axiom(&sub);
    assert_eq!(
        ont.axiom_index().get(AxiomType::SubClassOf).len(),
        0,
        "removed SubClassOf must disappear from the index"
    );
    assert_eq!(
        ont.axiom_index()
            .get(AxiomType::TransitiveObjectProperty)
            .len(),
        1,
        "unrelated axioms must stay indexed"
    );
    assert!(
        ont.axiom_index().transitive_object_properties.contains(&p),
        "characteristic set must retain the property"
    );
}

/// (b) `DatatypeDefinition` round-trips through Functional Syntax with native
/// data ranges: DatatypeRestriction with facets, DataComplementOf, DataOneOf.
#[test]
fn test_datatype_definition_functional_roundtrip_native_ranges() {
    let df = DF::new();

    let restriction_range = DataRange::DatatypeRestriction {
        datatype: xsd_iri("integer"),
        restrictions: vec![
            FacetRestriction {
                facet: xsd_iri("minInclusive"),
                value: Literal::with_datatype("0".to_string(), xsd_iri("integer")),
            },
            FacetRestriction {
                facet: xsd_iri("maxInclusive"),
                value: Literal::with_datatype("100".to_string(), xsd_iri("integer")),
            },
        ],
    };
    let complement_range =
        DataRange::DataComplementOf(Box::new(DataRange::Datatype(xsd_iri("string"))));
    let one_of_range = DataRange::DataOneOf(vec![
        Literal::with_datatype("red".to_string(), xsd_iri("string")),
        Literal::with_datatype("green".to_string(), xsd_iri("string")),
    ]);

    let ont = df.build_ontology(vec![
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom {
            id: df.next_id(),
            datatype: IRI::new("http://ex.org/myInt"),
            data_range: restriction_range,
            annotations: vec![],
        }),
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom {
            id: df.next_id(),
            datatype: IRI::new("http://ex.org/notString"),
            data_range: complement_range,
            annotations: vec![],
        }),
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom {
            id: df.next_id(),
            datatype: IRI::new("http://ex.org/color"),
            data_range: one_of_range,
            annotations: vec![],
        }),
    ]);

    let mut tb = TestBase::new();
    let reloaded = tb
        .round_trip(&ont, OntologyFormat::Functional)
        .unwrap_or_else(|e| panic!("Functional roundtrip of DatatypeDefinition failed: {e}"));

    let defs: Vec<&DatatypeDefinitionAxiom> = reloaded
        .axioms()
        .iter()
        .filter_map(|ax| match ax {
            Axiom::DatatypeDefinition(dd) => Some(dd),
            _ => None,
        })
        .collect();
    assert_eq!(
        defs.len(),
        3,
        "All 3 DatatypeDefinition axioms must survive the Functional roundtrip"
    );

    let mut has_restriction = false;
    let mut has_complement = false;
    let mut has_one_of = false;
    for dd in &defs {
        match &dd.data_range {
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                has_restriction = true;
                assert!(datatype.as_str().contains("integer"));
                assert_eq!(restrictions.len(), 2);
                assert!(restrictions[0].facet.as_str().contains("minInclusive"));
                assert_eq!(restrictions[0].value.value, "0");
                assert!(restrictions[1].facet.as_str().contains("maxInclusive"));
                assert_eq!(restrictions[1].value.value, "100");
            }
            DataRange::DataComplementOf(inner) => {
                has_complement = true;
                assert!(matches!(inner.as_ref(), DataRange::Datatype(_)));
            }
            DataRange::DataOneOf(literals) => {
                has_one_of = true;
                assert_eq!(literals.len(), 2);
                assert_eq!(literals[0].value, "red");
                assert_eq!(literals[1].value, "green");
            }
            other => panic!("Unexpected data range in roundtripped definition: {other:?}"),
        }
    }
    assert!(has_restriction, "DatatypeRestriction range must roundtrip");
    assert!(has_complement, "DataComplementOf range must roundtrip");
    assert!(has_one_of, "DataOneOf range must roundtrip");
}

/// (c) `signature()` returns the correct entities for a mixed ontology
/// touching every axiom family.
#[test]
fn test_signature_mixed_ontology_all_families() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = df.obj_prop("http://ex.org/p");
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ap = df.annotation_property("http://ex.org/ap");

    let ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.object_property_domain(p.clone(), a.clone()),
        df.data_property_range(dp.clone(), df.data_range_integer()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(5)),
        df.same_individual(vec![i.clone(), j.clone()]),
        df.transitive_object_property(p.clone()),
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new("http://ex.org/A")),
            property: ap.clone(),
            value: AnnotationValue::Literal(df.literal("label")),
            annotations: vec![],
        }),
        Axiom::DatatypeDefinition(DatatypeDefinitionAxiom {
            id: df.next_id(),
            datatype: IRI::new("http://ex.org/customDT"),
            data_range: DataRange::Datatype(xsd_iri("string")),
            annotations: vec![],
        }),
    ]);

    let sig = ont.signature().expect("signature() must succeed");

    assert!(
        sig.classes
            .iter()
            .any(|c| c.iri.as_str() == "http://ex.org/A")
    );
    assert!(
        sig.classes
            .iter()
            .any(|c| c.iri.as_str() == "http://ex.org/B")
    );
    assert!(
        sig.object_properties
            .iter()
            .any(|p| p.iri.as_str() == "http://ex.org/p")
    );
    assert!(
        sig.object_properties
            .iter()
            .any(|p| p.iri.as_str() == "http://www.w3.org/2002/07/owl#topObjectProperty")
    );
    assert!(
        sig.data_properties
            .iter()
            .any(|d| d.iri.as_str() == "http://ex.org/dp")
    );
    assert!(
        sig.individuals
            .iter()
            .any(|ind| matches!(ind, Individual::Named(n) if n.iri.as_str() == "http://ex.org/i"))
    );
    assert!(
        sig.individuals
            .iter()
            .any(|ind| matches!(ind, Individual::Named(n) if n.iri.as_str() == "http://ex.org/j"))
    );
    assert!(
        sig.annotation_properties
            .iter()
            .any(|ap| ap.iri.as_str() == "http://ex.org/ap")
    );
    // Built-in datatype used in a data range must appear in the signature.
    assert!(
        sig.datatypes
            .iter()
            .any(|d| d.iri.as_str() == "http://www.w3.org/2001/XMLSchema#integer")
    );
    // The datatype being defined and the range datatype both appear.
    assert!(
        sig.datatypes
            .iter()
            .any(|d| d.iri.as_str() == "http://ex.org/customDT")
    );
    assert!(
        sig.datatypes
            .iter()
            .any(|d| d.iri.as_str() == "http://www.w3.org/2001/XMLSchema#string")
    );
}

/// (d) No `horned_owl` types leak from `DatatypeDefinitionAxiom` /
/// `DataRange` / `FacetRestriction`. These are compile-time assertions: if the
/// field types ever revert to horned_owl types this file will not compile.
#[test]
fn test_no_horned_owl_types_leak() {
    fn check_data_range(_: &oxidowl::ontology::DataRange) {}
    fn check_facet_restriction(_: &oxidowl::ontology::FacetRestriction) {}

    fn check_datatype_definition(ax: &DatatypeDefinitionAxiom) {
        let _datatype: &oxidowl::ontology::IRI = &ax.datatype;
        let _range: &oxidowl::ontology::DataRange = &ax.data_range;
        let _anns: &Vec<oxidowl::ontology::Annotation> = &ax.annotations;
    }

    let df = DF::new();
    let ax = DatatypeDefinitionAxiom {
        id: df.next_id(),
        datatype: IRI::new("http://ex.org/dt"),
        data_range: DataRange::Datatype(xsd_iri("integer")),
        annotations: vec![],
    };
    check_datatype_definition(&ax);
    check_data_range(&ax.data_range);

    // `DataOneOf` must hold native `ontology::Literal`s.
    let one_of = DataRange::DataOneOf(vec![df.int_literal(1), df.int_literal(2)]);
    match &one_of {
        DataRange::DataOneOf(literals) => {
            check_literals_native(literals);
        }
        _ => unreachable!(),
    }

    // `FacetRestriction` must hold a native IRI facet + native Literal value.
    let fr = FacetRestriction {
        facet: xsd_iri("minInclusive"),
        value: df.int_literal(0),
    };
    check_facet_restriction(&fr);
    let _facet: &oxidowl::ontology::IRI = &fr.facet;
    let _value: &oxidowl::ontology::Literal = &fr.value;
}

fn check_literals_native(literals: &[oxidowl::ontology::Literal]) {
    assert!(literals.iter().any(|l| l.value == "1"));
}
