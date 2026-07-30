#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::transform::nnf::NNFConverter;
use oxidowl::transform::expressivity::DLExpressivityChecker;

// ══════════════════════════════════════════════════════════════════════════════
// NNF Converter Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn nnf_positive_class() {
    let conv = NNFConverter;
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let result = conv.to_nnf(&a);
    assert_eq!(result, a);
}

#[test]
fn nnf_double_negation() {
    let conv = NNFConverter;
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let not_not_a = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectComplementOf(Box::new(a.clone())),
    ));
    let result = conv.to_nnf(&not_not_a);
    assert_eq!(result, a);
}

#[test]
fn nnf_de_morgan_intersection() {
    let conv = NNFConverter;
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    // ¬(A ⊓ B) → ¬A ⊔ ¬B
    let original = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectIntersectionOf(vec![a.clone(), b.clone()])),
    );
    let result = conv.to_nnf(&original);
    assert!(matches!(result, ClassExpression::ObjectUnionOf(_)));
}

#[test]
fn nnf_de_morgan_union() {
    let conv = NNFConverter;
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    // ¬(A ⊔ B) → ¬A ⊓ ¬B
    let original = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectUnionOf(vec![a.clone(), b.clone()])),
    );
    let result = conv.to_nnf(&original);
    assert!(matches!(result, ClassExpression::ObjectIntersectionOf(_)));
}

#[test]
fn nnf_some_values_from_complement() {
    let conv = NNFConverter;
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    // ¬(∃P.B) → ∀P.¬B
    let original = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectSomeValuesFrom {
            property: p,
            filler: Box::new(b),
        },
    ));
    let result = conv.to_nnf(&original);
    assert!(matches!(result, ClassExpression::ObjectAllValuesFrom { .. }));
}

#[test]
fn nnf_all_values_from_complement() {
    let conv = NNFConverter;
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    // ¬(∀P.B) → ∃P.¬B
    let original = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectAllValuesFrom {
            property: p,
            filler: Box::new(b),
        },
    ));
    let result = conv.to_nnf(&original);
    assert!(matches!(result, ClassExpression::ObjectSomeValuesFrom { .. }));
}

#[test]
fn nnf_nested_quantifier() {
    let conv = NNFConverter;
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    // ¬(∃P.(A ⊓ B))
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    let inner = ClassExpression::ObjectIntersectionOf(vec![a, b]);
    let original = ClassExpression::ObjectComplementOf(Box::new(
        ClassExpression::ObjectSomeValuesFrom {
            property: p,
            filler: Box::new(inner),
        },
    ));
    let result = conv.to_nnf(&original);
    assert!(matches!(result, ClassExpression::ObjectAllValuesFrom { .. }));
}

// ══════════════════════════════════════════════════════════════════════════════
// Built-in Entity Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn builtin_owl_thing() {
    let thing = Class::thing();
    assert!(thing.is_thing());
    assert!(!thing.is_nothing());
    assert_eq!(thing.iri.as_str(), "http://www.w3.org/2002/07/owl#Thing");
    assert!(IRI::owl_thing().is_owl_thing());
}

#[test]
fn builtin_owl_nothing() {
    let nothing = Class::nothing();
    assert!(nothing.is_nothing());
    assert!(!nothing.is_thing());
    assert!(IRI::owl_nothing().is_owl_nothing());
}

#[test]
fn builtin_reserved_vocabulary() {
    assert!(IRI::new("http://www.w3.org/2002/07/owl#Thing").is_reserved_vocabulary());
    assert!(IRI::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").is_reserved_vocabulary());
    assert!(!IRI::new("http://example.org/MyClass").is_reserved_vocabulary());
}

#[test]
fn builtin_annotation_property_detection() {
    assert!(IRI::new("http://www.w3.org/2000/01/rdf-schema#label").is_builtin_annotation_property());
    assert!(IRI::new("http://www.w3.org/2000/01/rdf-schema#comment").is_builtin_annotation_property());
    assert!(!IRI::new("http://example.org/myProp").is_builtin_annotation_property());
}

// ══════════════════════════════════════════════════════════════════════════════
// Class Expression Construction Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn class_expression_named_class() {
    let ce = ClassExpression::class(IRI::new("http://ex.org/A"));
    assert!(ce.is_named_class());
    assert_eq!(ce.as_class().unwrap().iri.as_str(), "http://ex.org/A");
}

