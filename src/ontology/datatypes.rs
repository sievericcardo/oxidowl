use crate::error::OxidowlError;
use crate::ontology::axioms::*;
use horned_owl::model::*;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// Complete OWL 2 Datatype Map implementation according to Section 4 of OWL 2 Specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OWL2Datatype {
    // Core XML Schema datatypes (Section 4.2)
    String,
    Boolean,
    Decimal,
    Float,
    Double,
    DateTime,
    Time,
    Date,
    GYearMonth,
    GYear,
    GMonthDay,
    GDay,
    GMonth,
    Duration,
    DateTimeStamp,
    Base64Binary,
    HexBinary,
    AnyURI,

    // Numeric datatypes derived from decimal and integer (Section 4.3)
    Integer,
    NonNegativeInteger,
    NonPositiveInteger,
    PositiveInteger,
    NegativeInteger,
    Long,
    Int,
    Short,
    Byte,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,

    // RDF datatypes (Section 4.5)
    XMLLiteral,
    Literal,
    PlainLiteral,

    // OWL 2 specific datatypes (Section 4.1)
    Real,
    Rational,

    // ── Extended XSD string subtypes ──
    NormalizedString,
    Token,
    Language,
    Name,
    NCName,
    NMTOKEN,
    NMTOKENS,

    // ── XSD duration subtypes ──
    DayTimeDuration,
    YearMonthDuration,

    // ── RDF extended datatypes ──
    LangString,
    RdfText,

    // ── OWL extended ──
    RdfPlainLiteral,
    RdfXMLLiteral,
}

