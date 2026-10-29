use oxidowl::config::ReasonerConfig;
/// Integration tests for profile-specific reasoners (EL and RL)
use oxidowl::error::Result;
use oxidowl::ontology::axioms::{
    Axiom, ClassAssertionAxiom, ObjectPropertyAssertionAxiom, SubClassOfAxiom,
};
use oxidowl::ontology::individuals::Individual;
use oxidowl::ontology::{Class, ClassExpression, IRI, ObjectPropertyExpression, Ontology};
use oxidowl::profiles::{ELReasoner, RLReasoner};

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ELValidator;
use oxidowl::config::{OWLProfile, ReasoningConfig};
use oxidowl::core::reasoner::{ClassificationResult, Reasoner};
use oxidowl::profiles::ProfileValidator;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn test_el_reasoner_basic() -> Result<()> {
    // Create simple EL ontology
    let mut ontology = Ontology::new();

    // Classes
    let animal = Class::new(IRI::new("http://example.org/Animal"));
    let mammal = Class::new(IRI::new("http://example.org/Mammal"));
    let dog = Class::new(IRI::new("http://example.org/Dog"));

    // Mammal ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(mammal.clone()),
        superclass: ClassExpression::Class(animal.clone()),
        annotations: Vec::new(),
    }));

    // Dog ⊑ Mammal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(dog.clone()),
        superclass: ClassExpression::Class(mammal.clone()),
        annotations: Vec::new(),
    }));

    let config = ReasonerConfig::default();
    let mut reasoner = ELReasoner::new(config);
    reasoner.initialize(&ontology)?;

    let hierarchy = reasoner.classify()?;

    // Dog should be subclass of Animal (transitivity)
    // Check in the hierarchy HashMap
    if let Some(superclasses) = hierarchy
        .hierarchy
        .get(&ClassExpression::Class(dog.clone()))
    {
        assert!(superclasses.contains(&ClassExpression::Class(animal)));
    }

    Ok(())
}

#[test]
fn test_el_reasoner_concurrent() -> Result<()> {
    // Create ontology
    let mut ontology = Ontology::new();

    // Create multiple class hierarchy
    for i in 0..10 {
        let class_a = Class::new(IRI::new(&format!("http://example.org/Class{i}")));
        let class_b = Class::new(IRI::new(&format!("http://example.org/Class{}", i + 1)));

        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 0,
            subclass: ClassExpression::Class(class_a),
            superclass: ClassExpression::Class(class_b),
            annotations: Vec::new(),
        }));
    }

    let mut config = ReasonerConfig::default();
    config
        .performance
        .enable(oxidowl::config::PerformanceFeature::ParallelExpansion);

    let mut reasoner = ELReasoner::new(config);
    reasoner.initialize(&ontology)?;
    let _hierarchy = reasoner.classify()?;

    Ok(())
}

#[test]
fn test_rl_reasoner_basic() -> Result<()> {
    // Create RL ontology
    let mut ontology = Ontology::new();

    // Classes
    let person = Class::new(IRI::new("http://example.org/Person"));
    let animal = Class::new(IRI::new("http://example.org/Animal"));

    // Person ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(person.clone()),
        superclass: ClassExpression::Class(animal.clone()),
        annotations: Vec::new(),
    }));

    // Individual
    let john = Individual::named(IRI::new("http://example.org/john"));

    // john : Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0,
        individual: john.clone(),
        class: ClassExpression::Class(person.clone()),
        annotations: Vec::new(),
    }));

    let config = ReasonerConfig::default();
    let mut reasoner = RLReasoner::new(config);
    reasoner.initialize(&ontology)?;
    reasoner.materialize()?;

    // Check if john is an Animal (should be inferred)
    let is_animal = reasoner.is_instance_of(&john, &ClassExpression::Class(animal))?;
    assert!(is_animal);

    Ok(())
}

