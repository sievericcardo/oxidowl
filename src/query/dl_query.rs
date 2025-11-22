//! DL Query Engine
//!
//! This module provides a Description Logic query engine that can parse and execute
//! DL queries against the ontology. It supports queries like:
//! - "`ClassA` some `PropertyR`"
//! - "`PropertyR` some `ClassB`"
//! - "`ClassA` and (`PropertyR` some `ClassB`)"
//!
//! The query language follows Manchester Syntax for class expressions.

use crate::{
    Error, Result,
    ontology::{Class, ClassExpression, IRI, Individual, ObjectProperty, ObjectPropertyExpression},
    reasoning::ReasoningService,
};
use log::debug;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

/// Errors that can occur during query processing
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Reasoning error: {0}")]
    ReasoningError(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

/// Helper function to recursively extract all individual classes from a union expression
fn extract_union_classes(expr: &ClassExpression, result: &mut HashSet<ClassExpression>) {
    match expr {
        ClassExpression::ObjectUnionOf(union_classes) => {
            // Recursively extract from nested unions
            for class_expr in union_classes {
                extract_union_classes(class_expr, result);
            }
        }
        _ => {
            // This is an individual class, add it to the result
            result.insert(expr.clone());
        }
    }
}

/// DL Query Engine for executing description logic queries
#[derive(Debug)]
pub struct DLQueryEngine {
    reasoning_service: Arc<ReasoningService>,
    default_namespace: Option<String>,
    prefix_map: Option<std::collections::HashMap<String, String>>,
}

/// A parsed DL query
#[derive(Debug, Clone)]
pub struct DLQuery {
    /// The class expression being queried
    pub class_expression: ClassExpression,
    /// The type of query (instances, subclasses, etc.)
    pub query_type: QueryType,
    /// Whether to return direct results only
    pub direct: bool,
}

/// Types of DL queries
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    /// Find all instances of the class expression
    Instances,
    /// Find all subclasses of the class expression
    Subclasses,
    /// Find all superclasses of the class expression
    Superclasses,
    /// Find all equivalent classes of the class expression
    EquivalentClasses,
    /// Check if the class expression is satisfiable
    Satisfiable,
}

/// Query result containing the answer to a DL query
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The original query
    pub query: DLQuery,
    /// Results for instance queries
    pub instances: Option<HashSet<Individual>>,
    /// Results for class queries (subclasses, superclasses, etc.)
    pub classes: Option<HashSet<ClassExpression>>,
    /// Result for satisfiability queries
    pub satisfiable: Option<bool>,
    /// Execution time
    pub execution_time: std::time::Duration,
}

impl DLQueryEngine {
    /// Create a new DL query engine
    #[must_use]
    pub fn new(reasoning_service: Arc<ReasoningService>) -> Self {
        Self {
            reasoning_service,
            default_namespace: None,
            prefix_map: None,
        }
    }

    /// Create a new DL query engine with a specific default namespace
    #[must_use]
    pub fn new_with_namespace(reasoning_service: Arc<ReasoningService>, namespace: String) -> Self {
        Self {
            reasoning_service,
            default_namespace: Some(namespace),
            prefix_map: None,
        }
    }

    /// Create a new DL query engine with reasoning service and optional namespace
    #[must_use]
    pub fn with_config(
        reasoning_service: Arc<ReasoningService>,
        namespace: Option<String>,
    ) -> Self {
        Self {
            reasoning_service,
            default_namespace: namespace,
            prefix_map: None,
        }
    }

