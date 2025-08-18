//! SWRL Concrete Syntax Parser
//!
//! This module implements parsing of SWRL rules from human-readable concrete syntax.

use crate::swrl::{SWRLRule, SWRLAtom, SWRLVariable, SWRLIArgument, SWRLDArgument, SWRLValue};
use crate::ontology::{IRI, Individual, ClassExpression, Class, ObjectPropertyExpression, 
                      ObjectProperty, DataPropertyExpression, DataProperty, Literal};
use crate::{Error, Result};
use std::collections::HashMap;

/// Union type for SWRL arguments that can be either individual or data arguments
#[derive(Debug, Clone, PartialEq)]
pub enum SWRLArgument {
    Individual(SWRLIArgument),
    Data(SWRLDArgument),
}

// =============================================================================
// PARSER STRUCTURES
// =============================================================================

/// SWRL concrete syntax parser
pub struct SWRLParser {
    /// Namespace prefix mappings
    namespace_manager: NamespaceManager,
    /// Error reporting context
    current_position: usize,
    /// Input text being parsed
    input: String,
}

/// Manages namespace prefixes and IRI resolution
#[derive(Debug, Clone)]
pub struct NamespaceManager {
    /// Prefix to namespace mappings
    prefixes: HashMap<String, String>,
    /// Default namespace (if any)
    default_namespace: Option<String>,
}

/// Parse error information
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Position in input where error occurred
    pub position: usize,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
    /// Context around the error
    pub context: String,
}

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Variable: ?var
    Variable(String),
    /// Identifier: Person, hasAge
    Identifier(String),
    /// URI: <http://example.org/Person>
    URI(String),
    /// QName: prefix:local
    QName(String, String),
    /// String literal: "text"
    StringLiteral(String),
    /// Numeric literal: 42, 3.14
    NumericLiteral(String),
    /// Boolean literal: true, false
    BooleanLiteral(bool),
    /// Left parenthesis: (
    LeftParen,
    /// Right parenthesis: )
    RightParen,
    /// Comma: ,
    Comma,
    /// Conjunction: ∧ or AND
    Conjunction,
    /// Implication: → or ->
    Implication,
    /// End of input
    EOF,
}

/// Lexer for tokenizing SWRL syntax
pub struct Lexer {
    /// Input text
    input: String,
    /// Current position
    position: usize,
    /// Current character
    current_char: Option<char>,
}

// =============================================================================
// PARSER IMPLEMENTATION
// =============================================================================

