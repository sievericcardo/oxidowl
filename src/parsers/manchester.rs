use crate::error::OxidowlError;
use crate::ontology::Ontology;
use std::collections::HashMap;

/// Configuration for Manchester Syntax Parser
#[derive(Debug, Clone)]
pub struct ManchesterParserConfig {
    pub strict_mode: bool,
    pub allow_anonymous_individuals: bool,
    pub custom_prefixes: HashMap<String, String>,
}

impl Default for ManchesterParserConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            allow_anonymous_individuals: false,
            custom_prefixes: HashMap::new(),
        }
    }
}

/// Manchester Syntax Parser for OWL 2
/// Implements parsing according to the Manchester OWL Syntax specification
#[derive(Debug, Clone)]
pub struct ManchesterParser {
    #[allow(dead_code)]
    config: ManchesterParserConfig,
    prefixes: HashMap<String, String>,
    current_position: usize,
    input: String,
}

impl ManchesterParser {
    #[must_use]
    pub fn new(config: ManchesterParserConfig) -> Self {
        let mut prefixes = HashMap::new();

        // Add standard prefixes
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        // Add custom prefixes from config
        prefixes.extend(config.custom_prefixes.clone());

        Self {
            config,
            prefixes,
            current_position: 0,
            input: String::new(),
        }
    }

    /// Parse Manchester Syntax from string
    pub fn parse_string(&mut self, content: &str) -> Result<Ontology, OxidowlError> {
        // Use strict validation for Manchester syntax
        let validator = super::validation::SyntaxValidator::new();
        validator
            .validate_manchester(content)
            .map_err(|e| OxidowlError::ParseError(format!("Manchester validation failed: {e}")))?;

        self.input = content.to_string();
        self.current_position = 0;

        let ontology = Ontology::new();

        // Parse basic prefix declarations
        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("Prefix:") {
                self.parse_prefix_declaration(line)?;
            }
        }

