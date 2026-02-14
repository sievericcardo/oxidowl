//! N-Triples Parser
//!
//! This module implements parsing of OWL 2 ontologies from N-Triples format.
//! Supports N-Triples-star for RDF-star quoted triples.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use super::common::OntologySerializer;
use crate::{
    Error, Result,
    ontology::Ontology,
    semantics::{RdfTerm, Triple as RdfTriple},
};

/// RDF version mode for N-Triples parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfVersionMode {
    /// RDF 1.1 only - no RDF-star features
    RDF11,
    /// RDF 1.2 mode
    RDF12,
    /// RDF-star mode with quoted triples
    RDFStar,
    /// Auto-detect based on syntax
    Auto,
}

/// N-Triples Parser Configuration
#[derive(Debug, Clone)]
pub struct NTriplesConfig {
    /// RDF version mode
    pub rdf_version: RdfVersionMode,
    /// Enable RDF-star quoted triple parsing
    pub parse_rdf_star: bool,
    /// Strict RDF 1.1 mode - reject RDF-star syntax
    pub strict_rdf11_mode: bool,
}

impl Default for NTriplesConfig {
    fn default() -> Self {
        Self {
            rdf_version: RdfVersionMode::Auto,
            parse_rdf_star: true,
            strict_rdf11_mode: false,
        }
    }
}

/// N-Triples Parser
#[derive(Debug, Clone)]
pub struct NTriplesParser {
    config: NTriplesConfig,
}

impl NTriplesParser {
    /// Create a new N-Triples parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: NTriplesConfig::default(),
        }
    }

    /// Create a new N-Triples parser with custom configuration
    #[must_use]
    pub fn with_config(config: NTriplesConfig) -> Self {
        Self { config }
    }
}

impl Default for NTriplesParser {
    fn default() -> Self {
        Self::new()
    }
}

impl NTriplesParser {
    /// Parse N-Triples content from string
    pub fn parse_string(&self, content: &str) -> Result<Ontology> {
        let mut ontology = Ontology::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            if let Err(e) = self.parse_line(trimmed, &mut ontology) {
                return Err(Error::ontology_parsing(format!(
                    "Line {}: {}",
                    line_num + 1,
                    e
                )));
            }
        }

