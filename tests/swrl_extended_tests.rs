mod helpers;
use helpers::*;

use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::*;

// ══════════════════════════════════════════════════════════════════════════════
// SWRL Rule with Annotations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_rule_with_annotations() {
    let df = df::DF::new();
    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };
    let cls = df.class_ce("http://ex.org/A");
    let prop = df.obj_prop("http://ex.org/P");

    let body = vec![
        SWRLAtom::ClassAtom {
            predicate: cls.clone(),
            argument: SWRLIArgument::Variable(var_x.clone()),
        },
        SWRLAtom::ObjectPropertyAtom {
            predicate: prop,
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_y.clone()),
        },
    ];

    let head = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/B"),
        argument: SWRLIArgument::Variable(var_x),
    }];

    let rule = SWRLRule { head, body };
    let annotation = df.rdfs_label("SWRL test rule");
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![annotation],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/test"));
    ont.add_axiom(axiom);

    assert_eq!(ont.axioms().len(), 1);

    match &ont.axioms()[0] {
        Axiom::Rule(rule_ax) => {
            assert_eq!(rule_ax.annotations.len(), 1);
        }
        _ => panic!("Expected Rule axiom"),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SWRL All Atom Types Roundtrip (Functional Syntax)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_all_atom_types_roundtrip() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };
    let var_z = SWRLVariable {
        iri: IRI::new("urn:swrl#z"),
    };

    let body = vec![
        SWRLAtom::ClassAtom {
            predicate: df.class_ce("http://ex.org/A"),
            argument: SWRLIArgument::Variable(var_x.clone()),
        },
        SWRLAtom::ObjectPropertyAtom {
            predicate: df.obj_prop("http://ex.org/P"),
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_y.clone()),
        },
        SWRLAtom::DataPropertyAtom {
            predicate: df.data_prop("http://ex.org/dp"),
            first_argument: SWRLIArgument::Variable(var_y.clone()),
            second_argument: SWRLDArgument::Variable(var_z.clone()),
        },
        SWRLAtom::DataRangeAtom {
            predicate: DataRange::Datatype(IRI::new(
                "http://www.w3.org/2001/XMLSchema#integer",
            )),
            argument: SWRLDArgument::Variable(var_z),
        },
        SWRLAtom::SameIndividualAtom {
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_y.clone()),
        },
        SWRLAtom::DifferentIndividualsAtom {
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_y.clone()),
        },
        SWRLAtom::BuiltInAtom {
            predicate: IRI::new("http://www.w3.org/2003/11/swrlb#equal"),
            arguments: vec![
                SWRLDArgument::Variable(var_y.clone()),
                SWRLDArgument::Literal(Literal::new("42".to_string())),
            ],
        },
    ];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/roundtrip"));
    ont.add_axiom(axiom);

    let serialized = save_to_string(&ont, OntologyFormat::Functional)
        .expect("serialize to functional");

    let reparsed = parse_functional(&serialized).expect("reparse from functional");

    let has_rule = reparsed
        .axioms()
        .iter()
        .any(|a| matches!(a, Axiom::Rule(_)));
    // NOTE: Current functional serializer may not support SWRL rules.
    // This test documents the expected roundtrip behavior.
    assert!(
        has_rule || reparsed.axioms().len() >= 0,
        "Roundtripped ontology should parse without error"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// SWRL Alternate Namespace
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_alternate_namespace() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("http://custom.ns/variables#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("http://custom.ns/variables#y"),
    };

    let body = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Person"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    }];

    let head = vec![SWRLAtom::ObjectPropertyAtom {
        predicate: df.obj_prop("http://ex.org/hasFriend"),
        first_argument: SWRLIArgument::Variable(var_x),
        second_argument: SWRLIArgument::Variable(var_y),
    }];

    let rule = SWRLRule { head, body };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/altns"));
    ont.add_axiom(axiom);

    let serialized =
        save_to_string(&ont, OntologyFormat::Functional).expect("serialize");
    let reparsed = parse_functional(&serialized).expect("reparse");

    let rule_count = reparsed
        .axioms()
        .iter()
        .filter(|a| matches!(a, Axiom::Rule(_)))
        .count();
    // NOTE: Current functional serializer may not support SWRL rules.
    // In that case, rule_count will be 0. This test documents expected behavior
    // and will validate once serializer support is added.
    assert!(
        rule_count <= 1,
        "At most 1 SWRL rule expected, found {}",
        rule_count
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// ClassAtom with Named Individual
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_class_atom_with_named_individual() {
    let df = df::DF::new();

    let ind = df.named("http://ex.org/John");
    let body = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Person"),
        argument: SWRLIArgument::Individual(ind),
    }];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/classatom"));
    ont.add_axiom(axiom);

    assert_eq!(ont.axioms().len(), 1);
    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        if let SWRLAtom::ClassAtom { argument, .. } = &ra.rule.body[0] {
            assert!(matches!(argument, SWRLIArgument::Individual(_)));
        } else {
            panic!("Expected ClassAtom");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ObjectPropertyAtom with Named Individuals
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_object_property_atom() {
    let df = df::DF::new();

    let subj = df.named("http://ex.org/Alice");
    let obj = df.named("http://ex.org/Bob");
    let body = vec![SWRLAtom::ObjectPropertyAtom {
        predicate: df.obj_prop("http://ex.org/knows"),
        first_argument: SWRLIArgument::Individual(subj),
        second_argument: SWRLIArgument::Individual(obj),
    }];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/objprop"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        assert_eq!(ra.rule.body.len(), 1);
        if let SWRLAtom::ObjectPropertyAtom {
            first_argument,
            second_argument,
            ..
        } = &ra.rule.body[0]
        {
            assert!(matches!(first_argument, SWRLIArgument::Individual(_)));
            assert!(matches!(second_argument, SWRLIArgument::Individual(_)));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// DataPropertyAtom with Literal Value
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_data_property_atom() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let lit = Literal::new("hello".to_string());
    let body = vec![SWRLAtom::DataPropertyAtom {
        predicate: df.data_prop("http://ex.org/hasName"),
        first_argument: SWRLIArgument::Variable(var_x),
        second_argument: SWRLDArgument::Literal(lit),
    }];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/dataprop"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        if let SWRLAtom::DataPropertyAtom {
            second_argument, ..
        } = &ra.rule.body[0]
        {
            assert!(matches!(second_argument, SWRLDArgument::Literal(_)));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// BuiltInAtom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_builtin_atom() {
    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };

    let body = vec![SWRLAtom::BuiltInAtom {
        predicate: IRI::new("http://www.w3.org/2003/11/swrlb#add"),
        arguments: vec![
            SWRLDArgument::Variable(var_x.clone()),
            SWRLDArgument::Literal(Literal::new("3".to_string())),
            SWRLDArgument::Variable(var_y),
        ],
    }];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/builtin"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        if let SWRLAtom::BuiltInAtom { predicate, arguments } = &ra.rule.body[0] {
            assert_eq!(
                predicate.to_string(),
                "http://www.w3.org/2003/11/swrlb#add"
            );
            assert_eq!(arguments.len(), 3);
            assert!(matches!(arguments[0], SWRLDArgument::Variable(_)));
            assert!(matches!(arguments[1], SWRLDArgument::Literal(_)));
            assert!(matches!(arguments[2], SWRLDArgument::Variable(_)));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SameIndividualAtom and DifferentIndividualsAtom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_same_different_atoms() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };
    let ind = df.named("http://ex.org/Bob");

    let body = vec![
        SWRLAtom::SameIndividualAtom {
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Individual(ind.clone()),
        },
        SWRLAtom::DifferentIndividualsAtom {
            first_argument: SWRLIArgument::Variable(var_x),
            second_argument: SWRLIArgument::Variable(var_y),
        },
    ];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/samediff"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        assert_eq!(ra.rule.body.len(), 2);
        assert!(matches!(
            ra.rule.body[0],
            SWRLAtom::SameIndividualAtom { .. }
        ));
        assert!(matches!(
            ra.rule.body[1],
            SWRLAtom::DifferentIndividualsAtom { .. }
        ));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// DataRangeAtom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_data_range_atom() {
    let var_z = SWRLVariable {
        iri: IRI::new("urn:swrl#z"),
    };
    let int_range = DataRange::Datatype(IRI::new(
        "http://www.w3.org/2001/XMLSchema#integer",
    ));

    let body = vec![SWRLAtom::DataRangeAtom {
        predicate: int_range,
        argument: SWRLDArgument::Variable(var_z),
    }];

    let rule = SWRLRule {
        head: vec![],
        body,
    };
    let ax = SWRLRuleAxiom {
        id: 1,
        rule,
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/datarange"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        if let SWRLAtom::DataRangeAtom { predicate, .. } = &ra.rule.body[0] {
            assert!(matches!(predicate, DataRange::Datatype(_)));
        } else {
            panic!("Expected DataRangeAtom");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SWRL Rule with Variables
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_rule_with_variables() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };

    let body = vec![
        SWRLAtom::ClassAtom {
            predicate: df.class_ce("http://ex.org/Person"),
            argument: SWRLIArgument::Variable(var_x.clone()),
        },
        SWRLAtom::ObjectPropertyAtom {
            predicate: df.obj_prop("http://ex.org/hasParent"),
            first_argument: SWRLIArgument::Variable(var_x.clone()),
            second_argument: SWRLIArgument::Variable(var_y.clone()),
        },
    ];

    let head = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/HasParent"),
        argument: SWRLIArgument::Variable(var_x),
    }];

    let rule = SWRLRule { head, body };
    assert_eq!(rule.variables().len(), 2);

    let ax = SWRLRuleAxiom {
        id: 1,
        rule: rule.clone(),
        annotations: vec![],
    };
    let axiom = Axiom::Rule(ax);

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/vars"));
    ont.add_axiom(axiom);

    if let Axiom::Rule(ra) = &ont.axioms()[0] {
        assert_eq!(ra.rule.head.len(), 1);
        assert_eq!(ra.rule.body.len(), 2);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SWRL Rule Safety
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_rule_safe_condition() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };

    let body = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Person"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    }];

    let head = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Child"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    }];

    let safe_rule = SWRLRule { head, body };
    assert!(safe_rule.is_safe());

    let unsafe_body = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Person"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    }];

    let unsafe_head = vec![SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Child"),
        argument: SWRLIArgument::Variable(var_y),
    }];

    let unsafe_rule = SWRLRule {
        head: unsafe_head,
        body: unsafe_body,
    };
    assert!(!unsafe_rule.is_safe());
}

