//! RDF/XML Parser
//!
//! This module implements parsing of OWL 2 ontologies from RDF/XML format.
//! Supports RDF 1.1 reification and RDF 1.2 rdf:reifies.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
};

use super::common::OntologySerializer;
use crate::{
    Error, Result,
    ontology::Ontology,
    semantics::{IriValidationMode, RdfTerm, Triple as RdfTriple, vocabulary::*},
};

/// RDF version mode for RDF/XML parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfVersionMode {
    /// RDF 1.1 - preserve old-style reification
    RDF11,
    /// RDF 1.2 - convert reification to quoted triples
    RDF12,
    /// Auto-detect based on presence of rdf:reifies or reification patterns
    Auto,
}

/// Reification handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReificationMode {
    /// Preserve reification as-is (RDF 1.1 style)
    Preserve,
    /// Convert reification patterns to quoted triples (RDF 1.2 style)
    ConvertToQuotedTriples,
    /// Auto-detect based on document features
    Auto,
}

/// Configuration for the RDF/XML parser
#[derive(Debug, Clone)]
pub struct RdfXmlParserConfig {
    /// Whether to validate XML structure (default: true)
    pub validate_xml: bool,

    /// Whether to allow XML entities (default: true)
    pub allow_entities: bool,

    /// Whether to preserve XML namespaces (default: true)
    pub preserve_namespaces: bool,

    /// Whether to validate RDF semantics (default: true)
    pub validate_rdf: bool,

    /// Maximum nested depth for RDF structures (default: 100)
    pub max_depth: usize,

    /// Whether to use strict RDF/XML compliance (default: false)
    pub strict_mode: bool,

    /// Base URI for resolving relative URIs
    pub base_uri: Option<String>,

    /// RDF version mode (default: Auto)
    pub rdf_version: RdfVersionMode,

    /// Reification handling mode (default: Auto)
    pub reification_mode: ReificationMode,

    /// Whether to parse rdf:reifies (RDF 1.2) (default: true)
    pub parse_rdf_reifies: bool,

    /// Strict RDF 1.1 mode - reject RDF 1.2 features (default: false)
    pub strict_rdf11_mode: bool,

    /// IRI validation mode (default: RFC3987 for RDF 1.2)
    pub iri_validation_mode: IriValidationMode,

    /// Validate blank node labels for RDF 1.2 well-formedness (default: false)
    pub validate_blank_nodes: bool,
}

impl Default for RdfXmlParserConfig {
    fn default() -> Self {
        Self {
            validate_xml: true,
            allow_entities: true,
            preserve_namespaces: true,
            validate_rdf: true,
            max_depth: 100,
            strict_mode: false,
            base_uri: None,
            rdf_version: RdfVersionMode::Auto,
            reification_mode: ReificationMode::Auto,
            parse_rdf_reifies: true,
            strict_rdf11_mode: false,
            iri_validation_mode: IriValidationMode::RFC3987,
            validate_blank_nodes: false, // Lenient for backward compatibility
        }
    }
}

/// RDF/XML Parser
#[derive(Debug, Clone)]
pub struct RdfXmlParser {
    config: RdfXmlParserConfig,
}

