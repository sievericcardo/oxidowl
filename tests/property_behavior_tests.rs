#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

const EX: &str = "http://example.org/";

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.13 Object Property Tests
// ══════════════════════════════════════════════════════════════════════════════

/// test_object_property_subsumption_chain: P ⊑ Q, Q ⊑ R — verify transitive sub-property
#[test]
fn test_object_property_subsumption_chain() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));
    let q = df.obj_prop(&ex("Q"));
    let r = df.obj_prop(&ex("R"));

    let ax_pq = df.sub_object_property_of(p.clone(), q.clone());
    let ax_qr = df.sub_object_property_of(q.clone(), r.clone());

    match &ax_pq {
        Axiom::SubObjectPropertyOf(a) => {
            assert_eq!(a.sub_property, p);
            assert_eq!(a.super_property, q);
        }
        _ => panic!("Expected SubObjectPropertyOf"),
    }
    match &ax_qr {
        Axiom::SubObjectPropertyOf(a) => {
            assert_eq!(a.sub_property, q);
            assert_eq!(a.super_property, r);
        }
        _ => panic!("Expected SubObjectPropertyOf"),
    }

    // Build ontology with the chain and verify both axioms exist
    let mut o = Ontology::new();
    o.add_axiom(ax_pq);
    o.add_axiom(ax_qr);
    df.auto_declare(&mut o);

    let axioms: Vec<_> = o.axioms().to_vec();
    let sub_count = axioms
        .iter()
        .filter(|ax| matches!(ax, Axiom::SubObjectPropertyOf(_)))
        .count();
    assert_eq!(sub_count, 2, "Should have 2 SubObjectPropertyOf axioms in the chain");
}

/// test_object_property_inverse: InverseObjectProperties(P, Q) — verify structure
#[test]
fn test_object_property_inverse() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));
    let q = df.obj_prop(&ex("Q"));

    let ax = df.inverse_object_properties(p.clone(), q.clone());

    match &ax {
        Axiom::InverseObjectProperties(a) => {
            assert_eq!(a.property1, p);
            assert_eq!(a.property2, q);
        }
        _ => panic!("Expected InverseObjectProperties"),
    }

    // Verify the inverse expression is a valid ObjectPropertyExpression
    let inv_q = ObjectPropertyExpression::InverseObjectProperty(ObjectProperty {
        iri: IRI::new(&ex("Q")),
    });
    assert!(inv_q.is_inverse());
    assert!(!inv_q.is_property_chain());
}

/// test_property_chain: R ∘ S ⊑ T — verify property chain semantics
#[test]
fn test_property_chain() {
    let df = DF::new();
    let r = ObjectProperty {
        iri: IRI::new(&ex("R")),
    };
    let s = ObjectProperty {
        iri: IRI::new(&ex("S")),
    };
    let t = df.obj_prop(&ex("T"));

    let chain = ObjectPropertyExpression::PropertyChain(vec![
        ObjectPropertyExpression::ObjectProperty(r.clone()),
        ObjectPropertyExpression::ObjectProperty(s.clone()),
    ]);

    assert!(chain.is_property_chain());
    assert_eq!(chain.chain_length(), 2);

    let ax = Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
        id: df.next_id(),
        sub_property: chain.clone(),
        super_property: t.clone(),
        annotations: vec![],
    });

    match &ax {
        Axiom::SubObjectPropertyOf(a) => {
            assert_eq!(a.super_property, t);
            assert!(a.sub_property.is_property_chain());
            assert_eq!(a.sub_property.chain_length(), 2);
        }
        _ => panic!("Expected SubObjectPropertyOf"),
    }

    // Build ontology and verify structure survives
    let mut o = Ontology::new();
    o.add_axiom(ax);
    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 1);
    if let Axiom::SubObjectPropertyOf(sc) = &axioms[0] {
        assert!(sc.sub_property.is_property_chain());
        assert_eq!(sc.sub_property.chain_length(), 2);
    }
}

/// test_equivalent_object_properties: EquivalentObjectProperties(P, Q) — verify equivalence
#[test]
fn test_equivalent_object_properties() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));
    let q = df.obj_prop(&ex("Q"));

    let ax = df.equivalent_object_properties(vec![p.clone(), q.clone()]);

    match &ax {
        Axiom::EquivalentObjectProperties(a) => {
            assert_eq!(a.properties.len(), 2);
            assert!(a.properties.contains(&p));
            assert!(a.properties.contains(&q));
        }
        _ => panic!("Expected EquivalentObjectProperties"),
    }

    // Three equivalent properties
    let r = df.obj_prop(&ex("R"));
    let ax3 = df.equivalent_object_properties(vec![p.clone(), q.clone(), r.clone()]);

    match &ax3 {
        Axiom::EquivalentObjectProperties(a) => {
            assert_eq!(a.properties.len(), 3);
        }
        _ => panic!("Expected EquivalentObjectProperties with 3 elements"),
    }
}