impl OWL2Datatype {
    /// Get the IRI for this datatype
    #[must_use]
    pub fn iri(&self) -> crate::ontology::IRI {
        let iri_string = match self {
            // XML Schema datatypes
            OWL2Datatype::String => "http://www.w3.org/2001/XMLSchema#string",
            OWL2Datatype::Boolean => "http://www.w3.org/2001/XMLSchema#boolean",
            OWL2Datatype::Decimal => "http://www.w3.org/2001/XMLSchema#decimal",
            OWL2Datatype::Float => "http://www.w3.org/2001/XMLSchema#float",
            OWL2Datatype::Double => "http://www.w3.org/2001/XMLSchema#double",
            OWL2Datatype::DateTime => "http://www.w3.org/2001/XMLSchema#dateTime",
            OWL2Datatype::Time => "http://www.w3.org/2001/XMLSchema#time",
            OWL2Datatype::Date => "http://www.w3.org/2001/XMLSchema#date",
            OWL2Datatype::GYearMonth => "http://www.w3.org/2001/XMLSchema#gYearMonth",
            OWL2Datatype::GYear => "http://www.w3.org/2001/XMLSchema#gYear",
            OWL2Datatype::GMonthDay => "http://www.w3.org/2001/XMLSchema#gMonthDay",
            OWL2Datatype::GDay => "http://www.w3.org/2001/XMLSchema#gDay",
            OWL2Datatype::GMonth => "http://www.w3.org/2001/XMLSchema#gMonth",
            OWL2Datatype::Duration => "http://www.w3.org/2001/XMLSchema#duration",
            OWL2Datatype::DateTimeStamp => "http://www.w3.org/2001/XMLSchema#dateTimeStamp",
            OWL2Datatype::Base64Binary => "http://www.w3.org/2001/XMLSchema#base64Binary",
            OWL2Datatype::HexBinary => "http://www.w3.org/2001/XMLSchema#hexBinary",
            OWL2Datatype::AnyURI => "http://www.w3.org/2001/XMLSchema#anyURI",

            // Numeric datatypes
            OWL2Datatype::Integer => "http://www.w3.org/2001/XMLSchema#integer",
            OWL2Datatype::NonNegativeInteger => {
                "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            }
            OWL2Datatype::NonPositiveInteger => {
                "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
            }
            OWL2Datatype::PositiveInteger => "http://www.w3.org/2001/XMLSchema#positiveInteger",
            OWL2Datatype::NegativeInteger => "http://www.w3.org/2001/XMLSchema#negativeInteger",
            OWL2Datatype::Long => "http://www.w3.org/2001/XMLSchema#long",
            OWL2Datatype::Int => "http://www.w3.org/2001/XMLSchema#int",
            OWL2Datatype::Short => "http://www.w3.org/2001/XMLSchema#short",
            OWL2Datatype::Byte => "http://www.w3.org/2001/XMLSchema#byte",
            OWL2Datatype::UnsignedLong => "http://www.w3.org/2001/XMLSchema#unsignedLong",
            OWL2Datatype::UnsignedInt => "http://www.w3.org/2001/XMLSchema#unsignedInt",
            OWL2Datatype::UnsignedShort => "http://www.w3.org/2001/XMLSchema#unsignedShort",
            OWL2Datatype::UnsignedByte => "http://www.w3.org/2001/XMLSchema#unsignedByte",

            // RDF datatypes
            OWL2Datatype::XMLLiteral => "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
            OWL2Datatype::Literal => "http://www.w3.org/2000/01/rdf-schema#Literal",
            OWL2Datatype::PlainLiteral => "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",

            // OWL 2 datatypes
            OWL2Datatype::Real => "http://www.w3.org/2002/07/owl#real",
            OWL2Datatype::Rational => "http://www.w3.org/2002/07/owl#rational",

            // Extended string subtypes
            OWL2Datatype::NormalizedString => "http://www.w3.org/2001/XMLSchema#normalizedString",
            OWL2Datatype::Token => "http://www.w3.org/2001/XMLSchema#token",
            OWL2Datatype::Language => "http://www.w3.org/2001/XMLSchema#language",
            OWL2Datatype::Name => "http://www.w3.org/2001/XMLSchema#Name",
            OWL2Datatype::NCName => "http://www.w3.org/2001/XMLSchema#NCName",
            OWL2Datatype::NMTOKEN => "http://www.w3.org/2001/XMLSchema#NMTOKEN",
            OWL2Datatype::NMTOKENS => "http://www.w3.org/2001/XMLSchema#NMTOKENS",

            // Duration subtypes
            OWL2Datatype::DayTimeDuration => "http://www.w3.org/2001/XMLSchema#dayTimeDuration",
            OWL2Datatype::YearMonthDuration => "http://www.w3.org/2001/XMLSchema#yearMonthDuration",

            // RDF extended
            OWL2Datatype::LangString => "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
            OWL2Datatype::RdfText => "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML",

            // OWL extended (aliases)
            OWL2Datatype::RdfPlainLiteral => "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
            OWL2Datatype::RdfXMLLiteral => "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
        };

        crate::ontology::IRI::new(iri_string)
    }

    /// Check if this datatype is numeric
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            OWL2Datatype::Decimal
                | OWL2Datatype::Float
                | OWL2Datatype::Double
                | OWL2Datatype::Integer
                | OWL2Datatype::NonNegativeInteger
                | OWL2Datatype::NonPositiveInteger
                | OWL2Datatype::PositiveInteger
                | OWL2Datatype::NegativeInteger
                | OWL2Datatype::Long
                | OWL2Datatype::Int
                | OWL2Datatype::Short
                | OWL2Datatype::Byte
                | OWL2Datatype::UnsignedLong
                | OWL2Datatype::UnsignedInt
                | OWL2Datatype::UnsignedShort
                | OWL2Datatype::UnsignedByte
                | OWL2Datatype::Real
                | OWL2Datatype::Rational
        )
    }

    /// Check if this datatype is a date/time type
    #[must_use]
    pub fn is_datetime(&self) -> bool {
        matches!(
            self,
            OWL2Datatype::DateTime
                | OWL2Datatype::Time
                | OWL2Datatype::Date
                | OWL2Datatype::GYearMonth
                | OWL2Datatype::GYear
                | OWL2Datatype::GMonthDay
                | OWL2Datatype::GDay
                | OWL2Datatype::GMonth
                | OWL2Datatype::Duration
                | OWL2Datatype::DateTimeStamp
        )
    }

    /// Check if this datatype supports ordering
    #[must_use]
    pub fn is_ordered(&self) -> bool {
        self.is_numeric() || self.is_datetime() || matches!(self, OWL2Datatype::String)
    }

    /// Get the parent datatype in the hierarchy
    #[must_use]
    pub fn parent_datatype(&self) -> Option<OWL2Datatype> {
        match self {
            // Integer hierarchy
            OWL2Datatype::Integer => Some(OWL2Datatype::Decimal),
            OWL2Datatype::NonNegativeInteger => Some(OWL2Datatype::Integer),
            OWL2Datatype::NonPositiveInteger => Some(OWL2Datatype::Integer),
            OWL2Datatype::PositiveInteger => Some(OWL2Datatype::NonNegativeInteger),
            OWL2Datatype::NegativeInteger => Some(OWL2Datatype::NonPositiveInteger),
            OWL2Datatype::Long => Some(OWL2Datatype::Integer),
            OWL2Datatype::Int => Some(OWL2Datatype::Long),
            OWL2Datatype::Short => Some(OWL2Datatype::Int),
            OWL2Datatype::Byte => Some(OWL2Datatype::Short),
            OWL2Datatype::UnsignedLong => Some(OWL2Datatype::NonNegativeInteger),
            OWL2Datatype::UnsignedInt => Some(OWL2Datatype::UnsignedLong),
            OWL2Datatype::UnsignedShort => Some(OWL2Datatype::UnsignedInt),
            OWL2Datatype::UnsignedByte => Some(OWL2Datatype::UnsignedShort),

            // Real hierarchy
            OWL2Datatype::Decimal => Some(OWL2Datatype::Real),
            OWL2Datatype::Rational => Some(OWL2Datatype::Real),

            _ => None,
        }
    }
}

impl FromStr for OWL2Datatype {
    type Err = OxidowlError;

