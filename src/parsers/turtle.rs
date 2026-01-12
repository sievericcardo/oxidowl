//! Turtle Parser
//!
//! This module implements comprehensive parsing of OWL 2 ontologies from Turtle format.
//! It handles complex turtle syntax including disjoint unions, lists, blank nodes, and multi-line statements.

use super::common::OntologySerializer;
use crate::{
    Error, Result,
    ontology::{
        Class, ClassExpression, DataProperty, DataPropertyExpression, DataRange, FacetRestriction,
        IRI, Individual, Literal, NamedIndividual, ObjectProperty, ObjectPropertyExpression,
        Ontology,
        axioms::{
            Axiom, ClassAssertionAxiom, DataPropertyAssertionAxiom, DeclarationAxiom,
            DisjointUnionAxiom, Entity, EquivalentClassesAxiom, ObjectPropertyAssertionAxiom,
            SubClassOfAxiom,
        },
    },
};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Parse an IRI from a string, handling both absolute and relative URIs
fn parse_iri_to_url(uri_str: &str) -> Result<url::Url> {
    // Try to parse as absolute URL first
    if let Ok(url) = url::Url::parse(uri_str) {
        Ok(url)
    } else {
        // If it's not a valid absolute URL, treat it as a simple IRI string
        // This handles relative URIs and other IRI formats
        if uri_str.is_empty() {
            Err(Error::ontology_parsing("Empty IRI string"))
        } else {
            // Create a simple URL-like structure for the IRI
            // Use a dummy base for relative URIs to make them parseable
            let full_uri = if uri_str.starts_with("http://") || uri_str.starts_with("https://") {
                uri_str.to_string()
            } else {
                format!("http://example.org/{}", uri_str.trim_start_matches('#'))
            };

            url::Url::parse(&full_uri)
                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))
        }
    }
}

/// Enhanced configuration for the Turtle parser
#[derive(Debug, Clone)]
pub struct TurtleParserConfig {
    /// Whether to allow relative IRIs (default: true)
    pub allow_relative_iris: bool,

    /// Whether to validate IRIs during parsing (default: true)
    pub validate_iris: bool,

    /// Whether to ignore comments (default: true)
    pub ignore_comments: bool,

    /// Whether to allow blank node labels (default: true)
    pub allow_blank_nodes: bool,

    /// Maximum prefix resolution depth (default: 10)
    pub max_prefix_depth: usize,

    /// Whether to perform strict Turtle compliance checking (default: false)
    pub strict_mode: bool,

    /// Whether to parse OWL constructs like disjoint unions (default: true)
    pub parse_owl_constructs: bool,

    /// Whether to handle multi-line statements (default: true)
    pub handle_multiline: bool,
}

impl Default for TurtleParserConfig {
    fn default() -> Self {
        Self {
            allow_relative_iris: true,
            validate_iris: true,
            ignore_comments: true,
            allow_blank_nodes: true,
            max_prefix_depth: 10,
            strict_mode: false,
            parse_owl_constructs: true,
            handle_multiline: true,
        }
    }
}

/// Enhanced Turtle Parser supporting complex OWL constructs
#[derive(Debug, Clone)]
pub struct TurtleParser {
    config: TurtleParserConfig,
}

/// Token types for enhanced parsing
#[derive(Debug, Clone, PartialEq)]
enum Token {
    IRI(String),
    PrefixedName(String, String), // prefix, local
    Literal(String),
    BlankNode(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Period,
    Keyword(String),
}

/// Parser state for handling complex structures
#[derive(Debug)]
struct ParseState {
    prefixes: HashMap<String, String>,
    base_uri: Option<String>,
    current_subject: Option<String>,
    current_predicate: Option<String>,
    blank_node_counter: u32,
}

impl TurtleParser {
    /// Create a new Turtle parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TurtleParserConfig::default(),
        }
    }

    /// Create a new Turtle parser with custom configuration
    #[must_use]
    pub fn with_config(config: TurtleParserConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &TurtleParserConfig {
        &self.config
    }

    /// Set a new configuration
    pub fn set_config(&mut self, config: TurtleParserConfig) {
        self.config = config;
    }
}

impl Default for TurtleParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Turtle from string content
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = TurtleParser::new();
    parser.parse_string(content)
}

impl TurtleParser {
    /// Parse Turtle content into an ontology with enhanced OWL construct support
    pub fn parse_string(&self, content: &str) -> Result<Ontology> {
        // Validate syntax before parsing
        let validator = super::validation::SyntaxValidator::new();
        validator.validate_turtle(content)?;

        let mut ontology = Ontology::new();
        let mut state = ParseState {
            prefixes: HashMap::new(),
            base_uri: None,
            current_subject: None,
            current_predicate: None,
            blank_node_counter: 0,
        };

        // Add default prefixes
        state.prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        state.prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        state.prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        state.prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        // Enhanced parsing with proper multi-line handling
        let normalized_content = self.normalize_content(content);
        let statements = self.split_into_statements(&normalized_content)?;

        for statement in statements {
            self.parse_statement(&statement, &mut ontology, &mut state)?;
        }

        // Extract ontology IRI if present
        if ontology.get_iri().is_none() {
            if let Some(iri) = Self::extract_ontology_iri_from_content(content) {
                ontology.set_ontology_iri(Some(iri));
            }
        }

        Ok(ontology)
    }
    
