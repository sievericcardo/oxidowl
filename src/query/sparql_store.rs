//! In-process SPARQL store for Oxidowl
//!
//! This module provides a [`SparqlStore`] struct that wraps an `oxigraph::Store`
//! and exposes methods for loading ontology data, performing incremental triple
//! mutations, and executing SPARQL SELECT / ASK / CONSTRUCT / UPDATE operations
//! entirely in-process — no HTTP server required.
//!
//! It is the building block consumed by the SMOL interpreter's `TripleManager`
//! so that every interpreter step can query and update the semantic model
//! without network round-trips.
//!
//! # Feature gate
//! This module is compiled only when the `sparql-store` feature is enabled.

use crate::{
    Error, Result,
    ontology::{Axiom, ClassExpression, Ontology},
    reasoning::ReasoningService,
    semantics::{RdfTerm, Triple as OxidowlTriple},
};
use oxigraph::{
    io::{RdfFormat, RdfParser, RdfSerializer},
    model::{
        BlankNode as OxBlankNode, GraphName, GraphNameRef, Literal as OxLiteral, NamedNode,
        NamedNodeRef, NamedOrBlankNode, Quad, Term, Triple as OxTriple,
    },
    sparql::{QueryResults, SparqlEvaluator},
    store::Store,
};
use std::{collections::HashMap, sync::Arc};
use url::Url;

// ---------------------------------------------------------------------------
// Public vocabulary: XSD / RDF / RDFS / OWL IRI constants
// ---------------------------------------------------------------------------

/// RDF-schema vocabulary constants (XSD datatypes, RDF, RDFS, OWL 2).
///
/// These are re-exported at the crate level so that library consumers can
/// reference standard IRIs without hard-coding strings:
///
/// ```rust,ignore
/// use oxidowl::query::sparql_store::vocabulary;
/// assert_eq!(vocabulary::xsd::INTEGER, "http://www.w3.org/2001/XMLSchema#integer");
/// ```
pub mod vocabulary {
    /// XSD datatype IRIs (all types recognised by the RDF 1.1 / RDF-MT spec).
    pub mod xsd {
        // ── Primitive types ────────────────────────────────────────────────
        pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
        pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
        pub const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
        pub const FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
        pub const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        // ── Date/time ──────────────────────────────────────────────────────
        pub const DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
        pub const TIME: &str = "http://www.w3.org/2001/XMLSchema#time";
        pub const DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
        pub const G_YEAR_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gYearMonth";
        pub const G_YEAR: &str = "http://www.w3.org/2001/XMLSchema#gYear";
        pub const G_MONTH_DAY: &str = "http://www.w3.org/2001/XMLSchema#gMonthDay";
        pub const G_DAY: &str = "http://www.w3.org/2001/XMLSchema#gDay";
        pub const G_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gMonth";
        // ── Binary / URI ───────────────────────────────────────────────────
        pub const HEX_BINARY: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";
        pub const BASE64_BINARY: &str = "http://www.w3.org/2001/XMLSchema#base64Binary";
        pub const ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";
        // ── String-derived ────────────────────────────────────────────────
        pub const NORMALISED_STRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";
        pub const TOKEN: &str = "http://www.w3.org/2001/XMLSchema#token";
        pub const LANGUAGE: &str = "http://www.w3.org/2001/XMLSchema#language";
        pub const NMTOKEN: &str = "http://www.w3.org/2001/XMLSchema#NMTOKEN";
        pub const NAME: &str = "http://www.w3.org/2001/XMLSchema#Name";
        pub const NC_NAME: &str = "http://www.w3.org/2001/XMLSchema#NCName";
        // ── Integer hierarchy ─────────────────────────────────────────────
        pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
        pub const NON_POSITIVE_INTEGER: &str =
            "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";
        pub const NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";
        pub const LONG: &str = "http://www.w3.org/2001/XMLSchema#long";
        pub const INT: &str = "http://www.w3.org/2001/XMLSchema#int";
        pub const SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";
        pub const BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
        pub const NON_NEGATIVE_INTEGER: &str =
            "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
        pub const UNSIGNED_LONG: &str = "http://www.w3.org/2001/XMLSchema#unsignedLong";
        pub const UNSIGNED_INT: &str = "http://www.w3.org/2001/XMLSchema#unsignedInt";
        pub const UNSIGNED_SHORT: &str = "http://www.w3.org/2001/XMLSchema#unsignedShort";
        pub const UNSIGNED_BYTE: &str = "http://www.w3.org/2001/XMLSchema#unsignedByte";
        pub const POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";
    }

    /// RDF 1.1 core vocabulary IRIs.
    pub mod rdf {
        pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
        pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
        pub const HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";
        pub const XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";
    }

    /// RDFS vocabulary IRIs.
    pub mod rdfs {
        pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
        pub const SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
        pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
        pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
        pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
        pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
        pub const SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
        pub const IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";
        pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
        pub const LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
        pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
        pub const DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
    }

    /// OWL 2 vocabulary IRIs.
    pub mod owl {
        pub const CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
        pub const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
        pub const DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
        pub const ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
        pub const NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
        pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";
        pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
        pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
        pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
        pub const EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
        pub const INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
        pub const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
        pub const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
        pub const ALL_DIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";
        pub const DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
        pub const ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
        pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
        pub const VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
    }
}

/// Internal shorthand alias for vocabulary::xsd used within this module.
mod xsd {
    pub use super::vocabulary::xsd::*;
}
/// Internal shorthand for vocabulary::rdf.
mod vocab {
    pub mod rdf {
        pub use super::super::vocabulary::rdf::*;
    }
    pub mod rdfs {
        pub use super::super::vocabulary::rdfs::*;
    }
    pub mod owl {
        pub use super::super::vocabulary::owl::*;
    }
}

// ---------------------------------------------------------------------------
// XsdType: type-safe handle for every XSD datatype in the RDF-MT spec
// ---------------------------------------------------------------------------

/// Every XSD datatype defined in the RDF 1.1 semantics specification
/// (<https://www.w3.org/TR/2004/REC-rdf-mt-20040210/>).
///
/// Used with [`SparqlStore::typed_literal`] and the individual
/// `*_literal` constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XsdType {
    // Primitive
    /// `xsd:string`
    String,
    /// `xsd:boolean`
    Boolean,
    /// `xsd:decimal`
    Decimal,
    /// `xsd:float`
    Float,
    /// `xsd:double`
    Double,
    // Date / time
    /// `xsd:dateTime`
    DateTime,
    /// `xsd:time`
    Time,
    /// `xsd:date`
    Date,
    /// `xsd:gYearMonth`
    GYearMonth,
    /// `xsd:gYear`
    GYear,
    /// `xsd:gMonthDay`
    GMonthDay,
    /// `xsd:gDay`
    GDay,
    /// `xsd:gMonth`
    GMonth,
    // Binary / URI
    /// `xsd:hexBinary`
    HexBinary,
    /// `xsd:base64Binary`
    Base64Binary,
    /// `xsd:anyURI`
    AnyUri,
    // String-derived
    /// `xsd:normalizedString`
    NormalizedString,
    /// `xsd:token`
    Token,
    /// `xsd:language`
    Language,
    /// `xsd:NMTOKEN`
    Nmtoken,
    /// `xsd:Name`
    Name,
    /// `xsd:NCName`
    NcName,
    // Integer hierarchy
    /// `xsd:integer`
    Integer,
    /// `xsd:nonPositiveInteger`
    NonPositiveInteger,
    /// `xsd:negativeInteger`
    NegativeInteger,
    /// `xsd:long`
    Long,
    /// `xsd:int`
    Int,
    /// `xsd:short`
    Short,
    /// `xsd:byte`
    Byte,
    /// `xsd:nonNegativeInteger`
    NonNegativeInteger,
    /// `xsd:unsignedLong`
    UnsignedLong,
    /// `xsd:unsignedInt`
    UnsignedInt,
    /// `xsd:unsignedShort`
    UnsignedShort,
    /// `xsd:unsignedByte`
    UnsignedByte,
    /// `xsd:positiveInteger`
    PositiveInteger,
}

