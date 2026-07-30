//! Ontology Loader — convenience methods for loading ontologies from
//! various sources and all supported formats.

use crate::Result;
use crate::manager::OntologyManager;
use crate::manager::loader_config::LoaderConfig;
use crate::ontology::{IRI, Ontology, OntologyFormat, OntologyRef};
use crate::parsers;
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

    /// Create a new loader with an explicit LoaderConfig.
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
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<OntologyRef> {
        let format = {
            let temp_path = path.as_ref().to_path_buf();
            parsers::detect_format_from_content_public(
                &temp_path,
                &std::fs::read_to_string(&temp_path).unwrap_or_default(),
            )
            .unwrap_or(OntologyFormat::Functional)
        };
        self.load_file_with_format(path, format)
    }

    /// Load an ontology from a file with an explicit format hint.
    pub fn load_file_with_format<P: AsRef<Path>>(
        &self,
        path: P,
        format: OntologyFormat,
    ) -> Result<OntologyRef> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::io(format!("Failed to read {}: {e}", path.display())))?;
        let ontology = self.parse_content(&content, format, path)?;
        self.register(ontology)
    }

    /// Load an ontology from an in-memory string with explicit format.
    /// The `label` is used as a descriptive label for the document IRI.
    pub fn load_from_string(
        &self,
        content: &str,
        format: OntologyFormat,
        label: &str,
    ) -> Result<OntologyRef> {
        let parser = parsers::ParserFactory::create_parser(format)?;
        let ontology = parser.parse(content)?;
        let mut ont = ontology;
        ont.set_iri(IRI::new(&format!("urn:document:{label}")));
        self.register(ont)
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

    /// Parse content with the given format and path context.
    fn parse_content(
        &self,
        content: &str,
        format: OntologyFormat,
        _path: &Path,
    ) -> Result<Ontology> {
        let parser = parsers::ParserFactory::create_parser(format)?;
        parser.parse(content)
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
                    let _ = manager.add_import(iri.clone(), import.imported_ontology_iri.clone());
                }
            }
        }
        Ok(ont_ref)
    }
}
