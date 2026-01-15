//! Tests for SPARQL INSERT and DELETE operations

use oxidowl::{
    Result,
    ontology::{Ontology, IRI},
    core::reasoner::Reasoner,
    config::ReasonerConfig,
};

#[test]
fn test_insert_class_assertion() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert a class assertion
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Inserted 1 triples"));

    Ok(())
}

#[test]
fn test_insert_multiple_triples() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert multiple triples
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person> .
            <http://example.org/test#Bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person> .
            <http://example.org/test#Charlie> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Inserted 3 triples"));

    Ok(())
}

#[test]
fn test_insert_object_property() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert object property assertion
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://example.org/test#knows> <http://example.org/test#Bob>
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Inserted 1 triples"));

    Ok(())
}

#[test]
fn test_insert_data_property() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert data property assertion with literal
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://example.org/test#age> "30"
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Inserted 1 triples"));

    Ok(())
}

#[test]
fn test_delete_triple() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // First insert a triple
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;
    reasoner.execute_sparql_query(insert_query)?;

    // Then delete it
    let delete_query = r#"
        DELETE DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(delete_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Deleted 1 triples"));

    Ok(())
}

#[test]
fn test_delete_nonexistent_triple() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Try to delete a triple that doesn't exist
    let delete_query = r#"
        DELETE DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(delete_query)?;
    assert!(result.contains("success"));
    assert!(result.contains("Deleted 0 triples"));

    Ok(())
}

#[test]
fn test_insert_then_query() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert data
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person> .
            <http://example.org/test#Bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;
    reasoner.execute_sparql_query(insert_query)?;

    // Query to verify
    let select_query = r#"
        SELECT ?person WHERE {
            ?person <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(select_query)?;
    assert!(result.contains("Alice"));
    assert!(result.contains("Bob"));

    Ok(())
}

#[test]
fn test_insert_delete_cycle() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Resource1> <http://example.org/test#property> <http://example.org/test#Resource2>
        }
    "#;
    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("Inserted 1 triples"));

    // Delete
    let delete_query = r#"
        DELETE DATA {
            <http://example.org/test#Resource1> <http://example.org/test#property> <http://example.org/test#Resource2>
        }
    "#;
    let result = reasoner.execute_sparql_query(delete_query)?;
    assert!(result.contains("Deleted 1 triples"));

    // Insert again
    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("Inserted 1 triples"));

    Ok(())
}

#[test]
fn test_invalid_insert_query() {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config).unwrap();
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology).unwrap();

    // Invalid query - missing DATA keyword
    let invalid_query = r#"
        INSERT {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(invalid_query);
    assert!(result.is_err());
}

#[test]
fn test_insert_with_semicolons() -> Result<()> {
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;
    
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/test#"));
    reasoner.load_ontology(ontology)?;

    // Insert using semicolons instead of periods
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/test#Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person> ;
            <http://example.org/test#Bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/test#Person>
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    assert!(result.contains("success"));

    Ok(())
}