        Ok(ontology)
    }

    /// Parse a single N-Triples line
    fn parse_line(&self, line: &str, ontology: &mut Ontology) -> Result<()> {
        // N-Triples lines must end with period
        if !line.ends_with('.') {
            return Err(Error::ontology_parsing(
                "N-Triples statement must end with '.'".to_string(),
            ));
        }

        let statement = &line[..line.len() - 1].trim(); // Remove the trailing '.'

        // Check for RDF-star syntax
        if statement.contains("<<") {
            if self.config.strict_rdf11_mode {
                return Err(Error::ontology_parsing(
                    "RDF-star syntax not allowed in strict RDF 1.1 mode".to_string(),
                ));
            }
            if !self.config.parse_rdf_star {
                return Err(Error::ontology_parsing(
                    "RDF-star syntax encountered but parsing is disabled".to_string(),
                ));
            }
            return self.parse_rdfstar_line(statement, ontology);
        }

        // Parse standard N-Triples triple
        let (_subject, rest) = self.parse_term(statement)?;
        let (_predicate, rest) = self.parse_term(rest.trim())?;
        let (_object, _) = self.parse_term(rest.trim())?;

        // Convert to ontology constructs (basic implementation)
        // For now, just validate parsing works
        // TODO: Actually add to ontology based on predicate type

        Ok(())
    }

    /// Parse an RDF-star line with quoted triples
    fn parse_rdfstar_line(&self, statement: &str, _ontology: &mut Ontology) -> Result<()> {
        let (_subject, rest) = self.parse_rdfstar_term(statement)?;
        let (_predicate, rest) = self.parse_rdfstar_term(rest.trim())?;
        let (_object, _) = self.parse_rdfstar_term(rest.trim())?;

        // For now, just validate RDF-star parsing works
        // TODO: Store RDF-star triples in ontology graph

        Ok(())
    }

    /// Parse a term that may be a quoted triple (RDF-star)
    fn parse_rdfstar_term<'a>(&self, input: &'a str) -> Result<(RdfTerm, &'a str)> {
        let trimmed = input.trim();

        // Check for quoted triple
        if trimmed.starts_with("<<") {
            return self.parse_quoted_triple(trimmed);
        }

        // Otherwise parse as regular term and convert to RdfTerm
        let (term_str, rest) = self.parse_term(trimmed)?;
        let rdf_term = self.string_to_rdf_term(term_str)?;
        Ok((rdf_term, rest))
    }

    /// Parse a quoted triple << s p o >>
    fn parse_quoted_triple<'a>(&self, input: &'a str) -> Result<(RdfTerm, &'a str)> {
        if !input.starts_with("<<") {
            return Err(Error::ontology_parsing(
                "Expected '<<' at start of quoted triple".to_string(),
            ));
        }

        let rest = &input[2..].trim();

        // Parse subject
        let (subject, rest) = self.parse_rdfstar_term(rest)?;

        // Parse predicate
        let (predicate, rest) = self.parse_rdfstar_term(rest.trim())?;

        // Parse object
        let (object, rest) = self.parse_rdfstar_term(rest.trim())?;

        // Expect >>
        let rest = rest.trim();
        if !rest.starts_with(">>") {
            return Err(Error::ontology_parsing(
                "Expected '>>' at end of quoted triple".to_string(),
            ));
        }

        let rest = &rest[2..];

        let triple = RdfTriple {
            subject,
            predicate,
            object,
        };

        Ok((RdfTerm::QuotedTriple(Box::new(triple)), rest))
    }

    /// Convert a string term to RdfTerm
    fn string_to_rdf_term(&self, s: &str) -> Result<RdfTerm> {
        if s.starts_with('<') && s.ends_with('>') {
            // IRI
            let iri_str = &s[1..s.len() - 1];
            let url = url::Url::parse(iri_str).map_err(|e| {
                Error::ontology_parsing(format!("Invalid IRI: {}", e))
            })?;
            Ok(RdfTerm::Iri(url))
        } else if s.starts_with("_:") {
            // Blank node
            Ok(RdfTerm::BlankNode(s.to_string()))
        } else if s.starts_with('"') {
            // Literal
            self.parse_literal(s)
        } else {
            Err(Error::ontology_parsing(format!(
                "Cannot parse term: {}",
                s
            )))
        }
    }

    /// Parse a literal with optional language tag or datatype
    fn parse_literal(&self, s: &str) -> Result<RdfTerm> {
        if !s.starts_with('"') {
            return Err(Error::ontology_parsing(
                "Literal must start with '\"'".to_string(),
            ));
        }

        // Find the closing quote
        let mut i = 1;
        let chars: Vec<char> = s.chars().collect();
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                i += 2; // Skip escaped character
            } else if chars[i] == '"' {
                break;
            } else {
                i += 1;
            }
        }

        if i >= chars.len() {
            return Err(Error::ontology_parsing(
                "Unterminated literal".to_string(),
            ));
        }

        let value = chars[1..i].iter().collect::<String>();
        let rest = &s[i + 1..];

        // Check for language tag or datatype
        let language = if rest.starts_with('@') {
            let end = rest.find(|c: char| c.is_whitespace() || c == '.' || c == '>')
                .unwrap_or(rest.len());
            Some(rest[1..end].to_string())
        } else {
            None
        };

        let datatype = if rest.starts_with("^^") {
            let datatype_str = &rest[2..];
            if datatype_str.starts_with('<') {
                let end = datatype_str.find('>').ok_or_else(|| {
                    Error::ontology_parsing("Unterminated datatype IRI".to_string())
                })?;
                let url = url::Url::parse(&datatype_str[1..end]).map_err(|e| {
                    Error::ontology_parsing(format!("Invalid datatype IRI: {}", e))
                })?;
                Some(url)
            } else {
                None
            }
        } else {
            None
        };

        Ok(RdfTerm::Literal {
            value,
            datatype,
            language,
        })
    }

    /// Parse a term (IRI, blank node, or literal) from N-Triples
    /// Returns the term and the remaining input
    fn parse_term<'a>(&self, input: &'a str) -> Result<(&'a str, &'a str)> {
        let trimmed = input.trim();

        if trimmed.starts_with('<') {
            // IRI
            let end = trimmed.find('>').ok_or_else(|| {
                Error::ontology_parsing("Unterminated IRI".to_string())
            })?;
            Ok((&trimmed[..=end], &trimmed[end + 1..]))
        } else if trimmed.starts_with("_:") {
            // Blank node
            let end = trimmed
                .find(|c: char| c.is_whitespace())
                .unwrap_or(trimmed.len());
            Ok((&trimmed[..end], &trimmed[end..]))
        } else if trimmed.starts_with('"') {
            // Literal - find the closing quote, accounting for escapes
            let mut i = 1;
            let chars: Vec<char> = trimmed.chars().collect();
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 2; // Skip escaped character
                } else if chars[i] == '"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }

            // Check for language tag or datatype
            let mut end = i;
            if i < chars.len() {
                if chars[i] == '@' {
                    // Language tag
                    while end < chars.len() && !chars[end].is_whitespace() {
                        end += 1;
                    }
                } else if i + 1 < chars.len() && chars[i] == '^' && chars[i + 1] == '^' {
                    // Datatype
                    end = i + 2;
                    if end < chars.len() && chars[end] == '<' {
                        while end < chars.len() && chars[end] != '>' {
                            end += 1;
                        }
                        if end < chars.len() {
                            end += 1; // Include the '>'
                        }
                    }
                }
            }

            let term: String = chars[..end].iter().collect();
            let rest: String = chars[end..].iter().collect();
            // We need to return string slices from the original input
            // Find the position in the original string
            let term_len = term.len();
            Ok((&trimmed[..term_len], &trimmed[term_len..]))
        } else {
            Err(Error::ontology_parsing(format!(
                "Cannot parse term from: {}",
                trimmed
            )))
        }
    }
}