/// test_disjoint_object_properties: DisjointObjectProperties(P, Q) — verify disjointness
#[test]
fn test_disjoint_object_properties() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));
    let q = df.obj_prop(&ex("Q"));

    let ax = df.disjoint_object_properties(vec![p.clone(), q.clone()]);

    match &ax {
        Axiom::DisjointObjectProperties(a) => {
            assert_eq!(a.properties.len(), 2);
            assert!(a.properties.contains(&p));
            assert!(a.properties.contains(&q));
        }
        _ => panic!("Expected DisjointObjectProperties"),
    }

    // Verify ontology persistence
    let mut o = Ontology::new();
    o.add_axiom(ax);
    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 1);
    assert!(matches!(&axioms[0], Axiom::DisjointObjectProperties(_)));
}

/// test_all_object_property_characteristics: All 7 characteristics
/// (Functional, Transitive, Symmetric, Asymmetric, Reflexive, Irreflexive, InverseFunctional)
#[test]
fn test_all_object_property_characteristics() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));

    // 1. Functional
    let func = df.functional_object_property(p.clone());
    assert!(matches!(func, Axiom::FunctionalObjectProperty(_)));

    // 2. Transitive
    let trans = df.transitive_object_property(p.clone());
    assert!(matches!(trans, Axiom::TransitiveObjectProperty(_)));

    // 3. Symmetric
    let sym = df.symmetric_object_property(p.clone());
    assert!(matches!(sym, Axiom::SymmetricObjectProperty(_)));

    // 4. Asymmetric
    let asym = df.asymmetric_object_property(p.clone());
    assert!(matches!(asym, Axiom::AsymmetricObjectProperty(_)));

    // 5. Reflexive
    let refl = df.reflexive_object_property(p.clone());
    assert!(matches!(refl, Axiom::ReflexiveObjectProperty(_)));

    // 6. Irreflexive
    let irrefl = df.irreflexive_object_property(p.clone());
    assert!(matches!(irrefl, Axiom::IrreflexiveObjectProperty(_)));

    // 7. InverseFunctional
    let inv_func = df.inverse_functional_object_property(p.clone());
    assert!(matches!(
        inv_func,
        Axiom::InverseFunctionalObjectProperty(_)
    ));

    // All 7 axioms in an ontology
    let mut o = Ontology::new();
    o.add_axiom(func);
    o.add_axiom(trans);
    o.add_axiom(sym);
    o.add_axiom(asym);
    o.add_axiom(refl);
    o.add_axiom(irrefl);
    o.add_axiom(inv_func);

    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 7, "Should have all 7 characteristic axioms");

    // Count per type
    let count_by_type = |variant: fn(&Axiom) -> bool| -> usize {
        axioms.iter().filter(|ax| variant(ax)).count()
    };
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::FunctionalObjectProperty(_))), 1);
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::TransitiveObjectProperty(_))), 1);
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::SymmetricObjectProperty(_))), 1);
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::AsymmetricObjectProperty(_))), 1);
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::ReflexiveObjectProperty(_))), 1);
    assert_eq!(count_by_type(|ax| matches!(ax, Axiom::IrreflexiveObjectProperty(_))), 1);
    assert_eq!(
        count_by_type(|ax| matches!(ax, Axiom::InverseFunctionalObjectProperty(_))),
        1
    );
}