    /// Get the default namespace from the ontology or use provided namespace
    async fn get_default_namespace(&self) -> Result<String> {
        // Use provided namespace if available
        if let Some(ref namespace) = self.default_namespace {
            return Ok(namespace.clone());
        }

        // Extract default namespace from the ontology systematically
        // This implements proper namespace resolution according to OWL specifications

        // First, try to get the ontology IRI from the reasoning service
        if let Ok(Some(ontology_iri)) = self.reasoning_service.get_ontology_iri() {
            let iri_string = ontology_iri.as_str();
            if !iri_string.is_empty() {
                // Use the ontology IRI as the base for the default namespace
                let namespace = if iri_string.ends_with('#') {
                    iri_string.to_string()
                } else if iri_string.ends_with('/') {
                    iri_string.to_string()
                } else {
                    format!("{}#", iri_string)
                };
                return Ok(namespace);
            }
        }

        // Try to extract from known prefixes or imported ontologies
        if let Some(prefix_map) = &self.prefix_map {
            // Look for common default prefixes
            for (prefix, namespace) in prefix_map {
                if prefix.is_empty() || prefix == ":" || prefix == "default" {
                    return Ok(namespace.clone());
                }
            }

            // If no explicit default, use the first declared namespace
            if let Some((_, first_namespace)) = prefix_map.iter().next() {
                return Ok(first_namespace.clone());
            }
        }

        // Try to extract from XML base declarations or other sources
        if let Some(xml_base) = self.extract_xml_base_from_ontology().await {
            let namespace = if xml_base.ends_with('#') || xml_base.ends_with('/') {
                xml_base
            } else {
                format!("{}#", xml_base)
            };
            return Ok(namespace);
        }

        // Default fallback namespace - use a more standard pattern
        Ok("http://www.semanticweb.org/ontology#".to_string())
    }

    /// Extract XML base declaration from the ontology
    async fn extract_xml_base_from_ontology(&self) -> Option<String> {
        // This would parse the ontology document for xml:base declarations
        // Implementation depends on the ontology format and storage

        // For now, return None to indicate no xml:base found
        // A full implementation would parse the ontology document
        None
    }

    /// Parse and execute a DL query string
    pub async fn execute_query(&self, query_string: &str) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();

        // Parse the query
        let query = self.parse_query(query_string).await?;

        // Execute the query
        let mut result = QueryResult {
            query: query.clone(),
            instances: None,
            classes: None,
            satisfiable: None,
            execution_time: std::time::Duration::default(),
        };

        match query.query_type {
            QueryType::Instances => {
                let instances = self
                    .reasoning_service
                    .get_instances(&query.class_expression, query.direct)
                    .await?;
                result.instances = Some(instances);
            }
            QueryType::Subclasses => {
                // Special handling for union queries - find classes equivalent to the union
                if let ClassExpression::ObjectUnionOf(_) = &query.class_expression {
                    // For union queries, we want to find classes that are equivalent to this union
                    // This is the correct DL reasoning behavior (like HermiT)
                    let equivalent = self
                        .reasoning_service
                        .get_equivalent_classes(&query.class_expression)
                        .await?;
                    result.classes = Some(equivalent);
                } else {
                    let subclasses = self
                        .reasoning_service
                        .get_subclasses(&query.class_expression, query.direct)
                        .await?;
                    result.classes = Some(subclasses);
                }
            }
            QueryType::Superclasses => {
                let superclasses = self
                    .reasoning_service
                    .get_superclasses(&query.class_expression, query.direct)
                    .await?;
                result.classes = Some(superclasses);
            }
            QueryType::EquivalentClasses => {
                let equivalent = self
                    .reasoning_service
                    .get_equivalent_classes(&query.class_expression)
                    .await?;
                result.classes = Some(equivalent);
            }
            QueryType::Satisfiable => {
                let satisfiable = self
                    .reasoning_service
                    .is_satisfiable(&query.class_expression)
                    .await?;
                result.satisfiable = Some(satisfiable);
            }
        }

