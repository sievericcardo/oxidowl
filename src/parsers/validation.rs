//! Syntax Validation Module
//!
//! This module provides strict validation for parsed ontologies to catch
//! malformed syntax that lenient parsers might accept.

use crate::{Error, Result};

/// Validates raw content before parsing to catch common syntax errors
pub struct SyntaxValidator {
    strict_mode: bool,
}

impl SyntaxValidator {
    /// Create a new validator with default settings (strict)
    pub fn new() -> Self {
        Self { strict_mode: true }
    }

    /// Create a lenient validator
    pub fn lenient() -> Self {
        Self { strict_mode: false }
    }

    /// Validate Turtle syntax
    pub fn validate_turtle(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        let lines: Vec<&str> = content.lines().collect();
        let mut expecting_end = false;  // Expecting either dot, semicolon, or comma
        let mut previous_was_object = false;
        
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Skip prefix and base declarations - must end with dot
            if trimmed.starts_with("@prefix") || trimmed.starts_with("@base") {
                // Validate @prefix format: @prefix prefix: <IRI> .
                if trimmed.starts_with("@prefix") {
                    // Check that IRI is in angle brackets
                    if !trimmed.contains('<') || !trimmed.contains('>') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: @prefix directive must have IRI in angle brackets: @prefix ns: <IRI> .",
                            line_num + 1
                        )));
                    }
                }
                
                if !trimmed.ends_with('.') && (line_num + 1 >= lines.len() || !lines[line_num + 1].trim().starts_with('.')) {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Prefix/base declaration must end with a dot",
                        line_num + 1
                    )));
                }
                continue;
            }

            // Check for malformed IRIs (unclosed angle brackets)
            if let Some(open_pos) = trimmed.find('<') {
                if !trimmed[open_pos..].contains('>') {
                    // Check if it continues on next line
                    if line_num + 1 >= lines.len() || !lines[line_num + 1].contains('>') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Unclosed IRI angle bracket",
                            line_num + 1
                        )));
                    }
                }
            }

            // Check for invalid IRI characters in angle brackets
            if let Some(start) = trimmed.find('<') {
                if let Some(end_pos) = trimmed[start..].find('>') {
                    let iri_content = &trimmed[start + 1..start + end_pos];
                    // IRIs should not contain unencoded spaces
                    if iri_content.contains(' ') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Invalid IRI - contains unencoded space: <{}>",
                            line_num + 1,
                            iri_content
                        )));
                    }
                }
            }

            // Core validation: check for proper triple termination
            // A triple looks like: subject predicate object .
            // Where "a" is shorthand for rdf:type
            
            // If previous line expected termination
            if expecting_end {
                // Check if this line starts with proper punctuation
                if !trimmed.starts_with('.') && !trimmed.starts_with(';') && !trimmed.starts_with(',') {
                    // This is a new statement but previous wasn't terminated
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Previous statement not properly terminated (missing . ; or ,)",
                        line_num
                    )));
                }
            }
            
            // Detect potential triple pattern: subject predicate object
            // Simple heuristic: contains " a " or has IRI/prefix followed by predicate
            let has_triple_pattern = trimmed.contains(" a ") || 
                                     (trimmed.contains(':') && trimmed.split_whitespace().count() >= 3) ||
                                     (trimmed.starts_with('<') && trimmed.split_whitespace().count() >= 3);
            
            if has_triple_pattern {
                // Check if properly terminated
                if trimmed.ends_with('.') {
                    expecting_end = false;
                    previous_was_object = false;
                } else if trimmed.ends_with(';') || trimmed.ends_with(',') {
                    expecting_end = false;
                    previous_was_object = true;
                } else {
                    // No termination - need to check next line
                    expecting_end = true;
                    previous_was_object = true;
                }
            }
        }

        // Final check: if we're still expecting termination, error
        if expecting_end {
            return Err(Error::ontology_parsing(
                "File ends with unterminated statement (missing .)"
            ));
        }

        Ok(())
    }

    /// Validate Functional Syntax
    pub fn validate_functional(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        // Check parenthesis matching
        let mut paren_depth = 0;
        let mut in_iri = false;
        let mut ontology_count = 0;

        for (i, ch) in content.chars().enumerate() {
            match ch {
                '<' => in_iri = true,
                '>' => in_iri = false,
                '(' if !in_iri => {
                    paren_depth += 1;
                    // Check if this starts an Ontology declaration
                    if i > 0 {
                        let prefix = &content[..i].trim_end();
                        if prefix.ends_with("Ontology") {
                            ontology_count += 1;
                        }
                    }
                }
                ')' if !in_iri => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        return Err(Error::ontology_parsing(
                            "Unmatched closing parenthesis"
                        ));
                    }
                }
                _ => {}
            }
        }

        if paren_depth != 0 {
            return Err(Error::ontology_parsing(format!(
                "Unmatched parentheses: {} unclosed",
                paren_depth
            )));
        }

        // Check for nested Ontology declarations
        if ontology_count > 1 {
            // Need to check if they're actually nested
            let mut depth = 0;
            let mut max_depth_at_ontology = 0;
            in_iri = false;

            for (i, ch) in content.chars().enumerate() {
                match ch {
                    '<' => in_iri = true,
                    '>' => in_iri = false,
                    '(' if !in_iri => {
                        depth += 1;
                        if i > 0 {
                            let prefix = &content[..i].trim_end();
                            if prefix.ends_with("Ontology") {
                                if max_depth_at_ontology > 0 && depth > max_depth_at_ontology {
                                    return Err(Error::ontology_parsing(
                                        "Nested Ontology declarations are not allowed"
                                    ));
                                }
                                max_depth_at_ontology = depth;
                            }
                        }
                    }
                    ')' if !in_iri => depth -= 1,
                    _ => {}
                }
            }
        }

        // Basic structure check: must start with "Ontology" (after whitespace)
        let trimmed = content.trim();
        if !trimmed.starts_with("Ontology(") && !trimmed.starts_with("Prefix(") {
            return Err(Error::ontology_parsing(
                "Functional syntax must start with Ontology( or Prefix("
            ));
        }

        Ok(())
    }

    /// Validate OWL/XML syntax
    pub fn validate_owl_xml(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        // Basic XML well-formedness checks
        let trimmed = content.trim();
        
        if !trimmed.starts_with('<') {
            return Err(Error::ontology_parsing("XML document must start with <"));
        }

        // Check for required root element (Ontology)
        if !trimmed.contains("<Ontology") && !trimmed.contains("<owl:Ontology") {
            return Err(Error::ontology_parsing(
                "OWL/XML must contain an Ontology root element"
            ));
        }

        // Simple tag matching (not a full XML validator, but catches common errors)
        let mut tag_stack: Vec<String> = Vec::new();
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut is_closing = false;

        for ch in trimmed.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    current_tag.clear();
                    is_closing = false;
                }
                '/' if in_tag && current_tag.is_empty() => {
                    is_closing = true;
                }
                '>' if in_tag => {
                    if current_tag.ends_with('/') {
                        // Self-closing tag
                        current_tag.pop();
                    } else if is_closing {
                        // Closing tag
                        if let Some(open_tag) = tag_stack.pop() {
                            let expected = open_tag.split_whitespace().next().unwrap_or("");
                            let actual = current_tag.split_whitespace().next().unwrap_or("");
                            if expected != actual {
                                return Err(Error::ontology_parsing(format!(
                                    "Mismatched XML tags: expected </{expected}>, found </{actual}>"
                                )));
                            }
                        } else {
                            return Err(Error::ontology_parsing(format!(
                                "Unexpected closing tag: </{current_tag}>"
                            )));
                        }
                    } else {
                        // Opening tag
                        tag_stack.push(current_tag.clone());
                    }
                    in_tag = false;
                }
                _ if in_tag => {
                    current_tag.push(ch);
                }
                _ => {}
            }
        }

        if !tag_stack.is_empty() {
            return Err(Error::ontology_parsing(format!(
                "Unclosed XML tags: {}",
                tag_stack.join(", ")
            )));
        }

        Ok(())
    }

    /// Validate RDF/XML syntax
    pub fn validate_rdf_xml(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        // RDF/XML has similar requirements to OWL/XML
        let trimmed = content.trim();
        
        if !trimmed.starts_with('<') {
            return Err(Error::ontology_parsing("XML document must start with <"));
        }

        // Check for RDF namespace
        if !trimmed.contains("rdf:RDF") && !trimmed.contains("xmlns:rdf") {
            return Err(Error::ontology_parsing(
                "RDF/XML must contain RDF namespace declaration"
            ));
        }

        // Reuse the XML validation logic
        self.validate_owl_xml(content)?;

        Ok(())
    }
}

