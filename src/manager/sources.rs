//! Document sources for reading ontology content from various locations.
//!
//! Abstracts over file paths, URLs, and compressed formats so the
//! loader can handle them uniformly.

use crate::ontology::{IRI, OntologyFormat};
use crate::Result;
use std::io::Read;
use std::path::PathBuf;

/// A source from which an ontology document can be read.
pub trait OntologyDocumentSource: Send + Sync {
    /// Read the entire document content as a string.
    fn read_to_string(&mut self) -> Result<String>;

    /// Get the document IRI (the physical location).
    fn get_document_iri(&self) -> &IRI;

    /// Get the format hint, if available.
    fn get_format(&self) -> Option<OntologyFormat> {
        None
    }
}

/// An in-memory string source (for testing or pre-loaded content).
pub struct StringDocumentSource {
    content: String,
    document_iri: IRI,
    format: Option<OntologyFormat>,
}

impl StringDocumentSource {
    #[must_use]
    pub fn new(content: String, document_iri: IRI) -> Self {
        Self {
            content,
            document_iri,
            format: None,
        }
    }

    #[must_use]
    pub fn with_format(mut self, format: OntologyFormat) -> Self {
        self.format = Some(format);
        self
    }
}

impl OntologyDocumentSource for StringDocumentSource {
    fn read_to_string(&mut self) -> Result<String> {
        Ok(std::mem::take(&mut self.content))
    }

    fn get_document_iri(&self) -> &IRI {
        &self.document_iri
    }

    fn get_format(&self) -> Option<OntologyFormat> {
        self.format
    }
}

/// A local file source.
pub struct FileDocumentSource {
    path: PathBuf,
    document_iri: IRI,
    format: Option<OntologyFormat>,
}

impl FileDocumentSource {
    /// Create a file source from a path.
    pub fn new(path: PathBuf) -> Result<Self> {
        let file_url = format!("file://{}", path.display());
        let document_iri = IRI::new(&file_url);
        let format = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(OntologyFormat::from_extension);
        Ok(Self {
            path,
            document_iri,
            format,
        })
    }

    /// Create a file source with an explicit format hint.
    #[must_use]
    pub fn with_format(mut self, format: OntologyFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Get the underlying path.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl OntologyDocumentSource for FileDocumentSource {
    fn read_to_string(&mut self) -> Result<String> {
        std::fs::read_to_string(&self.path)
            .map_err(|e| crate::Error::io(format!("Failed to read {}: {e}", self.path.display())))
    }

    fn get_document_iri(&self) -> &IRI {
        &self.document_iri
    }

    fn get_format(&self) -> Option<OntologyFormat> {
        self.format
    }
}

#[cfg(feature = "http-imports")]
/// A remote HTTP(S) URL source.
pub struct UrlDocumentSource {
    url: url::Url,
    document_iri: IRI,
    format: Option<OntologyFormat>,
}

#[cfg(feature = "http-imports")]
impl UrlDocumentSource {
    /// Create a URL source.
    #[must_use]
    pub fn new(url: url::Url) -> Self {
        let document_iri = IRI::new(url.as_str());
        Self {
            url,
            document_iri,
            format: None,
        }
    }

    #[must_use]
    pub fn with_format(mut self, format: OntologyFormat) -> Self {
        self.format = Some(format);
        self
    }
}

#[cfg(feature = "http-imports")]
impl OntologyDocumentSource for UrlDocumentSource {
    fn read_to_string(&mut self) -> Result<String> {
        // Use synchronous reqwest for document loading
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(self.url.clone())
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .map_err(|e| crate::Error::network(format!("HTTP request failed: {e}")))?;
        if !response.status().is_success() {
            return Err(crate::Error::network(format!(
                "HTTP {}: {}",
                response.status().as_u16(),
                self.url
            )));
        }
        response.text().map_err(|e| {
            crate::Error::network(format!("Failed to read response body: {e}"))
        })
    }

    fn get_document_iri(&self) -> &IRI {
        &self.document_iri
    }

    fn get_format(&self) -> Option<OntologyFormat> {
        self.format
    }
}

/// A gzip-compressed stdin source.
/// Uses `std::io::Read` and `flate2` to decompress gzip content from bytes.
pub struct GzipDocumentSource {
    inner: FileDocumentSource,
}

impl GzipDocumentSource {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            inner: FileDocumentSource::new(path)?,
        })
    }
}

impl OntologyDocumentSource for GzipDocumentSource {
    fn read_to_string(&mut self) -> Result<String> {
        let compressed = std::fs::read(self.inner.path())
            .map_err(|e| crate::Error::io(format!("Failed to read gzip file: {e}")))?;
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut content = String::new();
        decoder
            .read_to_string(&mut content)
            .map_err(|e| crate::Error::io(format!("Failed to decompress gzip: {e}")))?;
        Ok(content)
    }

    fn get_document_iri(&self) -> &IRI {
        self.inner.get_document_iri()
    }

    fn get_format(&self) -> Option<OntologyFormat> {
        self.inner.get_format()
    }
}

/// A gzip-compressed string source (for in-memory gzip content).
pub struct GzipStringDocumentSource {
    compressed: Vec<u8>,
    document_iri: IRI,
    format: Option<OntologyFormat>,
}

impl GzipStringDocumentSource {
    #[must_use]
    pub fn new(compressed: Vec<u8>, document_iri: IRI) -> Self {
        Self {
            compressed,
            document_iri,
            format: None,
        }
    }

    #[must_use]
    pub fn with_format(mut self, format: OntologyFormat) -> Self {
        self.format = Some(format);
        self
    }
}

impl OntologyDocumentSource for GzipStringDocumentSource {
    fn read_to_string(&mut self) -> Result<String> {
        let mut decoder = flate2::read::GzDecoder::new(&self.compressed[..]);
        let mut content = String::new();
        decoder
            .read_to_string(&mut content)
            .map_err(|e| crate::Error::io(format!("Failed to decompress gzip: {e}")))?;
        Ok(content)
    }

    fn get_document_iri(&self) -> &IRI {
        &self.document_iri
    }

    fn get_format(&self) -> Option<OntologyFormat> {
        self.format
    }
}
