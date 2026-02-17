//! Common infrastructure for OWL ontology parsing and serialization
//!
//! This module provides unified traits and utilities to eliminate code duplication
//! across different ontology format parsers. Each parser implements these common
//! traits while maintaining format-specific logic.

use crate::{error::Error, ontology::Ontology};
use std::{fs::File, io::Write, path::Path};

/// Common trait for ontology serialization to different formats
pub trait OntologySerializer {
    /// Serialize an ontology to a string representation
    ///
    /// This method handles the format-specific serialization logic
    /// and returns the complete serialized content as a string.
    fn serialize(&self, ontology: &Ontology) -> std::result::Result<String, Error>;

    /// Serialize an ontology directly to a file
    ///
    /// This provides a common implementation for file writing with
    /// consistent error handling across all formats.
    fn serialize_to_file<P: AsRef<Path>>(
        &self,
        ontology: &Ontology,
        path: P,
    ) -> std::result::Result<(), Error> {
        let content = self.serialize(ontology)?;

        let mut file =
            File::create(path).map_err(|e| Error::io(format!("Failed to create file: {e}")))?;

        file.write_all(content.as_bytes())
            .map_err(|e| Error::io(format!("Failed to write to file: {e}")))?;

        file.flush()
            .map_err(|e| Error::io(format!("Failed to flush file: {e}")))?;

        Ok(())
    }
}

/// Common trait for ontology parsing from different formats
pub trait OntologyParser {
    /// Parse an ontology from string content
    ///
    /// This method handles the format-specific parsing logic
    /// and returns a complete ontology object.
    fn parse(&self, content: &str) -> std::result::Result<Ontology, Error>;

    /// Parse an ontology from a file
    ///
    /// This provides a common implementation for file reading with
    /// consistent error handling across all formats.
    fn parse_from_file<P: AsRef<Path>>(&self, path: P) -> std::result::Result<Ontology, Error> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;
        self.parse(&content)
    }
}

/// Utility function for consistent error handling in parsers
pub fn parsing_error(message: impl Into<String>) -> Error {
    Error::ontology_parsing(message.into())
}

/// Utility function for consistent error handling in serializers
pub fn serialization_error(message: impl Into<String>) -> Error {
    Error::ontology_parsing(message.into())
}

/// Common validation for ontology content before serialization
pub fn validate_ontology_for_serialization(ontology: &Ontology) -> std::result::Result<(), Error> {
    // Perform basic validation that's common across all formats
    if ontology.axioms().is_empty() {
        return Err(Error::ontology_parsing("Cannot serialize empty ontology"));
    }

    // Additional common validation can be added here
    Ok(())
}

/// Common formatting utilities
pub mod format_utils {
    use url::Url;

    /// Format an IRI for serialization with proper escaping
    #[must_use]
    pub fn format_iri(iri: &Url) -> String {
        // Common IRI formatting logic
        format!("<{iri}>")
    }

    /// Escape special characters in string literals
    #[must_use]
    pub fn escape_string_literal(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Generate indentation for pretty-printing
    #[must_use]
    pub fn indent(level: usize) -> String {
        "  ".repeat(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Mock serializer for testing
    struct MockSerializer;

    impl OntologySerializer for MockSerializer {
        fn serialize(&self, _ontology: &Ontology) -> std::result::Result<String, Error> {
            Ok("Mock serialized content".to_string())
        }
    }

    // Mock parser for testing
    struct MockParser;

    impl OntologyParser for MockParser {
        fn parse(&self, _content: &str) -> std::result::Result<Ontology, Error> {
            Ok(Ontology::new())
        }
    }

    #[test]
    fn test_serialize_to_file() {
        let serializer = MockSerializer;
        let ontology = Ontology::new();

        let temp_file = NamedTempFile::new().expect("Failed to create temporary file for test");
        let path = temp_file.path();

        // Test serialization to file
        let result = serializer.serialize_to_file(&ontology, path);
        assert!(result.is_ok());

        // Verify file content
        let content = std::fs::read_to_string(path).expect("Failed to read file content as string");
        assert_eq!(content, "Mock serialized content");
    }

    #[test]
    fn test_parse_from_file() {
        let parser = MockParser;

        // Create a temporary file with test content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file for test");
        writeln!(temp_file, "Test ontology content")
            .expect("Failed to write content to temporary file");
        temp_file.flush().expect("Failed to flush temporary file");

        let result = parser.parse_from_file(temp_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_utils() {
        use super::format_utils::*;

        // Test string literal escaping
        let escaped = escape_string_literal("Hello \"world\"\nNew line");
        assert_eq!(escaped, "Hello \\\"world\\\"\\nNew line");

        // Test indentation
        let indent_2 = indent(2);
        assert_eq!(indent_2, "    ");
    }
}