impl Default for SyntaxValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turtle_missing_dot() {
        let validator = SyntaxValidator::new();
        let content = "@prefix : <http://example.org/> .\n:Person a owl:Class\n:John a :Person .";
        assert!(validator.validate_turtle(content).is_err());
    }

    #[test]
    fn test_turtle_invalid_iri() {
        let validator = SyntaxValidator::new();
        let content = "<http://example.org/ obj>";
        assert!(validator.validate_turtle(content).is_err());
    }

    #[test]
    fn test_functional_unmatched_parens() {
        let validator = SyntaxValidator::new();
        let content = "Ontology(<http://example.org/>missing closing parenthesis";
        assert!(validator.validate_functional(content).is_err());
    }

    #[test]
    fn test_functional_nested_ontology() {
        let validator = SyntaxValidator::new();
        let content = "Ontology(<http://example.org/>\n  Ontology(http://example.org/))";
        assert!(validator.validate_functional(content).is_err());
    }

    #[test]
    fn test_valid_turtle() {
        let validator = SyntaxValidator::new();
        let content = "@prefix : <http://example.org/> .\n:Person a owl:Class .";
        assert!(validator.validate_turtle(content).is_ok());
    }

    #[test]
    fn test_valid_functional() {
        let validator = SyntaxValidator::new();
        let content = "Ontology(<http://example.org/>)";
        assert!(validator.validate_functional(content).is_ok());
    }
}