impl XsdType {
    /// Return the canonical IRI string for this XSD datatype.
    #[must_use]
    pub fn iri(self) -> &'static str {
        use vocabulary::xsd as x;
        match self {
            Self::String => x::STRING,
            Self::Boolean => x::BOOLEAN,
            Self::Decimal => x::DECIMAL,
            Self::Float => x::FLOAT,
            Self::Double => x::DOUBLE,
            Self::DateTime => x::DATE_TIME,
            Self::Time => x::TIME,
            Self::Date => x::DATE,
            Self::GYearMonth => x::G_YEAR_MONTH,
            Self::GYear => x::G_YEAR,
            Self::GMonthDay => x::G_MONTH_DAY,
            Self::GDay => x::G_DAY,
            Self::GMonth => x::G_MONTH,
            Self::HexBinary => x::HEX_BINARY,
            Self::Base64Binary => x::BASE64_BINARY,
            Self::AnyUri => x::ANY_URI,
            Self::NormalizedString => x::NORMALISED_STRING,
            Self::Token => x::TOKEN,
            Self::Language => x::LANGUAGE,
            Self::Nmtoken => x::NMTOKEN,
            Self::Name => x::NAME,
            Self::NcName => x::NC_NAME,
            Self::Integer => x::INTEGER,
            Self::NonPositiveInteger => x::NON_POSITIVE_INTEGER,
            Self::NegativeInteger => x::NEGATIVE_INTEGER,
            Self::Long => x::LONG,
            Self::Int => x::INT,
            Self::Short => x::SHORT,
            Self::Byte => x::BYTE,
            Self::NonNegativeInteger => x::NON_NEGATIVE_INTEGER,
            Self::UnsignedLong => x::UNSIGNED_LONG,
            Self::UnsignedInt => x::UNSIGNED_INT,
            Self::UnsignedShort => x::UNSIGNED_SHORT,
            Self::UnsignedByte => x::UNSIGNED_BYTE,
            Self::PositiveInteger => x::POSITIVE_INTEGER,
        }
    }

    /// Try to parse an IRI string back to an [`XsdType`].
    /// Returns `None` for unknown IRIs.
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        use vocabulary::xsd as x;
        match iri {
            s if s == x::STRING => Some(Self::String),
            s if s == x::BOOLEAN => Some(Self::Boolean),
            s if s == x::DECIMAL => Some(Self::Decimal),
            s if s == x::FLOAT => Some(Self::Float),
            s if s == x::DOUBLE => Some(Self::Double),
            s if s == x::DATE_TIME => Some(Self::DateTime),
            s if s == x::TIME => Some(Self::Time),
            s if s == x::DATE => Some(Self::Date),
            s if s == x::G_YEAR_MONTH => Some(Self::GYearMonth),
            s if s == x::G_YEAR => Some(Self::GYear),
            s if s == x::G_MONTH_DAY => Some(Self::GMonthDay),
            s if s == x::G_DAY => Some(Self::GDay),
            s if s == x::G_MONTH => Some(Self::GMonth),
            s if s == x::HEX_BINARY => Some(Self::HexBinary),
            s if s == x::BASE64_BINARY => Some(Self::Base64Binary),
            s if s == x::ANY_URI => Some(Self::AnyUri),
            s if s == x::NORMALISED_STRING => Some(Self::NormalizedString),
            s if s == x::TOKEN => Some(Self::Token),
            s if s == x::LANGUAGE => Some(Self::Language),
            s if s == x::NMTOKEN => Some(Self::Nmtoken),
            s if s == x::NAME => Some(Self::Name),
            s if s == x::NC_NAME => Some(Self::NcName),
            s if s == x::INTEGER => Some(Self::Integer),
            s if s == x::NON_POSITIVE_INTEGER => Some(Self::NonPositiveInteger),
            s if s == x::NEGATIVE_INTEGER => Some(Self::NegativeInteger),
            s if s == x::LONG => Some(Self::Long),
            s if s == x::INT => Some(Self::Int),
            s if s == x::SHORT => Some(Self::Short),
            s if s == x::BYTE => Some(Self::Byte),
            s if s == x::NON_NEGATIVE_INTEGER => Some(Self::NonNegativeInteger),
            s if s == x::UNSIGNED_LONG => Some(Self::UnsignedLong),
            s if s == x::UNSIGNED_INT => Some(Self::UnsignedInt),
            s if s == x::UNSIGNED_SHORT => Some(Self::UnsignedShort),
            s if s == x::UNSIGNED_BYTE => Some(Self::UnsignedByte),
            s if s == x::POSITIVE_INTEGER => Some(Self::PositiveInteger),
            _ => None,
        }
    }
}

impl std::fmt::Display for XsdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.iri())
    }
}

// ---------------------------------------------------------------------------
// SparqlStore
// ---------------------------------------------------------------------------

/// Supported RDF serialisation formats for [`SparqlStore::dump`] and
/// [`SparqlStore::load_rdf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Turtle (`.ttl`)
    Turtle,
    /// N-Triples (`.nt`)
    NTriples,
    /// N-Quads (`.nq`) – preserves named-graph information
    NQuads,
    /// TriG (`.trig`) – Turtle with named-graph blocks
    TriG,
    /// RDF/XML (`.rdf` / `.owl`)
    RdfXml,
}

impl From<SerializationFormat> for RdfFormat {
    fn from(fmt: SerializationFormat) -> Self {
        match fmt {
            SerializationFormat::Turtle => RdfFormat::Turtle,
            SerializationFormat::NTriples => RdfFormat::NTriples,
            SerializationFormat::NQuads => RdfFormat::NQuads,
            SerializationFormat::TriG => RdfFormat::TriG,
            SerializationFormat::RdfXml => RdfFormat::RdfXml,
        }
    }
}

/// An in-process SPARQL store backed by Oxigraph.
///
/// Use [`SparqlStore::new`] to create an empty store, or
/// [`SparqlStore::from_ontology`] to pre-populate it from an [`Ontology`] and
/// its inferred classification hierarchy.
pub struct SparqlStore {
    store: Store,
}

impl Default for SparqlStore {
    fn default() -> Self {
        Self::new().expect("in-memory Oxigraph store allocation should never fail")
    }
}

impl SparqlStore {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a new, empty in-process SPARQL store.
    ///
    /// # Errors
    /// Returns an error if the underlying Oxigraph store fails to initialise.
    pub fn new() -> Result<Self> {
        let store = Store::new().map_err(|e| Error::Sparql {
            message: format!("Failed to create SPARQL store: {e}"),
        })?;
        Ok(Self { store })
    }

    /// Create a store pre-populated from an [`Ontology`] plus inferred triples
    /// produced by `reasoning_service`.
    ///
    /// This is the primary constructor used by the SMOL interpreter at startup.
    ///
    /// # Errors
    /// Returns an error if the reasoning service cannot be accessed or if any
    /// triple conversion fails.
    pub async fn from_ontology(
        ontology: &Ontology,
        reasoning_service: &Arc<ReasoningService>,
    ) -> Result<Self> {
        let mut store_wrapper = Self::new()?;

        // Insert axiom-derived triples
        for axiom in &ontology.axioms {
            let triples = axiom_to_oxtriples(axiom)?;
            for triple in triples {
                store_wrapper.insert_oxtriple(triple)?;
            }
        }

        // Insert inferred subClassOf hierarchy
        let classification = reasoning_service.classify().await?;
        let rdfs_sub = make_named_node(vocab::rdfs::SUB_CLASS_OF)?;
        for (subclass, superclasses) in &classification.hierarchy {
            if let Some(sub_iri) = class_expr_to_named_node(subclass) {
                for superclass in superclasses {
                    if let Some(sup_iri) = class_expr_to_named_node(superclass) {
                        store_wrapper.insert_oxtriple(OxTriple::new(
                            sub_iri.clone(),
                            rdfs_sub.clone(),
                            sup_iri,
                        ))?;
                    }
                }
            }
        }

        // Insert RDF-star triples from the ontology graph (if any)
        if let Some(rdf_graph) = ontology.get_rdf_graph() {
            for oxidowl_triple in rdf_graph.triples() {
                if let Ok(quad) = oxidowl_triple_to_quad(oxidowl_triple) {
                    store_wrapper
                        .store
                        .insert(&quad)
                        .map_err(|e| Error::Sparql {
                            message: e.to_string(),
                        })?;
                }
            }
        }

        Ok(store_wrapper)
    }

    // -----------------------------------------------------------------------
    // Incremental mutation — default graph
    // -----------------------------------------------------------------------

    /// Insert a batch of oxidowl triples into the default graph.
    ///
    /// Silently skips quads that cannot be converted (e.g. quoted triples in
    /// subject position that require the `rdf-12` oxigraph feature).
    ///
    /// # Errors
    /// Returns an error if the underlying store rejects an insert.
    pub fn update_from_triples(&mut self, triples: &[OxidowlTriple]) -> Result<()> {
        for t in triples {
            match oxidowl_triple_to_quad(t) {
                Ok(quad) => {
                    self.store.insert(&quad).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                }
                Err(e) => {
                    tracing::warn!("Skipping unconvertible triple: {e}");
                }
            }
        }
        Ok(())
    }

    /// Remove a batch of oxidowl triples from the default graph.
    ///
    /// # Errors
    /// Returns an error if the underlying store rejects a removal.
    pub fn remove_triples(&mut self, triples: &[OxidowlTriple]) -> Result<()> {
        for t in triples {
            match oxidowl_triple_to_quad(t) {
                Ok(quad) => {
                    self.store.remove(&quad).map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                }
                Err(e) => {
                    tracing::warn!("Skipping unconvertible triple during remove: {e}");
                }
            }
        }
        Ok(())
    }

