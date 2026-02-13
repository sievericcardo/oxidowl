//! Functional Syntax Parser
//!
//! This module implements parsing of OWL 2 ontologies from Functional Syntax.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use phf::phf_set;

use crate::{
    Error, Result,
    ontology::{ClassExpression, Ontology},
    parsers::{ErrorVerbosity, ParserConfig, common::OntologySerializer},
};

/// Compile-time perfect hash set of all OWL 2 keywords
/// Provides O(1) lookup with zero runtime overhead
static OWL_KEYWORDS: phf::Set<&'static str> = phf_set! {
    // Class expressions
    "ObjectIntersectionOf", "ObjectUnionOf", "ObjectComplementOf",
    "ObjectOneOf", "ObjectSomeValuesFrom", "ObjectAllValuesFrom",
    "ObjectHasValue", "ObjectHasSelf", "ObjectMinCardinality",
    "ObjectMaxCardinality", "ObjectExactCardinality",
    "ObjectInverseOf",

    // Data ranges and restrictions
    "DataSomeValuesFrom", "DataAllValuesFrom", "DataHasValue",
    "DataMinCardinality", "DataMaxCardinality", "DataExactCardinality",
    "DataIntersectionOf", "DataUnionOf", "DataComplementOf",
    "DataOneOf", "DatatypeRestriction",

    // Axioms
    "SubClassOf", "EquivalentClasses", "DisjointClasses", "DisjointUnion",
    "SubObjectPropertyOf", "EquivalentObjectProperties", "DisjointObjectProperties",
    "InverseObjectProperties", "ObjectPropertyDomain", "ObjectPropertyRange",
    "FunctionalObjectProperty", "InverseFunctionalObjectProperty",
    "ReflexiveObjectProperty", "IrreflexiveObjectProperty",
    "SymmetricObjectProperty", "AsymmetricObjectProperty", "TransitiveObjectProperty",
    "SubDataPropertyOf", "EquivalentDataProperties", "DisjointDataProperties",
    "DataPropertyDomain", "DataPropertyRange", "FunctionalDataProperty",
    "DatatypeDefinition", "HasKey", "SameIndividual", "DifferentIndividuals",
    "ClassAssertion", "ObjectPropertyAssertion", "NegativeObjectPropertyAssertion",
    "DataPropertyAssertion", "NegativeDataPropertyAssertion",

    // Annotations
    "Annotation", "AnnotationAssertion", "SubAnnotationPropertyOf",
    "AnnotationPropertyDomain", "AnnotationPropertyRange",

    // SWRL
    "DLSafeRule", "Body", "Head",

    // Ontology structure
    "Ontology", "Import", "Prefix", "Declaration",
};

/// Check if a token is an OWL keyword with O(1) lookup
#[inline(always)]
fn is_owl_keyword(token: &str) -> bool {
    OWL_KEYWORDS.contains(token)
}

/// Check if a token is a structural token that should not be expanded as an IRI
#[inline(always)]
fn is_structural_token(token: &str) -> bool {
    token == "(" || token == ")" || is_owl_keyword(token)
}

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Lightweight context for tracking parse position
#[derive(Debug, Clone, Copy)]
struct ParseContext {
    line: u32,
    column: u32,
}

impl ParseContext {
    #[inline(always)]
    fn new() -> Self {
        Self { line: 1, column: 1 }
    }

    /// Update context based on character (for tracking position during tokenization)
    #[inline(always)]
    fn update(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
}

/// Functional Syntax Parser
#[derive(Debug, Clone)]
pub struct FunctionalParser {
    /// Parser configuration
    config: ParserConfig,
}

impl FunctionalParser {
    /// Create a new functional syntax parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a new functional syntax parser with custom configuration
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Construct an error with appropriate verbosity
    /// Mark as cold to optimize branch prediction
    #[cold]
    #[inline(never)]
    fn make_error(&self, message: String, token: Option<String>) -> Error {
        match self.config.error_verbosity {
            ErrorVerbosity::Minimal => Error::ontology_parsing(message),
            ErrorVerbosity::Standard => {
                // Line/column tracking not implemented yet, but structure ready
                Error::ontology_parsing_detailed(message, None, None, None, token)
            }
            ErrorVerbosity::Detailed => {
                // Full context available
                Error::ontology_parsing_detailed(message, None, None, None, token)
            }
        }
    }
}

impl Default for FunctionalParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Functional Syntax from string content
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = FunctionalParser::new();
    parser.parse_string(content)
}

impl FunctionalParser {
    /// Parse Functional Syntax content into an ontology
    pub fn parse_string(&self, content: &str) -> Result<Ontology> {
        // Validate syntax before parsing
        let validator = super::validation::SyntaxValidator::new();
        validator.validate_functional(content)?;

        let mut ontology = Ontology::new();

        // Handle placeholder or empty content
        let trimmed = content.trim();
        if trimmed == "(placeholder)" || trimmed.is_empty() {
            return Ok(ontology);
        }

        let mut prefixes = std::collections::HashMap::<String, String>::new();
        let mut base_iri: Option<String> = None;

        // Tokenize the content
        let tokens = self.tokenize(content)?;
        let mut position = 0;

        while position < tokens.len() {
            position = self.parse_statement(
                &tokens,
                position,
                &mut ontology,
                &mut prefixes,
                &mut base_iri,
            )?;
        }

        Ok(ontology)
    }