impl RdfXmlParser {
    /// Create a new RDF/XML parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RdfXmlParserConfig::default(),
        }
    }

    /// Create a new RDF/XML parser with custom configuration
    #[must_use]
    pub fn with_config(config: RdfXmlParserConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &RdfXmlParserConfig {
        &self.config
    }

    /// Set a new configuration
    pub fn set_config(&mut self, config: RdfXmlParserConfig) {
        self.config = config;
    }

    /// Parse RDF/XML content into an ontology
    pub fn parse_string(&self, content: &str) -> Result<Ontology> {
        // Only validate as RDF/XML if content actually contains RDF markers
        // Otherwise it might be OWL/XML or other XML format
        if content.contains("rdf:RDF") || content.contains("xmlns:rdf") {
            let validator = super::validation::SyntaxValidator::new();
            validator.validate_rdf_xml(content)?;
        }

        // Basic XML validation if enabled
        if self.config.validate_xml {
            self.validate_xml_structure(content)?;
        }

        let mut ontology = Ontology::new();

        // Basic RDF/XML structure detection and parsing
        if content.contains("<rdf:RDF") || content.contains("<RDF") {
            // This is a full RDF/XML document
            self.parse_rdf_xml_content(content, &mut ontology)?;
        } else if content.trim().starts_with('<') {
            // This might be an RDF/XML fragment (standalone Description, property, etc.)
            // Wrap it in an RDF document and parse
            let wrapped = format!(
                r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:ex="http://example.org/">
{content}
</rdf:RDF>"#
            );
            self.parse_rdf_xml_content(&wrapped, &mut ontology)?;
        } else {
            return Err(Error::ParseError(
                "Invalid RDF/XML document: missing RDF root element".to_string(),
            ));
        }

        Ok(ontology)
    }

    /// Parse RDF/XML content and extract ontology elements
    fn parse_rdf_xml_content(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Parse namespace declarations
        self.extract_namespaces(content, ontology)?;

        // Legacy entity extractors are disabled — extract_rdf_triples handles
        // all OWL axiom types (SubClassOf, ClassAssertion, Declarations, etc.)
        // via proper quick-xml traversal of Subject-Predicate-Object triples.
        // The old regex-based extract_classes, extract_properties, and
        // extract_individuals incorrectly created NamedIndividual declarations
        // from class IRIs, conflicting with extract_rdf_triples.
        //
        // self.extract_classes(content, ontology)?;
        // self.extract_properties(content, ontology)?;
        // self.extract_individuals(content, ontology)?;
        // self.extract_axioms(content, ontology)?;

        // Parse RDF triples from XML for full OWL-to-RDF roundtrip support
        self.extract_rdf_triples(content, ontology)?;

        // Parse reification patterns (RDF 1.1) and rdf:reifies (RDF 1.2)
        self.extract_reifications(content, ontology)?;

        Ok(())
    }

    /// Extract namespace declarations from RDF/XML
    fn extract_namespaces(&self, content: &str, _ontology: &mut Ontology) -> Result<()> {
        // Look for xmlns declarations
        for line in content.lines() {
            if line.contains("xmlns") {
                // Extract namespace URIs and prefixes
                // This is a simplified extraction
                if let Some(ns_start) = line.find("xmlns:")
                    && let Some(eq_pos) = line[ns_start..].find('=')
                    && let Some(quote_start) = line[ns_start + eq_pos..].find('"')
                    && let Some(quote_end) = line[ns_start + eq_pos + quote_start + 1..].find('"')
                {
                    let prefix = &line[ns_start + 6..ns_start + eq_pos];
                    let uri = &line[ns_start + eq_pos + quote_start + 1
                        ..ns_start + eq_pos + quote_start + 1 + quote_end];

                    // Add to ontology prefixes if the ontology supports it
                    // For now, we'll store this information internally
                    log::debug!("Found namespace: {prefix} -> {uri}");
                }
            }
        }
        Ok(())
    }

    /// Extract class declarations
    fn extract_classes(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Enhanced RDF/XML class extraction with proper XML parsing
        // This follows the RDF/XML specification more closely

        use quick_xml::{Reader, events::Event};

        let mut reader = Reader::from_str(content);
        // reader.trim_text(true); // Removed as this method doesn't exist in current quick-xml

        let mut buf: Vec<u8> = Vec::new();
        let mut _current_element = None;
        let mut current_attributes = std::collections::HashMap::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    _current_element = Some(name.clone());

                    // Parse attributes
                    current_attributes.clear();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);
                        current_attributes.insert(key.to_string(), value.to_string());
                    }

                    // Check for owl:Class elements
                    if (name == "owl:Class" || name.ends_with(":Class"))
                        && let Some(class_iri) = self.extract_resource_iri(&current_attributes)
                    {
                        self.add_class_declaration(class_iri, ontology)?;
                    }

                    // Check for rdf:type relationships to owl:Class
                    if (name == "rdf:Description" || name.contains("Description"))
                        && let Some(subject_iri) = self.extract_resource_iri(&current_attributes)
                    {
                        // Look for nested rdf:type owl:Class
                        if self.has_class_type(&current_attributes) {
                            self.add_class_declaration(subject_iri, ontology)?;
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    // Parse attributes for self-closing elements
                    current_attributes.clear();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);
                        current_attributes.insert(key.to_string(), value.to_string());
                    }

                    // Handle self-closing owl:Class elements
                    if (name == "owl:Class" || name.ends_with(":Class"))
                        && let Some(class_iri) = self.extract_resource_iri(&current_attributes)
                    {
                        self.add_class_declaration(class_iri, ontology)?;
                    }
                }
                Ok(Event::Text(ref e)) => {
                    // Handle text content if needed for class declarations
                    let _text = String::from_utf8_lossy(e).to_string();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    // Return XML parsing error as fatal error
                    return Err(Error::ParseError(format!("XML parsing error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    /// Extract resource IRI from XML attributes
    fn extract_resource_iri(
        &self,
        attributes: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        // Check for rdf:about
        if let Some(about) = attributes.get("rdf:about") {
            return Some(self.resolve_uri(about));
        }

        // Check for rdf:ID
        if let Some(id) = attributes.get("rdf:ID") {
            return Some(self.resolve_fragment_id(id));
        }

        // Check for rdf:nodeID for blank nodes
        if let Some(node_id) = attributes.get("rdf:nodeID") {
            return Some(format!("_:{node_id}"));
        }

        None
    }

    /// Check if attributes indicate this is a class
    fn has_class_type(&self, attributes: &std::collections::HashMap<String, String>) -> bool {
        // Check for rdf:type attribute pointing to owl:Class
        if let Some(type_val) = attributes.get("rdf:type") {
            return type_val == "owl:Class" || type_val.ends_with("#Class");
        }

        false
    }

    /// Add class declaration to ontology
    fn add_class_declaration(&self, class_iri: String, ontology: &mut Ontology) -> Result<()> {
        let iri = crate::ontology::IRI::new(&class_iri);

        // Add declaration axiom
        let decl_axiom = crate::ontology::axioms::DeclarationAxiom {
            id: ontology.axioms().len() as u64,
            entity: crate::ontology::axioms::Entity::Class(iri),
        };
        ontology.add_axiom(crate::ontology::axioms::Axiom::Declaration(decl_axiom));

        Ok(())
    }

    /// Resolve relative URI against base URI
    fn resolve_uri(&self, uri: &str) -> String {
        if uri.starts_with("http://") || uri.starts_with("https://") {
            uri.to_string()
        } else if let Some(base) = &self.config.base_uri {
            format!("{base}{uri}")
        } else {
            uri.to_string()
        }
    }

    /// Resolve fragment ID against base URI
    fn resolve_fragment_id(&self, id: &str) -> String {
        if let Some(base) = &self.config.base_uri {
            format!("{base}#{id}")
        } else {
            format!("#{id}")
        }
    }

    /// Extract property declarations
    fn extract_properties(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Look for owl:ObjectProperty and owl:DatatypeProperty declarations
        let obj_prop_patterns = [
            r#"<owl:ObjectProperty rdf:about="([^"]+)""#,
            r#"<owl:ObjectProperty rdf:ID="([^"]+)""#,
        ];

        for pattern in &obj_prop_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for caps in regex.captures_iter(content) {
                    if let Some(prop_iri) = caps.get(1) {
                        let iri = crate::ontology::IRI::new(prop_iri.as_str());
                        let decl_axiom = crate::ontology::axioms::DeclarationAxiom {
                            id: ontology.axioms().len() as u64,
                            entity: crate::ontology::axioms::Entity::ObjectProperty(iri),
                        };
                        ontology.add_axiom(crate::ontology::axioms::Axiom::Declaration(decl_axiom));
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract individual declarations
    fn extract_individuals(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Look for individual declarations and class assertions
        let individual_patterns = [
            r#"<owl:NamedIndividual rdf:about="([^"]+)""#,
            r#"<([^>\s]+)\s+rdf:about="([^"]+)"[^>]*>"#,
        ];

        for pattern in &individual_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for caps in regex.captures_iter(content) {
                    if caps.len() >= 2 {
                        let ind_iri = if caps.len() == 2 {
                            caps.get(1)
                                .expect("Failed to get regex capture group")
                                .as_str()
                        } else {
                            caps.get(2)
                                .expect("Failed to get regex capture group")
                                .as_str()
                        };

                        let iri = crate::ontology::IRI::new(ind_iri);

                        let decl_axiom = crate::ontology::axioms::DeclarationAxiom {
                            id: ontology.axioms().len() as u64,
                            entity: crate::ontology::axioms::Entity::NamedIndividual(iri),
                        };
                        ontology.add_axiom(crate::ontology::axioms::Axiom::Declaration(decl_axiom));
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract axioms from RDF/XML
    fn extract_axioms(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Look for subclass relationships
        self.extract_subclass_axioms(content, ontology)?;

        // Look for property assertions
        self.extract_property_assertions(content, ontology)?;

        Ok(())
    }

    /// Extract RDF triples from RDF/XML by walking XML elements.
    /// Child elements within a subject element are interpreted as predicate-object pairs.
    fn extract_rdf_triples(&self, content: &str, ontology: &mut crate::ontology::Ontology) -> Result<()> {
        use std::collections::HashMap;
        use quick_xml::Reader;
        use quick_xml::events::Event;
        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut subject: Option<String> = None;
        let mut depth: usize = 0;
        let mut subj_depth: usize = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let mut attrs = HashMap::new();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(&attr.value).to_string();
                        attrs.insert(key, value);
                    }
                    depth += 1;
                    let has_about = attrs.keys().any(|k| k.ends_with(":about") || *k == "about" || k.ends_with(":ID"));
                    if has_about {
                        if let Some(s) = self.extract_resource_iri(&attrs) {
                            subject = Some(s);
                            subj_depth = depth;
                        }
                    } else if subject.is_some() && depth == subj_depth + 1 {
                        let pred = self.resolve_element_iri(&name);
                        let obj = attrs.get("rdf:resource")
                            .or_else(|| attrs.iter().find(|(k,_)| k.ends_with(":resource")).map(|(_,v)| v));
                        if let (Some(s), Some(o)) = (subject.clone(), obj) {
                            process_owl_triple_inline(ontology, &s, &pred, o);
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let mut attrs = HashMap::new();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(&attr.value).to_string();
                        attrs.insert(key, value);
                    }
                    if subject.is_some() && depth == subj_depth {
                        let pred = self.resolve_element_iri(&name);
                        let obj = attrs.get("rdf:resource")
                            .or_else(|| attrs.iter().find(|(k,_)| k.ends_with(":resource")).map(|(_,v)| v))
                            .or_else(|| attrs.iter().find(|(k,_)| k.ends_with(":about") || *k == "about").map(|(_,v)| v));
                        if let (Some(s), Some(o)) = (subject.clone(), obj) {
                            process_owl_triple_inline(ontology, &s, &pred, o);
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    if depth == subj_depth { subject = None; }
                    depth = depth.saturating_sub(1);
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    /// Resolve a prefixed XML element name to its full IRI.
    fn resolve_element_iri(&self, element_name: &str) -> String {
        if let Some(colon) = element_name.find(':') {
            let prefix = &element_name[..colon];
            let local = &element_name[colon + 1..];
            match prefix {
                "rdf" => format!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{local}"),
                "rdfs" => format!("http://www.w3.org/2000/01/rdf-schema#{local}"),
                "owl" => format!("http://www.w3.org/2002/07/owl#{local}"),
                "xsd" => format!("http://www.w3.org/2001/XMLSchema#{local}"),
                _ => element_name.to_string(),
            }
        } else {
            element_name.to_string()
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Legacy regex-based extraction functions (kept for compatibility)
    // ══════════════════════════════════════════════════════════════════════════

    /// Extract subclass axioms
    fn extract_subclass_axioms(&self, content: &str, _ontology: &mut Ontology) -> Result<()> {
        // Look for rdfs:subClassOf relationships
        let subclass_pattern = r#"<rdfs:subClassOf rdf:resource="([^"]+)""#;

        if let Ok(regex) = regex::Regex::new(subclass_pattern) {
            for caps in regex.captures_iter(content) {
                if let Some(superclass_iri) = caps.get(1) {
                    // We would need more context to get the subclass IRI
                    // This is a simplified extraction
                    log::debug!(
                        "Found subclass relationship to: {}",
                        superclass_iri.as_str()
                    );
                }
            }
        }

        Ok(())
    }

    /// Extract property assertions
    fn extract_property_assertions(&self, _content: &str, _ontology: &mut Ontology) -> Result<()> {
        // This would involve more complex parsing to extract property assertions
        // from the RDF/XML structure
        Ok(())
    }

    /// Extract reification patterns and rdf:reifies
    fn extract_reifications(&self, content: &str, ontology: &mut Ontology) -> Result<()> {
        // Check for strict RDF 1.1 mode - reject RDF 1.2 features
        if self.config.strict_rdf11_mode && content.contains("rdf:reifies") {
            return Err(Error::ontology_parsing(
                "RDF 1.2 rdf:reifies not allowed in strict RDF 1.1 mode".to_string(),
            ));
        }

        // Detect rdf:reifies (RDF 1.2)
        let has_rdf_reifies = content.contains("rdf:reifies");

        // Detect standard reification patterns (RDF 1.1)
        let has_reification = content.contains("rdf:Statement")
            || (content.contains("rdf:subject")
                && content.contains("rdf:predicate")
                && content.contains("rdf:object"));

        // Determine which mode to use
        let reification_mode = match self.config.reification_mode {
            ReificationMode::Preserve => ReificationMode::Preserve,
            ReificationMode::ConvertToQuotedTriples => ReificationMode::ConvertToQuotedTriples,
            ReificationMode::Auto => {
                // Auto-detect based on document content
                if has_rdf_reifies {
                    ReificationMode::ConvertToQuotedTriples
                } else {
                    ReificationMode::Preserve
                }
            }
        };

        // Parse rdf:reifies if present and enabled
        if has_rdf_reifies && self.config.parse_rdf_reifies {
            let reified_triples = self.extract_rdf_reifies(content)?;

            // Store reified triples in ontology RDF graph
            let graph = ontology.get_or_create_rdf_graph();
            for (reifier_id, triple) in reified_triples {
                // Create a reifier RdfTerm from the reifier_id string
                let reifier_term = if reifier_id.starts_with("_:") {
                    crate::semantics::RdfTerm::BlankNode(reifier_id)
                } else {
                    match url::Url::parse(&reifier_id) {
                        Ok(url) => crate::semantics::RdfTerm::Iri(url),
                        Err(_) => crate::semantics::RdfTerm::BlankNode(format!("_:{reifier_id}")),
                    }
                };

                // Add `reifier rdf:reifies <<s p o>>` using the RDF 1.2 API
                graph.add_reifying_triple(reifier_term, triple);
            }
        }

        // Parse standard reification patterns if present
        if has_reification {
            let reifications = self.extract_rdf11_reifications(content)?;

            match reification_mode {
                ReificationMode::ConvertToQuotedTriples => {
                    // Convert RDF 1.1 reification to quoted triples
                    let graph = ontology.get_or_create_rdf_graph();

                    for (stmt_id, triple) in reifications {
                        // Create a quoted triple for RDF-star representation
                        let quoted_triple =
                            crate::semantics::RdfTerm::QuotedTriple(Box::new(triple.clone()));

                        // Store the original triple in the graph
                        graph.add_triple(triple);

                        // Store association between reification ID and quoted triple
                        // This can be used later for querying
                        // Create a meta-triple linking the statement ID to the quoted triple
                        // Create a unique IRI for this statement node
                        let stmt_iri = crate::semantics::RdfTerm::Iri(
                            url::Url::parse(&format!("http://example.org/stmt/{stmt_id}"))
                                .map_err(|e| {
                                    crate::Error::ontology_parsing(format!(
                                        "Invalid statement IRI for stmt '{stmt_id}': {e}"
                                    ))
                                })?,
                        );
                        let reifies_pred = crate::semantics::RdfTerm::Iri(RDF_REIFIES.clone());

                        let meta_triple = crate::semantics::Triple {
                            subject: stmt_iri,
                            predicate: reifies_pred,
                            object: quoted_triple,
                        };
                        graph.add_triple(meta_triple);
                    }
                }
                ReificationMode::Preserve => {
                    // Keep RDF 1.1 reification as-is in the graph
                    let graph = ontology.get_or_create_rdf_graph();

                    for (stmt_id, triple) in reifications {
                        // Store the reification pattern as separate triples
                        let stmt_iri = crate::semantics::RdfTerm::Iri(
                            url::Url::parse(&format!("http://example.org/stmt/{stmt_id}"))
                                .map_err(|e| {
                                    crate::Error::ontology_parsing(format!(
                                        "Invalid statement IRI for stmt '{stmt_id}': {e}"
                                    ))
                                })?,
                        );

                        // rdf:type rdf:Statement
                        graph.add_triple(crate::semantics::Triple {
                            subject: stmt_iri.clone(),
                            predicate: crate::semantics::RdfTerm::Iri(RDF_TYPE.clone()),
                            object: crate::semantics::RdfTerm::Iri(RDF_STATEMENT.clone()),
                        });

                        // rdf:subject
                        graph.add_triple(crate::semantics::Triple {
                            subject: stmt_iri.clone(),
                            predicate: crate::semantics::RdfTerm::Iri(RDF_SUBJECT.clone()),
                            object: triple.subject.clone(),
                        });

                        // rdf:predicate
                        graph.add_triple(crate::semantics::Triple {
                            subject: stmt_iri.clone(),
                            predicate: crate::semantics::RdfTerm::Iri(RDF_PREDICATE.clone()),
                            object: triple.predicate.clone(),
                        });

                        // rdf:object
                        graph.add_triple(crate::semantics::Triple {
                            subject: stmt_iri,
                            predicate: crate::semantics::RdfTerm::Iri(RDF_OBJECT.clone()),
                            object: triple.object.clone(),
                        });
                    }
                }
                ReificationMode::Auto => {
                    // This case should already be handled above in mode selection
                    return Err(crate::Error::ontology_parsing(
                        "Auto reification mode should have been resolved before this point"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Extract rdf:reifies patterns (RDF 1.2)
    /// Returns a list of (`reifying_resource`, `reified_triple`) pairs
    fn extract_rdf_reifies(&self, content: &str) -> Result<Vec<(String, RdfTriple)>> {
        let mut results = Vec::new();

        // Pattern: <rdf:Description rdf:about="resource"><rdf:reifies><<triple>></rdf:reifies></rdf:Description>
        // Simplified regex for rdf:reifies detection
        if let Ok(regex) = regex::Regex::new(r"rdf:reifies[^>]*>(.*?)</.*?:reifies>") {
            for caps in regex.captures_iter(content) {
                if let Some(reified_content) = caps.get(1) {
                    let content_str = reified_content.as_str().trim();

                    // Check if this looks like a quoted triple reference
                    // In RDF/XML, rdf:reifies would reference a triple resource
                    // Format: rdf:reifies rdf:resource="#triple1"
                    // We'll parse this as a placeholder for now
                    // TODO: Full RDF/XML rdf:reifies parsing

                    // For now, create placeholder triple
                    let placeholder_triple = RdfTriple {
                        subject: RdfTerm::BlankNode("_:s".to_string()),
                        predicate: RdfTerm::BlankNode("_:p".to_string()),
                        object: RdfTerm::BlankNode("_:o".to_string()),
                    };

                    results.push((content_str.to_string(), placeholder_triple));
                }
            }
        }

        Ok(results)
    }

    /// Extract RDF 1.1 reification patterns
    /// Returns a map of statement ID to triple
    fn extract_rdf11_reifications(&self, content: &str) -> Result<HashMap<String, RdfTriple>> {
        let mut reifications: HashMap<String, HashMap<String, String>> = HashMap::new();

        // Pattern: <rdf:Statement rdf:about="#stmt1">
        //            <rdf:subject rdf:resource="..."/>
        //            <rdf:predicate rdf:resource="..."/>
        //            <rdf:object rdf:resource="..."/>
        //          </rdf:Statement>

        // Extract Statement elements
        if let Ok(stmt_regex) = regex::Regex::new(r#"<rdf:Statement[^>]*rdf:about="([^"]+)"[^>]*>"#)
        {
            for stmt_caps in stmt_regex.captures_iter(content) {
                if let Some(stmt_id) = stmt_caps.get(1) {
                    let stmt_id_str = stmt_id.as_str().to_string();
                    reifications.entry(stmt_id_str).or_default();
                }
            }
        }

        // Extract subject, predicate, object for each statement
        if let Ok(subj_regex) = regex::Regex::new(r#"<rdf:subject rdf:resource="([^"]+)""#) {
            for caps in subj_regex.captures_iter(content) {
                if let Some(subject) = caps.get(1) {
                    // Find which statement this belongs to (simplified approach)
                    // In real implementation, would need to track XML nesting
                    for reif in reifications.values_mut() {
                        if !reif.contains_key("subject") {
                            reif.insert("subject".to_string(), subject.as_str().to_string());
                            break;
                        }
                    }
                }
            }
        }

        if let Ok(pred_regex) = regex::Regex::new(r#"<rdf:predicate rdf:resource="([^"]+)""#) {
            for caps in pred_regex.captures_iter(content) {
                if let Some(predicate) = caps.get(1) {
                    for reif in reifications.values_mut() {
                        if reif.contains_key("subject") && !reif.contains_key("predicate") {
                            reif.insert("predicate".to_string(), predicate.as_str().to_string());
                            break;
                        }
                    }
                }
            }
        }

        if let Ok(obj_regex) = regex::Regex::new(r#"<rdf:object rdf:resource="([^"]+)""#) {
            for caps in obj_regex.captures_iter(content) {
                if let Some(object) = caps.get(1) {
                    for reif in reifications.values_mut() {
                        if reif.contains_key("predicate") && !reif.contains_key("object") {
                            reif.insert("object".to_string(), object.as_str().to_string());
                            break;
                        }
                    }
                }
            }
        }

        // Convert to RdfTriple
        let mut result = HashMap::new();
        for (stmt_id, components) in reifications {
            if let (Some(subject), Some(predicate), Some(object)) = (
                components.get("subject"),
                components.get("predicate"),
                components.get("object"),
            ) {
                // Parse URIs into RdfTerms
                let subject_term = if let Ok(url) = url::Url::parse(subject) {
                    RdfTerm::Iri(url)
                } else if subject.starts_with("_:") {
                    RdfTerm::BlankNode(subject.clone())
                } else {
                    continue; // Skip invalid terms
                };

                let predicate_term = if let Ok(url) = url::Url::parse(predicate) {
                    RdfTerm::Iri(url)
                } else {
                    continue;
                };

                let object_term = if let Ok(url) = url::Url::parse(object) {
                    RdfTerm::Iri(url)
                } else if object.starts_with("_:") {
                    RdfTerm::BlankNode(object.clone())
                } else {
                    continue;
                };

                let triple = RdfTriple {
                    subject: subject_term,
                    predicate: predicate_term,
                    object: object_term,
                };

                result.insert(stmt_id, triple);
            }
        }

        Ok(result)
    }

    /// Validate XML structure
    fn validate_xml_structure(&self, content: &str) -> Result<()> {
        // Basic XML well-formedness check
        let mut tag_stack = Vec::new();
        let mut in_tag = false;
        let mut tag_content = String::new();

        for ch in content.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    tag_content.clear();
                }
                '>' => {
                    if in_tag {
                        in_tag = false;
                        self.process_xml_tag(&tag_content, &mut tag_stack)?;
                    }
                }
                _ => {
                    if in_tag {
                        tag_content.push(ch);
                    }
                }
            }
        }

        if !tag_stack.is_empty() {
            return Err(Error::xml_parsing("Unclosed XML tags detected".to_string()));
        }

        Ok(())
    }

    /// Process an XML tag during validation
    fn process_xml_tag(&self, tag_content: &str, tag_stack: &mut Vec<String>) -> Result<()> {
        let tag_content = tag_content.trim();

        if tag_content.is_empty() {
            return Ok(());
        }

        if let Some(stripped) = tag_content.strip_prefix('/') {
            // Closing tag
            let tag_name = stripped.split_whitespace().next().unwrap_or("");
            if let Some(last_tag) = tag_stack.pop() {
                if last_tag != tag_name {
                    return Err(Error::xml_parsing(format!(
                        "Mismatched XML tags: expected {last_tag}, found {tag_name}"
                    )));
                }
            } else {
                return Err(Error::xml_parsing(format!(
                    "Unexpected closing tag: {tag_name}"
                )));
            }
        } else if tag_content.ends_with('/') {
            // Self-closing tag - no action needed
        } else if tag_content.starts_with('?') {
            // XML declaration - no action needed
        } else if tag_content.starts_with('!') {
            // Comment or DTD - no action needed
        } else {
            // Opening tag
            let tag_name = tag_content
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if !tag_name.is_empty() {
                tag_stack.push(tag_name);
            }
        }

        Ok(())
    }
}

impl Default for RdfXmlParser {
    fn default() -> Self {
        Self::new()
    }
}

/// RDF/XML Serializer
#[derive(Debug, Clone)]
pub struct RdfXmlSerializer {}

impl RdfXmlSerializer {
    /// Create a new RDF/XML serializer
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RdfXmlSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl OntologySerializer for RdfXmlSerializer {
    fn serialize(&self, ontology: &Ontology) -> std::result::Result<String, Error> {
        let mut result = String::new();

        // XML header and RDF root
        result.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
        result.push_str("         xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"\n");
        result.push_str("         xmlns:owl=\"http://www.w3.org/2002/07/owl#\">\n");

        // Ontology declaration
        let iri_str = ontology
            .get_iri()
            .map_or("http://example.org/ontology", |iri| iri.as_str());
        result.push_str(&format!("  <owl:Ontology rdf:about=\"{iri_str}\" />\n\n"));

        // Serialize classes
        if !ontology.classes().is_empty() {
            for (_, class) in ontology.classes() {
                result.push_str(&format!(
                    "  <owl:Class rdf:about=\"{}\" />\n",
                    class.iri.as_str()
                ));
            }
            result.push('\n');
        }

        // Serialize object properties
        let object_properties = ontology.object_properties();
        if !object_properties.is_empty() {
            for prop in object_properties {
                result.push_str(&format!(
                    "  <owl:ObjectProperty rdf:about=\"{}\" />\n",
                    prop.iri.as_str()
                ));
            }
            result.push('\n');
        }

        // Serialize data properties from axioms
        let mut data_properties = std::collections::HashSet::new();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::DataPropertyAssertion(assertion) => {
                    let crate::ontology::DataPropertyExpression::DataProperty(prop) =
                        &assertion.property;
                    data_properties.insert(prop.clone());
                }
                crate::ontology::Axiom::SubDataPropertyOf(sub_prop) => {
                    let crate::ontology::DataPropertyExpression::DataProperty(sub) =
                        &sub_prop.sub_property;
                    data_properties.insert(sub.clone());
                    let crate::ontology::DataPropertyExpression::DataProperty(super_prop) =
                        &sub_prop.super_property;
                    data_properties.insert(super_prop.clone());
                }
                _ => {}
            }
        }

        if !data_properties.is_empty() {
            for prop in data_properties {
                result.push_str(&format!(
                    "  <owl:DatatypeProperty rdf:about=\"{}\" />\n",
                    prop.iri.as_str()
                ));
            }
            result.push('\n');
        }

        // Serialize individuals
        if !ontology.individuals().is_empty() {
            for (_, individual) in ontology.individuals() {
                if let crate::ontology::Individual::Named(named) = individual {
                    result.push_str(&format!(
                        "  <owl:NamedIndividual rdf:about=\"{}\" />\n",
                        named.iri.as_str()
                    ));
                }
            }
            result.push('\n');
        }

        // ── Axiom serialization via shared OWL-to-RDF mapping ──────────
        use crate::semantics::owl_rdf_mapping::{axiom_to_triples, BlankNodeCounter};
        use crate::semantics::RdfTerm;
        let mut counter = BlankNodeCounter::new();
        for axiom in ontology.axioms() {
            let triples = axiom_to_triples(axiom, &mut counter);
            for triple in triples {
                let subj_str = match &triple.subject {
                    RdfTerm::Iri(url) => url.as_str().to_string(),
                    RdfTerm::BlankNode(id) => format!("#{id}"),
                    _ => continue,
                };
                let pred_str = match &triple.predicate {
                    RdfTerm::Iri(url) => url.as_str().to_string(),
                    _ => continue,
                };
                result.push_str(&format!("  <rdf:Description rdf:about=\"{subj_str}\">\n"));
                match &triple.object {
                    RdfTerm::Iri(url) => {
                        result.push_str(&format!("    <{} rdf:resource=\"{}\"/>\n",
                            pred_to_element_name(&pred_str), url));
                    }
                    RdfTerm::Literal { value, datatype, language, .. } => {
                        if let Some(dt) = datatype {
                            result.push_str(&format!("    <{} rdf:datatype=\"{}\">{}</{}>\n",
                                pred_to_element_name(&pred_str), dt, value, pred_to_element_name(&pred_str)));
                        } else if let Some(lang) = language {
                            result.push_str(&format!("    <{} xml:lang=\"{}\">{}</{}>\n",
                                pred_to_element_name(&pred_str), lang, value, pred_to_element_name(&pred_str)));
                        } else {
                            result.push_str(&format!("    <{}>{}</{}>\n",
                                pred_to_element_name(&pred_str), value, pred_to_element_name(&pred_str)));
                        }
                    }
                    _ => {}
                }
                result.push_str("  </rdf:Description>\n");
            }
        }

        result.push_str("</rdf:RDF>\n");
        Ok(result)
    }
}

/// Parse RDF/XML from string content using default parser
pub fn parse(content: &str) -> Result<Ontology> {
    let parser = RdfXmlParser::new();
    parser.parse_string(content)
}

/// Parse RDF/XML from file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let file = File::open(path).map_err(|e| Error::io(format!("Failed to open file: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse(&content)
}

/// Save ontology to RDF/XML file
pub fn save_file<P: AsRef<Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let serializer = RdfXmlSerializer::new();
    serializer.serialize_to_file(ontology, path.as_ref())
}

/// Serialize an axiom to RDF/XML format
#[allow(dead_code)]
fn serialize_axiom_to_rdf_xml<W: Write>(
    writer: &mut W,
    axiom: &crate::ontology::axioms::Axiom,
) -> Result<()> {
    use crate::ontology::axioms::Axiom;

    match axiom {
        Axiom::SubClassOf(sub_axiom) => {
            writeln!(
                writer,
                "  <!-- SubClassOf: {} rdfs:subClassOf {} -->",
                sub_axiom.subclass, sub_axiom.superclass
            )?;
            writeln!(
                writer,
                "  <rdf:Description rdf:about=\"{}\">",
                sub_axiom.subclass
            )?;
            writeln!(
                writer,
                "    <rdfs:subClassOf rdf:resource=\"{}\" />",
                sub_axiom.superclass
            )?;
            writeln!(writer, "  </rdf:Description>")?;
        }
        Axiom::ClassAssertion(class_axiom) => {
            if let crate::ontology::Individual::Named(named) = &class_axiom.individual {
                writeln!(
                    writer,
                    "  <!-- ClassAssertion: {} rdf:type {} -->",
                    named.iri, class_axiom.class
                )?;
                writeln!(writer, "  <rdf:Description rdf:about=\"{}\">", named.iri)?;
                writeln!(
                    writer,
                    "    <rdf:type rdf:resource=\"{}\" />",
                    class_axiom.class
                )?;
                writeln!(writer, "  </rdf:Description>")?;
            }
        }
        Axiom::ObjectPropertyAssertion(prop_axiom) => {
            if let (
                crate::ontology::Individual::Named(subj),
                crate::ontology::Individual::Named(obj),
            ) = (&prop_axiom.source, &prop_axiom.target)
            {
                writeln!(writer, "  <!-- ObjectPropertyAssertion -->")?;
                writeln!(writer, "  <rdf:Description rdf:about=\"{}\">", subj.iri)?;
                writeln!(
                    writer,
                    "    <{} rdf:resource=\"{}\" />",
                    prop_axiom.property, obj.iri
                )?;
                writeln!(writer, "  </rdf:Description>")?;
            }
        }
        _ => {
            // For other axiom types, add a comment for now
            writeln!(
                writer,
                "  <!-- Axiom serialized via shared OWL-to-RDF mapping module -->"
            )?;
        }
    }
    Ok(())
}

/// Process a single RDF triple (subject, predicate, object) into an OWL axiom.
/// This function mirrors the Turtle parser's `process_enhanced_triple`.
/// Uses pre-extracted axiom IDs to avoid borrow conflicts.
fn process_owl_triple_inline(
    ontology: &mut crate::ontology::Ontology,
    subject: &str,
    predicate: &str,
    object: &str,
) {
    
    use crate::ontology::*;
    let id = ontology.next_axiom_id();
    match predicate {
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" => match object {
            "http://www.w3.org/2002/07/owl#Class" => ontology.add_axiom(Axiom::Declaration(
                DeclarationAxiom { id, entity: Entity::Class(IRI::new(subject)) },
            )),
            "http://www.w3.org/2002/07/owl#ObjectProperty" => ontology.add_axiom(Axiom::Declaration(
                DeclarationAxiom { id, entity: Entity::ObjectProperty(IRI::new(subject)) },
            )),
            "http://www.w3.org/2002/07/owl#DatatypeProperty" => ontology.add_axiom(Axiom::Declaration(
                DeclarationAxiom { id, entity: Entity::DataProperty(IRI::new(subject)) },
            )),
            "http://www.w3.org/2002/07/owl#NamedIndividual" => ontology.add_axiom(Axiom::Declaration(
                DeclarationAxiom { id, entity: Entity::NamedIndividual(IRI::new(subject)) },
            )),
            "http://www.w3.org/2002/07/owl#AnnotationProperty" => ontology.add_axiom(Axiom::Declaration(
                DeclarationAxiom { id, entity: Entity::AnnotationProperty(IRI::new(subject)) },
            )),
            "http://www.w3.org/2002/07/owl#Ontology" => {
                ontology.set_iri(IRI::new(subject));
            }
            "http://www.w3.org/2002/07/owl#FunctionalProperty" => {
                ontology.add_axiom(Axiom::FunctionalObjectProperty(
                    FunctionalObjectPropertyAxiom { id, property: ObjectPropertyExpression::ObjectProperty(
                        ObjectProperty { iri: IRI::new(subject) }), annotations: vec![] },
                ));
            }
            "http://www.w3.org/2002/07/owl#TransitiveProperty" => {
                ontology.add_axiom(Axiom::TransitiveObjectProperty(
                    TransitiveObjectPropertyAxiom { id, property: ObjectPropertyExpression::ObjectProperty(
                        ObjectProperty { iri: IRI::new(subject) }), annotations: vec![] },
                ));
            }
            _ => {
                let cid = ontology.next_axiom_id();
                ontology.add_axiom(Axiom::ClassAssertion(
                    ClassAssertionAxiom { id: cid,
                        individual: Individual::Named(NamedIndividual { iri: IRI::new(subject) }),
                        class: ClassExpression::Class(Class::new(IRI::new(object))),
                        annotations: vec![] },
                ));
            }
        },
        "http://www.w3.org/2000/01/rdf-schema#subClassOf" => ontology.add_axiom(Axiom::SubClassOf(
            SubClassOfAxiom { id,
                subclass: ClassExpression::Class(Class::new(IRI::new(subject))),
                superclass: ClassExpression::Class(Class::new(IRI::new(object))),
                annotations: vec![] },
        )),
        "http://www.w3.org/2002/07/owl#equivalentClass" => ontology.add_axiom(Axiom::EquivalentClasses(
            EquivalentClassesAxiom { id, classes: vec![
                ClassExpression::Class(Class::new(IRI::new(subject))),
                ClassExpression::Class(Class::new(IRI::new(object))),
            ], annotations: vec![] },
        )),
        "http://www.w3.org/2002/07/owl#disjointWith" => ontology.add_axiom(Axiom::DisjointClasses(
            DisjointClassesAxiom { id, classes: vec![
                ClassExpression::Class(Class::new(IRI::new(subject))),
                ClassExpression::Class(Class::new(IRI::new(object))),
            ], annotations: vec![] },
        )),
        _ => ontology.add_axiom(Axiom::ObjectPropertyAssertion(
            ObjectPropertyAssertionAxiom { id,
                property: ObjectPropertyExpression::ObjectProperty(
                    ObjectProperty { iri: IRI::new(predicate) }),
                source: Individual::Named(NamedIndividual { iri: IRI::new(subject) }),
                target: Individual::Named(NamedIndividual { iri: IRI::new(object) }),
                annotations: vec![] },
        )),
    }
}

/// Convert an OWL/RDF predicate IRI to a safe XML element name.
fn pred_to_element_name(pred_iri: &str) -> String {
    if pred_iri.contains("#subClassOf") { return "rdfs:subClassOf".into(); }
    if pred_iri.contains("#subPropertyOf") { return "rdfs:subPropertyOf".into(); }
    if pred_iri.contains("#domain") { return "rdfs:domain".into(); }
    if pred_iri.contains("#range") { return "rdfs:range".into(); }
    if pred_iri.contains("#type") { return "rdf:type".into(); }
    if pred_iri.contains("#equivalentClass") { return "owl:equivalentClass".into(); }
    if pred_iri.contains("#disjointWith") { return "owl:disjointWith".into(); }
    if pred_iri.contains("#sameAs") { return "owl:sameAs".into(); }
    if pred_iri.contains("#differentFrom") { return "owl:differentFrom".into(); }
    if pred_iri.contains("#inverseOf") { return "owl:inverseOf".into(); }
    if let Some(hash) = pred_iri.rfind('#') {
        let local = &pred_iri[hash + 1..];
        if pred_iri.contains("/rdf-schema") { return format!("rdfs:{local}"); }
        if pred_iri.contains("/owl") { return format!("owl:{local}"); }
        if pred_iri.contains("/rdf-syntax") { return format!("rdf:{local}"); }
        local.into()
    } else {
        pred_iri.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rdf11_reification() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Statement rdf:about="#stmt1">
    <rdf:subject rdf:resource="http://example.org/alice"/>
    <rdf:predicate rdf:resource="http://example.org/knows"/>
    <rdf:object rdf:resource="http://example.org/bob"/>
  </rdf:Statement>
  <rdf:Description rdf:about="#stmt1">
    <ex:certainty>high</ex:certainty>
  </rdf:Description>
</rdf:RDF>"##;

        let parser = RdfXmlParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_reification_to_quoted_triples() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Statement rdf:about="#stmt1">
    <rdf:subject rdf:resource="http://example.org/alice"/>
    <rdf:predicate rdf:resource="http://example.org/knows"/>
    <rdf:object rdf:resource="http://example.org/bob"/>
  </rdf:Statement>
</rdf:RDF>"##;

        let config = RdfXmlParserConfig {
            reification_mode: ReificationMode::ConvertToQuotedTriples,
            ..Default::default()
        };
        let parser = RdfXmlParser::with_config(config);
        let result = parser.parse_string(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_preserve_reification_rdf11_mode() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Statement rdf:about="#stmt1">
    <rdf:subject rdf:resource="http://example.org/alice"/>
    <rdf:predicate rdf:resource="http://example.org/knows"/>
    <rdf:object rdf:resource="http://example.org/bob"/>
  </rdf:Statement>
</rdf:RDF>"##;

        let config = RdfXmlParserConfig {
            rdf_version: RdfVersionMode::RDF11,
            reification_mode: ReificationMode::Preserve,
            ..Default::default()
        };
        let parser = RdfXmlParser::with_config(config);
        let result = parser.parse_string(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_strict_rdf11_rejects_rdf_reifies() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="http://example.org/statement1">
    <rdf:reifies rdf:resource="#triple1"/>
  </rdf:Description>
</rdf:RDF>"##;

        let config = RdfXmlParserConfig {
            strict_rdf11_mode: true,
            ..Default::default()
        };
        let parser = RdfXmlParser::with_config(config);
        let result = parser.parse_string(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_auto_detect_reification_mode() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Statement rdf:about="#stmt1">
    <rdf:subject rdf:resource="http://example.org/s"/>
    <rdf:predicate rdf:resource="http://example.org/p"/>
    <rdf:object rdf:resource="http://example.org/o"/>
  </rdf:Statement>
</rdf:RDF>"##;

        let config = RdfXmlParserConfig {
            reification_mode: ReificationMode::Auto,
            ..Default::default()
        };
        let parser = RdfXmlParser::with_config(config);
        let result = parser.parse_string(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_rdf11_reifications() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Statement rdf:about="#stmt1">
    <rdf:subject rdf:resource="http://example.org/alice"/>
    <rdf:predicate rdf:resource="http://example.org/knows"/>
    <rdf:object rdf:resource="http://example.org/bob"/>
  </rdf:Statement>
</rdf:RDF>"##;

        let parser = RdfXmlParser::new();
        let reifications = parser.extract_rdf11_reifications(content);
        assert!(reifications.is_ok());
        let reifs = reifications.unwrap();
        assert!(!reifs.is_empty());
    }

    #[test]
    fn test_basic_rdf_xml_parsing() {
        let content = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Class rdf:about="http://example.org/Person"/>
</rdf:RDF>"##;

        let parser = RdfXmlParser::new();
        let result = parser.parse_string(content);
        assert!(result.is_ok());
    }
}