    /// Insert a single raw Oxigraph triple into the default graph.
    fn insert_oxtriple(&mut self, triple: OxTriple) -> Result<()> {
        let quad = Quad::new(
            triple.subject,
            triple.predicate,
            triple.object,
            GraphName::DefaultGraph,
        );
        self.store.insert(&quad).map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // Loading from serialised RDF
    // -----------------------------------------------------------------------

    /// Bulk-load a Turtle document into the default graph.
    ///
    /// This is the primary way to load background ontologies; it is far more
    /// efficient than inserting triples one-by-one via SPARQL UPDATE.
    ///
    /// # Errors
    /// Returns an error if the Turtle cannot be parsed or the store rejects a
    /// triple.
    pub fn load_turtle(&mut self, turtle: &str) -> Result<()> {
        self.load_rdf(turtle, SerializationFormat::Turtle)
    }

    /// Bulk-load any supported RDF format into the default graph.
    ///
    /// # Errors
    /// Returns an error if parsing or store insertion fails.
    pub fn load_rdf(&mut self, data: &str, format: SerializationFormat) -> Result<()> {
        self.store
            .load_from_slice(RdfParser::from_format(format.into()), data)
            .map_err(|e| Error::Sparql {
                message: format!("Failed to bulk-load RDF ({format:?}): {e}"),
            })
    }

    /// Bulk-load any supported RDF format into a specific named graph.
    ///
    /// The named graph is created if it does not already exist.
    ///
    /// # Errors
    /// Returns an error if parsing or store insertion fails.
    pub fn load_rdf_into_graph(
        &mut self,
        data: &str,
        format: SerializationFormat,
        graph_iri: &str,
    ) -> Result<()> {
        let graph_name = make_named_node(graph_iri)?;
        // Ensure the named graph exists
        self.store
            .insert_named_graph(graph_name.as_ref())
            .map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;

        // Parse and re-insert every quad with the target graph name
        let parser = RdfParser::from_format(format.into());
        let result = parser.for_slice(data.as_bytes());
        for quad_result in result {
            let quad = quad_result.map_err(|e| Error::Sparql {
                message: format!("RDF parse error: {e}"),
            })?;
            // Override whatever graph name was in the source
            let rewritten = Quad::new(
                quad.subject,
                quad.predicate,
                quad.object,
                GraphName::NamedNode(graph_name.clone()),
            );
            self.store.insert(&rewritten).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Named-graph management
    // -----------------------------------------------------------------------

    /// Insert a quad into the specified named graph (creates the graph if needed).
    ///
    /// # Errors
    /// Returns an error if graph creation or quad insertion fails.
    pub fn insert_into_named_graph(
        &mut self,
        triple: &OxidowlTriple,
        graph_iri: &str,
    ) -> Result<()> {
        let graph_name = make_named_node(graph_iri)?;
        self.store
            .insert_named_graph(graph_name.as_ref())
            .map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;

        let subject = rdfterm_to_oxsubject(&triple.subject)?;
        let predicate = rdfterm_to_oxpredicate(&triple.predicate)?;
        let object = rdfterm_to_oxterm(&triple.object)?;
        let quad = Quad::new(subject, predicate, object, GraphName::NamedNode(graph_name));
        self.store.insert(&quad).map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Remove all triples from a named graph without deleting the graph itself.
    ///
    /// # Errors
    /// Returns an error if the store operation fails.
    pub fn clear_named_graph(&mut self, graph_iri: &str) -> Result<()> {
        let graph_name = make_named_node(graph_iri)?;
        self.store
            .clear_graph(GraphNameRef::NamedNode(graph_name.as_ref()))
            .map_err(|e| Error::Sparql {
                message: e.to_string(),
            })
    }

    /// Remove a named graph and all its triples.
    ///
    /// # Errors
    /// Returns an error if the store operation fails.
    pub fn remove_named_graph(&mut self, graph_iri: &str) -> Result<()> {
        let graph_name = make_named_node(graph_iri)?;
        self.store
            .remove_named_graph(graph_name.as_ref())
            .map_err(|e| Error::Sparql {
                message: e.to_string(),
            })
    }

    /// Return a list of all named-graph IRIs present in the store.
    ///
    /// # Errors
    /// Returns an error if the iteration fails.
    pub fn named_graphs(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        for graph_name_result in self.store.named_graphs() {
            let gn = graph_name_result.map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            result.push(gn.to_string());
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // SPARQL query execution
    // -----------------------------------------------------------------------

    /// Execute a SPARQL SELECT query.
    ///
    /// Returns each solution row as a `HashMap<variable_name, RdfTerm>`.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_select(&self, query: &str) -> Result<Vec<HashMap<String, RdfTerm>>> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Solutions(solutions) => {
                let mut rows = Vec::new();
                for solution in solutions {
                    let sol = solution.map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    let row: HashMap<String, RdfTerm> = sol
                        .iter()
                        .map(|(var, term)| (var.as_str().to_string(), oxterm_to_rdfterm(term)))
                        .collect();
                    rows.push(row);
                }
                Ok(rows)
            }
            _ => Err(Error::Sparql {
                message: "Expected SELECT query results but got ASK/CONSTRUCT".to_string(),
            }),
        }
    }

    /// Execute a SPARQL ASK query.  Returns `true` if the pattern matches.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_ask(&self, query: &str) -> Result<bool> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Boolean(b) => Ok(b),
            _ => Err(Error::Sparql {
                message: "Expected ASK query results but got SELECT/CONSTRUCT".to_string(),
            }),
        }
    }

    /// Execute a SPARQL CONSTRUCT query.
    ///
    /// Returns the resulting graph as a `Vec<OxidowlTriple>`.
    ///
    /// # Errors
    /// Returns an error if the query fails to parse or execute.
    pub fn execute_construct(&self, query: &str) -> Result<Vec<OxidowlTriple>> {
        let results = self.run_query(query)?;
        match results {
            QueryResults::Graph(graph) => {
                let mut triples = Vec::new();
                for triple_result in graph {
                    let t = triple_result.map_err(|e| Error::Sparql {
                        message: e.to_string(),
                    })?;
                    triples.push(quad_triple_to_oxidowl(&t)?);
                }
                Ok(triples)
            }
            _ => Err(Error::Sparql {
                message: "Expected CONSTRUCT/DESCRIBE query results but got SELECT/ASK".to_string(),
            }),
        }
    }

    /// Execute a SPARQL UPDATE operation (INSERT DATA / DELETE DATA / etc.).
    ///
    /// # Errors
    /// Returns an error if the update fails to parse or execute.
    pub fn execute_update(&mut self, update: &str) -> Result<()> {
        self.store.update(update).map_err(|e| Error::Sparql {
            message: format!("SPARQL update failed: {e}"),
        })
    }

    // -----------------------------------------------------------------------
    // Quad-pattern access
    // -----------------------------------------------------------------------

    /// Retrieve all quads matching the given pattern.
    ///
    /// Any parameter may be `None` to act as a wildcard.  The graph name
    /// defaults to the default graph when all three of `subject`, `predicate`,
    /// and `object` are `None` and `graph` is also `None`; to match every
    /// graph pass an explicit `None` for each.
    ///
    /// Returns quads as `(subject, predicate, object, graph)` 4-tuples
    /// represented as [`RdfTerm`] values (graph name as `Option<RdfTerm>`).
    ///
    /// # Errors
    /// Returns an error if the underlying Oxigraph iteration fails.
    pub fn quads_for_pattern(
        &self,
        subject: Option<&RdfTerm>,
        predicate: Option<&RdfTerm>,
        object: Option<&RdfTerm>,
        graph: Option<&str>,
    ) -> Result<Vec<(RdfTerm, RdfTerm, RdfTerm, Option<String>)>> {
        // Convert the filter terms into Oxigraph's typed refs.  We hold
        // intermediate owned values so the refs stay alive.
        let ox_subject: Option<NamedOrBlankNode> = subject.map(rdfterm_to_oxsubject).transpose()?;
        let ox_predicate: Option<NamedNode> = predicate.map(rdfterm_to_oxpredicate).transpose()?;
        let ox_object: Option<Term> = object.map(rdfterm_to_oxterm).transpose()?;
        let ox_graph: Option<NamedNode> = graph.map(make_named_node).transpose()?;

        let subj_ref = ox_subject.as_ref().map(NamedOrBlankNode::as_ref);
        let pred_ref = ox_predicate
            .as_ref()
            .map(|n| NamedNodeRef::new_unchecked(n.as_str()));
        let obj_ref = ox_object.as_ref().map(Term::as_ref);
        let graph_ref = ox_graph
            .as_ref()
            .map(|n| GraphNameRef::NamedNode(NamedNodeRef::new_unchecked(n.as_str())));

        let mut result = Vec::new();
        for quad_result in self
            .store
            .quads_for_pattern(subj_ref, pred_ref, obj_ref, graph_ref)
        {
            let quad = quad_result.map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            let subj = ox_subject_to_rdfterm(&quad.subject);
            let pred = ox_named_node_to_rdfterm(&quad.predicate);
            let obj = oxterm_to_rdfterm(&quad.object);
            let gn = match &quad.graph_name {
                GraphName::DefaultGraph => None,
                GraphName::NamedNode(n) => Some(n.as_str().to_string()),
                GraphName::BlankNode(b) => Some(b.as_str().to_string()),
            };
            result.push((subj, pred, obj, gn));
        }
        Ok(result)
    }

    /// Iterate over every quad in the store (all graphs).
    ///
    /// # Errors
    /// Returns an error if the underlying Oxigraph iteration fails.
    pub fn all_quads(&self) -> Result<Vec<(RdfTerm, RdfTerm, RdfTerm, Option<String>)>> {
        self.quads_for_pattern(None, None, None, None)
    }

    // -----------------------------------------------------------------------
    // Statistics & maintenance
    // -----------------------------------------------------------------------

    /// Return the total number of quads stored (across all graphs).
    ///
    /// # Errors
    /// Returns an error if the underlying count operation fails.
    pub fn len(&self) -> Result<usize> {
        self.store.len().map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Return `true` if the store contains no quads.
    ///
    /// # Errors
    /// Returns an error if the underlying check fails.
    pub fn is_empty(&self) -> Result<bool> {
        self.store.is_empty().map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Return `true` if the store contains the given triple in the default graph.
    ///
    /// # Errors
    /// Returns an error if the triple cannot be converted or the lookup fails.
    pub fn contains_triple(&self, triple: &OxidowlTriple) -> Result<bool> {
        let quad = oxidowl_triple_to_quad(triple)?;
        self.store.contains(&quad).map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Remove all quads from every graph in the store.
    ///
    /// # Errors
    /// Returns an error if the underlying clear fails.
    pub fn clear(&mut self) -> Result<()> {
        self.store.clear().map_err(|e| Error::Sparql {
            message: e.to_string(),
        })
    }

    /// Remove all triples from the default graph.
    ///
    /// # Errors
    /// Returns an error if the underlying clear fails.
    pub fn clear_default_graph(&mut self) -> Result<()> {
        self.store
            .clear_graph(GraphNameRef::DefaultGraph)
            .map_err(|e| Error::Sparql {
                message: e.to_string(),
            })
    }

    // -----------------------------------------------------------------------
    // Serialisation
    // -----------------------------------------------------------------------

    /// Serialise the entire store (all graphs) to a byte vector.
    ///
    /// Only dataset formats ([`SerializationFormat::NQuads`],
    /// [`SerializationFormat::TriG`]) can capture named-graph information.
    /// For graph-only formats (Turtle, N-Triples, RDF/XML) use
    /// [`SparqlStore::dump_default_graph`] to serialise just the default graph.
    ///
    /// # Errors
    /// Returns an error if the format is not a dataset format or if
    /// serialisation fails.
    pub fn dump(&self, format: SerializationFormat) -> Result<Vec<u8>> {
        let ox_fmt: RdfFormat = format.into();
        if !ox_fmt.supports_datasets() {
            return Err(Error::Sparql {
                message: format!(
                    "Format {format:?} does not support datasets (multiple graphs). \
                     Use dump_default_graph() for Turtle / N-Triples / RDF/XML."
                ),
            });
        }
        let mut buf = Vec::new();
        self.store
            .dump_to_writer(RdfSerializer::from_format(ox_fmt), &mut buf)
            .map_err(|e| Error::Sparql {
                message: format!("Serialisation failed ({format:?}): {e}"),
            })?;
        Ok(buf)
    }

    /// Serialise the entire store to a UTF-8 string.
    ///
    /// Only dataset formats are supported; see [`SparqlStore::dump`].
    ///
    /// # Errors
    /// Returns an error if serialisation or UTF-8 conversion fails.
    pub fn dump_str(&self, format: SerializationFormat) -> Result<String> {
        let bytes = self.dump(format)?;
        String::from_utf8(bytes).map_err(|e| Error::Sparql {
            message: format!("Serialised bytes are not valid UTF-8: {e}"),
        })
    }

    /// Serialise the **default graph only** to a byte vector in the given format.
    ///
    /// This works with any format, including Turtle, N-Triples, and RDF/XML.
    ///
    /// # Errors
    /// Returns an error if serialisation fails.
    pub fn dump_default_graph(&self, format: SerializationFormat) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.store
            .dump_graph_to_writer(
                GraphNameRef::DefaultGraph,
                RdfSerializer::from_format(format.into()),
                &mut buf,
            )
            .map_err(|e| Error::Sparql {
                message: format!("Serialisation failed ({format:?}): {e}"),
            })?;
        Ok(buf)
    }

    /// Serialise the **default graph** to a UTF-8 Turtle string.
    ///
    /// # Errors
    /// Returns an error if serialisation fails.
    pub fn dump_turtle(&self) -> Result<String> {
        let bytes = self.dump_default_graph(SerializationFormat::Turtle)?;
        String::from_utf8(bytes).map_err(|e| Error::Sparql {
            message: format!("Serialised bytes are not valid UTF-8: {e}"),
        })
    }

    /// Convenience wrapper: serialise all quads as N-Quads (dataset format).
    ///
    /// # Errors
    /// Returns an error if serialisation fails.
    pub fn dump_nquads(&self) -> Result<String> {
        self.dump_str(SerializationFormat::NQuads)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Execute any SPARQL query via the Oxigraph `SparqlEvaluator` API.
    fn run_query(&self, query: &str) -> Result<QueryResults<'_>> {
        SparqlEvaluator::new()
            .parse_query(query)
            .map_err(|e| Error::Sparql {
                message: format!("SPARQL parse error: {e}"),
            })?
            .on_store(&self.store)
            .execute()
            .map_err(|e| Error::Sparql {
                message: format!("SPARQL execution error: {e}"),
            })
    }

    /// Return a reference to the raw Oxigraph store (for advanced use-cases).
    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Return a mutable reference to the raw Oxigraph store.
    #[must_use]
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    // -----------------------------------------------------------------------
    // Typed literal constructors
    // -----------------------------------------------------------------------

    /// Build a typed [`RdfTerm`] literal with an explicit XSD datatype.
    ///
    /// ```rust,ignore
    /// let t = SparqlStore::typed_literal("2024-01-01", XsdType::Date);
    /// ```
    #[must_use]
    pub fn typed_literal(value: &str, datatype: XsdType) -> RdfTerm {
        let dt_url = url::Url::parse(datatype.iri()).ok();
        RdfTerm::Literal {
            value: value.to_owned(),
            datatype: dt_url,
            language: None,
            direction: None,
        }
    }

    /// `xsd:string` literal.
    #[must_use]
    pub fn string_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::String)
    }

    /// `xsd:boolean` literal (`"true"` / `"false"`).
    #[must_use]
    pub fn boolean_literal(value: bool) -> RdfTerm {
        Self::typed_literal(if value { "true" } else { "false" }, XsdType::Boolean)
    }

    /// `xsd:integer` literal.
    #[must_use]
    pub fn integer_literal(value: i64) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Integer)
    }

    /// `xsd:decimal` literal (supply a pre-formatted string like `"3.14"`).
    #[must_use]
    pub fn decimal_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::Decimal)
    }

    /// `xsd:double` literal.
    #[must_use]
    pub fn double_literal(value: f64) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Double)
    }

    /// `xsd:float` literal.
    #[must_use]
    pub fn float_literal(value: f32) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Float)
    }

    /// `xsd:long` literal.
    #[must_use]
    pub fn long_literal(value: i64) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Long)
    }

    /// `xsd:int` literal.
    #[must_use]
    pub fn int_literal(value: i32) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Int)
    }

    /// `xsd:short` literal.
    #[must_use]
    pub fn short_literal(value: i16) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Short)
    }

    /// `xsd:byte` literal.
    #[must_use]
    pub fn byte_literal(value: i8) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::Byte)
    }

    /// `xsd:unsignedLong` literal.
    #[must_use]
    pub fn unsigned_long_literal(value: u64) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::UnsignedLong)
    }

    /// `xsd:unsignedInt` literal.
    #[must_use]
    pub fn unsigned_int_literal(value: u32) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::UnsignedInt)
    }

    /// `xsd:unsignedShort` literal.
    #[must_use]
    pub fn unsigned_short_literal(value: u16) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::UnsignedShort)
    }

    /// `xsd:unsignedByte` literal.
    #[must_use]
    pub fn unsigned_byte_literal(value: u8) -> RdfTerm {
        Self::typed_literal(&value.to_string(), XsdType::UnsignedByte)
    }

    /// `xsd:dateTime` literal (ISO-8601 string, e.g. `"2024-01-01T12:00:00"`).
    #[must_use]
    pub fn datetime_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::DateTime)
    }

    /// `xsd:date` literal (ISO-8601 string, e.g. `"2024-01-01"`).
    #[must_use]
    pub fn date_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::Date)
    }

    /// `xsd:time` literal (e.g. `"12:00:00"`).
    #[must_use]
    pub fn time_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::Time)
    }

    /// `xsd:anyURI` literal.
    #[must_use]
    pub fn any_uri_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::AnyUri)
    }

    /// `xsd:hexBinary` literal.
    #[must_use]
    pub fn hex_binary_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::HexBinary)
    }

    /// `xsd:base64Binary` literal.
    #[must_use]
    pub fn base64_binary_literal(value: &str) -> RdfTerm {
        Self::typed_literal(value, XsdType::Base64Binary)
    }

    // -----------------------------------------------------------------------
    // Semantic query helpers
    // -----------------------------------------------------------------------

    /// Return all `rdf:type` values for `individual_iri`.
    pub fn get_types(&self, individual_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?t WHERE {{ <{individual_iri}> <{}> ?t }}",
            vocabulary::rdf::TYPE
        );
        self.collect_first_binding(&q, "t")
    }

    /// Return all individuals asserted to be of type `class_iri` via
    /// `rdf:type`.
    pub fn find_instances(&self, class_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?i WHERE {{ ?i <{}> <{class_iri}> }}",
            vocabulary::rdf::TYPE
        );
        self.collect_first_binding(&q, "i")
    }

    /// Return all direct and inferred subclasses of `class_iri` via
    /// `rdfs:subClassOf`.
    pub fn find_subclasses(&self, class_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?c WHERE {{ ?c <{}> <{class_iri}> }}",
            vocabulary::rdfs::SUB_CLASS_OF
        );
        self.collect_first_binding(&q, "c")
    }

    /// Return all superclasses of `class_iri` via `rdfs:subClassOf`.
    pub fn find_superclasses(&self, class_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?c WHERE {{ <{class_iri}> <{}> ?c }}",
            vocabulary::rdfs::SUB_CLASS_OF
        );
        self.collect_first_binding(&q, "c")
    }

    /// Return all `rdfs:label` values for `iri` as plain strings.
    pub fn find_labels(&self, iri: &str) -> Result<Vec<String>> {
        let terms = self.collect_first_binding(
            &format!(
                "SELECT ?l WHERE {{ <{iri}> <{}> ?l }}",
                vocabulary::rdfs::LABEL
            ),
            "l",
        )?;
        Ok(terms
            .into_iter()
            .filter_map(|t| match t {
                RdfTerm::Literal { value, .. } => Some(value),
                _ => None,
            })
            .collect())
    }

    /// Return all `rdfs:comment` values for `iri` as plain strings.
    pub fn find_comments(&self, iri: &str) -> Result<Vec<String>> {
        let terms = self.collect_first_binding(
            &format!(
                "SELECT ?c WHERE {{ <{iri}> <{}> ?c }}",
                vocabulary::rdfs::COMMENT
            ),
            "c",
        )?;
        Ok(terms
            .into_iter()
            .filter_map(|t| match t {
                RdfTerm::Literal { value, .. } => Some(value),
                _ => None,
            })
            .collect())
    }

    /// Return the `rdfs:domain` declarations for `property_iri`.
    pub fn find_domain(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?d WHERE {{ <{property_iri}> <{}> ?d }}",
            vocabulary::rdfs::DOMAIN
        );
        self.collect_first_binding(&q, "d")
    }

    /// Return the `rdfs:range` declarations for `property_iri`.
    pub fn find_range(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?r WHERE {{ <{property_iri}> <{}> ?r }}",
            vocabulary::rdfs::RANGE
        );
        self.collect_first_binding(&q, "r")
    }

    /// Return classes declared `owl:equivalentClass` to `class_iri`.
    pub fn find_equivalent_classes(&self, class_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?c WHERE {{ {{ <{class_iri}> <{eq}> ?c }} UNION {{ ?c <{eq}> <{class_iri}> }} }}",
            eq = vocabulary::owl::EQUIVALENT_CLASS
        );
        self.collect_first_binding(&q, "c")
    }

    /// Return classes declared `owl:disjointWith` from `class_iri`.
    pub fn find_disjoint_classes(&self, class_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?c WHERE {{ {{ <{class_iri}> <{dw}> ?c }} UNION {{ ?c <{dw}> <{class_iri}> }} }}",
            dw = vocabulary::owl::DISJOINT_WITH
        );
        self.collect_first_binding(&q, "c")
    }

    /// Return individuals declared `owl:sameAs` the given `individual_iri`.
    pub fn find_same_as(&self, individual_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?s WHERE {{ {{ <{individual_iri}> <{sa}> ?s }} UNION {{ ?s <{sa}> <{individual_iri}> }} }}",
            sa = vocabulary::owl::SAME_AS
        );
        self.collect_first_binding(&q, "s")
    }

    /// Return individuals declared `owl:differentFrom` the given `individual_iri`.
    pub fn find_different_from(&self, individual_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?d WHERE {{ {{ <{individual_iri}> <{df}> ?d }} UNION {{ ?d <{df}> <{individual_iri}> }} }}",
            df = vocabulary::owl::DIFFERENT_FROM
        );
        self.collect_first_binding(&q, "d")
    }

    /// Collect all members of an `rdf:List` starting at `list_iri`.
    ///
    /// Uses a SPARQL 1.1 property-path query (`rdf:rest*/rdf:first`) so that
    /// blank-node intermediate list spine nodes are handled transparently by
    /// the SPARQL engine.  Note: member order is determined by `rdf:rest` chain
    /// depth; the results are returned in path order because Oxigraph evaluates
    /// the closure in graph traversal order.
    pub fn rdf_list_members(&self, list_iri: &str) -> Result<Vec<RdfTerm>> {
        let first = vocabulary::rdf::FIRST;
        let rest = vocabulary::rdf::REST;
        let q = format!("SELECT ?member WHERE {{ <{list_iri}> (<{rest}>*/<{first}>) ?member }}");
        self.collect_first_binding(&q, "member")
    }

    /// Find all properties whose subproperty chain includes `property_iri`
    /// (i.e., `?p rdfs:subPropertyOf <property_iri>`).
    pub fn find_sub_properties(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?p WHERE {{ ?p <{}> <{property_iri}> }}",
            vocabulary::rdfs::SUB_PROPERTY_OF
        );
        self.collect_first_binding(&q, "p")
    }

    /// Find all super-properties of `property_iri`
    /// (i.e., `<property_iri> rdfs:subPropertyOf ?p`).
    pub fn find_super_properties(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?p WHERE {{ <{property_iri}> <{}> ?p }}",
            vocabulary::rdfs::SUB_PROPERTY_OF
        );
        self.collect_first_binding(&q, "p")
    }

    /// Find all equivalent properties of `property_iri` via
    /// `owl:equivalentProperty`.
    pub fn find_equivalent_properties(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?p WHERE {{ {{ <{property_iri}> <{ep}> ?p }} UNION {{ ?p <{ep}> <{property_iri}> }} }}",
            ep = vocabulary::owl::EQUIVALENT_PROPERTY
        );
        self.collect_first_binding(&q, "p")
    }

    /// Return all inverse properties for `property_iri` via `owl:inverseOf`.
    pub fn find_inverse_properties(&self, property_iri: &str) -> Result<Vec<RdfTerm>> {
        let q = format!(
            "SELECT ?p WHERE {{ {{ <{property_iri}> <{io}> ?p }} UNION {{ ?p <{io}> <{property_iri}> }} }}",
            io = vocabulary::owl::INVERSE_OF
        );
        self.collect_first_binding(&q, "p")
    }

    // -----------------------------------------------------------------------
    // Internal helper: collect the first variable from SELECT results
    // -----------------------------------------------------------------------

    fn collect_first_binding(&self, sparql: &str, var: &str) -> Result<Vec<RdfTerm>> {
        match self.run_query(sparql)? {
            QueryResults::Solutions(solutions) => {
                let mut out = Vec::new();
                for sol in solutions {
                    let sol = sol.map_err(|e| Error::Sparql {
                        message: format!("SPARQL row error: {e}"),
                    })?;
                    if let Some(term) = sol.get(var) {
                        out.push(oxterm_to_rdfterm(term));
                    }
                }
                Ok(out)
            }
            _ => Err(Error::Sparql {
                message: "Expected SELECT results".to_owned(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers: oxidowl → oxigraph
// ---------------------------------------------------------------------------

/// Small helper: create a validated [`NamedNode`] from an IRI string.
fn make_named_node(iri: &str) -> Result<NamedNode> {
    NamedNode::new(iri).map_err(|e| Error::Sparql {
        message: format!("Invalid IRI '{iri}': {e}"),
    })
}

/// Convert an oxidowl axiom to a (possibly empty) list of raw Oxigraph triples.
///
/// Covers all OWL 2 assertion and schema axiom types that map naturally to
/// RDF triples.  Complex class expressions (intersections, restrictions, etc.)
/// are intentionally skipped here because they require blank-node skolemisation
/// that is handled separately by the reasoner / parser pipeline.
fn axiom_to_oxtriples(axiom: &Axiom) -> Result<Vec<OxTriple>> {
    let mut triples = Vec::new();

    match axiom {
        // ----------------------------------------------------------------
        // Declaration axioms → rdf:type triples (OWL vocabulary)
        // ----------------------------------------------------------------
        Axiom::Declaration(ax) => {
            use crate::ontology::axioms::Entity;
            let (entity_iri, type_iri) = match &ax.entity {
                Entity::Class(iri) => (iri.as_str(), vocab::owl::CLASS),
                Entity::ObjectProperty(iri) => (iri.as_str(), vocab::owl::OBJECT_PROPERTY),
                Entity::DataProperty(iri) => (iri.as_str(), vocab::owl::DATA_PROPERTY),
                Entity::AnnotationProperty(iri) => (iri.as_str(), vocab::owl::ANNOTATION_PROPERTY),
                Entity::NamedIndividual(iri) => (iri.as_str(), vocab::owl::NAMED_INDIVIDUAL),
                Entity::Datatype(iri) => {
                    // owl:Datatype or rdfs:Datatype – emit rdf:type owl:Class as
                    // a conservative fallback (OWL 2 spec also allows this).
                    (iri.as_str(), vocab::owl::CLASS)
                }
            };
            let subj = make_named_node(entity_iri)?;
            let rdf_type = make_named_node(vocab::rdf::TYPE)?;
            let obj = make_named_node(type_iri)?;
            triples.push(OxTriple::new(subj, rdf_type, obj));
        }

        // ----------------------------------------------------------------
        // Class axioms
        // ----------------------------------------------------------------
        Axiom::SubClassOf(ax) => {
            if let (Some(sub_iri), Some(sup_iri)) = (
                class_expr_to_named_node(&ax.subclass),
                class_expr_to_named_node(&ax.superclass),
            ) {
                triples.push(OxTriple::new(
                    sub_iri,
                    make_named_node(vocab::rdfs::SUB_CLASS_OF)?,
                    sup_iri,
                ));
            }
        }

        Axiom::EquivalentClasses(ax) => {
            let nodes: Vec<_> = ax
                .classes
                .iter()
                .filter_map(class_expr_to_named_node)
                .collect();
            let eq_prop = make_named_node(vocab::owl::EQUIVALENT_CLASS)?;
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    triples.push(OxTriple::new(
                        nodes[i].clone(),
                        eq_prop.clone(),
                        nodes[j].clone(),
                    ));
                    // Emit the symmetric direction too
                    triples.push(OxTriple::new(
                        nodes[j].clone(),
                        eq_prop.clone(),
                        nodes[i].clone(),
                    ));
                }
            }
        }

        Axiom::DisjointClasses(ax) => {
            let nodes: Vec<_> = ax
                .classes
                .iter()
                .filter_map(class_expr_to_named_node)
                .collect();
            let dj_prop = make_named_node(vocab::owl::DISJOINT_WITH)?;
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    triples.push(OxTriple::new(
                        nodes[i].clone(),
                        dj_prop.clone(),
                        nodes[j].clone(),
                    ));
                    triples.push(OxTriple::new(
                        nodes[j].clone(),
                        dj_prop.clone(),
                        nodes[i].clone(),
                    ));
                }
            }
        }

        // ----------------------------------------------------------------
        // Object property axioms
        // ----------------------------------------------------------------
        Axiom::SubObjectPropertyOf(ax) => {
            if let (Some(sub), Some(sup)) = (
                obj_prop_expr_iri_str(&ax.sub_property).and_then(|s| make_named_node(s).ok()),
                obj_prop_expr_iri_str(&ax.super_property).and_then(|s| make_named_node(s).ok()),
            ) {
                triples.push(OxTriple::new(
                    sub,
                    make_named_node(vocab::rdfs::SUB_PROPERTY_OF)?,
                    sup,
                ));
            }
        }

        Axiom::EquivalentObjectProperties(ax) => {
            let nodes: Vec<_> = ax
                .properties
                .iter()
                .filter_map(|p| obj_prop_expr_iri_str(p).and_then(|s| make_named_node(s).ok()))
                .collect();
            let eq_prop = make_named_node(vocab::owl::EQUIVALENT_PROPERTY)?;
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    triples.push(OxTriple::new(
                        nodes[i].clone(),
                        eq_prop.clone(),
                        nodes[j].clone(),
                    ));
                    triples.push(OxTriple::new(
                        nodes[j].clone(),
                        eq_prop.clone(),
                        nodes[i].clone(),
                    ));
                }
            }
        }

        Axiom::InverseObjectProperties(ax) => {
            if let (Some(p1), Some(p2)) = (
                obj_prop_expr_iri_str(&ax.property1).and_then(|s| make_named_node(s).ok()),
                obj_prop_expr_iri_str(&ax.property2).and_then(|s| make_named_node(s).ok()),
            ) {
                let inv_of = make_named_node(vocab::owl::INVERSE_OF)?;
                triples.push(OxTriple::new(p1.clone(), inv_of.clone(), p2.clone()));
                triples.push(OxTriple::new(p2, inv_of, p1));
            }
        }

        Axiom::ObjectPropertyDomain(ax) => {
            if let (Some(prop), Some(domain)) = (
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok()),
                class_expr_to_named_node(&ax.domain),
            ) {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdfs::DOMAIN)?,
                    domain,
                ));
            }
        }

        Axiom::ObjectPropertyRange(ax) => {
            if let (Some(prop), Some(range)) = (
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok()),
                class_expr_to_named_node(&ax.range),
            ) {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdfs::RANGE)?,
                    range,
                ));
            }
        }

        Axiom::FunctionalObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#FunctionalProperty")?,
                ));
            }
        }

        Axiom::InverseFunctionalObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#InverseFunctionalProperty")?,
                ));
            }
        }

        Axiom::TransitiveObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#TransitiveProperty")?,
                ));
            }
        }

        Axiom::SymmetricObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#SymmetricProperty")?,
                ));
            }
        }

        Axiom::AsymmetricObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#AsymmetricProperty")?,
                ));
            }
        }

        Axiom::ReflexiveObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#ReflexiveProperty")?,
                ));
            }
        }

        Axiom::IrreflexiveObjectProperty(ax) => {
            if let Some(prop) =
                obj_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#IrreflexiveProperty")?,
                ));
            }
        }

        // ----------------------------------------------------------------
        // Data property axioms
        // ----------------------------------------------------------------
        Axiom::SubDataPropertyOf(ax) => {
            if let (Some(sub), Some(sup)) = (
                data_prop_expr_iri_str(&ax.sub_property).and_then(|s| make_named_node(s).ok()),
                data_prop_expr_iri_str(&ax.super_property).and_then(|s| make_named_node(s).ok()),
            ) {
                triples.push(OxTriple::new(
                    sub,
                    make_named_node(vocab::rdfs::SUB_PROPERTY_OF)?,
                    sup,
                ));
            }
        }

        Axiom::DataPropertyDomain(ax) => {
            if let (Some(prop), Some(domain)) = (
                data_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok()),
                class_expr_to_named_node(&ax.domain),
            ) {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdfs::DOMAIN)?,
                    domain,
                ));
            }
        }

        Axiom::FunctionalDataProperty(ax) => {
            if let Some(prop) =
                data_prop_expr_iri_str(&ax.property).and_then(|s| make_named_node(s).ok())
            {
                triples.push(OxTriple::new(
                    prop,
                    make_named_node(vocab::rdf::TYPE)?,
                    make_named_node("http://www.w3.org/2002/07/owl#FunctionalProperty")?,
                ));
            }
        }

        // ----------------------------------------------------------------
        // Individual axioms
        // ----------------------------------------------------------------
        Axiom::ClassAssertion(ax) => {
            if let Some(class_iri) = class_expr_to_named_node(&ax.class)
                && let Some(ind_str) = individual_iri_str(&ax.individual)
            {
                let ind_iri = make_named_node(ind_str)?;
                let rdf_type = make_named_node(vocab::rdf::TYPE)?;
                triples.push(OxTriple::new(ind_iri, rdf_type, class_iri));
            }
        }

        Axiom::ObjectPropertyAssertion(ax) => {
            if let Some(prop_str) = obj_prop_expr_iri_str(&ax.property)
                && let (Some(subj_str), Some(obj_str)) = (
                    individual_iri_str(&ax.source),
                    individual_iri_str(&ax.target),
                )
            {
                triples.push(OxTriple::new(
                    make_named_node(subj_str)?,
                    make_named_node(prop_str)?,
                    make_named_node(obj_str)?,
                ));
            }
        }

        Axiom::DataPropertyAssertion(ax) => {
            if let Some(prop_str) = data_prop_expr_iri_str(&ax.property)
                && let Some(ind_str) = individual_iri_str(&ax.individual)
            {
                let subj = make_named_node(ind_str)?;
                let pred = make_named_node(prop_str)?;
                let obj = ontology_literal_to_oxterm(&ax.value)?;
                triples.push(OxTriple::new(subj, pred, obj));
            }
        }

        Axiom::SameIndividual(ax) => {
            let nodes: Vec<_> = ax
                .individuals
                .iter()
                .filter_map(|i| individual_iri_str(i))
                .filter_map(|s| make_named_node(s).ok())
                .collect();
            let same_as = make_named_node(vocab::owl::SAME_AS)?;
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    triples.push(OxTriple::new(
                        nodes[i].clone(),
                        same_as.clone(),
                        nodes[j].clone(),
                    ));
                    triples.push(OxTriple::new(
                        nodes[j].clone(),
                        same_as.clone(),
                        nodes[i].clone(),
                    ));
                }
            }
        }

        Axiom::DifferentIndividuals(ax) => {
            let nodes: Vec<_> = ax
                .individuals
                .iter()
                .filter_map(|i| individual_iri_str(i))
                .filter_map(|s| make_named_node(s).ok())
                .collect();
            let diff_from = make_named_node(vocab::owl::DIFFERENT_FROM)?;
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    triples.push(OxTriple::new(
                        nodes[i].clone(),
                        diff_from.clone(),
                        nodes[j].clone(),
                    ));
                    triples.push(OxTriple::new(
                        nodes[j].clone(),
                        diff_from.clone(),
                        nodes[i].clone(),
                    ));
                }
            }
        }

        // ----------------------------------------------------------------
        // Annotation axioms
        // ----------------------------------------------------------------
        Axiom::AnnotationAssertion(ax) => {
            use crate::ontology::{AnnotationSubject, AnnotationValue};
            let subj: Option<NamedNode> = match &ax.subject {
                AnnotationSubject::IRI(iri) => make_named_node(iri.as_str()).ok(),
                AnnotationSubject::AnonymousIndividual(_) => None,
            };
            let pred = make_named_node(ax.property.iri.as_str()).ok();
            if let (Some(s), Some(p)) = (subj, pred) {
                let obj: Option<Term> = match &ax.value {
                    AnnotationValue::IRI(iri) => {
                        make_named_node(iri.as_str()).ok().map(Term::NamedNode)
                    }
                    AnnotationValue::Literal(lit) => ontology_literal_to_oxterm(lit).ok(),
                    AnnotationValue::AnonymousIndividual(_) => None,
                };
                if let Some(o) = obj {
                    triples.push(OxTriple::new(s, p, o));
                }
            }
        }

        // Remaining axiom types require complex blank-node patterns
        // (restrictions, property chains, SWRL, etc.) and are handled
        // by the reasoner/parser pipeline rather than here.
        _ => {}
    }

    Ok(triples)
}

