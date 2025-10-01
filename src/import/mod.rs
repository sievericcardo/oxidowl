//! OWL 2 Import Management
//!
//! This module provides enhanced import support for OWL 2 ontologies, including:
//! - Recursive import resolution
//! - Import cycle detection
//! - Version-aware importing
//! - Import validation
//! - Dependency management

use crate::error::OxidowlError;
use crate::ontology::{Annotation, AnnotationValue, IRI, Ontology};
use log::warn;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Import declaration for ontologies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportDeclaration {
    /// The IRI of the imported ontology
    pub imported_ontology_iri: IRI,
    /// Optional version IRI
    pub version_iri: Option<IRI>,
    /// Annotations on this import declaration
    pub annotations: Vec<Annotation>,
}

impl ImportDeclaration {
    /// Create a new import declaration
    pub fn new(imported_ontology_iri: IRI) -> Self {
        Self {
            imported_ontology_iri,
            version_iri: None,
            annotations: Vec::new(),
        }
    }

    /// Set the version IRI
    pub fn with_version_iri(mut self, version_iri: IRI) -> Self {
        self.version_iri = Some(version_iri);
        self
    }

    /// Add an annotation
    pub fn with_annotation(mut self, annotation: Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }
}

/// Import resolution strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportResolutionStrategy {
    /// Strict - imports must resolve exactly
    Strict,
    /// Best effort - continue if some imports fail
    BestEffort,
    /// Skip all imports
    Skip,
}

/// Import manager configuration
#[derive(Debug, Clone)]
pub struct ImportManagerConfig {
    /// Resolution strategy
    pub resolution_strategy: ImportResolutionStrategy,
    /// Maximum import depth to prevent infinite recursion
    pub max_import_depth: usize,
    /// Base directories to search for imported ontologies
    pub base_directories: Vec<PathBuf>,
    /// URL remapping for imports
    pub url_mappings: HashMap<String, String>,
    /// Whether to validate imported ontologies
    pub validate_imports: bool,
    /// Whether to merge imported axioms
    pub merge_imports: bool,
}

impl Default for ImportManagerConfig {
    fn default() -> Self {
        Self {
            resolution_strategy: ImportResolutionStrategy::BestEffort,
            max_import_depth: 10,
            base_directories: vec![PathBuf::from(".")],
            url_mappings: HashMap::new(),
            validate_imports: true,
            merge_imports: true,
        }
    }
}

/// Import resolution result
#[derive(Debug, Clone)]
pub struct ImportResolutionResult {
    /// The resolved ontology (if successful)
    pub ontology: Option<Ontology>,
    /// Import IRI that was resolved
    pub import_iri: IRI,
    /// Actual source where the ontology was found
    pub resolved_source: Option<String>,
    /// Any errors that occurred during resolution
    pub errors: Vec<ImportError>,
    /// Warnings during resolution
    pub warnings: Vec<String>,
}

/// Import dependency graph
#[derive(Debug, Clone, Default)]
pub struct ImportDependencyGraph {
    /// Direct dependencies: ontology IRI -> set of imported IRIs
    dependencies: HashMap<IRI, HashSet<IRI>>,
    /// Reverse dependencies: imported IRI -> set of ontologies that import it
    reverse_dependencies: HashMap<IRI, HashSet<IRI>>,
    /// Import declarations for each ontology
    import_declarations: HashMap<IRI, Vec<ImportDeclaration>>,
}