impl SWRLParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            namespace_manager: NamespaceManager::new(),
            current_position: 0,
            input: String::new(),
        }
    }
    
    /// Add a namespace prefix
    pub fn add_prefix(&mut self, prefix: &str, namespace: &str) {
        self.namespace_manager.add_prefix(prefix, namespace);
    }
    
    /// Set default namespace
    pub fn set_default_namespace(&mut self, namespace: &str) {
        self.namespace_manager.set_default_namespace(namespace);
    }
    
    /// Parse a SWRL rule from text
    pub fn parse_rule(&mut self, input: &str) -> Result<SWRLRule> {
        self.input = input.to_string();
        self.current_position = 0;
        
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        
        self.parse_rule_from_tokens(&tokens)
    }
    
    /// Parse multiple rules from text (one per line)
    pub fn parse_rules(&mut self, input: &str) -> Result<Vec<SWRLRule>> {
        let mut rules = Vec::new();
        
        for (line_num, line) in input.lines().enumerate() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            
            // Handle namespace declarations
            if trimmed.starts_with("@prefix") {
                self.parse_prefix_declaration(trimmed)?;
                continue;
            }
            
            // Parse rule
            match self.parse_rule(trimmed) {
                Ok(rule) => rules.push(rule),
                Err(e) => {
                    return Err(Error::reasoning(format!(
                        "Parse error on line {}: {}", 
                        line_num + 1, 
                        e
                    )));
                }
            }
        }
        
        Ok(rules)
    }
    
    /// Parse rule from tokens
    fn parse_rule_from_tokens(&mut self, tokens: &[Token]) -> Result<SWRLRule> {
        if tokens.is_empty() {
            return Err(self.error("Empty rule"));
        }
        
        // Find implication operator
        let implication_pos = tokens.iter().position(|t| matches!(t, Token::Implication))
            .ok_or_else(|| self.error("Missing implication operator (-> or →)"))?;
        
        // Parse body (antecedent)
        let body_tokens = &tokens[..implication_pos];
        let body = self.parse_atom_list(body_tokens)?;
        
        // Parse head (consequent)
        let head_tokens = &tokens[implication_pos + 1..];
        let head = self.parse_atom_list(head_tokens)?;
        
        if head.is_empty() {
            return Err(self.error("Rule head cannot be empty"));
        }
        
        Ok(SWRLRule { body, head })
    }
    
    /// Parse a list of atoms (connected by conjunction)
    fn parse_atom_list(&mut self, tokens: &[Token]) -> Result<Vec<SWRLAtom>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut atoms = Vec::new();
        let mut current_atom_tokens = Vec::new();
        let mut paren_depth = 0;
        
        for token in tokens {
            match token {
                Token::LeftParen => {
                    paren_depth += 1;
                    current_atom_tokens.push(token.clone());
                }
                Token::RightParen => {
                    paren_depth -= 1;
                    current_atom_tokens.push(token.clone());
                }
                Token::Comma | Token::Conjunction => {
                    if paren_depth == 0 {
                        // This comma/conjunction is between atoms, not inside an atom
                        if !current_atom_tokens.is_empty() {
                            atoms.push(self.parse_atom(&current_atom_tokens)?);
                            current_atom_tokens.clear();
                        }
                    } else {
                        // This comma is inside parentheses, part of the current atom
                        current_atom_tokens.push(token.clone());
                    }
                }
                _ => {
                    current_atom_tokens.push(token.clone());
                }
            }
        }
        
        // Parse last atom
        if !current_atom_tokens.is_empty() {
            atoms.push(self.parse_atom(&current_atom_tokens)?);
        }
        
        Ok(atoms)
    }
    
    /// Parse a single atom
    fn parse_atom(&mut self, tokens: &[Token]) -> Result<SWRLAtom> {
        if tokens.len() < 4 {
            return Err(self.error("Atom must have at least predicate and arguments"));
        }
        
        // Expected format: Predicate(arg1, arg2, ...)
        let predicate_token = &tokens[0];
        
        if !matches!(tokens[1], Token::LeftParen) {
            return Err(self.error("Expected '(' after predicate"));
        }
        
        let predicate_name = match predicate_token {
            Token::Identifier(name) => name.clone(),
            Token::QName(prefix, local) => {
                self.namespace_manager.resolve_qname(prefix, local)?
            }
            Token::URI(uri) => uri.clone(),
            _ => return Err(self.error("Invalid predicate")),
        };
        
        // Parse arguments
        let arg_tokens = &tokens[2..tokens.len()-1]; // Remove LeftParen and RightParen
        let arguments = self.parse_arguments(arg_tokens)?;
        
        // Determine atom type based on predicate and argument count
        self.create_atom(&predicate_name, &arguments)
    }
    
    /// Parse arguments list
    fn parse_arguments(&mut self, tokens: &[Token]) -> Result<Vec<SWRLArgument>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut arguments = Vec::new();
        let mut current_arg_tokens = Vec::new();
        
        for token in tokens {
            match token {
                Token::Comma => {
                    if !current_arg_tokens.is_empty() {
                        arguments.push(self.parse_argument(&current_arg_tokens)?);
                        current_arg_tokens.clear();
                    }
                }
                _ => {
                    current_arg_tokens.push(token.clone());
                }
            }
        }
        
        // Parse last argument
        if !current_arg_tokens.is_empty() {
            arguments.push(self.parse_argument(&current_arg_tokens)?);
        }
        
        Ok(arguments)
    }
    
    /// Parse a single argument
    fn parse_argument(&mut self, tokens: &[Token]) -> Result<SWRLArgument> {
        if tokens.len() != 1 {
            return Err(self.error("Each argument must be a single token"));
        }
        
        match &tokens[0] {
            Token::Variable(name) => {
                let var_iri = format!("urn:swrl:var#{}", name);
                Ok(SWRLArgument::Individual(SWRLIArgument::Variable(SWRLVariable::new(IRI::new(&var_iri)))))
            }
            Token::Identifier(name) => {
                let individual_iri = self.namespace_manager.resolve_identifier(name)?;
                Ok(SWRLArgument::Individual(SWRLIArgument::Individual(Individual::named(IRI::new(&individual_iri)))))
            }
            Token::QName(prefix, local) => {
                let iri = self.namespace_manager.resolve_qname(prefix, local)?;
                Ok(SWRLArgument::Individual(SWRLIArgument::Individual(Individual::named(IRI::new(&iri)))))
            }
            Token::URI(uri) => {
                Ok(SWRLArgument::Individual(SWRLIArgument::Individual(Individual::named(IRI::new(uri)))))
            }
            Token::StringLiteral(value) => {
                Ok(SWRLArgument::Data(SWRLDArgument::Literal(Literal {
                    value: value.clone(),
                    datatype: IRI::new("http://www.w3.org/2001/XMLSchema#string").to_url().ok(),
                    language: None,
                })))
            }
            Token::NumericLiteral(value) => {
                let datatype = if value.contains('.') {
                    "http://www.w3.org/2001/XMLSchema#double"
                } else {
                    "http://www.w3.org/2001/XMLSchema#integer"
                };
                
                Ok(SWRLArgument::Data(SWRLDArgument::Literal(Literal {
                    value: value.clone(),
                    datatype: IRI::new(&datatype).to_url().ok(),
                    language: None,
                })))
            }
            Token::BooleanLiteral(value) => {
                Ok(SWRLArgument::Data(SWRLDArgument::Literal(Literal {
                    value: value.to_string(),
                    datatype: IRI::new("http://www.w3.org/2001/XMLSchema#boolean").to_url().ok(),
                    language: None,
                })))
            }
            _ => Err(self.error("Invalid argument type")),
        }
    }
    
    /// Create atom from predicate name and arguments
    fn create_atom(&mut self, predicate_name: &str, arguments: &[SWRLArgument]) -> Result<SWRLAtom> {
        // Check if it's a built-in predicate
        if predicate_name.contains("swrlb#") || predicate_name.starts_with("http://www.w3.org/2003/11/swrlb#") {
            // Convert SWRLArguments to SWRLDArguments for built-ins (most built-ins work with data)
            let data_arguments: Vec<SWRLDArgument> = arguments.iter().filter_map(|arg| {
                match arg {
                    SWRLArgument::Data(data_arg) => Some(data_arg.clone()),
                    SWRLArgument::Individual(SWRLIArgument::Variable(var)) => {
                        // Variables can be used in both contexts
                        Some(SWRLDArgument::Variable(var.clone()))
                    },
                    _ => None, // Skip individual constants for data-focused built-ins
                }
            }).collect();
            
            return Ok(SWRLAtom::BuiltInAtom {
                predicate: IRI::new(predicate_name),
                arguments: data_arguments,
            });
        }
        
        // Determine atom type based on argument count
        match arguments.len() {
            1 => {
                // Class atom: Class(individual)
                let class_iri = self.namespace_manager.resolve_identifier(predicate_name)?;
                Ok(SWRLAtom::ClassAtom {
                    predicate: ClassExpression::Class(Class::new(IRI::new(&class_iri))),
                    argument: match &arguments[0] {
                        SWRLArgument::Individual(arg) => arg.clone(),
                        _ => return Err(Error::ontology_parsing("Class atom argument must be an individual")),
                    },
                })
            }
            2 => {
                // Check for special predicates
                if predicate_name == "sameAs" || predicate_name == "owl:sameAs" {
                    return Ok(SWRLAtom::SameIndividualAtom {
                        first_argument: match &arguments[0] {
                            SWRLArgument::Individual(arg) => arg.clone(),
                            _ => return Err(Error::ontology_parsing("SameAs arguments must be individuals")),
                        },
                        second_argument: match &arguments[1] {
                            SWRLArgument::Individual(arg) => arg.clone(),
                            _ => return Err(Error::ontology_parsing("SameAs arguments must be individuals")),
                        },
                    });
                }
                
                if predicate_name == "differentFrom" || predicate_name == "owl:differentFrom" {
                    return Ok(SWRLAtom::DifferentIndividualsAtom {
                        first_argument: match &arguments[0] {
                            SWRLArgument::Individual(arg) => arg.clone(),
                            _ => return Err(Error::ontology_parsing("Arguments must be individuals in differentFrom atom")),
                        },
                        second_argument: match &arguments[1] {
                            SWRLArgument::Individual(arg) => arg.clone(),
                            _ => return Err(Error::ontology_parsing("Arguments must be individuals in differentFrom atom")),
                        },
                    });
                }
                
                // Check if second argument is literal (data property) or individual (object property)
                match &arguments[1] {
                    SWRLArgument::Data(_) => {
                        // Data property atom
                        let prop_iri = self.namespace_manager.resolve_identifier(predicate_name)?;
                        Ok(SWRLAtom::DataPropertyAtom {
                            predicate: DataPropertyExpression::DataProperty(DataProperty { iri: IRI::new(&prop_iri) }),
                            first_argument: match &arguments[0] {
                                SWRLArgument::Individual(arg) => arg.clone(),
                                _ => return Err(Error::ontology_parsing("First argument must be individual in data property atom")),
                            },
                            second_argument: match &arguments[1] {
                                SWRLArgument::Data(arg) => arg.clone(),
                                _ => return Err(Error::ontology_parsing("Second argument must be data in data property atom")),
                            },
                        })
                    }
                    _ => {
                        // Object property atom
                        let prop_iri = self.namespace_manager.resolve_identifier(predicate_name)?;
                        Ok(SWRLAtom::ObjectPropertyAtom {
                            predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty::new(IRI::new(&prop_iri))?),
                            first_argument: match &arguments[0] {
                                SWRLArgument::Individual(arg) => arg.clone(),
                                _ => return Err(Error::ontology_parsing("First argument must be individual in object property atom")),
                            },
                            second_argument: match &arguments[1] {
                                SWRLArgument::Individual(arg) => arg.clone(),
                                _ => return Err(Error::ontology_parsing("Second argument must be individual in object property atom")),
                            },
                        })
                    }
                }
            }
            _ => {
                // Built-in atom with variable arity - convert to data arguments
                let data_arguments: Vec<SWRLDArgument> = arguments.iter().filter_map(|arg| {
                    match arg {
                        SWRLArgument::Data(data_arg) => Some(data_arg.clone()),
                        SWRLArgument::Individual(SWRLIArgument::Variable(var)) => {
                            Some(SWRLDArgument::Variable(var.clone()))
                        },
                        _ => None,
                    }
                }).collect();
                
                Ok(SWRLAtom::BuiltInAtom {
                    predicate: IRI::new(predicate_name),
                    arguments: data_arguments,
                })
            }
        }
    }
    
    /// Parse namespace prefix declaration
    fn parse_prefix_declaration(&mut self, line: &str) -> Result<()> {
        // Format: @prefix prefix: <namespace>
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() != 3 {
            return Err(self.error("Invalid prefix declaration format"));
        }
        
        let prefix = parts[1].trim_end_matches(':');
        let namespace = parts[2].trim_start_matches('<').trim_end_matches('>');
        
        self.add_prefix(prefix, namespace);
        Ok(())
    }
    
    /// Create parse error
    fn error(&self, message: &str) -> Error {
        Error::reasoning(format!("Parse error: {}", message))
    }
}

