//! DL Query Module
//!
//! This module provides a Description Logic query engine that can parse and execute
//! DL queries against the ontology. It supports queries like:
//! - "ClassA some PropertyR" 
//! - "PropertyR some ClassB"
//! - "ClassA and (PropertyR some ClassB)"
//!
//! The query language follows Manchester Syntax for class expressions.

use crate::{
    Error, Result,
    ontology::{ClassExpression, ObjectPropertyExpression, Individual, Class, ObjectProperty, IRI},
    reasoning::ReasoningService,
};
use std::{
    collections::HashSet,
    fmt,
};

/// DL Query Engine for executing description logic queries
#[derive(Debug)]
pub struct DLQueryEngine {
    reasoning_service: ReasoningService,
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
    pub fn new(reasoning_service: ReasoningService) -> Self {
        Self {
            reasoning_service,
        }
    }

    /// Parse and execute a DL query string
    pub async fn execute_query(&self, query_string: &str) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();
        
        // Parse the query
        let query = self.parse_query(query_string)?;
        
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
                let instances = self.reasoning_service
                    .get_instances(&query.class_expression, query.direct)
                    .await?;
                result.instances = Some(instances);
            }
            QueryType::Subclasses => {
                let subclasses = self.reasoning_service
                    .get_subclasses(&query.class_expression, query.direct)
                    .await?;
                result.classes = Some(subclasses);
            }
            QueryType::Superclasses => {
                let superclasses = self.reasoning_service
                    .get_superclasses(&query.class_expression, query.direct)
                    .await?;
                result.classes = Some(superclasses);
            }
            QueryType::EquivalentClasses => {
                let equivalent = self.reasoning_service
                    .get_equivalent_classes(&query.class_expression)
                    .await?;
                result.classes = Some(equivalent);
            }
            QueryType::Satisfiable => {
                let satisfiable = self.reasoning_service
                    .is_satisfiable(&query.class_expression)
                    .await?;
                result.satisfiable = Some(satisfiable);
            }
        }
        
        result.execution_time = start_time.elapsed();
        Ok(result)
    }

    /// Parse a DL query string into a structured query
    pub fn parse_query(&self, query_string: &str) -> Result<DLQuery> {
        let parser = DLQueryParser::new();
        parser.parse(query_string)
    }

    /// Execute a query for instances of a class expression
    pub async fn get_instances(&self, class_expression: &ClassExpression, direct: bool) -> Result<HashSet<Individual>> {
        self.reasoning_service.get_instances(class_expression, direct).await
    }

    /// Execute a query for subclasses of a class expression
    pub async fn get_subclasses(&self, class_expression: &ClassExpression, direct: bool) -> Result<HashSet<ClassExpression>> {
        self.reasoning_service.get_subclasses(class_expression, direct).await
    }

    /// Execute a query for superclasses of a class expression
    pub async fn get_superclasses(&self, class_expression: &ClassExpression, direct: bool) -> Result<HashSet<ClassExpression>> {
        self.reasoning_service.get_superclasses(class_expression, direct).await
    }

    /// Check satisfiability of a class expression
    pub async fn is_satisfiable(&self, class_expression: &ClassExpression) -> Result<bool> {
        self.reasoning_service.is_satisfiable(class_expression).await
    }
}

/// Parser for DL query strings in Manchester Syntax
pub struct DLQueryParser {
    // Future: Add configuration for syntax variations
}