#[test]
fn class_expression_intersection() {
    let a = ClassExpression::class(IRI::new("http://ex.org/A"));
    let b = ClassExpression::class(IRI::new("http://ex.org/B"));
    let ce = ClassExpression::intersection_of(vec![a.clone(), b.clone()]);
    assert!(matches!(ce, ClassExpression::ObjectIntersectionOf(_)));
    if let ClassExpression::ObjectIntersectionOf(vec) = &ce {
        assert_eq!(vec.len(), 2);
    }
}

#[test]
fn class_expression_union() {
    let a = ClassExpression::class(IRI::new("http://ex.org/A"));
    let b = ClassExpression::class(IRI::new("http://ex.org/B"));
    let ce = ClassExpression::union_of(vec![a, b]);
    assert!(matches!(ce, ClassExpression::ObjectUnionOf(_)));
}

#[test]
fn class_expression_complement() {
    let a = ClassExpression::class(IRI::new("http://ex.org/A"));
    let ce = ClassExpression::complement_of(a);
    assert!(matches!(ce, ClassExpression::ObjectComplementOf(_)));
}

#[test]
fn class_expression_some_values_from() {
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let b = ClassExpression::class(IRI::new("http://ex.org/B"));
    let ce = ClassExpression::some_values_from(p, b);
    assert!(matches!(ce, ClassExpression::ObjectSomeValuesFrom { .. }));
}

#[test]
fn class_expression_thing_and_nothing() {
    let thing = ClassExpression::thing();
    let nothing = ClassExpression::nothing();
    assert!(thing.as_class().unwrap().is_thing());
    assert!(nothing.as_class().unwrap().is_nothing());
}

#[test]
fn class_expression_has_self() {
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let expr = ClassExpression::ObjectHasSelf { property: p };
    assert!(matches!(expr, ClassExpression::ObjectHasSelf { .. }));
}

#[test]
fn class_expression_one_of() {
    let i = Individual::Named(NamedIndividual { iri: IRI::new("http://ex.org/i") });
    let j = Individual::Named(NamedIndividual { iri: IRI::new("http://ex.org/j") });
    let expr = ClassExpression::ObjectOneOf(vec![i, j]);
    assert!(matches!(expr, ClassExpression::ObjectOneOf(_)));
}

#[test]
fn class_expression_cardinality_variants() {
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let b = ClassExpression::class(IRI::new("http://ex.org/B"));
    let min = ClassExpression::ObjectMinCardinality {
        property: p.clone(),
        cardinality: 2,
        filler: Box::new(b.clone()),
    };
    let max = ClassExpression::ObjectMaxCardinality {
        property: p.clone(),
        cardinality: 5,
        filler: Box::new(b.clone()),
    };
    let exact = ClassExpression::ObjectExactCardinality {
        property: p,
        cardinality: 3,
        filler: Box::new(b),
    };
    assert!(matches!(min, ClassExpression::ObjectMinCardinality { .. }));
    assert!(matches!(max, ClassExpression::ObjectMaxCardinality { .. }));
    assert!(matches!(exact, ClassExpression::ObjectExactCardinality { .. }));
}

#[test]
fn class_expression_is_complex() {
    let named = ClassExpression::class(IRI::new("http://ex.org/A"));
    assert!(!named.is_complex_class_expression());
    let complement = ClassExpression::complement_of(named);
    assert!(complement.is_complex_class_expression());
}

// ══════════════════════════════════════════════════════════════════════════════
// DL Expressivity Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn expressivity_simple_ontology() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&onto);
    let name = expr.to_name();
    assert!(!name.is_empty());
    assert!(!expr.has_complement);
    assert!(!expr.has_union);
}

#[test]
fn expressivity_with_existential() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty { iri: IRI::new("http://ex.org/P") }
    );
    let svf = ClassExpression::ObjectSomeValuesFrom {
        property: p,
        filler: Box::new(b),
    };
    let mut o = Ontology::new();
    o.add_axiom(df.sub_class_of(a, svf));
    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&o);
    assert!(expr.has_existential);
}

#[test]
fn expressivity_with_complement() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let not_b = ClassExpression::ObjectComplementOf(Box::new(b));
    let mut o = Ontology::new();
    o.add_axiom(df.sub_class_of(a, not_b));
    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&o);
    assert!(expr.has_complement);
}
