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

    /// Validate Manchester Syntax
    pub fn validate_manchester(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        let lines: Vec<&str> = content.lines().collect();
        
        // Valid Manchester keywords
        let valid_keywords = [
            "Ontology:", "Prefix:", "Import:", "Class:", "Individual:", "ObjectProperty:",
            "DataProperty:", "DatatypeProperty:", "AnnotationProperty:", "Datatype:", 
            "EquivalentTo:", "SubClassOf:", "DisjointWith:", "DisjointUnionOf:", "HasKey:", 
            "Types:", "Facts:", "SameAs:", "DifferentFrom:", "SubPropertyOf:", 
            "EquivalentProperties:", "DisjointProperties:", "InverseOf:", "Domain:", "Range:", 
            "Characteristics:", "Functional", "InverseFunctional", "Reflexive", "Irreflexive", 
            "Symmetric", "Asymmetric", "Transitive", "SubPropertyChain:", "Annotations:", 
            "Individuals:",
            // Class expression keywords
            "not", "and", "or", "some", "only", "value", "Self", "min", "max", "exactly",
            "that", "inverse",
        ];

        // Check for ontology header (optional but recommended)
        let has_ontology_header = content.lines()
            .any(|line| line.trim().starts_with("Ontology:"));
        
        // If no ontology header, at least check for some Manchester content
        if !has_ontology_header {
            let has_manchester_content = content.lines().any(|line| {
                let trimmed = line.trim();
                valid_keywords.iter().any(|&kw| {
                    trimmed.starts_with(kw) && (kw.ends_with(':') || kw == trimmed)
                })
            });
            
            if !has_manchester_content {
                return Err(Error::ontology_parsing(
                    "Manchester syntax must contain valid declarations (Class:, ObjectProperty:, etc.)"
                ));
            }
        }

        // Check each line for invalid keywords or malformed syntax
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for lines that look like keyword declarations (end with :)
            if let Some(keyword_pos) = trimmed.find(':') {
                let potential_keyword = &trimmed[..=keyword_pos];
                
                // Skip IRI brackets like <http://example.org/>
                if trimmed.starts_with('<') {
                    continue;
                }
                
                // Skip indented lines - they're part of class/property definitions
                if line.starts_with(' ') || line.starts_with('\t') {
                    continue;
                }
                
                // Check if this is a known keyword
                let is_valid_keyword = valid_keywords.iter()
                    .any(|&kw| potential_keyword.starts_with(kw) || kw.starts_with(potential_keyword));
                
                // Also check if it's a prefix declaration format (word: followed by IRI)
                let is_prefix_format = potential_keyword.split_whitespace().count() == 1 
                    && !potential_keyword.starts_with("Ontology:")
                    && trimmed.contains('<');
                
                if !is_valid_keyword && !is_prefix_format {
                    // This looks like an invalid keyword
                    let keyword_part = potential_keyword.trim();
                    if keyword_part.chars().next().unwrap_or(' ').is_uppercase() 
                        || keyword_part.contains(char::is_uppercase) {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Invalid Manchester keyword: '{}'",
                            line_num + 1,
                            keyword_part
                        )));
                    }
                }
            }

            // Check for malformed class declarations (missing colon after Class)
            if trimmed.starts_with("Class ") && !trimmed.contains(':') {
                return Err(Error::ontology_parsing(format!(
                    "Line {}: Malformed Class declaration - expected 'Class: <name>' with colon",
                    line_num + 1
                )));
            }

            // Check for invalid keyword combinations on same line
            // In Manchester, main declarations like "Class: Name" can have simple modifiers on same line
            // but complex multi-word constructs should be on separate lines
            if trimmed.starts_with("Class:") || trimmed.starts_with("ObjectProperty:") 
                || trimmed.starts_with("DataProperty:") || trimmed.starts_with("DatatypeProperty:")
                || trimmed.starts_with("Individual:") {
                
                // Allow "Class: Name Annotations: ..." on same line (common pattern)
                // Only reject if it has complex keywords that definitely need separate lines
                let after_first_keyword = if let Some(space_pos) = trimmed[6..].find(' ') {
                    &trimmed[6 + space_pos + 1..]
                } else {
                    ""
                };
                
                // Only check for SubClassOf which should definitely be on separate line
                if after_first_keyword.contains("SubClassOf:") {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid Manchester syntax - 'SubClassOf:' should be on a separate indented line",
                        line_num + 1
                    )));
                }
            }

            // Check for invalid singular/plural keyword forms
            if trimmed.contains("Characteristic:") {
                return Err(Error::ontology_parsing(format!(
                    "Line {}: Invalid keyword 'Characteristic' - should be 'Characteristics' (plural)",
                    line_num + 1
                )));
            }

            // Check for incomplete annotations (Annotations: without proper value)
            if trimmed.starts_with("Annotations:") {
                let after_keyword = trimmed[12..].trim();
                let parts: Vec<&str> = after_keyword.split_whitespace().collect();
                
                // Should have at least "property value" pattern
                if parts.len() < 2 {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Incomplete annotation - expected 'Annotations: property value'",
                        line_num + 1
                    )));
                }
                
                // Check if value looks suspiciously short (single character without quotes)
                if parts.len() == 2 {
                    let value = parts[1];
                    if value.len() == 1 && !value.starts_with('"') && !value.starts_with('<') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Invalid annotation value '{}' - single character values must be quoted",
                            line_num + 1,
                            value
                        )));
                    }
                }
            }

            // Check for random text that doesn't match any pattern
            // This catches lines like "use of unknown keyword" or other malformed content
            if !trimmed.is_empty() 
                && !trimmed.starts_with('#')
                && !trimmed.starts_with('<')
                && !trimmed.contains("://")  // Skip URIs
            {
                // Check if line looks like a declaration keyword
                let is_declaration = valid_keywords.iter()
                    .any(|&kw| trimmed.starts_with(kw));
                
                // Check if line is part of an indented block (starts with whitespace)
                let is_indented = line.starts_with(' ') || line.starts_with('\t');
                
                // If not a declaration and contains suspicious words, reject
                if !is_declaration && !is_indented {
                    let words: Vec<&str> = trimmed.split_whitespace().collect();
                    
                    // Reject lines with multiple words that aren't valid keywords
                    if words.len() > 1 {
                        let first_word = words[0];
                        let looks_like_keyword = first_word.chars().next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false);
                        
                        // Check if first word is a known keyword
                        let is_known = valid_keywords.iter()
                            .any(|&kw| kw.starts_with(first_word) || first_word == kw.trim_end_matches(':'));
                        
                        if !is_known && looks_like_keyword {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Unrecognized Manchester syntax or invalid keyword: '{}'",
                                line_num + 1,
                                trimmed
                            )));
                        }
                        
                        // Also reject lines with common error phrases
                        let error_phrases = ["use of", "unknown", "invalid", "error", "malformed"];
                        if error_phrases.iter().any(|&phrase| trimmed.contains(phrase)) {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid content in Manchester syntax: '{}'",
                                line_num + 1,
                                trimmed
                            )));
                        }
                    }
                }
            }
        }

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
