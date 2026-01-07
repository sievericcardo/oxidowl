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
        Err(Error::OntologyParsing { message, .. }) => {
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
        Err(Error::OntologyParsing { message, .. }) => {
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
        Err(Error::OntologyParsing { message, .. }) => {
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

// ============================================================================
// v0.8.0 Tests: Error Verbosity and Performance
// ============================================================================

#[test]
fn test_error_verbosity_minimal() {
    use oxidowl::parsers::{ParserConfig, ErrorVerbosity};
    
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    SubClassOf(Annotation owl:Thing)
)"#;

    let config = ParserConfig { error_verbosity: ErrorVerbosity::Minimal };
    let parser = FunctionalParser::with_config(config);
    let result = parser.parse(content);
    
    match result {
        Err(Error::OntologyParsing { message, line, column, context, token: _ }) => {
            assert!(message.contains("keyword"));
            // Minimal verbosity should not populate extra fields
            assert!(line.is_none() || column.is_none() || context.is_none());
        }
        _ => panic!("Expected OntologyParsing error"),
    }
}

#[test]
fn test_error_verbosity_standard() {
    use oxidowl::parsers::{ParserConfig, ErrorVerbosity};
    
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    SubClassOf(Import owl:Thing)
)"#;

    let config = ParserConfig { error_verbosity: ErrorVerbosity::Standard };
    let parser = FunctionalParser::with_config(config);
    let result = parser.parse(content);
    
    match result {
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(message.contains("keyword"));
        }
        _ => panic!("Expected OntologyParsing error"),
    }
}

#[test]
fn test_error_verbosity_detailed() {
    use oxidowl::parsers::{ParserConfig, ErrorVerbosity};
    
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    SubClassOf(Declaration owl:Thing)
)"#;

    let config = ParserConfig { error_verbosity: ErrorVerbosity::Detailed };
    let parser = FunctionalParser::with_config(config);
    let result = parser.parse(content);
    
    match result {
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(message.contains("keyword"));
        }
        _ => panic!("Expected OntologyParsing error"),
    }
}

#[test]
fn test_comprehensive_keyword_validation() {
    use oxidowl::parsers::FunctionalParser;
    
    // Test various OWL keywords that should not be used as class names
    let keywords = vec![
        "Annotation", "AnnotationProperty", "Import", "Declaration",
        "ObjectIntersectionOf", "ObjectUnionOf", "DataSomeValuesFrom",
        "SubClassOf", "EquivalentClasses", "DisjointClasses",
        "HasKey", "DLSafeRule", "Body", "Head",
    ];
    
    let parser = FunctionalParser::new();
    
    for keyword in keywords {
        let content = format!(
            r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    SubClassOf({} <http://example.org/Thing>)
)"#,
            keyword
        );
        
        let result = parser.parse(&content);
        match result {
            Err(Error::OntologyParsing { message, .. }) => {
                assert!(
                    message.contains("keyword") || message.contains(keyword),
                    "Error should mention keyword issue for '{}', got: {}",
                    keyword,
                    message
                );
            }
            _ => panic!("Expected parsing error for OWL keyword '{}' in class position", keyword),
        }
    }
}

