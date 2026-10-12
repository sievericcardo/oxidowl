//! Ontology Loader — convenience methods for loading ontologies from
//! various sources and all supported formats.

use crate::Result;
use crate::manager::OntologyManager;
use crate::manager::loader_config::{LoaderConfig, MissingImportStrategy};
use crate::ontology::{IRI, Ontology, OntologyFormat, OntologyRef};
use crate::parsers;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

/// A convenience wrapper for loading ontologies from files, URLs,
/// compressed sources, or in-memory strings.
pub struct OntologyLoader {
    /// The manager that will own loaded ontologies.
    manager: Arc<RwLock<OntologyManager>>,
    /// Loader configuration (timeouts, retries, etc.).
    config: LoaderConfig,
}

impl std::fmt::Debug for OntologyLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OntologyLoader").finish_non_exhaustive()
    }
}

impl OntologyLoader {
    /// Create a new loader backed by the given manager with default config.
    #[must_use]
    pub fn new(manager: Arc<RwLock<OntologyManager>>) -> Self {
        let config = LoaderConfig::default();
        Self { manager, config }
    }

    /// Create a new loader with an explicit `LoaderConfig`.
    #[must_use]
    pub fn new_with_config(manager: Arc<RwLock<OntologyManager>>, config: LoaderConfig) -> Self {
        Self { manager, config }
    }

    /// Get the loader configuration.
    #[must_use]
    pub fn config(&self) -> &LoaderConfig {
        &self.config
    }

    /// Get a mutable reference to the loader configuration.
    pub fn config_mut(&mut self) -> &mut LoaderConfig {
        &mut self.config
    }