    /// Extract ontology IRI from Turtle content
    fn extract_ontology_iri_from_content(content: &str) -> Option<IRI> {
        // Match pattern: <http://...> rdf:type owl:Ontology
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("rdf:type") && trimmed.contains("owl:Ontology") {
                // Extract IRI between < and >
                if let Some(start) = trimmed.find('<') {
                    if let Some(end) = trimmed[start..].find('>') {
                        let iri_str = &trimmed[start + 1..start + end];
                        if iri_str.starts_with("http") {
                            return Some(IRI::new(iri_str));
                        }
                    }
                }
            }
        }
        None
    }

    /// Normalize content by handling multi-line statements properly
    fn normalize_content(&self, content: &str) -> String {
        let mut normalized = String::new();
        let mut in_string = false;
        let mut in_comment = false;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;

        for line in content.lines() {
            let mut line_content = String::new();
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            let mut angle_depth = 0; // Track angle brackets for IRIs

            while i < chars.len() {
                let ch = chars[i];

                match ch {
                    '<' if !in_string && !in_comment => angle_depth += 1,
                    '>' if !in_string && !in_comment => angle_depth -= 1,
                    '#' if !in_string && angle_depth == 0 => {
                        in_comment = true;
                        if self.config.ignore_comments {
                            break;
                        }
                    }
                    '"' => in_string = !in_string,
                    '[' if !in_string && !in_comment => bracket_depth += 1,
                    ']' if !in_string && !in_comment => bracket_depth -= 1,
                    '(' if !in_string && !in_comment => paren_depth += 1,
                    ')' if !in_string && !in_comment => paren_depth -= 1,
                    _ => {}
                }

                if !in_comment {
                    line_content.push(ch);
                }
                i += 1;
            }

            if !line_content.trim().is_empty() {
                // Check if statement continues on next line
                if bracket_depth > 0
                    || paren_depth > 0
                    || (!line_content.trim_end().ends_with('.')
                        && !line_content.trim_end().ends_with(';')
                        && !line_content.contains("@prefix")
                        && !line_content.contains("@base"))
                {
                    normalized.push_str(&line_content);
                    normalized.push(' ');
                } else {
                    normalized.push_str(&line_content);
                    normalized.push('\n');
                }
            }
            in_comment = false;
        }

        normalized
    }

    /// Split content into individual statements
    fn split_into_statements(&self, content: &str) -> Result<Vec<String>> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut in_string = false;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;
        let mut angle_depth = 0; // Track < > brackets for IRIs

        for ch in content.chars() {
            match ch {
                '"' => in_string = !in_string,
                '[' if !in_string => bracket_depth += 1,
                ']' if !in_string => bracket_depth -= 1,
                '(' if !in_string => paren_depth += 1,
                ')' if !in_string => paren_depth -= 1,
                '<' if !in_string => {
                    angle_depth += 1;
                }
                '>' if !in_string => {
                    angle_depth -= 1;
                }
                '.' if !in_string && bracket_depth == 0 && paren_depth == 0 && angle_depth == 0 => {
                    current_statement.push(ch);
                    let stmt = current_statement.trim().to_string();
                    if !stmt.is_empty() {
                        statements.push(stmt);
                    }
                    current_statement.clear();
                    continue;
                }
                _ => {}
            }
            current_statement.push(ch);
        }

        if !current_statement.trim().is_empty() {
            statements.push(current_statement.trim().to_string());
        }

        Ok(statements)
    }

    /// Parse an individual statement
    fn parse_statement(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        let trimmed = statement.trim();

        if trimmed.is_empty() {
            return Ok(());
        }

        // Handle prefix declarations
        if trimmed.starts_with("@prefix") {
            return self.parse_prefix_declaration(trimmed, state);
        }

        // Handle base declarations
        if trimmed.starts_with("@base") {
            return self.parse_base_declaration(trimmed, state);
        }

        // Parse as triple statement
        self.parse_triple_statement(trimmed, ontology, state)
    }

    /// Enhanced prefix declaration parsing
    fn parse_prefix_declaration(&self, statement: &str, state: &mut ParseState) -> Result<()> {
        // @prefix prefix: <uri> .
        // Find the IRI within angle brackets
        if let Some(start) = statement.find('<') {
            if let Some(end) = statement.find('>') {
                let uri = &statement[start + 1..end];

                // Extract prefix name (between @prefix and :)
                let prefix_part = &statement[7..start].trim(); // Skip "@prefix"
                if let Some(colon_pos) = prefix_part.find(':') {
                    let prefix_name = prefix_part[..colon_pos].trim();
                    state
                        .prefixes
                        .insert(prefix_name.to_string(), uri.to_string());
                }
            }
        }
        Ok(())
    }

    /// Parse base declaration
    fn parse_base_declaration(&self, statement: &str, state: &mut ParseState) -> Result<()> {
        if let Some(start) = statement.find('<') {
            if let Some(end) = statement.find('>') {
                state.base_uri = Some(statement[start + 1..end].to_string());
            }
        }
        Ok(())
    }

    /// Parse triple statement with enhanced OWL construct support
    fn parse_triple_statement(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        // Handle complex statements with blank nodes and lists
        if statement.contains("owl:disjointUnionOf") {
            return self.parse_disjoint_union(statement, ontology, state);
        }

        // Check if this statement contains both owl:equivalentClass and other predicates
        if statement.contains("owl:equivalentClass") {
            // Process the equivalent class part
            self.parse_equivalent_class_enhanced(statement, ontology, state)?;

            // Also check for additional predicates like rdfs:subClassOf in the same statement
            if statement.contains("rdfs:subClassOf") {
                // Extract and process the subClassOf relationship
                self.extract_and_process_subclass_from_statement(statement, ontology, state)?;
            }
            return Ok(());
        }

        // Parse standard triple
        let tokens = self.tokenize_statement(statement)?;

        // Handle semicolon-separated predicates, blank nodes, or collections
        // Also route to parse_semicolon_statement if object is a collection or blank node
        if tokens.iter().any(|t| matches!(t, Token::Semicolon))
            || tokens.iter().any(|t| matches!(t, Token::LeftBracket))
            || tokens.iter().any(|t| matches!(t, Token::LeftParen))
            || tokens.iter().any(|t| matches!(t, Token::Comma))
        {
            return self.parse_semicolon_statement(&tokens, ontology, state);
        }

        if tokens.len() >= 3 {
            let subject = self.resolve_token(&tokens[0], state)?;
            let predicate = self.resolve_token(&tokens[1], state)?;
            let object = self.resolve_token(&tokens[2], state)?;

            self.process_enhanced_triple(ontology, subject, predicate, object)?;
        }

        Ok(())
    }

    /// Parse semicolon statement with support for blank nodes and collections
    fn parse_semicolon_statement(
        &self,
        tokens: &[Token],
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }

        // Check for special case: subject ( collection ) . (no explicit predicate)
        // This is Turtle shorthand for defining the subject as equivalent to the collection
        if tokens.len() >= 3
            && !matches!(tokens[0], Token::LeftBracket)
            && matches!(tokens[1], Token::LeftParen)
        {
            let subject = self.resolve_token(&tokens[0], state)?;

            // Find the matching closing paren
            let mut depth = 0;
            let mut collection_end = 1;
            for (i, token) in tokens[1..].iter().enumerate() {
                match token {
                    Token::LeftParen => depth += 1,
                    Token::RightParen => {
                        depth -= 1;
                        if depth == 0 {
                            collection_end = i + 2; // +1 for offset, +1 to include )
                            break;
                        }
                    }
                    _ => {}
                }
            }

            // Extract collection tokens (between parens)
            if collection_end > 2 {
                let collection_tokens = &tokens[2..collection_end - 1];
                let list_id = self.parse_collection(collection_tokens, ontology, state)?;

                // Use owl:sameAs as the implicit predicate
                let predicate = "http://www.w3.org/2002/07/owl#sameAs".to_string();
                self.process_enhanced_triple(ontology, subject, predicate, list_id)?;
                return Ok(());
            }
        }

        // Check if the statement starts with a blank node (anonymous subject)
        let subject = if matches!(tokens[0], Token::LeftBracket) {
            // Generate a blank node ID for anonymous blank node
            state.blank_node_counter += 1;
            format!("_:b{}", state.blank_node_counter)
        } else {
            // Regular subject
            self.resolve_token(&tokens[0], state).map_err(|e| {
                Error::ontology_parsing(format!("Failed to resolve subject token: {}", e))
            })?
        };

        // If it was a blank node, skip to find where it closes
        let mut start_index = if matches!(tokens[0], Token::LeftBracket) {
            let mut depth = 1;
            let mut i = 1;
            while i < tokens.len() && depth > 0 {
                match tokens[i] {
                    Token::LeftBracket => depth += 1,
                    Token::RightBracket => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    i += 1;
                }
            }
            // Process the content inside the blank node
            if i > 1 {
                let blank_node_tokens = &tokens[1..i];
                self.parse_blank_node_content(blank_node_tokens, ontology, state, &subject)?;
            }
            i + 1 // Continue after the blank node
        } else {
            1 // Start after the regular subject
        };

        // Process predicate-object pairs separated by semicolons
        let mut current_predicate: Option<String> = None;

        while start_index < tokens.len() {
            // Skip semicolons (mark end of predicate-object list, reset current predicate)
            if matches!(tokens[start_index], Token::Semicolon) {
                start_index += 1;
                current_predicate = None;
                continue;
            }

            // Skip periods (end of statement)
            if matches!(tokens[start_index], Token::Period) {
                break;
            }

            // We need at least predicate and object
            if start_index + 1 >= tokens.len() {
                break;
            }

            // If we don't have a current predicate, this token should be a predicate
            // If we have a current predicate and see a comma, skip it and continue with same predicate
            if matches!(tokens[start_index], Token::Comma) {
                start_index += 1;
                // Continue with the same predicate for the next object
                continue;
            }

            let predicate = if let Some(ref pred) = current_predicate {
                // Reuse the current predicate for comma-separated objects
                pred.clone()
            } else {
                // Parse new predicate
                let pred = self
                    .resolve_token(&tokens[start_index], state)
                    .map_err(|e| {
                        Error::ontology_parsing(format!(
                            "Failed to resolve predicate token at index {}: {}",
                            start_index, e
                        ))
                    })?;
                start_index += 1; // Move past the predicate
                current_predicate = Some(pred.clone());
                pred
            };

            // Check if the object is a complex structure (blank node, list, etc.)
            if start_index < tokens.len() {
                // First, check if we've reached the end of the statement
                if matches!(tokens[start_index], Token::Period | Token::Semicolon) {
                    break;
                }

                match &tokens[start_index] {
                    Token::LeftBracket => {
                        // Parse blank node as object
                        let mut depth = 1;
                        start_index += 1; // Move past opening bracket
                        let blank_start = start_index;

                        while start_index < tokens.len() && depth > 0 {
                            match tokens[start_index] {
                                Token::LeftBracket => depth += 1,
                                Token::RightBracket => depth -= 1,
                                _ => {}
                            }
                            start_index += 1;
                        }

                        // Create a blank node for this anonymous object
                        state.blank_node_counter += 1;
                        let blank_node_id = format!("_:b{}", state.blank_node_counter);

                        // Parse the content of the blank node
                        if start_index > blank_start {
                            let blank_tokens = &tokens[blank_start..start_index - 1];
                            self.parse_blank_node_content(
                                blank_tokens,
                                ontology,
                                state,
                                &blank_node_id,
                            )?;
                        }

                        // Create triple with blank node as object
                        self.process_enhanced_triple(
                            ontology,
                            subject.clone(),
                            predicate.clone(),
                            blank_node_id,
                        )?;
                        continue;
                    }
                    Token::LeftParen => {
                        // Parse RDF collection ()
                        let mut depth = 1;
                        start_index += 1; // Move past opening paren
                        let collection_start = start_index;

                        while start_index < tokens.len() && depth > 0 {
                            match tokens[start_index] {
                                Token::LeftParen => depth += 1,
                                Token::RightParen => depth -= 1,
                                _ => {}
                            }
                            start_index += 1;
                        }

                        // Create RDF list from collection
                        if start_index > collection_start {
                            let collection_tokens = &tokens[collection_start..start_index - 1];
                            let list_id =
                                self.parse_collection(collection_tokens, ontology, state)?;

                            // Create triple with list as object
                            self.process_enhanced_triple(
                                ontology,
                                subject.clone(),
                                predicate.clone(),
                                list_id,
                            )?;
                        }
                        continue;
                    }
                    Token::Literal(lit_value) => {
                        // Handle literal values as data property assertions
                        let literal_value = lit_value.clone();
                        start_index += 1; // Move past literal

                        // Check if there's a type annotation (^^datatype)
                        let datatype = if start_index < tokens.len() {
                            let next_token = &tokens[start_index];
                            match next_token {
                                Token::Keyword(kw) if kw.starts_with("^^") => {
                                    // Extract datatype IRI
                                    let dt_str = kw[2..].to_string();
                                    start_index += 1; // Skip the type annotation

                                    // Resolve prefix if needed
                                    let resolved_dt =
                                        if dt_str.contains(':') && !dt_str.starts_with("http") {
                                            if let Some(colon_pos) = dt_str.find(':') {
                                                let prefix = &dt_str[..colon_pos];
                                                let local = &dt_str[colon_pos + 1..];
                                                if let Some(base) = state.prefixes.get(prefix) {
                                                    format!("{}{}", base, local)
                                                } else {
                                                    dt_str
                                                }
                                            } else {
                                                dt_str
                                            }
                                        } else {
                                            dt_str
                                        };

                                    let dt_iri = IRI::new(&resolved_dt);
                                    dt_iri.to_url().ok()
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };

                        // Create DataPropertyAssertion
                        let subject_ind = Individual::Named(NamedIndividual {
                            iri: IRI::new(&subject),
                        });
                        let data_property = DataProperty {
                            iri: IRI::new(&predicate),
                        };
                        let literal = Literal {
                            value: self.decode_escape_sequences(&literal_value.trim_matches('"')),
                            language: None,
                            datatype,
                        };

                        let axiom = DataPropertyAssertionAxiom {
                            id: generate_axiom_id(),
                            property: crate::ontology::DataPropertyExpression::DataProperty(
                                data_property,
                            ),
                            individual: subject_ind,
                            value: literal,
                            annotations: vec![],
                        };
                        ontology.add_axiom(Axiom::DataPropertyAssertion(axiom));
                        continue;
                    }
                    _ => {} // Continue with normal processing
                }
            }

            // Try to resolve the object token for simple objects
            let object = match self.resolve_token(&tokens[start_index], state) {
                Ok(obj) => obj,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to resolve object token at index {}: {}. Skipping this predicate-object pair.",
                        start_index, e
                    );
                    // Skip to next statement
                    while start_index < tokens.len()
                        && !matches!(tokens[start_index], Token::Period | Token::Semicolon)
                    {
                        start_index += 1;
                    }
                    current_predicate = None;
                    continue;
                }
            };

            // Process this triple
            self.process_enhanced_triple(ontology, subject.clone(), predicate.clone(), object)?;

            // Move to next object or predicate
            start_index += 1;

            // Handle comma-separated objects for the same predicate
            while start_index < tokens.len() && matches!(tokens[start_index], Token::Comma) {
                start_index += 1; // Skip comma
                if start_index < tokens.len()
                    && !matches!(tokens[start_index], Token::Semicolon | Token::Period)
                {
                    // Skip complex objects like blank nodes that start with [
                    if matches!(tokens[start_index], Token::LeftBracket) {
                        // Skip to the end of the blank node
                        let mut bracket_depth = 1;
                        start_index += 1;
                        while start_index < tokens.len() && bracket_depth > 0 {
                            match tokens[start_index] {
                                Token::LeftBracket => bracket_depth += 1,
                                Token::RightBracket => bracket_depth -= 1,
                                _ => {}
                            }
                            start_index += 1;
                        }
                        continue;
                    }

                    // Only process simple objects
                    match self.resolve_token(&tokens[start_index], state) {
                        Ok(next_object) => {
                            self.process_enhanced_triple(
                                ontology,
                                subject.clone(),
                                predicate.clone(),
                                next_object,
                            )?;
                            start_index += 1;
                        }
                        Err(_) => {
                            // Skip problematic tokens instead of failing
                            start_index += 1;
                        }
                    }
                }
            }

            // Reset current predicate if we hit a semicolon or period
            if start_index < tokens.len()
                && matches!(tokens[start_index], Token::Semicolon | Token::Period)
            {
                current_predicate = None;
            }
        }

        Ok(())
    }

    /// Parse the content of a blank node (between [ ])
    fn parse_blank_node_content(
        &self,
        tokens: &[Token],
        ontology: &mut Ontology,
        state: &mut ParseState,
        blank_node_id: &str,
    ) -> Result<()> {
        // Parse predicate-object pairs within the blank node
        let mut i = 0;
        let mut current_predicate: Option<String> = None;

        while i < tokens.len() {
            if matches!(tokens[i], Token::Semicolon) {
                i += 1;
                current_predicate = None;
                continue;
            }

            if matches!(tokens[i], Token::Comma) {
                i += 1;
                continue;
            }

            // Get predicate
            let predicate = if let Some(ref pred) = current_predicate {
                pred.clone()
            } else if i < tokens.len() {
                let pred = self.resolve_token(&tokens[i], state)?;
                i += 1;
                current_predicate = Some(pred.clone());
                pred
            } else {
                break;
            };

            // Get object
            if i < tokens.len() {
                let object = match &tokens[i] {
                    Token::LeftBracket => {
                        // Handle nested blank node
                        i += 1; // Skip the opening bracket
                        let mut bracket_depth = 1;
                        let start_idx = i;

                        // Find matching closing bracket
                        while i < tokens.len() && bracket_depth > 0 {
                            match &tokens[i] {
                                Token::LeftBracket => bracket_depth += 1,
                                Token::RightBracket => bracket_depth -= 1,
                                _ => {}
                            }
                            if bracket_depth > 0 {
                                i += 1;
                            }
                        }

                        // Parse the nested blank node content
                        let nested_blank_node_id = format!("_:b{}", state.blank_node_counter);
                        state.blank_node_counter += 1;

                        let nested_tokens = &tokens[start_idx..i];
                        self.parse_blank_node_content(
                            nested_tokens,
                            ontology,
                            state,
                            &nested_blank_node_id,
                        )?;

                        i += 1; // Skip the closing bracket
                        nested_blank_node_id
                    }
                    Token::LeftParen => {
                        // Handle collection
                        i += 1; // Skip the opening paren
                        let mut paren_depth = 1;
                        let start_idx = i;

                        // Find matching closing paren
                        while i < tokens.len() && paren_depth > 0 {
                            match &tokens[i] {
                                Token::LeftParen => paren_depth += 1,
                                Token::RightParen => paren_depth -= 1,
                                _ => {}
                            }
                            if paren_depth > 0 {
                                i += 1;
                            }
                        }

                        // Parse the collection
                        let collection_tokens = &tokens[start_idx..i];
                        let collection_id =
                            self.parse_collection(collection_tokens, ontology, state)?;

                        i += 1; // Skip the closing paren
                        collection_id
                    }
                    _ => {
                        // Regular token
                        let obj = self.resolve_token(&tokens[i], state)?;
                        i += 1;
                        obj
                    }
                };

                // Create triple
                self.process_enhanced_triple(
                    ontology,
                    blank_node_id.to_string(),
                    predicate,
                    object,
                )?;
            }
        }

        Ok(())
    }

    /// Parse an RDF collection () into a linked list structure
    fn parse_collection(
        &self,
        tokens: &[Token],
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<String> {
        if tokens.is_empty() {
            // Empty collection is rdf:nil
            return Ok("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil".to_string());
        }

        // Parse items in the collection
        let mut items = Vec::new();
        for token in tokens {
            if matches!(token, Token::Comma) {
                continue; // Skip commas
            }
            items.push(self.resolve_token(token, state)?);
        }

        // Create linked list structure using rdf:first and rdf:rest
        let rdf_first = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        let rdf_rest = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        let rdf_nil = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

        let mut current_list_node = format!("_:list{}", state.blank_node_counter);
        state.blank_node_counter += 1;
        let first_node = current_list_node.clone();

        for (idx, item) in items.iter().enumerate() {
            // Add rdf:first triple
            self.process_enhanced_triple(
                ontology,
                current_list_node.clone(),
                rdf_first.to_string(),
                item.clone(),
            )?;

            // Add rdf:rest triple
            let rest_value = if idx == items.len() - 1 {
                // Last item points to rdf:nil
                rdf_nil.to_string()
            } else {
                // Create next list node
                let next_node = format!("_:list{}", state.blank_node_counter);
                state.blank_node_counter += 1;
                next_node
            };

            self.process_enhanced_triple(
                ontology,
                current_list_node.clone(),
                rdf_rest.to_string(),
                rest_value.clone(),
            )?;

            current_list_node = rest_value;
        }

        Ok(first_node)
    }

    /// Parse statements with semicolon-separated predicates (OLD VERSION - REPLACED ABOVE)
    fn parse_semicolon_statement_old(
        &self,
        tokens: &[Token],
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }

        // Check if the statement starts with a blank node (anonymous subject)
        // These are complex structures we don't fully parse yet, so skip them
        if matches!(tokens[0], Token::LeftBracket) {
            return Ok(());
        }

        // First token should be the subject
        let subject = self.resolve_token(&tokens[0], state).map_err(|e| {
            Error::ontology_parsing(format!("Failed to resolve subject token: {}", e))
        })?;

        // Process predicate-object pairs separated by semicolons
        let mut i = 1;
        let mut current_predicate: Option<String> = None;

        while i < tokens.len() {
            // Skip semicolons (mark end of predicate-object list, reset current predicate)
            if matches!(tokens[i], Token::Semicolon) {
                i += 1;
                current_predicate = None;
                continue;
            }

            // Skip periods (end of statement)
            if matches!(tokens[i], Token::Period) {
                break;
            }

            // We need at least predicate and object
            if i + 1 >= tokens.len() {
                break;
            }

            // If we don't have a current predicate, this token should be a predicate
            // If we have a current predicate and see a comma, skip it and continue with same predicate
            if matches!(tokens[i], Token::Comma) {
                i += 1;
                // Continue with the same predicate for the next object
                continue;
            }

            let predicate = if let Some(ref pred) = current_predicate {
                // Reuse the current predicate for comma-separated objects
                pred.clone()
            } else {
                // Parse new predicate
                let pred = self.resolve_token(&tokens[i], state).map_err(|e| {
                    Error::ontology_parsing(format!(
                        "Failed to resolve predicate token at index {}: {}",
                        i, e
                    ))
                })?;
                i += 1; // Move past the predicate
                current_predicate = Some(pred.clone());
                pred
            };

            // Check if the object is a complex structure (blank node, list, etc.) that we should skip
            if i < tokens.len() {
                match &tokens[i] {
                    Token::LeftBracket | Token::LeftParen => {
                        // Skip complex structures (blank nodes, lists)
                        let mut depth = 1;
                        let is_bracket = matches!(tokens[i], Token::LeftBracket);
                        i += 1; // Move past opening bracket/paren

                        while i < tokens.len() && depth > 0 {
                            match tokens[i] {
                                Token::LeftBracket if is_bracket => depth += 1,
                                Token::RightBracket if is_bracket => depth -= 1,
                                Token::LeftParen if !is_bracket => depth += 1,
                                Token::RightParen if !is_bracket => depth -= 1,
                                _ => {}
                            }
                            i += 1;
                        }
                        // After closing bracket/paren, continue to check for comma or semicolon
                        continue;
                    }
                    Token::Literal(lit_value) => {
                        // Handle literal values as data property assertions
                        let literal_value = lit_value.clone();
                        i += 1; // Move past literal

                        // Check if there's a type annotation (^^datatype)
                        let datatype = if i < tokens.len() {
                            let next_token = &tokens[i];
                            match next_token {
                                Token::Keyword(kw) if kw.starts_with("^^") => {
                                    // Extract datatype IRI
                                    let dt_str = kw[2..].to_string();
                                    i += 1; // Skip the type annotation

                                    // Resolve prefix if needed
                                    let resolved_dt =
                                        if dt_str.contains(':') && !dt_str.starts_with("http") {
                                            if let Some(colon_pos) = dt_str.find(':') {
                                                let prefix = &dt_str[..colon_pos];
                                                let local = &dt_str[colon_pos + 1..];
                                                if let Some(base) = state.prefixes.get(prefix) {
                                                    format!("{}{}", base, local)
                                                } else {
                                                    dt_str
                                                }
                                            } else {
                                                dt_str
                                            }
                                        } else {
                                            dt_str
                                        };

                                    let dt_iri = IRI::new(&resolved_dt);
                                    dt_iri.to_url().ok()
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };

                        // Create DataPropertyAssertion
                        let subject_ind = Individual::Named(NamedIndividual {
                            iri: IRI::new(&subject),
                        });
                        let data_property = DataProperty {
                            iri: IRI::new(&predicate),
                        };
                        let literal = Literal {
                            value: literal_value.trim_matches('"').to_string(),
                            language: None,
                            datatype,
                        };

                        let axiom = DataPropertyAssertionAxiom {
                            id: generate_axiom_id(),
                            property: crate::ontology::DataPropertyExpression::DataProperty(
                                data_property,
                            ),
                            individual: subject_ind,
                            value: literal,
                            annotations: vec![],
                        };
                        ontology.add_axiom(Axiom::DataPropertyAssertion(axiom));
                        continue;
                    }
                    _ => {} // Continue with normal processing
                }
            }

            // Try to resolve the object token for simple objects
            let object = match self.resolve_token(&tokens[i], state) {
                Ok(obj) => obj,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to resolve object token at index {}: {}. Skipping this predicate-object pair.",
                        i, e
                    );
                    // Skip to next statement
                    while i < tokens.len() && !matches!(tokens[i], Token::Period | Token::Semicolon)
                    {
                        i += 1;
                    }
                    current_predicate = None;
                    continue;
                }
            };

            // Process this triple
            self.process_enhanced_triple(ontology, subject.clone(), predicate.clone(), object)?;

            // Move to next object or predicate
            i += 1;

            // Handle comma-separated objects for the same predicate
            // (Note: this is now mostly handled by the top-level comma check,
            //  but we keep this for additional complex cases)
            while i < tokens.len() && matches!(tokens[i], Token::Comma) {
                i += 1; // Skip comma
                if i < tokens.len() && !matches!(tokens[i], Token::Semicolon | Token::Period) {
                    // Skip complex objects like blank nodes that start with [
                    if matches!(tokens[i], Token::LeftBracket) {
                        // Skip to the end of the blank node
                        let mut bracket_depth = 1;
                        i += 1;
                        while i < tokens.len() && bracket_depth > 0 {
                            match tokens[i] {
                                Token::LeftBracket => bracket_depth += 1,
                                Token::RightBracket => bracket_depth -= 1,
                                _ => {}
                            }
                            i += 1;
                        }
                        continue;
                    }

                    // Only process simple objects
                    match self.resolve_token(&tokens[i], state) {
                        Ok(next_object) => {
                            self.process_enhanced_triple(
                                ontology,
                                subject.clone(),
                                predicate.clone(),
                                next_object,
                            )?;
                            i += 1;
                        }
                        Err(_) => {
                            // Skip problematic tokens instead of failing
                            i += 1;
                        }
                    }
                }
            }

            // Reset current predicate if we hit a semicolon or period
            if i < tokens.len() && matches!(tokens[i], Token::Semicolon | Token::Period) {
                current_predicate = None;
            }
        }

        Ok(())
    }

    /// Parse OWL disjoint union constructs
    fn parse_disjoint_union(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        // Extract the class that has the disjoint union
        let tokens = self.tokenize_statement(statement)?;

        // Find the subject (the class being defined)
        let subject = if let Some(first_token) = tokens.first() {
            self.resolve_token(first_token, state)?
        } else {
            return Err(Error::ontology_parsing(
                "No subject found in disjoint union statement",
            ));
        };

        // Extract the list of disjoint classes
        let list_start = statement
            .find('(')
            .ok_or_else(|| Error::ontology_parsing("No list found in disjoint union"))?;
        let list_end = statement
            .rfind(')')
            .ok_or_else(|| Error::ontology_parsing("Unclosed list in disjoint union"))?;
        let list_content = &statement[list_start + 1..list_end];

        let mut disjoint_classes = Vec::new();
        for class_name in list_content.split_whitespace() {
            let class_name = class_name.trim();
            if !class_name.is_empty() {
                let expanded_uri = self.expand_prefixed_name(class_name, state)?;
                let class = Class::new(IRI::new(&expanded_uri));
                disjoint_classes.push(ClassExpression::Class(class));
            }
        }

        if !disjoint_classes.is_empty() {
            // Create the class being defined
            let main_class = Class::new(IRI::new(&subject));

            // Add declaration for the main class
            let decl_axiom = DeclarationAxiom {
                id: generate_axiom_id(),
                entity: Entity::Class(IRI::new(&subject)),
            };
            ontology.add_axiom(Axiom::Declaration(decl_axiom));

            // Create disjoint union axiom
            let disjoint_union_axiom = DisjointUnionAxiom {
                id: generate_axiom_id(),
                class: ClassExpression::Class(main_class.clone()),
                disjoint_classes,
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::DisjointUnion(disjoint_union_axiom));

            println!("Created DisjointUnion axiom for class: {subject}");
        }

        Ok(())
    }

    /// Parse OWL equivalent class constructs
    fn parse_equivalent_class(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        // This would handle owl:equivalentClass statements
        // For now, we'll implement basic support
        let tokens = self.tokenize_statement(statement)?;

        if tokens.len() >= 3 {
            let subject = self.resolve_token(&tokens[0], state)?;
            let object = self.resolve_token(&tokens[2], state)?;

            let class1 = Class::new(IRI::new(&subject));
            let class2 = Class::new(IRI::new(&object));

            let equiv_axiom = EquivalentClassesAxiom {
                id: generate_axiom_id(),
                classes: vec![
                    ClassExpression::Class(class1),
                    ClassExpression::Class(class2),
                ],
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::EquivalentClasses(equiv_axiom));
        }

        Ok(())
    }

    /// Enhanced equivalent class parsing that handles complex class expressions
    fn parse_equivalent_class_enhanced(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        eprintln!("Parsing equivalent class statement: {}", statement.trim());

        // Extract the subject (class being defined)
        if let Some(subject_end) = statement.find("owl:equivalentClass") {
            let subject_part = &statement[..subject_end].trim();

            // Extract subject IRI
            let subject_iri = if subject_part.contains(' ') {
                // Handle cases like "ast:HealthyMoistStrategy rdf:type owl:Class ;"
                subject_part
                    .split_whitespace()
                    .next()
                    .unwrap_or(subject_part)
            } else {
                subject_part
            };

            // Convert to token and resolve
            let subject_token = if subject_iri.starts_with('<') && subject_iri.ends_with('>') {
                Token::IRI(subject_iri[1..subject_iri.len() - 1].to_string())
            } else if subject_iri.contains(':') {
                let parts: Vec<&str> = subject_iri.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    Token::PrefixedName("".to_string(), subject_iri.to_string())
                }
            } else {
                Token::PrefixedName("".to_string(), subject_iri.to_string())
            };

            let subject_uri = self.resolve_token(&subject_token, state)?;
            let subject_class = Class::new(IRI::new(&subject_uri));

            // Look for intersection patterns like "[ owl:intersectionOf ( ast:Healthy ast:MoistPlant ) ]"
            if statement.contains("owl:intersectionOf") {
                if let Some(start) = statement.find("owl:intersectionOf") {
                    if let Some(paren_start) = statement[start..].find('(') {
                        // Find matching closing parenthesis, handling nesting
                        let search_start = start + paren_start + 1;
                        let mut paren_depth = 1;
                        let mut paren_end_pos = None;

                        for (i, ch) in statement[search_start..].chars().enumerate() {
                            match ch {
                                '(' => paren_depth += 1,
                                ')' => {
                                    paren_depth -= 1;
                                    if paren_depth == 0 {
                                        paren_end_pos = Some(i);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let Some(paren_end) = paren_end_pos {
                            let classes_str = &statement[search_start..search_start + paren_end];

                            // Parse individual classes in the intersection
                            let mut intersection_classes = Vec::new();
                            let mut current_token = String::new();
                            let mut in_restriction = false;
                            let mut bracket_depth = 0;

                            for char in classes_str.chars() {
                                match char {
                                    '[' => {
                                        bracket_depth += 1;
                                        in_restriction = true;
                                        current_token.push(char);
                                    }
                                    ']' => {
                                        bracket_depth -= 1;
                                        current_token.push(char);
                                        if bracket_depth == 0 {
                                            in_restriction = false;
                                            // Parse the restriction if we accumulated one
                                            let restriction_content = current_token.trim();
                                            if !restriction_content.is_empty()
                                                && restriction_content.starts_with('[')
                                            {
                                                let class_expr = self.parse_restriction(
                                                    restriction_content,
                                                    state,
                                                )?;
                                                intersection_classes.push(class_expr);
                                            }
                                            current_token.clear();
                                        }
                                    }
                                    ' ' | '\t' | '\n' => {
                                        if in_restriction {
                                            // Keep whitespace when inside a restriction
                                            current_token.push(char);
                                        } else if !current_token.trim().is_empty() {
                                            let class_token_str = current_token.trim();
                                            if self.is_valid_class_reference(class_token_str) {
                                                // Convert to token and resolve
                                                let class_token = if class_token_str
                                                    .starts_with('<')
                                                    && class_token_str.ends_with('>')
                                                {
                                                    Token::IRI(
                                                        class_token_str
                                                            [1..class_token_str.len() - 1]
                                                            .to_string(),
                                                    )
                                                } else if class_token_str.contains(':') {
                                                    let parts: Vec<&str> =
                                                        class_token_str.splitn(2, ':').collect();
                                                    if parts.len() == 2 {
                                                        Token::PrefixedName(
                                                            parts[0].to_string(),
                                                            parts[1].to_string(),
                                                        )
                                                    } else {
                                                        Token::PrefixedName(
                                                            "".to_string(),
                                                            class_token_str.to_string(),
                                                        )
                                                    }
                                                } else {
                                                    Token::PrefixedName(
                                                        "".to_string(),
                                                        class_token_str.to_string(),
                                                    )
                                                };

                                                match self.resolve_token(&class_token, state) {
                                                    Ok(class_uri) => {
                                                        let class =
                                                            Class::new(IRI::new(&class_uri));
                                                        intersection_classes
                                                            .push(ClassExpression::Class(class));
                                                        eprintln!(
                                                            "Added intersection class: {}",
                                                            class_uri
                                                        );
                                                    }
                                                    Err(e) => {
                                                        eprintln!(
                                                            "Warning: Could not resolve intersection class {}: {}",
                                                            class_token_str, e
                                                        );
                                                    }
                                                }
                                            }
                                            current_token.clear();
                                        } else {
                                            current_token.push(char);
                                        }
                                    }
                                    _ => {
                                        current_token.push(char);
                                    }
                                }
                            }

                            // Handle the last token
                            if !in_restriction && !current_token.trim().is_empty() {
                                let class_token_str = current_token.trim();
                                if self.is_valid_class_reference(class_token_str) {
                                    // Convert to token and resolve - same logic as above
                                    let class_token = if class_token_str.starts_with('<')
                                        && class_token_str.ends_with('>')
                                    {
                                        Token::IRI(
                                            class_token_str[1..class_token_str.len() - 1]
                                                .to_string(),
                                        )
                                    } else if class_token_str.contains(':') {
                                        let parts: Vec<&str> =
                                            class_token_str.splitn(2, ':').collect();
                                        if parts.len() == 2 {
                                            Token::PrefixedName(
                                                parts[0].to_string(),
                                                parts[1].to_string(),
                                            )
                                        } else {
                                            Token::PrefixedName(
                                                "".to_string(),
                                                class_token_str.to_string(),
                                            )
                                        }
                                    } else {
                                        Token::PrefixedName(
                                            "".to_string(),
                                            class_token_str.to_string(),
                                        )
                                    };

                                    match self.resolve_token(&class_token, state) {
                                        Ok(class_uri) => {
                                            let class = Class::new(IRI::new(&class_uri));
                                            intersection_classes
                                                .push(ClassExpression::Class(class));
                                            eprintln!("Added intersection class: {}", class_uri);
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Warning: Could not resolve intersection class {}: {}",
                                                class_token_str, e
                                            );
                                        }
                                    }
                                }
                            }

                            if !intersection_classes.is_empty() {
                                // Create equivalent classes axiom with intersection
                                let intersection_expr =
                                    ClassExpression::ObjectIntersectionOf(intersection_classes);

                                let equiv_axiom = EquivalentClassesAxiom {
                                    id: generate_axiom_id(),
                                    classes: vec![
                                        ClassExpression::Class(subject_class),
                                        intersection_expr,
                                    ],
                                    annotations: vec![],
                                };

                                eprintln!("Creating EquivalentClasses axiom for intersection");
                                ontology.add_axiom(Axiom::EquivalentClasses(equiv_axiom));
                                return Ok(());
                            }
                        }
                    }
                }
            }

            // Fallback to simple equivalent class handling
            return self.parse_equivalent_class(statement, ontology, state);
        }

        Ok(())
    }

    /// Check if a token is a valid class reference (not OWL keywords or literals)
    fn is_valid_class_reference(&self, token: &str) -> bool {
        !token.is_empty()
            && !token.starts_with('[')
            && !token.starts_with('"')
            && !token.starts_with("rdf:")
            && !token.starts_with("owl:")
            && !token.starts_with("rdfs:")
            && !token.starts_with("xsd:")
            && token != ";"
            && token != ","
            && token != "("
            && token != ")"
            && token != "."
    }

    /// Parse an OWL restriction from a blank node string like "[ rdf:type owl:Restriction ; owl:onProperty ... ]"
    fn parse_restriction(
        &self,
        restriction_str: &str,
        state: &mut ParseState,
    ) -> Result<ClassExpression> {
        // Extract property name
        let property_name = if let Some(prop_start) = restriction_str.find("owl:onProperty") {
            let after_prop = &restriction_str[prop_start + 14..].trim_start();
            after_prop
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(';')
                .trim()
        } else {
            return Err(Error::ontology_parsing(
                "No owl:onProperty found in restriction",
            ));
        };

        // Check if it's owl:someValuesFrom or owl:hasValue
        if restriction_str.contains("owl:someValuesFrom") {
            // Data property restriction with datatype

            // Resolve property
            let property_token = if property_name.contains(':') {
                let parts: Vec<&str> = property_name.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid property name: {}",
                        property_name
                    )));
                }
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid property name: {}",
                    property_name
                )));
            };

            let property_iri = self.resolve_token(&property_token, state)?;
            let data_property = DataProperty {
                iri: IRI::new(&property_iri),
            };

            // Parse the datatype restriction
            let data_range = self.parse_datatype_restriction(restriction_str, state)?;

            Ok(ClassExpression::DataSomeValuesFrom {
                property: DataPropertyExpression::DataProperty(data_property),
                filler: data_range,
            })
        } else if restriction_str.contains("owl:hasValue") {
            // Data property with specific value

            // Resolve property
            let property_token = if property_name.contains(':') {
                let parts: Vec<&str> = property_name.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid property name: {}",
                        property_name
                    )));
                }
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid property name: {}",
                    property_name
                )));
            };

            let property_iri = self.resolve_token(&property_token, state)?;
            let data_property = DataProperty {
                iri: IRI::new(&property_iri),
            };

            // Extract the value - look for quoted string or unquoted value
            let value_str = if let Some(val_start) = restriction_str.find("owl:hasValue") {
                let after_val = &restriction_str[val_start + 12..].trim_start();

                // Check if it's a quoted string
                if let Some(quote_start) = after_val.find('"') {
                    // Find the closing quote
                    let after_quote = &after_val[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        after_quote[..quote_end].to_string()
                    } else {
                        // No closing quote found, take until semicolon or bracket
                        after_val
                            .split(&[';', ']'][..])
                            .next()
                            .unwrap_or("")
                            .trim()
                            .trim_matches('"')
                            .to_string()
                    }
                } else {
                    // Unquoted value - take until semicolon or bracket
                    after_val
                        .split(&[';', ']'][..])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                }
            } else {
                return Err(Error::ontology_parsing("No owl:hasValue found"));
            };

            let literal = Literal {
                value: value_str,
                datatype: None,
                language: None,
            };

            Ok(ClassExpression::DataHasValue {
                property: DataPropertyExpression::DataProperty(data_property),
                value: literal,
            })
        } else if restriction_str.contains("owl:minQualifiedCardinality")
            || restriction_str.contains("owl:maxQualifiedCardinality")
            || restriction_str.contains("owl:qualifiedCardinality")
        {
            // Qualified cardinality restrictions with owl:onClass

            // Resolve property - this is an object property
            let property_token = if property_name.contains(':') {
                let parts: Vec<&str> = property_name.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid property name: {}",
                        property_name
                    )));
                }
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid property name: {}",
                    property_name
                )));
            };

            let property_iri = self.resolve_token(&property_token, state)?;
            let object_property = ObjectProperty {
                iri: IRI::new(&property_iri),
            };

            // Extract cardinality value
            let cardinality_str = if let Some(card_start) = restriction_str
                .find("owl:minQualifiedCardinality")
                .or_else(|| restriction_str.find("owl:maxQualifiedCardinality"))
                .or_else(|| restriction_str.find("owl:qualifiedCardinality"))
            {
                let after_card = &restriction_str[card_start..];
                let after_prop_name = &after_card[after_card
                    .find("Cardinality")
                    .expect("Failed to find Cardinality keyword in restriction")
                    + 11..]
                    .trim_start();

                // Extract the literal value (could be "2"^^xsd:nonNegativeInteger or just "2")
                if let Some(quote_start) = after_prop_name.find('"') {
                    let after_quote = &after_prop_name[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        after_quote[..quote_end].to_string()
                    } else {
                        return Err(Error::ontology_parsing("Invalid cardinality value format"));
                    }
                } else {
                    // Unquoted number
                    after_prop_name
                        .split_whitespace()
                        .next()
                        .unwrap_or("0")
                        .trim_end_matches(';')
                        .to_string()
                }
            } else {
                return Err(Error::ontology_parsing("No cardinality property found"));
            };

            let cardinality: u32 = cardinality_str.parse().map_err(|_| {
                Error::ontology_parsing(format!("Invalid cardinality value: {}", cardinality_str))
            })?;

            // Extract the onClass (filler)
            let class_name = if let Some(class_start) = restriction_str.find("owl:onClass") {
                let after_class = &restriction_str[class_start + 11..].trim_start();
                after_class
                    .split(&[';', ']', ' ', '\n', '\t'][..])
                    .next()
                    .unwrap_or("")
                    .trim()
            } else {
                // If no onClass is specified, use owl:Thing as default
                "owl:Thing"
            };

            // Resolve the class IRI
            let class_token = if class_name.contains(':') {
                let parts: Vec<&str> = class_name.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    Token::PrefixedName("owl".to_string(), "Thing".to_string())
                }
            } else if class_name.starts_with('<') && class_name.ends_with('>') {
                Token::IRI(class_name[1..class_name.len() - 1].to_string())
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid class name: {}",
                    class_name
                )));
            };

            let class_iri = self.resolve_token(&class_token, state)?;
            let filler = Box::new(ClassExpression::Class(Class {
                iri: IRI::new(&class_iri),
            }));

            // Determine which type of cardinality restriction
            if restriction_str.contains("owl:minQualifiedCardinality") {
                Ok(ClassExpression::ObjectMinCardinality {
                    property: ObjectPropertyExpression::ObjectProperty(object_property),
                    cardinality,
                    filler,
                })
            } else if restriction_str.contains("owl:maxQualifiedCardinality") {
                Ok(ClassExpression::ObjectMaxCardinality {
                    property: ObjectPropertyExpression::ObjectProperty(object_property),
                    cardinality,
                    filler,
                })
            } else {
                // owl:qualifiedCardinality
                Ok(ClassExpression::ObjectExactCardinality {
                    property: ObjectPropertyExpression::ObjectProperty(object_property),
                    cardinality,
                    filler,
                })
            }
        } else if restriction_str.contains("owl:hasSelf") {
            // ObjectHasSelf restriction

            // Validate that hasSelf has proper boolean value
            if let Some(has_self_start) = restriction_str.find("owl:hasSelf") {
                let after_has_self = &restriction_str[has_self_start + 11..].trim_start();

                // Extract the value
                let value_str = if let Some(quote_start) = after_has_self.find('"') {
                    let after_quote = &after_has_self[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        after_quote[..quote_end].to_string()
                    } else {
                        return Err(Error::ontology_parsing("Invalid owl:hasSelf value format"));
                    }
                } else {
                    return Err(Error::ontology_parsing(
                        "owl:hasSelf must have a literal boolean value",
                    ));
                };

                // Validate it's "true" or "false"
                if value_str != "true" && value_str != "false" {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid owl:hasSelf value: '{}'. Must be 'true' or 'false'",
                        value_str
                    )));
                }

                // Check for datatype annotation
                if after_has_self.contains("^^") {
                    let after_quotes = &after_has_self[after_has_self
                        .find('"')
                        .expect("Failed to find quote in hasSelf restriction")..];
                    if let Some(quote_end) = after_quotes[1..].find('"') {
                        let after_closing_quote = &after_quotes[quote_end + 2..];
                        if after_closing_quote.trim_start().starts_with("^^") {
                            let datatype_part = after_closing_quote.trim_start()[2..]
                                .split(&[';', ']', ' '][..])
                                .next()
                                .unwrap_or("");
                            // Validate it's xsd:boolean
                            if datatype_part != "xsd:boolean"
                                && !datatype_part.contains("XMLSchema#boolean")
                            {
                                return Err(Error::ontology_parsing(format!(
                                    "Invalid datatype for owl:hasSelf: '{}'. Must be xsd:boolean",
                                    datatype_part
                                )));
                            }
                        }
                    }
                }
            }

            // Resolve property
            let property_token = if property_name.contains(':') {
                let parts: Vec<&str> = property_name.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                } else {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid property name: {}",
                        property_name
                    )));
                }
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid property name: {}",
                    property_name
                )));
            };

            let property_iri = self.resolve_token(&property_token, state)?;
            let object_property = ObjectProperty {
                iri: IRI::new(&property_iri),
            };

            Ok(ClassExpression::ObjectHasSelf {
                property: ObjectPropertyExpression::ObjectProperty(object_property),
            })
        } else {
            Err(Error::ontology_parsing("Unsupported restriction type"))
        }
    }

    /// Parse a datatype restriction like "[ rdf:type rdfs:Datatype ; owl:onDatatype xsd:double ; owl:withRestrictions ( ... ) ]"
    fn parse_datatype_restriction(
        &self,
        restriction_str: &str,
        state: &mut ParseState,
    ) -> Result<DataRange> {
        // Extract base datatype
        let datatype_name = if let Some(dt_start) = restriction_str.find("owl:onDatatype") {
            let after_dt = &restriction_str[dt_start + 14..].trim_start();
            after_dt
                .split_whitespace()
                .next()
                .unwrap_or("xsd:string")
                .trim_end_matches(';')
                .trim()
        } else {
            "xsd:string"
        };

        // Resolve datatype IRI
        let datatype_token = if datatype_name.contains(':') {
            let parts: Vec<&str> = datatype_name.splitn(2, ':').collect();
            if parts.len() == 2 {
                Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
            } else {
                Token::PrefixedName("xsd".to_string(), "string".to_string())
            }
        } else {
            Token::PrefixedName("xsd".to_string(), "string".to_string())
        };

        let datatype_iri = self.resolve_token(&datatype_token, state)?;

        // Parse facet restrictions
        let mut facets = Vec::new();

        if let Some(restr_start) = restriction_str.find("owl:withRestrictions") {
            if let Some(paren_start) = restriction_str[restr_start..].find('(') {
                if let Some(paren_end) = restriction_str[restr_start + paren_start..].find(')') {
                    let facets_str = &restriction_str
                        [restr_start + paren_start + 1..restr_start + paren_start + paren_end];
                    eprintln!("Parsing facets: {}", facets_str);

                    // Parse facets like "[ xsd:maxExclusive \"80.0\"^^xsd:double ]"
                    let mut in_bracket = false;
                    let mut current_facet = String::new();

                    for ch in facets_str.chars() {
                        match ch {
                            '[' => {
                                in_bracket = true;
                                current_facet.clear();
                            }
                            ']' => {
                                if in_bracket {
                                    // Parse the facet
                                    if let Ok(facet) = self.parse_facet(&current_facet, state) {
                                        facets.push(facet);
                                    }
                                    in_bracket = false;
                                }
                            }
                            _ => {
                                if in_bracket {
                                    current_facet.push(ch);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(DataRange::DatatypeRestriction {
            datatype: IRI::new(&datatype_iri),
            restrictions: facets,
        })
    }

    /// Parse a facet restriction like "xsd:maxExclusive \"80.0\"^^xsd:double"
    fn parse_facet(&self, facet_str: &str, state: &mut ParseState) -> Result<FacetRestriction> {
        let parts: Vec<&str> = facet_str.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(Error::ontology_parsing("Invalid facet format"));
        }

        // Resolve facet IRI
        let facet_token = if parts[0].contains(':') {
            let p: Vec<&str> = parts[0].splitn(2, ':').collect();
            if p.len() == 2 {
                Token::PrefixedName(p[0].to_string(), p[1].to_string())
            } else {
                return Err(Error::ontology_parsing(format!(
                    "Invalid facet name: {}",
                    parts[0]
                )));
            }
        } else {
            return Err(Error::ontology_parsing(format!(
                "Invalid facet name: {}",
                parts[0]
            )));
        };

        let facet_iri = self.resolve_token(&facet_token, state)?;

        // Extract value (remove quotes and datatype annotation)
        // The value might be like "80.0"^^xsd:double or just "80.0"
        let value_part = parts[1];
        let value_str = value_part
            .split("^^")
            .next()
            .unwrap_or(value_part)
            .trim_matches('"')
            .trim_matches('\\') // Remove escape characters if present
            .to_string();

        let literal = Literal {
            value: value_str,
            datatype: None,
            language: None,
        };

        Ok(FacetRestriction {
            facet: IRI::new(&facet_iri),
            value: literal,
        })
    }

    /// Enhanced tokenization
    /// Extract and process rdfs:subClassOf relationships from complex statements
    fn extract_and_process_subclass_from_statement(
        &self,
        statement: &str,
        ontology: &mut Ontology,
        state: &mut ParseState,
    ) -> Result<()> {
        // Look for rdfs:subClassOf pattern in the statement
        if let Some(subclass_start) = statement.find("rdfs:subClassOf") {
            // Extract the subject (everything before the first predicate)
            let subject_part = if let Some(type_pos) = statement.find("rdf:type") {
                &statement[..type_pos].trim()
            } else {
                return Ok(()); // Can't determine subject
            };

            let subject_iri = subject_part.split_whitespace().next().unwrap_or("").trim();

            // Extract the object (everything after rdfs:subClassOf until end or next predicate)
            let after_subclass = &statement[subclass_start + 15..]; // Skip "rdfs:subClassOf"
            let object_part = after_subclass
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();

            // Clean up object (remove trailing punctuation)
            let object_iri = object_part
                .trim_end_matches('.')
                .trim_end_matches(';')
                .trim();

            if !subject_iri.is_empty() && !object_iri.is_empty() {
                // Resolve tokens
                let subject_token = if subject_iri.contains(':') {
                    let parts: Vec<&str> = subject_iri.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                };

                let object_token = if object_iri.contains(':') {
                    let parts: Vec<&str> = object_iri.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        Token::PrefixedName(parts[0].to_string(), parts[1].to_string())
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                };

                // Resolve to URIs
                let subject_uri = self.resolve_token(&subject_token, state)?;
                let object_uri = self.resolve_token(&object_token, state)?;

                // Create SubClassOf axiom
                let subclass = ClassExpression::Class(Class {
                    iri: IRI::new(&subject_uri),
                });
                let superclass = ClassExpression::Class(Class {
                    iri: IRI::new(&object_uri),
                });

                let axiom = SubClassOfAxiom {
                    id: generate_axiom_id(),
                    subclass,
                    superclass,
                    annotations: vec![],
                };

                println!(
                    "Creating enhanced SubClassOf axiom: {} rdfs:subClassOf {}",
                    subject_uri, object_uri
                );
                ontology.add_axiom(Axiom::SubClassOf(axiom));
            }
        }

        Ok(())
    }

    fn tokenize_statement(&self, statement: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_iri = false;
        let mut in_literal = false;

        let chars: Vec<char> = statement.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch {
                '<' if !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    in_iri = true;
                    current_token.push(ch);
                }
                '>' if in_iri => {
                    current_token.push(ch);
                    tokens.push(Token::IRI(
                        current_token[1..current_token.len() - 1].to_string(),
                    ));
                    current_token.clear();
                    in_iri = false;
                }
                '"' => {
                    if in_literal {
                        current_token.push(ch);
                        tokens.push(Token::Literal(
                            current_token[1..current_token.len() - 1].to_string(),
                        ));
                        current_token.clear();
                        in_literal = false;

                        // Check if next is ^^ for datatype annotation
                        if i + 2 < chars.len() && chars[i + 1] == '^' && chars[i + 2] == '^' {
                            i += 2; // Skip the ^^
                            let mut datatype_token = String::from("^^");
                            i += 1;

                            // Read the datatype (could be <IRI> or prefix:local)
                            if i < chars.len() && chars[i] == '<' {
                                // IRI datatype
                                datatype_token.push('<');
                                i += 1;
                                while i < chars.len() && chars[i] != '>' {
                                    datatype_token.push(chars[i]);
                                    i += 1;
                                }
                                if i < chars.len() {
                                    datatype_token.push('>');
                                }
                            } else {
                                // Prefixed name datatype - read until whitespace or delimiter
                                while i < chars.len() {
                                    let ch = chars[i];
                                    if ch.is_whitespace()
                                        || ch == ','
                                        || ch == ';'
                                        || ch == '.'
                                        || ch == ')'
                                        || ch == ']'
                                    {
                                        break;
                                    }
                                    datatype_token.push(ch);
                                    i += 1;
                                }
                                i -= 1; // Back up one since we'll increment at end of loop
                            }

                            tokens.push(Token::Keyword(datatype_token));
                        }
                    } else {
                        if !current_token.is_empty() {
                            self.add_token_from_string(&current_token, &mut tokens);
                            current_token.clear();
                        }
                        in_literal = true;
                        current_token.push(ch);
                    }
                }
                '(' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::LeftParen);
                }
                ')' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::RightParen);
                }
                '[' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::LeftBracket);
                }
                ']' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::RightBracket);
                }
                ',' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::Comma);
                }
                ';' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::Semicolon);
                }
                '.' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                    tokens.push(Token::Period);
                }
                ' ' | '\t' | '\n' | '\r' if !in_iri && !in_literal => {
                    if !current_token.is_empty() {
                        self.add_token_from_string(&current_token, &mut tokens);
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
            i += 1;
        }

        if !current_token.is_empty() {
            self.add_token_from_string(&current_token, &mut tokens);
        }

        Ok(tokens)
    }

    /// Helper to add token from string
    fn add_token_from_string(&self, token_str: &str, tokens: &mut Vec<Token>) {
        // Check for blank node labels (e.g., _:b0)
        // Blank nodes MUST have the format _:label (with colon)
        if token_str.starts_with("_:") {
            tokens.push(Token::BlankNode(token_str.to_string()));
        } else if token_str.starts_with('_') && !token_str.contains(':') {
            // Invalid blank node format (missing colon) - treat as keyword to trigger error
            tokens.push(Token::Keyword(token_str.to_string()));
        } else if token_str.contains(':') && !token_str.starts_with("http") {
            if let Some(colon_pos) = token_str.find(':') {
                let prefix = token_str[..colon_pos].to_string();
                let local = token_str[colon_pos + 1..].to_string();
                tokens.push(Token::PrefixedName(prefix, local));
            } else {
                tokens.push(Token::Keyword(token_str.to_string()));
            }
        } else {
            tokens.push(Token::Keyword(token_str.to_string()));
        }
    }

    /// Resolve token to URI string
    fn resolve_token(&self, token: &Token, state: &ParseState) -> Result<String> {
        match token {
            Token::IRI(iri) => Ok(self.decode_escape_sequences(iri)),
            Token::PrefixedName(prefix, local) => {
                self.expand_prefixed_name(&format!("{prefix}:{local}"), state)
            }
            Token::Keyword(keyword) => {
                // Don't expand datatype annotations (^^xsd:type)
                if keyword.starts_with("^^") {
                    Ok(keyword.clone())
                } else if keyword.contains(':') {
                    // Try to expand as prefixed name if it contains ':'
                    self.expand_prefixed_name(keyword, state)
                } else {
                    Ok(keyword.clone())
                }
            }
            Token::BlankNode(id) => {
                // Handle blank nodes properly
                if id.starts_with("_:") {
                    Ok(id.clone())
                } else {
                    Ok(format!("_:{id}"))
                }
            }
            Token::Literal(lit) => {
                // Decode escape sequences in literals
                Ok(self.decode_escape_sequences(lit))
            }
            _ => {
                let token_desc = format!("{:?}", token);
                Err(Error::ontology_parsing(format!(
                    "Cannot resolve token to URI: {:?}",
                    token_desc
                )))
            }
        }
    }

    /// Decode escape sequences in strings
    fn decode_escape_sequences(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        'n' => {
                            result.push('\n');
                            chars.next();
                        }
                        't' => {
                            result.push('\t');
                            chars.next();
                        }
                        'r' => {
                            result.push('\r');
                            chars.next();
                        }
                        '\\' => {
                            result.push('\\');
                            chars.next();
                        }
                        '"' => {
                            result.push('"');
                            chars.next();
                        }
                        'u' => {
                            // Unicode escape \uXXXX
                            chars.next(); // consume 'u'
                            let hex: String = chars.by_ref().take(4).collect();
                            if hex.len() == 4 {
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(unicode_char) = char::from_u32(code) {
                                        result.push(unicode_char);
                                        continue;
                                    }
                                }
                            }
                            // If parsing failed, keep the escape sequence
                            result.push('\\');
                            result.push('u');
                            result.push_str(&hex);
                        }
                        'U' => {
                            // Unicode escape \UXXXXXXXX
                            chars.next(); // consume 'U'
                            let hex: String = chars.by_ref().take(8).collect();
                            if hex.len() == 8 {
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(unicode_char) = char::from_u32(code) {
                                        result.push(unicode_char);
                                        continue;
                                    }
                                }
                            }
                            // If parsing failed, keep the escape sequence
                            result.push('\\');
                            result.push('U');
                            result.push_str(&hex);
                        }
                        _ => {
                            // Unknown escape, keep as-is
                            result.push(ch);
                            result.push(next_ch);
                            chars.next();
                        }
                    }
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Enhanced URI expansion with proper prefix handling
    fn expand_prefixed_name(&self, name: &str, state: &ParseState) -> Result<String> {
        if name.starts_with('<') && name.ends_with('>') {
            let result = name[1..name.len() - 1].to_string();
            return Ok(result);
        }

        if let Some(colon_pos) = name.find(':') {
            let prefix = &name[..colon_pos];
            let local = &name[colon_pos + 1..];

            if let Some(base) = state.prefixes.get(prefix) {
                let result = format!("{base}{local}");
                Ok(result)
            } else {
                // Handle unknown prefixes - this should be an error for proper Turtle parsing
                return Err(Error::ontology_parsing(format!("Undefined prefix: {}", prefix)));
            }
        } else if let Some(base) = &state.base_uri {
            let result = format!("{base}{name}");
            Ok(result)
        } else {
            // Relative URI without base - this should be an error
            Err(Error::ontology_parsing(format!("Relative URI without base: {}", name)))
        }
    }

    /// Enhanced triple processing with better OWL support
    fn process_enhanced_triple(
        &self,
        ontology: &mut Ontology,
        subject: String,
        predicate: String,
        object: String,
    ) -> Result<()> {
        match predicate.as_str() {
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" => {
                match object.as_str() {
                    "http://www.w3.org/2002/07/owl#Class" => {
                        // Class declaration
                        let class = Class::new(IRI::new(&subject));
                        let decl_axiom = DeclarationAxiom {
                            id: generate_axiom_id(),
                            entity: Entity::Class(IRI::new(&subject)),
                        };
                        ontology.add_axiom(Axiom::Declaration(decl_axiom));
                    }
                    "http://www.w3.org/2002/07/owl#ObjectProperty" => {
                        // Object property declaration
                        let property = ObjectProperty::new(IRI::new(&subject));
                        let decl_axiom = DeclarationAxiom {
                            id: generate_axiom_id(),
                            entity: Entity::ObjectProperty(IRI::new(&subject)),
                        };
                        ontology.add_axiom(Axiom::Declaration(decl_axiom));
                    }
                    _ => {
                        // Class assertion
                        let individual = Individual::Named(NamedIndividual {
                            iri: IRI::new(&subject),
                        });
                        let class = Class::new(IRI::new(&object));

                        let axiom = ClassAssertionAxiom {
                            id: generate_axiom_id(),
                            individual,
                            class: ClassExpression::Class(class),
                            annotations: vec![],
                        };
                        ontology.add_axiom(Axiom::ClassAssertion(axiom));
                    }
                }
            }
            "http://www.w3.org/2000/01/rdf-schema#subClassOf" => {
                let subclass = Class::new(IRI::new(&subject));
                let superclass = Class::new(IRI::new(&object));

                let axiom = SubClassOfAxiom {
                    id: generate_axiom_id(),
                    subclass: ClassExpression::Class(subclass),
                    superclass: ClassExpression::Class(superclass),
                    annotations: vec![],
                };
                println!("Creating enhanced SubClassOf axiom: {subject} rdfs:subClassOf {object}");
                ontology.add_axiom(Axiom::SubClassOf(axiom));
            }
            "http://www.w3.org/2002/07/owl#hasKey" => {
                // owl:hasKey requires list syntax: owl:hasKey ( properties )
                // If object is not a list (rdf:first/rest), it's invalid
                if !object.starts_with("_:list") && !object.contains("22-rdf-syntax-ns#first") {
                    return Err(Error::ontology_parsing(
                        "owl:hasKey requires list syntax: owl:hasKey ( property1 property2 ... )",
                    ));
                }
                // For now, just validate - full hasKey axiom support would go here
            }
            "http://www.w3.org/2002/07/owl#propertyChainAxiom" => {
                // owl:propertyChainAxiom requires list syntax: owl:propertyChainAxiom ( prop1 prop2 ... )
                // If object is not a list (rdf:first/rest), it's invalid
                if !object.starts_with("_:list") && !object.contains("22-rdf-syntax-ns#first") {
                    return Err(Error::ontology_parsing(
                        "owl:propertyChainAxiom requires list syntax: owl:propertyChainAxiom ( property1 property2 ... )",
                    ));
                }
                // For now, just validate - full property chain axiom support would go here
            }
            _ => {
                // Detect if object is a literal (data property) or an individual (object property)
                let is_literal = object.starts_with('"') || object.contains("^^");

                if is_literal {
                    // DataPropertyAssertion
                    let subject_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(&subject),
                    });
                    let data_property = DataProperty {
                        iri: IRI::new(&predicate),
                    };

                    // Parse literal value and datatype
                    let (value, datatype) = if let Some(idx) = object.find("^^") {
                        let val = object[..idx].trim_matches('"').to_string();
                        let dt_str = object[idx + 2..].to_string();
                        let dt_iri = IRI::new(&dt_str);
                        let dt_url = dt_iri.to_url().ok();
                        (val, dt_url)
                    } else {
                        (object.trim_matches('"').to_string(), None)
                    };

                    let literal = Literal {
                        value,
                        language: None,
                        datatype,
                    };

                    let axiom = DataPropertyAssertionAxiom {
                        id: generate_axiom_id(),
                        property: crate::ontology::DataPropertyExpression::DataProperty(
                            data_property,
                        ),
                        individual: subject_ind,
                        value: literal,
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::DataPropertyAssertion(axiom));
                } else {
                    // ObjectPropertyAssertion
                    let subject_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(&subject),
                    });
                    let object_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(&object),
                    });
                    let property = ObjectProperty::new(IRI::new(&predicate))?;

                    let axiom = ObjectPropertyAssertionAxiom {
                        id: generate_axiom_id(),
                        property: crate::ontology::ObjectPropertyExpression::ObjectProperty(
                            property,
                        ),
                        source: subject_ind,
                        target: object_ind,
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::ObjectPropertyAssertion(axiom));
                }
            }
        }

        Ok(())
    }
}