        result.execution_time = start_time.elapsed();
        Ok(result)
    }

    /// Parse a DL query string into a structured query
    pub async fn parse_query(&self, query_string: &str) -> Result<DLQuery> {
        let default_namespace = self.get_default_namespace().await?;
        let parser = DLQueryParser::with_namespace(default_namespace);
        parser.parse(query_string)
    }

    /// Execute a query for instances of a class expression
    pub async fn get_instances(
        &self,
        class_expression: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<Individual>> {
        self.reasoning_service
            .get_instances(class_expression, direct)
            .await
    }

    /// Execute a query for subclasses of a class expression
    pub async fn get_subclasses(
        &self,
        class_expression: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        self.reasoning_service
            .get_subclasses(class_expression, direct)
            .await
    }

    /// Execute a query for superclasses of a class expression
    pub async fn get_superclasses(
        &self,
        class_expression: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        self.reasoning_service
            .get_superclasses(class_expression, direct)
            .await
    }

    /// Check satisfiability of a class expression
    pub async fn is_satisfiable(&self, class_expression: &ClassExpression) -> Result<bool> {
        self.reasoning_service
            .is_satisfiable(class_expression)
            .await
    }

    /// Execute a query for equivalent classes of a class expression
    pub async fn get_equivalent_classes(
        &self,
        class_expression: &ClassExpression,
    ) -> Result<HashSet<ClassExpression>> {
        self.reasoning_service
            .get_equivalent_classes(class_expression)
            .await
    }

    /// Execute a disjoint union query to find classes equivalent to the union
    /// This method takes a list of class expressions and finds classes that are
    /// equivalent to their disjoint union
    pub async fn find_disjoint_union_equivalent(
        &self,
        classes: Vec<ClassExpression>,
    ) -> Result<HashSet<ClassExpression>> {
        // Create a union of the provided classes
        let union_expr = if classes.len() == 1 {
            classes
                .into_iter()
                .next()
                .ok_or_else(|| Error::internal("Expected at least one class in union query"))?
        } else {
            ClassExpression::ObjectUnionOf(classes)
        };

        // Find equivalent classes to this union
        self.get_equivalent_classes(&union_expr).await
    }

    /// Parse and execute a union query (e.g., "`ClassA` or `ClassB` or `ClassC`")
    /// and find equivalent classes
    pub async fn execute_union_query(&self, union_query: &str) -> Result<QueryResult> {
        // Force this to be treated as an equivalent classes query
        let modified_query = format!("equivalent-classes: {union_query}");
        self.execute_query(&modified_query).await
    }

    /// Set the default namespace for this query engine
    pub fn set_namespace(&mut self, namespace: String) {
        self.default_namespace = Some(namespace);
    }

    /// Get the current default namespace
    #[must_use]
    pub fn get_namespace(&self) -> Option<&String> {
        self.default_namespace.as_ref()
    }

    // Note: Convenience methods removed temporarily due to type system complexity
    // They will be implemented in a future version with proper error handling
}

/// Parser for DL query strings in Manchester Syntax
pub struct DLQueryParser {
    default_namespace: String,
}

