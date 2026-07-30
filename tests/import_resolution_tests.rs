use oxidowl::import::{ImportDeclaration, ImportDependencyGraph, ImportError, ImportManager, ImportManagerConfig, ImportResolutionStrategy};
use oxidowl::manager::changes::OntologyChange;
use oxidowl::manager::iri_mapper::{AutoIRIMapper, NonMappingOntologyIRIMapper, OntologyIRIMapper, SimpleIRIMapper};
use oxidowl::manager::ManagerConfig;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::*;
use std::sync::Arc;

mod helpers;
use helpers::test_base::TestBase;

// ══════════════════════════════════════════════════════════════════════════════
// test_import_declaration_add_remove
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_declaration_add_remove() {
    let mut tb = TestBase::new();
    let base = "http://test.org/imports/";

    let imported_iri = IRI::new(&format!("{base}B"));
    let mut imported = Ontology::new();
    imported.set_iri(imported_iri.clone());
    imported.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassB"), EntityType::Class),
    ));

    let main_iri = IRI::new(&format!("{base}A"));
    let mut main = Ontology::new();
    main.set_iri(main_iri.clone());
    main.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassA"), EntityType::Class),
    ));
    main.imports.push(ImportsDeclaration {
        imported_ontology_iri: imported_iri.clone(),
    });

    let imported_ref = Arc::new(std::sync::RwLock::new(imported));
    let main_ref = Arc::new(std::sync::RwLock::new(main));
    tb.manager.register_ontology(imported_ref.clone());
    tb.manager.register_ontology(main_ref.clone());
    tb.manager.refresh_imports_closure();

    let closure = tb
        .manager
        .get_imports_closure(&main_ref)
        .expect("Should compute imports closure");
    assert_eq!(
        closure.len(),
        2,
        "Import closure should contain main + imported, got {}",
        closure.len()
    );

    tb.manager.remove_import(&main_iri, &imported_iri);
    {
        let mut a_guard = main_ref.write().unwrap();
        a_guard.imports.clear();
    }
    tb.manager.refresh_imports_closure();

    let closure_after = tb
        .manager
        .get_imports_closure(&main_ref)
        .expect("Should compute imports closure after removal");
    assert_eq!(
        closure_after.len(),
        1,
        "Import closure should shrink to just main after removal, got {}",
        closure_after.len()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_cycle_detection
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_cycle_detection() {
    let mut tb = TestBase::new();
    let base = "http://test.org/cycle/";

    let iri_a = IRI::new(&format!("{base}A"));
    let iri_b = IRI::new(&format!("{base}B"));
    let iri_c = IRI::new(&format!("{base}C"));

    let mut a = Ontology::new();
    a.set_iri(iri_a.clone());
    a.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassA"), EntityType::Class),
    ));
    a.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_b.clone(),
    });

    let mut b = Ontology::new();
    b.set_iri(iri_b.clone());
    b.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassB"), EntityType::Class),
    ));
    b.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_c.clone(),
    });

    let mut c = Ontology::new();
    c.set_iri(iri_c.clone());
    c.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassC"), EntityType::Class),
    ));
    c.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_a.clone(),
    });

    let a_ref = Arc::new(std::sync::RwLock::new(a));
    let b_ref = Arc::new(std::sync::RwLock::new(b));
    let c_ref = Arc::new(std::sync::RwLock::new(c));
    tb.manager.register_ontology(a_ref.clone());
    tb.manager.register_ontology(b_ref.clone());
    tb.manager.register_ontology(c_ref.clone());
    tb.manager.refresh_imports_closure();

    let cycles = tb.manager.detect_import_cycles();
    assert!(
        !cycles.is_empty(),
        "Should detect at least one import cycle in A->B->C->A"
    );

    let cycle_contains_a = cycles.iter().any(|cycle| cycle.contains(&iri_a));
    let cycle_contains_b = cycles.iter().any(|cycle| cycle.contains(&iri_b));
    let cycle_contains_c = cycles.iter().any(|cycle| cycle.contains(&iri_c));
    assert!(
        cycle_contains_a && cycle_contains_b && cycle_contains_c,
        "Cycle should contain A, B, and C. Found cycles: {:?}",
        cycles
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_transitive_import_chain
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transitive_import_chain() {
    let mut tb = TestBase::new();
    let base = "http://test.org/transitive/";

    let iri_a = IRI::new(&format!("{base}A"));
    let iri_b = IRI::new(&format!("{base}B"));
    let iri_c = IRI::new(&format!("{base}C"));

    let mut a = Ontology::new();
    a.set_iri(iri_a.clone());
    a.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassA"), EntityType::Class),
    ));
    a.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_b.clone(),
    });

    let mut b = Ontology::new();
    b.set_iri(iri_b.clone());
    b.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassB"), EntityType::Class),
    ));
    b.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_c.clone(),
    });

    let mut c = Ontology::new();
    c.set_iri(iri_c.clone());
    c.add_axiom(tb.df.declaration_axiom(
        tb.df.make_entity(format!("{base}ClassC"), EntityType::Class),
    ));

    let a_ref = Arc::new(std::sync::RwLock::new(a));
    let b_ref = Arc::new(std::sync::RwLock::new(b));
    let c_ref = Arc::new(std::sync::RwLock::new(c));
    tb.manager.register_ontology(a_ref.clone());
    tb.manager.register_ontology(b_ref.clone());
    tb.manager.register_ontology(c_ref.clone());
    tb.manager.refresh_imports_closure();

    let closure = tb
        .manager
        .get_imports_closure(&a_ref)
        .expect("Should compute transitive imports closure");
    assert_eq!(
        closure.len(),
        3,
        "Transitive closure should contain A, B, and C, got {}",
        closure.len()
    );

    let closure_iris: Vec<IRI> = closure
        .iter()
        .filter_map(|r| r.read().ok())
        .filter_map(|g| g.get_iri().cloned())
        .collect();
    assert!(closure_iris.contains(&&iri_a));
    assert!(closure_iris.contains(&&iri_b));
    assert!(closure_iris.contains(&&iri_c));
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_dependency_graph
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_dependency_graph() {
    let base = "http://test.org/depgraph/";

    let iri_a = IRI::new(&format!("{base}A"));
    let iri_b = IRI::new(&format!("{base}B"));
    let iri_c = IRI::new(&format!("{base}C"));
    let iri_d = IRI::new(&format!("{base}D"));

    let mut graph = ImportDependencyGraph::new();

    graph.add_dependency(iri_a.clone(), ImportDeclaration::new(iri_b.clone()));
    graph.add_dependency(iri_a.clone(), ImportDeclaration::new(iri_c.clone()));
    graph.add_dependency(iri_b.clone(), ImportDeclaration::new(iri_c.clone()));
    graph.add_dependency(iri_c.clone(), ImportDeclaration::new(iri_d.clone()));

    let transitive_a = graph.get_transitive_dependencies(&iri_a);
    assert!(transitive_a.contains(&iri_b));
    assert!(transitive_a.contains(&iri_c));
    assert!(transitive_a.contains(&iri_d));
    assert_eq!(
        transitive_a.len(),
        3,
        "A should transitively depend on B, C, D"
    );

    let deps_b = graph.get_dependencies(&iri_b);
    assert!(deps_b.is_some());
    let deps_b = deps_b.unwrap();
    assert!(deps_b.contains(&iri_c));
    assert_eq!(deps_b.len(), 1);

    let cycles = graph.detect_cycles();
    assert!(cycles.is_empty(), "Acyclic graph should have no cycles");

    let sorted = graph
        .topological_sort()
        .expect("Topological sort should succeed for acyclic graph");
    let pos_a = sorted
        .iter()
        .position(|iri| iri == &iri_a)
        .expect("A should be in sorted order");
    let pos_d = sorted
        .iter()
        .position(|iri| iri == &iri_d)
        .expect("D should be in sorted order");
    let pos_c = sorted
        .iter()
        .position(|iri| iri == &iri_c)
        .expect("C should be in sorted order");
    let pos_b = sorted
        .iter()
        .position(|iri| iri == &iri_b)
        .expect("B should be in sorted order");
    assert!(pos_c < pos_b);
    assert!(pos_b < pos_a);
    assert!(pos_d < pos_c);
}