/// Represents a parsed RDF triple
#[derive(Debug, Clone)]
struct Triple {
    subject: String,
    predicate: String,
    object: TripleObject,
}

/// Object part of an RDF triple
#[derive(Debug, Clone)]
enum TripleObject {
    Uri(String),
    Literal(String),
    BlankNode(String),
    List(Vec<TripleObject>),
}

/// Parse from file with enhanced support
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let parser = TurtleParser::new();
    parser.parse_string(&content)
}

/// Parse from file with custom configuration
pub fn parse_file_with_config<P: AsRef<Path>>(
    path: P,
    config: TurtleParserConfig,
) -> Result<Ontology> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let parser = TurtleParser::with_config(config);
    parser.parse_string(&content)
}

/// Turtle format serializer implementing the common serialization interface
#[derive(Debug, Clone, Default)]
pub struct TurtleSerializer;

impl TurtleSerializer {
    /// Create a new TurtleSerializer instance
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl OntologySerializer for TurtleSerializer {
    fn serialize(&self, ontology: &Ontology) -> std::result::Result<String, Error> {
        let mut content = String::new();

        // Add standard prefixes
        content.push_str("@prefix : <http://example.org/> .\n");
        content.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
        content.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        content.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        content.push('\n');

        // Write ontology declaration
        if let Some(iri) = ontology.get_iri() {
            content.push_str(&format!("<{iri}> rdf:type owl:Ontology .\n\n"));
        }

        // Write class declarations
        for (iri, _class) in ontology.classes() {
            content.push_str(&format!("<{iri}> rdf:type owl:Class .\n"));
        }
        content.push('\n');

        // Write object property declarations
        for prop in ontology.object_properties() {
            content.push_str(&format!("<{}> rdf:type owl:ObjectProperty .\n", prop.iri));
        }
        content.push('\n');

        // Write axioms (basic serialization)
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::SubClassOf(sub) => {
                    if let (ClassExpression::Class(subclass), ClassExpression::Class(superclass)) =
                        (&sub.subclass, &sub.superclass)
                    {
                        content.push_str(&format!(
                            "<{}> rdfs:subClassOf <{}> .\n",
                            subclass.iri, superclass.iri
                        ));
                    }
                }
                crate::ontology::Axiom::ClassAssertion(assertion) => {
                    if let ClassExpression::Class(class) = &assertion.class {
                        if let Some(individual_iri) = assertion.individual.iri() {
                            content.push_str(&format!(
                                "<{}> rdf:type <{}> .\n",
                                individual_iri, class.iri
                            ));
                        }
                    }
                }
                crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                    if let crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) =
                        &assertion.property
                    {
                        if let (Some(source_iri), Some(target_iri)) =
                            (assertion.source.iri(), assertion.target.iri())
                        {
                            content.push_str(&format!(
                                "<{}> <{}> <{}> .\n",
                                source_iri, prop.iri, target_iri
                            ));
                        }
                    }
                }
                _ => {
                    // Skip complex axioms for now
                }
            }
        }

        Ok(content)
    }
}