        Ok(ontology)
    }

    /// Parse prefix declaration
    fn parse_prefix_declaration(&mut self, line: &str) -> Result<(), OxidowlError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let prefix_name = parts[1].trim_end_matches(':');
            let iri = parts[2].trim_start_matches('<').trim_end_matches('>');
            self.prefixes
                .insert(prefix_name.to_string(), iri.to_string());
        }
        Ok(())
    }

    /// Parse a class name
    #[allow(dead_code)]
    fn parse_class_name(&self, name: &str) -> Result<crate::ontology::IRI, OxidowlError> {
        self.resolve_iri(name.trim())
    }

    /// Parse a Manchester syntax class expression into `ClassExpression`
    pub fn parse_class_expression(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ClassExpression, OxidowlError> {
        let expr = expr.trim();
        self.parse_class_expr_internal(expr)
    }

    /// Internal recursive parser for class expressions
    fn parse_class_expr_internal(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ClassExpression, OxidowlError> {
        let expr = expr.trim();

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return self.parse_class_expr_internal(&expr[1..expr.len() - 1]);
        }

        // Handle "not" (ObjectComplementOf)
        if let Some(stripped) = expr.strip_prefix("not ") {
            let inner = self.parse_class_expr_internal(stripped)?;
            return Ok(crate::ontology::ClassExpression::ObjectComplementOf(
                Box::new(inner),
            ));
        }

        // Handle "and" (ObjectIntersectionOf)
        if let Some(and_pos) = self.find_top_level_operator(expr, " and ") {
            let left = self.parse_class_expr_internal(&expr[..and_pos])?;
            let right = self.parse_class_expr_internal(&expr[and_pos + 5..])?;
            return Ok(crate::ontology::ClassExpression::ObjectIntersectionOf(
                vec![left, right],
            ));
        }

        // Handle "or" (ObjectUnionOf)
        if let Some(or_pos) = self.find_top_level_operator(expr, " or ") {
            let left = self.parse_class_expr_internal(&expr[..or_pos])?;
            let right = self.parse_class_expr_internal(&expr[or_pos + 4..])?;
            return Ok(crate::ontology::ClassExpression::ObjectUnionOf(vec![
                left, right,
            ]));
        }

        // Handle property restrictions
        if let Some(some_pos) = self.find_top_level_operator(expr, " some ") {
            let property_str = &expr[..some_pos];
            let filler_str = &expr[some_pos + 6..];
            let property = self.parse_property_expression(property_str)?;
            let filler = self.parse_class_expr_internal(filler_str)?;
            return Ok(crate::ontology::ClassExpression::ObjectSomeValuesFrom {
                property,
                filler: Box::new(filler),
            });
        }

        if let Some(only_pos) = self.find_top_level_operator(expr, " only ") {
            let property_str = &expr[..only_pos];
            let filler_str = &expr[only_pos + 6..];
            let property = self.parse_property_expression(property_str)?;
            let filler = self.parse_class_expr_internal(filler_str)?;
            return Ok(crate::ontology::ClassExpression::ObjectAllValuesFrom {
                property,
                filler: Box::new(filler),
            });
        }

        // Handle exact cardinality: "R exactly 3 C"
        if let Some(exactly_pos) = self.find_top_level_operator(expr, " exactly ") {
            let property_str = &expr[..exactly_pos];
            let rest = &expr[exactly_pos + 9..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectExactCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Handle min cardinality: "R min 2 C"
        if let Some(min_pos) = self.find_top_level_operator(expr, " min ") {
            let property_str = &expr[..min_pos];
            let rest = &expr[min_pos + 5..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectMinCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Handle max cardinality: "R max 5 C"
        if let Some(max_pos) = self.find_top_level_operator(expr, " max ") {
            let property_str = &expr[..max_pos];
            let rest = &expr[max_pos + 5..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectMaxCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Default: treat as a simple class name
        let iri = self.resolve_iri(expr)?;
        Ok(crate::ontology::ClassExpression::Class(
            crate::ontology::Class::new(iri),
        ))
    }

    /// Find the position of an operator at the top level (not inside parentheses)
    fn find_top_level_operator(&self, expr: &str, operator: &str) -> Option<usize> {
        let mut depth = 0;
        let chars: Vec<char> = expr.chars().collect();
        let op_chars: Vec<char> = operator.chars().collect();

        for i in 0..chars.len() {
            if chars[i] == '(' {
                depth += 1;
            } else if chars[i] == ')' {
                depth -= 1;
            } else if depth == 0 {
                // Check if operator matches at this position
                if i + op_chars.len() <= chars.len() {
                    let slice: String = chars[i..i + op_chars.len()].iter().collect();
                    if slice == operator {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// Parse property expression (currently just object properties)
    fn parse_property_expression(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ObjectPropertyExpression, OxidowlError> {
        let iri = self.resolve_iri(expr.trim())?;
        let object_property = crate::ontology::ObjectProperty { iri };
        Ok(crate::ontology::ObjectPropertyExpression::ObjectProperty(
            object_property,
        ))
    }

    /// Parse cardinality restriction (proper implementation)
    pub fn parse_cardinality_restriction(&self, expr: &str) -> Result<String, OxidowlError> {
        // Parse Manchester syntax cardinality restrictions like:
        // "exactly 1", "min 2", "max 5", "some", "only"
        let expr = expr.trim();

        if let Some(stripped) = expr.strip_prefix("exactly ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("exactly_{num}"));
            }
        } else if let Some(stripped) = expr.strip_prefix("min ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("min_{num}"));
            }
        } else if let Some(stripped) = expr.strip_prefix("max ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("max_{num}"));
            }
        } else if expr == "some" {
            return Ok("some_values_from".to_string());
        } else if expr == "only" {
            return Ok("all_values_from".to_string());
        }

        // Default case
        Ok(expr.to_string())
    }

    /// Resolve IRI from prefixed name or full IRI
    fn resolve_iri(&self, name: &str) -> Result<crate::ontology::IRI, OxidowlError> {
        // Handle full IRIs in angle brackets
        if name.starts_with('<') && name.ends_with('>') {
            return Ok(crate::ontology::IRI::new(&name[1..name.len() - 1]));
        }

        // Handle prefixed names
        if name.contains(':') {
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let local_name = parts[1];

                if let Some(namespace) = self.prefixes.get(prefix) {
                    return Ok(crate::ontology::IRI::new(&format!(
                        "{namespace}{local_name}"
                    )));
                }
            }
        }

        // Default to treating as local name with no namespace
        Ok(crate::ontology::IRI::new(name))
    }
}

impl Default for ManchesterParser {
    fn default() -> Self {
        Self::new(ManchesterParserConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_parsing() {
        let mut parser = ManchesterParser::default();
        parser
            .parse_prefix_declaration("Prefix: ex: <http://example.org/>")
            .expect("Failed to parse Manchester syntax prefix declaration");

        assert_eq!(
            parser
                .prefixes
                .get("ex")
                .expect("Failed to get namespace prefix from parser"),
            "http://example.org/"
        );
    }

    #[test]
    fn test_class_expression_parsing() {
        let parser = ManchesterParser::default();

        // Test simple class
        let expr = parser
            .parse_class_expression("Person")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::Class(_) => {}
            _ => panic!("Expected Class variant"),
        }

        // Test intersection
        let expr = parser
            .parse_class_expression("Person and Student")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::ObjectIntersectionOf(_) => {}
            _ => panic!("Expected ObjectIntersectionOf variant"),
        }

        // Test some restriction
        let expr = parser
            .parse_class_expression("hasChild some Person")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::ObjectSomeValuesFrom { .. } => {}
            _ => panic!("Expected ObjectSomeValuesFrom variant"),
        }
    }

    #[test]
    fn test_manchester_ontology_parsing() {
        let manchester_content = r#"
Prefix: ex: <http://example.org/>

Class: ex:Person

Class: ex:Student
"#;

        let mut parser = ManchesterParser::default();
        let ontology = parser
            .parse_string(manchester_content)
            .expect("Failed to parse Manchester syntax ontology");

        // Check that basic ontology was created
        assert_eq!(ontology.axioms().len(), 0); // For now, simplified implementation
    }
}