    /// Tokenize the functional syntax content
    #[inline(always)]
    pub fn tokenize(&self, content: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_iri = false;
        let mut in_string = false;
        let mut paren_depth = 0;
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' if !in_iri => {
                    // Handle quoted strings
                    if in_string {
                        // End of string
                        current_token.push(ch);
                        tokens.push(current_token.clone());
                        current_token.clear();
                        in_string = false;
                        
                        // Check for ^^ after the string
                        if chars.peek() == Some(&'^') {
                            chars.next(); // consume first ^
                            if chars.peek() == Some(&'^') {
                                chars.next(); // consume second ^
                                tokens.push("^^".to_string());
                            } else {
                                current_token.push('^');
                            }
                        }
                    } else {
                        // Start of string
                        if !current_token.is_empty() {
                            tokens.push(current_token.trim().to_string());
                            current_token.clear();
                        }
                        in_string = true;
                        current_token.push(ch);
                    }
                }
                '<' if !in_iri && !in_string => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    in_iri = true;
                    current_token.push(ch);
                }
                '>' if in_iri && !in_string => {
                    current_token.push(ch);
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                    in_iri = false;
                }
                '(' | ')' if !in_iri && !in_string => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                    if ch == '(' {
                        paren_depth += 1;
                    } else {
                        paren_depth -= 1;
                    }
                }
                ' ' | '\t' | '\n' | '\r' if !in_iri && !in_string => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token.trim().to_string());
        }

        Ok(tokens)
    }

    /// Parse a single statement from tokens
    fn parse_statement(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &mut std::collections::HashMap<String, String>,
        base_iri: &mut Option<String>,
    ) -> Result<usize> {
        if position >= tokens.len() {
            return Ok(position);
        }

        match tokens[position].as_str() {
            "Prefix" => {
                position = self.parse_prefix(tokens, position, prefixes)?;
            }
            "Ontology" => {
                position = self
                    .parse_ontology_declaration(tokens, position, ontology, prefixes, base_iri)?;
            }
            "Declaration" => {
                position =
                    self.parse_declaration(tokens, position, ontology, prefixes, base_iri)?;
            }
            "SubClassOf" => {
                position =
                    self.parse_subclass_of(tokens, position, ontology, prefixes, base_iri)?;
            }
            "DisjointClasses" => {
                position =
                    self.parse_disjoint_classes(tokens, position, ontology, prefixes, base_iri)?;
            }
            "EquivalentClasses" => {
                position =
                    self.parse_equivalent_classes(tokens, position, ontology, prefixes, base_iri)?;
            }
            "ClassAssertion" => {
                position =
                    self.parse_class_assertion(tokens, position, ontology, prefixes, base_iri)?;
            }
            "ObjectPropertyAssertion" => {
                position =
                    self.parse_object_property_assertion(tokens, position, ontology, prefixes)?;
            }
            "TransitiveObjectProperty" => {
                position =
                    self.parse_transitive_object_property(tokens, position, ontology, prefixes)?;
            }
            "SymmetricObjectProperty" => {
                position =
                    self.parse_symmetric_object_property(tokens, position, ontology, prefixes)?;
            }
            "ReflexiveObjectProperty" => {
                position =
                    self.parse_reflexive_object_property(tokens, position, ontology, prefixes)?;
            }
            "FunctionalObjectProperty" => {
                position =
                    self.parse_functional_object_property(tokens, position, ontology, prefixes)?;
            }
            "InverseFunctionalObjectProperty" => {
                position = self.parse_inverse_functional_object_property(
                    tokens, position, ontology, prefixes,
                )?;
            }
            "HasKey" => {
                position = self.parse_has_key(tokens, position, ontology, prefixes)?;
            }
            "DLSafeRule" => {
                // Simple SWRL rule parsing with minimal overhead
                position = self.parse_swrl_rule(tokens, position)?;
            }
            _ => {
                // Skip unknown constructs
                position += 1;
            }
        }

        Ok(position)
    }

    /// Parse prefix declaration: Prefix(prefix:=<IRI>)
    fn parse_prefix(
        &self,
        tokens: &[String],
        mut position: usize,
        prefixes: &mut std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "Prefix"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            if position + 1 < tokens.len() {
                // Check for two-token format: ":=" "<IRI>" or "prefix:=" "<IRI>"
                if tokens[position].ends_with(":=") && tokens[position + 1].starts_with("<") {
                    let prefix_part = &tokens[position];
                    let prefix_name = if prefix_part == ":=" {
                        "".to_string() // Default prefix
                    } else {
                        prefix_part[..prefix_part.len() - 2].to_string() // Remove ":="
                    };
                    let iri = tokens[position + 1]
                        .trim_matches(['<', '>'].as_ref())
                        .to_string();
                    prefixes.insert(prefix_name, iri);
                    position += 2;
                } else if tokens[position].contains(":=") {
                    // Single token format: "prefix:=<IRI>"
                    let prefix_def = &tokens[position];
                    if let Some(eq_pos) = prefix_def.find(":=") {
                        let prefix_name = prefix_def[..eq_pos].to_string();
                        let iri = prefix_def[eq_pos + 2..]
                            .trim_matches(['<', '>'].as_ref())
                            .to_string();
                        prefixes.insert(prefix_name, iri);
                    }
                    position += 1;
                } else {
                    position += 1;
                }
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Skip over Annotation(...) sequences that can precede axioms
    /// Returns the new position after all annotations
    #[inline(always)]
    fn skip_annotations(&self, tokens: &[String], mut position: usize) -> usize {
        // Skip any number of Annotation(...) constructs
        while position < tokens.len() && tokens[position] == "Annotation" {
            // Check if next token is "(" - if not, this is not an Annotation construct
            if position + 1 >= tokens.len() || tokens[position + 1] != "(" {
                // Not an Annotation(...) - stop skipping
                break;
            }

            position += 1; // Skip "Annotation"
            position += 1; // Skip "("

            // Count parentheses to find the matching closing paren
            let mut paren_count = 1;
            while position < tokens.len() && paren_count > 0 {
                if tokens[position] == "(" {
                    paren_count += 1;
                } else if tokens[position] == ")" {
                    paren_count -= 1;
                }
                position += 1;
            }
        }
        position
    }

    /// Parse SWRL rule with minimal overhead
    /// Uses simple validation - just checks basic structure
    fn parse_swrl_rule(&self, tokens: &[String], mut position: usize) -> Result<usize> {
        position += 1; // Skip "DLSafeRule"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Count parentheses to find the matching closing paren
            let mut paren_count = 1;

            while position < tokens.len() && paren_count > 0 {
                if tokens[position] == "(" {
                    paren_count += 1;
                } else if tokens[position] == ")" {
                    paren_count -= 1;
                }
                position += 1;
            }

            // Minimal validation only in Detailed mode
            if matches!(self.config.error_verbosity, ErrorVerbosity::Detailed) {
                // Check that we found the closing paren
                if paren_count != 0 {
                    return Err(Error::ontology_parsing(
                        "Unbalanced parentheses in SWRL rule".to_string(),
                    ));
                }
            }
        }

        Ok(position)
    }

    /// Parse ontology declaration: Ontology(<IRI> ...)
    fn parse_ontology_declaration(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &mut std::collections::HashMap<String, String>,
        base_iri: &mut Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "Ontology"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            if position < tokens.len() && tokens[position].starts_with('<') {
                let iri_str = tokens[position].trim_matches(['<', '>'].as_ref());
                if url::Url::parse(iri_str).is_ok() {
                    ontology.set_iri(crate::ontology::IRI::new(iri_str));
                    // Store base IRI for resolving relative IRIs
                    *base_iri = Some(iri_str.to_string());
                    // Add default prefix (empty string) for relative IRIs starting with ":"
                    let default_base = if iri_str.ends_with('#') || iri_str.ends_with('/') {
                        iri_str.to_string()
                    } else {
                        format!("{}#", iri_str)
                    };
                    prefixes.insert(String::new(), default_base);
                }
                position += 1;
            }

            // Parse content inside the ontology declaration
            let mut paren_count = 1;

            while position < tokens.len() && paren_count > 0 {
                if tokens[position] == "(" {
                    paren_count += 1;
                } else if tokens[position] == ")" {
                    paren_count -= 1;
                    if paren_count == 0 {
                        break; // Exit when we find the matching closing parenthesis
                    }
                }

                // Parse statements inside the ontology
                if paren_count == 1 {
                    // Only parse top-level statements
                    position =
                        self.parse_statement(tokens, position, ontology, prefixes, base_iri)?;
                } else {
                    position += 1;
                }
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip final ")"
            }
        }

        Ok(position)
    }

    /// Parse declaration: Declaration(Class(<IRI>))
    fn parse_declaration(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
        _base_iri: &Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "Declaration"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            if position < tokens.len() {
                match tokens[position].as_str() {
                    "Class" => {
                        position += 1;
                        if position < tokens.len() && tokens[position] == "(" {
                            position += 1;
                            if position < tokens.len() {
                                let iri = self.expand_iri(&tokens[position], prefixes)?;
                                let class = crate::ontology::Class {
                                    iri: url::Url::parse(&iri)
                                        .map_err(|e| {
                                            Error::ontology_parsing(format!("Invalid IRI: {e}"))
                                        })?
                                        .into(),
                                };
                                ontology.add_class(class);
                                position += 1;
                            }
                            if position < tokens.len() && tokens[position] == ")" {
                                position += 1;
                            }
                        }
                    }
                    "ObjectProperty" => {
                        position += 1;
                        if position < tokens.len() && tokens[position] == "(" {
                            position += 1;
                            if position < tokens.len() {
                                let iri = self.expand_iri(&tokens[position], prefixes)?;
                                let property = crate::ontology::ObjectProperty {
                                    iri: crate::ontology::IRI::new(&iri),
                                };
                                ontology.add_object_property(property);
                                position += 1;
                            }
                            if position < tokens.len() && tokens[position] == ")" {
                                position += 1;
                            }
                        }
                    }
                    _ => position += 1,
                }
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Parse an object property expression from tokens
    #[inline(always)]
    fn parse_object_property_expression(
        &self,
        tokens: &[String],
        mut position: usize,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<(crate::ontology::ObjectPropertyExpression, usize)> {
        if position >= tokens.len() {
            return Err(Error::ontology_parsing(
                "Unexpected end of tokens while parsing object property expression".to_string(),
            ));
        }

        let token = &tokens[position];

        // Handle ObjectInverseOf
        if token == "ObjectInverseOf" {
            position += 1; // Skip "ObjectInverseOf"
            if position < tokens.len() && tokens[position] == "(" {
                position += 1; // Skip "("

                // Parse the property inside
                if position >= tokens.len() {
                    return Err(Error::ontology_parsing(
                        "Expected object property inside ObjectInverseOf".to_string(),
                    ));
                }

                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| {
                            Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                        })?
                        .into(),
                };

                Ok((
                    crate::ontology::ObjectPropertyExpression::InverseObjectProperty(property),
                    position,
                ))
            } else {
                Err(Error::ontology_parsing(
                    "Expected '(' after ObjectInverseOf".to_string(),
                ))
            }
        } else {
            // Simple object property IRI
            let property_iri = self.expand_iri(&tokens[position], prefixes)?;
            position += 1;

            let property = crate::ontology::ObjectProperty {
                iri: url::Url::parse(&property_iri)
                    .map_err(|e| {
                        Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                    })?
                    .into(),
            };

            Ok((
                crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                position,
            ))
        }
    }

    /// Parse a data range from tokens
    #[inline(always)]
    fn parse_data_range(
        &self,
        tokens: &[String],
        mut position: usize,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<(crate::ontology::DataRange, usize)> {
        if position >= tokens.len() {
            return Err(Error::ontology_parsing(
                "Unexpected end of tokens while parsing data range".to_string(),
            ));
        }

        let token = &tokens[position];

        // Handle complex data ranges
        match token.as_str() {
            "DataIntersectionOf" => {
                position += 1; // Skip "DataIntersectionOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut ranges = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let (range, new_pos) = self.parse_data_range(tokens, position, prefixes)?;
                        ranges.push(range);
                        position = new_pos;
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((crate::ontology::DataRange::DataIntersectionOf(ranges), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataIntersectionOf".to_string(),
                    ))
                }
            }
            "DataUnionOf" => {
                position += 1; // Skip "DataUnionOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut ranges = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let (range, new_pos) = self.parse_data_range(tokens, position, prefixes)?;
                        ranges.push(range);
                        position = new_pos;
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((crate::ontology::DataRange::DataUnionOf(ranges), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataUnionOf".to_string(),
                    ))
                }
            }
            "DataComplementOf" => {
                position += 1; // Skip "DataComplementOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let (range, new_pos) = self.parse_data_range(tokens, position, prefixes)?;
                    position = new_pos;

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((crate::ontology::DataRange::DataComplementOf(Box::new(range)), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataComplementOf".to_string(),
                    ))
                }
            }
            "DataOneOf" => {
                position += 1; // Skip "DataOneOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut literals = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let literal_token = &tokens[position];
                        position += 1;

                        // Parse the literal - check for typed literals (^^datatype)
                        let literal = if position < tokens.len() && tokens[position] == "^^" {
                            position += 1; // Skip "^^"
                            if position >= tokens.len() {
                                return Err(Error::ontology_parsing(
                                    "Expected datatype after ^^".to_string(),
                                ));
                            }
                            let datatype_iri = self.expand_iri(&tokens[position], prefixes)?;
                            position += 1;

                            crate::ontology::Literal {
                                value: literal_token.trim_matches('"').to_string(),
                                language: None,
                                datatype: Some(url::Url::parse(&datatype_iri).map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?),
                            }
                        } else {
                            // Untyped literal or language-tagged literal
                            crate::ontology::Literal {
                                value: literal_token.trim_matches('"').to_string(),
                                language: None,
                                datatype: None,
                            }
                        };

                        literals.push(literal);
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((crate::ontology::DataRange::DataOneOf(literals), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataOneOf".to_string(),
                    ))
                }
            }
            "DatatypeRestriction" => {
                position += 1; // Skip "DatatypeRestriction"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse base datatype
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected base datatype in DatatypeRestriction".to_string(),
                        ));
                    }
                    let base_datatype_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse facet-value pairs
                    let mut restrictions = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        // Parse facet IRI
                        let facet_iri = self.expand_iri(&tokens[position], prefixes)?;
                        position += 1;

                        // Parse literal value
                        if position >= tokens.len() {
                            return Err(Error::ontology_parsing(
                                "Expected literal value after facet in DatatypeRestriction".to_string(),
                            ));
                        }
                        let literal_token = &tokens[position];
                        position += 1;

                        // Parse literal with potential datatype
                        let literal = if position < tokens.len() && tokens[position] == "^^" {
                            position += 1; // Skip "^^"
                            if position >= tokens.len() {
                                return Err(Error::ontology_parsing(
                                    "Expected datatype after ^^".to_string(),
                                ));
                            }
                            let datatype_iri = self.expand_iri(&tokens[position], prefixes)?;
                            position += 1;

                            crate::ontology::Literal {
                                value: literal_token.trim_matches('"').to_string(),
                                language: None,
                                datatype: Some(url::Url::parse(&datatype_iri).map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?),
                            }
                        } else {
                            crate::ontology::Literal {
                                value: literal_token.trim_matches('"').to_string(),
                                language: None,
                                datatype: None,
                            }
                        };

                        restrictions.push(crate::ontology::FacetRestriction {
                            facet: url::Url::parse(&facet_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid facet IRI: {e}"))
                                })?
                                .into(),
                            value: literal,
                        });
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((
                        crate::ontology::DataRange::DatatypeRestriction {
                            datatype: url::Url::parse(&base_datatype_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?
                                .into(),
                            restrictions,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DatatypeRestriction".to_string(),
                    ))
                }
            }
            _ => {
                // Simple datatype IRI
                let datarange_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                Ok((
                    crate::ontology::DataRange::Datatype(
                        url::Url::parse(&datarange_iri)
                            .map_err(|e| {
                                Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                            })?
                            .into(),
                    ),
                    position,
                ))
            }
        }
    }

    /// Parse a class expression from tokens
    #[inline(always)]
    fn parse_class_expression(
        &self,
        tokens: &[String],
        mut position: usize,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<(ClassExpression, usize)> {
        if position >= tokens.len() {
            return Err(Error::ontology_parsing(
                "Unexpected end of tokens while parsing class expression".to_string(),
            ));
        }

        let token = &tokens[position];

        // Handle complex class expressions
        match token.as_str() {
            "DataHasValue" => {
                position += 1; // Skip "DataHasValue"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property after DataHasValue(".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse literal value
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected literal value in DataHasValue".to_string(),
                        ));
                    }
                    let literal_token = &tokens[position];
                    position += 1;

                    // Parse the literal - check for typed literals (^^datatype)
                    let literal = if position < tokens.len() && tokens[position] == "^^" {
                        position += 1; // Skip "^^"
                        if position >= tokens.len() {
                            return Err(Error::ontology_parsing(
                                "Expected datatype after ^^".to_string(),
                            ));
                        }
                        let datatype_iri = self.expand_iri(&tokens[position], prefixes)?;
                        position += 1;

                        crate::ontology::Literal {
                            value: literal_token.trim_matches('"').to_string(),
                            language: None,
                            datatype: Some(url::Url::parse(&datatype_iri).map_err(|e| {
                                Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                            })?),
                        }
                    } else {
                        // Untyped literal
                        crate::ontology::Literal {
                            value: literal_token.trim_matches('"').to_string(),
                            language: None,
                            datatype: None,
                        }
                    };

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataHasValue {
                            property,
                            value: literal,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataHasValue".to_string(),
                    ))
                }
            }
            "DataSomeValuesFrom" => {
                position += 1; // Skip "DataSomeValuesFrom"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property after DataSomeValuesFrom(".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse data range using the helper function
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data range in DataSomeValuesFrom".to_string(),
                        ));
                    }

                    let (filler, new_pos) = self.parse_data_range(tokens, position, prefixes)?;
                    position = new_pos;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataSomeValuesFrom { property, filler },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataSomeValuesFrom".to_string(),
                    ))
                }
            }
            "DataAllValuesFrom" => {
                position += 1; // Skip "DataAllValuesFrom"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property after DataAllValuesFrom(".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse data range using the helper function
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data range in DataAllValuesFrom".to_string(),
                        ));
                    }

                    let (filler, new_pos) = self.parse_data_range(tokens, position, prefixes)?;
                    position = new_pos;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataAllValuesFrom { property, filler },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataAllValuesFrom".to_string(),
                    ))
                }
            }
            "ObjectOneOf" => {
                position += 1; // Skip "ObjectOneOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut individuals = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let individual_iri = self.expand_iri(&tokens[position], prefixes)?;
                        let individual =
                            crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                                iri: url::Url::parse(&individual_iri)
                                    .map_err(|e| {
                                        Error::ontology_parsing(format!(
                                            "Invalid individual IRI: {e}"
                                        ))
                                    })?
                                    .into(),
                            });
                        individuals.push(individual);
                        position += 1;
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    Ok((ClassExpression::ObjectOneOf(individuals), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectOneOf".to_string(),
                    ))
                }
            }
            "ObjectIntersectionOf" => {
                position += 1; // Skip "ObjectIntersectionOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut expressions = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let (expr, new_pos) =
                            self.parse_class_expression(tokens, position, prefixes)?;
                        expressions.push(expr);
                        position = new_pos;
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((ClassExpression::ObjectIntersectionOf(expressions), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectIntersectionOf".to_string(),
                    ))
                }
            }
            "ObjectUnionOf" => {
                position += 1; // Skip "ObjectUnionOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let mut expressions = Vec::new();
                    while position < tokens.len() && tokens[position] != ")" {
                        let (expr, new_pos) =
                            self.parse_class_expression(tokens, position, prefixes)?;
                        expressions.push(expr);
                        position = new_pos;
                    }

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((ClassExpression::ObjectUnionOf(expressions), position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectUnionOf".to_string(),
                    ))
                }
            }
            "ObjectComplementOf" => {
                position += 1; // Skip "ObjectComplementOf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    let (expr, new_pos) =
                        self.parse_class_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }

                    Ok((
                        ClassExpression::ObjectComplementOf(Box::new(expr)),
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectComplementOf".to_string(),
                    ))
                }
            }
            "ObjectSomeValuesFrom" => {
                position += 1; // Skip "ObjectSomeValuesFrom"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse object property expression using helper function
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property after ObjectSomeValuesFrom(".to_string(),
                        ));
                    }

                    let (property, new_pos) = self.parse_object_property_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    // Parse filler class expression
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected class expression in ObjectSomeValuesFrom".to_string(),
                        ));
                    }
                    let (filler, new_pos) =
                        self.parse_class_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    Ok((
                        ClassExpression::ObjectSomeValuesFrom {
                            property,
                            filler: Box::new(filler),
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectSomeValuesFrom".to_string(),
                    ))
                }
            }
            "ObjectAllValuesFrom" => {
                position += 1; // Skip "ObjectAllValuesFrom"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse object property expression using helper function
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property after ObjectAllValuesFrom(".to_string(),
                        ));
                    }

                    let (property, new_pos) = self.parse_object_property_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    // Parse filler class expression
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected class expression in ObjectAllValuesFrom".to_string(),
                        ));
                    }
                    let (filler, new_pos) =
                        self.parse_class_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    Ok((
                        ClassExpression::ObjectAllValuesFrom {
                            property,
                            filler: Box::new(filler),
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectAllValuesFrom".to_string(),
                    ))
                }
            }
            "ObjectMinCardinality" => {
                position += 1; // Skip "ObjectMinCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after ObjectMinCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse object property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property in ObjectMinCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse optional filler class expression
                    // If the next token is ")", filler is omitted and defaults to owl:Thing
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        let (expr, new_pos) =
                            self.parse_class_expression(tokens, position, prefixes)?;
                        position = new_pos;
                        expr
                    } else {
                        // Default filler is owl:Thing
                        ClassExpression::Class(crate::ontology::Class {
                            iri: url::Url::parse("http://www.w3.org/2002/07/owl#Thing")
                                .expect("Failed to parse owl:Thing IRI")
                                .into(),
                        })
                    };

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::ObjectMinCardinality {
                            property,
                            cardinality,
                            filler: Box::new(filler),
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectMinCardinality".to_string(),
                    ))
                }
            }
            "ObjectMaxCardinality" => {
                position += 1; // Skip "ObjectMaxCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after ObjectMaxCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse object property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property in ObjectMaxCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse optional filler class expression
                    // If the next token is ")", filler is omitted and defaults to owl:Thing
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        let (expr, new_pos) =
                            self.parse_class_expression(tokens, position, prefixes)?;
                        position = new_pos;
                        expr
                    } else {
                        // Default filler is owl:Thing
                        ClassExpression::Class(crate::ontology::Class {
                            iri: url::Url::parse("http://www.w3.org/2002/07/owl#Thing")
                                .expect("Failed to parse owl:Thing IRI")
                                .into(),
                        })
                    };

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::ObjectMaxCardinality {
                            property,
                            cardinality,
                            filler: Box::new(filler),
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectMaxCardinality".to_string(),
                    ))
                }
            }
            "ObjectExactCardinality" => {
                position += 1; // Skip "ObjectExactCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after ObjectExactCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse object property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property in ObjectExactCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse optional filler class expression
                    // If the next token is ")", filler is omitted and defaults to owl:Thing
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        let (expr, new_pos) =
                            self.parse_class_expression(tokens, position, prefixes)?;
                        position = new_pos;
                        expr
                    } else {
                        // Default filler is owl:Thing
                        ClassExpression::Class(crate::ontology::Class {
                            iri: url::Url::parse("http://www.w3.org/2002/07/owl#Thing")
                                .expect("Failed to parse owl:Thing IRI")
                                .into(),
                        })
                    };

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::ObjectExactCardinality {
                            property,
                            cardinality,
                            filler: Box::new(filler),
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectExactCardinality".to_string(),
                    ))
                }
            }
            "ObjectHasValue" => {
                position += 1; // Skip "ObjectHasValue"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse object property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property after ObjectHasValue(".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Parse individual
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected individual in ObjectHasValue".to_string(),
                        ));
                    }
                    let individual_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    let individual =
                        crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                            iri: url::Url::parse(&individual_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid individual IRI: {e}"))
                                })?
                                .into(),
                        });

                    Ok((
                        ClassExpression::ObjectHasValue {
                            property,
                            value: individual,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectHasValue".to_string(),
                    ))
                }
            }
            "ObjectHasSelf" => {
                position += 1; // Skip "ObjectHasSelf"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse object property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected object property after ObjectHasSelf(".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Skip closing ")"
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((ClassExpression::ObjectHasSelf { property }, position))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after ObjectHasSelf".to_string(),
                    ))
                }
            }
            "DataMinCardinality" => {
                position += 1; // Skip "DataMinCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after DataMinCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property in DataMinCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Skip closing ")" or parse optional data range filler
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        // Has a data range filler
                        let datarange_iri = self.expand_iri(&tokens[position], prefixes)?;
                        position += 1;
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse(&datarange_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?
                                .into(),
                        )
                    } else {
                        // No filler, use rdfs:Literal as default
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#Literal")
                                .expect("Failed to complete operation successfully")
                                .into(),
                        )
                    };

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataMinCardinality {
                            property,
                            cardinality,
                            filler,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataMinCardinality".to_string(),
                    ))
                }
            }
            "DataMaxCardinality" => {
                position += 1; // Skip "DataMaxCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after DataMaxCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property in DataMaxCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Skip closing ")" or parse optional data range filler
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        // Has a data range filler
                        let datarange_iri = self.expand_iri(&tokens[position], prefixes)?;
                        position += 1;
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse(&datarange_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?
                                .into(),
                        )
                    } else {
                        // No filler, use rdfs:Literal as default
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#Literal")
                                .expect("Failed to complete operation successfully")
                                .into(),
                        )
                    };

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataMaxCardinality {
                            property,
                            cardinality,
                            filler,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataMaxCardinality".to_string(),
                    ))
                }
            }
            "DataExactCardinality" => {
                position += 1; // Skip "DataExactCardinality"
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("

                    // Parse cardinality number
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected cardinality number after DataExactCardinality(".to_string(),
                        ));
                    }
                    let cardinality: u32 = tokens[position].parse().map_err(|e| {
                        Error::ontology_parsing(format!("Invalid cardinality number: {e}"))
                    })?;
                    position += 1;

                    // Parse data property
                    if position >= tokens.len() {
                        return Err(Error::ontology_parsing(
                            "Expected data property in DataExactCardinality".to_string(),
                        ));
                    }
                    let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                    position += 1;

                    // Skip closing ")" or parse optional data range filler
                    let filler = if position < tokens.len() && tokens[position] != ")" {
                        // Has a data range filler
                        let datarange_iri = self.expand_iri(&tokens[position], prefixes)?;
                        position += 1;
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse(&datarange_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid datatype IRI: {e}"))
                                })?
                                .into(),
                        )
                    } else {
                        // No filler, use rdfs:Literal as default
                        crate::ontology::DataRange::Datatype(
                            url::Url::parse("http://www.w3.org/2000/01/rdf-schema#Literal")
                                .expect("Failed to complete operation successfully")
                                .into(),
                        )
                    };

                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1;
                    }

                    let property = crate::ontology::DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: url::Url::parse(&property_iri)
                                .map_err(|e| {
                                    Error::ontology_parsing(format!("Invalid property IRI: {e}"))
                                })?
                                .into(),
                        },
                    );

                    Ok((
                        ClassExpression::DataExactCardinality {
                            property,
                            cardinality,
                            filler,
                        },
                        position,
                    ))
                } else {
                    Err(Error::ontology_parsing(
                        "Expected '(' after DataExactCardinality".to_string(),
                    ))
                }
            }
            _ => {
                // Check if this is a structural token (parentheses) or OWL keyword
                if is_structural_token(token) {
                    return Err(Error::ontology_parsing(format!(
                        "OWL keyword '{}' cannot be used as a class name. Expected a class IRI or class expression.",
                        token
                    )));
                }

                // Default: treat as a named class (IRI)
                let class_iri = self.expand_iri(token, prefixes)?;
                let class = crate::ontology::Class {
                    iri: url::Url::parse(&class_iri)
                        .map_err(|e| {
                            Error::ontology_parsing(format!(
                                "Invalid class IRI '{}': {}",
                                class_iri, e
                            ))
                        })?
                        .into(),
                };
                Ok((ClassExpression::Class(class), position + 1))
            }
        }
    }

    /// Parse `SubClassOf` axiom: `SubClassOf`(<subclass> <superclass>)
    fn parse_subclass_of(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
        _base_iri: &Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "SubClassOf"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse subclass expression
                let (subclass, new_pos) =
                    self.parse_class_expression(tokens, position, prefixes)?;
                position = new_pos;

                // Parse superclass expression
                if position < tokens.len() {
                    let (superclass, new_pos) =
                        self.parse_class_expression(tokens, position, prefixes)?;
                    position = new_pos;

                    let axiom = crate::ontology::SubClassOfAxiom {
                        id: generate_axiom_id(),
                        subclass,
                        superclass,
                        annotations: vec![],
                    };
                    ontology.add_axiom(crate::ontology::Axiom::SubClassOf(axiom));
                }
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Parse `ClassAssertion` axiom: `ClassAssertion`(<class> <individual>)
    fn parse_class_assertion(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
        _base_iri: &Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "ClassAssertion"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse class expression (can be simple IRI or complex expression like ObjectSomeValuesFrom)
                let (class_expr, new_pos) =
                    self.parse_class_expression(tokens, position, prefixes)?;
                position = new_pos;

                // Parse individual IRI
                if position >= tokens.len() {
                    return Err(Error::ontology_parsing(
                        "Expected individual IRI in ClassAssertion".to_string(),
                    ));
                }
                let individual_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let individual =
                    crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&individual_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    });

                let axiom = crate::ontology::ClassAssertionAxiom {
                    id: generate_axiom_id(),
                    class: class_expr,
                    individual,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::ClassAssertion(axiom));
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Parse `DisjointClasses` axiom: `DisjointClasses`(<class1> <class2> ...)
    fn parse_disjoint_classes(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
        _base_iri: &Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "DisjointClasses"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            let mut classes = Vec::new();
            while position < tokens.len() && tokens[position] != ")" {
                // Parse class expression (could be simple class or complex expression)
                let (class_expr, new_pos) =
                    self.parse_class_expression(tokens, position, prefixes)?;
                classes.push(class_expr);
                position = new_pos;
            }

            if classes.len() >= 2 {
                let axiom = crate::ontology::DisjointClassesAxiom {
                    id: generate_axiom_id(),
                    classes,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::DisjointClasses(axiom));
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Parse `EquivalentClasses` axiom: `EquivalentClasses`(<class1> <class2> ...)
    fn parse_equivalent_classes(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
        _base_iri: &Option<String>,
    ) -> Result<usize> {
        position += 1; // Skip "EquivalentClasses"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            let mut classes = Vec::new();
            while position < tokens.len() && tokens[position] != ")" {
                // Parse class expression (could be simple class or complex expression)
                let (class_expr, new_pos) =
                    self.parse_class_expression(tokens, position, prefixes)?;
                classes.push(class_expr);
                position = new_pos;
            }

            if classes.len() >= 2 {
                let axiom = crate::ontology::EquivalentClassesAxiom {
                    id: generate_axiom_id(),
                    classes,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::EquivalentClasses(axiom));
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Parse `ObjectPropertyAssertion`: `ObjectPropertyAssertion`(<prop> <subj> <obj>)
    fn parse_object_property_assertion(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "ObjectPropertyAssertion"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position + 2 < tokens.len() {
                let prop_iri = self.expand_iri(&tokens[position], prefixes)?;
                let subj_iri = self.expand_iri(&tokens[position + 1], prefixes)?;
                let obj_iri = self.expand_iri(&tokens[position + 2], prefixes)?;

                let property = crate::ontology::ObjectProperty {
                    iri: crate::ontology::IRI::new(&prop_iri),
                };
                let subject =
                    crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                        iri: crate::ontology::IRI::new(&subj_iri),
                    });
                let object = crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                    iri: crate::ontology::IRI::new(&obj_iri),
                });

                let axiom = crate::ontology::ObjectPropertyAssertionAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    source: subject,
                    target: object,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::ObjectPropertyAssertion(axiom));

                position += 3;
            }

            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }

        Ok(position)
    }

    /// Expand a prefixed IRI
    #[inline(always)]
    fn expand_iri(
        &self,
        iri: &str,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        // Full IRIs in angle brackets can contain any text, including keywords
        if iri.starts_with('<') && iri.ends_with('>') {
            // Already a full IRI - extract content without validation
            return Ok(iri[1..iri.len() - 1].to_string());
        }

        // Handle blank nodes (_:nodeID)
        if iri.starts_with("_:") {
            // Blank nodes are represented as-is in functional syntax
            // They are local identifiers within the ontology
            // Convert to a unique IRI in the blank node namespace
            return Ok(format!("http://www.w3.org/2002/07/owl#blank{}", &iri[2..]));
        }

        // Validate that non-bracketed tokens are not structural tokens or OWL keywords
        if is_structural_token(iri) {
            return Err(Error::ontology_parsing(format!(
                "OWL keyword '{}' cannot be used as an IRI. This may indicate a malformed construct or empty element.",
                iri
            )));
        }

        if iri.starts_with(':') {
            // Relative IRI with default prefix (e.g., ":Employee")
            let local = &iri[1..];
            if let Some(base) = prefixes.get("") {
                // Empty string key is the default prefix from ontology IRI
                Ok(format!("{}{}", base, local))
            } else {
                // No base IRI defined, return as-is but this will likely fail validation
                Err(Error::ontology_parsing(format!(
                    "Relative IRI '{}' found but no base ontology IRI is defined. \
                     Relative IRIs require an ontology header like: Ontology(<http://example.org/> ...)",
                    iri
                )))
            }
        } else if let Some(colon_pos) = iri.find(':') {
            // Prefixed IRI (e.g., "ex:Person")
            let prefix = &iri[..colon_pos];
            let local = &iri[colon_pos + 1..];

            if let Some(base) = prefixes.get(prefix) {
                let expanded = format!("{}{}", base, local);
                // Validate the expanded IRI can be parsed as a URL
                if url::Url::parse(&expanded).is_err() {
                    return Err(Error::ontology_parsing(format!(
                        "Invalid IRI: relative URL without a base. Original: '{}', Expanded: '{}', Available prefixes: {:?}",
                        iri, expanded, prefixes
                    )));
                }
                Ok(expanded)
            } else {
                // Prefix not found - if it looks like it might be a URL scheme, return as-is
                // Otherwise, it's an error (e.g., undefined prefix)
                if prefix
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
                    && local.starts_with("//")
                {
                    // Looks like a URL with scheme (e.g., http://, https://, ftp://)
                    Ok(iri.to_string())
                } else {
                    // Undefined prefix - create a more informative error
                    Err(Error::ontology_parsing(format!(
                        "Undefined prefix '{}' in IRI '{}'. Available prefixes: {:?}",
                        prefix,
                        iri,
                        prefixes.keys().collect::<Vec<_>>()
                    )))
                }
            }
        } else {
            // No colon found - this is a relative IRI without a prefix
            // This should have a default base IRI to resolve against
            Err(Error::ontology_parsing(format!(
                "Relative IRI '{}' without a prefix or base IRI. Available prefixes: {:?}",
                iri,
                prefixes.keys().collect::<Vec<_>>()
            )))
        }
    }

    /// Parse `HasKey` axiom: `HasKey`(<class> (<object_properties>) (<data_properties>))
    fn parse_has_key(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "HasKey"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse class
                let class_iri = self.expand_iri(&tokens[position], prefixes)?;
                let class = crate::ontology::Class {
                    iri: url::Url::parse(&class_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                        .into(),
                };
                position += 1;

                // Parse object properties list
                let mut object_properties = Vec::new();
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("
                    while position < tokens.len() && tokens[position] != ")" {
                        let prop_iri = self.expand_iri(&tokens[position], prefixes)?;
                        let object_prop = crate::ontology::ObjectProperty {
                            iri: url::Url::parse(&prop_iri)
                                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                                .into(),
                        };
                        object_properties.push(
                            crate::ontology::ObjectPropertyExpression::ObjectProperty(object_prop),
                        );
                        position += 1;
                    }
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }
                }

                // Parse data properties list
                let mut data_properties = Vec::new();
                if position < tokens.len() && tokens[position] == "(" {
                    position += 1; // Skip "("
                    while position < tokens.len() && tokens[position] != ")" {
                        let prop_iri = self.expand_iri(&tokens[position], prefixes)?;
                        let data_prop = crate::ontology::DataProperty {
                            iri: url::Url::parse(&prop_iri)
                                .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                                .into(),
                        };
                        data_properties.push(
                            crate::ontology::DataPropertyExpression::DataProperty(data_prop),
                        );
                        position += 1;
                    }
                    if position < tokens.len() && tokens[position] == ")" {
                        position += 1; // Skip ")"
                    }
                }

                let axiom = crate::ontology::HasKeyAxiom {
                    id: generate_axiom_id(),
                    class: ClassExpression::Class(class),
                    object_properties,
                    data_properties,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::HasKey(axiom));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }

    /// Parse TransitiveObjectProperty axiom
    fn parse_transitive_object_property(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut crate::ontology::Ontology,
        prefixes: &HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "TransitiveObjectProperty"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse property IRI
                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid property IRI: {e}")))?
                        .into(),
                };

                let axiom = crate::ontology::TransitiveObjectPropertyAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::TransitiveObjectProperty(axiom));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }

    /// Parse SymmetricObjectProperty axiom
    fn parse_symmetric_object_property(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut crate::ontology::Ontology,
        prefixes: &HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "SymmetricObjectProperty"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse property IRI
                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid property IRI: {e}")))?
                        .into(),
                };

                let axiom = crate::ontology::SymmetricObjectPropertyAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::SymmetricObjectProperty(axiom));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }

    /// Parse ReflexiveObjectProperty axiom
    fn parse_reflexive_object_property(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut crate::ontology::Ontology,
        prefixes: &HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "ReflexiveObjectProperty"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse property IRI
                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid property IRI: {e}")))?
                        .into(),
                };

                let axiom = crate::ontology::ReflexiveObjectPropertyAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::ReflexiveObjectProperty(axiom));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }

    /// Parse FunctionalObjectProperty axiom
    fn parse_functional_object_property(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut crate::ontology::Ontology,
        prefixes: &HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "FunctionalObjectProperty"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse property IRI
                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid property IRI: {e}")))?
                        .into(),
                };

                let axiom = crate::ontology::FunctionalObjectPropertyAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::FunctionalObjectProperty(axiom));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }

    /// Parse InverseFunctionalObjectProperty axiom
    fn parse_inverse_functional_object_property(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut crate::ontology::Ontology,
        prefixes: &HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "InverseFunctionalObjectProperty"

        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            // Skip any Annotation(...) sequences
            position = self.skip_annotations(tokens, position);

            if position < tokens.len() {
                // Parse property IRI
                let property_iri = self.expand_iri(&tokens[position], prefixes)?;
                position += 1;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&property_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid property IRI: {e}")))?
                        .into(),
                };

                let axiom = crate::ontology::InverseFunctionalObjectPropertyAxiom {
                    id: generate_axiom_id(),
                    property: crate::ontology::ObjectPropertyExpression::ObjectProperty(property),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::InverseFunctionalObjectProperty(
                    axiom,
                ));

                if position < tokens.len() && tokens[position] == ")" {
                    position += 1; // Skip ")"
                }
            }
        }

        Ok(position)
    }
}