/// Save ontology to Turtle file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = TurtleSerializer::new();
    serializer.serialize_to_file(ontology, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_disjoint_union_parsing() {
        let content = r"
@prefix ast: <http://www.smolang.org/greenhouseDT#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

ast:Pump rdf:type owl:Class ;
         owl:disjointUnionOf ( ast:Maintenance
                               ast:Operational
                               ast:Overheating
                               ast:Underheating
                             ) .
";

        let parser = TurtleParser::new();
        let result = parser.parse_string(content);

        if let Err(err) = &result {
            println!("Parse error: {:?}", err);
        }
        assert!(result.is_ok(), "Enhanced parsing should succeed");

        let ontology = result.expect("Failed to parse ontology from test input");

        // Check that disjoint union axiom was created
        let has_disjoint_union = ontology
            .axioms()
            .iter()
            .any(|axiom| matches!(axiom, crate::ontology::axioms::Axiom::DisjointUnion(_)));

        assert!(
            has_disjoint_union,
            "Should have created disjoint union axiom"
        );
    }

    #[test]
    fn test_enhanced_subclass_parsing() {
        let content = r"
@prefix ast: <http://www.smolang.org/greenhouseDT#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

ast:Maintenance rdfs:subClassOf ast:Pump .
ast:Operational rdfs:subClassOf ast:Pump .
";

        let parser = TurtleParser::new();
        let result = parser.parse_string(content);

        if let Err(err) = &result {
            println!("Parse error: {:?}", err);
        }
        assert!(result.is_ok(), "Enhanced parsing should succeed");

        let ontology = result.expect("Failed to parse ontology from test input");

        // Check that SubClassOf axioms were created
        let subclass_count = ontology
            .axioms()
            .iter()
            .filter(|axiom| matches!(axiom, crate::ontology::axioms::Axiom::SubClassOf(_)))
            .count();

        assert!(
            subclass_count >= 2,
            "Should have created at least 2 SubClassOf axioms, found: {}",
            subclass_count
        );
    }
}
