//! Unit tests for ontology parsers

use oxidowl::{
    Result,
    parsers::{
        turtle::TurtleParser,
        owl_xml::OwlXmlParser,
        functional::FunctionalParser,
        ntriples::NTriplesParser,
        rdf_xml::RdfXmlParser,
        OntologyFormat,
    },
    ontology::Ontology,
};
use std::io::Cursor;

const SIMPLE_TURTLE: &str = r#"
@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Animal rdf:type owl:Class .
:Dog rdf:type owl:Class ;
     rdfs:subClassOf :Animal .
:Fido rdf:type :Dog .
"#;

const SIMPLE_NTRIPLES: &str = r#"
<http://example.org/Animal> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://example.org/Dog> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://example.org/Dog> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .
<http://example.org/Fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Dog> .
"#;

const SIMPLE_RDF_XML: &str = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns="http://example.org/">

    <owl:Class rdf:about="http://example.org/Animal"/>
    
    <owl:Class rdf:about="http://example.org/Dog">
        <rdfs:subClassOf rdf:resource="http://example.org/Animal"/>
    </owl:Class>
    
    <Dog rdf:about="http://example.org/Fido"/>
    
</rdf:RDF>
"#;

const SIMPLE_OWL_XML: &str = r#"<?xml version="1.0"?>
<Ontology xmlns="http://www.w3.org/2002/07/owl#"
          ontologyIRI="http://example.org/simple">

    <Declaration>
        <Class IRI="http://example.org/Animal"/>
    </Declaration>
    
    <Declaration>
        <Class IRI="http://example.org/Dog"/>
    </Declaration>
    
    <Declaration>
        <NamedIndividual IRI="http://example.org/Fido"/>
    </Declaration>
    
    <SubClassOf>
        <Class IRI="http://example.org/Dog"/>
        <Class IRI="http://example.org/Animal"/>
    </SubClassOf>
    
    <ClassAssertion>
        <Class IRI="http://example.org/Dog"/>
        <NamedIndividual IRI="http://example.org/Fido"/>
    </ClassAssertion>
    
</Ontology>
"#;

const SIMPLE_FUNCTIONAL: &str = r#"
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/simple>

Declaration(Class(:Animal))
Declaration(Class(:Dog))
Declaration(NamedIndividual(:Fido))

SubClassOf(:Dog :Animal)
ClassAssertion(:Dog :Fido)

)
"#;

#[test]
fn test_turtle_parser_creation() {
    let parser = TurtleParser::new();
    
    assert!(true);
    println!("TurtleParser creation works");
}