// ══════════════════════════════════════════════════════════════════════════════
// test_auto_iri_mapper
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_auto_iri_mapper() {
    let tb = TestBase::new();
    let dir = tb.temp_dir.path();

    let onto_iri_str = "http://example.org/auto";
    let onto_iri = IRI::new(onto_iri_str);
    let file_path = dir.join("auto.owl");
    let rdf_content = format!(
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<{}> rdf:type owl:Ontology .\n",
        onto_iri_str
    );
    std::fs::write(&file_path, rdf_content).expect("Should write test file");

    let mapper = AutoIRIMapper::new(dir.to_path_buf());
    let doc_iri = mapper.get_document_iri(&onto_iri);
    assert!(
        doc_iri.is_some(),
        "AutoIRIMapper should resolve ontology IRI to file IRI"
    );
    let doc_iri_str = doc_iri.unwrap().as_str().to_string();
    assert!(
        doc_iri_str.starts_with("file://"),
        "Document IRI should be a file URL, got: {}",
        doc_iri_str
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_simple_iri_mapper
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_simple_iri_mapper() {
    let mut tb = TestBase::new();

    let ontology_iri = IRI::new("http://example.org/myont");
    let document_iri = IRI::new("file:///tmp/myont.owl");
    let mapper = SimpleIRIMapper::new(ontology_iri.clone(), document_iri.clone());

    let resolved = mapper.get_document_iri(&ontology_iri);
    assert!(
        resolved.is_some(),
        "SimpleIRIMapper should resolve the ontology IRI"
    );
    assert_eq!(
        resolved.unwrap().as_str(),
        document_iri.as_str(),
        "Resolved IRI should match the mapped document IRI"
    );

    let unresolved = mapper.get_document_iri(&IRI::new("http://example.org/other"));
    assert!(unresolved.is_none(), "Unmapped IRI should return None");

    tb.manager.add_iri_mapper(Box::new(mapper));
    let manager_resolved = tb.manager.resolve_document_iri(&ontology_iri);
    assert!(
        manager_resolved.is_some(),
        "Manager should resolve via SimpleIRIMapper"
    );
    assert_eq!(
        manager_resolved.unwrap().as_str(),
        document_iri.as_str()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_non_mapping_iri_mapper
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_non_mapping_iri_mapper() {
    let mapper = NonMappingOntologyIRIMapper::default();
    let result = mapper.get_document_iri(&IRI::new("http://example.org/anything"));
    assert!(
        result.is_none(),
        "NonMapping mapper should always return None"
    );

    let result2 = mapper.get_document_iri(&IRI::new("http://www.w3.org/2002/07/owl#"));
    assert!(result2.is_none(), "NonMapping mapper should always return None");

    assert_eq!(mapper.name(), "NonMappingIRIMapper");
}

// ══════════════════════════════════════════════════════════════════════════════
// test_missing_import_silent_mode
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_missing_import_silent_mode() {
    let config = ManagerConfig {
        silent_missing_imports: true,
        max_import_depth: 10,
        enable_change_history: false,
        max_history_size: 100,
    };
    let mut manager = OntologyManager::new_with_config(config);

    let main_iri = IRI::new("http://test.org/main");
    let missing_iri = IRI::new("http://test.org/nonexistent");

    let mut main = Ontology::new();
    main.set_iri(main_iri.clone());
    main.imports.push(ImportsDeclaration {
        imported_ontology_iri: missing_iri.clone(),
    });

    let main_ref = Arc::new(std::sync::RwLock::new(main));
    manager.register_ontology(main_ref.clone());

    let result = manager.get_imports_closure(&main_ref);
    assert!(
        result.is_ok(),
        "With silent_missing_imports=true, missing imports should not error"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_resolution_strict
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_resolution_strict() {
    let config = ImportManagerConfig {
        resolution_strategy: ImportResolutionStrategy::Strict,
        max_import_depth: 10,
        base_directories: vec![],
        url_mappings: std::collections::HashMap::new(),
        validate_imports: false,
        merge_imports: false,
        timeout: Some(std::time::Duration::from_secs(5)),
    };
    let import_manager = ImportManager::new(config);

    let tb = TestBase::new();
    let missing_iri = IRI::new("urn:test:definitely_does_not_exist");
    let mut onto = tb.df.build_ontology_with_iri(
        "http://test.org/caller",
        vec![],
    );

    let imports_prop = AnnotationProperty {
        iri: IRI::new("http://www.w3.org/2002/07/owl#imports"),
    };
    onto.annotations.push(Annotation::new(
        imports_prop,
        AnnotationValue::IRI(missing_iri.clone()),
        vec![],
    ));

    let results = import_manager.resolve_imports(&mut onto);
    assert!(
        results.is_ok(),
        "resolve_imports should complete even when import cannot be resolved"
    );
    let results = results.unwrap();
    assert!(!results.is_empty(), "Should have at least one resolution result");
    let resolution = &results[0];
    let has_resolution_error = resolution.errors.iter().any(|e| {
        matches!(e, ImportError::ResolutionFailed { .. })
    });
    assert!(
        has_resolution_error,
        "Resolution should fail for nonexistent import"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_declaration_with_annotations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_declaration_with_annotations() {
    let mut tb = TestBase::new();
    let base = "http://test.org/annotated/";

    let imported_iri = IRI::new(&format!("{base}Imported"));
    let label_annotation = tb.df.rdfs_label("Imported ontology description");

    let import_decl = ImportDeclaration::new(imported_iri.clone())
        .with_annotation(label_annotation.clone());

    assert_eq!(
        import_decl.imported_ontology_iri,
        imported_iri,
        "Import declaration should preserve IRI"
    );
    assert_eq!(
        import_decl.annotations.len(),
        1,
        "Import declaration should have one annotation"
    );

    let main_iri = IRI::new(&format!("{base}Main"));
    let mut main = Ontology::new();
    main.set_iri(main_iri.clone());

    tb.manager.register_ontology(Arc::new(std::sync::RwLock::new(main.clone())));

    tb.manager.apply_change(OntologyChange::AddImport {
        ontology_iri: main_iri.clone(),
        import: import_decl.clone(),
    });

    {
        let ont_ref = tb
            .manager
            .get_ontology(&main_iri)
            .expect("Main ontology should be registered");
        let guard = ont_ref.read().unwrap();
        assert_eq!(
            guard.imports.len(),
            1,
            "Main ontology should have one import after AddImport"
        );
        assert_eq!(
            guard.imports[0].imported_ontology_iri,
            imported_iri,
            "Import IRI should be correctly stored"
        );
    }

    tb.manager.apply_change(OntologyChange::RemoveImport {
        ontology_iri: main_iri.clone(),
        import: import_decl.clone(),
    });

    {
        let ont_ref = tb
            .manager
            .get_ontology(&main_iri)
            .expect("Main ontology should be registered");
        let guard = ont_ref.read().unwrap();
        assert!(
            guard.imports.is_empty(),
            "Imports should be empty after RemoveImport"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_declaration_version_iri
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_declaration_version_iri() {
    let ontology_iri = IRI::new("http://example.org/versioned");
    let version_iri = IRI::new("http://example.org/versioned/v1.0");

    let decl = ImportDeclaration::new(ontology_iri.clone())
        .with_version_iri(version_iri.clone());

    assert_eq!(decl.imported_ontology_iri, ontology_iri);
    assert_eq!(decl.version_iri, Some(version_iri.clone()));
    assert!(decl.annotations.is_empty());

    let decl_no_version = ImportDeclaration::new(ontology_iri.clone());
    assert_eq!(decl_no_version.version_iri, None);
    assert_ne!(decl, decl_no_version);
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_dependency_graph_with_cycles
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_dependency_graph_with_cycles() {
    let iri_a = IRI::new("http://test.org/gcycle/A");
    let iri_b = IRI::new("http://test.org/gcycle/B");

    let mut graph = ImportDependencyGraph::new();
    graph.add_dependency(iri_a.clone(), ImportDeclaration::new(iri_b.clone()));
    graph.add_dependency(iri_b.clone(), ImportDeclaration::new(iri_a.clone()));

    let cycles = graph.detect_cycles();
    assert!(!cycles.is_empty(), "Should detect A<->B cycle");
    let first_cycle = &cycles[0];
    assert!(
        first_cycle.contains(&iri_a) && first_cycle.contains(&iri_b),
        "Cycle should contain both A and B"
    );

    let sort_result = graph.topological_sort();
    assert!(
        sort_result.is_err(),
        "Topological sort should fail for cyclic graph"
    );
    match sort_result {
        Err(ImportError::CircularDependency { .. }) => {}
        _ => panic!("Expected CircularDependency error"),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_resolution_skip_strategy
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_resolution_skip_strategy() {
    let config = ImportManagerConfig {
        resolution_strategy: ImportResolutionStrategy::Skip,
        max_import_depth: 10,
        base_directories: vec![],
        url_mappings: std::collections::HashMap::new(),
        validate_imports: false,
        merge_imports: false,
        timeout: None,
    };
    let import_manager = ImportManager::new(config);

    let tb = TestBase::new();
    let mut onto = tb.df.build_ontology_with_iri(
        "http://test.org/skip",
        vec![tb.df.declaration_axiom(
            tb.df.make_entity("http://test.org/skip#C", EntityType::Class),
        )],
    );
    let imports_prop = AnnotationProperty {
        iri: IRI::new("http://www.w3.org/2002/07/owl#imports"),
    };
    onto.annotations.push(Annotation::new(
        imports_prop,
        AnnotationValue::IRI(IRI::new("urn:test:nonexistent_for_skip")),
        vec![],
    ));

    let axiom_count_before = onto.axioms().len();
    let results = import_manager.resolve_imports(&mut onto);
    assert!(results.is_ok(), "Skip strategy should not error");
    assert_eq!(
        onto.axioms().len(),
        axiom_count_before,
        "Skip strategy should not modify ontology axioms"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_import_depth_exceeded
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_depth_exceeded() {
    let config = ManagerConfig {
        enable_change_history: false,
        max_history_size: 100,
        silent_missing_imports: true,
        max_import_depth: 1,
    };
    let mut manager = OntologyManager::new_with_config(config);

    let base = "http://test.org/deep/";
    let iri_l0 = IRI::new(&format!("{base}Level0"));
    let iri_l1 = IRI::new(&format!("{base}Level1"));
    let iri_l2 = IRI::new(&format!("{base}Level2"));
    let iri_l3 = IRI::new(&format!("{base}Level3"));

    let mut l0 = Ontology::new();
    l0.set_iri(iri_l0.clone());
    l0.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_l1.clone(),
    });

    let mut l1 = Ontology::new();
    l1.set_iri(iri_l1.clone());
    l1.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_l2.clone(),
    });

    let mut l2 = Ontology::new();
    l2.set_iri(iri_l2.clone());
    l2.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_l3.clone(),
    });

    let l3 = Ontology::new();
    let l3_ref = Arc::new(std::sync::RwLock::new(l3));

    let l0_ref = Arc::new(std::sync::RwLock::new(l0));
    let l1_ref = Arc::new(std::sync::RwLock::new(l1));
    let l2_ref = Arc::new(std::sync::RwLock::new(l2));
    manager.register_ontology(l0_ref.clone());
    manager.register_ontology(l1_ref.clone());
    manager.register_ontology(l2_ref.clone());
    manager.register_ontology(l3_ref.clone());
    manager.refresh_imports_closure();

    let closure = manager
        .get_imports_closure(&l0_ref)
        .expect("Should compute imports closure");
    assert!(
        closure.len() < 4,
        "With max_import_depth=1, closure should be truncated, got {} entries",
        closure.len()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// test_refresh_imports_closure_from_ontology_declarations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_refresh_imports_closure_from_ontology_declarations() {
    let mut tb = TestBase::new();
    let base = "http://test.org/refresh/";

    let iri_a = IRI::new(&format!("{base}A"));
    let iri_b = IRI::new(&format!("{base}B"));

    let mut a = Ontology::new();
    a.set_iri(iri_a.clone());
    a.imports.push(ImportsDeclaration {
        imported_ontology_iri: iri_b.clone(),
    });

    let mut b = Ontology::new();
    b.set_iri(iri_b.clone());

    let a_ref = Arc::new(std::sync::RwLock::new(a));
    let b_ref = Arc::new(std::sync::RwLock::new(b));
    tb.manager.register_ontology(a_ref.clone());
    tb.manager.register_ontology(b_ref.clone());
    tb.manager.refresh_imports_closure();

    let closure = tb
        .manager
        .get_imports_closure(&a_ref)
        .expect("Should compute imports closure");
    assert_eq!(closure.len(), 2, "Should include A and B in closure");

    {
        let mut a_guard = a_ref.write().unwrap();
        a_guard.imports.clear();
    }
    tb.manager.refresh_imports_closure();

    let closure_after = tb
        .manager
        .get_imports_closure(&a_ref)
        .expect("Should compute imports closure after refresh");
    assert_eq!(
        closure_after.len(),
        1,
        "Closure should shrink to just A after import removal and refresh"
    );
}