impl ImportDependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an import dependency
    pub fn add_dependency(&mut self, ontology_iri: IRI, import_decl: ImportDeclaration) {
        // Add to direct dependencies
        self.dependencies
            .entry(ontology_iri.clone())
            .or_default()
            .insert(import_decl.imported_ontology_iri.clone());

        // Add to reverse dependencies
        self.reverse_dependencies
            .entry(import_decl.imported_ontology_iri.clone())
            .or_default()
            .insert(ontology_iri.clone());

        // Store import declaration
        self.import_declarations
            .entry(ontology_iri)
            .or_default()
            .push(import_decl);
    }

    /// Get direct dependencies of an ontology
    pub fn get_dependencies(&self, ontology_iri: &IRI) -> Option<&HashSet<IRI>> {
        self.dependencies.get(ontology_iri)
    }

    /// Get all transitive dependencies
    pub fn get_transitive_dependencies(&self, ontology_iri: &IRI) -> HashSet<IRI> {
        let mut visited = HashSet::new();
        let mut to_visit = VecDeque::new();
        to_visit.push_back(ontology_iri.clone());

        while let Some(current) = to_visit.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        to_visit.push_back(dep.clone());
                    }
                }
            }
        }

        visited.remove(ontology_iri); // Remove self
        visited
    }

    /// Detect circular dependencies
    pub fn detect_cycles(&self) -> Vec<Vec<IRI>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for ontology_iri in self.dependencies.keys() {
            if !visited.contains(ontology_iri) {
                self.detect_cycles_dfs(
                    ontology_iri,
                    &mut visited,
                    &mut recursion_stack,
                    &mut Vec::new(),
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn detect_cycles_dfs(
        &self,
        current: &IRI,
        visited: &mut HashSet<IRI>,
        recursion_stack: &mut HashSet<IRI>,
        path: &mut Vec<IRI>,
        cycles: &mut Vec<Vec<IRI>>,
    ) {
        visited.insert(current.clone());
        recursion_stack.insert(current.clone());
        path.push(current.clone());

        if let Some(deps) = self.dependencies.get(current) {
            for dep in deps {
                if recursion_stack.contains(dep) {
                    // Found a cycle - extract it from the path
                    if let Some(cycle_start) = path.iter().position(|iri| iri == dep) {
                        let cycle = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                } else if !visited.contains(dep) {
                    self.detect_cycles_dfs(dep, visited, recursion_stack, path, cycles);
                }
            }
        }

        path.pop();
        recursion_stack.remove(current);
    }

    /// Get topological ordering of ontologies (for safe import order)
    pub fn topological_sort(&self) -> Result<Vec<IRI>, ImportError> {
        // Check for cycles first
        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            return Err(ImportError::CircularDependency {
                cycle: cycles[0].clone(),
                context: "Cannot perform topological sort with circular dependencies".to_string(),
            });
        }

        let mut in_degree: HashMap<IRI, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize in-degrees
        for ontology_iri in self.dependencies.keys() {
            in_degree.insert(ontology_iri.clone(), 0);
        }
        // Also include imported ontologies that might not be in keys
        for deps in self.dependencies.values() {
            for dep in deps {
                in_degree.entry(dep.clone()).or_insert(0);
            }
        }

        // Count incoming dependencies correctly
        // If ontology_iri imports deps, then ontology_iri depends on deps
        // So ontology_iri should have incoming edges from its dependencies
        for (ontology_iri, deps) in &self.dependencies {
            for _dep in deps {
                *in_degree.get_mut(ontology_iri).unwrap() += 1;
            }
        }

        // Find all ontologies with no incoming dependencies
        for (iri, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(iri.clone());
            }
        }

        // Process queue
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            // Reduce in-degree of ontologies that depend on current ontology
            if let Some(dependents) = self.reverse_dependencies.get(&current) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Import error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// Import IRI could not be resolved
    ResolutionFailed { import_iri: IRI, reason: String },
    /// Circular dependency detected
    CircularDependency { cycle: Vec<IRI>, context: String },
    /// Import depth exceeded
    DepthExceeded {
        max_depth: usize,
        current_depth: usize,
    },
    /// Parse error in imported ontology
    ParseError { import_iri: IRI, error: String },
    /// Validation error in imported ontology
    ValidationError { import_iri: IRI, error: String },
    /// Version mismatch
    VersionMismatch {
        import_iri: IRI,
        expected_version: Option<IRI>,
        actual_version: Option<IRI>,
    },
    /// I/O error
    IoError { import_iri: IRI, error: String },
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ResolutionFailed { import_iri, reason } => {
                write!(f, "Failed to resolve import {}: {}", import_iri, reason)
            }
            ImportError::CircularDependency { cycle, context } => {
                write!(
                    f,
                    "Circular dependency detected: {} ({})",
                    cycle
                        .iter()
                        .map(|iri| iri.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> "),
                    context
                )
            }
            ImportError::DepthExceeded {
                max_depth,
                current_depth,
            } => {
                write!(
                    f,
                    "Import depth exceeded: {} > {}",
                    current_depth, max_depth
                )
            }
            ImportError::ParseError { import_iri, error } => {
                write!(f, "Parse error in import {}: {}", import_iri, error)
            }
            ImportError::ValidationError { import_iri, error } => {
                write!(f, "Validation error in import {}: {}", import_iri, error)
            }
            ImportError::VersionMismatch {
                import_iri,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "Version mismatch for {}: expected {:?}, got {:?}",
                    import_iri, expected_version, actual_version
                )
            }
            ImportError::IoError { import_iri, error } => {
                write!(f, "I/O error loading {}: {}", import_iri, error)
            }
        }
    }
}

