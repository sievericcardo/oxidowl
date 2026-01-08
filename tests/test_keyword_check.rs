// Test to verify is_owl_keyword function works correctly

#[test]
fn test_object_some_values_from_is_keyword() {
    // We can't directly access is_owl_keyword since it's private,
    // but we can test it indirectly by trying to parse it as a class
    use oxidowl::parsers::functional::FunctionalParser;
    use oxidowl::parsers::Parser;
    
    let content = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
    Declaration(Class(ObjectSomeValuesFrom))
)"#;
    
    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    match result {
        Err(e) => {
            let err_msg = format!("{}", e);
            println!("Error (expected): {}", err_msg);
            assert!(err_msg.contains("ObjectSomeValuesFrom"), 
                   "Error should mention ObjectSomeValuesFrom");
            assert!(err_msg.contains("keyword") || err_msg.contains("IRI"),
                   "Error should mention it's a keyword issue");
        }
        Ok(_) => panic!("Should have rejected ObjectSomeValuesFrom as a class name"),
    }
}

#[test]
fn test_prefixed_object_some_values_from_allowed() {
    use oxidowl::parsers::functional::FunctionalParser;
    use oxidowl::parsers::Parser;
    
    let content = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(ex:=<http://example.org/>)
Ontology(<http://example.org/test>
    Declaration(Class(ex:ObjectSomeValuesFrom))
)"#;
    
    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    assert!(result.is_ok(), "Prefixed ObjectSomeValuesFrom should be allowed");
}

#[test]
fn test_bracketed_object_some_values_from_allowed() {
    use oxidowl::parsers::functional::FunctionalParser;
    use oxidowl::parsers::Parser;
    
    let content = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
    Declaration(Class(<http://example.org/ObjectSomeValuesFrom>))
)"#;
    
    let parser = FunctionalParser::new();
    let result = parser.parse(content);
    
    assert!(result.is_ok(), "Bracketed ObjectSomeValuesFrom should be allowed");
}