// =============================================================================
// NAMESPACE MANAGER
// =============================================================================

impl NamespaceManager {
    /// Create new namespace manager
    pub fn new() -> Self {
        let mut manager = Self {
            prefixes: HashMap::new(),
            default_namespace: None,
        };
        
        // Add standard prefixes
        manager.add_prefix("owl", "http://www.w3.org/2002/07/owl#");
        manager.add_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
        manager.add_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
        manager.add_prefix("swrl", "http://www.w3.org/2003/11/swrl#");
        manager.add_prefix("swrlb", "http://www.w3.org/2003/11/swrlb#");
        manager.add_prefix("xsd", "http://www.w3.org/2001/XMLSchema#");
        
        manager
    }
    
    /// Add namespace prefix
    pub fn add_prefix(&mut self, prefix: &str, namespace: &str) {
        self.prefixes.insert(prefix.to_string(), namespace.to_string());
    }
    
    /// Set default namespace
    pub fn set_default_namespace(&mut self, namespace: &str) {
        self.default_namespace = Some(namespace.to_string());
    }
    
    /// Resolve QName to full IRI
    pub fn resolve_qname(&self, prefix: &str, local: &str) -> Result<String> {
        if let Some(namespace) = self.prefixes.get(prefix) {
            Ok(format!("{}{}", namespace, local))
        } else {
            Err(Error::reasoning(format!("Unknown namespace prefix: {}", prefix)))
        }
    }
    