/// test_object_property_domain_range: ObjectPropertyDomain(P, C), ObjectPropertyRange(P, D)
#[test]
fn test_object_property_domain_range() {
    let df = DF::new();
    let p = df.obj_prop(&ex("P"));
    let c = df.class_ce(&ex("C"));
    let d = df.class_ce(&ex("D"));

    let domain_ax = df.object_property_domain(p.clone(), c.clone());
    let range_ax = df.object_property_range(p.clone(), d.clone());

    match &domain_ax {
        Axiom::ObjectPropertyDomain(ax) => {
            assert_eq!(ax.property, p);
            assert_eq!(ax.domain, c);
        }
        _ => panic!("Expected ObjectPropertyDomain"),
    }
    match &range_ax {
        Axiom::ObjectPropertyRange(ax) => {
            assert_eq!(ax.property, p);
            assert_eq!(ax.range, d);
        }
        _ => panic!("Expected ObjectPropertyRange"),
    }

    // Build ontology
    let mut o = Ontology::new();
    o.add_axiom(domain_ax);
    o.add_axiom(range_ax);
    df.auto_declare(&mut o);

    let axioms: Vec<_> = o.axioms().to_vec();
    let domain_count = axioms
        .iter()
        .filter(|ax| matches!(ax, Axiom::ObjectPropertyDomain(_)))
        .count();
    let range_count = axioms
        .iter()
        .filter(|ax| matches!(ax, Axiom::ObjectPropertyRange(_)))
        .count();
    assert!(domain_count >= 1);
    assert!(range_count >= 1);
}

/// test_inverse_property_expression: ObjectInverseOf(P) in complex expressions
#[test]
fn test_inverse_property_expression() {
    let df = DF::new();
    let inv_p = ObjectPropertyExpression::InverseObjectProperty(ObjectProperty {
        iri: IRI::new(&ex("P")),
    });
    let b = df.class_ce(&ex("B"));

    // ∃P⁻.B
    let expr = ClassExpression::ObjectSomeValuesFrom {
        property: inv_p.clone(),
        filler: Box::new(b),
    };

    match &expr {
        ClassExpression::ObjectSomeValuesFrom { property, .. } => {
            assert!(property.is_inverse());
            assert_eq!(
                property.get_named_property().iri.as_str(),
                &ex("P")
            );
        }
        _ => panic!("Expected ObjectSomeValuesFrom"),
    }

    // Build a subclass axiom using inverse property
    let a = df.class_ce(&ex("A"));
    let ax = Axiom::SubClassOf(SubClassOfAxiom {
        id: df.next_id(),
        subclass: a,
        superclass: expr,
        annotations: vec![],
    });

    match &ax {
        Axiom::SubClassOf(sc) => {
            if let ClassExpression::ObjectSomeValuesFrom { property, .. } = &sc.superclass {
                assert!(property.is_inverse());
            } else {
                panic!("Superclass should be ObjectSomeValuesFrom with inverse property");
            }
        }
        _ => panic!("Expected SubClassOf"),
    }

    // Verify inverse of inverse returns original
    let inv_inv = inv_p.get_inverse();
    assert!(!inv_inv.is_inverse());
    assert!(matches!(
        inv_inv,
        ObjectPropertyExpression::ObjectProperty(_)
    ));
}

/// test_multi_arity_property_chain: 3+ properties in PropertyChain
#[test]
fn test_multi_arity_property_chain() {
    let df = DF::new();
    let p = ObjectProperty {
        iri: IRI::new(&ex("P")),
    };
    let q = ObjectProperty {
        iri: IRI::new(&ex("Q")),
    };
    let r = ObjectProperty {
        iri: IRI::new(&ex("R")),
    };
    let s = ObjectProperty {
        iri: IRI::new(&ex("S")),
    };
    let t = df.obj_prop(&ex("T"));

    // P ∘ Q ∘ R ∘ S ⊑ T  — 4-arity chain
    let chain = ObjectPropertyExpression::PropertyChain(vec![
        ObjectPropertyExpression::ObjectProperty(p),
        ObjectPropertyExpression::ObjectProperty(q),
        ObjectPropertyExpression::ObjectProperty(r),
        ObjectPropertyExpression::ObjectProperty(s),
    ]);

    assert!(chain.is_property_chain());
    assert_eq!(chain.chain_length(), 4);

    let ax = Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
        id: df.next_id(),
        sub_property: chain,
        super_property: t,
        annotations: vec![],
    });

    if let Axiom::SubObjectPropertyOf(sc) = &ax {
        assert_eq!(sc.sub_property.chain_length(), 4);
    }

    // Also test 3-arity
    let p2 = ObjectProperty {
        iri: IRI::new(&ex("P2")),
    };
    let q2 = ObjectProperty {
        iri: IRI::new(&ex("Q2")),
    };
    let r2 = ObjectProperty {
        iri: IRI::new(&ex("R2")),
    };
    let chain3 = ObjectPropertyExpression::PropertyChain(vec![
        ObjectPropertyExpression::ObjectProperty(p2),
        ObjectPropertyExpression::ObjectProperty(q2),
        ObjectPropertyExpression::ObjectProperty(r2),
    ]);
    assert_eq!(chain3.chain_length(), 3);
    assert!(chain3.is_property_chain());
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.12 Data Property Tests
// ══════════════════════════════════════════════════════════════════════════════