/// Parse Functional Syntax from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path).map_err(|e| Error::io(format!("Failed to open file: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse(&content)
}

/// Save ontology to Functional Syntax file (using common infrastructure)
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = FunctionalSyntaxSerializer::new();
    serializer.serialize_to_file(ontology, path)
}

fn serialize_axiom(axiom: &crate::ontology::Axiom) -> String {
    match axiom {
        crate::ontology::Axiom::SubClassOf(sub) => {
            format!(
                "SubClassOf({} {})",
                serialize_class_expression(&sub.subclass),
                serialize_class_expression(&sub.superclass)
            )
        }
        crate::ontology::Axiom::ClassAssertion(ca) => {
            format!(
                "ClassAssertion({} {})",
                serialize_class_expression(&ca.class),
                serialize_individual(&ca.individual)
            )
        }
        crate::ontology::Axiom::Declaration(decl) => {
            format!("Declaration({})", serialize_entity(&decl.entity))
        }
        _ => format!("# Unsupported axiom type: {:?}", axiom),
    }
}

fn serialize_class_expression(ce: &crate::ontology::ClassExpression) -> String {
    match ce {
        crate::ontology::ClassExpression::Class(class) => format!("<{}>", class.iri),
        crate::ontology::ClassExpression::ObjectIntersectionOf(classes) => {
            let class_strs: Vec<String> = classes.iter().map(serialize_class_expression).collect();
            format!("ObjectIntersectionOf({})", class_strs.join(" "))
        }
        crate::ontology::ClassExpression::ObjectUnionOf(classes) => {
            let class_strs: Vec<String> = classes.iter().map(serialize_class_expression).collect();
            format!("ObjectUnionOf({})", class_strs.join(" "))
        }
        _ => format!("# Unsupported class expression: {:?}", ce),
    }
}

