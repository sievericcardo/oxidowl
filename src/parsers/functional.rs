//! Functional Syntax Parser
//!
//! This module implements parsing of OWL 2 ontologies from Functional Syntax.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
};

use crate::{
    Error, Result,
    ontology::{ClassExpression, Ontology},
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
    #[must_use]
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
    pub fn tokenize(&self, content: &str) -> Result<Vec<String>> {
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
                    if ch == '(' {
                        paren_depth += 1;
                    } else {
                        paren_depth -= 1;
                    }
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
                position = self.parse_ontology_declaration(tokens, position, ontology, prefixes)?;
            }
            "Declaration" => {
                position = self.parse_declaration(tokens, position, ontology, prefixes)?;
            }
            "SubClassOf" => {
                position = self.parse_subclass_of(tokens, position, ontology, prefixes)?;
            }
            "DisjointClasses" => {
                position = self.parse_disjoint_classes(tokens, position, ontology, prefixes)?;
            }
            "ClassAssertion" => {
                position = self.parse_class_assertion(tokens, position, ontology, prefixes)?;
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

    /// Parse ontology declaration: Ontology(<IRI> ...)
    fn parse_ontology_declaration(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &mut std::collections::HashMap<String, String>,
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
                    position = self.parse_statement(tokens, position, ontology, prefixes)?;
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
                                    iri: url::Url::parse(&iri).map_err(|e| {
                                        Error::ontology_parsing(format!("Invalid IRI: {e}"))
                                    })?,
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

    /// Parse `SubClassOf` axiom: `SubClassOf`(<subclass> <superclass>)
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
                        .map_err(|e| {
                            Error::ontology_parsing(format!(
                                "Invalid subclass IRI '{}': {}",
                                sub_iri, e
                            ))
                        })?
                        .into(),
                };
                let superclass = crate::ontology::Class {
                    iri: url::Url::parse(&super_iri)
                        .map_err(|e| {
                            Error::ontology_parsing(format!(
                                "Invalid superclass IRI '{}': {}",
                                super_iri, e
                            ))
                        })?
                        .into(),
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

    /// Parse `ClassAssertion` axiom: `ClassAssertion`(<class> <individual>)
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
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                        .into(),
                };
                let individual =
                    crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&individual_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    });

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

    /// Parse `DisjointClasses` axiom: `DisjointClasses`(<class1> <class2> ...)
    fn parse_disjoint_classes(
        &self,
        tokens: &[String],
        mut position: usize,
        ontology: &mut Ontology,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<usize> {
        position += 1; // Skip "DisjointClasses"
        if position < tokens.len() && tokens[position] == "(" {
            position += 1; // Skip "("

            let mut classes = Vec::new();
            while position < tokens.len() && tokens[position] != ")" {
                let class_iri = self.expand_iri(&tokens[position], prefixes)?;
                let class = crate::ontology::Class {
                    iri: url::Url::parse(&class_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                        .into(),
                };
                classes.push(ClassExpression::Class(class));
                position += 1;
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

            if position + 2 < tokens.len() {
                let prop_iri = self.expand_iri(&tokens[position], prefixes)?;
                let subj_iri = self.expand_iri(&tokens[position + 1], prefixes)?;
                let obj_iri = self.expand_iri(&tokens[position + 2], prefixes)?;

                let property = crate::ontology::ObjectProperty {
                    iri: url::Url::parse(&prop_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?,
                };
                let subject =
                    crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                        iri: url::Url::parse(&subj_iri)
                            .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                            .into(), // Convert URL to IRI
                    });
                let object = crate::ontology::Individual::Named(crate::ontology::NamedIndividual {
                    iri: url::Url::parse(&obj_iri)
                        .map_err(|e| Error::ontology_parsing(format!("Invalid IRI: {e}")))?
                        .into(), // Convert URL to IRI
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
    fn expand_iri(
        &self,
        iri: &str,
        prefixes: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        if iri.starts_with('<') && iri.ends_with('>') {
            Ok(iri[1..iri.len() - 1].to_string())
        } else if let Some(colon_pos) = iri.find(':') {
            let prefix = &iri[..colon_pos];
            let local = &iri[colon_pos + 1..];

            if let Some(base) = prefixes.get(prefix) {
                let expanded = format!("{base}{local}");
                // Validate the expanded IRI can be parsed as a URL
                if url::Url::parse(&expanded).is_err() {
                    return Err(crate::error::Error::OntologyParsing {
                        message: format!(
                            "Invalid IRI: relative URL without a base. Original: '{}', Expanded: '{}', Available prefixes: {:?}",
                            iri, expanded, prefixes
                        ),
                    });
                }
                Ok(expanded)
            } else {
                Ok(iri.to_string())
            }
        } else {
            Ok(iri.to_string())
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

/// Save ontology to Functional Syntax file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let mut file =
        File::create(path).map_err(|e| Error::io(format!("Failed to create file: {e}")))?;

    // Write ontology header
    if let Some(onto_iri) = ontology.get_iri() {
        if let Some(version_iri) = &ontology.version_iri {
            writeln!(file, "Ontology(<{}> <{}>", onto_iri, version_iri)?;
        } else {
            writeln!(file, "Ontology(<{}>", onto_iri)?;
        }
    } else {
        writeln!(file, "Ontology(")?;
    }

    // Write imports
    for import in &ontology.imports {
        writeln!(file, "  Import(<{}>)", import)?;
    }

    // Write annotations
    for annotation in &ontology.annotations {
        writeln!(file, "  Annotation({} {})", 
                serialize_annotation_property(&annotation.property),
                serialize_annotation_value(&annotation.value))?;
    }

    // Write axioms
    for axiom in ontology.axioms() {
        writeln!(file, "  {}", serialize_axiom(axiom))?;
    }

    writeln!(file, ")")?;
    Ok(())
}

fn serialize_axiom(axiom: &crate::ontology::Axiom) -> String {
    match axiom {
        crate::ontology::Axiom::SubClassOf(sub) => {
            format!("SubClassOf({} {})", 
                   serialize_class_expression(&sub.subclass),
                   serialize_class_expression(&sub.superclass))
        }
        crate::ontology::Axiom::ClassAssertion(ca) => {
            format!("ClassAssertion({} {})",
                   serialize_class_expression(&ca.class),
                   serialize_individual(&ca.individual))
        }
        crate::ontology::Axiom::Declaration(decl) => {
            format!("Declaration({})", serialize_entity(&decl.entity))
        }
        _ => format!("# Unsupported axiom type: {:?}", axiom)
    }
}

fn serialize_class_expression(ce: &crate::ontology::ClassExpression) -> String {
    match ce {
        crate::ontology::ClassExpression::Class(class) => format!("<{}>", class.iri),
        crate::ontology::ClassExpression::ObjectIntersectionOf(classes) => {
            let class_strs: Vec<String> = classes.iter()
                .map(serialize_class_expression)
                .collect();
            format!("ObjectIntersectionOf({})", class_strs.join(" "))
        }
        crate::ontology::ClassExpression::ObjectUnionOf(classes) => {
            let class_strs: Vec<String> = classes.iter()
                .map(serialize_class_expression)
                .collect();
            format!("ObjectUnionOf({})", class_strs.join(" "))
        }
        _ => format!("# Unsupported class expression: {:?}", ce)
    }
}

fn serialize_individual(ind: &crate::ontology::Individual) -> String {
    format!("<{}>", ind.iri().map(|iri| iri.as_str()).unwrap_or("_:anonymous"))
}

fn serialize_entity(entity: &crate::ontology::Entity) -> String {
    match entity {
        crate::ontology::Entity::Class(class) => format!("Class(<{}>)", class.as_str()),
        crate::ontology::Entity::ObjectProperty(prop) => format!("ObjectProperty(<{}>)", prop.as_str()),
        crate::ontology::Entity::DataProperty(prop) => format!("DataProperty(<{}>)", prop.as_str()),
        crate::ontology::Entity::NamedIndividual(ind) => format!("NamedIndividual(<{}>)", ind.as_str()),
        crate::ontology::Entity::Datatype(dt) => format!("Datatype(<{}>)", dt.as_str()),
        crate::ontology::Entity::AnnotationProperty(ap) => format!("AnnotationProperty(<{}>)", ap.as_str()),
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
        },
        crate::ontology::AnnotationValue::AnonymousIndividual(anon) => {
            format!("_:{}", anon.id)
        }
    }
}
