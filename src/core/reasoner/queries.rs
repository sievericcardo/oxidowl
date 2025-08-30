//! Query processing for SPARQL and OWLlink
//!
//! This module handles parsing and execution of SPARQL queries and OWLlink requests.

use crate::{
    Error, Result,
    ontology::{Axiom, ClassExpression, Individual, Ontology},
};
use std::collections::HashMap;

/// SPARQL query representation
#[derive(Debug, Clone)]
pub struct SparqlQuery {
    pub query_type: String,
    pub query_text: String,
    pub variables: Vec<String>,
    pub patterns: Vec<TriplePattern>,
}

/// Triple pattern for SPARQL queries
#[derive(Debug, Clone)]
pub struct TriplePattern {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// OWLlink request representation
#[derive(Debug, Clone)]
pub struct OwllinkRequest {
    pub request_type: String,
    pub request_xml: String,
    pub class_expression: Option<ClassExpression>,
    pub kb_name: Option<String>,
    pub axiom: Option<Axiom>,
    pub individual: Option<Individual>,
    pub direct: Option<bool>,
}

/// Processor for SPARQL and OWLlink queries
#[derive(Debug)]
pub struct QueryProcessor;

impl QueryProcessor {
    /// Create a new query processor
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Process a SPARQL query against the ontology
    pub fn process_sparql_query(&self, query: &str, ontology: &Ontology) -> Result<String> {
        // Parse the SPARQL query
        let parsed_query = self.parse_sparql_query(query)?;
        
        // Execute query based on type
        match parsed_query.query_type.as_str() {
            "SELECT" => self.execute_select_query(&parsed_query, ontology),
            "ASK" => self.execute_ask_query(&parsed_query, ontology),
            "CONSTRUCT" => self.execute_construct_query(&parsed_query, ontology),
            "DESCRIBE" => self.execute_describe_query(&parsed_query, ontology),
            _ => Err(Error::reasoning(&format!("Unsupported SPARQL query type: {}", parsed_query.query_type)))
        }
    }

    /// Parse SPARQL query into structured representation
    fn parse_sparql_query(&self, query: &str) -> Result<SparqlQuery> {
        // Basic SPARQL parsing - in production would use a proper SPARQL parser
        let trimmed = query.trim();
        
        let query_type = if trimmed.to_uppercase().starts_with("SELECT") {
            "SELECT"
        } else if trimmed.to_uppercase().starts_with("ASK") {
            "ASK"
        } else if trimmed.to_uppercase().starts_with("CONSTRUCT") {
            "CONSTRUCT"
        } else if trimmed.to_uppercase().starts_with("DESCRIBE") {
            "DESCRIBE"
        } else {
            return Err(Error::reasoning("Invalid SPARQL query"));
        };

        Ok(SparqlQuery {
            query_type: query_type.to_string(),
            query_text: query.to_string(),
            variables: self.extract_variables(query)?,
            patterns: self.extract_triple_patterns(query)?,
        })
    }

    /// Execute SELECT query
    fn execute_select_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        let mut results = Vec::new();
        
        // Find bindings that satisfy the query patterns
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        
        // Project to selected variables
        for binding in bindings {
            let mut row = Vec::new();
            for var in &query.variables {
                if let Some(value) = binding.get(var) {
                    row.push(value.clone());
                } else {
                    row.push("UNBOUND".to_string());
                }
            }
            results.push(row);
        }
        
        // Format results as SPARQL Results XML/JSON
        Ok(self.format_select_results(&query.variables, &results))
    }