/// Convert an atomic `ClassExpression` to a `NamedNode`; returns `None` for
/// complex expressions that cannot be mapped to a single IRI.
fn class_expr_to_named_node(expr: &ClassExpression) -> Option<NamedNode> {
    match expr {
        ClassExpression::Class(cls) => NamedNode::new(cls.iri.as_str()).ok(),
        _ => None,
    }
}

/// Extract the IRI string from an `Individual`, returning `None` for anonymous individuals.
fn individual_iri_str(ind: &crate::ontology::Individual) -> Option<&str> {
    match ind {
        crate::ontology::Individual::Named(n) => Some(n.iri.as_str()),
        crate::ontology::Individual::Anonymous(_) => None,
    }
}

/// Extract the IRI string from an `ObjectPropertyExpression`, returning `None` for chains.
fn obj_prop_expr_iri_str(expr: &crate::ontology::ObjectPropertyExpression) -> Option<&str> {
    match expr {
        crate::ontology::ObjectPropertyExpression::ObjectProperty(p) => Some(p.iri.as_str()),
        crate::ontology::ObjectPropertyExpression::InverseObjectProperty(p) => Some(p.iri.as_str()),
        crate::ontology::ObjectPropertyExpression::PropertyChain(_) => None,
    }
}

/// Extract the IRI string from a `DataPropertyExpression`.
fn data_prop_expr_iri_str(expr: &crate::ontology::DataPropertyExpression) -> Option<&str> {
    match expr {
        crate::ontology::DataPropertyExpression::DataProperty(p) => Some(p.iri.as_str()),
    }
}

