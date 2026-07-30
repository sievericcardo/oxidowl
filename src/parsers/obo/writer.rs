//! OBO Format Writer — serializes ontologies to OBO 1.4 format.

use crate::ontology::Ontology;
use crate::Result;
use super::converter::Owl2Obo;

/// Configuration for OBO output.
#[derive(Debug, Clone)]
pub struct OBOOutputConfig {
    pub include_stanza_comments: bool,
    pub indent_level: usize,
    pub sort_stanzas: bool,
}

impl Default for OBOOutputConfig {
    fn default() -> Self { Self { include_stanza_comments: false, indent_level: 0, sort_stanzas: true } }
}

/// Writes ontologies in OBO format.
#[derive(Debug, Clone)]
pub struct OBOWriter {
    #[allow(dead_code)]
    config: OBOOutputConfig,
}

impl Default for OBOWriter {
    fn default() -> Self { Self { config: OBOOutputConfig::default() } }
}

impl OBOWriter {
    #[must_use]
    pub fn new(config: OBOOutputConfig) -> Self { Self { config } }

    /// Serialize an ontology to OBO string.
    pub fn serialize(&self, ontology: &Ontology) -> String {
        let converter = Owl2Obo::new();
        converter.serialize(ontology)
    }
}

/// Save an ontology as OBO format to a file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let writer = OBOWriter::default();
    let content = writer.serialize(ontology);
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("Failed to write OBO: {e}")))
}