    fn from_str(iri: &str) -> Result<Self, Self::Err> {
        match iri {
            "http://www.w3.org/2001/XMLSchema#string" => Ok(OWL2Datatype::String),
            "http://www.w3.org/2001/XMLSchema#boolean" => Ok(OWL2Datatype::Boolean),
            "http://www.w3.org/2001/XMLSchema#decimal" => Ok(OWL2Datatype::Decimal),
            "http://www.w3.org/2001/XMLSchema#float" => Ok(OWL2Datatype::Float),
            "http://www.w3.org/2001/XMLSchema#double" => Ok(OWL2Datatype::Double),
            "http://www.w3.org/2001/XMLSchema#dateTime" => Ok(OWL2Datatype::DateTime),
            "http://www.w3.org/2001/XMLSchema#time" => Ok(OWL2Datatype::Time),
            "http://www.w3.org/2001/XMLSchema#date" => Ok(OWL2Datatype::Date),
            "http://www.w3.org/2001/XMLSchema#gYearMonth" => Ok(OWL2Datatype::GYearMonth),
            "http://www.w3.org/2001/XMLSchema#gYear" => Ok(OWL2Datatype::GYear),
            "http://www.w3.org/2001/XMLSchema#gMonthDay" => Ok(OWL2Datatype::GMonthDay),
            "http://www.w3.org/2001/XMLSchema#gDay" => Ok(OWL2Datatype::GDay),
            "http://www.w3.org/2001/XMLSchema#gMonth" => Ok(OWL2Datatype::GMonth),
            "http://www.w3.org/2001/XMLSchema#duration" => Ok(OWL2Datatype::Duration),
            "http://www.w3.org/2001/XMLSchema#dateTimeStamp" => Ok(OWL2Datatype::DateTimeStamp),
            "http://www.w3.org/2001/XMLSchema#base64Binary" => Ok(OWL2Datatype::Base64Binary),
            "http://www.w3.org/2001/XMLSchema#hexBinary" => Ok(OWL2Datatype::HexBinary),
            "http://www.w3.org/2001/XMLSchema#anyURI" => Ok(OWL2Datatype::AnyURI),
            "http://www.w3.org/2001/XMLSchema#integer" => Ok(OWL2Datatype::Integer),
            "http://www.w3.org/2001/XMLSchema#nonNegativeInteger" => {
                Ok(OWL2Datatype::NonNegativeInteger)
            }
            "http://www.w3.org/2001/XMLSchema#nonPositiveInteger" => {
                Ok(OWL2Datatype::NonPositiveInteger)
            }
            "http://www.w3.org/2001/XMLSchema#positiveInteger" => Ok(OWL2Datatype::PositiveInteger),
            "http://www.w3.org/2001/XMLSchema#negativeInteger" => Ok(OWL2Datatype::NegativeInteger),
            "http://www.w3.org/2001/XMLSchema#long" => Ok(OWL2Datatype::Long),
            "http://www.w3.org/2001/XMLSchema#int" => Ok(OWL2Datatype::Int),
            "http://www.w3.org/2001/XMLSchema#short" => Ok(OWL2Datatype::Short),
            "http://www.w3.org/2001/XMLSchema#byte" => Ok(OWL2Datatype::Byte),
            "http://www.w3.org/2001/XMLSchema#unsignedLong" => Ok(OWL2Datatype::UnsignedLong),
            "http://www.w3.org/2001/XMLSchema#unsignedInt" => Ok(OWL2Datatype::UnsignedInt),
            "http://www.w3.org/2001/XMLSchema#unsignedShort" => Ok(OWL2Datatype::UnsignedShort),
            "http://www.w3.org/2001/XMLSchema#unsignedByte" => Ok(OWL2Datatype::UnsignedByte),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral" => Ok(OWL2Datatype::XMLLiteral),
            "http://www.w3.org/2000/01/rdf-schema#Literal" => Ok(OWL2Datatype::Literal),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral" => {
                Ok(OWL2Datatype::PlainLiteral)
            }
            "http://www.w3.org/2002/07/owl#real" => Ok(OWL2Datatype::Real),
            "http://www.w3.org/2002/07/owl#rational" => Ok(OWL2Datatype::Rational),
            "http://www.w3.org/2001/XMLSchema#normalizedString" => Ok(OWL2Datatype::NormalizedString),
            "http://www.w3.org/2001/XMLSchema#token" => Ok(OWL2Datatype::Token),
            "http://www.w3.org/2001/XMLSchema#language" => Ok(OWL2Datatype::Language),
            "http://www.w3.org/2001/XMLSchema#Name" => Ok(OWL2Datatype::Name),
            "http://www.w3.org/2001/XMLSchema#NCName" => Ok(OWL2Datatype::NCName),
            "http://www.w3.org/2001/XMLSchema#NMTOKEN" => Ok(OWL2Datatype::NMTOKEN),
            "http://www.w3.org/2001/XMLSchema#NMTOKENS" => Ok(OWL2Datatype::NMTOKENS),
            "http://www.w3.org/2001/XMLSchema#dayTimeDuration" => Ok(OWL2Datatype::DayTimeDuration),
            "http://www.w3.org/2001/XMLSchema#yearMonthDuration" => Ok(OWL2Datatype::YearMonthDuration),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString" => Ok(OWL2Datatype::LangString),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML" => Ok(OWL2Datatype::RdfText),
            _ => Err(OxidowlError::InvalidDatatype(format!(
                "Unknown datatype IRI: {iri}"
            ))),
        }
    }
}