/// Convert an ontology `Literal` to an Oxigraph `Term` (always a `Literal` variant).
fn ontology_literal_to_oxterm(lit: &crate::ontology::Literal) -> Result<Term> {
    let ox_lit = if let Some(lang) = &lit.language {
        OxLiteral::new_language_tagged_literal(&lit.value, lang).map_err(|e| Error::Sparql {
            message: e.to_string(),
        })?
    } else if let Some(dt_url) = &lit.datatype {
        let dt_node = make_named_node(dt_url.as_str())?;
        OxLiteral::new_typed_literal(&lit.value, dt_node)
    } else {
        OxLiteral::new_simple_literal(&lit.value)
    };
    Ok(Term::Literal(ox_lit))
}

/// Convert an oxidowl `Triple` to an Oxigraph `Quad` in the default graph.
pub fn oxidowl_triple_to_quad(t: &OxidowlTriple) -> Result<Quad> {
    let subject = rdfterm_to_oxsubject(&t.subject)?;
    let predicate = rdfterm_to_oxpredicate(&t.predicate)?;
    let object = rdfterm_to_oxterm(&t.object)?;
    Ok(Quad::new(
        subject,
        predicate,
        object,
        GraphName::DefaultGraph,
    ))
}

fn rdfterm_to_oxsubject(term: &RdfTerm) -> Result<NamedOrBlankNode> {
    match term {
        RdfTerm::Iri(url) => {
            let node = NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(NamedOrBlankNode::NamedNode(node))
        }
        RdfTerm::BlankNode(id) => {
            let node = OxBlankNode::new(id).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(NamedOrBlankNode::BlankNode(node))
        }
        RdfTerm::QuotedTriple(_) => Err(Error::Sparql {
            message: "Quoted triples in subject position require the rdf-12 oxigraph feature"
                .to_string(),
        }),
        _ => Err(Error::Sparql {
            message: format!("Invalid RDF term in subject position: {term:?}"),
        }),
    }
}