fn serialize_individual(ind: &crate::ontology::Individual) -> String {
    format!(
        "<{}>",
        ind.iri().map(|iri| iri.as_str()).unwrap_or("_:anonymous")
    )
}

fn serialize_entity(entity: &crate::ontology::Entity) -> String {
    match entity {
        crate::ontology::Entity::Class(class) => format!("Class(<{}>)", class.as_str()),
        crate::ontology::Entity::ObjectProperty(prop) => {
            format!("ObjectProperty(<{}>)", prop.as_str())
        }
        crate::ontology::Entity::DataProperty(prop) => format!("DataProperty(<{}>)", prop.as_str()),
        crate::ontology::Entity::NamedIndividual(ind) => {
            format!("NamedIndividual(<{}>)", ind.as_str())
        }
        crate::ontology::Entity::Datatype(dt) => format!("Datatype(<{}>)", dt.as_str()),
        crate::ontology::Entity::AnnotationProperty(ap) => {
            format!("AnnotationProperty(<{}>)", ap.as_str())
        }
    }
}

fn serialize_annotation_property(prop: &crate::ontology::AnnotationProperty) -> String {
    format!("<{}>", prop.iri)
}

fn serialize_annotation_value(value: &crate::ontology::AnnotationValue) -> String {
    match value {
        crate::ontology::AnnotationValue::IRI(iri) => format!("<{}>", iri),
        crate::ontology::AnnotationValue::Literal(lit) => {
            if let Some(datatype) = &lit.datatype {
                format!("\"{}\"^^<{}>", lit.value, datatype.as_str())
            } else if let Some(lang) = &lit.language {
                format!("\"{}\"@{}", lit.value, lang)
            } else {
                format!("\"{}\"", lit.value)
            }
        }
        crate::ontology::AnnotationValue::AnonymousIndividual(anon) => {
            format!("_:{}", anon.id)
        }
    }
}

/// Functional Syntax Serializer
#[derive(Debug, Clone, Default)]
pub struct FunctionalSyntaxSerializer;

impl FunctionalSyntaxSerializer {
    /// Create a new functional syntax serializer
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl OntologySerializer for FunctionalSyntaxSerializer {
    fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut content = String::new();

        // Write ontology header
        if let Some(onto_iri) = ontology.get_iri() {
            if let Some(version_iri) = &ontology.version_iri {
                content.push_str(&format!("Ontology(<{}> <{}>\n", onto_iri, version_iri));
            } else {
                content.push_str(&format!("Ontology(<{}>\n", onto_iri));
            }
        } else {
            content.push_str("Ontology(\n");
        }

        // Write imports
        for import in &ontology.imports {
            content.push_str(&format!("  Import(<{}>)\n", import));
        }

        // Write annotations
        for annotation in &ontology.annotations {
            content.push_str(&format!(
                "  Annotation({} {})\n",
                serialize_annotation_property(&annotation.property),
                serialize_annotation_value(&annotation.value)
            ));
        }

        // Write axioms
        for axiom in ontology.axioms() {
            content.push_str(&format!("  {}\n", serialize_axiom(axiom)));
        }

        content.push_str(")\n");
        Ok(content)
    }
}