// ── DatatypeCategory ─────────────────────────────────────────────────────────

/// Category for OWL 2 datatype classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatatypeCategory {
    Numeric,
    String,
    Time,
    Boolean,
    Binary,
    URI,
    RdfSpecial,
    OwlSpecial,
}

// ── OWLFacet ─────────────────────────────────────────────────────────────────

/// Constraining facets as defined in OWL 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OWLFacet {
    XsdLength,
    XsdMinLength,
    XsdMaxLength,
    XsdPattern,
    XsdMinInclusive,
    XsdMaxInclusive,
    XsdMinExclusive,
    XsdMaxExclusive,
    XsdTotalDigits,
    XsdFractionDigits,
    RdfLangRange,
}

impl OWLFacet {
    #[must_use]
    pub fn iri(&self) -> crate::ontology::IRI {
        let xsd = "http://www.w3.org/2001/XMLSchema#";
        crate::ontology::IRI::new(match self {
            OWLFacet::XsdLength => format!("{xsd}length"),
            OWLFacet::XsdMinLength => format!("{xsd}minLength"),
            OWLFacet::XsdMaxLength => format!("{xsd}maxLength"),
            OWLFacet::XsdPattern => format!("{xsd}pattern"),
            OWLFacet::XsdMinInclusive => format!("{xsd}minInclusive"),
            OWLFacet::XsdMaxInclusive => format!("{xsd}maxInclusive"),
            OWLFacet::XsdMinExclusive => format!("{xsd}minExclusive"),
            OWLFacet::XsdMaxExclusive => format!("{xsd}maxExclusive"),
            OWLFacet::XsdTotalDigits => format!("{xsd}totalDigits"),
            OWLFacet::XsdFractionDigits => format!("{xsd}fractionDigits"),
            OWLFacet::RdfLangRange => "http://www.w3.org/1999/02/22-rdf-syntax-ns#langRange".to_string(),
        }.as_str())
    }

    #[must_use]
    pub fn short_name(&self) -> &'static str {
        match self {
            OWLFacet::XsdLength => "length",
            OWLFacet::XsdMinLength => "minLength",
            OWLFacet::XsdMaxLength => "maxLength",
            OWLFacet::XsdPattern => "pattern",
            OWLFacet::XsdMinInclusive => "minInclusive",
            OWLFacet::XsdMaxInclusive => "maxInclusive",
            OWLFacet::XsdMinExclusive => "minExclusive",
            OWLFacet::XsdMaxExclusive => "maxExclusive",
            OWLFacet::XsdTotalDigits => "totalDigits",
            OWLFacet::XsdFractionDigits => "fractionDigits",
            OWLFacet::RdfLangRange => "langRange",
        }
    }
}

// ── Extended OWL2Datatype Methods ────────────────────────────────────────────