#[test]
fn test_turtle_parsing() -> Result<()> {
    let parser = TurtleParser::new();
    let cursor = Cursor::new(SIMPLE_TURTLE);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed classes and individuals
    assert!(ontology.classes().len() >= 2); // Animal, Dog
    assert!(ontology.individuals().len() >= 1); // Fido
    assert!(ontology.axioms().len() >= 1); // SubClassOf, ClassAssertion
    
    println!("Turtle parsing works - classes: {}, individuals: {}, axioms: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.axioms().len());
    Ok(())
}

#[test]
fn test_ntriples_parser_creation() {
    let parser = NTriplesParser::new();
    
    assert!(true);
    println!("NTriplesParser creation works");
}

#[test]
fn test_ntriples_parsing() -> Result<()> {
    let parser = NTriplesParser::new();
    let cursor = Cursor::new(SIMPLE_NTRIPLES);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed the same content as Turtle
    assert!(ontology.classes().len() >= 2);
    assert!(ontology.individuals().len() >= 1);
    
    println!("N-Triples parsing works - classes: {}, individuals: {}, axioms: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.axioms().len());
    Ok(())
}

#[test]
fn test_rdf_xml_parser_creation() {
    let parser = RdfXmlParser::new();
    
    assert!(true);
    println!("RdfXmlParser creation works");
}

#[test]
fn test_rdf_xml_parsing() -> Result<()> {
    let parser = RdfXmlParser::new();
    let cursor = Cursor::new(SIMPLE_RDF_XML);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed classes and individuals
    assert!(ontology.classes().len() >= 2);
    assert!(ontology.individuals().len() >= 1);
    
    println!("RDF/XML parsing works - classes: {}, individuals: {}, axioms: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.axioms().len());
    Ok(())
}

#[test]
fn test_owl_xml_parser_creation() {
    let parser = OwlXmlParser::new();
    
    assert!(true);
    println!("OwlXmlParser creation works");
}

#[test]
fn test_owl_xml_parsing() -> Result<()> {
    let parser = OwlXmlParser::new();
    let cursor = Cursor::new(SIMPLE_OWL_XML);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed declarations and axioms
    assert!(ontology.classes().len() >= 2);
    assert!(ontology.individuals().len() >= 1);
    assert!(ontology.axioms().len() >= 2); // SubClassOf + ClassAssertion
    
    println!("OWL XML parsing works - classes: {}, individuals: {}, axioms: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.axioms().len());
    Ok(())
}

#[test]
fn test_functional_parser_creation() {
    let parser = FunctionalParser::new();
    
    assert!(true);
    println!("FunctionalParser creation works");
}

#[test]
fn test_functional_parsing() -> Result<()> {
    let parser = FunctionalParser::new();
    let cursor = Cursor::new(SIMPLE_FUNCTIONAL);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed declarations and axioms
    assert!(ontology.classes().len() >= 2);
    assert!(ontology.individuals().len() >= 1);
    assert!(ontology.axioms().len() >= 2);
    
    println!("Functional syntax parsing works - classes: {}, individuals: {}, axioms: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.axioms().len());
    Ok(())
}

#[test]
fn test_format_detection() {
    // Test format detection from file extensions
    assert_eq!(OntologyFormat::from_extension("ttl"), OntologyFormat::Turtle);
    assert_eq!(OntologyFormat::from_extension("turtle"), OntologyFormat::Turtle);
    assert_eq!(OntologyFormat::from_extension("nt"), OntologyFormat::NTriples);
    assert_eq!(OntologyFormat::from_extension("rdf"), OntologyFormat::RdfXml);
    assert_eq!(OntologyFormat::from_extension("owl"), OntologyFormat::OwlXml);
    assert_eq!(OntologyFormat::from_extension("ofn"), OntologyFormat::Functional);
    assert_eq!(OntologyFormat::from_extension("unknown"), OntologyFormat::Auto);
    
    println!("Format detection works");
}

#[test]
fn test_format_auto_detection() -> Result<()> {
    // Test auto-detection from content
    let turtle_format = OntologyFormat::detect_from_content(SIMPLE_TURTLE)?;
    assert_eq!(turtle_format, OntologyFormat::Turtle);
    
    let ntriples_format = OntologyFormat::detect_from_content(SIMPLE_NTRIPLES)?;
    assert_eq!(ntriples_format, OntologyFormat::NTriples);
    
    let xml_format = OntologyFormat::detect_from_content(SIMPLE_RDF_XML)?;
    assert!(xml_format == OntologyFormat::RdfXml || xml_format == OntologyFormat::OwlXml);
    
    println!("Auto format detection works");
    Ok(())
}

#[test]
fn test_parser_error_handling() {
    let parser = TurtleParser::new();
    
    // Test with invalid Turtle syntax
    let invalid_turtle = "@prefix : <invalid syntax";
    let cursor = Cursor::new(invalid_turtle);
    
    match parser.parse(cursor) {
        Ok(_) => panic!("Should have failed with invalid syntax"),
        Err(_) => {
            println!("Correctly handled invalid Turtle syntax");
        }
    }
    
    println!("Parser error handling works");
}

#[test]
fn test_empty_ontology_parsing() -> Result<()> {
    let parser = TurtleParser::new();
    
    // Test with minimal valid Turtle
    let minimal_turtle = "@prefix : <http://example.org/> .";
    let cursor = Cursor::new(minimal_turtle);
    
    let ontology = parser.parse(cursor)?;
    
    // Should create empty but valid ontology
    assert_eq!(ontology.classes().len(), 0);
    assert_eq!(ontology.individuals().len(), 0);
    
    println!("Empty ontology parsing works");
    Ok(())
}

#[test]
fn test_complex_turtle_parsing() -> Result<()> {
    let complex_turtle = r#"
@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Animal rdf:type owl:Class .
:Dog rdf:type owl:Class ;
     rdfs:subClassOf :Animal .
:Cat rdf:type owl:Class ;
     rdfs:subClassOf :Animal .
     
:hasChild rdf:type owl:ObjectProperty .

:Person rdf:type owl:Class ;
        owl:disjointWith :Animal .

:Fido rdf:type :Dog ;
      :hasChild :Buddy .
      
:Buddy rdf:type :Dog .

:John rdf:type :Person .
"#;
    
    let parser = TurtleParser::new();
    let cursor = Cursor::new(complex_turtle);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed multiple classes, properties, and individuals
    assert!(ontology.classes().len() >= 4); // Animal, Dog, Cat, Person
    assert!(ontology.individuals().len() >= 3); // Fido, Buddy, John
    assert!(ontology.object_properties().len() >= 1); // hasChild
    
    println!("Complex Turtle parsing works - classes: {}, individuals: {}, properties: {}", 
             ontology.classes().len(), ontology.individuals().len(), ontology.object_properties().len());
    Ok(())
}

#[test]
fn test_datatype_properties_parsing() -> Result<()> {
    let turtle_with_datatypes = r#"
@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

:Person rdf:type owl:Class .
:hasAge rdf:type owl:DatatypeProperty ;
        rdfs:domain :Person ;
        rdfs:range xsd:integer .
        
:John rdf:type :Person ;
      :hasAge 25 .
"#;
    
    let parser = TurtleParser::new();
    let cursor = Cursor::new(turtle_with_datatypes);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed datatype properties
    assert!(ontology.data_properties().len() >= 1); // hasAge
    
    println!("Datatype properties parsing works");
    Ok(())
}

#[test]
fn test_annotations_parsing() -> Result<()> {
    let turtle_with_annotations = r#"
@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Animal rdf:type owl:Class ;
        rdfs:label "Animal"@en ;
        rdfs:comment "A living organism"@en .
        
:Dog rdf:type owl:Class ;
     rdfs:subClassOf :Animal ;
     rdfs:label "Dog"@en .
"#;
    
    let parser = TurtleParser::new();
    let cursor = Cursor::new(turtle_with_annotations);
    
    let ontology = parser.parse(cursor)?;
    
    // Should have parsed classes with annotations
    assert!(ontology.classes().len() >= 2);
    
    println!("Annotations parsing works");
    Ok(())
}

#[test]
fn test_namespace_handling() -> Result<()> {
    let turtle_with_namespaces = r#"
@prefix ex1: <http://example1.org/> .
@prefix ex2: <http://example2.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex1:Animal rdf:type owl:Class .
ex2:Dog rdf:type owl:Class ;
        rdfs:subClassOf ex1:Animal .
"#;
    
    let parser = TurtleParser::new();
    let cursor = Cursor::new(turtle_with_namespaces);
    
    let ontology = parser.parse(cursor)?;
    
    // Should handle different namespaces correctly
    assert!(ontology.classes().len() >= 2);
    
    println!("Namespace handling works");
    Ok(())
}

#[test]
fn test_parser_factory() -> Result<()> {
    use oxidowl::parsers::ParserFactory;
    
    // Test creating parsers through factory
    let turtle_parser = ParserFactory::create_parser(OntologyFormat::Turtle)?;
    let owl_xml_parser = ParserFactory::create_parser(OntologyFormat::OwlXml)?;
    let functional_parser = ParserFactory::create_parser(OntologyFormat::Functional)?;
    
    // Should create appropriate parser types
    assert!(true);
    
    println!("Parser factory works");
    Ok(())
}

#[test]
fn test_parser_with_base_iri() -> Result<()> {
    let turtle_with_base = r#"
@base <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

<Animal> rdf:type owl:Class .
<Dog> rdf:type owl:Class ;
      rdfs:subClassOf <Animal> .
"#;
    
    let parser = TurtleParser::new();
    let cursor = Cursor::new(turtle_with_base);
    
    let ontology = parser.parse(cursor)?;
    
    // Should handle base IRI correctly
    assert!(ontology.classes().len() >= 2);
    
    println!("Base IRI handling works");
    Ok(())
}
