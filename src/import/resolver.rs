//! Import Resolution System
//!
//! This module handles resolving and loading imported ontologies.

use crate::{Error, Result};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use url::Url;

/// Import resolution service
#[derive(Debug)]
pub struct ImportResolver {
    /// Mapping from IRIs to resolved locations
    iri_mappings: Arc<RwLock<HashMap<String, String>>>,
    /// Cache of loaded ontologies
    ontology_cache: Arc<RwLock<HashMap<String, CachedOntology>>>,
    /// Base directories for relative imports
    base_directories: Vec<PathBuf>,
    /// Whether to allow remote imports
    allow_remote: bool,
    /// Import recursion depth limit
    max_depth: usize,
}

impl ImportResolver {
    /// Create a new import resolver
    #[must_use] 
    pub fn new() -> Self {
        Self {
            iri_mappings: Arc::new(RwLock::new(HashMap::new())),
            ontology_cache: Arc::new(RwLock::new(HashMap::new())),
            base_directories: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            allow_remote: true,
            max_depth: 10,
        }
    }

    /// Configure the resolver
    #[must_use] 
    pub fn with_config(mut self, config: ImportResolverConfig) -> Self {
        self.base_directories = config.base_directories;
        self.allow_remote = config.allow_remote;
        self.max_depth = config.max_depth;
        self
    }

    /// Add IRI mapping
    pub async fn add_iri_mapping(&self, iri: String, location: String) -> Result<()> {
        let mut mappings = self.iri_mappings.write().await;
        mappings.insert(iri, location);
        Ok(())
    }

    /// Resolve import IRI to actual location
    #[must_use] 
    pub fn resolve_import<'a>(
        &'a self,
        import_iri: &'a str,
        base_iri: Option<&'a str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ResolvedImport>> + Send + 'a>>
    {
        Box::pin(async move {
            // Check mappings first
            {
                let mappings = self.iri_mappings.read().await;
                if let Some(mapped_location) = mappings.get(import_iri) {
                    return self
                        .create_resolved_import(import_iri, mapped_location)
                        .await;
                }
            }

            // Try to resolve as absolute URL
            if let Ok(url) = Url::parse(import_iri) {
                match url.scheme() {
                    "http" | "https" => {
                        if self.allow_remote {
                            return self.create_resolved_import(import_iri, import_iri).await;
                        } else {
                            return Err(Error::import_error(format!(
                                "Remote imports disabled: {import_iri}"
                            )));
                        }
                    }
                    "file" => {
                        let path = url.to_file_path().map_err(|_| {
                            Error::import_error(format!("Invalid file URL: {import_iri}"))
                        })?;
                        return self
                            .create_resolved_import(import_iri, &path.to_string_lossy())
                            .await;
                    }
                    _ => {
                        return Err(Error::import_error(format!(
                            "Unsupported URL scheme: {}",
                            url.scheme()
                        )));
                    }
                }
            }

            // Try relative resolution
            if let Some(base) = base_iri
                && let Ok(base_url) = Url::parse(base)
                    && let Ok(resolved_url) = base_url.join(import_iri) {
                        return self.resolve_import(resolved_url.as_ref(), None).await;
                    }

            // Try file system resolution
            for base_dir in &self.base_directories {
                let candidate_path = base_dir.join(import_iri);
                if candidate_path.exists() {
                    return self
                        .create_resolved_import(import_iri, &candidate_path.to_string_lossy())
                        .await;
                }

                // Try with common extensions
                for ext in &[".owl", ".rdf", ".ttl", ".n3"] {
                    let with_ext = candidate_path.with_extension(&ext[1..]);
                    if with_ext.exists() {
                        return self
                            .create_resolved_import(import_iri, &with_ext.to_string_lossy())
                            .await;
                    }
                }
            }

            Err(Error::import_error(format!(
                "Cannot resolve import: {import_iri}"
            )))
        })
    }