/// test_data_property_subsumption: SubDataPropertyOf(DP, DQ)
#[test]
fn test_data_property_subsumption() {
    let df = DF::new();
    let dp = df.data_prop(&ex("dp1"));
    let dq = df.data_prop(&ex("dp2"));

    let ax = df.sub_data_property_of(dp.clone(), dq.clone());

    match &ax {
        Axiom::SubDataPropertyOf(a) => {
            assert_eq!(a.sub_property, dp);
            assert_eq!(a.super_property, dq);
        }
        _ => panic!("Expected SubDataPropertyOf"),
    }

    let mut o = Ontology::new();
    o.add_axiom(ax);
    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 1);
    assert!(matches!(&axioms[0], Axiom::SubDataPropertyOf(_)));
}

/// test_data_property_functional: FunctionalDataProperty(DP)
#[test]
fn test_data_property_functional() {
    let df = DF::new();
    let dp = df.data_prop(&ex("dp"));

    let ax = df.functional_data_property(dp.clone());

    match &ax {
        Axiom::FunctionalDataProperty(a) => {
            assert_eq!(a.property, dp);
        }
        _ => panic!("Expected FunctionalDataProperty"),
    }
    assert!(matches!(ax, Axiom::FunctionalDataProperty(_)));
}

/// test_data_property_domain_range: DataPropertyDomain(DP, C), DataPropertyRange(DP, Integer)
#[test]
fn test_data_property_domain_range() {
    let df = DF::new();
    let dp = df.data_prop(&ex("dp"));
    let c = df.class_ce(&ex("C"));
    let int_range = df.data_range_integer();

    let domain_ax = df.data_property_domain(dp.clone(), c.clone());
    let range_ax = df.data_property_range(dp.clone(), int_range.clone());

    match &domain_ax {
        Axiom::DataPropertyDomain(a) => {
            assert_eq!(a.property, dp);
            assert_eq!(a.domain, c);
        }
        _ => panic!("Expected DataPropertyDomain"),
    }
    match &range_ax {
        Axiom::DataPropertyRange(a) => {
            assert_eq!(a.property, dp);
            assert_eq!(a.range, int_range);
        }
        _ => panic!("Expected DataPropertyRange"),
    }

    // Build ontology
    let mut o = Ontology::new();
    o.add_axiom(domain_ax);
    o.add_axiom(range_ax);
    df.auto_declare(&mut o);

    let axioms: Vec<_> = o.axioms().to_vec();
    let domain_found = axioms
        .iter()
        .any(|ax| matches!(ax, Axiom::DataPropertyDomain(_)));
    let range_found = axioms
        .iter()
        .any(|ax| matches!(ax, Axiom::DataPropertyRange(_)));
    assert!(domain_found);
    assert!(range_found);
}

/// test_equivalent_data_properties: EquivalentDataProperties(DP, DQ)
#[test]
fn test_equivalent_data_properties() {
    let df = DF::new();
    let dp = df.data_prop(&ex("dp1"));
    let dq = df.data_prop(&ex("dp2"));

    let ax = df.equivalent_data_properties(vec![dp.clone(), dq.clone()]);

    match &ax {
        Axiom::EquivalentDataProperties(a) => {
            assert_eq!(a.properties.len(), 2);
            assert!(a.properties.contains(&dp));
            assert!(a.properties.contains(&dq));
        }
        _ => panic!("Expected EquivalentDataProperties"),
    }

    let mut o = Ontology::new();
    o.add_axiom(ax);
    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 1);
    assert!(matches!(&axioms[0], Axiom::EquivalentDataProperties(_)));
}

/// test_disjoint_data_properties: DisjointDataProperties(DP, DQ)
#[test]
fn test_disjoint_data_properties() {
    let df = DF::new();
    let dp = df.data_prop(&ex("dp1"));
    let dq = df.data_prop(&ex("dp2"));

    let ax = df.disjoint_data_properties(vec![dp.clone(), dq.clone()]);

    match &ax {
        Axiom::DisjointDataProperties(a) => {
            assert_eq!(a.properties.len(), 2);
            assert!(a.properties.contains(&dp));
            assert!(a.properties.contains(&dq));
        }
        _ => panic!("Expected DisjointDataProperties"),
    }

    let mut o = Ontology::new();
    o.add_axiom(ax);
    let axioms: Vec<_> = o.axioms().to_vec();
    assert_eq!(axioms.len(), 1);
    assert!(matches!(&axioms[0], Axiom::DisjointDataProperties(_)));
}