#[test]
fn test_rl_reasoner_property_hierarchy() -> Result<()> {
    // Create RL ontology with property hierarchy
    let mut ontology = Ontology::new();

    // Classes
    let person = Class::new(IRI::new("http://example.org/Person"));

    // Properties
    let has_parent = ObjectPropertyExpression::ObjectProperty(oxidowl::ontology::ObjectProperty {
        iri: IRI::new("http://example.org/hasParent"),
    });
    let has_ancestor =
        ObjectPropertyExpression::ObjectProperty(oxidowl::ontology::ObjectProperty {
            iri: IRI::new("http://example.org/hasAncestor"),
        });

    // hasParent ⊑ hasAncestor
    ontology.add_axiom(Axiom::SubObjectPropertyOf(
        oxidowl::ontology::axioms::SubObjectPropertyOfAxiom {
            id: 0,
            sub_property: has_parent.clone(),
            super_property: has_ancestor.clone(),
            annotations: Vec::new(),
        },
    ));

    // Individuals
    let alice = Individual::named(IRI::new("http://example.org/alice"));
    let bob = Individual::named(IRI::new("http://example.org/bob"));

    // alice : Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0,
        individual: alice.clone(),
        class: ClassExpression::Class(person.clone()),
        annotations: Vec::new(),
    }));

    // bob : Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0,
        individual: bob.clone(),
        class: ClassExpression::Class(person.clone()),
        annotations: Vec::new(),
    }));

    // alice hasParent bob
    ontology.add_axiom(Axiom::ObjectPropertyAssertion(
        ObjectPropertyAssertionAxiom {
            id: 0,
            source: alice.clone(),
            target: bob.clone(),
            property: has_parent.clone(),
            annotations: Vec::new(),
        },
    ));

    let config = ReasonerConfig::default();
    let mut reasoner = RLReasoner::new(config);
    reasoner.initialize(&ontology)?;
    reasoner.materialize()?;

    // RL reasoner should have materialized successfully and built a hierarchy
    let result = reasoner.classify()?;
    // Verify hierarchy is not empty - should contain at least the Person class
    assert!(
        !result.hierarchy.is_empty(),
        "Hierarchy should contain classes"
    );
    assert!(
        result
            .hierarchy
            .contains_key(&ClassExpression::Class(person)),
        "Hierarchy should contain Person class"
    );

    Ok(())
}

#[test]
fn test_rl_reasoner_hierarchy_correctness() -> Result<()> {
    // Create RL ontology with class hierarchy
    let mut ontology = Ontology::new();

    // Classes
    let animal = Class::new(IRI::new("http://example.org/Animal"));
    let mammal = Class::new(IRI::new("http://example.org/Mammal"));
    let dog = Class::new(IRI::new("http://example.org/Dog"));

    // Mammal ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(mammal.clone()),
        superclass: ClassExpression::Class(animal.clone()),
        annotations: Vec::new(),
    }));

    // Dog ⊑ Mammal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(dog.clone()),
        superclass: ClassExpression::Class(mammal.clone()),
        annotations: Vec::new(),
    }));

    let config = ReasonerConfig::default();
    let mut reasoner = RLReasoner::new(config);
    reasoner.initialize(&ontology)?;
    reasoner.materialize()?;

    let result = reasoner.classify()?;

    // Verify hierarchy contains all classes
    assert!(
        result
            .hierarchy
            .contains_key(&ClassExpression::Class(animal.clone()))
    );
    assert!(
        result
            .hierarchy
            .contains_key(&ClassExpression::Class(mammal.clone()))
    );
    assert!(
        result
            .hierarchy
            .contains_key(&ClassExpression::Class(dog.clone()))
    );

    // Verify Dog ⊑ Mammal (transitive closure: Dog should have Mammal as superclass)
    let dog_superclasses = result
        .hierarchy
        .get(&ClassExpression::Class(dog.clone()))
        .unwrap();
    assert!(
        dog_superclasses.contains(&ClassExpression::Class(mammal.clone())),
        "Dog should be subclass of Mammal"
    );

    // Verify transitivity: Dog ⊑ Animal
    assert!(
        dog_superclasses.contains(&ClassExpression::Class(animal.clone())),
        "Dog should be subclass of Animal (via transitivity)"
    );

    // Verify reflexivity: Dog ⊑ Dog
    assert!(
        dog_superclasses.contains(&ClassExpression::Class(dog.clone())),
        "Dog should be subclass of itself (reflexivity)"
    );

    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// ORE benchmark regression: EL fast-path routing + fallback
// (`Reasoner::classify` used to propagate `Error::unsupported` for nominals,
//  exiting 1 on an EL-detected ontology containing `ObjectOneOf`.)
// ══════════════════════════════════════════════════════════════════════════════

/// Canonical classification hierarchy: subclass → sorted superclasses.
fn canonical_hierarchy(result: &ClassificationResult) -> BTreeMap<String, BTreeSet<String>> {
    result
        .hierarchy
        .iter()
        .map(|(sub, sups)| {
            (
                format!("{sub:?}"),
                sups.iter().map(|s| format!("{s:?}")).collect(),
            )
        })
        .collect()
}