impl OWL2Datatype {
    /// Human-readable short name.
    #[must_use]
    pub fn short_name(&self) -> &'static str {
        match self {
            OWL2Datatype::String => "string",
            OWL2Datatype::Boolean => "boolean",
            OWL2Datatype::Decimal => "decimal",
            OWL2Datatype::Float => "float",
            OWL2Datatype::Double => "double",
            OWL2Datatype::DateTime => "dateTime",
            OWL2Datatype::Time => "time",
            OWL2Datatype::Date => "date",
            OWL2Datatype::GYearMonth => "gYearMonth",
            OWL2Datatype::GYear => "gYear",
            OWL2Datatype::GMonthDay => "gMonthDay",
            OWL2Datatype::GDay => "gDay",
            OWL2Datatype::GMonth => "gMonth",
            OWL2Datatype::Duration => "duration",
            OWL2Datatype::DateTimeStamp => "dateTimeStamp",
            OWL2Datatype::Base64Binary => "base64Binary",
            OWL2Datatype::HexBinary => "hexBinary",
            OWL2Datatype::AnyURI => "anyURI",
            OWL2Datatype::Integer => "integer",
            OWL2Datatype::NonNegativeInteger => "nonNegativeInteger",
            OWL2Datatype::NonPositiveInteger => "nonPositiveInteger",
            OWL2Datatype::PositiveInteger => "positiveInteger",
            OWL2Datatype::NegativeInteger => "negativeInteger",
            OWL2Datatype::Long => "long",
            OWL2Datatype::Int => "int",
            OWL2Datatype::Short => "short",
            OWL2Datatype::Byte => "byte",
            OWL2Datatype::UnsignedLong => "unsignedLong",
            OWL2Datatype::UnsignedInt => "unsignedInt",
            OWL2Datatype::UnsignedShort => "unsignedShort",
            OWL2Datatype::UnsignedByte => "unsignedByte",
            OWL2Datatype::XMLLiteral => "XMLLiteral",
            OWL2Datatype::Literal => "Literal",
            OWL2Datatype::PlainLiteral => "PlainLiteral",
            OWL2Datatype::Real => "real",
            OWL2Datatype::Rational => "rational",
            OWL2Datatype::NormalizedString => "normalizedString",
            OWL2Datatype::Token => "token",
            OWL2Datatype::Language => "language",
            OWL2Datatype::Name => "Name",
            OWL2Datatype::NCName => "NCName",
            OWL2Datatype::NMTOKEN => "NMTOKEN",
            OWL2Datatype::NMTOKENS => "NMTOKENS",
            OWL2Datatype::DayTimeDuration => "dayTimeDuration",
            OWL2Datatype::YearMonthDuration => "yearMonthDuration",
            OWL2Datatype::LangString => "langString",
            OWL2Datatype::RdfText => "rdf:HTML",
            OWL2Datatype::RdfPlainLiteral => "rdf:PlainLiteral",
            OWL2Datatype::RdfXMLLiteral => "rdf:XMLLiteral",
        }
    }

    /// Category for reasoning purposes.
    #[must_use]
    pub fn category(&self) -> DatatypeCategory {
        if self.is_numeric() { DatatypeCategory::Numeric }
        else if self.is_datetime() || matches!(self, OWL2Datatype::Duration | OWL2Datatype::DayTimeDuration | OWL2Datatype::YearMonthDuration) { DatatypeCategory::Time }
        else if matches!(self, OWL2Datatype::Boolean) { DatatypeCategory::Boolean }
        else if matches!(self, OWL2Datatype::Base64Binary | OWL2Datatype::HexBinary) { DatatypeCategory::Binary }
        else if matches!(self, OWL2Datatype::AnyURI) { DatatypeCategory::URI }
        else if matches!(self, OWL2Datatype::Real | OWL2Datatype::Rational) { DatatypeCategory::OwlSpecial }
        else if matches!(self, OWL2Datatype::XMLLiteral | OWL2Datatype::PlainLiteral | OWL2Datatype::RdfXMLLiteral | OWL2Datatype::RdfPlainLiteral | OWL2Datatype::LangString | OWL2Datatype::RdfText | OWL2Datatype::Literal) { DatatypeCategory::RdfSpecial }
        else { DatatypeCategory::String }
    }

    /// Whether this is a built-in OWL 2 datatype.
    #[must_use]
    pub fn is_built_in(&self) -> bool { true }

    /// Get allowed constraining facets.
    #[must_use]
    pub fn facets(&self) -> Vec<OWLFacet> {
        match self.category() {
            DatatypeCategory::Numeric => vec![
                OWLFacet::XsdMinInclusive, OWLFacet::XsdMaxInclusive,
                OWLFacet::XsdMinExclusive, OWLFacet::XsdMaxExclusive,
                OWLFacet::XsdTotalDigits, OWLFacet::XsdFractionDigits,
            ],
            DatatypeCategory::String => vec![
                OWLFacet::XsdLength, OWLFacet::XsdMinLength, OWLFacet::XsdMaxLength,
                OWLFacet::XsdPattern, OWLFacet::RdfLangRange,
            ],
            DatatypeCategory::Time => vec![
                OWLFacet::XsdMinInclusive, OWLFacet::XsdMaxInclusive,
                OWLFacet::XsdMinExclusive, OWLFacet::XsdMaxExclusive,
            ],
            DatatypeCategory::Binary => vec![
                OWLFacet::XsdLength, OWLFacet::XsdMinLength, OWLFacet::XsdMaxLength,
            ],
            _ => vec![],
        }
    }

    /// Check if this datatype is a subtype of another.
    #[must_use]
    pub fn is_subtype_of(&self, other: &OWL2Datatype) -> bool {
        if self == other { return true; }
        let mut current = self.parent_datatype();
        while let Some(parent) = current {
            if &parent == other { return true; }
            current = parent.parent_datatype();
        }
        false
    }

    /// Validate a lexical form for this datatype.
    pub fn validate_lexical_form(&self, form: &str) -> crate::Result<()> {
        match self.category() {
            DatatypeCategory::Numeric => {
                form.parse::<f64>().map_err(|_| crate::Error::InvalidLiteral(
                    format!("Not a valid number: {form}")
                ))?;
            }
            DatatypeCategory::Boolean => {
                if form != "true" && form != "false" && form != "1" && form != "0" {
                    return Err(crate::Error::InvalidLiteral(format!("Not a boolean: {form}")));
                }
            }
            DatatypeCategory::Binary => {
                if self == &OWL2Datatype::HexBinary && !form.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(crate::Error::InvalidLiteral(format!("Not hex: {form}")));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Parse a lexical form into a literal.
    #[must_use]
    pub fn parse_literal(&self, form: &str) -> crate::ontology::Literal {
        crate::ontology::Literal::with_datatype(form.to_string(), self.iri())
    }

    /// Lookup from IRI string.
    #[must_use]
    pub fn from_iri(iri: &crate::ontology::IRI) -> Option<Self> {
        iri.as_str().parse().ok()
    }
}

/// Datatype definition axiom implementation (OWL 2 Section 9.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DatatypeDefinitionAxiom {
    pub id: AxiomId,
    pub datatype: IRI<String>,
    pub data_range: horned_owl::model::DataRange<String>,
    pub annotations: Vec<Annotation<String>>,
}

impl DatatypeDefinitionAxiom {
    #[must_use]
    pub fn new(
        id: AxiomId,
        datatype: IRI<String>,
        data_range: horned_owl::model::DataRange<String>,
        annotations: Vec<Annotation<String>>,
    ) -> Self {
        Self {
            id,
            datatype,
            data_range,
            annotations,
        }
    }
}

/// Enhanced `DataRange` enum to support all OWL 2 data range constructs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataRange {
    /// Named datatype
    Datatype(IRI<String>),
    /// Datatype restriction with facet constraints
    DatatypeRestriction {
        datatype: IRI<String>,
        facets: Vec<FacetRestriction>,
    },
    /// Intersection of data ranges
    DataIntersectionOf(Vec<DataRange>),
    /// Union of data ranges
    DataUnionOf(Vec<DataRange>),
    /// Complement of a data range
    DataComplementOf(Box<DataRange>),
    /// Enumeration of literals
    DataOneOf(Vec<horned_owl::model::Literal<String>>),
}

