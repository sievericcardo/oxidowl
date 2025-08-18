//! Tests for HasKey axiom implementation

use oxidowl::{
    ontology::{
        Ontology, axioms::{HasKeyAxiom, Axiom}, 
        concepts::{ClassExpression, Class}, 
        ObjectProperty, ObjectPropertyExpression, DataPropertyExpression, IRI
    },
    core::reasoner::Reasoner,
    config::ReasonerConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_key_axiom_creation() {
        let mut ontology = Ontology::new();
        
        // Create a class
        let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        
        // Create properties
        let ssn_prop = oxidowl::ontology::DataProperty { iri: IRI::new("http://example.org/ssn") };
        let spouse_prop = ObjectProperty::new(IRI::new("http://example.org/spouse")).unwrap();
        
        // Create HasKey axiom: Person has key properties ssn and spouse
        let has_key_axiom = HasKeyAxiom {
            id: 1,
            class: person_class,
            object_properties: vec![ObjectPropertyExpression::ObjectProperty(spouse_prop)],
            data_properties: vec![DataPropertyExpression::DataProperty(ssn_prop)],
            annotations: vec![],
        };
        
        // Add axiom to ontology
        ontology.add_axiom(Axiom::HasKey(has_key_axiom));
        
        // Verify axiom was added
        assert_eq!(ontology.axioms().len(), 1);
    }

    #[test]
    fn test_has_key_axiom_reasoning() {
        let mut ontology = Ontology::new();
        
        // Create classes and properties
        let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let ssn_prop = oxidowl::ontology::DataProperty { iri: IRI::new("http://example.org/ssn") };
        
        // Create HasKey axiom
        let has_key_axiom = HasKeyAxiom {
            id: 2,
            class: person_class,
            object_properties: vec![], // No object properties
            data_properties: vec![DataPropertyExpression::DataProperty(ssn_prop)],
            annotations: vec![],
        };
        
        ontology.add_axiom(Axiom::HasKey(has_key_axiom));
        
        // Create reasoner
        let config = ReasonerConfig::default();
        let mut reasoner = Reasoner::new(config).unwrap();
        
        // Load the ontology
        let _ = reasoner.load_ontology(ontology);
        
        // Test that reasoner accepts the ontology with HasKey axioms
        assert!(reasoner.is_consistent().unwrap());
    }

    #[test]
    fn test_has_key_axiom_access() {
        let mut ontology = Ontology::new();
        let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
        let ssn_prop = oxidowl::ontology::DataProperty { iri: IRI::new("http://example.org/ssn") };

        let has_key_axiom = HasKeyAxiom {
            id: 3,
            class: person_class,
            object_properties: vec![],
            data_properties: vec![DataPropertyExpression::DataProperty(ssn_prop)],
            annotations: vec![],
        };

        ontology.add_axiom(Axiom::HasKey(has_key_axiom));

        // Verify we can access the HasKey axiom
        let axiom_count = ontology.axioms().len();
        assert_eq!(axiom_count, 1);

        let has_haskey = ontology.axioms().iter().any(|axiom| {
            matches!(axiom, Axiom::HasKey(_))
        });
        assert!(has_haskey);
    }
}
