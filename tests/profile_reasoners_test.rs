use oxidowl::config::ReasonerConfig;
/// Integration tests for profile-specific reasoners (EL and RL)
use oxidowl::error::Result;
use oxidowl::ontology::axioms::{
    Axiom, ClassAssertionAxiom, ObjectPropertyAssertionAxiom, SubClassOfAxiom,
};
use oxidowl::ontology::individuals::Individual;
use oxidowl::ontology::{Class, ClassExpression, IRI, ObjectPropertyExpression, Ontology};
use oxidowl::profiles::{ELReasoner, RLReasoner};

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
        let class_a = Class::new(IRI::new(&format!("http://example.org/Class{}", i)));
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