/// Facet restrictions for datatype restrictions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FacetRestriction {
    pub facet: ConstrainingFacet,
    pub literal: horned_owl::model::Literal<String>,
}

/// OWL 2 constraining facets (Section 4.3)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstrainingFacet {
    Length,
    MinLength,
    MaxLength,
    Pattern,
    Enumeration,
    WhiteSpace,
    MaxInclusive,
    MaxExclusive,
    MinInclusive,
    MinExclusive,
    TotalDigits,
    FractionDigits,
}

impl ConstrainingFacet {
    /// Get the IRI for this facet
    #[must_use]
    pub fn iri(&self) -> crate::ontology::IRI {
        let iri_string = match self {
            ConstrainingFacet::Length => "http://www.w3.org/2001/XMLSchema#length",
            ConstrainingFacet::MinLength => "http://www.w3.org/2001/XMLSchema#minLength",
            ConstrainingFacet::MaxLength => "http://www.w3.org/2001/XMLSchema#maxLength",
            ConstrainingFacet::Pattern => "http://www.w3.org/2001/XMLSchema#pattern",
            ConstrainingFacet::Enumeration => "http://www.w3.org/2001/XMLSchema#enumeration",
            ConstrainingFacet::WhiteSpace => "http://www.w3.org/2001/XMLSchema#whiteSpace",
            ConstrainingFacet::MaxInclusive => "http://www.w3.org/2001/XMLSchema#maxInclusive",
            ConstrainingFacet::MaxExclusive => "http://www.w3.org/2001/XMLSchema#maxExclusive",
            ConstrainingFacet::MinInclusive => "http://www.w3.org/2001/XMLSchema#minInclusive",
            ConstrainingFacet::MinExclusive => "http://www.w3.org/2001/XMLSchema#minExclusive",
            ConstrainingFacet::TotalDigits => "http://www.w3.org/2001/XMLSchema#totalDigits",
            ConstrainingFacet::FractionDigits => "http://www.w3.org/2001/XMLSchema#fractionDigits",
        };

        crate::ontology::IRI::new(iri_string)
    }

    /// Check if this facet is applicable to the given datatype
    #[must_use]
    pub fn is_applicable_to(&self, datatype: &OWL2Datatype) -> bool {
        match self {
            ConstrainingFacet::Length
            | ConstrainingFacet::MinLength
            | ConstrainingFacet::MaxLength => {
                matches!(
                    datatype,
                    OWL2Datatype::String | OWL2Datatype::Base64Binary | OWL2Datatype::HexBinary
                )
            }
            ConstrainingFacet::Pattern => {
                matches!(datatype, OWL2Datatype::String)
            }
            ConstrainingFacet::Enumeration => true, // Applicable to all datatypes
            ConstrainingFacet::WhiteSpace => {
                matches!(datatype, OWL2Datatype::String)
            }
            ConstrainingFacet::MaxInclusive
            | ConstrainingFacet::MaxExclusive
            | ConstrainingFacet::MinInclusive
            | ConstrainingFacet::MinExclusive => datatype.is_ordered(),
            ConstrainingFacet::TotalDigits | ConstrainingFacet::FractionDigits => {
                datatype.is_numeric()
            }
        }
    }
}

