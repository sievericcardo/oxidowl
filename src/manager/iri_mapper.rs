//! OWL Ontology IRI Mapper
//!
//! Provides IRI-to-document resolution for loading ontologies when the
//! logical IRI does not match the physical location.

use crate::ontology::IRI;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maps ontology IRIs to document IRIs (URLs or local paths).
/// Used for loading ontologies when the logical IRI does not match
/// the physical location.
pub trait OntologyIRIMapper: Send + Sync {
    /// Resolve an ontology IRI to a document IRI.
    /// Returns `None` if this mapper cannot resolve the IRI.
    fn get_document_iri(&self, ontology_iri: &IRI) -> Option<IRI>;

    /// Returns a human-readable name for this mapper.
    fn name(&self) -> &str;
}

// ── SimpleIRIMapper ──────────────────────────────────────────────────────────

/// Static one-to-one mapping from ontology IRI to document IRI.
///
/// Example:
/// ```rust,ignore
/// let mapper = SimpleIRIMapper::new(
///     IRI::new("http://example.org/ont"),
///     IRI::new("file:///path/to/ont.owl"),
/// );
/// ```
pub struct SimpleIRIMapper {
    name: String,
    mapping: HashMap<IRI, IRI>,
}

impl SimpleIRIMapper {
    /// Create a new mapper with a single ontology-to-document mapping.
    #[must_use]
    pub fn new(ontology_iri: IRI, document_iri: IRI) -> Self {
        let mut mapping = HashMap::with_capacity(1);
        mapping.insert(ontology_iri.clone(), document_iri);
        Self {
            name: format!("Simple({ontology_iri})"),
            mapping,
        }
    }

    /// Create a mapper from a pre-built mapping table.
    #[must_use]
    pub fn from_map(name: &str, mapping: HashMap<IRI, IRI>) -> Self {
        Self {
            name: name.to_string(),
            mapping,
        }
    }

    /// Add a single mapping entry.
    pub fn add_mapping(&mut self, ontology_iri: IRI, document_iri: IRI) {
        self.mapping.insert(ontology_iri, document_iri);
    }
}

impl OntologyIRIMapper for SimpleIRIMapper {
    fn get_document_iri(&self, ontology_iri: &IRI) -> Option<IRI> {
        self.mapping.get(ontology_iri).cloned()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ── AutoIRIMapper ────────────────────────────────────────────────────────────

/// Scans a directory for ontology files, mapping their internal ontology IRI
/// to the file path.
///
/// Example: If `/path/to/ont.owl` contains `http://example.org/ont`,
/// then `AutoIRIMapper::new("/path/to")` will resolve
/// `http://example.org/ont` → `file:///path/to/ont.owl`.
pub struct AutoIRIMapper {
    directory: PathBuf,
    suffix: String,
    /// Interior mutability for lazy-init scan.
    inner: std::sync::Mutex<AutoIRIMapperInner>,
}

struct AutoIRIMapperInner {
    mapping: HashMap<IRI, IRI>,
    scanned: bool,
}

impl AutoIRIMapper {
    /// Create a new auto-mapper for the given directory.
    /// The directory will be scanned lazily on first use.
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            suffix: String::new(),
            inner: std::sync::Mutex::new(AutoIRIMapperInner {
                mapping: HashMap::new(),
                scanned: false,
            }),
        }
    }

    /// Set a file suffix filter (e.g., ".owl", ".ttl").
    #[must_use]
    pub fn with_suffix(mut self, suffix: &str) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Ensure the directory has been scanned (lazy init).
    fn ensure_scanned(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if inner.scanned {
            return;
        }
        inner.scanned = true;

        let Ok(entries) = std::fs::read_dir(&self.directory) else {
            return;
        };
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !self.suffix.is_empty() {
                let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                if !ext.eq_ignore_ascii_case(self.suffix.trim_start_matches('.')) {
                    continue;
                }
            }
            if let Some(iri) = Self::extract_ontology_iri(&path) {
                let file_url = format!("file://{}", path.display());
                inner.mapping.insert(iri, IRI::new(&file_url));
            }
        }
    }

    /// Attempt to extract the ontology IRI from an ontology file.
    fn extract_ontology_iri(path: &Path) -> Option<IRI> {
        let content = std::fs::read_to_string(path).ok()?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("rdf:type") && trimmed.contains("owl:Ontology")
                && let Some(start) = trimmed.find('<')
                    && let Some(end) = trimmed[start..].find('>')
                {
                    let iri_str = &trimmed[start + 1..start + end];
                    if iri_str.starts_with("http") || iri_str.starts_with("https") {
                        return Some(IRI::new(iri_str));
                    }
                }
        }
        None
    }
}