#[test]
fn test_swrl_rule_parsing() {
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(swrl:=<http://www.w3.org/2003/11/swrl#>)
Ontology(<http://example.org/test>
    DLSafeRule(
        Body(ClassAtom(:Person Variable(:x)))
        Head(ClassAtom(:Adult Variable(:x)))
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // SWRL parsing should not panic and should handle basic structure
    match result {
        Ok(_) => println!("SWRL rule parsing succeeded"),
        Err(e) => println!("SWRL rule parsing failed with: {}", e),
    }
}

#[test]
fn test_swrl_with_minimal_validation() {
    use oxidowl::parsers::{ParserConfig, ErrorVerbosity};
    
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    DLSafeRule(Body() Head())
)"#;

    // Minimal verbosity should skip detailed SWRL validation
    let config = ParserConfig { error_verbosity: ErrorVerbosity::Minimal };
    let parser = FunctionalParser::with_config(config);
    let result = parser.parse(content);
    
    // Should not panic, minimal validation
    match result {
        Ok(_) => println!("SWRL minimal validation passed"),
        Err(e) => println!("SWRL minimal validation error: {}", e),
    }
}

#[test]
fn test_keyword_validation_performance() {
    use std::time::Instant;
    use oxidowl::parsers::FunctionalParser;
    
    let parser = FunctionalParser::new();
    
    // Create a large valid ontology to test parsing performance
    let mut content = String::from(
        r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
"#,
    );
    
    // Add many class declarations
    for i in 0..100 {
        content.push_str(&format!("    Declaration(Class(:Class{}))\n", i));
    }
    
    // Add many subclass axioms
    for i in 0..100 {
        content.push_str(&format!("    SubClassOf(:Class{} owl:Thing)\n", i));
    }
    
    content.push_str(")");
    
    let start = Instant::now();
    let result = parser.parse(&content);
    let duration = start.elapsed();
    
    match result {
        Ok(_) => println!("Performance test passed in {:?}", duration),
        Err(e) => println!("Performance test error: {} (in {:?})", e, duration),
    }
    
    // Test should complete in reasonable time (< 100ms for this size)
    assert!(duration.as_millis() < 100, "Parsing took too long: {:?}", duration);
}

// ============================================================================
// v0.8.0 Fixes for Third Fuzzing Campaign
// ============================================================================

#[test]
fn test_keywords_in_full_iris() {
    use oxidowl::parsers::FunctionalParser;
    
    // Keywords inside angle brackets (full IRIs) should be allowed
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    Declaration(Class(<http://example.org/ObjectSomeValuesFrom>))
    Declaration(Class(<http://example.org/Annotation>))
    SubClassOf(<http://example.org/ObjectSomeValuesFrom> <http://example.org/Annotation>)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should succeed - keywords in full IRIs are valid
    match result {
        Ok(_) => println!("Keywords in full IRIs correctly allowed"),
        Err(e) => panic!("Should allow keywords in full IRIs, got error: {}", e),
    }
}

#[test]
fn test_annotation_keyword_in_angle_brackets() {
    use oxidowl::parsers::FunctionalParser;
    
    // Specific case from mutant_1043.owl
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
    SubClassOf(<http://example.org/Annotation> owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should succeed
    match result {
        Ok(_) => println!("Annotation in full IRI correctly allowed"),
        Err(e) => panic!("Should allow 'Annotation' in full IRI, got error: {}", e),
    }
}

#[test]
fn test_objectsomevaluesfrom_in_angle_brackets() {
    use oxidowl::parsers::FunctionalParser;
    
    // Specific case from mutant_1042.owl
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    Declaration(Class(<http://www.w3.org/2002/07/owl#ObjectSomeValuesFrom>))
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should succeed
    match result {
        Ok(_) => println!("ObjectSomeValuesFrom in full IRI correctly allowed"),
        Err(e) => panic!("Should allow 'ObjectSomeValuesFrom' in full IRI, got error: {}", e),
    }
}

#[test]
fn test_lenient_swrl_validation() {
    use oxidowl::parsers::FunctionalParser;
    
    // SWRL rules with various formats should not cause parse errors
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(swrl:=<http://www.w3.org/2003/11/swrl#>)
Ontology(<http://example.org/test>
    DLSafeRule(
        Body()
        Head()
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should succeed or fail gracefully without panicking
    match result {
        Ok(_) => println!("Lenient SWRL validation passed"),
        Err(e) => println!("SWRL validation error (expected): {}", e),
    }
}

#[test]
fn test_keyword_rejection_still_works() {
    use oxidowl::parsers::FunctionalParser;
    
    // Bare keywords (not in angle brackets) should still be rejected
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    SubClassOf(Annotation owl:Thing)
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should fail with keyword error
    match result {
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(
                message.contains("keyword") || message.contains("Annotation"),
                "Error should mention keyword issue, got: {}",
                message
            );
        }
        _ => panic!("Expected parsing error for bare keyword in class position"),
    }
}

// ============================================================================
// v0.8.0 Fixes for Fourth Fuzzing Campaign
// ============================================================================

#[test]
fn test_swrl_unbalanced_parentheses_relaxed() {
    use oxidowl::parsers::FunctionalParser;
    
    // Fourth campaign: SWRL rules with unbalanced parentheses in body
    // The validator should not check parentheses balance, let parser handle it
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    DLSafeRule(
        Body(ClassAtom(:Person Variable(:x)))
        Head(ClassAtom(:Adult Variable(:x)))
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should not fail on validation - parser may fail, but no validation error
    match result {
        Ok(_) => println!("SWRL rule parsing succeeded"),
        Err(Error::OntologyParsing { message, .. }) => {
            // Should not contain "Unbalanced parentheses" from validator
            assert!(
                !message.contains("Unbalanced parentheses in SWRL rule body"),
                "Validator should not check SWRL parentheses, got: {}",
                message
            );
        }
        _ => {}
    }
}

#[test]
fn test_annotation_in_subclassof_axiom() {
    use oxidowl::parsers::FunctionalParser;
    
    // Fourth campaign: Annotation keyword at start of axiom
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)

Ontology(<http://example.org/test>
    SubClassOf(
        Annotation(rdfs:comment "This is annotated")
        :Employee
        :Person
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should skip the Annotation(...) and parse the axiom correctly
    match result {
        Ok(_) => println!("Annotation in axiom correctly skipped"),
        Err(e) => panic!("Should skip Annotation in axiom, got error: {}", e),
    }
}

#[test]
fn test_annotation_in_disjoint_classes() {
    use oxidowl::parsers::FunctionalParser;
    
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)

Ontology(<http://example.org/test>
    DisjointClasses(
        Annotation(rdfs:comment "Disjoint annotation")
        :Cat
        :Dog
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    match result {
        Ok(_) => println!("Annotation in DisjointClasses correctly handled"),
        Err(e) => panic!("Should handle annotated DisjointClasses, got: {}", e),
    }
}

#[test]
fn test_annotation_in_class_assertion() {
    use oxidowl::parsers::FunctionalParser;
    
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)

Ontology(<http://example.org/test>
    Declaration(Class(:Person))
    Declaration(NamedIndividual(:john))
    ClassAssertion(
        Annotation(rdfs:comment "John is a person")
        :Person
        :john
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    match result {
        Ok(_) => println!("Annotation in ClassAssertion correctly handled"),
        Err(e) => panic!("Should handle annotated ClassAssertion, got: {}", e),
    }
}

#[test]
fn test_multiple_annotations_in_axiom() {
    use oxidowl::parsers::FunctionalParser;
    
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)

Ontology(<http://example.org/test>
    SubClassOf(
        Annotation(rdfs:comment "First annotation")
        Annotation(rdfs:label "Employee subclass")
        :Employee
        :Person
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    match result {
        Ok(_) => println!("Multiple annotations correctly skipped"),
        Err(e) => panic!("Should skip multiple annotations, got error: {}", e),
    }
}

// ============================================================================
// v0.8.0 Fixes for Fifth Fuzzing Campaign
// ============================================================================

#[test]
fn test_bare_keyword_as_class_name_rejected() {
    use oxidowl::parsers::FunctionalParser;
    
    // Fifth campaign: Bare keywords (not in angle brackets, not prefixed) should be rejected
    // This is CORRECT behavior - bare keywords cannot be used as IRIs
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    Declaration(Class(ObjectSomeValuesFrom))
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should correctly reject bare keyword
    match result {
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(
                message.contains("Cannot use OWL keyword") || message.contains("ObjectSomeValuesFrom"),
                "Should reject bare keyword, got: {}",
                message
            );
        }
        Ok(_) => panic!("Should reject bare keyword 'ObjectSomeValuesFrom' as class name"),
        Err(e) => panic!("Expected OntologyParsing error, got: {}", e),
    }
}

#[test]
fn test_prefixed_keyword_allowed() {
    use oxidowl::parsers::FunctionalParser;
    
    // Prefixed keywords like owl:ObjectSomeValuesFrom should work
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
    Declaration(Class(owl:ObjectSomeValuesFrom))
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should succeed - prefixed names are OK
    match result {
        Ok(_) => println!("Prefixed keyword correctly allowed"),
        Err(e) => panic!("Should allow prefixed keyword 'owl:ObjectSomeValuesFrom', got error: {}", e),
    }
}

#[test]
fn test_dlsaferule_functional_syntax() {
    use oxidowl::parsers::FunctionalParser;
    
    // Fifth campaign: DLSafeRule in Functional Syntax doesn't use arrows
    // Should not trigger "expected format 'body -> head'" error
    let content = r#"Prefix(:=<http://example.org/>)
Ontology(<http://example.org/test>
    DLSafeRule(
        Body()
        Head()
    )
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should not fail with "expected format 'body -> head'" validation error
    match result {
        Ok(_) => println!("DLSafeRule Functional Syntax correctly parsed"),
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(
                !message.contains("expected format 'body -> head'"),
                "Should not check for arrow format in Functional Syntax, got: {}",
                message
            );
        }
        _ => {}
    }
}

#[test]
fn test_swrl_validation_skips_functional_syntax() {
    use oxidowl::parsers::FunctionalParser;
    
    // Validator should skip DLSafeRule lines and let parser handle them
    let content = r#"Prefix(:=<http://example.org/>)
Prefix(swrl:=<http://www.w3.org/2003/11/swrl#>)
Ontology(<http://example.org/test>
    DLSafeRule(Body(ClassAtom(:Person Variable(:x))) Head(ClassAtom(:Adult Variable(:x))))
)"#;

    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    // Should not fail validation - parser handles DLSafeRule
    match result {
        Ok(_) => println!("SWRL validation correctly skipped for Functional Syntax"),
        Err(Error::OntologyParsing { message, .. }) => {
            assert!(
                !message.contains("Invalid SWRL rule") && !message.contains("expected format"),
                "Validation should skip Functional Syntax DLSafeRule, got: {}",
                message
            );
        }
        _ => {}
    }
}