    /// Resolve identifier (may use default namespace)
    pub fn resolve_identifier(&self, identifier: &str) -> Result<String> {
        if identifier.starts_with("http://") || identifier.starts_with("https://") {
            Ok(identifier.to_string())
        } else if let Some(default_ns) = &self.default_namespace {
            Ok(format!("{}{}", default_ns, identifier))
        } else {
            Ok(format!("http://example.org/{}", identifier))
        }
    }
}

// =============================================================================
// LEXER IMPLEMENTATION
// =============================================================================

impl Lexer {
    /// Create new lexer
    pub fn new(input: &str) -> Self {
        let mut lexer = Self {
            input: input.to_string(),
            position: 0,
            current_char: None,
        };
        
        if !input.is_empty() {
            lexer.current_char = input.chars().next();
        }
        
        lexer
    }
    
    /// Tokenize input
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while let Some(token) = self.next_token()? {
            if !matches!(token, Token::EOF) {
                tokens.push(token);
            } else {
                break;
            }
        }
        
        Ok(tokens)
    }
    
    /// Get next token
    fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace();
        
        if self.current_char.is_none() {
            return Ok(Some(Token::EOF));
        }
        
        let ch = self.current_char.unwrap();
        
        match ch {
            '?' => {
                self.advance();
                let name = self.read_identifier()?;
                Ok(Some(Token::Variable(name)))
            }
            '<' => {
                self.advance();
                let uri = self.read_until('>')?;
                self.advance(); // skip '>'
                Ok(Some(Token::URI(uri)))
            }
            '"' => {
                self.advance();
                let string = self.read_until('"')?;
                self.advance(); // skip '"'
                Ok(Some(Token::StringLiteral(string)))
            }
            '(' => {
                self.advance();
                Ok(Some(Token::LeftParen))
            }
            ')' => {
                self.advance();
                Ok(Some(Token::RightParen))
            }
            ',' => {
                self.advance();
                Ok(Some(Token::Comma))
            }
            '∧' => {
                self.advance();
                Ok(Some(Token::Conjunction))
            }
            '→' => {
                self.advance();
                Ok(Some(Token::Implication))
            }
            '-' if self.peek() == Some('>') => {
                self.advance(); // skip '-'
                self.advance(); // skip '>'
                Ok(Some(Token::Implication))
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let identifier = self.read_identifier()?;
                
                // Check for special keywords
                match identifier.as_str() {
                    "true" => Ok(Some(Token::BooleanLiteral(true))),
                    "false" => Ok(Some(Token::BooleanLiteral(false))),
                    "AND" | "and" => Ok(Some(Token::Conjunction)),
                    _ => {
                        // Check if it's a QName
                        if identifier.contains(':') {
                            let parts: Vec<&str> = identifier.splitn(2, ':').collect();
                            Ok(Some(Token::QName(parts[0].to_string(), parts[1].to_string())))
                        } else {
                            Ok(Some(Token::Identifier(identifier)))
                        }
                    }
                }
            }
            _ if ch.is_ascii_digit() || ch == '.' => {
                let number = self.read_number()?;
                Ok(Some(Token::NumericLiteral(number)))
            }
            _ => {
                Err(Error::reasoning(format!("Unexpected character: {}", ch)))
            }
        }
    }
    
    /// Advance to next character
    fn advance(&mut self) {
        self.position += 1;
        if self.position >= self.input.len() {
            self.current_char = None;
        } else {
            self.current_char = self.input.chars().nth(self.position);
        }
    }
    
    /// Peek at next character
    fn peek(&self) -> Option<char> {
        if self.position + 1 >= self.input.len() {
            None
        } else {
            self.input.chars().nth(self.position + 1)
        }
    }
    
    /// Skip whitespace
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Read identifier
    fn read_identifier(&mut self) -> Result<String> {
        let mut identifier = String::new();
        
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '-' {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(identifier)
    }
    
    /// Read number
    fn read_number(&mut self) -> Result<String> {
        let mut number = String::new();
        
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() || ch == '.' {
                number.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(number)
    }
    
    /// Read until specific character
    fn read_until(&mut self, delimiter: char) -> Result<String> {
        let mut result = String::new();
        
        while let Some(ch) = self.current_char {
            if ch == delimiter {
                break;
            }
            result.push(ch);
            self.advance();
        }
        
        Ok(result)
    }
}

impl Default for SWRLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_rule_parsing() {
        let mut parser = SWRLParser::new();
        
        let rule_text = "Person(?p) -> Adult(?p)";
        let result = parser.parse_rule(rule_text);
        
        assert!(result.is_ok());
        let rule = result.unwrap();
        assert_eq!(rule.body.len(), 1);
        assert_eq!(rule.head.len(), 1);
    }
    
    #[test]
    fn test_namespace_resolution() {
        let mut manager = NamespaceManager::new();
        manager.add_prefix("ex", "http://example.org/");
        
        let result = manager.resolve_qname("ex", "Person");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://example.org/Person");
    }
    
    #[test]
    fn test_lexer_tokenization() {
        let mut lexer = Lexer::new("Person(?p)");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert_eq!(tokens[1], Token::LeftParen);
        assert!(matches!(tokens[2], Token::Variable(_)));
        assert_eq!(tokens[3], Token::RightParen);
    }
    
    #[test]
    fn test_complex_rule_parsing() {
        let mut parser = SWRLParser::new();
        
        let rule_text = r#"Person(?p), hasAge(?p, ?age), swrlb:greaterThan(?age, 18) -> Adult(?p)"#;
        let result = parser.parse_rule(rule_text);
        
        if let Err(ref e) = result {
            println!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
        let rule = result.unwrap();
        assert_eq!(rule.body.len(), 3);
        assert_eq!(rule.head.len(), 1);
    }
    
    #[test]
    fn test_prefix_declaration() {
        let mut parser = SWRLParser::new();
        
        let rules_text = r#"
            @prefix ex: <http://example.org/>
            ex:Person(?p) -> ex:Adult(?p)
        "#;
        
        let result = parser.parse_rules(rules_text);
        assert!(result.is_ok());
        
        let rules = result.unwrap();
        assert_eq!(rules.len(), 1);
    }
}