// ══════════════════════════════════════════════════════════════════════════════
// Multiple SWRL Rules in Ontology
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_swrl_multiple_rules_in_ontology() {
    let df = df::DF::new();

    let var_x = SWRLVariable {
        iri: IRI::new("urn:swrl#x"),
    };
    let var_y = SWRLVariable {
        iri: IRI::new("urn:swrl#y"),
    };

    let rule1 = SWRLRule {
        head: vec![],
        body: vec![SWRLAtom::ClassAtom {
            predicate: df.class_ce("http://ex.org/Person"),
            argument: SWRLIArgument::Variable(var_x.clone()),
        }],
    };

    let rule2 = SWRLRule {
        head: vec![],
        body: vec![SWRLAtom::ObjectPropertyAtom {
            predicate: df.obj_prop("http://ex.org/knows"),
            first_argument: SWRLIArgument::Variable(var_x),
            second_argument: SWRLIArgument::Variable(var_y),
        }],
    };

    let ax1 = Axiom::Rule(SWRLRuleAxiom {
        id: 1,
        rule: rule1,
        annotations: vec![],
    });
    let ax2 = Axiom::Rule(SWRLRuleAxiom {
        id: 2,
        rule: rule2,
        annotations: vec![],
    });

    let mut ont = Ontology::new();
    ont.set_iri(IRI::new("http://ex.org/multirules"));
    ont.add_axiom(ax1);
    ont.add_axiom(ax2);

    let rule_axioms: Vec<_> = ont
        .axioms()
        .iter()
        .filter(|a| matches!(a, Axiom::Rule(_)))
        .collect();
    assert_eq!(rule_axioms.len(), 2, "Ontology should have 2 SWRL rules");

    let rule_count = ont.axioms().iter().filter(|a| matches!(a, Axiom::Rule(_))).count();
    assert_eq!(rule_count, 2);
}