fn rdfterm_to_oxpredicate(term: &RdfTerm) -> Result<NamedNode> {
    match term {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
            message: e.to_string(),
        }),
        _ => Err(Error::Sparql {
            message: format!("Predicate must be an IRI, got: {term:?}"),
        }),
    }
}

fn rdfterm_to_oxterm(term: &RdfTerm) -> Result<Term> {
    match term {
        RdfTerm::Iri(url) => {
            let node = NamedNode::new(url.as_str()).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(Term::NamedNode(node))
        }
        RdfTerm::BlankNode(id) => {
            let node = OxBlankNode::new(id).map_err(|e| Error::Sparql {
                message: e.to_string(),
            })?;
            Ok(Term::BlankNode(node))
        }
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                OxLiteral::new_language_tagged_literal(value, lang).map_err(|e| Error::Sparql {
                    message: e.to_string(),
                })?
            } else if let Some(dt_url) = datatype {
                let dt_node = NamedNode::new(dt_url.as_str()).map_err(|e| Error::Sparql {
                    message: e.to_string(),
                })?;
                OxLiteral::new_typed_literal(value, dt_node)
            } else {
                OxLiteral::new_simple_literal(value)
            };
            Ok(Term::Literal(lit))
        }
        RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_) => Err(Error::Sparql {
            message: "Quoted triples in object position require the rdf-12 oxigraph feature"
                .to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers: oxigraph → oxidowl
// ---------------------------------------------------------------------------

/// Convert an Oxigraph `Term` to an oxidowl `RdfTerm`.
pub fn oxterm_to_rdfterm(term: &Term) -> RdfTerm {
    match term {
        Term::NamedNode(n) => {
            // Best-effort URL parse; fall back to opaque IRI as BlankNode label
            match Url::parse(n.as_str()) {
                Ok(url) => RdfTerm::Iri(url),
                Err(_) => RdfTerm::BlankNode(n.as_str().to_string()),
            }
        }
        Term::BlankNode(b) => RdfTerm::BlankNode(b.as_str().to_string()),
        Term::Literal(lit) => {
            let datatype = {
                let dt_str = lit.datatype().as_str();
                if dt_str == xsd::STRING {
                    None
                } else {
                    Url::parse(dt_str).ok()
                }
            };
            let language = lit.language().map(std::string::ToString::to_string);
            RdfTerm::Literal {
                value: lit.value().to_string(),
                datatype,
                language,
                direction: None,
            }
        }
        #[allow(unreachable_patterns)]
        _ => RdfTerm::BlankNode(term.to_string()),
    }
}

fn ox_subject_to_rdfterm(subj: &NamedOrBlankNode) -> RdfTerm {
    match subj {
        NamedOrBlankNode::NamedNode(n) => Url::parse(n.as_str())
            .map(RdfTerm::Iri)
            .unwrap_or_else(|_| RdfTerm::BlankNode(n.as_str().to_string())),
        NamedOrBlankNode::BlankNode(b) => RdfTerm::BlankNode(b.as_str().to_string()),
    }
}

fn ox_named_node_to_rdfterm(n: &NamedNode) -> RdfTerm {
    Url::parse(n.as_str())
        .map(RdfTerm::Iri)
        .unwrap_or_else(|_| RdfTerm::BlankNode(n.as_str().to_string()))
}

/// Convert an Oxigraph `Triple` (from a CONSTRUCT result) to an oxidowl `Triple`.
fn quad_triple_to_oxidowl(t: &OxTriple) -> Result<OxidowlTriple> {
    Ok(OxidowlTriple::new(
        ox_subject_to_rdfterm(&t.subject),
        ox_named_node_to_rdfterm(&t.predicate),
        oxterm_to_rdfterm(&t.object),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_store_creation() {
        let store = SparqlStore::new();
        assert!(store.is_ok());
    }

    #[test]
    fn test_default_construction() {
        let s = SparqlStore::default();
        assert_eq!(s.len().unwrap(), 0);
        assert!(s.is_empty().unwrap());
    }

    #[test]
    fn test_insert_and_select() {
        let mut s = SparqlStore::new().unwrap();

        // Insert a triple via SPARQL UPDATE
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let rows = s
            .execute_select(
                "PREFIX : <http://example.org/>  \
                 SELECT ?o WHERE { :alice :knows ?o }",
            )
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains_key("o"));
    }

    #[test]
    fn test_ask() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let result = s
            .execute_ask(
                "PREFIX : <http://example.org/>  \
                 ASK { :alice :knows :bob }",
            )
            .unwrap();
        assert!(result);

        let result2 = s
            .execute_ask(
                "PREFIX : <http://example.org/>  \
                 ASK { :alice :knows :charlie }",
            )
            .unwrap();
        assert!(!result2);
    }

    #[test]
    fn test_construct() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX : <http://example.org/>  \
             INSERT DATA { :alice :knows :bob }",
        )
        .unwrap();

        let triples = s
            .execute_construct(
                "PREFIX : <http://example.org/>  \
                 CONSTRUCT { ?s :knows ?o } WHERE { ?s :knows ?o }",
            )
            .unwrap();
        assert_eq!(triples.len(), 1);
    }

    #[test]
    fn test_incremental_updates() {
        let mut s = SparqlStore::new().unwrap();

        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/alice").unwrap(),
            RdfTerm::iri("http://example.org/knows").unwrap(),
            RdfTerm::iri("http://example.org/bob").unwrap(),
        );

        s.update_from_triples(&[triple.clone()]).unwrap();

        let count = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count, 1);

        s.remove_triples(&[triple]).unwrap();

        let count2 = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count2, 0);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut s = SparqlStore::new().unwrap();
        assert!(s.is_empty().unwrap());
        assert_eq!(s.len().unwrap(), 0);

        s.execute_update("PREFIX : <http://example.org/> INSERT DATA { :a :b :c }")
            .unwrap();

        assert!(!s.is_empty().unwrap());
        assert_eq!(s.len().unwrap(), 1);
    }

    #[test]
    fn test_contains_triple() {
        let mut s = SparqlStore::new().unwrap();
        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/alice").unwrap(),
            RdfTerm::iri("http://example.org/knows").unwrap(),
            RdfTerm::iri("http://example.org/bob").unwrap(),
        );
        s.update_from_triples(&[triple.clone()]).unwrap();
        assert!(s.contains_triple(&triple).unwrap());

        let other = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/alice").unwrap(),
            RdfTerm::iri("http://example.org/knows").unwrap(),
            RdfTerm::iri("http://example.org/charlie").unwrap(),
        );
        assert!(!s.contains_triple(&other).unwrap());
    }

    #[test]
    fn test_clear_default_graph() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update("PREFIX : <http://example.org/> INSERT DATA { :a :b :c . :d :e :f }")
            .unwrap();
        assert_eq!(s.len().unwrap(), 2);

        s.clear_default_graph().unwrap();
        assert_eq!(s.len().unwrap(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update("PREFIX : <http://example.org/> INSERT DATA { :a :b :c }")
            .unwrap();
        assert_eq!(s.len().unwrap(), 1);
        s.clear().unwrap();
        assert_eq!(s.len().unwrap(), 0);
    }

    #[test]
    fn test_load_turtle() {
        let mut s = SparqlStore::new().unwrap();
        let ttl = r#"
            @prefix ex: <http://example.org/> .
            ex:Alice ex:knows ex:Bob .
            ex:Bob ex:knows ex:Charlie .
        "#;
        s.load_turtle(ttl).unwrap();
        assert_eq!(s.len().unwrap(), 2);
    }

    #[test]
    fn test_load_rdf_ntriples() {
        let mut s = SparqlStore::new().unwrap();
        let nt =
            "<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .\n";
        s.load_rdf(nt, SerializationFormat::NTriples).unwrap();
        assert_eq!(s.len().unwrap(), 1);
    }

    #[test]
    fn test_dump_nquads() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> INSERT DATA { ex:Alice ex:knows ex:Bob }",
        )
        .unwrap();
        let nq = s.dump_nquads().unwrap();
        assert!(nq.contains("example.org/Alice"));
        assert!(nq.contains("example.org/knows"));
        assert!(nq.contains("example.org/Bob"));
    }

    #[test]
    fn test_dump_turtle() {
        let mut s = SparqlStore::new().unwrap();
        s.load_turtle("@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .")
            .unwrap();
        // dump_turtle serialises the default graph only; result must be non-empty
        let ttl = s.dump_turtle().unwrap();
        assert!(!ttl.is_empty());
        assert!(ttl.contains("example.org"));
    }

    #[test]
    fn test_dump_rejects_non_dataset_format() {
        let s = SparqlStore::new().unwrap();
        // dump() requires a dataset format; Turtle is not one
        let result = s.dump(SerializationFormat::Turtle);
        assert!(result.is_err());
    }

    #[test]
    fn test_dump_default_graph_ntriples() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> INSERT DATA { ex:Alice ex:knows ex:Bob }",
        )
        .unwrap();
        let nt = s.dump_default_graph(SerializationFormat::NTriples).unwrap();
        let text = String::from_utf8(nt).unwrap();
        assert!(text.contains("example.org/Alice"));
    }

    #[test]
    fn test_named_graph_insert_and_query() {
        let mut s = SparqlStore::new().unwrap();
        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/alice").unwrap(),
            RdfTerm::iri("http://example.org/knows").unwrap(),
            RdfTerm::iri("http://example.org/bob").unwrap(),
        );
        s.insert_into_named_graph(&triple, "http://example.org/graph1")
            .unwrap();

        // The triple should NOT appear in the default graph
        let count_default = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count_default, 0);

        // But it should appear when querying FROM the named graph
        let count_ng = s
            .execute_select("SELECT * FROM <http://example.org/graph1> WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count_ng, 1);
    }

    #[test]
    fn test_named_graphs_list() {
        let mut s = SparqlStore::new().unwrap();
        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/s").unwrap(),
            RdfTerm::iri("http://example.org/p").unwrap(),
            RdfTerm::iri("http://example.org/o").unwrap(),
        );
        s.insert_into_named_graph(&triple, "http://example.org/g1")
            .unwrap();
        s.insert_into_named_graph(&triple, "http://example.org/g2")
            .unwrap();

        let graphs = s.named_graphs().unwrap();
        assert_eq!(graphs.len(), 2);
    }

    #[test]
    fn test_clear_named_graph() {
        let mut s = SparqlStore::new().unwrap();
        let triple = OxidowlTriple::new(
            RdfTerm::iri("http://example.org/s").unwrap(),
            RdfTerm::iri("http://example.org/p").unwrap(),
            RdfTerm::iri("http://example.org/o").unwrap(),
        );
        s.insert_into_named_graph(&triple, "http://example.org/g1")
            .unwrap();
        assert_eq!(s.len().unwrap(), 1);

        s.clear_named_graph("http://example.org/g1").unwrap();
        assert_eq!(s.len().unwrap(), 0);
    }

    #[test]
    fn test_load_rdf_into_named_graph() {
        let mut s = SparqlStore::new().unwrap();
        let ttl = "@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .";
        s.load_rdf_into_graph(ttl, SerializationFormat::Turtle, "http://example.org/g")
            .unwrap();
        assert_eq!(s.len().unwrap(), 1);

        // Check the triple is in the named graph, not default
        let count_ng = s
            .execute_select("SELECT * FROM <http://example.org/g> WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count_ng, 1);

        let count_default = s
            .execute_select("SELECT * WHERE { ?s ?p ?o }")
            .unwrap()
            .len();
        assert_eq!(count_default, 0);
    }

    #[test]
    fn test_quads_for_pattern_wildcard() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             INSERT DATA { ex:Alice ex:knows ex:Bob . ex:Alice ex:likes ex:Charlie }",
        )
        .unwrap();

        let all = s.quads_for_pattern(None, None, None, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_quads_for_pattern_subject_filter() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             INSERT DATA { ex:Alice ex:knows ex:Bob . ex:Bob ex:knows ex:Charlie }",
        )
        .unwrap();

        let alice = RdfTerm::iri("http://example.org/Alice").unwrap();
        let filtered = s.quads_for_pattern(Some(&alice), None, None, None).unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_literal_round_trip() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            r#"PREFIX ex: <http://example.org/>
               PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
               INSERT DATA { ex:Alice ex:age "30"^^xsd:integer }"#,
        )
        .unwrap();

        let rows = s
            .execute_select(
                "SELECT ?age WHERE { <http://example.org/Alice> <http://example.org/age> ?age }",
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        let age = rows[0].get("age").unwrap();
        match age {
            RdfTerm::Literal { value, .. } => assert_eq!(value, "30"),
            other => panic!("Expected a literal, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // vocabulary / XsdType tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_vocabulary_constants_present() {
        assert!(vocabulary::xsd::STRING.ends_with("#string"));
        assert!(vocabulary::xsd::DATE_TIME.ends_with("#dateTime"));
        assert!(vocabulary::xsd::HEX_BINARY.ends_with("#hexBinary"));
        assert!(vocabulary::xsd::NON_NEGATIVE_INTEGER.ends_with("#nonNegativeInteger"));
        assert!(vocabulary::xsd::POSITIVE_INTEGER.ends_with("#positiveInteger"));
        assert!(vocabulary::rdf::TYPE.contains("type"));
        assert!(vocabulary::rdf::FIRST.contains("first"));
        assert!(vocabulary::rdfs::LABEL.contains("label"));
        assert!(vocabulary::rdfs::COMMENT.contains("comment"));
        assert!(vocabulary::owl::THING.contains("Thing"));
        assert!(vocabulary::owl::NOTHING.contains("Nothing"));
    }

    #[test]
    fn test_xsdtype_iri_round_trip() {
        // Every XsdType should survive an iri() → from_iri() round-trip.
        let types = [
            XsdType::String,
            XsdType::Boolean,
            XsdType::Decimal,
            XsdType::Float,
            XsdType::Double,
            XsdType::DateTime,
            XsdType::Time,
            XsdType::Date,
            XsdType::GYearMonth,
            XsdType::GYear,
            XsdType::GMonthDay,
            XsdType::GDay,
            XsdType::GMonth,
            XsdType::HexBinary,
            XsdType::Base64Binary,
            XsdType::AnyUri,
            XsdType::NormalizedString,
            XsdType::Token,
            XsdType::Language,
            XsdType::Nmtoken,
            XsdType::Name,
            XsdType::NcName,
            XsdType::Integer,
            XsdType::NonPositiveInteger,
            XsdType::NegativeInteger,
            XsdType::Long,
            XsdType::Int,
            XsdType::Short,
            XsdType::Byte,
            XsdType::NonNegativeInteger,
            XsdType::UnsignedLong,
            XsdType::UnsignedInt,
            XsdType::UnsignedShort,
            XsdType::UnsignedByte,
            XsdType::PositiveInteger,
        ];
        for t in types {
            assert_eq!(
                XsdType::from_iri(t.iri()),
                Some(t),
                "round-trip failed for {t}"
            );
        }
    }

    #[test]
    fn test_typed_literal_constructors() {
        let s = SparqlStore::boolean_literal(true);
        assert!(matches!(&s, RdfTerm::Literal { value, .. } if value == "true"));

        let i = SparqlStore::integer_literal(42);
        assert!(matches!(&i, RdfTerm::Literal { value, .. } if value == "42"));

        let d = SparqlStore::double_literal(3.14);
        // Check it has the xsd:double datatype IRI
        if let RdfTerm::Literal {
            datatype: Some(dt), ..
        } = &d
        {
            assert!(dt.as_str().ends_with("#double"));
        } else {
            panic!("expected typed literal");
        }

        let date = SparqlStore::date_literal("2024-12-31");
        if let RdfTerm::Literal {
            value,
            datatype: Some(dt),
            ..
        } = &date
        {
            assert_eq!(value, "2024-12-31");
            assert!(dt.as_str().ends_with("#date"));
        } else {
            panic!("expected typed literal");
        }
    }

    #[test]
    fn test_typed_literal_display() {
        // XsdType Display should return the IRI string
        assert_eq!(format!("{}", XsdType::Integer), vocabulary::xsd::INTEGER);
    }

    // -----------------------------------------------------------------------
    // Semantic helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_types_and_find_instances() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
             INSERT DATA { \
               ex:Alice rdf:type ex:Person . \
               ex:Bob   rdf:type ex:Person \
             }",
        )
        .unwrap();

        let types = s.get_types("http://example.org/Alice").unwrap();
        assert_eq!(types.len(), 1);

        let instances = s.find_instances("http://example.org/Person").unwrap();
        assert_eq!(instances.len(), 2);
    }

    #[test]
    fn test_find_subclasses_and_superclasses() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
             INSERT DATA { \
               ex:Student rdfs:subClassOf ex:Person . \
               ex:GradStudent rdfs:subClassOf ex:Student \
             }",
        )
        .unwrap();

        let subs = s.find_subclasses("http://example.org/Person").unwrap();
        assert_eq!(subs.len(), 1); // Student only (not transitive by default)

        let supers = s.find_superclasses("http://example.org/Student").unwrap();
        assert_eq!(supers.len(), 1);
    }

    #[test]
    fn test_find_labels_and_comments() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            r#"PREFIX ex: <http://example.org/>
               PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
               INSERT DATA {
                 ex:Alice rdfs:label "Alice Smith" .
                 ex:Alice rdfs:comment "A test individual"
               }"#,
        )
        .unwrap();

        let labels = s.find_labels("http://example.org/Alice").unwrap();
        assert_eq!(labels, vec!["Alice Smith".to_owned()]);

        let comments = s.find_comments("http://example.org/Alice").unwrap();
        assert_eq!(comments, vec!["A test individual".to_owned()]);
    }

    #[test]
    fn test_find_domain_and_range() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
             INSERT DATA { \
               ex:knows rdfs:domain ex:Person . \
               ex:knows rdfs:range  ex:Person \
             }",
        )
        .unwrap();

        let dom = s.find_domain("http://example.org/knows").unwrap();
        assert_eq!(dom.len(), 1);

        let rng = s.find_range("http://example.org/knows").unwrap();
        assert_eq!(rng.len(), 1);
    }

    #[test]
    fn test_find_equivalent_and_disjoint_classes() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX owl: <http://www.w3.org/2002/07/owl#> \
             INSERT DATA { \
               ex:Human owl:equivalentClass ex:Person . \
               ex:Cat   owl:disjointWith    ex:Dog \
             }",
        )
        .unwrap();

        let eq = s
            .find_equivalent_classes("http://example.org/Person")
            .unwrap();
        assert_eq!(eq.len(), 1);

        let dj = s.find_disjoint_classes("http://example.org/Cat").unwrap();
        assert_eq!(dj.len(), 1);
    }

    #[test]
    fn test_find_same_as_and_different_from() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX owl: <http://www.w3.org/2002/07/owl#> \
             INSERT DATA { \
               ex:Alice owl:sameAs ex:AliceSmith . \
               ex:Alice owl:differentFrom ex:Bob \
             }",
        )
        .unwrap();

        let same = s.find_same_as("http://example.org/Alice").unwrap();
        assert_eq!(same.len(), 1);

        let diff = s.find_different_from("http://example.org/Alice").unwrap();
        assert_eq!(diff.len(), 1);
    }

    #[test]
    fn test_rdf_list_members() {
        let mut s = SparqlStore::new().unwrap();
        s.execute_update(
            "PREFIX ex: <http://example.org/> \
             PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
             INSERT DATA { \
               ex:myList rdf:first ex:a ; \
                         rdf:rest  [ rdf:first ex:b ; rdf:rest rdf:nil ] \
             }",
        )
        .unwrap();

        let members = s.rdf_list_members("http://example.org/myList").unwrap();
        assert_eq!(members.len(), 2);
    }
}