    /// Load an ontology from a file, auto-detecting the format.
    /// The ontology is parsed, registered with the manager, and returned.
    ///
    /// Detection works on raw bytes so binary formats (`BinaryRdf`, `Hdt`)
    /// are recognised by their magic headers instead of failing a
    /// `read_to_string` (B5 format registry).
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<OntologyRef> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .map_err(|e| crate::Error::io(format!("Failed to read {}: {e}", path.display())))?;
        let format =
            parsers::detect_format_from_bytes(path, &bytes).unwrap_or(OntologyFormat::Functional);
        // Reuse the already-read bytes — `load_file_with_format` would
        // otherwise read the file a second time.
        let ontology = parse_bytes_with_format(&bytes, format)?;
        self.register_with_import_resolution(ontology)
    }

    /// Load an ontology from a file with an explicit format hint.
    pub fn load_file_with_format<P: AsRef<Path>>(
        &self,
        path: P,
        format: OntologyFormat,
    ) -> Result<OntologyRef> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .map_err(|e| crate::Error::io(format!("Failed to read {}: {e}", path.display())))?;
        let ontology = parse_bytes_with_format(&bytes, format)?;
        self.register_with_import_resolution(ontology)
    }

    /// Load an ontology from an in-memory string with explicit format.
    /// The `label` is used as a fallback document IRI when the content does not
    /// declare an ontology IRI.
    pub fn load_from_string(
        &self,
        content: &str,
        format: OntologyFormat,
        label: &str,
    ) -> Result<OntologyRef> {
        let parser = parsers::ParserFactory::create_parser(format)?;
        let ontology = parser.parse(content)?;
        let mut ont = ontology;
        if ont.get_iri().is_none() {
            ont.set_iri(IRI::new(&format!("urn:document:{label}")));
        }
        self.register_with_import_resolution(ont)
    }

    /// Load from a gzip-compressed file.
    pub fn load_gzip<P: AsRef<Path>>(&self, path: P) -> Result<OntologyRef> {
        let path = path.as_ref().to_path_buf();
        let compressed = std::fs::read(&path)
            .map_err(|e| crate::Error::io(format!("Failed to read gzip: {e}")))?;
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut content = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut content)
            .map_err(|e| crate::Error::io(format!("Failed to decompress: {e}")))?;

        let format = parsers::detect_format_from_content_public(&path, &content)
            .unwrap_or(OntologyFormat::Functional);
        self.load_from_string(&content, format, &path.to_string_lossy())
    }

    /// Load from an in-memory gzip-compressed buffer.
    pub fn load_gzip_buffer(&self, compressed: &[u8], label: &str) -> Result<OntologyRef> {
        let mut decoder = flate2::read::GzDecoder::new(compressed);
        let mut content = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut content)
            .map_err(|e| crate::Error::io(format!("Failed to decompress: {e}")))?;

        let path = PathBuf::from(label);
        let format = parsers::detect_format_from_content_public(&path, &content)
            .unwrap_or(OntologyFormat::Functional);
        self.load_from_string(&content, format, label)
    }

    /// Load from an xz-compressed file, auto-detecting the format.
    pub fn load_xz<P: AsRef<Path>>(&self, path: P) -> Result<OntologyRef> {
        let path = path.as_ref().to_path_buf();
        let content = parsers::load_xz_to_string(&path)?;

        let format = parsers::detect_format_from_content_public(&path, &content)
            .unwrap_or(OntologyFormat::Functional);
        self.load_from_string(&content, format, &path.to_string_lossy())
    }

    #[cfg(feature = "http-imports")]
    /// Load an ontology from an HTTP(S) URL.
    pub fn load_from_url(&self, url: &url::Url) -> Result<OntologyRef> {
        let mut last_error = None;

        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                let delay = self.config.retry_backoff * 2u32.pow(attempt - 1);
                log::info!(
                    "Retrying load from {url} (attempt {attempt}/{}) after {delay:?}",
                    self.config.retry_count
                );
                std::thread::sleep(delay);
            }

            let client = reqwest::blocking::Client::builder()
                .timeout(self.config.read_timeout)
                .connect_timeout(self.config.connection_timeout)
                .build()
                .map_err(|e| crate::Error::network(format!("Failed to build HTTP client: {e}")))?;

            match client.get(url.clone()).send() {
                Ok(response) => {
                    if !response.status().is_success() {
                        last_error = Some(crate::Error::network(format!(
                            "HTTP {}: {url}",
                            response.status().as_u16()
                        )));
                        continue;
                    }
                    match response.text() {
                        Ok(content) => {
                            let path = PathBuf::from(url.path());
                            let format =
                                parsers::detect_format_from_content_public(&path, &content)
                                    .unwrap_or(OntologyFormat::Functional);
                            return self.load_from_string(&content, format, url.path());
                        }
                        Err(e) => {
                            last_error = Some(crate::Error::network(format!(
                                "Failed to read response body: {e}"
                            )));
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(crate::Error::network(format!("HTTP request failed: {e}")));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| crate::Error::network(format!("All retries exhausted for {url}"))))
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// Register an ontology with the manager and, when import resolution is
    /// enabled, recursively pull in its `owl:imports` closure.
    fn register_with_import_resolution(&self, ontology: Ontology) -> Result<OntologyRef> {
        let ont_ref = self.register(ontology)?;
        if self.config.resolve_imports {
            let mut visited = HashSet::new();
            self.resolve_imports_recursive(&ont_ref, 0, &mut visited)?;
        }
        Ok(ont_ref)
    }

    /// Register an ontology with the manager and return the shared ref.
    fn register(&self, ontology: Ontology) -> Result<OntologyRef> {
        let iri = ontology
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:anonymous"));
        let ont_ref = OntologyRef::new(RwLock::new(ontology));

        if let Ok(mut manager) = self.manager.write() {
            manager.register_ontology(ont_ref.clone());
            if let Ok(guard) = ont_ref.read() {
                for import in &guard.imports {
                    let () = manager.add_import(iri.clone(), import.imported_ontology_iri.clone());
                }
            }
        }
        Ok(ont_ref)
    }

    // ── Import resolution ─────────────────────────────────────────────────

    /// Recursively resolve the `owl:imports` of an ontology, loading and
    /// registering each imported ontology with the manager.
    ///
    /// Document IRIs are resolved through the manager's IRI mappers first,
    /// then via direct file-path lookup, then (with the `http-imports`
    /// feature) via HTTP(S).
    fn resolve_imports_recursive(
        &self,
        ont_ref: &OntologyRef,
        depth: usize,
        visited: &mut HashSet<IRI>,
    ) -> Result<()> {
        if depth > self.config.max_import_depth {
            return Ok(());
        }

        let import_iris: Vec<IRI> = {
            let guard = ont_ref
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard
                .imports
                .iter()
                .map(|decl| decl.imported_ontology_iri.clone())
                .collect()
        };

        for import_iri in import_iris {
            if !visited.insert(import_iri.clone()) {
                continue;
            }

            // Already loaded?
            {
                let manager = self
                    .manager
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if manager.contains_ontology(&import_iri) {
                    continue;
                }
            }

            let resolved = self.resolve_import_document(&import_iri);
            match resolved {
                Ok(Some((content, format, label))) => {
                    let imported = self.load_from_string(&content, format, &label)?;
                    self.resolve_imports_recursive(&imported, depth + 1, visited)?;
                }
                Ok(None) | Err(_) => match self.config.missing_import_strategy {
                    MissingImportStrategy::ThrowException => {
                        let reason = match resolved {
                            Err(e) => e.to_string(),
                            _ => format!("Could not resolve {import_iri} as file or URL"),
                        };
                        return Err(crate::Error::invalid_input(format!(
                            "Failed to resolve import {import_iri}: {reason}"
                        )));
                    }
                    MissingImportStrategy::Silent => {
                        log::warn!("Skipping unresolved import: {import_iri}");
                    }
                },
            }
        }

        Ok(())
    }

    /// Resolve a single import IRI to its document content.
    ///
    /// Returns `(content, format, label)` when the document was found.
    fn resolve_import_document(
        &self,
        import_iri: &IRI,
    ) -> Result<Option<(String, OntologyFormat, String)>> {
        // 1. Try the manager's IRI mappers (logical IRI -> document IRI).
        let doc_iri = {
            let manager = self
                .manager
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            manager.resolve_document_iri(import_iri)
        };

        if let Some(doc_iri) = doc_iri {
            if let Some(path) = file_path_from_iri(&doc_iri) {
                if path.exists() {
                    return self.read_document(&path, &doc_iri.to_string());
                }
            } else if doc_iri.as_str().starts_with("http://")
                || doc_iri.as_str().starts_with("https://")
            {
                #[cfg(feature = "http-imports")]
                {
                    if let Ok(url) = url::Url::parse(doc_iri.as_str()) {
                        return self.load_from_url_internal(&url);
                    }
                }
                return Ok(None);
            }
        }

        // 2. Try the import IRI as a direct file path.
        let raw = import_iri.as_str();
        let direct_path = PathBuf::from(raw);
        if direct_path.exists() {
            return self.read_document(&direct_path, raw);
        }

        // 3. Try resolving a `file://` import IRI.
        if let Some(path) = file_path_from_iri(import_iri)
            && path.exists()
        {
            return self.read_document(&path, import_iri.as_str());
        }

        // 4. Try HTTP(S) import IRIs.
        if raw.starts_with("http://") || raw.starts_with("https://") {
            #[cfg(feature = "http-imports")]
            {
                if let Ok(url) = url::Url::parse(raw) {
                    return self.load_from_url_internal(&url);
                }
            }
        }

        Ok(None)
    }

    /// Read and parse a local document for an import.
    fn read_document(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<Option<(String, OntologyFormat, String)>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::io(format!("Failed to read {}: {e}", path.display())))?;
        let format = parsers::detect_format_from_content_public(path, &content)
            .unwrap_or(OntologyFormat::Functional);
        Ok(Some((content, format, label.to_string())))
    }

    /// Fetch a document over HTTP(S) (feature-gated).
    #[cfg(feature = "http-imports")]
    fn load_from_url_internal(
        &self,
        url: &url::Url,
    ) -> Result<Option<(String, OntologyFormat, String)>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.config.read_timeout)
            .connect_timeout(self.config.connection_timeout)
            .build()
            .map_err(|e| crate::Error::network(format!("Failed to build HTTP client: {e}")))?;
        let response = client
            .get(url.clone())
            .send()
            .map_err(|e| crate::Error::network(format!("HTTP request failed: {e}")))?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let content = response
            .text()
            .map_err(|e| crate::Error::network(format!("Failed to read response body: {e}")))?;
        let path = PathBuf::from(url.path());
        let format = parsers::detect_format_from_content_public(&path, &content)
            .unwrap_or(OntologyFormat::Functional);
        Ok(Some((content, format, url.path().to_string())))
    }
}

/// Extract a local file path from a `file://` document IRI.
fn file_path_from_iri(iri: &IRI) -> Option<PathBuf> {
    let s = iri.as_str();
    if s.starts_with("file://") {
        url::Url::parse(s).ok().and_then(|u| u.to_file_path().ok())
    } else if s.starts_with('/') {
        Some(PathBuf::from(s))
    } else {
        None
    }
}

/// Parse raw file bytes in an explicit format.
///
/// Binary formats (`BinaryRdf`, `Hdt`) take the byte path so non-UTF-8
/// payloads survive; everything else must be valid UTF-8 and is handed to the
/// `ParserFactory` bridge (which covers all RIO text formats after the B5
/// format-registry work).
fn parse_bytes_with_format(bytes: &[u8], format: OntologyFormat) -> Result<Ontology> {
    match format {
        OntologyFormat::BinaryRdf | OntologyFormat::Hdt => parsers::parse_bytes(bytes, format),
        _ => {
            let content = std::str::from_utf8(bytes).map_err(|e| {
                crate::Error::io(format!("Ontology content is not valid UTF-8: {e}"))
            })?;
            let parser = parsers::ParserFactory::create_parser(format)?;
            parser.parse(content)
        }
    }
}
