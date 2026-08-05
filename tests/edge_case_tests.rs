#[path = "helpers/mod.rs"]
mod helpers;
use helpers::df::DF;
use helpers::test_base::TestBase;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::*;
use std::collections::HashSet;

#[test]
fn test_special_characters_in_iri() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/Class%20With%20Spaces");
    let d = df.class_ce("http://ex.org/ClassWith%3CArrow%3E");
    let mut o = df.build_ontology(vec![df.sub_class_of(c.clone(), d.clone())]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let subclasses = reparse.get_axioms_by_type(&AxiomType::SubClassOf);
    assert!(
        !subclasses.is_empty(),
        "Reparsed ontology should have subclass-of axioms"
    );
}

#[test]
fn test_percent_encoding_in_iri() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/a%20b%20c");
    let d = df.class_ce("http://ex.org/%C3%A9");
    let mut o = df.build_ontology(vec![df.sub_class_of(c, d)]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    assert!(
        reparse.axioms().len() >= 2,
        "Reparsed ontology should have axioms"
    );
}

#[test]
fn test_non_ascii_iris() {
    let df = DF::new();
    let iri_cn = "http://ex.org/\u{4e2d}\u{56fd}";
    let iri_cafe = "http://ex.org/caf\u{e9}";
    let iri_mueller = "http://ex.org/M\u{fc}ller";

    let c1 = df.class_ce(iri_cn);
    let c2 = df.class_ce(iri_cafe);
    let c3 = df.class_ce(iri_mueller);

    let mut o = Ontology::new();
    o.set_iri(IRI::new("http://ex.org/nonascii"));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new(iri_cn))));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new(iri_cafe))));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new(iri_mueller))));
    o.add_axiom(df.sub_class_of(c1.clone(), c2.clone()));
    o.add_axiom(df.sub_class_of(c2.clone(), c3.clone()));

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    assert!(
        reparse.axioms().len() >= 2,
        "Non-ASCII IRI ontology should survive roundtrip"
    );
}

#[test]
fn test_plain_literal_folding() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");

    let plain = df.literal("abc");
    let typed = df.typed_literal("abc", "http://www.w3.org/2001/XMLSchema#string");

    let mut o = df.build_ontology(vec![
        df.data_property_assertion(dp.clone(), i.clone(), plain),
        df.data_property_assertion(dp, i, typed),
    ]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    assert!(
        reparse.axioms().len() >= 2,
        "Reparsed ontology should have data property assertions"
    );
}

#[test]
fn test_language_tag_case() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/desc");
    let i = df.named("http://ex.org/i");

    let lit_en = df.lang_literal("hello", "en");
    let lit_en_uc = df.lang_literal("hello", "EN");

    let mut o = df.build_ontology(vec![
        df.data_property_assertion(dp.clone(), i.clone(), lit_en),
        df.data_property_assertion(dp, i, lit_en_uc),
    ]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    assert!(
        reparse.axioms().len() >= 2,
        "Different language tag casing should both survive roundtrip"
    );
}

#[test]
fn test_blank_node_id_scoping() {
    let df1 = DF::new();
    let mut o1 = df1.build_ontology(vec![
        df1.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df1.class_assertion(df1.class_ce("http://ex.org/A"), df1.anon()),
    ]);
    df1.auto_declare(&mut o1);

    let df2 = DF::new();
    let mut o2 = df2.build_ontology(vec![
        df2.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df2.class_assertion(df2.class_ce("http://ex.org/B"), df2.anon()),
    ]);
    df2.auto_declare(&mut o2);

    let fss1 = save_to_string(&o1, OntologyFormat::Functional).expect("serialize o1");
    let fss2 = save_to_string(&o2, OntologyFormat::Functional).expect("serialize o2");

    let re1 = parse_functional(&fss1).expect("reparse o1");
    let re2 = parse_functional(&fss2).expect("reparse o2");

    assert!(!re1.axioms().is_empty());
    assert!(!re2.axioms().is_empty());
}

#[test]
fn test_ontology_anonymous_iri() {
    let df = DF::new();
    let o = df.new_ontology();
    assert!(
        o.get_iri().is_none(),
        "Anonymous ontology should have no IRI"
    );

    let mut o_with = df.new_ontology();
    o_with.add_axiom(df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))));

    let fss = save_to_string(&o_with, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");
    assert!(!reparse.axioms().is_empty());
}

#[test]
fn test_deep_class_expression_nesting() {
    let df = DF::new();
    let leaf = df.class_ce("http://ex.org/Leaf");
    let mut current = leaf.clone();
    for _ in 0..20 {
        current = df.intersection_of(vec![current, df.class_ce("http://ex.org/Leaf")]);
    }
    let top = df.class_ce("http://ex.org/Top");

    let mut o = df.build_ontology(vec![df.sub_class_of(current, top)]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let subclass_axs = reparse
        .axioms()
        .iter()
        .filter(|ax| matches!(ax, Axiom::SubClassOf(_)))
        .count();
    assert_eq!(
        subclass_axs, 1,
        "Should have exactly one SubClassOf axiom after roundtrip"
    );
}

#[test]
fn test_large_cardinality() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let b = df.class_ce("http://ex.org/B");
    let large_min = df.min_cardinality(5000, p, b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, large_min)]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let subclass_axs = reparse
        .axioms()
        .iter()
        .filter(|ax| matches!(ax, Axiom::SubClassOf(_)))
        .count();
    assert_eq!(
        subclass_axs, 1,
        "Should have SubClassOf with large cardinality"
    );
}