/// test_data_property_assertion_values: DataPropertyAssertion(DP, i, "value"), verify retrieval
#[test]
fn test_data_property_assertion_values() {
    let df = DF::new();
    let dp = df.data_prop(&ex("hasName"));
    let individual = df.named(&ex("john"));
    let value = df.literal("John Doe");

    let ax = df.data_property_assertion(dp.clone(), individual.clone(), value.clone());

    match &ax {
        Axiom::DataPropertyAssertion(a) => {
            assert_eq!(a.property, dp);
            assert_eq!(a.individual, individual);
            assert_eq!(a.value, value);
        }
        _ => panic!("Expected DataPropertyAssertion"),
    }

    // Build ontology and verify axiom is retrievable
    let mut o = Ontology::new();
    o.add_axiom(ax);
    df.auto_declare(&mut o);

    let axioms: Vec<_> = o.axioms().to_vec();
    let dp_assertion_count = axioms
        .iter()
        .filter(|ax| matches!(ax, Axiom::DataPropertyAssertion(_)))
        .count();
    assert_eq!(dp_assertion_count, 1, "Should have 1 DataPropertyAssertion");

    // Verify the assertion value
    if let Axiom::DataPropertyAssertion(a) = &axioms[0] {
        assert_eq!(a.value.value, "John Doe");
    } else {
        // DataPropertyAssertion might not be first (auto_declare adds declarations)
        let found = axioms.iter().find_map(|ax| {
            if let Axiom::DataPropertyAssertion(a) = ax {
                Some(a)
            } else {
                None
            }
        });
        assert!(found.is_some(), "DataPropertyAssertion should be in ontology");
        if let Some(a) = found {
            assert_eq!(a.value.value, "John Doe");
        }
    }
}

/// test_annotated_property_chain_roundtrip: Property chain with axiom annotations
#[test]
fn test_annotated_property_chain_roundtrip() {
    let df = DF::new();

    // Build: R ∘ S ⊑ T with a rdfs:comment annotation
    let r = ObjectProperty {
        iri: IRI::new(&ex("R")),
    };
    let s = ObjectProperty {
        iri: IRI::new(&ex("S")),
    };
    let t = df.obj_prop(&ex("T"));

    let chain = ObjectPropertyExpression::PropertyChain(vec![
        ObjectPropertyExpression::ObjectProperty(r),
        ObjectPropertyExpression::ObjectProperty(s),
    ]);

    let comment_ann = df.rdfs_comment("This is a property chain axiom");

    let ax = Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
        id: df.next_id(),
        sub_property: chain.clone(),
        super_property: t.clone(),
        annotations: vec![comment_ann],
    });

    // Verify the annotation is present
    match &ax {
        Axiom::SubObjectPropertyOf(sc) => {
            assert!(sc.sub_property.is_property_chain());
            assert_eq!(sc.sub_property.chain_length(), 2);
            assert_eq!(sc.super_property, t);
            assert_eq!(sc.annotations.len(), 1);
            let ann = &sc.annotations[0];
            assert_eq!(
                ann.property.iri.as_str(),
                "http://www.w3.org/2000/01/rdf-schema#comment"
            );
            if let AnnotationValue::Literal(lit) = &ann.value {
                assert_eq!(lit.value, "This is a property chain axiom");
            } else {
                panic!("Annotation value should be a literal");
            }
        }
        _ => panic!("Expected SubObjectPropertyOf"),
    }

    // Build ontology and verify roundtrip survival
    let mut o = Ontology::new();
    o.add_axiom(ax);
    df.auto_declare(&mut o);

    let axioms: Vec<_> = o.axioms().to_vec();
    let chain_ax = axioms
        .iter()
        .find(|ax| {
            if let Axiom::SubObjectPropertyOf(sc) = ax {
                sc.sub_property.is_property_chain()
            } else {
                false
            }
        })
        .expect("Property chain axiom should be in ontology");

    // Verify annotation survived
    if let Axiom::SubObjectPropertyOf(sc) = chain_ax {
        assert_eq!(sc.annotations.len(), 1);
        assert_eq!(sc.sub_property.chain_length(), 2);
        let ann = &sc.annotations[0];
        assert!(
            ann.property.iri.as_str().contains("comment"),
            "Should have rdfs:comment annotation"
        );
    }
}