/// A `Reasoner` configured for a specific `target_profile`.
fn reasoner_with_profile(profile: OWLProfile) -> Reasoner {
    Reasoner::new(oxidowl::config::ReasonerConfig {
        reasoning: ReasoningConfig {
            target_profile: profile,
            ..Default::default()
        },
        ..Default::default()
    })
    .expect("reasoner")
}

/// Ontology containing a **single-individual `ObjectOneOf`** — the exact
/// construct that crashed `classify()` with exit 1 on the ORE benchmark
/// (`Unsupported operation: Not supported in EL: ObjectOneOf([...])`).
fn nominal_ontology() -> Ontology {
    let df = DF::new();
    let mut onto = df.new_ontology();
    onto.add_axiom(df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    ));
    onto.add_axiom(df.sub_class_of(
        df.class_ce("http://ex.org/B"),
        df.class_ce("http://ex.org/C"),
    ));
    // A ⊑ {i} — single-individual nominal
    onto.add_axiom(df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.one_of(vec![df.named("http://ex.org/i")]),
    ));
    onto.add_axiom(df.class_assertion(df.class_ce("http://ex.org/A"), df.named("http://ex.org/i")));
    df.auto_declare(&mut onto);
    onto
}

/// EL-conforming ontology whose `DataSomeValuesFrom` the `ELReasoner`
/// normal form cannot translate (so `ELReasoner::initialize` fails).  The
/// `ClassAssertion` keeps `el_covered_by_absorption` false, i.e. it bypasses the
/// A8 sub-path and forces the EL fast path — which must fall back instead of
/// propagating the error.
fn el_but_untranslatable_ontology() -> Ontology {
    let df = DF::new();
    let mut onto = df.new_ontology();
    onto.add_axiom(df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    ));
    onto.add_axiom(df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.data_some_values_from(
            df.data_prop("http://ex.org/dp"),
            df.data_range_dt("http://www.w3.org/2001/XMLSchema#string"),
        ),
    ));
    onto.add_axiom(df.class_assertion(df.class_ce("http://ex.org/A"), df.named("http://ex.org/i")));
    df.auto_declare(&mut onto);
    onto
}

fn classify_with(profile: OWLProfile, ontology: Ontology) -> Result<ClassificationResult> {
    let mut reasoner = reasoner_with_profile(profile);
    reasoner.load_ontology(ontology)?;
    reasoner.classify()
}

#[test]
fn test_classify_nominal_ontology_does_not_error() -> Result<()> {
    // (a) classify() on an ontology containing a single-individual
    // ObjectOneOf must succeed (it used to exit 1 on the ORE benchmark).
    let result = classify_with(OWLProfile::Auto, nominal_ontology())?;
    assert!(
        !result.hierarchy.is_empty(),
        "classification of the nominal ontology should produce a hierarchy"
    );
    Ok(())
}

#[test]
fn test_classify_nominal_ontology_matches_dl_path() -> Result<()> {
    // (b) the Auto-profile result must equal the general DL-path result.
    let auto = classify_with(OWLProfile::Auto, nominal_ontology())?;
    let dl = classify_with(OWLProfile::DL, nominal_ontology())?;

    assert_eq!(
        canonical_hierarchy(&auto),
        canonical_hierarchy(&dl),
        "classify() under target_profile=Auto must agree with target_profile=DL \
         on an ontology containing a single-individual ObjectOneOf"
    );
    Ok(())
}

#[test]
fn test_classify_falls_back_when_el_fast_path_unsupported() -> Result<()> {
    // The fixture must be EL-conforming, otherwise this test would never
    // enter the EL branch.
    let report = ELValidator::new().validate(&el_but_untranslatable_ontology())?;
    assert!(
        report.conforms,
        "fixture must conform to OWL 2 EL to exercise the EL fast path"
    );

    // classify() must not propagate the EL fast path's `Error::unsupported`.
    let auto = classify_with(OWLProfile::Auto, el_but_untranslatable_ontology())?;
    assert!(!auto.hierarchy.is_empty(), "fallback must classify");

    // ...and the fallback result must equal the general DL path.
    let dl = classify_with(OWLProfile::DL, el_but_untranslatable_ontology())?;
    assert_eq!(
        canonical_hierarchy(&auto),
        canonical_hierarchy(&dl),
        "EL fast-path fallback must produce the same hierarchy as the DL path"
    );
    Ok(())
}