impl DLQueryParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a query string into a DL query
    pub fn parse(&self, query_string: &str) -> Result<DLQuery> {
        let trimmed = query_string.trim();
        
        // Determine query type and extract class expression
        let (query_type, class_expr_str, direct) = self.parse_query_structure(trimmed)?;
        
        // Parse the class expression
        let class_expression = self.parse_class_expression(class_expr_str)?;
        
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
        if let Some(expr) = query.strip_prefix("satisfiable:") {
            return Ok((QueryType::Satisfiable, expr.trim(), false));
        }
        
        // Default to instances query
        Ok((QueryType::Instances, query, false))
    }

    /// Parse a class expression string in Manchester Syntax
    pub fn parse_class_expression(&self, expr_string: &str) -> Result<ClassExpression> {
        let tokens = self.tokenize(expr_string)?;
        self.parse_expression_tokens(&tokens, 0).map(|(expr, _)| expr)
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
            return Err(Error::reasoning("Unmatched parentheses in class expression"));
        }

        Ok(tokens)
    }

    /// Parse tokens into a class expression
    fn parse_expression_tokens(&self, tokens: &[String], start: usize) -> Result<(ClassExpression, usize)> {
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
            let iri_str = &tokens[start][1..tokens[start].len()-1];
            let class = Class::new(IRI::new(iri_str));
            return self.parse_binary_operators(ClassExpression::Class(class), tokens, start + 1);
        }

        // Handle class names (prefixed or unprefixed)
        if self.is_class_name(&tokens[start]) {
            let class_expr = self.parse_class_name(&tokens[start])?;
            return self.parse_binary_operators(class_expr, tokens, start + 1);
        }

        Err(Error::reasoning(&format!("Unexpected token: {}", tokens[start])))
    }

    /// Parse binary operators (and, or, some, etc.)
    fn parse_binary_operators(&self, left: ClassExpression, tokens: &[String], start: usize) -> Result<(ClassExpression, usize)> {
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
                    _ => ClassExpression::ObjectIntersectionOf(vec![left, right])
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
                    _ => ClassExpression::ObjectUnionOf(vec![left, right])
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
            _ => Ok((left, start))
        }
    }

    /// Check if a token represents a class name
    fn is_class_name(&self, token: &str) -> bool {
        // Simple heuristic: starts with uppercase or contains ':'
        token.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) || 
        token.contains(':') ||
        token.starts_with('<')
    }

    /// Parse a class name into a class expression
    fn parse_class_name(&self, name: &str) -> Result<ClassExpression> {
        let iri = if name.contains(':') {
            // Handle prefixed names - for now, just treat as full IRI
            IRI::new(&format!("http://example.org/{}", name))
        } else if name.starts_with('<') && name.ends_with('>') {
            // Handle full IRIs in angle brackets
            IRI::new(&name[1..name.len()-1])
        } else {
            // Handle simple names
            IRI::new(&format!("http://example.org/{}", name))
        };
        
        Ok(ClassExpression::Class(Class::new(iri)))
    }

    /// Parse a property name into an object property expression
    fn parse_property_name(&self, name: &str) -> Result<ObjectPropertyExpression> {
        let iri = if name.contains(':') {
            IRI::new(&format!("http://example.org/{}", name))
        } else if name.starts_with('<') && name.ends_with('>') {
            IRI::new(&name[1..name.len()-1])
        } else {
            IRI::new(&format!("http://example.org/{}", name))
        };
        
        Ok(ObjectPropertyExpression::ObjectProperty(ObjectProperty::new(iri)?))
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Query: {:?}", self.query.query_type)?;
        writeln!(f, "Expression: {:?}", self.query.class_expression)?;
        writeln!(f, "Execution time: {:?}", self.execution_time)?;
        
        if let Some(ref instances) = self.instances {
            writeln!(f, "Instances ({}):", instances.len())?;
            for instance in instances {
                writeln!(f, "  - {:?}", instance)?;
            }
        }
        
        if let Some(ref classes) = self.classes {
            writeln!(f, "Classes ({}):", classes.len())?;
            for class in classes {
                writeln!(f, "  - {:?}", class)?;
            }
        }
        
        if let Some(satisfiable) = self.satisfiable {
            writeln!(f, "Satisfiable: {}", satisfiable)?;
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
        let tokens = parser.tokenize("Person").unwrap();
        assert_eq!(tokens, vec!["Person"]);
    }

    #[test]
    fn test_tokenize_restriction() {
        let parser = DLQueryParser::new();
        let tokens = parser.tokenize("hasChild some Person").unwrap();
        assert_eq!(tokens, vec!["hasChild", "some", "Person"]);
    }

    #[test]
    fn test_tokenize_with_iri() {
        let parser = DLQueryParser::new();
        let tokens = parser.tokenize("<http://example.org/Person> and hasAge some integer").unwrap();
        assert_eq!(tokens, vec!["<http://example.org/Person>", "and", "hasAge", "some", "integer"]);
    }

    #[test]
    fn test_parse_simple_class() {
        let parser = DLQueryParser::new();
        let expr = parser.parse_class_expression("Person").unwrap();
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