/// Datatype manager for handling OWL 2 datatypes
pub struct DatatypeManager {
    datatype_definitions: HashMap<url::Url, DatatypeDefinitionAxiom>,
    #[allow(dead_code)]
    facet_restrictions: HashMap<url::Url, Vec<FacetRestriction>>,
    datatype_hierarchy: HashMap<OWL2Datatype, HashSet<OWL2Datatype>>,
}

impl DatatypeManager {
    #[must_use]
    pub fn new() -> Self {
        let mut manager = Self {
            datatype_definitions: HashMap::new(),
            facet_restrictions: HashMap::new(),
            datatype_hierarchy: HashMap::new(),
        };

        manager.initialize_datatype_hierarchy();
        manager
    }

    /// Initialize the built-in datatype hierarchy
    fn initialize_datatype_hierarchy(&mut self) {
        // Build the datatype hierarchy based on parent relationships
        for datatype in self.all_owl2_datatypes() {
            if let Some(parent) = datatype.parent_datatype() {
                self.datatype_hierarchy
                    .entry(parent)
                    .or_default()
                    .insert(datatype);
            }
        }
    }

    /// Get all OWL 2 datatypes
    fn all_owl2_datatypes(&self) -> Vec<OWL2Datatype> {
        vec![
            OWL2Datatype::String,
            OWL2Datatype::Boolean,
            OWL2Datatype::Decimal,
            OWL2Datatype::Float,
            OWL2Datatype::Double,
            OWL2Datatype::DateTime,
            OWL2Datatype::Time,
            OWL2Datatype::Date,
            OWL2Datatype::GYearMonth,
            OWL2Datatype::GYear,
            OWL2Datatype::GMonthDay,
            OWL2Datatype::GDay,
            OWL2Datatype::GMonth,
            OWL2Datatype::Duration,
            OWL2Datatype::DateTimeStamp,
            OWL2Datatype::Base64Binary,
            OWL2Datatype::HexBinary,
            OWL2Datatype::AnyURI,
            OWL2Datatype::Integer,
            OWL2Datatype::NonNegativeInteger,
            OWL2Datatype::NonPositiveInteger,
            OWL2Datatype::PositiveInteger,
            OWL2Datatype::NegativeInteger,
            OWL2Datatype::Long,
            OWL2Datatype::Int,
            OWL2Datatype::Short,
            OWL2Datatype::Byte,
            OWL2Datatype::UnsignedLong,
            OWL2Datatype::UnsignedInt,
            OWL2Datatype::UnsignedShort,
            OWL2Datatype::UnsignedByte,
            OWL2Datatype::XMLLiteral,
            OWL2Datatype::Literal,
            OWL2Datatype::PlainLiteral,
            OWL2Datatype::Real,
            OWL2Datatype::Rational,
        ]
    }

    /// Add a datatype definition
    pub fn add_datatype_definition(&mut self, definition: DatatypeDefinitionAxiom) {
        if let Ok(url) = url::Url::parse(definition.datatype.as_ref()) {
            self.datatype_definitions.insert(url, definition);
        }
    }

    /// Check if a datatype is recognized (built-in or defined)
    #[must_use]
    pub fn is_recognized_datatype(&self, datatype_iri: &crate::ontology::IRI) -> bool {
        // Check if it's a built-in OWL 2 datatype
        if OWL2Datatype::from_str(&datatype_iri.to_string()).is_ok() {
            return true;
        }

        // Check if it's a user-defined datatype
        if let Ok(url) = datatype_iri.to_url() {
            self.datatype_definitions.contains_key(&url)
        } else {
            false
        }
    }

    /// Validate a literal value against its datatype
    pub fn validate_literal(&self, literal: &crate::ontology::Literal) -> Result<(), OxidowlError> {
        if let Some(datatype_url) = &literal.datatype {
            if let Ok(owl2_datatype) = OWL2Datatype::from_str(datatype_url.as_ref()) {
                return self.validate_against_builtin_datatype(&literal.value, &owl2_datatype);
            }

            if let Some(definition) = self.datatype_definitions.get(datatype_url) {
                return self.validate_against_defined_datatype(literal, definition);
            }

            return Err(OxidowlError::InvalidDatatype(format!(
                "Unrecognized datatype: {datatype_url}"
            )));
        }

        // Plain literal (no datatype)
        Ok(())
    }

