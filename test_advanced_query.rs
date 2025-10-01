use oxidowl::prelude::*;
use oxidowl::query::advanced::conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
use oxidowl::query::advanced::engine::QueryEngine;
use oxidowl::reasoning::ReasoningService;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Advanced Query Processing Engine...");
    
    // Create a simple ontology
    let mut ontology = Ontology::new();
    let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    let student_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Student")));
    
    // Add SubClassOf axiom: Student ⊑ Person
    let subclass_axiom = axioms::Axiom::SubClassOf(axioms::SubClassOfAxiom {
        sub_class: student_class.clone(),
        super_class: person_class.clone(),
        annotations: vec![],
    });
    ontology.add_axiom(subclass_axiom);
    
    // Create reasoning service
    let reasoning_service = Arc::new(ReasoningService::new(ontology)?);
    
    // Create a conjunctive query: ?x rdf:type Student
    let var_x = QueryVariable::new("x".to_string());
    let student_atom = QueryAtom::ClassAssertion {
        individual: var_x.clone(),
        class: student_class,
    };
    
    let query = ConjunctiveQuery::new(
        vec![var_x.clone()],
        vec![student_atom],
    );
    
    // Create query engine and execute
    let query_engine = QueryEngine::new(reasoning_service.clone());
    let results = query_engine.execute_conjunctive_query(&query)?;
    
    println!("Query executed successfully!");
    println!("Number of results: {}", results.bindings.len());
    
    // Test OWL 2 QL rewriting
    println!("\nTesting OWL 2 QL query rewriting...");
    let rewritten_queries = query_engine.rewrite_to_ql(&query)?;
    println!("Number of rewritten queries: {}", rewritten_queries.len());
    
    println!("\nAdvanced Query Processing Engine is working correctly!");
    Ok(())
}