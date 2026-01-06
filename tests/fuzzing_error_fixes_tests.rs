/// Tests for fixes to the 28 fuzzing errors found
/// This test suite ensures that the error fixes are working correctly
use oxidowl::error::Error;
use oxidowl::parsers::functional::FunctionalParser;
use oxidowl::parsers::Parser;

#[test]
fn test_utf8_char_boundary_fix() {
    // Test case from mutant_126.owl - contains UTF-8 multi-byte chars like 'è'
    let content = r#"Prefix(:=<http://owl.cs.man.ac.uk/dlapproximated/ontoma.ontology-of-alternative-medicine-french.1.orig.owl.xml#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(rdf:=<http://www.w3.org/1999/02/22-rdf-syntax-ns#>)
Ontology(
    SubClassOf(:Thérapie :Treatment)
)"#;

    let parser = FunctionalParser::new();
    // Should not panic on UTF-8 chars like 'é', 'è', 'ã'
    let result = parser.parse(content);
    // May succeed or fail with proper error, but should never panic
    match result {
        Ok(_) => println!("Validation passed"),
        Err(e) => println!("Validation failed with proper error: {}", e),
    }
}

#[test]
fn test_utf8_char_boundary_fix_2() {
    // Test case from mutant_315.owl - contains UTF-8 multi-byte chars like 'ã'
    let content = r#"Prefix(:=<http://owl.cs.man.ac.uk/qlapproximated/e787d3b6-fcc0-4495-b7b3-4af50deea232_sonalidade.owl.owl.xml#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(rdf:=<http://www.w3.org/1999/02/22-rdf-syntax-ns#>)
Ontology(
    SubClassOf(:Personalização :Treatment)
)"#;

    let parser = FunctionalParser::new();
    // Should not panic on UTF-8 chars
    let result = parser.parse(content);
    match result {
        Ok(_) => println!("Validation passed"),
        Err(e) => println!("Validation failed with proper error: {}", e),
    }
}

#[test]
fn test_annotation_keyword_not_class() {
    // Test case where "Annotation" keyword is used where class IRI expected
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/test>
    SubClassOf(Annotation owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should fail with clear error about keyword misuse, not relative URL error
    match result {
        Err(Error::OntologyParsing { message }) => {
            assert!(
                message.contains("Unexpected keyword") || message.contains("Annotation"),
                "Error should mention keyword issue, got: {}",
                message
            );
        }
        _ => panic!("Expected parsing error for Annotation keyword in class position"),
    }
}

#[test]
fn test_relative_iri_without_prefix() {
    // Test case where relative IRI without prefix is used
    let content = r#"Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/test>
    SubClassOf(RelativeClass owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should fail with clear error about undefined prefix or relative IRI
    match result {
        Err(Error::OntologyParsing { message }) => {
            assert!(
                message.contains("Relative IRI") || message.contains("Undefined prefix") || message.contains("relative URL"),
                "Error should mention relative IRI or undefined prefix, got: {}",
                message
            );
        }
        _ => panic!("Expected parsing error for relative IRI without prefix"),
    }
}

#[test]
fn test_undefined_prefix() {
    // Test case where undefined prefix is used
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/test>
    SubClassOf(undefined:Class owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should fail with clear error about undefined prefix
    match result {
        Err(Error::OntologyParsing { message }) => {
            assert!(
                message.contains("Undefined prefix") || message.contains("undefined"),
                "Error should mention undefined prefix, got: {}",
                message
            );
        }
        _ => panic!("Expected parsing error for undefined prefix"),
    }
}

#[test]
fn test_swrl_lenient_validation() {
    // Test case where SWRL syntax might be complex but valid
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(swrl:=<http://www.w3.org/2003/11/swrl#>)

Ontology(<http://example.org/test>
    DLSafeRule(
        Body(ClassAtom(:Person Variable(:x)))
        Head(ClassAtom(:Adult Variable(:x)))
    )
)"#;

    let parser = FunctionalParser::new();
    // Should not fail with SWRL syntax error for valid SWRL rules
    let result = parser.parse(content);
    match result {
        Ok(_) => println!("SWRL validation passed"),
        Err(e) => {
            // If it fails, should not be about missing '->' or ':-'
            let msg = format!("{}", e);
            assert!(
                !msg.contains("expected rule with '->'") && !msg.contains("expected rule with ':-'"),
                "Should not fail with arrow requirement for DLSafeRule syntax, got: {}",
                msg
            );
        }
    }
}

#[test]
fn test_valid_utf8_ontology() {
    // Test that valid UTF-8 ontology with proper IRIs works
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)

Ontology(<http://example.org/test>
    Declaration(Class(:Persönlichkeit))
    Declaration(Class(:Thérapie))
    SubClassOf(:Persönlichkeit owl:Thing)
    SubClassOf(:Thérapie owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should parse successfully with UTF-8 in IRIs
    assert!(result.is_ok(), "Valid UTF-8 ontology should parse successfully");
}

#[test]
fn test_error_handling_no_panic() {
    // Collection of edge cases that previously caused panics
    let test_cases = vec![
        // Empty content
        "",
        // Just whitespace
        "   \n\t  ",
        // Incomplete ontology
        "Ontology(",
        // Mismatched parens
        "Ontology())",
        // UTF-8 everywhere
        "Préfixé(:=<ürî>)",
    ];

    let parser = FunctionalParser::new();
    
    for (i, test_case) in test_cases.iter().enumerate() {
        // None of these should panic - they should either succeed or fail gracefully
        match parser.parse(test_case) {
            Ok(_) => println!("Test case {} passed validation", i),
            Err(e) => println!("Test case {} failed with proper error: {}", i, e),
        }
    }
}

#[test]
fn test_nested_utf8_chars() {
    // Test deeply nested structures with UTF-8 chars
    let content = r#"Prefix(:=<http://example.org/café/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/café>
    SubClassOf(
        ObjectIntersectionOf(
            :Café
            :Résumé
        )
        owl:Thing
    )
)"#;

    let parser = FunctionalParser::new();
    // Should not panic on nested structures with UTF-8
    let result = parser.parse(content);
    match result {
        Ok(_) => println!("Nested UTF-8 validation passed"),
        Err(e) => println!("Nested UTF-8 validation failed with proper error: {}", e),
    }
}