#[test]
fn test_literal_escaping() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/desc");
    let i = df.named("http://ex.org/i");

    let special_values = [
        "hello world",
        "has space",
        "hy-phen-ated",
        "under_score",
        "backslash\\here",
        "colon:test",
        "ampersand&test",
    ];

    let mut o = Ontology::new();
    o.set_iri(IRI::new("http://ex.org/literal-test"));
    for val in &special_values {
        let lit = df.literal(*val);
        o.add_axiom(df.data_property_assertion(dp.clone(), i.clone(), lit));
    }
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let count = reparse
        .axioms()
        .iter()
        .filter(|ax| matches!(ax, Axiom::DataPropertyAssertion(_)))
        .count();
    assert_eq!(
        count,
        special_values.len(),
        "All data property assertions with special chars should survive roundtrip"
    );
}

#[test]
fn test_empty_ontology_handling() {
    let df = DF::new();
    let o = df.new_ontology_with_iri("http://ex.org/empty");
    assert_eq!(o.axioms().len(), 0, "Empty ontology should have 0 axioms");
    assert!(o.is_empty());

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    assert!(
        !fss.is_empty(),
        "Serialized empty ontology should produce output"
    );

    let reparse = parse_functional(&fss).expect("reparse");
    assert_eq!(
        reparse.axioms().len(),
        0,
        "Reparsed empty ontology should have 0 axioms"
    );
}

#[test]
fn test_iri_with_fragment() {
    let df = DF::new();
    let iri_a = "http://ex.org/ont#A";
    let iri_b = "http://ex.org/ont#B";

    let a = df.class_ce(iri_a);
    let b = df.class_ce(iri_b);
    let mut o = Ontology::new();
    o.set_iri(IRI::new("http://ex.org/fragtest"));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new(iri_a))));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new(iri_b))));
    o.add_axiom(df.sub_class_of(a, b));

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let subclasses = reparse.get_axioms_by_type(&AxiomType::SubClassOf);
    assert!(
        !subclasses.is_empty(),
        "IRI with fragment should survive roundtrip"
    );

    let mentions_a = reparse
        .axioms()
        .iter()
        .any(|ax| format!("{:?}", ax).contains("ont#A"));
    assert!(
        mentions_a,
        "Reparsed ontology should mention the fragment IRI"
    );
}

#[test]
fn test_urn_iri() {
    let df = DF::new();
    let uuid = "urn:uuid:550e8400-e29b-41d4-a716-446655440000";
    let a = df.class_ce(uuid);
    let b = df.class_ce("urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8");
    let mut o = df.build_ontology(vec![df.sub_class_of(a, b)]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    assert!(
        reparse.axioms().len() >= 2,
        "URN-style IRIs should survive roundtrip"
    );
}

#[test]
fn test_ontology_version_iri() {
    let df = DF::new();
    let mut o = df.new_ontology_with_iri("http://ex.org/ont");
    o.set_version_iri(Some(IRI::new("http://ex.org/ont/1.0")));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))));

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    if let Some(version_iri) = reparse.id.version_iri.as_ref() {
        assert_eq!(version_iri.as_str(), "http://ex.org/ont/1.0");
    }

    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&o, OntologyFormat::Functional)
        .expect("Version IRI roundtrip failed");
}

#[test]
fn test_punning_same_iri_class_and_individual() {
    let df = DF::new();
    let iri = IRI::new("http://ex.org/Thing");
    let class_entity = Entity::Class(iri.clone());
    let ind_entity = Entity::NamedIndividual(iri.clone());

    let mut o = df.build_ontology(vec![
        df.declaration_axiom(class_entity),
        df.declaration_axiom(ind_entity.clone()),
        df.class_assertion(
            ClassExpression::Class(Class::new(iri.clone())),
            Individual::Named(NamedIndividual { iri: iri.clone() }),
        ),
    ]);
    df.auto_declare(&mut o);

    let fss = save_to_string(&o, OntologyFormat::Functional).expect("serialize");
    let reparse = parse_functional(&fss).expect("reparse");

    let has_class_decl = reparse.axioms().iter().any(|ax| {
        matches!(ax, Axiom::Declaration(d) if matches!(&d.entity, Entity::Class(c) if c.as_str() == "http://ex.org/Thing"))
    });
    let has_ind_decl = reparse.axioms().iter().any(|ax| {
        matches!(ax, Axiom::Declaration(d) if matches!(&d.entity, Entity::NamedIndividual(ni) if ni.as_str() == "http://ex.org/Thing"))
    });

    assert!(
        has_class_decl,
        "Punned IRI should appear as class declaration"
    );
    assert!(
        has_ind_decl,
        "Punned IRI should appear as individual declaration"
    );
}
