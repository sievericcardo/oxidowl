//! Functional Syntax Parser
//!
//! This module implements parsing of OWL 2 ontologies from Functional Syntax.

use std::{
    fs::File,
    io::{BufReader, Write, Read},
    path::Path,
};

use crate::{
    Error, Result,
    ontology::{Ontology, ClassExpression},
};

/// Generate a unique axiom ID
fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Functional Syntax Parser
#[derive(Debug, Clone)]
pub struct FunctionalParser {
    // Parser configuration could go here
}

impl FunctionalParser {
    /// Create a new functional syntax parser
    pub fn new() -> Self {
        Self {}
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
        let mut ontology = Ontology::new();
        let mut prefixes = std::collections::HashMap::<String, String>::new();
        
        // Tokenize the content
        let tokens = self.tokenize(content)?;
        let mut position = 0;
        
        while position < tokens.len() {
            position = self.parse_statement(&tokens, position, &mut ontology, &mut prefixes)?;
        }
        
        Ok(ontology)
    }
    
    /// Tokenize the functional syntax content
    fn tokenize(&self, content: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_iri = false;
        let mut paren_depth = 0;
        
        for ch in content.chars() {
            match ch {
                '<' if !in_iri => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    in_iri = true;
                    current_token.push(ch);
                }
                '>' if in_iri => {
                    current_token.push(ch);
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                    in_iri = false;
                }
                '(' | ')' if !in_iri => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_string());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                    if ch == '(' { paren_depth += 1; } else { paren_depth -= 1; }
                }
                ' ' | '\t' | '\n' | '\r' if !in_iri => {
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
    ) -> Result<usize> {
        if position >= tokens.len() {
            return Ok(position);
        }
        
        match tokens[position].as_str() {
            "Prefix" => {
                position = self.parse_prefix(tokens, position, prefixes)?;
            }
            "Ontology" => {
                position = self.parse_ontology_declaration(tokens, position, ontology)?;
            }
            "Declaration" => {
                position = self.parse_declaration(tokens, position, ontology, prefixes)?;
            }
            "SubClassOf" => {
                position = self.parse_subclass_of(tokens, position, ontology, prefixes)?;
            }
            "ClassAssertion" => {
                position = self.parse_class_assertion(tokens, position, ontology, prefixes)?;
            }
            "ObjectPropertyAssertion" => {
                position = self.parse_object_property_assertion(tokens, position, ontology, prefixes)?;
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
            
            if position < tokens.len() {
                let prefix_def = &tokens[position];
                if let Some(eq_pos) = prefix_def.find(":=") {
                    let prefix_name = prefix_def[..eq_pos].to_string();
                    let iri = prefix_def[eq_pos+2..].trim_matches(['<', '>'].as_ref()).to_string();
                    prefixes.insert(prefix_name, iri);
                }
                position += 1;
            }
            
            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
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
    ) -> Result<usize> {
        position += 1; // Skip "Ontology"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("
            
            if position < tokens.len() && tokens[position].starts_with('<') {
                let iri_str = tokens[position].trim_matches(['<', '>'].as_ref());
                if url::Url::parse(iri_str).is_ok() {
                    ontology.set_iri(crate::ontology::IRI::new(iri_str));
                }
                position += 1;
            }
        }
        
        // Skip to matching closing parenthesis
        let mut paren_count = 1;
        while position < tokens.len() && paren_count > 0 {
            if tokens[position] == "(" {
                paren_count += 1;
            } else if tokens[position] == ")" {
                paren_count -= 1;
            }
            position += 1;
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
                                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                                        .into()
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
                                    iri: url::Url::parse(&iri)
                                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
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
    
    /// Parse SubClassOf axiom: SubClassOf(<subclass> <superclass>)
    fn parse_subclass_of(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "SubClassOf"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("
            
            if position + 1 < tokens.len() {
                let sub_iri = self.expand_iri(&tokens[position], prefixes)?;
                let super_iri = self.expand_iri(&tokens[position + 1], prefixes)?;
                
                let subclass = crate::ontology::Class {
                    iri: url::Url::parse(&sub_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?.into()
                };
                let superclass = crate::ontology::Class {
                    iri: url::Url::parse(&super_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?.into()
                };
                
                let axiom = crate::ontology::SubClassOfAxiom {
                    id: generate_axiom_id(),
                    subclass: ClassExpression::Class(subclass),
                    superclass: ClassExpression::Class(superclass),
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::SubClassOf(axiom));
                
                position += 2;
            }
            
            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }
        
        Ok(position)
    }
    
    /// Parse ClassAssertion axiom: ClassAssertion(<class> <individual>)
    fn parse_class_assertion(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "ClassAssertion"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("
            
            if position + 1 < tokens.len() {
                let class_iri = self.expand_iri(&tokens[position], prefixes)?;
                let individual_iri = self.expand_iri(&tokens[position + 1], prefixes)?;
                
                let class = crate::ontology::Class {
                    iri: url::Url::parse(&class_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?.into()
                };
                let individual = crate::ontology::Individual::Named(
                    crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&individual_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    }
                );
                
                let axiom = crate::ontology::ClassAssertionAxiom {
                    id: generate_axiom_id(),
                    class: ClassExpression::Class(class),
                    individual,
                    annotations: vec![],
                };
                ontology.add_axiom(crate::ontology::Axiom::ClassAssertion(axiom));
                
                position += 2;
            }
            
            if position < tokens.len() && tokens[position] == ")" {
                position += 1; // Skip ")"
            }
        }
        
        Ok(position)
    }
    
    /// Parse ObjectPropertyAssertion: ObjectPropertyAssertion(<prop> <subj> <obj>)
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
            
            if position + 2 < tokens.len() {
                let prop_iri = self.expand_iri(&tokens[position], prefixes)?;
                let subj_iri = self.expand_iri(&tokens[position + 1], prefixes)?;
                let obj_iri = self.expand_iri(&tokens[position + 2], prefixes)?;
                
                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&prop_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                };
                let subject = crate::ontology::Individual::Named(
                    crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&subj_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    }
                );
                let object = crate::ontology::Individual::Named(
                    crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&obj_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    }
                );
                
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
    fn expand_iri(
        &self,
        iri: &str,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        if iri.starts_with('<') && iri.ends_with('>') {
            Ok(iri[1..iri.len()-1].to_string())
        } else if let Some(colon_pos) = iri.find(':') {
            let prefix = &iri[..colon_pos];
            let local = &iri[colon_pos+1..];
            
            if let Some(base) = prefixes.get(prefix) {
                Ok(format!("{base}{local}"))
            } else {
                Ok(iri.to_string())
            }
        } else {
            Ok(iri.to_string())
        }
    }
}

/// Parse Functional Syntax from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path)
        .map_err(|e| Error::io(format!("Failed to open file: {e}")))?;
    
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;
    
    parse(&content)
}

/// Save ontology to Functional Syntax file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let mut file = File::create(path)
        .map_err(|e| Error::io(format!("Failed to create file: {e}")))?;
    
    // TODO: Implement serialization to Functional Syntax
    writeln!(file, "# Placeholder for Functional Syntax serialization")?;
    
    Ok(())
}