    /// Load ontology with imports
    pub async fn load_with_imports(
        &self,
        ontology_iri: &str,
        max_depth: Option<usize>,
    ) -> Result<ImportedOntology> {
        let max_depth = max_depth.unwrap_or(self.max_depth);
        let mut loaded = HashSet::new();
        let mut pending = vec![(ontology_iri.to_string(), 0)];
        let mut imports = HashMap::new();

        while let Some((iri, depth)) = pending.pop() {
            if depth > max_depth {
                return Err(Error::import_error(format!(
                    "Import depth limit exceeded: {max_depth}"
                )));
            }

            if loaded.contains(&iri) {
                continue;
            }

            // Check cache first
            {
                let cache = self.ontology_cache.read().await;
                if let Some(cached) = cache.get(&iri) {
                    imports.insert(iri.clone(), cached.clone());
                    loaded.insert(iri);
                    continue;
                }
            }

            // Resolve and load the ontology
            let resolved = self.resolve_import(&iri, None).await?;
            let content = self.load_content(&resolved).await?;
            let import_iris = self.extract_imports(&content)?;

            let cached_ontology = CachedOntology {
                iri: iri.clone(),
                location: resolved.location.clone(),
                content: content.clone(),
                imports: import_iris.clone(),
                loaded_at: std::time::SystemTime::now(),
            };

            // Cache the ontology
            {
                let mut cache = self.ontology_cache.write().await;
                cache.insert(iri.clone(), cached_ontology.clone());
            }

            imports.insert(iri.clone(), cached_ontology);
            loaded.insert(iri);

            // Add imports to pending queue
            for import_iri in import_iris {
                if !loaded.contains(&import_iri) {
                    pending.push((import_iri, depth + 1));
                }
            }
        }

        Ok(ImportedOntology {
            root_iri: ontology_iri.to_string(),
            imports,
        })
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.ontology_cache.write().await;
        cache.clear();
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.ontology_cache.read().await;
        CacheStats {
            cached_ontologies: cache.len(),
            total_size_bytes: cache.values().map(|ont| ont.content.len()).sum(),
        }
    }

    // Helper methods

    async fn create_resolved_import(&self, iri: &str, location: &str) -> Result<ResolvedImport> {
        Ok(ResolvedImport {
            original_iri: iri.to_string(),
            location: location.to_string(),
            import_type: self.determine_import_type(location),
        })
    }

    fn determine_import_type(&self, location: &str) -> ImportType {
        if location.starts_with("http://") || location.starts_with("https://") {
            ImportType::Remote
        } else {
            ImportType::Local
        }
    }

    async fn load_content(&self, resolved: &ResolvedImport) -> Result<String> {
        match resolved.import_type {
            ImportType::Local => tokio::fs::read_to_string(&resolved.location)
                .await
                .map_err(|e| {
                    Error::import_error(format!(
                        "Failed to read local file {}: {}",
                        resolved.location, e
                    ))
                }),
            ImportType::Remote => {
                // For now, return error - would implement HTTP client
                Err(Error::import_error(format!(
                    "Remote import loading not yet implemented: {}",
                    resolved.location
                )))
            }
        }
    }

    fn extract_imports(&self, content: &str) -> Result<Vec<String>> {
        // Simplified import extraction - would use proper parser
        let mut imports = Vec::new();

        // Look for owl:imports patterns
        for line in content.lines() {
            if line.contains("owl:imports") {
                // Very basic extraction - would need proper parsing
                if let Some(start) = line.find('"')
                    && let Some(end) = line[start + 1..].find('"') {
                        let import_iri = &line[start + 1..start + 1 + end];
                        imports.push(import_iri.to_string());
                    }
            }
        }

        Ok(imports)
    }
}

impl Default for ImportResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for import resolver
#[derive(Debug, Clone)]
pub struct ImportResolverConfig {
    /// Base directories for resolving relative imports
    pub base_directories: Vec<PathBuf>,
    /// Whether to allow remote imports
    pub allow_remote: bool,
    /// Maximum import depth
    pub max_depth: usize,
}

impl Default for ImportResolverConfig {
    fn default() -> Self {
        Self {
            base_directories: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            allow_remote: true,
            max_depth: 10,
        }
    }
}

/// Resolved import information
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// Original import IRI
    pub original_iri: String,
    /// Resolved location (file path or URL)
    pub location: String,
    /// Type of import
    pub import_type: ImportType,
}

/// Type of import
#[derive(Debug, Clone, PartialEq)]
pub enum ImportType {
    /// Local file
    Local,
    /// Remote URL
    Remote,
}

/// Cached ontology
#[derive(Debug, Clone)]
pub struct CachedOntology {
    /// Ontology IRI
    pub iri: String,
    /// Location where it was loaded from
    pub location: String,
    /// Ontology content
    pub content: String,
    /// Import IRIs found in this ontology
    pub imports: Vec<String>,
    /// When it was loaded
    pub loaded_at: std::time::SystemTime,
}

/// Imported ontology with all its dependencies
#[derive(Debug)]
pub struct ImportedOntology {
    /// Root ontology IRI
    pub root_iri: String,
    /// Map of all imported ontologies (including transitive imports)
    pub imports: HashMap<String, CachedOntology>,
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    /// Number of cached ontologies
    pub cached_ontologies: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
}