impl OntologyIRIMapper for AutoIRIMapper {
    fn get_document_iri(&self, ontology_iri: &IRI) -> Option<IRI> {
        self.ensure_scanned();
        let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.mapping.get(ontology_iri).cloned()
    }

    fn name(&self) -> &str {
        "AutoIRIMapper"
    }
}

// ── NonMappingOntologyIRIMapper ──────────────────────────────────────────────

/// A mapper that returns `None` for every request.
/// Used as a null/default implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonMappingOntologyIRIMapper;

impl OntologyIRIMapper for NonMappingOntologyIRIMapper {
    fn get_document_iri(&self, _ontology_iri: &IRI) -> Option<IRI> {
        None
    }

    fn name(&self) -> &str {
        "NonMappingIRIMapper"
    }
}

// ── CompositeIRIMapper ───────────────────────────────────────────────────────

/// A mapper that tries a sequence of mappers in order.
/// Returns the result from the first mapper that succeeds.
pub struct CompositeIRIMapper {
    mappers: Vec<Box<dyn OntologyIRIMapper>>,
}

impl CompositeIRIMapper {
    /// Create a composite mapper from a list of mappers.
    #[must_use]
    pub fn new(mappers: Vec<Box<dyn OntologyIRIMapper>>) -> Self {
        Self { mappers }
    }

    /// Add a mapper to the end of the chain.
    pub fn add_mapper(&mut self, mapper: Box<dyn OntologyIRIMapper>) {
        self.mappers.push(mapper);
    }
}

impl OntologyIRIMapper for CompositeIRIMapper {
    fn get_document_iri(&self, ontology_iri: &IRI) -> Option<IRI> {
        for mapper in &self.mappers {
            if let Some(doc_iri) = mapper.get_document_iri(ontology_iri) {
                return Some(doc_iri);
            }
        }
        None
    }

    fn name(&self) -> &str {
        "CompositeIRIMapper"
    }
}

// ── ZipIRIMapper ─────────────────────────────────────────────────────────────

/// Maps ontology IRIs to document IRIs inside a ZIP archive.
/// Scans the entries of a ZIP file to build a mapping.
pub struct ZipIRIMapper {
    /// Maps ontology IRI -> ZIP entry path
    mappings: HashMap<IRI, String>,
    /// The path to the ZIP file for extraction
    zip_path: PathBuf,
}

impl ZipIRIMapper {
    /// Create a new ZipIRIMapper by scanning a ZIP file.
    /// Each entry whose name ends with `.owl`, `.rdf`, `.ttl`, `.ofn`, `.owx`,
    /// or `.omn` is considered a potential ontology file.
    pub fn new(zip_path: PathBuf) -> Self {
        let mappings = HashMap::new();

        if let Ok(file) = std::fs::File::open(&zip_path) {
            #[cfg(feature = "zip-imports")]
            {
                if let Ok(mut archive) = zip::ZipArchive::new(file) {
                    let ontology_extensions = [".owl", ".rdf", ".ttl", ".ofn", ".owx", ".omn"];

                    for i in 0..archive.len() {
                        if let Ok(entry) = archive.by_index(i) {
                            let name = entry.name().to_string();
                            if ontology_extensions.iter().any(|ext| name.ends_with(ext)) {
                                let local_name = std::path::Path::new(&name)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&name);
                                let iri = IRI::new(&format!("http://example.org/{local_name}"));
                                mappings.insert(iri, name);
                            }
                        }
                    }
                }
            }

            #[cfg(not(feature = "zip-imports"))]
            {
                log::warn!(
                    "ZipIRIMapper: zip-imports feature not enabled. ZIP scanning is unavailable."
                );
                let _ = file;
            }
        } else {
            log::warn!("ZipIRIMapper: ZIP file not found at {}", zip_path.display());
        }

        ZipIRIMapper { mappings, zip_path }
    }

    /// Resolve an ontology IRI to its entry within the ZIP archive.
    /// Returns the entry name if found.
    #[must_use]
    pub fn resolve(&self, ontology_iri: &IRI) -> Option<&str> {
        self.mappings.get(ontology_iri).map(std::string::String::as_str)
    }

    /// Get the underlying ZIP file path.
    #[must_use]
    pub fn zip_path(&self) -> &PathBuf {
        &self.zip_path
    }

    /// List all known ontology IRI -> entry mappings.
    #[must_use]
    pub fn mappings(&self) -> &HashMap<IRI, String> {
        &self.mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_iri_mapper_creation() {
        let mapper = ZipIRIMapper::new(PathBuf::from("nonexistent.zip"));
        assert!(mapper.mappings().is_empty());
    }
}