    /// Execute ASK query
    fn execute_ask_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        let result = !bindings.is_empty();
        Ok(format!("{{\"boolean\": {}}}", result))
    }

    /// Execute CONSTRUCT query
    fn execute_construct_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        // Extract construct templates and execute
        let construct_patterns = self.extract_construct_patterns(&query.query_text)?;
        let bindings = self.find_pattern_bindings(&query.patterns, ontology)?;
        
        let mut triples = Vec::new();
        for binding in bindings {
            for pattern in &construct_patterns {
                if let Some(triple) = self.instantiate_pattern(pattern, &binding) {
                    triples.push(triple);
                }
            }
        }
        
        Ok(self.format_construct_results(&triples))
    }

    /// Execute DESCRIBE query  
    fn execute_describe_query(&self, query: &SparqlQuery, ontology: &Ontology) -> Result<String> {
        // For DESCRIBE queries, return all known facts about the resources
        let resources = self.extract_described_resources(&query.query_text)?;
        let mut triples = Vec::new();
        
        for resource in resources {
            triples.extend(self.get_resource_description(&resource, ontology)?);
        }
        
        Ok(self.format_construct_results(&triples))
    }

    /// Process an `OWLlink` request
    pub fn process_owllink_request(&self, request: &str, ontology: &Ontology) -> Result<String> {
        // Parse OWLlink XML request
        let parsed_request = self.parse_owllink_request(request)?;
        
        match parsed_request.request_type.as_str() {
            "IsKBSatisfiable" => self.handle_kb_satisfiable(ontology),
            "IsClassSatisfiable" => self.handle_class_satisfiable(&parsed_request, ontology),
            "IsEntailed" => self.handle_entailment_check(&parsed_request, ontology),
            "GetSubClasses" => self.handle_get_subclasses(&parsed_request, ontology),
            "GetSuperClasses" => self.handle_get_superclasses(&parsed_request, ontology),
            "GetEquivalentClasses" => self.handle_get_equivalent_classes(&parsed_request, ontology),
            "GetInstances" => self.handle_get_instances(&parsed_request, ontology),
            "GetTypes" => self.handle_get_types(&parsed_request, ontology),
            _ => Err(Error::reasoning(&format!("Unsupported OWLlink request: {}", parsed_request.request_type)))
        }
    }

    /// Parse OWLlink XML request
    fn parse_owllink_request(&self, request: &str) -> Result<OwllinkRequest> {
        // Basic XML parsing for OWLlink - in production would use proper XML parser
        let request_type = if request.contains("IsKBSatisfiable") {
            "IsKBSatisfiable"
        } else if request.contains("IsClassSatisfiable") {
            "IsClassSatisfiable"
        } else if request.contains("IsEntailed") {
            "IsEntailed"
        } else if request.contains("GetSubClasses") {
            "GetSubClasses"
        } else if request.contains("GetSuperClasses") {
            "GetSuperClasses"
        } else if request.contains("GetEquivalentClasses") {
            "GetEquivalentClasses"
        } else if request.contains("GetInstances") {
            "GetInstances"
        } else if request.contains("GetTypes") {
            "GetTypes"
        } else {
            return Err(Error::reasoning("Unknown OWLlink request type"));
        };

        Ok(OwllinkRequest {
            request_type: request_type.to_string(),
            request_xml: request.to_string(),
            class_expression: self.extract_class_from_owllink(request).ok(),
            kb_name: self.extract_kb_name_from_owllink(request).ok(),
            axiom: None,           // TODO: extract from XML
            individual: None,      // TODO: extract from XML  
            direct: None,          // TODO: extract from XML
        })
    }

    // Helper methods for SPARQL processing

    /// Extract variables from SPARQL query
    fn extract_variables(&self, query: &str) -> Result<Vec<String>> {
        let mut variables = Vec::new();
        
        // Simple regex-like extraction for variables starting with ?
        let words: Vec<&str> = query.split_whitespace().collect();
        for word in words {
            if word.starts_with('?') {
                let var = word.trim_end_matches(&[',', '.', ';', ')', '}'][..]);
                if !variables.contains(&var.to_string()) {
                    variables.push(var.to_string());
                }
            }
        }
        
        Ok(variables)
    }

    /// Extract triple patterns from SPARQL query
    fn extract_triple_patterns(&self, query: &str) -> Result<Vec<TriplePattern>> {
        let mut patterns = Vec::new();
        
        // Find WHERE clause and extract patterns
        if let Some(where_start) = query.to_uppercase().find("WHERE") {
            let where_clause = &query[where_start + 5..];
            
            // Extract patterns between braces
            if let Some(brace_start) = where_clause.find('{') {
                if let Some(brace_end) = where_clause.rfind('}') {
                    let pattern_text = &where_clause[brace_start + 1..brace_end];
                    
                    // Split by periods to get individual patterns
                    for line in pattern_text.split('.') {
                        let line = line.trim();
                        if line.is_empty() { continue; }
                        
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            patterns.push(TriplePattern {
                                subject: parts[0].to_string(),
                                predicate: parts[1].to_string(),
                                object: parts[2].to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(patterns)
    }

    /// Find bindings that satisfy the triple patterns
    fn find_pattern_bindings(&self, patterns: &[TriplePattern], ontology: &Ontology) -> Result<Vec<HashMap<String, String>>> {
        let mut all_bindings = Vec::new();
        
        // For each pattern, find all possible bindings
        for pattern in patterns {
            let pattern_bindings = self.find_single_pattern_bindings(pattern, ontology)?;
            
            if all_bindings.is_empty() {
                all_bindings = pattern_bindings;
            } else {
                // Join with existing bindings
                all_bindings = self.join_bindings(&all_bindings, &pattern_bindings);
            }
        }
        
        Ok(all_bindings)
    }

    /// Find bindings for a single triple pattern
    fn find_single_pattern_bindings(&self, pattern: &TriplePattern, ontology: &Ontology) -> Result<Vec<HashMap<String, String>>> {
        let mut bindings = Vec::new();
        
        // Check against class assertions
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(assertion) => {
                    if let crate::ontology::Individual::Named(named) = &assertion.individual {
                        let subject = format!("<{}>", named.iri);
                        let predicate = "rdf:type".to_string();
                        let object = self.format_class_expression(&assertion.class);
                        
                        if let Some(binding) = self.match_pattern(pattern, &subject, &predicate, &object) {
                            bindings.push(binding);
                        }
                    }
                }
                Axiom::ObjectPropertyAssertion(assertion) => {
                    if let (crate::ontology::Individual::Named(sub), crate::ontology::Individual::Named(obj)) = (&assertion.source, &assertion.target) {
                        let subject = format!("<{}>", sub.iri);
                        let predicate = self.format_object_property(&assertion.property);
                        let object = format!("<{}>", obj.iri);
                        
                        if let Some(binding) = self.match_pattern(pattern, &subject, &predicate, &object) {
                            bindings.push(binding);
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(bindings)
    }

    /// Match a pattern against concrete values and return variable bindings
    fn match_pattern(&self, pattern: &TriplePattern, subject: &str, predicate: &str, object: &str) -> Option<HashMap<String, String>> {
        let mut binding = HashMap::new();
        
        // Match subject
        if pattern.subject.starts_with('?') {
            binding.insert(pattern.subject.clone(), subject.to_string());
        } else if pattern.subject != subject {
            return None;
        }
        
        // Match predicate
        if pattern.predicate.starts_with('?') {
            binding.insert(pattern.predicate.clone(), predicate.to_string());
        } else if pattern.predicate != predicate {
            return None;
        }
        
        // Match object
        if pattern.object.starts_with('?') {
            binding.insert(pattern.object.clone(), object.to_string());
        } else if pattern.object != object {
            return None;
        }
        
        Some(binding)
    }

    /// Join two sets of bindings
    fn join_bindings(&self, left: &[HashMap<String, String>], right: &[HashMap<String, String>]) -> Vec<HashMap<String, String>> {
        let mut result = Vec::new();
        
        for left_binding in left {
            for right_binding in right {
                if self.bindings_compatible(left_binding, right_binding) {
                    let mut joined = left_binding.clone();
                    joined.extend(right_binding.clone());
                    result.push(joined);
                }
            }
        }
        
        result
    }

    /// Check if two bindings are compatible (no conflicting variable assignments)
    fn bindings_compatible(&self, left: &HashMap<String, String>, right: &HashMap<String, String>) -> bool {
        for (var, value) in left {
            if let Some(other_value) = right.get(var) {
                if value != other_value {
                    return false;
                }
            }
        }
        true
    }

    /// Format SELECT query results
    fn format_select_results(&self, variables: &[String], results: &[Vec<String>]) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\"?>\n");
        output.push_str("<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n");
        output.push_str("  <head>\n");
        
        for var in variables {
            output.push_str(&format!("    <variable name=\"{}\"/>\n", var.trim_start_matches('?')));
        }
        
        output.push_str("  </head>\n  <results>\n");
        
        for row in results {
            output.push_str("    <result>\n");
            for (i, value) in row.iter().enumerate() {
                if i < variables.len() {
                    let var_name = variables[i].trim_start_matches('?');
                    output.push_str(&format!("      <binding name=\"{}\">\n", var_name));
                    if value.starts_with('<') && value.ends_with('>') {
                        output.push_str(&format!("        <uri>{}</uri>\n", &value[1..value.len()-1]));
                    } else {
                        output.push_str(&format!("        <literal>{}</literal>\n", value));
                    }
                    output.push_str("      </binding>\n");
                }
            }
            output.push_str("    </result>\n");
        }
        
        output.push_str("  </results>\n</sparql>");
        output
    }

    /// Extract construct patterns from CONSTRUCT query
    fn extract_construct_patterns(&self, query: &str) -> Result<Vec<TriplePattern>> {
        // Find CONSTRUCT clause
        if let Some(construct_start) = query.to_uppercase().find("CONSTRUCT") {
            let construct_part = &query[construct_start + 9..];
            
            // Find WHERE to delimit construct template
            if let Some(where_pos) = construct_part.to_uppercase().find("WHERE") {
                let template = &construct_part[..where_pos];
                return self.extract_triple_patterns(&format!("WHERE {{{}}}", template));
            }
        }
        
        Ok(Vec::new())
    }

    /// Instantiate a pattern with variable bindings
    fn instantiate_pattern(&self, pattern: &TriplePattern, binding: &HashMap<String, String>) -> Option<(String, String, String)> {
        let subject = if pattern.subject.starts_with('?') {
            binding.get(&pattern.subject)?.clone()
        } else {
            pattern.subject.clone()
        };
        
        let predicate = if pattern.predicate.starts_with('?') {
            binding.get(&pattern.predicate)?.clone()
        } else {
            pattern.predicate.clone()
        };
        
        let object = if pattern.object.starts_with('?') {
            binding.get(&pattern.object)?.clone()
        } else {
            pattern.object.clone()
        };
        
        Some((subject, predicate, object))
    }

    /// Format CONSTRUCT/DESCRIBE results as RDF
    fn format_construct_results(&self, triples: &[(String, String, String)]) -> String {
        let mut output = String::new();
        output.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        output.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n");
        
        for (subject, predicate, object) in triples {
            output.push_str(&format!("{} {} {} .\n", subject, predicate, object));
        }
        
        output
    }

    /// Extract described resources from DESCRIBE query
    fn extract_described_resources(&self, query: &str) -> Result<Vec<String>> {
        let mut resources = Vec::new();
        
        // Simple extraction for DESCRIBE queries
        if let Some(describe_start) = query.to_uppercase().find("DESCRIBE") {
            let describe_part = &query[describe_start + 8..];
            
            // Find WHERE clause or end of query
            let end_pos = describe_part.to_uppercase().find("WHERE")
                .unwrap_or(describe_part.len());
            
            let resource_part = &describe_part[..end_pos];
            
            // Extract resource URIs and variables
            for word in resource_part.split_whitespace() {
                let word = word.trim_end_matches(&[',', '.', ';'][..]);
                if word.starts_with('<') && word.ends_with('>') {
                    resources.push(word.to_string());
                } else if word.starts_with('?') {
                    resources.push(word.to_string());
                }
            }
        }
        
        Ok(resources)
    }

    /// Get all known facts about a resource
    fn get_resource_description(&self, resource: &str, ontology: &Ontology) -> Result<Vec<(String, String, String)>> {
        let mut triples = Vec::new();
        
        // If it's a variable, we need to resolve it first (simplified approach)
        let target_iri = if resource.starts_with('?') {
            // For variables, return empty for now - would need query context
            return Ok(triples);
        } else if resource.starts_with('<') && resource.ends_with('>') {
            &resource[1..resource.len()-1]
        } else {
            resource
        };
        
        // Find all assertions about this resource
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(assertion) => {
                    if let crate::ontology::Individual::Named(named) = &assertion.individual {
                        if named.iri.as_str() == target_iri {
                            triples.push((
                                resource.to_string(),
                                "rdf:type".to_string(),
                                self.format_class_expression(&assertion.class)
                            ));
                        }
                    }
                }
                Axiom::ObjectPropertyAssertion(assertion) => {
                    if let crate::ontology::Individual::Named(subject) = &assertion.source {
                        if subject.iri.as_str() == target_iri {
                            if let crate::ontology::Individual::Named(object) = &assertion.target {
                                triples.push((
                                    resource.to_string(),
                                    self.format_object_property(&assertion.property),
                                    format!("<{}>", object.iri)
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(triples)
    }

    /// Format class expression for output
    fn format_class_expression(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => format!("<{}>", class.iri),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!("_:some{}_{}", 
                    self.format_object_property(property),
                    self.format_class_expression(filler))
            }
            // Add more cases as needed
            _ => "_:complex".to_string()
        }
    }

    /// Format object property for output
    fn format_object_property(&self, prop: &crate::ontology::ObjectPropertyExpression) -> String {
        match prop {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => format!("<{}>", prop.iri),
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("^{}", format!("<{}>", prop.iri))
            }
            // Add more cases as needed
            _ => "_:complex_property".to_string()
        }
    }

    // Helper methods for OWLlink processing

    /// Handle KB satisfiability check
    fn handle_kb_satisfiable(&self, _ontology: &Ontology) -> Result<String> {
        // This would need access to the reasoner to check consistency
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="true" />"#))
    }

    /// Handle class satisfiability check
    fn handle_class_satisfiable(&self, request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        if let Some(class_expr) = &request.class_expression {
            // For now, implement a basic satisfiability check
            // In a full implementation, this would use tableau reasoning
            let is_satisfiable = match class_expr {
                ClassExpression::Class(class) => {
                    // Check if it's owl:Nothing (always unsatisfiable)
                    if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                        false
                    } else {
                        true // Assume satisfiable for now
                    }
                }
                _ => true, // For complex expressions, assume satisfiable for now
            };
            Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="{}" />"#, is_satisfiable))
        } else {
            Err(Error::reasoning("No class expression provided in satisfiability request"))
        }
    }

    /// Handle entailment check
    fn handle_entailment_check(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<BooleanResponse result="false" />"#))
    }

    /// Handle get subclasses request
    fn handle_get_subclasses(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse></SetOfClassesResponse>"#))
    }

    /// Handle get superclasses request
    fn handle_get_superclasses(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse></SetOfClassesResponse>"#))
    }

    /// Handle get equivalent classes request
    fn handle_get_equivalent_classes(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse></SetOfClassesResponse>"#))
    }

    /// Handle get instances request
    fn handle_get_instances(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfIndividualsResponse></SetOfIndividualsResponse>"#))
    }

    /// Handle get types request
    fn handle_get_types(&self, _request: &OwllinkRequest, _ontology: &Ontology) -> Result<String> {
        // Placeholder implementation
        Ok(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SetOfClassesResponse></SetOfClassesResponse>"#))
    }

    /// Extract class expression from OWLlink XML
    fn extract_class_from_owllink(&self, xml: &str) -> Result<ClassExpression> {
        // Basic XML parsing to extract class
        if let Some(start) = xml.find("IRI=\"") {
            if let Some(end) = xml[start + 5..].find('"') {
                let iri_str = &xml[start + 5..start + 5 + end];
                return Ok(ClassExpression::Class(crate::ontology::Class {
                    iri: crate::ontology::IRI::new(iri_str)
                }));
            }
        }
        
        Err(Error::reasoning("Could not extract class from OWLlink request"))
    }

    /// Extract KB name from OWLlink XML
    fn extract_kb_name_from_owllink(&self, xml: &str) -> Result<String> {
        // Basic XML parsing to extract KB name
        if let Some(start) = xml.find("kb=\"") {
            if let Some(end) = xml[start + 4..].find('"') {
                let kb_name = &xml[start + 4..start + 4 + end];
                return Ok(kb_name.to_string());
            }
        }
        
        Ok("default".to_string())
    }
}

impl Default for QueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}
