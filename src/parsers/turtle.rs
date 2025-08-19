//! Turtle Parser
//!
//! This module implements comprehensive parsing of OWL 2 ontologies from Turtle format.
//! It handles complex turtle syntax including disjoint unions, lists, blank nodes, and multi-line statements.

use crate::{
    Error, Result,
    ontology::{
        Class, ClassExpression, IRI, Individual, NamedIndividual, ObjectProperty, Ontology,
        axioms::{
            Axiom, ClassAssertionAxiom, DeclarationAxiom, DisjointUnionAxiom, Entity,
            EquivalentClassesAxiom, ObjectPropertyAssertionAxiom, SubClassOfAxiom,
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

        Ok(ontology)
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

        if statement.contains("owl:equivalentClass") {
            return self.parse_equivalent_class(statement, ontology, state);
        }

        // Parse standard triple
        let tokens = self.tokenize_statement(statement)?;
        if tokens.len() >= 3 {
            let subject = self.resolve_token(&tokens[0], state)?;
            let predicate = self.resolve_token(&tokens[1], state)?;
            let object = self.resolve_token(&tokens[2], state)?;

            self.process_enhanced_triple(ontology, subject, predicate, object)?;
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

    /// Enhanced tokenization
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
        if token_str.contains(':') && !token_str.starts_with("http") {
            if let Some(colon_pos) = token_str.find(':') {
                let prefix = token_str[..colon_pos].to_string();
                let local = token_str[colon_pos + 1..].to_string();
                tokens.push(Token::PrefixedName(prefix, local));
            } else {
                tokens.push(Token::Keyword(token_str.to_string()));
            }
        } else if token_str.starts_with('_') {
            tokens.push(Token::BlankNode(token_str.to_string()));
        } else {
            tokens.push(Token::Keyword(token_str.to_string()));
        }
    }

    /// Resolve token to URI string
    fn resolve_token(&self, token: &Token, state: &ParseState) -> Result<String> {
        match token {
            Token::IRI(iri) => Ok(iri.clone()),
            Token::PrefixedName(prefix, local) => {
                self.expand_prefixed_name(&format!("{prefix}:{local}"), state)
            }
            Token::Keyword(keyword) => {
                // Try to expand as prefixed name if it contains ':'
                if keyword.contains(':') {
                    self.expand_prefixed_name(keyword, state)
                } else {
                    Ok(keyword.clone())
                }
            }
            Token::BlankNode(id) => Ok(format!("_:{id}")),
            _ => Err(Error::ontology_parsing("Cannot resolve token to URI")),
        }
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
                return Err(crate::error::Error::OntologyParsing {
                    message: format!("Undefined prefix: {}", prefix),
                });
            }
        } else if let Some(base) = &state.base_uri {
            let result = format!("{base}{name}");
            Ok(result)
        } else {
            // Relative URI without base - this should be an error
            Err(crate::error::Error::OntologyParsing {
                message: format!("Relative URI without base: {}", name),
            })
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
            _ => {
                // Handle other property assertions
                let subject_ind = Individual::Named(NamedIndividual {
                    iri: IRI::new(&subject),
                });
                let object_ind = Individual::Named(NamedIndividual {
                    iri: IRI::new(&object),
                });
                let property = ObjectProperty::new(IRI::new(&predicate))?;

                let axiom = ObjectPropertyAssertionAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    source: subject_ind,
                    target: object_ind,
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::ObjectPropertyAssertion(axiom));
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

/// Save ontology to Turtle file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
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

    std::fs::write(path, content).map_err(|e| Error::io(format!("Failed to write file: {e}")))
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

        let ontology = result.unwrap();

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

        let ontology = result.unwrap();

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