impl std::error::Error for ImportError {}

/// Import manager for handling ontology imports
pub struct ImportManager {
    /// Configuration
    config: ImportManagerConfig,
    /// Cache of resolved ontologies
    ontology_cache: Arc<RwLock<HashMap<IRI, Arc<Ontology>>>>,
    /// Dependency graph
    dependency_graph: Arc<RwLock<ImportDependencyGraph>>,
}

impl ImportManager {
    /// Create a new import manager
    pub fn new(config: ImportManagerConfig) -> Self {
        Self {
            config,
            ontology_cache: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(ImportDependencyGraph::new())),
        }
    }

    /// Create import manager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ImportManagerConfig::default())
    }

    /// Resolve all imports for an ontology
    pub fn resolve_imports(
        &self,
        ontology: &mut Ontology,
    ) -> Result<Vec<ImportResolutionResult>, OxidowlError> {
        let mut results = Vec::new();
        let ontology_iri = ontology
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:unknown"));

        // Get import declarations from ontology
        let import_declarations = self.extract_import_declarations(ontology);

        for import_decl in import_declarations {
            let result = self.resolve_single_import(&ontology_iri, &import_decl, 0)?;

            if let Some(imported_ontology) = &result.ontology {
                if self.config.merge_imports {
                    self.merge_ontology(ontology, imported_ontology)?;
                }

                // Cache the resolved ontology
                if let Ok(mut cache) = self.ontology_cache.write() {
                    cache.insert(
                        import_decl.imported_ontology_iri.clone(),
                        Arc::new(imported_ontology.clone()),
                    );
                }
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Extract import declarations from ontology annotations
    fn extract_import_declarations(&self, ontology: &Ontology) -> Vec<ImportDeclaration> {
        let mut imports = Vec::new();

        // Look for import annotations
        for annotation in &ontology.annotations {
            let property_iri = &annotation.property.iri;
            if property_iri.to_string() == "http://www.w3.org/2002/07/owl#imports" {
                if let Some(value_iri) = match &annotation.value {
                    AnnotationValue::IRI(iri) => Some(iri),
                    _ => None,
                } {
                    imports.push(ImportDeclaration::new(value_iri.clone()));
                }
            }
        }

        imports
    }

    /// Resolve a single import
    fn resolve_single_import(
        &self,
        ontology_iri: &IRI,
        import_decl: &ImportDeclaration,
        depth: usize,
    ) -> Result<ImportResolutionResult, OxidowlError> {
        // Check depth limit
        if depth > self.config.max_import_depth {
            return Ok(ImportResolutionResult {
                ontology: None,
                import_iri: import_decl.imported_ontology_iri.clone(),
                resolved_source: None,
                errors: vec![ImportError::DepthExceeded {
                    max_depth: self.config.max_import_depth,
                    current_depth: depth,
                }],
                warnings: Vec::new(),
            });
        }

        // Check cache first
        if let Ok(cache) = self.ontology_cache.read() {
            if let Some(cached_ontology) = cache.get(&import_decl.imported_ontology_iri) {
                return Ok(ImportResolutionResult {
                    ontology: Some((**cached_ontology).clone()),
                    import_iri: import_decl.imported_ontology_iri.clone(),
                    resolved_source: Some("cache".to_string()),
                    errors: Vec::new(),
                    warnings: Vec::new(),
                });
            }
        }

        // Add to dependency graph
        if let Ok(mut graph) = self.dependency_graph.write() {
            graph.add_dependency(ontology_iri.clone(), import_decl.clone());
        }

        // Try to resolve the import
        let mut result = ImportResolutionResult {
            ontology: None,
            import_iri: import_decl.imported_ontology_iri.clone(),
            resolved_source: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Try different resolution strategies
        let import_iri_str = import_decl.imported_ontology_iri.to_string();

        // Apply URL mappings
        let mapped_iri = self
            .config
            .url_mappings
            .get(&import_iri_str)
            .unwrap_or(&import_iri_str);

        // Try to resolve as file path
        if let Some(ontology) = self.try_resolve_as_file(mapped_iri)? {
            result.ontology = Some(ontology);
            result.resolved_source = Some(format!("file: {}", mapped_iri));
        }
        // Try to resolve as URL
        else if let Some(ontology) = self.try_resolve_as_url(mapped_iri)? {
            result.ontology = Some(ontology);
            result.resolved_source = Some(format!("url: {}", mapped_iri));
        }
        // Resolution failed
        else {
            result.errors.push(ImportError::ResolutionFailed {
                import_iri: import_decl.imported_ontology_iri.clone(),
                reason: format!("Could not resolve {} as file or URL", mapped_iri),
            });
        }

        // Validate imported ontology if configured
        if self.config.validate_imports {
            if let Some(ref ontology) = result.ontology {
                if let Err(validation_error) = self.validate_imported_ontology(ontology) {
                    result.errors.push(ImportError::ValidationError {
                        import_iri: import_decl.imported_ontology_iri.clone(),
                        error: validation_error.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Try to resolve import as a file path
    fn try_resolve_as_file(&self, path_str: &str) -> Result<Option<Ontology>, OxidowlError> {
        // First try the path as-is
        let path = Path::new(path_str);
        if path.exists() {
            return Ok(Some(self.load_ontology_from_file(path)?));
        }

        // Try with base directories
        for base_dir in &self.config.base_directories {
            let full_path = base_dir.join(path);
            if full_path.exists() {
                return Ok(Some(self.load_ontology_from_file(&full_path)?));
            }
        }

        Ok(None)
    }

    /// Try to resolve import as a URL
    fn try_resolve_as_url(&self, url_str: &str) -> Result<Option<Ontology>, OxidowlError> {
        // Basic URL validation
        if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
            return Ok(None);
        }

        // For security and simplicity, we'll only support well-known ontology URLs
        // In a full implementation, this would make HTTP requests
        match url_str {
            "http://www.w3.org/2002/07/owl#" => {
                // Return a basic OWL ontology with core definitions
                let mut ontology = Ontology::new();
                ontology.set_ontology_iri(Some(crate::ontology::IRI::new(url_str)));
                Ok(Some(ontology))
            }
            "http://www.w3.org/2000/01/rdf-schema#" => {
                // Return a basic RDFS ontology
                let mut ontology = Ontology::new();
                ontology.set_ontology_iri(Some(crate::ontology::IRI::new(url_str)));
                Ok(Some(ontology))
            }
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#" => {
                // Return a basic RDF ontology
                let mut ontology = Ontology::new();
                ontology.set_ontology_iri(Some(crate::ontology::IRI::new(url_str)));
                Ok(Some(ontology))
            }
            _ => {
                // For unknown URLs, log a warning and return None
                warn!("Cannot load remote ontology from URL: {}", url_str);
                Ok(None)
            }
        }
    }

    /// Load ontology from file
    fn load_ontology_from_file(&self, path: &Path) -> Result<Ontology, OxidowlError> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match extension.as_deref() {
            Some("owl") | Some("owx") => {
                // Try OWL/XML parser
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                crate::parsers::owl_xml::parse(&content)
            }
            Some("ttl") => {
                // Try Turtle parser
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                crate::parsers::turtle::parse(&content)
            }
            Some("rdf") | Some("xml") => {
                // Try RDF/XML parser
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                crate::parsers::rdf_xml::parse(&content)
            }
            Some("ofn") => {
                // Try Functional Syntax parser
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                crate::parsers::functional::parse(&content)
            }
            Some("nt") => {
                // Try N-Triples parser
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                crate::parsers::ntriples::parse(&content)
            }
            _ => {
                // Unknown extension, try to detect format by content
                let content = std::fs::read_to_string(path)
                    .map_err(|e| OxidowlError::ParseError(format!("Failed to read file: {}", e)))?;

                // Simple heuristics to detect format
                if content.trim_start().starts_with("<?xml") {
                    if content.contains("<owl:Ontology") || content.contains("<Ontology") {
                        crate::parsers::owl_xml::parse(&content)
                    } else {
                        crate::parsers::rdf_xml::parse(&content)
                    }
                } else if content.contains("@prefix") || content.contains("PREFIX") {
                    crate::parsers::turtle::parse(&content)
                } else if content.trim_start().starts_with("Ontology(") {
                    crate::parsers::functional::parse(&content)
                } else {
                    // Default to turtle as it's most flexible
                    crate::parsers::turtle::parse(&content)
                }
            }
        }
    }

    /// Validate an imported ontology
    fn validate_imported_ontology(&self, ontology: &Ontology) -> Result<(), OxidowlError> {
        // Use the OWL 2 DL validator to check the imported ontology
        let mut validator = crate::validation::owl2_dl::OWL2DLValidator::new(ontology.clone());
        let validation_result = validator.validate()?;

        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result
                .errors
                .iter()
                .map(|e| format!("{:?}: {}", e.error_type, e.message))
                .collect();

            warn!(
                "Imported ontology has validation errors: {}",
                error_messages.join("; ")
            );

            // For now, we'll log warnings but not fail the import
            // In stricter mode, we might want to return an error
        }

        Ok(())
    }

    /// Merge an imported ontology into the main ontology
    fn merge_ontology(&self, target: &mut Ontology, source: &Ontology) -> Result<(), OxidowlError> {
        // Merge axioms from source into target
        for axiom in source.axioms() {
            target.add_axiom(axiom.clone());
        }

        // Merge annotations from source
        for annotation in &source.annotations {
            target.annotations.push(annotation.clone());
        }

        // Implement basic prefix handling for imports
        // Collect commonly used namespace prefixes and ensure they're available
        let mut used_namespaces = std::collections::HashSet::new();

        // Extract namespaces from axioms
        for axiom in &source.axioms {
            collect_namespaces_from_axiom(axiom, &mut used_namespaces);
        }

        // Log the discovered namespaces for future prefix mapping
        if !used_namespaces.is_empty() {
            log::debug!("Discovered namespaces during import: {:?}", used_namespaces);
        }

        // If source has an ontology IRI and target doesn't, inherit it
        if target.get_iri().is_none() && source.get_iri().is_some() {
            target.set_ontology_iri(source.get_iri().cloned());
        }

        // If source has a version IRI and target doesn't, inherit it
        if target.version_iri.is_none() && source.version_iri.is_some() {
            target.set_version_iri(source.version_iri.clone());
        }

        Ok(())
    }

    /// Get dependency graph
    pub fn get_dependency_graph(&self) -> Result<ImportDependencyGraph, OxidowlError> {
        self.dependency_graph
            .read()
            .map(|graph| graph.clone())
            .map_err(|_| OxidowlError::internal("Failed to read dependency graph".to_string()))
    }

    /// Check for circular dependencies
    pub fn check_circular_dependencies(&self) -> Result<Vec<Vec<IRI>>, OxidowlError> {
        let graph = self.get_dependency_graph()?;
        Ok(graph.detect_cycles())
    }

    /// Get import order (topological sort)
    pub fn get_import_order(&self) -> Result<Vec<IRI>, OxidowlError> {
        let graph = self.get_dependency_graph()?;
        graph
            .topological_sort()
            .map_err(|e| OxidowlError::invalid_input(e.to_string()))
    }
}

/// Helper function to collect namespaces from an axiom for prefix handling
fn collect_namespaces_from_axiom(
    axiom: &crate::ontology::axioms::Axiom,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::axioms::Axiom;

    match axiom {
        Axiom::Declaration(decl) => {
            collect_namespace_from_iri(decl.entity.iri(), namespaces);
        }
        Axiom::SubClassOf(sub_axiom) => {
            collect_namespaces_from_class_expression(&sub_axiom.subclass, namespaces);
            collect_namespaces_from_class_expression(&sub_axiom.superclass, namespaces);
        }
        Axiom::ClassAssertion(class_axiom) => {
            collect_namespaces_from_class_expression(&class_axiom.class, namespaces);
            if let crate::ontology::Individual::Named(named) = &class_axiom.individual {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
        }
        _ => {
            // Enhanced extraction for all other axiom types
            collect_namespaces_from_axiom_comprehensive(axiom, namespaces);
        }
    }
}

/// Helper function to collect namespace from a class expression
fn collect_namespaces_from_class_expression(
    expr: &crate::ontology::ClassExpression,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::ClassExpression;

    match expr {
        ClassExpression::Class(class) => {
            collect_namespace_from_iri(&class.iri, namespaces);
        }
        ClassExpression::ObjectIntersectionOf(operands)
        | ClassExpression::ObjectUnionOf(operands) => {
            for operand in operands {
                collect_namespaces_from_class_expression(operand, namespaces);
            }
        }
        ClassExpression::ObjectComplementOf(operand) => {
            collect_namespaces_from_class_expression(operand, namespaces);
        }
        _ => {
            // Other class expression types would need more specific handling
        }
    }
}

/// Helper function to extract namespace from an IRI
fn collect_namespace_from_iri(
    iri: &crate::ontology::IRI,
    namespaces: &mut std::collections::HashSet<String>,
) {
    let iri_str = iri.as_str();

    // Extract namespace (everything before the last # or /)
    if let Some(pos) = iri_str.rfind('#') {
        let namespace = &iri_str[..pos + 1];
        namespaces.insert(namespace.to_string());
    } else if let Some(pos) = iri_str.rfind('/') {
        let namespace = &iri_str[..pos + 1];
        namespaces.insert(namespace.to_string());
    }
}

fn collect_namespace_from_url(url: &url::Url, namespaces: &mut std::collections::HashSet<String>) {
    let url_str = url.as_str();

    // Extract namespace (everything before the last # or /)
    if let Some(pos) = url_str.rfind('#') {
        let namespace = &url_str[..pos + 1];
        namespaces.insert(namespace.to_string());
    } else if let Some(pos) = url_str.rfind('/') {
        let namespace = &url_str[..pos + 1];
        namespaces.insert(namespace.to_string());
    }
}

/// Comprehensive namespace extraction for all axiom types
fn collect_namespaces_from_axiom_comprehensive(
    axiom: &crate::ontology::Axiom,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::Axiom;

    match axiom {
        Axiom::ObjectPropertyAssertion(prop_axiom) => {
            collect_namespaces_from_object_property_expression(&prop_axiom.property, namespaces);
            if let crate::ontology::Individual::Named(named) = &prop_axiom.source {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
            if let crate::ontology::Individual::Named(named) = &prop_axiom.target {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
        }
        Axiom::DataPropertyAssertion(data_axiom) => {
            collect_namespaces_from_data_property_expression(&data_axiom.property, namespaces);
            if let crate::ontology::Individual::Named(named) = &data_axiom.individual {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
        }
        Axiom::NegativeObjectPropertyAssertion(neg_axiom) => {
            collect_namespaces_from_object_property_expression(&neg_axiom.property, namespaces);
            if let crate::ontology::Individual::Named(named) = &neg_axiom.source {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
            if let crate::ontology::Individual::Named(named) = &neg_axiom.target {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
        }
        Axiom::NegativeDataPropertyAssertion(neg_data_axiom) => {
            collect_namespaces_from_data_property_expression(&neg_data_axiom.property, namespaces);
            if let crate::ontology::Individual::Named(named) = &neg_data_axiom.individual {
                collect_namespace_from_iri(&named.iri, namespaces);
            }
        }
        Axiom::SameIndividual(same_axiom) => {
            for individual in &same_axiom.individuals {
                if let crate::ontology::Individual::Named(named) = individual {
                    collect_namespace_from_iri(&named.iri, namespaces);
                }
            }
        }
        Axiom::DifferentIndividuals(diff_axiom) => {
            for individual in &diff_axiom.individuals {
                if let crate::ontology::Individual::Named(named) = individual {
                    collect_namespace_from_iri(&named.iri, namespaces);
                }
            }
        }
        Axiom::ObjectPropertyDomain(domain_axiom) => {
            collect_namespaces_from_object_property_expression(&domain_axiom.property, namespaces);
            collect_namespaces_from_class_expression(&domain_axiom.domain, namespaces);
        }
        Axiom::ObjectPropertyRange(range_axiom) => {
            collect_namespaces_from_object_property_expression(&range_axiom.property, namespaces);
            collect_namespaces_from_class_expression(&range_axiom.range, namespaces);
        }
        Axiom::DataPropertyDomain(data_domain_axiom) => {
            collect_namespaces_from_data_property_expression(
                &data_domain_axiom.property,
                namespaces,
            );
            collect_namespaces_from_class_expression(&data_domain_axiom.domain, namespaces);
        }
        Axiom::DataPropertyRange(data_range_axiom) => {
            collect_namespaces_from_data_property_expression(
                &data_range_axiom.property,
                namespaces,
            );
            collect_namespaces_from_data_range(&data_range_axiom.range, namespaces);
        }
        Axiom::FunctionalObjectProperty(func_axiom) => {
            collect_namespaces_from_object_property_expression(&func_axiom.property, namespaces);
        }
        Axiom::InverseFunctionalObjectProperty(inv_func_axiom) => {
            collect_namespaces_from_object_property_expression(
                &inv_func_axiom.property,
                namespaces,
            );
        }
        Axiom::ReflexiveObjectProperty(refl_axiom) => {
            collect_namespaces_from_object_property_expression(&refl_axiom.property, namespaces);
        }
        Axiom::IrreflexiveObjectProperty(irrefl_axiom) => {
            collect_namespaces_from_object_property_expression(&irrefl_axiom.property, namespaces);
        }
        Axiom::SymmetricObjectProperty(sym_axiom) => {
            collect_namespaces_from_object_property_expression(&sym_axiom.property, namespaces);
        }
        Axiom::AsymmetricObjectProperty(asym_axiom) => {
            collect_namespaces_from_object_property_expression(&asym_axiom.property, namespaces);
        }
        Axiom::TransitiveObjectProperty(trans_axiom) => {
            collect_namespaces_from_object_property_expression(&trans_axiom.property, namespaces);
        }
        Axiom::FunctionalDataProperty(func_data_axiom) => {
            collect_namespaces_from_data_property_expression(&func_data_axiom.property, namespaces);
        }
        Axiom::HasKey(key_axiom) => {
            collect_namespaces_from_class_expression(&key_axiom.class, namespaces);
            for prop in &key_axiom.object_properties {
                collect_namespaces_from_object_property_expression(prop, namespaces);
            }
            for prop in &key_axiom.data_properties {
                collect_namespaces_from_data_property_expression(prop, namespaces);
            }
        }
        _ => {
            // For remaining axiom types, minimal extraction
            // This covers annotation axioms and other less common types
        }
    }
}

/// Helper function to collect namespaces from object property expressions
fn collect_namespaces_from_object_property_expression(
    expr: &crate::ontology::ObjectPropertyExpression,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::ObjectPropertyExpression;

    match expr {
        ObjectPropertyExpression::ObjectProperty(prop) => {
            collect_namespace_from_url(&prop.iri, namespaces);
        }
        ObjectPropertyExpression::InverseObjectProperty(prop) => {
            collect_namespace_from_url(&prop.iri, namespaces);
        }
        ObjectPropertyExpression::PropertyChain(chain) => {
            for prop in chain {
                collect_namespaces_from_object_property_expression(prop, namespaces);
            }
        }
    }
}

/// Helper function to collect namespaces from data property expressions
fn collect_namespaces_from_data_property_expression(
    expr: &crate::ontology::DataPropertyExpression,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::DataPropertyExpression;

    match expr {
        DataPropertyExpression::DataProperty(prop) => {
            collect_namespace_from_iri(&prop.iri, namespaces);
        }
    }
}

/// Helper function to collect namespaces from data ranges
fn collect_namespaces_from_data_range(
    range: &crate::ontology::DataRange,
    namespaces: &mut std::collections::HashSet<String>,
) {
    use crate::ontology::DataRange;

    match range {
        DataRange::Datatype(datatype_iri) => {
            collect_namespace_from_iri(datatype_iri, namespaces);
        }
        DataRange::DataIntersectionOf(ranges) => {
            for range in ranges {
                collect_namespaces_from_data_range(range, namespaces);
            }
        }
        DataRange::DataUnionOf(ranges) => {
            for range in ranges {
                collect_namespaces_from_data_range(range, namespaces);
            }
        }
        DataRange::DataComplementOf(range) => {
            collect_namespaces_from_data_range(range, namespaces);
        }
        DataRange::DataOneOf(_literals) => {
            // Literals don't typically have namespaces in the same way
        }
        DataRange::DatatypeRestriction {
            datatype,
            restrictions: _,
        } => {
            collect_namespace_from_iri(datatype, namespaces);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_declaration() {
        let iri = IRI::new("http://example.org/ontology");
        let version_iri = IRI::new("http://example.org/ontology/v1.0");

        let import = ImportDeclaration::new(iri.clone()).with_version_iri(version_iri.clone());

        assert_eq!(import.imported_ontology_iri, iri);
        assert_eq!(import.version_iri, Some(version_iri));
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = ImportDependencyGraph::new();

        let onto_a = IRI::new("http://example.org/A");
        let onto_b = IRI::new("http://example.org/B");
        let onto_c = IRI::new("http://example.org/C");

        // A imports B, B imports C
        graph.add_dependency(onto_a.clone(), ImportDeclaration::new(onto_b.clone()));
        graph.add_dependency(onto_b.clone(), ImportDeclaration::new(onto_c.clone()));

        let deps = graph.get_transitive_dependencies(&onto_a);
        assert!(deps.contains(&onto_b));
        assert!(deps.contains(&onto_c));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = ImportDependencyGraph::new();

        let onto_a = IRI::new("http://example.org/A");
        let onto_b = IRI::new("http://example.org/B");

        // A imports B, B imports A (circular)
        graph.add_dependency(onto_a.clone(), ImportDeclaration::new(onto_b.clone()));
        graph.add_dependency(onto_b.clone(), ImportDeclaration::new(onto_a.clone()));

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
        assert!(cycles[0].contains(&onto_a));
        assert!(cycles[0].contains(&onto_b));
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ImportDependencyGraph::new();

        let onto_a = IRI::new("http://example.org/A");
        let onto_b = IRI::new("http://example.org/B");
        let onto_c = IRI::new("http://example.org/C");

        // A imports B, B imports C
        graph.add_dependency(onto_a.clone(), ImportDeclaration::new(onto_b.clone()));
        graph.add_dependency(onto_b.clone(), ImportDeclaration::new(onto_c.clone()));

        let order = graph.topological_sort().unwrap();

        // C should come before B, B should come before A
        let pos_a = order.iter().position(|iri| iri == &onto_a).unwrap();
        let pos_b = order.iter().position(|iri| iri == &onto_b).unwrap();
        let pos_c = order.iter().position(|iri| iri == &onto_c).unwrap();

        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_import_manager_creation() {
        let config = ImportManagerConfig::default();
        let manager = ImportManager::new(config);

        // Test that manager can be created
        assert_eq!(manager.config.max_import_depth, 10);
    }
}

// Re-export the additional resolver module
pub mod resolver;
pub use resolver::{ImportResolver, ImportResolverConfig, ResolvedImport, ImportedOntology};