/// Parse N-Triples from string content
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = NTriplesParser::new();
    parser.parse_string(content)
}

/// Parse N-Triples from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path).map_err(|e| Error::io(format!("Failed to open file: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse(&content)
}

/// N-Triples format serializer implementing the common serialization interface
#[derive(Debug, Clone, Default)]
pub struct NTriplesSerializer;

impl NTriplesSerializer {
    /// Create a new NTriplesSerializer instance
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl OntologySerializer for NTriplesSerializer {
    fn serialize(&self, ontology: &Ontology) -> std::result::Result<String, Error> {
        let mut content = String::new();

        for (subject, class) in ontology.classes() {
            content.push_str(&format!("{} rdf:type {} .\n", subject, class.iri));
        }

        for (subject, individual) in ontology.individuals() {
            if let Some(iri) = individual.iri() {
                content.push_str(&format!("{iri} rdf:type Individual .\n"));
            }
        }

        Ok(content)
    }
}

/// Save ontology to N-Triples file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = NTriplesSerializer::new();
    serializer.serialize_to_file(ontology, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_ntriples() {
        let content = r#"
# Comment line
<http://example.org/subject> <http://example.org/predicate> <http://example.org/object> .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Basic N-Triples parsing should succeed");
    }

    #[test]
    fn test_parse_rdfstar_simple() {
        let content = r#"
<< <http://example.org/alice> <http://example.org/knows> <http://example.org/bob> >> <http://example.org/certainty> "high" .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Simple RDF-star parsing should succeed");
    }

    #[test]
    fn test_parse_rdfstar_nested() {
        let content = r#"
<< << <http://example.org/s1> <http://example.org/p1> <http://example.org/o1> >> <http://example.org/p2> <http://example.org/o2> >> <http://example.org/confidence> "0.9" .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Nested RDF-star parsing should succeed");
    }

    #[test]
    fn test_strict_rdf11_mode_rejects_rdfstar() {
        let content = r#"
<< <http://example.org/alice> <http://example.org/knows> <http://example.org/bob> >> <http://example.org/certainty> "high" .
"#;

        let config = NTriplesConfig {
            rdf_version: RdfVersionMode::RDF11,
            parse_rdf_star: false,
            strict_rdf11_mode: true,
        };
        let parser = NTriplesParser::with_config(config);
        let result = parser.parse_string(content);
        assert!(result.is_err(), "Strict RDF 1.1 mode should reject RDF-star syntax");
    }

    #[test]
    fn test_parse_literal_with_language_tag() {
        let content = r#"
<http://example.org/subject> <http://example.org/label> "Hello"@en .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Literal with language tag should parse");
    }

    #[test]
    fn test_parse_literal_with_datatype() {
        let content = r#"
<http://example.org/subject> <http://example.org/age> "42"^^<http://www.w3.org/2001/XMLSchema#integer> .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Literal with datatype should parse");
    }

    #[test]
    fn test_parse_blank_node() {
        let content = r#"
_:b1 <http://example.org/predicate> <http://example.org/object> .
"#;

        let parser = NTriplesParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok(), "Blank node should parse");
    }
}