    /// Validate a literal value against a built-in OWL 2 datatype
    fn validate_against_builtin_datatype(
        &self,
        value: &str,
        datatype: &OWL2Datatype,
    ) -> Result<(), OxidowlError> {
        match datatype {
            OWL2Datatype::Boolean => {
                if !matches!(value, "true" | "false" | "1" | "0") {
                    return Err(OxidowlError::InvalidLiteral(format!(
                        "Invalid boolean value: {value}"
                    )));
                }
            }
            OWL2Datatype::Integer => {
                if value.parse::<i64>().is_err() {
                    return Err(OxidowlError::InvalidLiteral(format!(
                        "Invalid integer value: {value}"
                    )));
                }
            }
            OWL2Datatype::Decimal => {
                if value.parse::<f64>().is_err() {
                    return Err(OxidowlError::InvalidLiteral(format!(
                        "Invalid decimal value: {value}"
                    )));
                }
            }
            OWL2Datatype::Float | OWL2Datatype::Double => {
                if value.parse::<f64>().is_err() && !matches!(value, "INF" | "-INF" | "NaN") {
                    return Err(OxidowlError::InvalidLiteral(format!(
                        "Invalid float/double value: {value}"
                    )));
                }
            }
            // Add validation for other datatypes as needed
            _ => {
                // Basic validation - just check that it's not empty for most types
                if value.is_empty() && !matches!(datatype, OWL2Datatype::String) {
                    return Err(OxidowlError::InvalidLiteral(format!(
                        "Empty value for datatype: {datatype:?}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate a literal against a user-defined datatype
    fn validate_against_defined_datatype(
        &self,
        _literal: &crate::ontology::Literal,
        _definition: &DatatypeDefinitionAxiom,
    ) -> Result<(), OxidowlError> {
        // This would implement validation against the data range constraints
        // For now, just accept it
        Ok(())
    }

    /// Check if one datatype is a subtype of another
    #[must_use]
    pub fn is_subtype_of(&self, subtype: &OWL2Datatype, supertype: &OWL2Datatype) -> bool {
        if subtype == supertype {
            return true;
        }

        if let Some(parent) = subtype.parent_datatype() {
            return self.is_subtype_of(&parent, supertype);
        }

        false
    }

    /// Get all subtypes of a datatype
    #[must_use]
    pub fn get_subtypes(&self, datatype: &OWL2Datatype) -> HashSet<OWL2Datatype> {
        self.datatype_hierarchy
            .get(datatype)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for DatatypeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datatype_iri_mapping() {
        assert_eq!(
            OWL2Datatype::String.iri().to_string(),
            "http://www.w3.org/2001/XMLSchema#string"
        );
        assert_eq!(
            OWL2Datatype::Integer.iri().to_string(),
            "http://www.w3.org/2001/XMLSchema#integer"
        );
    }

    #[test]
    fn test_datatype_hierarchy() {
        assert_eq!(
            OWL2Datatype::Integer.parent_datatype(),
            Some(OWL2Datatype::Decimal)
        );
        assert_eq!(
            OWL2Datatype::Int.parent_datatype(),
            Some(OWL2Datatype::Long)
        );
        assert_eq!(OWL2Datatype::String.parent_datatype(), None);
    }

    #[test]
    fn test_datatype_properties() {
        assert!(OWL2Datatype::Integer.is_numeric());
        assert!(OWL2Datatype::DateTime.is_datetime());
        assert!(OWL2Datatype::String.is_ordered());
        assert!(!OWL2Datatype::Boolean.is_numeric());
    }

    #[test]
    fn test_facet_applicability() {
        assert!(ConstrainingFacet::Length.is_applicable_to(&OWL2Datatype::String));
        assert!(ConstrainingFacet::MaxInclusive.is_applicable_to(&OWL2Datatype::Integer));
        assert!(!ConstrainingFacet::Length.is_applicable_to(&OWL2Datatype::Integer));
        assert!(!ConstrainingFacet::MaxInclusive.is_applicable_to(&OWL2Datatype::Boolean));
    }

    #[test]
    fn test_datatype_manager() {
        let manager = DatatypeManager::new();

        assert!(manager.is_recognized_datatype(&OWL2Datatype::String.iri()));
        assert!(manager.is_recognized_datatype(&OWL2Datatype::Integer.iri()));

        assert!(manager.is_subtype_of(&OWL2Datatype::Integer, &OWL2Datatype::Decimal));
        assert!(manager.is_subtype_of(&OWL2Datatype::Int, &OWL2Datatype::Integer));
        assert!(!manager.is_subtype_of(&OWL2Datatype::String, &OWL2Datatype::Integer));
    }

    #[test]
    fn test_literal_validation() {
        let manager = DatatypeManager::new();

        // Test boolean validation
        assert!(
            manager
                .validate_against_builtin_datatype("true", &OWL2Datatype::Boolean)
                .is_ok()
        );
        assert!(
            manager
                .validate_against_builtin_datatype("false", &OWL2Datatype::Boolean)
                .is_ok()
        );
        assert!(
            manager
                .validate_against_builtin_datatype("invalid", &OWL2Datatype::Boolean)
                .is_err()
        );

        // Test integer validation
        assert!(
            manager
                .validate_against_builtin_datatype("42", &OWL2Datatype::Integer)
                .is_ok()
        );
        assert!(
            manager
                .validate_against_builtin_datatype("-123", &OWL2Datatype::Integer)
                .is_ok()
        );
        assert!(
            manager
                .validate_against_builtin_datatype("not_a_number", &OWL2Datatype::Integer)
                .is_err()
        );
    }
}
