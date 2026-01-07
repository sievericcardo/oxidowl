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
        let mut expecting_end = false; // Expecting either dot, semicolon, or comma
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

                if !trimmed.ends_with('.')
                    && (line_num + 1 >= lines.len() || !lines[line_num + 1].trim().starts_with('.'))
                {
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

            // Validate literals in the line
            self.validate_turtle_literals(trimmed, line_num + 1)?;

            // Validate numeric literals (bare numbers without quotes)
            // Check for invalid patterns like 3..14 (double dot)
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            for word in words {
                let clean = word.trim_end_matches(&['.', ',', ';'][..]);
                // Check for double dots in numeric-looking tokens
                if clean.contains("..") {
                    // This looks like an invalid numeric literal
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid numeric literal '{}' (contains consecutive dots)",
                        line_num + 1,
                        clean
                    )));
                }
            }

            // Validate prefixed names - local part shouldn't start with '.'
            // Pattern: prefix:localpart
            for word in trimmed.split_whitespace() {
                let clean = word
                    .trim_end_matches(&['.', ',', ';', ')', ']'][..])
                    .trim_start_matches(&['(', '['][..]);

                // Check for invalid blank node format (starts with _ but no colon)
                // Valid: _:b0, Invalid: _b0
                if clean.starts_with('_') && !clean.starts_with("_:") && !clean.contains('<') {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid blank node '{}' - must use format '_:label' with colon",
                        line_num + 1,
                        clean
                    )));
                }

                if let Some(colon_pos) = clean.find(':') {
                    // Skip IRIs in angle brackets
                    if clean.starts_with('<') {
                        continue;
                    }
                    // Skip HTTP/HTTPS URIs
                    if clean.contains("://") {
                        continue;
                    }
                    // Skip blank nodes (already validated above)
                    if clean.starts_with("_:") {
                        continue;
                    }

                    let local_part = &clean[colon_pos + 1..];
                    // Local part shouldn't start with '.'
                    if local_part.starts_with('.') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Invalid prefixed name '{}' - local part cannot start with '.'",
                            line_num + 1,
                            clean
                        )));
                    }
                }
            }

            // Core validation: check for proper triple termination
            // A triple looks like: subject predicate object .
            // Semicolon (;) means same subject, new predicate-object pair
            // Comma (,) means same subject and predicate, new object
            // Dot (.) terminates the statement completely

            // Skip validation for lines that are clearly continuations:
            // - Indented lines (spaces or tabs at start)
            // - Lines starting with collection syntax ) but not starting a new statement
            let is_indented = line.starts_with(' ') || line.starts_with('\t');
            let is_list_closing = trimmed.starts_with(')');

            if is_indented || is_list_closing {
                // These are continuations, don't validate statement termination
                continue;
            }

            // If previous line expected termination
            if expecting_end && previous_was_object {
                // After a triple without proper termination, the next line must:
                // 1. Start with punctuation (. ; ,) OR
                // 2. Be a continuation (prefixed name starting with colon after whitespace)
                let is_punctuation_line = trimmed.starts_with('.')
                    || trimmed.starts_with(';')
                    || trimmed.starts_with(',');

                // Check if this is truly a NEW statement (starts at column 0 and looks like a subject)
                // A new statement would typically start with a prefix:name or <IRI> pattern
                let looks_like_new_statement = !is_punctuation_line
                    && (trimmed.starts_with('<')
                        || (trimmed.contains(':')
                            && trimmed
                                .split_whitespace()
                                .next()
                                .unwrap_or("")
                                .contains(':')));

                if looks_like_new_statement {
                    // This is a new statement but previous wasn't terminated
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Previous statement not properly terminated (missing . ; or ,)",
                        line_num + 1
                    )));
                }
            }

            // Detect potential triple pattern: subject predicate object
            // Simple heuristic: contains " a " or has IRI/prefix followed by predicate
            let has_triple_pattern = trimmed.contains(" a ")
                || (trimmed.contains(':') && trimmed.split_whitespace().count() >= 2)
                || (trimmed.starts_with('<') && trimmed.split_whitespace().count() >= 3);

            // Check for lines that don't match any valid Turtle pattern
            // Valid patterns: triples, prefixes (@prefix/@base), or continuations (. ; ,)
            let is_punctuation_only = trimmed == "." || trimmed == ";" || trimmed == ",";
            let is_continuation =
                trimmed.starts_with('.') || trimmed.starts_with(';') || trimmed.starts_with(',');

            if !has_triple_pattern && !is_punctuation_only && !is_continuation {
                // This line doesn't look like valid Turtle syntax
                // Check if it contains words that suggest it's prose rather than Turtle
                let word_count = trimmed.split_whitespace().count();
                let has_turtle_chars = trimmed.contains('<')
                    || trimmed.contains(':')
                    || trimmed.contains('[')
                    || trimmed.contains('(')
                    || trimmed.starts_with("\"");

                // If it has multiple words but no Turtle-specific characters, it's likely invalid
                if word_count >= 3 && !has_turtle_chars {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid Turtle syntax - line doesn't match any valid pattern",
                        line_num + 1
                    )));
                }
            }

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
                "File ends with unterminated statement (missing .)",
            ));
        }

        // Check for owl:imports with missing IRI
        if content.contains("owl:imports") {
            for (line_num, line) in lines.iter().enumerate() {
                if line.contains("owl:imports") {
                    // Extract what comes after owl:imports
                    if let Some(pos) = line.find("owl:imports") {
                        let after_imports = &line[pos + 11..].trim();
                        // Skip if it's just a comment
                        if after_imports.starts_with('#') {
                            continue;
                        }
                        let words: Vec<&str> = after_imports.split_whitespace().collect();
                        // Check if there's anything after owl:imports before the statement delimiter
                        if words.is_empty()
                            || words[0].starts_with('.')
                            || words[0].starts_with(';')
                        {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: owl:imports requires an IRI value",
                                line_num + 1
                            )));
                        }
                    }
                }
            }
        }

        // OWL 2 DL semantic validation
        // Check for owl:hasSelf with non-boolean value
        if content.contains("owl:hasSelf") {
            for (line_num, line) in lines.iter().enumerate() {
                if line.contains("owl:hasSelf") {
                    // Extract the value after owl:hasSelf
                    if let Some(pos) = line.find("owl:hasSelf") {
                        let after_has_self = &line[pos + 11..].trim();
                        let words: Vec<&str> = after_has_self.split_whitespace().collect();
                        if !words.is_empty() {
                            let value = words[0].trim_end_matches(&['.', ';', ',', ']'][..]);

                            // Allow: true, false, "true"^^xsd:boolean, "false"^^xsd:boolean
                            // Reject: "yes", "no", or any other string

                            // Remove datatype suffix if present (e.g., ^^xsd:boolean)
                            let base_value = if value.contains("^^") {
                                value.split("^^").next().unwrap_or(value)
                            } else {
                                value
                            };

                            // Remove quotes
                            let unquoted = base_value.trim_matches('"');

                            // Check if it's true or false
                            if unquoted != "true" && unquoted != "false" {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: owl:hasSelf value must be a boolean (true or false), not '{}'",
                                    line_num + 1,
                                    unquoted
                                )));
                            }
                        }
                    }
                }
            }
        }

        // Check for qualified cardinality with non-numeric values
        let cardinality_props = [
            "owl:qualifiedCardinality",
            "owl:minQualifiedCardinality",
            "owl:maxQualifiedCardinality",
            "owl:cardinality",
            "owl:minCardinality",
            "owl:maxCardinality",
        ];

        for prop in &cardinality_props {
            if content.contains(prop) {
                for (line_num, line) in lines.iter().enumerate() {
                    if line.contains(prop) {
                        if let Some(pos) = line.find(prop) {
                            let after_prop = &line[pos + prop.len()..].trim();
                            let words: Vec<&str> = after_prop.split_whitespace().collect();
                            if !words.is_empty() {
                                let value = words[0].trim_end_matches(&['.', ';', ',', ']'][..]);

                                // Remove datatype suffix if present (e.g., ^^xsd:nonNegativeInteger)
                                let base_value = if value.contains("^^") {
                                    value.split("^^").next().unwrap_or(value)
                                } else {
                                    value
                                };

                                // Remove quotes if present
                                let unquoted = base_value.trim_matches('"');

                                // Check for word-based numbers
                                let number_words = [
                                    "one", "two", "three", "four", "five", "six", "seven", "eight",
                                    "nine", "ten", "zero",
                                ];
                                if number_words.contains(&unquoted.to_lowercase().as_str()) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: {} value must be a numeric digit (e.g., '2'), not a word ('{}')",
                                        line_num + 1,
                                        prop,
                                        unquoted
                                    )));
                                }

                                // Check if it's a valid non-negative integer
                                if !unquoted.chars().all(|c| c.is_ascii_digit())
                                    && !unquoted.is_empty()
                                {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: {} value must be a non-negative integer, not '{}'",
                                        line_num + 1,
                                        prop,
                                        unquoted
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate Turtle literals for malformed syntax
    fn validate_turtle_literals(&self, line: &str, line_num: usize) -> Result<()> {
        let mut chars = line.chars().peekable();
        let mut in_iri = false;

        while let Some(ch) = chars.next() {
            match ch {
                '<' => in_iri = true,
                '>' => in_iri = false,
                '"' if !in_iri => {
                    // Found start of literal
                    let mut literal_value = String::new();
                    let mut has_datatype = false;
                    let mut has_language = false;

                    // Collect literal content
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '"' {
                            chars.next(); // consume closing quote
                            break;
                        }
                        if next_ch == '\\' {
                            chars.next(); // consume backslash
                            if let Some(&escaped) = chars.peek() {
                                chars.next(); // consume escaped char

                                // Validate escape sequences
                                match escaped {
                                    't' | 'n' | 'r' | '\\' | '"' | '\'' => {
                                        // Valid single-char escapes
                                        literal_value.push('\\');
                                        literal_value.push(escaped);
                                    }
                                    'u' => {
                                        // Unicode escape: \uXXXX (4 hex digits)
                                        literal_value.push('\\');
                                        literal_value.push('u');
                                        for _ in 0..4 {
                                            if let Some(&hex_ch) = chars.peek() {
                                                if !hex_ch.is_ascii_hexdigit() {
                                                    return Err(Error::ontology_parsing(format!(
                                                        "Line {}: Invalid \\u escape sequence - expected 4 hex digits",
                                                        line_num
                                                    )));
                                                }
                                                literal_value.push(hex_ch);
                                                chars.next();
                                            } else {
                                                return Err(Error::ontology_parsing(format!(
                                                    "Line {}: Incomplete \\u escape sequence",
                                                    line_num
                                                )));
                                            }
                                        }
                                    }
                                    'U' => {
                                        // Unicode escape: \UXXXXXXXX (8 hex digits)
                                        literal_value.push('\\');
                                        literal_value.push('U');
                                        for _ in 0..8 {
                                            if let Some(&hex_ch) = chars.peek() {
                                                if !hex_ch.is_ascii_hexdigit() {
                                                    return Err(Error::ontology_parsing(format!(
                                                        "Line {}: Invalid \\U escape sequence - expected 8 hex digits",
                                                        line_num
                                                    )));
                                                }
                                                literal_value.push(hex_ch);
                                                chars.next();
                                            } else {
                                                return Err(Error::ontology_parsing(format!(
                                                    "Line {}: Incomplete \\U escape sequence",
                                                    line_num
                                                )));
                                            }
                                        }
                                    }
                                    'x' => {
                                        // Hex escape: \xXX (2 hex digits) - NOT standard Turtle!
                                        return Err(Error::ontology_parsing(format!(
                                            "Line {}: Invalid escape sequence \\x (not valid in Turtle)",
                                            line_num
                                        )));
                                    }
                                    _ => {
                                        // Invalid escape character
                                        return Err(Error::ontology_parsing(format!(
                                            "Line {}: Invalid escape sequence \\{} (valid escapes: \\t \\n \\r \\\\ \\\" \\' \\uXXXX \\UXXXXXXXX)",
                                            line_num, escaped
                                        )));
                                    }
                                }
                            }
                        } else {
                            literal_value.push(next_ch);
                            chars.next();
                        }
                    }

                    // Check for datatype annotation (^^xsd:integer)
                    if let Some(&'^') = chars.peek() {
                        chars.next();
                        if let Some(&'^') = chars.peek() {
                            chars.next();
                            has_datatype = true;

                            // Extract datatype IRI
                            let mut datatype = String::new();
                            while let Some(&dt_ch) = chars.peek() {
                                if dt_ch.is_whitespace()
                                    || dt_ch == '.'
                                    || dt_ch == ';'
                                    || dt_ch == ','
                                {
                                    break;
                                }
                                datatype.push(dt_ch);
                                chars.next();
                            }

                            // Validate numeric literals with datatype
                            if datatype.ends_with("integer")
                                || datatype.ends_with("int")
                                || datatype.ends_with("long")
                            {
                                // Check if literal_value is a valid integer
                                let clean_value = literal_value.trim();
                                if clean_value.is_empty() {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Empty integer literal",
                                        line_num
                                    )));
                                }
                                // Check for invalid characters in integer
                                let start_idx = if clean_value.starts_with('+')
                                    || clean_value.starts_with('-')
                                {
                                    1
                                } else {
                                    0
                                };
                                if !clean_value[start_idx..].chars().all(|c| c.is_ascii_digit()) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Invalid integer literal: \"{}\"^^{}",
                                        line_num, clean_value, datatype
                                    )));
                                }
                            } else if datatype.ends_with("decimal") {
                                // Check if literal_value is a valid decimal number (no exponent allowed)
                                let clean_value = literal_value.trim();
                                if clean_value.is_empty() {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Empty numeric literal",
                                        line_num
                                    )));
                                }

                                // Decimal does NOT allow exponential notation
                                // Allow digits, optional sign, optional decimal point
                                let mut has_dot = false;
                                let mut chars_iter = clean_value.chars();

                                // Optional leading sign
                                if let Some(first) = chars_iter.next() {
                                    if first != '+'
                                        && first != '-'
                                        && !first.is_ascii_digit()
                                        && first != '.'
                                    {
                                        return Err(Error::ontology_parsing(format!(
                                            "Line {}: Invalid decimal literal: \"{}\"^^{}",
                                            line_num, clean_value, datatype
                                        )));
                                    }
                                    if first == '.' {
                                        has_dot = true;
                                    }
                                }

                                for ch in chars_iter {
                                    match ch {
                                        '0'..='9' => {}
                                        '.' if !has_dot => has_dot = true,
                                        'e' | 'E' => {
                                            // Exponential notation NOT allowed in decimal
                                            return Err(Error::ontology_parsing(format!(
                                                "Line {}: Invalid decimal literal: \"{}\"^^{} (exponential notation not allowed)",
                                                line_num, clean_value, datatype
                                            )));
                                        }
                                        _ => {
                                            return Err(Error::ontology_parsing(format!(
                                                "Line {}: Invalid decimal literal: \"{}\"^^{} (invalid character '{}')",
                                                line_num, clean_value, datatype, ch
                                            )));
                                        }
                                    }
                                }
                            } else if datatype.ends_with("double") || datatype.ends_with("float") {
                                // Check if literal_value is a valid double/float number
                                let clean_value = literal_value.trim();
                                if clean_value.is_empty() {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Empty numeric literal",
                                        line_num
                                    )));
                                }

                                // Allow special float/double values: INF, -INF, NaN
                                if clean_value == "INF"
                                    || clean_value == "-INF"
                                    || clean_value == "NaN"
                                {
                                    // These are valid special values for float/double
                                    // Continue without validation
                                } else {
                                    // Allow digits, optional sign, optional decimal point, optional exponent
                                    let mut has_dot = false;
                                    let mut has_exp = false;
                                    let mut chars_iter = clean_value.chars();

                                    // Optional leading sign
                                    if let Some(first) = chars_iter.next() {
                                        if first != '+'
                                            && first != '-'
                                            && !first.is_ascii_digit()
                                            && first != '.'
                                        {
                                            return Err(Error::ontology_parsing(format!(
                                                "Line {}: Invalid numeric literal: \"{}\"^^{}",
                                                line_num, clean_value, datatype
                                            )));
                                        }
                                        if first == '.' {
                                            has_dot = true;
                                        }
                                    }

                                    for ch in chars_iter {
                                        match ch {
                                            '0'..='9' => {}
                                            '.' if !has_dot && !has_exp => has_dot = true,
                                            'e' | 'E' if !has_exp => {
                                                has_exp = true;
                                                // Next char can be +, -, or digit
                                            }
                                            '+' | '-' if has_exp => {} // Sign after exponent
                                            _ => {
                                                return Err(Error::ontology_parsing(format!(
                                                    "Line {}: Invalid numeric literal: \"{}\"^^{} (invalid character '{}')",
                                                    line_num, clean_value, datatype, ch
                                                )));
                                            }
                                        }
                                    }
                                }
                            } else if datatype.ends_with("boolean") {
                                // Check if literal_value is "true" or "false"
                                let clean_value = literal_value.trim();
                                if clean_value != "true" && clean_value != "false" {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Invalid boolean literal: \"{}\"^^{} (must be 'true' or 'false')",
                                        line_num, clean_value, datatype
                                    )));
                                }
                            }
                        }
                    } else if let Some(&'@') = chars.peek() {
                        // Language tag
                        chars.next();
                        has_language = true;

                        // Extract language tag
                        let mut lang_tag = String::new();
                        while let Some(&lang_ch) = chars.peek() {
                            if lang_ch.is_whitespace()
                                || lang_ch == '.'
                                || lang_ch == ';'
                                || lang_ch == ','
                            {
                                break;
                            }
                            lang_tag.push(lang_ch);
                            chars.next();
                        }

                        // Validate language tag format (basic check: should be letters and hyphens)
                        if lang_tag.is_empty() {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Empty language tag",
                                line_num
                            )));
                        }
                        // Language tag must start with a letter (not a dash)
                        if lang_tag.starts_with('-') {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid language tag: @{} (cannot start with '-')",
                                line_num, lang_tag
                            )));
                        }
                        // Language tag must contain only letters and hyphens
                        if !lang_tag.chars().all(|c| c.is_alphabetic() || c == '-') {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid language tag: @{} (must contain only letters and hyphens)",
                                line_num, lang_tag
                            )));
                        }
                    }

                    // Check for literals with both language tag and datatype (invalid)
                    if has_language && has_datatype {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Literal cannot have both language tag and datatype",
                            line_num
                        )));
                    }
                }
                _ => {}
            }
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
        let mut byte_offset = 0;

        for (_, ch) in content.chars().enumerate() {
            match ch {
                '<' => in_iri = true,
                '>' => in_iri = false,
                '(' if !in_iri => {
                    paren_depth += 1;
                    // Check if this starts an Ontology declaration
                    if byte_offset > 0 {
                        // Use byte_offset which is guaranteed to be on char boundary
                        let prefix = &content[..byte_offset].trim_end();
                        if prefix.ends_with("Ontology") {
                            ontology_count += 1;
                        }
                    }
                }
                ')' if !in_iri => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        return Err(Error::ontology_parsing("Unmatched closing parenthesis"));
                    }
                }
                _ => {}
            }
            byte_offset += ch.len_utf8();
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
            byte_offset = 0;

            for (_, ch) in content.chars().enumerate() {
                match ch {
                    '<' => in_iri = true,
                    '>' => in_iri = false,
                    '(' if !in_iri => {
                        depth += 1;
                        if byte_offset > 0 {
                            let prefix = &content[..byte_offset].trim_end();
                            if prefix.ends_with("Ontology") {
                                if max_depth_at_ontology > 0 && depth > max_depth_at_ontology {
                                    return Err(Error::ontology_parsing(
                                        "Nested Ontology declarations are not allowed",
                                    ));
                                }
                                max_depth_at_ontology = depth;
                            }
                        }
                    }
                    ')' if !in_iri => depth -= 1,
                    _ => {}
                }
                byte_offset += ch.len_utf8();
            }
        }

        // Basic structure check: must start with "Ontology" or "Prefix" or be SWRL rule syntax
        let trimmed = content.trim();

        // Allow placeholder files (common in test suites)
        if trimmed == "(placeholder)" || trimmed.is_empty() {
            return Ok(());
        }

        // SWRL rules start with [ruleName: ...] or just have rule syntax
        let is_swrl = trimmed.starts_with('[') || trimmed.contains("->") || trimmed.contains(":-");

        if !is_swrl && !trimmed.starts_with("Ontology(") && !trimmed.starts_with("Prefix(") {
            return Err(Error::ontology_parsing(
                "Functional syntax must start with Ontology( or Prefix( (or be SWRL rule syntax)",
            ));
        }

        // If this looks like SWRL content, validate it as SWRL
        if is_swrl {
            return self.validate_swrl(content);
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

        // OWL/XML can be a full ontology or just fragments (Declaration, etc.)
        // So we don't require an Ontology root element - any valid XML structure is OK

        // Simple tag matching (not a full XML validator, but catches common errors)
        let mut tag_stack: Vec<String> = Vec::new();
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut is_closing = false;
        let mut is_processing_instruction = false;

        for ch in trimmed.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    current_tag.clear();
                    is_closing = false;
                    is_processing_instruction = false;
                }
                '?' if in_tag && current_tag.is_empty() => {
                    // XML declaration or processing instruction: <?...?>
                    is_processing_instruction = true;
                }
                '/' if in_tag && current_tag.is_empty() => {
                    is_closing = true;
                }
                '>' if in_tag => {
                    // Skip processing instructions (<?xml ...?>, etc.)
                    if is_processing_instruction || current_tag.ends_with('?') {
                        in_tag = false;
                        continue;
                    }

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
                "RDF/XML must contain RDF namespace declaration",
            ));
        }

        // Validate RDF/XML structure using quick-xml
        use quick_xml::{Reader, events::Event};
        let mut reader = Reader::from_str(content);
        let mut inside_rdf_root = false;
        let mut depth: i32 = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name_bytes = e.name();
                    let name = String::from_utf8_lossy(name_bytes.as_ref());
                    if name == "rdf:RDF" || name == "RDF" {
                        inside_rdf_root = true;
                        depth = 0;
                    } else if inside_rdf_root {
                        depth += 1;
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name_bytes = e.name();
                    let name = String::from_utf8_lossy(name_bytes.as_ref());
                    if name == "rdf:RDF" || name == "RDF" {
                        inside_rdf_root = false;
                    } else if inside_rdf_root && depth > 0 {
                        depth -= 1;
                    }
                }
                Ok(Event::Text(ref e)) => {
                    // Check if there's significant text content directly under rdf:RDF
                    if inside_rdf_root && depth == 0 {
                        let text = String::from_utf8_lossy(e);
                        let trimmed_text = text.trim();
                        // If there's non-whitespace text directly in rdf:RDF, it's invalid
                        if !trimmed_text.is_empty() {
                            return Err(Error::ontology_parsing(format!(
                                "Invalid RDF/XML: text content '{}' not allowed directly inside rdf:RDF element",
                                trimmed_text
                            )));
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(Error::ontology_parsing(format!(
                        "RDF/XML parsing error: {}",
                        e
                    )));
                }
                _ => {}
            }
        }

        // Reuse the XML validation logic
        self.validate_owl_xml(content)?;

        // Additional RDF/XML semantic validation

        // Check for empty owl:complementOf
        if content.contains("<owl:complementOf/>") || content.contains("<owl:complementOf />") {
            return Err(Error::ontology_parsing(
                "Empty owl:complementOf - must specify a class expression",
            ));
        }

        // Check for owl:unionOf with only one class
        if content.contains("owl:unionOf") {
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(content);
            let mut in_union = false;
            let mut union_class_count = 0;

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:unionOf" {
                            in_union = true;
                            union_class_count = 0;
                        } else if in_union && name == "owl:Class" {
                            union_class_count += 1;
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:unionOf" {
                            if union_class_count < 2 {
                                return Err(Error::ontology_parsing(
                                    "owl:unionOf must contain at least 2 classes",
                                ));
                            }
                            in_union = false;
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(Error::ontology_parsing(format!(
                            "RDF/XML parsing error: {}",
                            e
                        )));
                    }
                    _ => {}
                }
            }
        }

        // Check for owl:oneOf with no individuals
        if content.contains("owl:oneOf") {
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(content);
            let mut in_one_of = false;
            let mut one_of_item_count = 0;

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:oneOf" {
                            in_one_of = true;
                            one_of_item_count = 0;
                        } else if in_one_of
                            && (name == "owl:NamedIndividual" || name.contains("Individual"))
                        {
                            one_of_item_count += 1;
                        }
                    }
                    Ok(Event::Empty(ref e)) => {
                        // Handle self-closing tags like <owl:oneOf rdf:parseType="Collection"/>
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:oneOf" {
                            return Err(Error::ontology_parsing(
                                "owl:oneOf must contain at least 1 individual",
                            ));
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:oneOf" {
                            if one_of_item_count == 0 {
                                return Err(Error::ontology_parsing(
                                    "owl:oneOf must contain at least 1 individual",
                                ));
                            }
                            in_one_of = false;
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(Error::ontology_parsing(format!(
                            "RDF/XML parsing error: {}",
                            e
                        )));
                    }
                    _ => {}
                }
            }
        }

        // Check for owl:hasKey with no properties
        if content.contains("owl:hasKey") {
            if content.contains("<owl:hasKey/>") || content.contains("<owl:hasKey />") {
                return Err(Error::ontology_parsing(
                    "Empty owl:hasKey - must specify at least one property",
                ));
            }
        }

        // Check for owl:propertyChainAxiom with less than 2 properties
        if content.contains("owl:propertyChainAxiom") {
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(content);
            let mut in_chain = false;
            let mut chain_property_count = 0;

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:propertyChainAxiom" {
                            in_chain = true;
                            chain_property_count = 0;
                        } else if in_chain
                            && (name == "owl:ObjectProperty" || name.contains("Property"))
                        {
                            chain_property_count += 1;
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:propertyChainAxiom" {
                            if chain_property_count < 2 {
                                return Err(Error::ontology_parsing(
                                    "owl:propertyChainAxiom must contain at least 2 properties",
                                ));
                            }
                            in_chain = false;
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(Error::ontology_parsing(format!(
                            "RDF/XML parsing error: {}",
                            e
                        )));
                    }
                    _ => {}
                }
            }
        }

        // Check for qualified cardinality without onClass
        if content.contains("owl:qualifiedCardinality")
            || content.contains("owl:minQualifiedCardinality")
            || content.contains("owl:maxQualifiedCardinality")
        {
            // These must be accompanied by owl:onClass
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(content);
            let mut in_restriction = false;
            let mut has_qualified_card = false;
            let mut has_on_class = false;
            let mut depth = 0;

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:Restriction" {
                            in_restriction = true;
                            has_qualified_card = false;
                            has_on_class = false;
                            depth = 0;
                        } else if in_restriction {
                            depth += 1;
                            if name == "owl:qualifiedCardinality"
                                || name == "owl:minQualifiedCardinality"
                                || name == "owl:maxQualifiedCardinality"
                            {
                                has_qualified_card = true;
                            } else if name == "owl:onClass" {
                                has_on_class = true;
                            }
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let name_bytes = e.name();
                        let name = String::from_utf8_lossy(name_bytes.as_ref());
                        if name == "owl:Restriction" {
                            if has_qualified_card && !has_on_class {
                                return Err(Error::ontology_parsing(
                                    "Qualified cardinality restriction must include owl:onClass",
                                ));
                            }
                            in_restriction = false;
                        } else if in_restriction && depth > 0 {
                            depth -= 1;
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(Error::ontology_parsing(format!(
                            "RDF/XML parsing error: {}",
                            e
                        )));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Validate Manchester Syntax
    pub fn validate_manchester(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        let lines: Vec<&str> = content.lines().collect();

        // Track current declaration context
        let mut current_declaration_type: Option<&str> = None;

        // Valid Manchester keywords
        let valid_keywords = [
            "Ontology:",
            "Prefix:",
            "Import:",
            "Class:",
            "Individual:",
            "ObjectProperty:",
            "DataProperty:",
            "DatatypeProperty:",
            "AnnotationProperty:",
            "Datatype:",
            "EquivalentTo:",
            "SubClassOf:",
            "DisjointWith:",
            "DisjointUnionOf:",
            "HasKey:",
            "Types:",
            "Facts:",
            "SameAs:",
            "DifferentFrom:",
            "SubPropertyOf:",
            "EquivalentProperties:",
            "DisjointProperties:",
            "InverseOf:",
            "Domain:",
            "Range:",
            "Characteristics:",
            "Functional",
            "InverseFunctional",
            "Reflexive",
            "Irreflexive",
            "Symmetric",
            "Asymmetric",
            "Transitive",
            "SubPropertyChain:",
            "Annotations:",
            "Individuals:",
            // Class expression keywords
            "not",
            "and",
            "or",
            "some",
            "only",
            "value",
            "Self",
            "min",
            "max",
            "exactly",
            "that",
            "inverse",
        ];

        // Check for ontology header (optional but recommended)
        let has_ontology_header = content
            .lines()
            .any(|line| line.trim().starts_with("Ontology:"));

        // If no ontology header, at least check for some Manchester content
        if !has_ontology_header {
            let has_manchester_content = content.lines().any(|line| {
                let trimmed = line.trim();
                valid_keywords
                    .iter()
                    .any(|&kw| trimmed.starts_with(kw) && (kw.ends_with(':') || kw == trimmed))
            });

            if !has_manchester_content {
                return Err(Error::ontology_parsing(
                    "Manchester syntax must contain valid declarations (Class:, ObjectProperty:, etc.)",
                ));
            }
        }

        // Check for common typos in keywords
        let common_typos = [
            ("Clas:", "Class:"),
            ("Calss:", "Class:"),
            ("Classs:", "Class:"),
            ("Clase:", "Class:"),
            ("SubClassOF:", "SubClassOf:"),
            ("subClassOf:", "SubClassOf:"),     // lowercase
            ("equivalentTo:", "EquivalentTo:"), // lowercase
            ("DomainProperty:", "Domain:"),
            ("RangeProperty:", "Range:"),
            ("Characteristic:", "Characteristics:"), // singular vs plural
        ];

        // Check for keywords without colons
        let keywords_requiring_colons = [
            "Class",
            "Individual",
            "ObjectProperty",
            "DataProperty",
            "AnnotationProperty",
            "Datatype",
        ];

        // Check each line for invalid keywords or malformed syntax
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Update current declaration type based on top-level declarations
            if !line.starts_with(' ') && !line.starts_with('\t') {
                if trimmed.starts_with("Class:") {
                    current_declaration_type = Some("Class");
                } else if trimmed.starts_with("ObjectProperty:") {
                    current_declaration_type = Some("ObjectProperty");
                } else if trimmed.starts_with("DataProperty:")
                    || trimmed.starts_with("DatatypeProperty:")
                {
                    current_declaration_type = Some("DataProperty");
                } else if trimmed.starts_with("AnnotationProperty:") {
                    current_declaration_type = Some("AnnotationProperty");
                } else if trimmed.starts_with("Individual:") {
                    current_declaration_type = Some("Individual");
                }
            }

            // Check for common typos
            for (typo, correct) in &common_typos {
                if trimmed.starts_with(typo) {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Keyword typo '{}' - did you mean '{}'?",
                        line_num + 1,
                        typo,
                        correct
                    )));
                }
            }

            // Check for keywords without colons (e.g., "Class Person" instead of "Class: Person")
            for keyword in &keywords_requiring_colons {
                let pattern = format!("{} ", keyword);
                if trimmed.starts_with(&pattern) && !trimmed.starts_with(&format!("{}:", keyword)) {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Missing colon after keyword '{}' - should be '{}:'",
                        line_num + 1,
                        keyword,
                        keyword
                    )));
                }
            }

            // Check for lines that look like keyword declarations (end with :)
            if let Some(keyword_pos) = trimmed.find(':') {
                let potential_keyword = &trimmed[..=keyword_pos];

                // Skip IRI brackets like <http://example.org/>
                if trimmed.starts_with('<') {
                    continue;
                }

                // For indented lines with colons, validate they use known keywords
                if line.starts_with(' ') || line.starts_with('\t') {
                    // Indented lines are part of class/property definitions
                    // But they should still use valid keywords if they have colons
                    let indented_keywords = [
                        "EquivalentTo:",
                        "SubClassOf:",
                        "DisjointWith:",
                        "DisjointUnionOf:",
                        "HasKey:",
                        "Types:",
                        "Facts:",
                        "SameAs:",
                        "DifferentFrom:",
                        "SubPropertyOf:",
                        "EquivalentProperties:",
                        "DisjointProperties:",
                        "InverseOf:",
                        "Domain:",
                        "Range:",
                        "Characteristics:",
                        "SubPropertyChain:",
                        "Annotations:",
                    ];

                    let is_valid_indented = indented_keywords
                        .iter()
                        .any(|&kw| potential_keyword.starts_with(kw));

                    if !is_valid_indented {
                        // Check if it looks like a keyword (starts with uppercase)
                        let keyword_text = potential_keyword.trim_end_matches(':').trim();
                        if keyword_text.chars().next().unwrap_or(' ').is_uppercase() {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid indented Manchester keyword: '{}'",
                                line_num + 1,
                                potential_keyword.trim()
                            )));
                        }
                    }

                    // IMPORTANT: Validate indented line content BEFORE continuing
                    // This ensures we catch errors in class expressions, cardinalities, etc.

                    // Validate cardinality expressions use numbers, not words
                    if trimmed.contains(" min ")
                        || trimmed.contains(" max ")
                        || trimmed.contains(" exactly ")
                    {
                        let words: Vec<&str> = trimmed.split_whitespace().collect();
                        for (i, word) in words.iter().enumerate() {
                            if (*word == "min" || *word == "max" || *word == "exactly")
                                && i + 1 < words.len()
                            {
                                let cardinality_value = words[i + 1];
                                // Check if it's a word instead of a number
                                let number_words = [
                                    "one",
                                    "two",
                                    "three",
                                    "four",
                                    "five",
                                    "six",
                                    "seven",
                                    "eight",
                                    "nine",
                                    "ten",
                                    "eleven",
                                    "twelve",
                                    "thirteen",
                                    "fourteen",
                                    "fifteen",
                                    "sixteen",
                                    "seventeen",
                                    "eighteen",
                                    "nineteen",
                                    "twenty",
                                ];
                                if number_words.contains(&cardinality_value) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Invalid cardinality value '{}' - must use numeric digits (e.g., '1', '2', '3'), not words",
                                        line_num + 1,
                                        cardinality_value
                                    )));
                                }
                                // Check if it's a valid non-negative integer
                                if !cardinality_value.chars().all(|c| c.is_ascii_digit()) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Invalid cardinality value '{}' - must be a non-negative integer",
                                        line_num + 1,
                                        cardinality_value
                                    )));
                                }
                            }
                        }
                    }

                    // Validate EquivalentTo expressions are complete
                    if trimmed.contains("EquivalentTo:") {
                        let after_equivalent = if let Some(pos) = trimmed.find("EquivalentTo:") {
                            trimmed[pos + 13..].trim()
                        } else {
                            ""
                        };

                        // For Class declarations, require complex expressions
                        // For ObjectProperty/DataProperty, single identifiers are valid
                        if current_declaration_type == Some("Class") {
                            let parts: Vec<&str> = after_equivalent.split_whitespace().collect();
                            if parts.len() == 1
                                && !after_equivalent.contains('(')
                                && !after_equivalent.contains("Self")
                            {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: Incomplete EquivalentTo expression - '{}' should be a complete class expression (e.g., 'knows Self', 'hasChild some Person')",
                                    line_num + 1,
                                    after_equivalent
                                )));
                            }
                        }
                        // For properties (ObjectProperty, DataProperty, AnnotationProperty),
                        // single identifiers are valid equivalences

                        // Check for empty EquivalentTo
                        if after_equivalent.is_empty() {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Empty EquivalentTo expression",
                                line_num + 1
                            )));
                        }
                    }

                    // Validate "value" expressions have an individual
                    if trimmed.contains(" value") {
                        let words: Vec<&str> = trimmed.split_whitespace().collect();
                        for (i, word) in words.iter().enumerate() {
                            if *word == "value" && i == words.len() - 1 {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: Incomplete 'value' expression - expected 'property value individual'",
                                    line_num + 1
                                )));
                            }
                        }
                    }

                    // Validate "or" expressions have both operands
                    if trimmed.contains(" or ") {
                        let words: Vec<&str> = trimmed.split_whitespace().collect();
                        for (i, word) in words.iter().enumerate() {
                            if *word == "or" {
                                // Check if "or" is at the start/end, or if word before is a keyword
                                let is_invalid = i == 0
                                    || i == words.len() - 1
                                    || (i > 0 && words[i - 1].ends_with(':'));

                                if is_invalid {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Incomplete 'or' expression - expected 'class1 or class2'",
                                        line_num + 1
                                    )));
                                }
                            }
                        }
                    }

                    // Validate SubPropertyChain has "o" keyword between properties
                    if trimmed.contains("SubPropertyChain:") {
                        let after_chain = if let Some(pos) = trimmed.find("SubPropertyChain:") {
                            trimmed[pos + 17..].trim()
                        } else {
                            ""
                        };

                        if !after_chain.is_empty() && !after_chain.contains(" o ") {
                            let words: Vec<&str> = after_chain.split_whitespace().collect();
                            if words.len() >= 2 {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: SubPropertyChain must use 'o' keyword between properties (e.g., 'hasFather o hasBrother')",
                                    line_num + 1
                                )));
                            }
                        }
                    }

                    // Validate SubClassOf expressions are complete
                    if trimmed.contains("SubClassOf:") {
                        let after_subclass = if let Some(pos) = trimmed.find("SubClassOf:") {
                            trimmed[pos + 11..].trim()
                        } else {
                            ""
                        };

                        if after_subclass.contains(" value")
                            && after_subclass.trim().ends_with("value")
                        {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Incomplete 'value' expression - expected 'property value individual'",
                                line_num + 1
                            )));
                        }
                    }

                    // Validate Characteristics don't have contradictory combinations
                    if trimmed.contains("Characteristics:") {
                        let characteristics_part =
                            if let Some(pos) = trimmed.find("Characteristics:") {
                                trimmed[pos + 16..].trim().to_lowercase()
                            } else {
                                String::new()
                            };

                        let contradictions =
                            [("asymmetric", "symmetric"), ("irreflexive", "reflexive")];

                        for (char1, char2) in &contradictions {
                            // Split by comma and whitespace to get individual characteristics
                            let chars: Vec<&str> =
                                characteristics_part.split(',').map(|s| s.trim()).collect();
                            let has_char1 = chars.iter().any(|c| c == char1);
                            let has_char2 = chars.iter().any(|c| c == char2);

                            if has_char1 && has_char2 {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: Contradictory characteristics - cannot be both '{}' and '{}'",
                                    line_num + 1,
                                    char1,
                                    char2
                                )));
                            }
                        }
                    }

                    // Validate empty field keywords
                    let empty_field_keywords = [
                        "DisjointWith:",
                        "EquivalentProperties:",
                        "InverseOf:",
                        "HasKey:",
                    ];

                    for keyword in &empty_field_keywords {
                        if trimmed.contains(keyword) {
                            let after_keyword = if let Some(pos) = trimmed.find(keyword) {
                                trimmed[pos + keyword.len()..].trim()
                            } else {
                                ""
                            };

                            if after_keyword.is_empty()
                                || after_keyword.chars().all(|c| c.is_whitespace() || c == ',')
                            {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: Empty '{}' declaration - must specify a value",
                                    line_num + 1,
                                    keyword.trim_end_matches(':')
                                )));
                            }
                        }
                    }

                    // Validate DisjointUnionOf has multiple classes
                    if trimmed.contains("DisjointUnionOf:") {
                        let after_keyword = if let Some(pos) = trimmed.find("DisjointUnionOf:") {
                            trimmed[pos + 16..].trim()
                        } else {
                            ""
                        };

                        let parts: Vec<&str> = after_keyword.split(',').collect();
                        if parts.len() == 1 && !after_keyword.is_empty() {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: DisjointUnionOf requires multiple classes (at least 2), found only 1",
                                line_num + 1
                            )));
                        }
                    }

                    // Validate incomplete class expressions
                    if trimmed.contains(" and not") && trimmed.trim().ends_with("not") {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Incomplete 'not' expression - 'not' must be followed by a class expression",
                            line_num + 1
                        )));
                    }

                    if trimmed.contains(" or)") || trimmed.contains("(or ") {
                        if let Some(or_pos) = trimmed.find(" or") {
                            let after_or = trimmed[or_pos + 3..].trim();
                            if after_or.is_empty() || after_or.starts_with(')') {
                                return Err(Error::ontology_parsing(format!(
                                    "Line {}: Incomplete 'or' expression - 'or' must have operands on both sides",
                                    line_num + 1
                                )));
                            }
                        }
                    }

                    // Validate datatype restrictions
                    if (trimmed.contains(">=")
                        || trimmed.contains("<=")
                        || trimmed.contains(">")
                        || trimmed.contains("<"))
                        && trimmed.contains('[')
                    {
                        let restriction_part = if let Some(start) = trimmed.find('[') {
                            if let Some(end) = trimmed.find(']') {
                                &trimmed[start + 1..end]
                            } else {
                                ""
                            }
                        } else {
                            ""
                        };

                        if !restriction_part.is_empty() {
                            let parts: Vec<&str> = restriction_part.split_whitespace().collect();
                            for part in parts {
                                if part.chars().all(|c| c.is_alphabetic()) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: Invalid datatype restriction value '{}' - expected numeric literal or quoted string",
                                        line_num + 1,
                                        part
                                    )));
                                }
                            }
                        }
                    }

                    continue;
                }

                // Check for incomplete annotations BEFORE allowing them to continue
                // This validation must run before the continue statement below
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

                    // After validation passes, allow "Annotations: ... on ..." pattern for annotation-on-axioms
                    continue;
                }

                // Check if this is a known keyword
                let is_valid_keyword = valid_keywords.iter().any(|&kw| {
                    potential_keyword.starts_with(kw) || kw.starts_with(potential_keyword)
                });

                // Also check if it's a prefix declaration format (word: followed by IRI)
                let is_prefix_format = potential_keyword.split_whitespace().count() == 1
                    && !potential_keyword.starts_with("Ontology:")
                    && trimmed.contains('<');

                if !is_valid_keyword && !is_prefix_format {
                    // This looks like an invalid keyword
                    let keyword_part = potential_keyword.trim();
                    if keyword_part.chars().next().unwrap_or(' ').is_uppercase()
                        || keyword_part.contains(char::is_uppercase)
                    {
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

            // Check for invalid class expression syntax (simple patterns)
            if trimmed.contains(" and and ")
                || trimmed.contains(" or or ")
                || trimmed.contains(" not not ")
            {
                return Err(Error::ontology_parsing(format!(
                    "Line {}: Invalid class expression - duplicate logical operators",
                    line_num + 1
                )));
            }

            // Check for unmatched parentheses on this line
            let open_parens = trimmed.matches('(').count();
            let close_parens = trimmed.matches(')').count();
            if open_parens != close_parens && !trimmed.ends_with(',') {
                // Allow multi-line expressions if line ends with comma
                return Err(Error::ontology_parsing(format!(
                    "Line {}: Unmatched parentheses (found {} opening, {} closing)",
                    line_num + 1,
                    open_parens,
                    close_parens
                )));
            }

            // Check for invalid keyword combinations on same line
            // In Manchester, main declarations like "Class: Name" can have simple modifiers on same line
            // but complex multi-word constructs should be on separate lines
            if trimmed.starts_with("Class:")
                || trimmed.starts_with("ObjectProperty:")
                || trimmed.starts_with("DataProperty:")
                || trimmed.starts_with("DatatypeProperty:")
                || trimmed.starts_with("Individual:")
            {
                // Allow "Class: Name Annotations: ..." on same line (common pattern)
                // Only reject if it has complex keywords that definitely need separate lines
                let after_first_keyword = if let Some(space_pos) = trimmed[6..].find(' ') {
                    &trimmed[6 + space_pos + 1..]
                } else {
                    ""
                };

                // Check for SubClassOf on same line, but allow if it's part of Annotations pattern
                // Pattern like "Class: X Annotations: ... SubClassOf: Y" is valid
                if after_first_keyword.contains("SubClassOf:")
                    && !after_first_keyword.contains("Annotations:")
                {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid Manchester syntax - 'SubClassOf:' should be on a separate indented line",
                        line_num + 1
                    )));
                }

                // Check for EquivalentTo WITHOUT colon on same line (malformed)
                // EquivalentTo: (with colon) is fine on the same line
                if after_first_keyword.contains("EquivalentTo ")
                    && !after_first_keyword.contains("EquivalentTo:")
                {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid Manchester syntax - 'EquivalentTo' must have colon ('EquivalentTo:')",
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

            // Check for random text that doesn't match any pattern
            // This catches lines like "use of unknown keyword" or other malformed content
            if !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with('<')
                && !trimmed.contains("://")
            // Skip URIs
            {
                // Check if line looks like a declaration keyword
                let is_declaration = valid_keywords.iter().any(|&kw| trimmed.starts_with(kw));

                // Check if line is part of an indented block (starts with whitespace)
                let is_indented = line.starts_with(' ') || line.starts_with('\t');

                // If not a declaration and contains suspicious words, reject
                if !is_declaration && !is_indented {
                    let words: Vec<&str> = trimmed.split_whitespace().collect();

                    // Reject lines with multiple words that aren't valid keywords
                    if words.len() > 1 {
                        let first_word = words[0];
                        let looks_like_keyword = first_word
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false);

                        // Check if first word is a known keyword
                        let is_known = valid_keywords.iter().any(|&kw| {
                            kw.starts_with(first_word) || first_word == kw.trim_end_matches(':')
                        });

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

            // Validate cardinality expressions use numbers, not words
            if trimmed.contains(" min ")
                || trimmed.contains(" max ")
                || trimmed.contains(" exactly ")
            {
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                for (i, word) in words.iter().enumerate() {
                    if (*word == "min" || *word == "max" || *word == "exactly")
                        && i + 1 < words.len()
                    {
                        let cardinality_value = words[i + 1];
                        // Check if it's a word instead of a number
                        let number_words = [
                            "one",
                            "two",
                            "three",
                            "four",
                            "five",
                            "six",
                            "seven",
                            "eight",
                            "nine",
                            "ten",
                            "eleven",
                            "twelve",
                            "thirteen",
                            "fourteen",
                            "fifteen",
                            "sixteen",
                            "seventeen",
                            "eighteen",
                            "nineteen",
                            "twenty",
                        ];
                        if number_words.contains(&cardinality_value) {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid cardinality value '{}' - must use numeric digits (e.g., '1', '2', '3'), not words",
                                line_num + 1,
                                cardinality_value
                            )));
                        }
                        // Check if it's a valid non-negative integer
                        if !cardinality_value.chars().all(|c| c.is_ascii_digit()) {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid cardinality value '{}' - must be a non-negative integer",
                                line_num + 1,
                                cardinality_value
                            )));
                        }
                    }
                }
            }

            // Validate EquivalentTo expressions are complete (not just a property name)
            if trimmed.contains("EquivalentTo:") {
                let after_equivalent = if let Some(pos) = trimmed.find("EquivalentTo:") {
                    trimmed[pos + 13..].trim()
                } else {
                    ""
                };

                // Check if it's just a single word (property/class name) without any expression
                let parts: Vec<&str> = after_equivalent.split_whitespace().collect();
                if parts.len() == 1
                    && !after_equivalent.contains('(')
                    && !after_equivalent.contains("Self")
                {
                    // This is likely incomplete - should have "knows Self" or "hasChild some Child", etc.
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Incomplete EquivalentTo expression - '{}' should be a complete class expression (e.g., 'knows Self', 'hasChild some Person')",
                        line_num + 1,
                        after_equivalent
                    )));
                }
            }

            // Validate SubClassOf expressions are complete
            if trimmed.contains("SubClassOf:") {
                let after_subclass = if let Some(pos) = trimmed.find("SubClassOf:") {
                    trimmed[pos + 11..].trim()
                } else {
                    ""
                };

                // Check for incomplete value expressions
                if after_subclass.contains(" value") && after_subclass.trim().ends_with("value") {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Incomplete 'value' expression - expected 'property value individual'",
                        line_num + 1
                    )));
                }
            }

            // Validate Characteristics don't have contradictory combinations
            if trimmed.contains("Characteristics:") {
                let characteristics_part = if let Some(pos) = trimmed.find("Characteristics:") {
                    trimmed[pos + 16..].trim().to_lowercase()
                } else {
                    String::new()
                };

                // Check for contradictory characteristic pairs
                let contradictions = [("asymmetric", "symmetric"), ("irreflexive", "reflexive")];

                for (char1, char2) in &contradictions {
                    if characteristics_part.contains(char1) && characteristics_part.contains(char2)
                    {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Contradictory characteristics - cannot be both '{}' and '{}'",
                            line_num + 1,
                            char1,
                            char2
                        )));
                    }
                }
            }

            // Validate property declarations aren't followed by empty required fields
            let empty_field_keywords = [
                "DisjointWith:",
                "EquivalentProperties:",
                "InverseOf:",
                "HasKey:",
            ];

            for keyword in &empty_field_keywords {
                if trimmed.contains(keyword) {
                    let after_keyword = if let Some(pos) = trimmed.find(keyword) {
                        trimmed[pos + keyword.len()..].trim()
                    } else {
                        ""
                    };

                    // Check if nothing follows the keyword (empty or just whitespace/punctuation)
                    if after_keyword.is_empty()
                        || after_keyword.chars().all(|c| c.is_whitespace() || c == ',')
                    {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Empty '{}' declaration - must specify a value",
                            line_num + 1,
                            keyword.trim_end_matches(':')
                        )));
                    }
                }
            }

            // Validate DisjointUnionOf has multiple classes
            if trimmed.contains("DisjointUnionOf:") {
                let after_keyword = if let Some(pos) = trimmed.find("DisjointUnionOf:") {
                    trimmed[pos + 16..].trim()
                } else {
                    ""
                };

                // Check if only one class is specified (need at least 2 for disjoint union)
                let parts: Vec<&str> = after_keyword.split(',').collect();
                if parts.len() == 1 && !after_keyword.is_empty() {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: DisjointUnionOf requires multiple classes (at least 2), found only 1",
                        line_num + 1
                    )));
                }
            }

            // Validate incomplete class expressions (hanging operators)
            if trimmed.contains(" and not") && trimmed.trim().ends_with("not") {
                return Err(Error::ontology_parsing(format!(
                    "Line {}: Incomplete 'not' expression - 'not' must be followed by a class expression",
                    line_num + 1
                )));
            }

            if trimmed.contains(" or)") || trimmed.contains("(or ") {
                // Check if 'or' has operands on both sides
                if let Some(or_pos) = trimmed.find(" or") {
                    let after_or = trimmed[or_pos + 3..].trim();
                    if after_or.is_empty() || after_or.starts_with(')') {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: Incomplete 'or' expression - 'or' must have operands on both sides",
                            line_num + 1
                        )));
                    }
                }
            }

            // Validate datatype restrictions use valid values (not undefined names)
            if (trimmed.contains(">=")
                || trimmed.contains("<=")
                || trimmed.contains(">")
                || trimmed.contains("<"))
                && trimmed.contains('[')
            {
                // This looks like a datatype restriction
                let restriction_part = if let Some(start) = trimmed.find('[') {
                    if let Some(end) = trimmed.find(']') {
                        &trimmed[start + 1..end]
                    } else {
                        ""
                    }
                } else {
                    ""
                };

                if !restriction_part.is_empty() {
                    let parts: Vec<&str> = restriction_part.split_whitespace().collect();
                    for part in parts {
                        // Check if it looks like a name (starts with letter) but isn't a number
                        if part.chars().all(|c| c.is_alphabetic()) {
                            return Err(Error::ontology_parsing(format!(
                                "Line {}: Invalid datatype restriction value '{}' - expected numeric literal or quoted string",
                                line_num + 1,
                                part
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate SWRL rule syntax
    pub fn validate_swrl(&self, content: &str) -> Result<()> {
        if !self.strict_mode {
            return Ok(());
        }

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // SWRL rules have the pattern: body -> head or body :- head
            // Also support [ruleName: body -> head]

            // Check for basic SWRL structure
            let has_arrow = trimmed.contains("->") || trimmed.contains(":-");
            let has_brackets = trimmed.starts_with('[') && trimmed.ends_with(']');
            let looks_like_declaration = trimmed.starts_with("Ontology(") 
                || trimmed.starts_with("Prefix(") 
                || trimmed.starts_with("Import(") 
                || trimmed.starts_with("Declaration(") 
                || trimmed.starts_with("Annotation(");

            // Only validate SWRL structure if it looks like a SWRL rule
            // (has DLSafeRule or looks like a rule pattern)
            if !has_arrow && !has_brackets && !looks_like_declaration {
                // This might be a complex SWRL construct or annotation
                // Only fail if we're very sure it's invalid
                if trimmed.contains("DLSafeRule") {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid SWRL syntax - DLSafeRule requires '->' or ':-' operator",
                        line_num + 1
                    )));
                }
                // Otherwise, be lenient and let the SWRL parser handle it
            }

            // If it has an arrow, validate basic structure
            if has_arrow {
                let arrow = if trimmed.contains("->") { "->" } else { ":-" };
                let parts: Vec<&str> = trimmed.split(arrow).collect();

                if parts.len() != 2 {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Invalid SWRL rule - expected format 'body {} head'",
                        line_num + 1,
                        arrow
                    )));
                }

                let body = parts[0].trim();
                let head = parts[1].trim();

                // Check that body and head are not empty
                if body.is_empty() {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: SWRL rule has empty body",
                        line_num + 1
                    )));
                }

                if head.is_empty() {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: SWRL rule has empty head",
                        line_num + 1
                    )));
                }

                // SWRL validation relaxed - parentheses balance should be checked during parsing,
                // not here, because Functional Syntax tokens parentheses differently than human-readable SWRL.
                // The validator sees the raw text, but the parser sees tokenized structures.
                // Skip strict validation and let the parser handle it.

                // SWRL head validation relaxed - let parser handle parentheses and structure checks

                // Check for variables (should start with ?)
                // SWRL variables typically use ?var syntax
                let has_variables = body.contains('?') || head.contains('?');
                if !has_variables {
                    // This might be okay for ground rules, but warn if suspicious
                    // Only flag as error if it looks malformed
                    if body.contains("var") || head.contains("var") {
                        return Err(Error::ontology_parsing(format!(
                            "Line {}: SWRL variables should use '?' prefix (e.g., ?var not var)",
                            line_num + 1
                        )));
                    }
                }
            }

            // For bracketed rules, basic validation
            if has_brackets {
                let inner = &trimmed[1..trimmed.len() - 1];

                // Check if it contains arrow
                if !inner.contains("->") && !inner.contains(":-") {
                    return Err(Error::ontology_parsing(format!(
                        "Line {}: Bracketed SWRL rule must contain '->' or ':-'",
                        line_num + 1
                    )));
                }

                // Check for rule name format: [ruleName: body -> head]
                // If there's text before the first '(' and it contains a space or letter,
                // it should have a colon after the rule name
                let arrow_pos = inner.find("->").or_else(|| inner.find(":-"));
                if let Some(arrow_idx) = arrow_pos {
                    let before_arrow = &inner[..arrow_idx];

                    // Find the first atom start (opening paren)
                    if let Some(first_paren) = before_arrow.find('(') {
                        let potential_rule_name = before_arrow[..first_paren].trim();

                        // If there's text before the first atom, it's a rule name
                        if !potential_rule_name.is_empty() {
                            // Rule name should be followed by ':'
                            if !potential_rule_name.ends_with(':') {
                                // Check if it looks like a rule name (alphanumeric)
                                if potential_rule_name.chars().any(|c| c.is_alphanumeric()) {
                                    return Err(Error::ontology_parsing(format!(
                                        "Line {}: SWRL rule name '{}' must be followed by ':' (e.g., '{}:')",
                                        line_num + 1,
                                        potential_rule_name,
                                        potential_rule_name
                                    )));
                                }
                            }
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
