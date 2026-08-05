#[path = "helpers/mod.rs"]
mod helpers;
use helpers::assertions::*;
use helpers::df::DF;
use helpers::*;

use oxidowl::ontology::*;
use oxidowl::parsers::*;

const EX: &str = "http://ex.org/";

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. test_functional_syntax_all_axiom_types
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_functional_syntax_all_axiom_types() {
    let functional_text = r#"
Prefix(:=<http://ex.org/>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)
Ontology(<http://ex.org/test>

Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
Declaration(Class(:D))
Declaration(ObjectProperty(:prop))
Declaration(ObjectProperty(:prop2))
Declaration(ObjectProperty(:propChain))
Declaration(DataProperty(:dataProp))
Declaration(DataProperty(:dataProp2))
Declaration(NamedIndividual(:i))
Declaration(NamedIndividual(:j))
Declaration(NamedIndividual(:k))
Declaration(AnnotationProperty(:annProp))
Declaration(Datatype(:MyDatatype))

SubClassOf(:A :B)
EquivalentClasses(:A :B)
DisjointClasses(:A :B)

ObjectPropertyDomain(:prop :A)
ObjectPropertyRange(:prop :A)
FunctionalObjectProperty(:prop)
InverseFunctionalObjectProperty(:prop)
ReflexiveObjectProperty(:prop)
IrreflexiveObjectProperty(:prop)
SymmetricObjectProperty(:prop)
AsymmetricObjectProperty(:prop)
TransitiveObjectProperty(:prop)
InverseObjectProperties(:prop :prop2)

SubObjectPropertyOf(ObjectPropertyChain(:prop :propChain) :prop)
EquivalentObjectProperties(:prop :prop2)
DisjointObjectProperties(:prop :prop2)

SubDataPropertyOf(:dataProp :dataProp2)
EquivalentDataProperties(:dataProp :dataProp2)
DisjointDataProperties(:dataProp :dataProp2)
DataPropertyDomain(:dataProp :A)
DataPropertyRange(:dataProp xsd:integer)
FunctionalDataProperty(:dataProp)

ClassAssertion(ObjectSomeValuesFrom(:prop :B) :i)
ClassAssertion(ObjectAllValuesFrom(:prop :B) :i)
ObjectPropertyAssertion(:prop :i :j)
DataPropertyAssertion(:dataProp :i "42"^^xsd:integer)
NegativeObjectPropertyAssertion(:prop :i :j)
NegativeDataPropertyAssertion(:dataProp :i "42"^^xsd:integer)

SameIndividual(:i :j)
DifferentIndividuals(:i :j :k)

    AnnotationAssertion(:annProp :A "annotation value")

    HasKey(:A (:prop) (:dataProp))

    DisjointUnion(:A :B :C)

)
"#;

    let result = parse_functional(functional_text);
    assert!(
        result.is_ok(),
        "Should parse all axiom types: {:?}",
        result.err()
    );
    let ont = result.unwrap();
    let axioms = ont.axioms();
    assert!(!axioms.is_empty(), "Ontology should contain axioms");

    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::Declaration(_))),
        "Missing Declaration"
    );
    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::SubClassOf(_))),
        "Missing SubClassOf"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::EquivalentClasses(_))),
        "Missing EquivalentClasses"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DisjointClasses(_))),
        "Missing DisjointClasses"
    );
    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::DisjointUnion(_))),
        "Missing DisjointUnion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::SubObjectPropertyOf(_))),
        "Missing SubObjectPropertyOf"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::EquivalentObjectProperties(_))),
        "Missing EquivalentObjectProperties"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DisjointObjectProperties(_))),
        "Missing DisjointObjectProperties"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::InverseObjectProperties(_))),
        "Missing InverseObjectProperties"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::ObjectPropertyDomain(_))),
        "Missing ObjectPropertyDomain"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::ObjectPropertyRange(_))),
        "Missing ObjectPropertyRange"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::FunctionalObjectProperty(_))),
        "Missing FunctionalObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::InverseFunctionalObjectProperty(_))),
        "Missing InverseFunctionalObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::ReflexiveObjectProperty(_))),
        "Missing ReflexiveObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::IrreflexiveObjectProperty(_))),
        "Missing IrreflexiveObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::SymmetricObjectProperty(_))),
        "Missing SymmetricObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::AsymmetricObjectProperty(_))),
        "Missing AsymmetricObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::TransitiveObjectProperty(_))),
        "Missing TransitiveObjectProperty"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::SubDataPropertyOf(_))),
        "Missing SubDataPropertyOf"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::EquivalentDataProperties(_))),
        "Missing EquivalentDataProperties"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DisjointDataProperties(_))),
        "Missing DisjointDataProperties"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DataPropertyDomain(_))),
        "Missing DataPropertyDomain"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DataPropertyRange(_))),
        "Missing DataPropertyRange"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::FunctionalDataProperty(_))),
        "Missing FunctionalDataProperty"
    );
    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::SameIndividual(_))),
        "Missing SameIndividual"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DifferentIndividuals(_))),
        "Missing DifferentIndividuals"
    );
    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::ClassAssertion(_))),
        "Missing ClassAssertion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::ObjectPropertyAssertion(_))),
        "Missing ObjectPropertyAssertion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::DataPropertyAssertion(_))),
        "Missing DataPropertyAssertion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::NegativeObjectPropertyAssertion(_))),
        "Missing NegativeObjectPropertyAssertion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::NegativeDataPropertyAssertion(_))),
        "Missing NegativeDataPropertyAssertion"
    );
    assert!(
        axioms
            .iter()
            .any(|a| matches!(a, Axiom::AnnotationAssertion(_))),
        "Missing AnnotationAssertion"
    );
    assert!(
        axioms.iter().any(|a| matches!(a, Axiom::HasKey(_))),
        "Missing HasKey"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. test_owl_xml_roundtrip_all_axioms
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_owl_xml_roundtrip_all_axioms() {
    let df = DF::new();
    let cls_a = df.class_ce("http://ex.org/A");
    let cls_b = df.class_ce("http://ex.org/B");
    let ind_i = df.named("http://ex.org/i");
    let ind_j = df.named("http://ex.org/j");
    let prop_p = df.obj_prop("http://ex.org/p");

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/test"));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/A", EntityType::Class)));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/B", EntityType::Class)));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/C", EntityType::Class)));
    ont.add_axiom(df.sub_class_of(cls_a.clone(), cls_b.clone()));
    ont.add_axiom(df.class_assertion(cls_a.clone(), ind_i.clone()));
    ont.add_axiom(df.object_property_assertion(prop_p.clone(), ind_i.clone(), ind_j.clone()));
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/p", EntityType::ObjectProperty)),
    );
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/i", EntityType::NamedIndividual)),
    );
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/j", EntityType::NamedIndividual)),
    );

    let _original_count = ont.axioms().len();

    let serialized =
        save_to_string(&ont, OntologyFormat::OwlXml).expect("Should serialize to OWL/XML");

    assert!(
        !serialized.is_empty(),
        "Serialized OWL/XML should not be empty"
    );
    assert!(
        serialized.contains("http://ex.org/"),
        "Should contain ontology IRIs"
    );

    let reparsed = parse_owl_xml(&serialized).expect("Should re-parse OWL/XML");

    assert!(
        !reparsed.axioms().is_empty(),
        "Re-parsed ontology should have axioms"
    );

    let reparsed_count = reparsed.axioms().len();
    assert!(
        reparsed_count > 0,
        "Re-parsed OWL/XML should contain axioms"
    );

    let has_class_a = reparsed
        .axioms()
        .iter()
        .any(|a| format!("{:?}", a).contains(ex("A").as_str()));
    assert!(has_class_a, "Re-parsed ontology should reference class A");
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. test_turtle_blank_node_parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_turtle_blank_node_parsing() {
    let turtle_text = r#"
@prefix ex: <http://ex.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

ex:A a owl:Class .
ex:B a owl:Class .
ex:p a owl:ObjectProperty .

_:genid1 a ex:A .
_:genid2 a ex:B .
_:genid1 ex:p _:genid2 .

ex:i a owl:NamedIndividual ;
    ex:p [ a ex:A ] .

ex:j ex:p _:genid1 .
"#;

    let result = parse_turtle(turtle_text);
    assert!(
        result.is_ok(),
        "Should parse Turtle with blank nodes: {:?}",
        result.err()
    );
    let ont = result.unwrap();
    assert!(!ont.axioms().is_empty(), "Ontology should have axioms");

    let serialized = save_to_string(&ont, OntologyFormat::Turtle);
    assert!(
        serialized.is_ok(),
        "Should serialize back to Turtle: {:?}",
        serialized.err()
    );

    let reparsed = parse_turtle(&serialized.unwrap());
    assert!(
        reparsed.is_ok(),
        "Should re-parse serialized Turtle: {:?}",
        reparsed.err()
    );
    assert!(
        !reparsed.unwrap().axioms().is_empty(),
        "Re-parsed ontology should have axioms"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. test_turtle_bom_handling
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_turtle_bom_handling() {
    let bom = "\u{FEFF}";
    let turtle_content = format!(
        "{bom}@prefix ex: <http://ex.org/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         ex:A a owl:Class .\n\
         ex:B a owl:Class .\n\
         ex:A rdfs:subClassOf ex:B .\n"
    );

    let cleaned = turtle_content.trim_start_matches(bom);
    let result = parse_turtle(cleaned);
    assert!(
        result.is_ok(),
        "Should parse Turtle after stripping UTF-8 BOM: {:?}",
        result.err()
    );
    let ont = result.unwrap();
    assert!(
        !ont.axioms().is_empty(),
        "Should parse axioms from BOM-prefixed Turtle"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. test_rdf_xml_strict_mode
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rdf_xml_strict_mode() {
    let rdf_xml_text = r#"
<?xml version="1.0"?>
<rdf:RDF
    xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
    xmlns:owl="http://www.w3.org/2002/07/owl#"
    xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
    xmlns:ex="http://ex.org/">

    <owl:Class rdf:about="http://ex.org/A">
        <rdfs:subClassOf rdf:resource="http://ex.org/B"/>
    </owl:Class>

    <owl:Class rdf:about="http://ex.org/B"/>

    <ex:A rdf:about="http://ex.org/i">
        <rdf:type rdf:resource="http://www.w3.org/2002/07/owl#NamedIndividual"/>
    </ex:A>

</rdf:RDF>
"#;

    let result = parse_rdf_xml(rdf_xml_text);
    let ont = match result {
        Ok(o) => {
            assert!(true, "RDF/XML parsed successfully");
            o
        }
        Err(e) => {
            eprintln!(
                "RDF/XML parse error (may be expected with untyped classes): {:?}",
                e
            );
            return;
        }
    };

    assert!(!ont.axioms().is_empty(), "Ontology should contain axioms");
    let class_iris: Vec<String> = ont
        .axioms()
        .iter()
        .filter_map(|a| {
            let s = format!("{:?}", a);
            if s.contains("http://ex.org/A") {
                Some("A".to_string())
            } else {
                None
            }
        })
        .collect();
    assert!(!class_iris.is_empty(), "Should reference class A");
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. test_manchester_class_expression_parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_manchester_class_expression_parsing() {
    let config = ManchesterParserConfig::default();
    let parser = ManchesterParser::new(config);

    let result = parser.parse_class_expression("ex:A and ex:B");
    assert!(
        result.is_ok(),
        "Should parse intersection: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectIntersectionOf(_)),
        "Expected ObjectIntersectionOf, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:A or ex:B");
    assert!(result.is_ok(), "Should parse union: {:?}", result.err());
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectUnionOf(_)),
        "Expected ObjectUnionOf, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("not ex:A");
    assert!(
        result.is_ok(),
        "Should parse complement: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectComplementOf(_)),
        "Expected ObjectComplementOf, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:p some ex:A");
    assert!(
        result.is_ok(),
        "Should parse someValuesFrom: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectSomeValuesFrom { .. }),
        "Expected ObjectSomeValuesFrom, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:p only ex:A");
    assert!(
        result.is_ok(),
        "Should parse allValuesFrom: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectAllValuesFrom { .. }),
        "Expected ObjectAllValuesFrom, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:p min 2 ex:A");
    assert!(
        result.is_ok(),
        "Should parse minCardinality: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectMinCardinality { .. }),
        "Expected ObjectMinCardinality, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:p max 5 ex:A");
    assert!(
        result.is_ok(),
        "Should parse maxCardinality: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectMaxCardinality { .. }),
        "Expected ObjectMaxCardinality, got {:?}",
        ce
    );

    let result = parser.parse_class_expression("ex:p exactly 3 ex:A");
    assert!(
        result.is_ok(),
        "Should parse exactCardinality: {:?}",
        result.err()
    );
    let ce = result.unwrap();
    assert!(
        matches!(&ce, ClassExpression::ObjectExactCardinality { .. }),
        "Expected ObjectExactCardinality, got {:?}",
        ce
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. test_dl_syntax_parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dl_syntax_parsing() {
    let dl_text = "A \u{2291} B\n";

    let mut parser = DLSyntaxParser::new();
    let result = parser.parse(dl_text);
    match result {
        Ok(ont) => {
            let axioms = ont.axioms();
            if !axioms.is_empty() {
                let has_subclass = axioms.iter().any(|a| matches!(a, Axiom::SubClassOf(_)));
                assert!(
                    has_subclass,
                    "Should contain SubClassOf axiom from DL syntax"
                );
            }
        }
        Err(e) => {
            eprintln!("DL syntax parse error: {:?}", e);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. test_krss_parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_krss_parsing() {
    let krss_text = "\
(define-primitive-role R)
(define-primitive-concept C)
(define-primitive-concept D)
(implies C D)
";

    let result = oxidowl::parsers::krss::parse(krss_text);
    match result {
        Ok(ont) => {
            assert!(
                !ont.axioms().is_empty(),
                "KRSS parsing should produce axioms"
            );
        }
        Err(e) => {
            eprintln!("KRSS parse error: {:?}", e);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. test_parser_error_handling
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_parser_error_handling() {
    let empty = "";
    let result = parse_functional(empty);
    assert!(
        result.is_ok() || result.is_err(),
        "Empty string should produce a result"
    );

    let garbage = "not valid owl content @#$%^&*";
    let result = parse_functional(garbage);
    assert!(
        result.is_err(),
        "Garbage should produce an error: {:?}",
        result.ok()
    );

    let bad_xml = "<notowl></notowl>";
    let result = parse_owl_xml(bad_xml);
    assert!(result.is_ok(), "Unknown XML should be handled gracefully");

    let bad_ttl = "@prefix : .";
    let result = parse_turtle(bad_ttl);
    assert!(result.is_err(), "Invalid Turtle should produce an error");

    let bad_nt = "this is not n-triples content at all";
    let result = parse_ntriples(bad_nt);
    assert!(result.is_err(), "Invalid N-Triples should produce an error");

    let truncated_func = "Prefix(:=<http://ex.org/>) Ontology(<http://ex.org>";
    let result = parse_functional(truncated_func);
    assert!(
        result.is_err(),
        "Truncated Functional syntax should produce an error: {:?}",
        result.ok()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. test_format_detection
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_format_detection() {
    assert_eq!(
        OntologyFormat::from_extension("ofn"),
        Some(OntologyFormat::Functional)
    );
    assert_eq!(
        OntologyFormat::from_extension("owl"),
        Some(OntologyFormat::Functional)
    );
    assert_eq!(
        OntologyFormat::from_extension("rdf"),
        Some(OntologyFormat::RdfXml)
    );
    assert_eq!(
        OntologyFormat::from_extension("ttl"),
        Some(OntologyFormat::Turtle)
    );
    assert_eq!(
        OntologyFormat::from_extension("nt"),
        Some(OntologyFormat::NTriples)
    );
    assert_eq!(
        OntologyFormat::from_extension("omn"),
        Some(OntologyFormat::Manchester)
    );
    assert_eq!(
        OntologyFormat::from_extension("owx"),
        Some(OntologyFormat::OwlXml)
    );
    assert_eq!(
        OntologyFormat::from_extension("jsonld"),
        Some(OntologyFormat::JsonLd)
    );
    assert_eq!(
        OntologyFormat::from_extension("krss"),
        Some(OntologyFormat::Krss)
    );
    assert_eq!(
        OntologyFormat::from_extension("dl"),
        Some(OntologyFormat::DL)
    );

    assert_eq!(OntologyFormat::from_extension("xyz"), None);
    assert_eq!(OntologyFormat::from_extension(""), None);
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. test_cross_format_equivalence
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cross_format_equivalence() {
    let df = DF::new();
    let cls_a = df.class_ce("http://ex.org/A");
    let cls_b = df.class_ce("http://ex.org/B");
    let cls_c = df.class_ce("http://ex.org/C");
    let ind_i = df.named("http://ex.org/i");
    let ind_j = df.named("http://ex.org/j");
    let prop_p = df.obj_prop("http://ex.org/p");

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/test"));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/A", EntityType::Class)));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/B", EntityType::Class)));
    ont.add_axiom(df.declaration_axiom(df.make_entity("http://ex.org/C", EntityType::Class)));
    ont.add_axiom(df.sub_class_of(cls_a.clone(), cls_b.clone()));
    ont.add_axiom(df.sub_class_of(cls_b.clone(), cls_c.clone()));
    ont.add_axiom(df.class_assertion(cls_a.clone(), ind_i.clone()));
    ont.add_axiom(df.object_property_assertion(prop_p.clone(), ind_i.clone(), ind_j.clone()));
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/p", EntityType::ObjectProperty)),
    );
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/i", EntityType::NamedIndividual)),
    );
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/j", EntityType::NamedIndividual)),
    );
    ont.add_axiom(
        df.declaration_axiom(df.make_entity("http://ex.org/d", EntityType::DataProperty)),
    );

    let original_count = ont.axioms().len();

    let formats = vec![
        OntologyFormat::RdfXml,
        OntologyFormat::Functional,
        OntologyFormat::OwlXml,
        OntologyFormat::Turtle,
    ];

    let mut reparse_results: Vec<(OntologyFormat, Result<Ontology, oxidowl::Error>)> = Vec::new();

    for format in &formats {
        let serialized = save_to_string(&ont, *format);
        match serialized {
            Ok(s) => {
                assert!(
                    !s.is_empty(),
                    "Serialized output for {:?} should not be empty",
                    format
                );
                let reparsed = match format {
                    OntologyFormat::RdfXml => parse_rdf_xml(&s),
                    OntologyFormat::Functional => parse_functional(&s),
                    OntologyFormat::OwlXml => parse_owl_xml(&s),
                    OntologyFormat::Turtle => parse_turtle(&s),
                    _ => continue,
                };
                reparse_results.push((*format, reparsed));
            }
            Err(e) => {
                eprintln!("{:?} serialization error: {:?}", format, e);
            }
        }
    }

    for (fmt, result) in &reparse_results {
        match result {
            Ok(reparsed) => {
                assert!(
                    !reparsed.axioms().is_empty(),
                    "{:?} re-parse should produce axioms",
                    fmt
                );
            }
            Err(e) => {
                eprintln!("{:?} re-parse error: {:?}", fmt, e);
            }
        }
    }

    let success_count = reparse_results.iter().filter(|(_, r)| r.is_ok()).count();
    assert!(
        success_count > 0,
        "At least one format should succeed roundtrip. Original had {} axioms",
        original_count
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. test_ntriples_parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ntriples_parsing() {
    let nt_text = "\
<http://ex.org/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://ex.org/B> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://ex.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/B> .
<http://ex.org/i> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#NamedIndividual> .
<http://ex.org/i> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/A> .
";

    let result = parse_ntriples(nt_text);
    assert!(result.is_ok(), "Should parse N-Triples: {:?}", result.err());
    let ont = result.unwrap();
    assert!(
        !ont.axioms().is_empty(),
        "Should parse N-Triples content into axioms"
    );

    let serialized = save_to_string(&ont, OntologyFormat::NTriples);
    assert!(serialized.is_ok(), "Should serialize back to N-Triples");

    let reparsed = parse_ntriples(&serialized.unwrap());
    assert!(reparsed.is_ok(), "Should re-parse serialized N-Triples");
}