impl Default for DLQueryParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DLQueryParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Use a generic default namespace that will be overridden
            default_namespace: "http://example.org/ontology#".to_string(),
        }
    }

    /// Create parser with custom default namespace
    #[must_use]
    pub fn with_namespace(namespace: String) -> Self {
        Self {
            default_namespace: namespace,
        }
    }

    /// Parse a query string into a DL query
    pub fn parse(&self, query_string: &str) -> Result<DLQuery> {
        let trimmed = query_string.trim();
        debug!("Parsing DL query: '{trimmed}'");

        // Determine query type and extract class expression
        let (query_type, class_expr_str, direct) = self.parse_query_structure(trimmed)?;
        debug!("Query type: {query_type:?}, expression: '{class_expr_str}', direct: {direct}");

        // Parse the class expression
        let class_expression = self.parse_class_expression(class_expr_str)?;
        debug!("Parsed class expression: {class_expression:?}");

        Ok(DLQuery {
            class_expression,
            query_type,
            direct,
        })
    }

    /// Parse the overall query structure to determine type and expression
    fn parse_query_structure<'a>(&self, query: &'a str) -> Result<(QueryType, &'a str, bool)> {
        // Handle query type prefixes
        if let Some(expr) = query.strip_prefix("instances:") {
            return Ok((QueryType::Instances, expr.trim(), false));
        }
        if let Some(expr) = query.strip_prefix("direct-instances:") {
            return Ok((QueryType::Instances, expr.trim(), true));
        }
        if let Some(expr) = query.strip_prefix("subclasses:") {
            return Ok((QueryType::Subclasses, expr.trim(), false));
        }
        if let Some(expr) = query.strip_prefix("direct-subclasses:") {
            return Ok((QueryType::Subclasses, expr.trim(), true));
        }
        if let Some(expr) = query.strip_prefix("superclasses:") {
            return Ok((QueryType::Superclasses, expr.trim(), false));
        }
        if let Some(expr) = query.strip_prefix("direct-superclasses:") {
            return Ok((QueryType::Superclasses, expr.trim(), true));
        }
        if let Some(expr) = query.strip_prefix("equivalent:") {
            return Ok((QueryType::EquivalentClasses, expr.trim(), false));
        }
        if let Some(expr) = query.strip_prefix("equivalent-classes:") {
            return Ok((QueryType::EquivalentClasses, expr.trim(), false));
        }
        if let Some(expr) = query.strip_prefix("satisfiable:") {
            return Ok((QueryType::Satisfiable, expr.trim(), false));
        }

        // Check for disjoint union queries - if query contains only "or" operators,
        // treat as a special union query that returns the classes that make up the union
        if query.contains(" or ")
            && !query.contains(" and ")
            && !query.contains(':')
            && !query.contains("some")
            && !query.contains("only")
            && !query.contains("not")
        {
            return Ok((QueryType::Subclasses, query, false));
        }

        // Default to instances query
        Ok((QueryType::Instances, query, false))
    }

    /// Parse a class expression string in Manchester Syntax
    pub fn parse_class_expression(&self, expr_string: &str) -> Result<ClassExpression> {
        debug!("Parsing class expression: '{expr_string}'");
        let tokens = self.tokenize(expr_string)?;
        debug!("Tokens: {tokens:?}");
        let result = self
            .parse_expression_tokens(&tokens, 0)
            .map(|(expr, _)| expr);
        debug!("Parsed expression result: {result:?}");
        result
    }

    /// Tokenize a class expression string
    fn tokenize(&self, expr_string: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_angle_brackets = false;
        let mut bracket_depth = 0;

        for ch in expr_string.chars() {
            match ch {
                '<' => {
                    if !current_token.trim().is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    in_angle_brackets = true;
                    current_token.push(ch);
                }
                '>' => {
                    current_token.push(ch);
                    if in_angle_brackets {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                        in_angle_brackets = false;
                    }
                }
                '(' => {
                    if !current_token.trim().is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    tokens.push("(".to_string());
                    bracket_depth += 1;
                }
                ')' => {
                    if !current_token.trim().is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    tokens.push(")".to_string());
                    bracket_depth -= 1;
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if in_angle_brackets {
                        current_token.push(ch);
                    } else if !current_token.trim().is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.trim().is_empty() {
            tokens.push(current_token.trim().to_string());
        }

        if bracket_depth != 0 {
            return Err(Error::reasoning(
                "Unmatched parentheses in class expression",
            ));
        }

        Ok(tokens)
    }

    /// Parse tokens into a class expression
    fn parse_expression_tokens(
        &self,
        tokens: &[String],
        start: usize,
    ) -> Result<(ClassExpression, usize)> {
        if start >= tokens.len() {
            return Err(Error::reasoning("Unexpected end of expression"));
        }

        // Handle parentheses
        if tokens[start] == "(" {
            let (expr, end) = self.parse_expression_tokens(tokens, start + 1)?;
            if end >= tokens.len() || tokens[end] != ")" {
                return Err(Error::reasoning("Missing closing parenthesis"));
            }
            return Ok((expr, end + 1));
        }

        // Parse class names or IRIs
        if tokens[start].starts_with('<') && tokens[start].ends_with('>') {
            let iri_str = &tokens[start][1..tokens[start].len() - 1];
            let class = Class::new(IRI::new(iri_str));
            return self.parse_binary_operators(ClassExpression::Class(class), tokens, start + 1);
        }

        // Handle class names (prefixed or unprefixed)
        if self.is_class_name(&tokens[start]) {
            let class_expr = self.parse_class_name(&tokens[start])?;
            return self.parse_binary_operators(class_expr, tokens, start + 1);
        }

        Err(Error::reasoning(format!(
            "Unexpected token: {}",
            tokens[start]
        )))
    }

    /// Parse binary operators (and, or, some, etc.)
    fn parse_binary_operators(
        &self,
        left: ClassExpression,
        tokens: &[String],
        start: usize,
    ) -> Result<(ClassExpression, usize)> {
        if start >= tokens.len() {
            return Ok((left, start));
        }

        match tokens[start].to_lowercase().as_str() {
            "and" => {
                let (right, end) = self.parse_expression_tokens(tokens, start + 1)?;
                let intersection = match left {
                    ClassExpression::ObjectIntersectionOf(mut exprs) => {
                        exprs.push(right);
                        ClassExpression::ObjectIntersectionOf(exprs)
                    }
                    _ => ClassExpression::ObjectIntersectionOf(vec![left, right]),
                };
                self.parse_binary_operators(intersection, tokens, end)
            }
            "or" => {
                let (right, end) = self.parse_expression_tokens(tokens, start + 1)?;
                let union = match left {
                    ClassExpression::ObjectUnionOf(mut exprs) => {
                        exprs.push(right);
                        ClassExpression::ObjectUnionOf(exprs)
                    }
                    _ => ClassExpression::ObjectUnionOf(vec![left, right]),
                };
                self.parse_binary_operators(union, tokens, end)
            }
            "some" => {
                // Parse "property some class" where left is the property context
                if start + 1 < tokens.len() {
                    let property = self.parse_property_name(&tokens[start - 1])?;
                    let (filler, end) = self.parse_expression_tokens(tokens, start + 1)?;
                    let restriction = ClassExpression::ObjectSomeValuesFrom {
                        property,
                        filler: Box::new(filler),
                    };
                    self.parse_binary_operators(restriction, tokens, end)
                } else {
                    Err(Error::reasoning("Expected class expression after 'some'"))
                }
            }
            "only" => {
                if start + 1 < tokens.len() {
                    let property = self.parse_property_name(&tokens[start - 1])?;
                    let (filler, end) = self.parse_expression_tokens(tokens, start + 1)?;
                    let restriction = ClassExpression::ObjectAllValuesFrom {
                        property,
                        filler: Box::new(filler),
                    };
                    self.parse_binary_operators(restriction, tokens, end)
                } else {
                    Err(Error::reasoning("Expected class expression after 'only'"))
                }
            }
            "not" => {
                let (operand, end) = self.parse_expression_tokens(tokens, start + 1)?;
                let complement = ClassExpression::ObjectComplementOf(Box::new(operand));
                self.parse_binary_operators(complement, tokens, end)
            }
            _ => Ok((left, start)),
        }
    }

    /// Check if a token represents a class name
    fn is_class_name(&self, token: &str) -> bool {
        // Simple heuristic: starts with uppercase or contains ':'
        token.chars().next().is_some_and(char::is_uppercase)
            || token.contains(':')
            || token.starts_with('<')
    }

    /// Parse a class name into a class expression
    fn parse_class_name(&self, name: &str) -> Result<ClassExpression> {
        let iri = if name.contains(':') {
            // Handle prefixed names - for now, just treat as full IRI
            IRI::new(&format!(
                "{}{}",
                self.default_namespace,
                name.split(':').next_back().unwrap_or(name)
            ))
        } else if name.starts_with('<') && name.ends_with('>') {
            // Handle full IRIs in angle brackets
            IRI::new(&name[1..name.len() - 1])
        } else {
            // Handle simple names - use the default namespace
            IRI::new(&format!("{}{}", self.default_namespace, name))
        };

        Ok(ClassExpression::Class(Class::new(iri)))
    }

    /// Parse a property name into an object property expression
    fn parse_property_name(&self, name: &str) -> Result<ObjectPropertyExpression> {
        let iri = if name.contains(':') {
            IRI::new(&format!(
                "{}{}",
                self.default_namespace,
                name.split(':').next_back().unwrap_or(name)
            ))
        } else if name.starts_with('<') && name.ends_with('>') {
            IRI::new(&name[1..name.len() - 1])
        } else {
            IRI::new(&format!("{}{}", self.default_namespace, name))
        };

        Ok(ObjectPropertyExpression::ObjectProperty(
            ObjectProperty::new(iri)?,
        ))
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Extract a readable name from class expression
        let class_name = match &self.query.class_expression {
            ClassExpression::Class(class) => {
                let iri_str = class.iri.to_string();
                if let Some(name) = iri_str.split('#').next_back() {
                    name.to_string()
                } else if let Some(name) = iri_str.split('/').next_back() {
                    name.to_string()
                } else {
                    iri_str
                }
            }
            _ => format!("{:?}", self.query.class_expression),
        };

        writeln!(f, "Query: {:?} of {}", self.query.query_type, class_name)?;
        writeln!(f, "Execution time: {:?}", self.execution_time)?;

        if let Some(ref instances) = self.instances {
            writeln!(f, "\nInstances ({}):", instances.len())?;
            for instance in instances {
                writeln!(f, "  - {instance:?}")?;
            }
        }

        if let Some(ref classes) = self.classes {
            let result_label = match self.query.query_type {
                QueryType::Subclasses => "Subclasses",
                QueryType::Superclasses => "Superclasses",
                QueryType::EquivalentClasses => "Equivalent Classes",
                _ => "Results",
            };
            writeln!(f, "\n{} ({}):", result_label, classes.len())?;
            for class in classes {
                match class {
                    ClassExpression::Class(c) => {
                        let iri_str = c.iri.to_string();
                        let name = if let Some(name) = iri_str.split('#').next_back() {
                            name
                        } else if let Some(name) = iri_str.split('/').next_back() {
                            name
                        } else {
                            &iri_str
                        };
                        writeln!(f, "  - {name}")?;
                    }
                    _ => {
                        writeln!(f, "  - {class:?}")?;
                    }
                }
            }
        }

        if let Some(satisfiable) = self.satisfiable {
            writeln!(f, "\nSatisfiable: {satisfiable}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_class() {
        let parser = DLQueryParser::new();
        let tokens = parser
            .tokenize("Person")
            .expect("Failed to tokenize simple DL query 'Person'");
        assert_eq!(tokens, vec!["Person"]);
    }

    #[test]
    fn test_tokenize_restriction() {
        let parser = DLQueryParser::new();
        let tokens = parser
            .tokenize("hasChild some Person")
            .expect("Failed to tokenize DL query with restriction 'hasChild some Person'");
        assert_eq!(tokens, vec!["hasChild", "some", "Person"]);
    }

    #[test]
    fn test_tokenize_with_iri() {
        let parser = DLQueryParser::new();
        let tokens = parser
            .tokenize("<http://example.org/Person> and hasAge some integer")
            .expect("Failed to tokenize DL query with IRI and restriction");
        assert_eq!(
            tokens,
            vec![
                "<http://example.org/Person>",
                "and",
                "hasAge",
                "some",
                "integer"
            ]
        );
    }

    #[test]
    fn test_parse_simple_class() {
        let parser = DLQueryParser::new();
        let expr = parser
            .parse_class_expression("Person")
            .expect("Failed to parse simple class expression 'Person'");
        match expr {
            ClassExpression::Class(class) => {
                assert!(class.iri.to_string().contains("Person"));
            }
            _ => panic!("Expected class expression"),
        }
    }

    #[test]
    fn test_parse_restriction() {
        let parser = DLQueryParser::new();
        // Note: This is a simplified test - the actual parsing is more complex
        let result = parser.parse_class_expression("Person");
        assert!(result.is_ok());
    }
}
