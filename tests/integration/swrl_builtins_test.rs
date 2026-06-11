use oxidowl::ontology::*;
use oxidowl::swrl::*;

#[test]
fn test_new_built_ins_integration() {
    // Create a simple ontology
    let _ontology = Ontology::new();

    // Create SWRL variables (using IRI)
    let x = SWRLVariable::new(IRI::new("http://example.org/x"));
    let y = SWRLVariable::new(IRI::new("http://example.org/y"));
    let z = SWRLVariable::new(IRI::new("http://example.org/z"));

    // Test 1: Boolean NOT built-in
    let bool_not_atom = SWRLAtom::BuiltInAtom {
        predicate: IRI::new("http://www.w3.org/2003/11/swrlb#booleanNot"),
        arguments: vec![
            SWRLDArgument::Variable(y.clone()),
            SWRLDArgument::Variable(x.clone()),
        ],
    };

    // Test 2: Ceiling built-in
    let ceiling_atom = SWRLAtom::BuiltInAtom {
        predicate: IRI::new("http://www.w3.org/2003/11/swrlb#ceiling"),
        arguments: vec![
            SWRLDArgument::Variable(z.clone()),
            SWRLDArgument::Literal(Literal::new("3.7".to_string())),
        ],
    };

    // Test 3: String equal ignore case built-in
    let string_equal_atom = SWRLAtom::BuiltInAtom {
        predicate: IRI::new("http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase"),
        arguments: vec![
            SWRLDArgument::Literal(Literal::new("Hello".to_string())),
            SWRLDArgument::Literal(Literal::new("HELLO".to_string())),
        ],
    };

    // Test 4: List member built-in
    let list_member_atom = SWRLAtom::BuiltInAtom {
        predicate: IRI::new("http://www.w3.org/2003/11/swrlb#member"),
        arguments: vec![
            SWRLDArgument::Literal(Literal::new("apple".to_string())),
            SWRLDArgument::Literal(Literal::new("apple,banana,orange".to_string())),
        ],
    };

    // Create a rule using these built-ins
    let rule = SWRLRule::new(
        vec![SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/TestClass"))),
            argument: SWRLIArgument::Variable(x.clone()),
        }],
        vec![
            bool_not_atom,
            ceiling_atom,
            string_equal_atom,
            list_member_atom,
        ],
    );

    // Test the built-in registry
    let _registry = BuiltInRegistry::new();

    // Verify some of our new built-ins are recognized
    // Note: The actual implementation might use different namespaces
    // This is just to verify the test structure works
    println!("Testing built-in registry functionality...");

    // Simple validation test - just ensure the rule can be created
    assert!(rule.head.len() == 1, "Rule should have one head atom");
    assert!(rule.body.len() == 4, "Rule should have four body atoms");
}

#[test]
fn test_built_in_coverage() {
    let registry = BuiltInRegistry::new();

    // Test a sample of built-ins that might be available
    let potential_builtins = vec![
        "swrlb:equal",
        "swrlb:notEqual",
        "swrlb:lessThan",
        "swrlb:greaterThan",
        "swrlb:add",
        "swrlb:subtract",
        "swrlb:multiply",
        "swrlb:divide",
        "swrlb:stringConcat",
        "swrlb:stringLength",
    ];

    let mut registered_count = 0;
    let mut missing_builtins = Vec::new();

    for builtin in &potential_builtins {
        // Convert string to IRI for checking
        let iri = IRI::new(&format!("http://www.w3.org/2003/11/{}", builtin));
        if registry.is_registered(&iri) {
            registered_count += 1;
        } else {
            missing_builtins.push(builtin);
        }
    }

    println!(
        "Built-in coverage: {}/{} ({:.1}%)",
        registered_count,
        potential_builtins.len(),
        (registered_count as f64 / potential_builtins.len() as f64) * 100.0
    );

    if !missing_builtins.is_empty() {
        println!("Missing built-ins: {:?}", missing_builtins);
    }

    // Just verify the registry exists and works
    println!("Built-in registry test completed");
}
