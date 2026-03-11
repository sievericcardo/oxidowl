//! Internal SHACL data model.
//!
//! Defines the Rust types that represent parsed SHACL shapes, targets,
//! property paths, constraints, severities, and node kinds.

use crate::semantics::RdfTerm;
use serde::{Deserialize, Serialize};

/// Unique identifier for a SHACL shape (its RDF node).
pub type ShapeId = RdfTerm;

/// A SHACL shape — either a `sh:NodeShape` or `sh:PropertyShape`.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaclShape {
    /// Node shape: constrains focus nodes themselves.
    NodeShape(NodeShape),
    /// Property shape: constrains the values reached via `sh:path`.
    PropertyShape(PropertyShape),
}

impl ShaclShape {
    /// The shape's RDF node (IRI or blank node).
    pub fn id(&self) -> &ShapeId {
        match self {
            ShaclShape::NodeShape(s) => &s.id,
            ShaclShape::PropertyShape(s) => &s.id,
        }
    }

    /// Whether the shape is deactivated (`sh:deactivated true`).
    pub fn is_deactivated(&self) -> bool {
        match self {
            ShaclShape::NodeShape(s) => s.deactivated,
            ShaclShape::PropertyShape(s) => s.deactivated,
        }
    }

    /// The explicit severity, or `Violation` by default.
    pub fn severity(&self) -> &ShaclSeverity {
        match self {
            ShaclShape::NodeShape(s) => &s.severity,
            ShaclShape::PropertyShape(s) => &s.severity,
        }
    }

    /// Static messages attached to the shape.
    pub fn messages(&self) -> &[ShaclMessage] {
        match self {
            ShaclShape::NodeShape(s) => &s.messages,
            ShaclShape::PropertyShape(s) => &s.messages,
        }
    }

    /// The targets declared on this shape.
    pub fn targets(&self) -> &[ShaclTarget] {
        match self {
            ShaclShape::NodeShape(s) => &s.targets,
            ShaclShape::PropertyShape(s) => &s.targets,
        }
    }

    /// Constraints attached directly to this shape.
    pub fn constraints(&self) -> &[ShaclConstraint] {
        match self {
            ShaclShape::NodeShape(s) => &s.constraints,
            ShaclShape::PropertyShape(s) => &s.constraints,
        }
    }
}

/// A `sh:NodeShape`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeShape {
    /// The shape node (IRI or blank node).
    pub id: ShapeId,
    /// Target declarations.
    pub targets: Vec<ShaclTarget>,
    /// Constraint parameters.
    pub constraints: Vec<ShaclConstraint>,
    /// Severity (default: `sh:Violation`).
    pub severity: ShaclSeverity,
    /// Static `sh:message` values.
    pub messages: Vec<ShaclMessage>,
    /// Whether this shape is deactivated.
    pub deactivated: bool,
    /// Nested property shapes referenced via `sh:property`.
    pub properties: Vec<ShapeId>,
}

/// A `sh:PropertyShape`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyShape {
    /// The shape node (IRI or blank node).
    pub id: ShapeId,
    /// The property path (`sh:path`).
    pub path: ShaclPath,
    /// Target declarations (property shapes rarely have their own targets but
    /// the spec allows it).
    pub targets: Vec<ShaclTarget>,
    /// Constraint parameters.
    pub constraints: Vec<ShaclConstraint>,
    /// Severity.
    pub severity: ShaclSeverity,
    /// Static `sh:message` values.
    pub messages: Vec<ShaclMessage>,
    /// Whether this shape is deactivated.
    pub deactivated: bool,
}

// ── Targets ─────────────────────────────────────────────────────────────────

/// A SHACL target declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaclTarget {
    /// `sh:targetNode` — a specific RDF term.
    TargetNode(RdfTerm),
    /// `sh:targetClass` — class (including subclasses via `rdfs:subClassOf*`).
    TargetClass(RdfTerm),
    /// Implicit class target: the shape node is also a class.
    ImplicitClassTarget(RdfTerm),
    /// `sh:targetSubjectsOf` — all subjects of the given predicate.
    TargetSubjectsOf(RdfTerm),
    /// `sh:targetObjectsOf` — all objects of the given predicate.
    TargetObjectsOf(RdfTerm),
}

// ── Property Paths ───────────────────────────────────────────────────────────

/// A SHACL property path expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShaclPath {
    /// A single predicate IRI.
    Predicate(String),
    /// A path sequence: `p1/p2/…`
    Sequence(Vec<ShaclPath>),
    /// A path alternative: `p1|p2|…`
    Alternative(Vec<ShaclPath>),
    /// An inverse path: `^p`
    Inverse(Box<ShaclPath>),
    /// Zero-or-more: `p*`
    ZeroOrMore(Box<ShaclPath>),
    /// One-or-more: `p+`
    OneOrMore(Box<ShaclPath>),
    /// Zero-or-one: `p?`
    ZeroOrOne(Box<ShaclPath>),
}

// ── Constraints ──────────────────────────────────────────────────────────────

/// A single SHACL constraint component instance.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaclConstraint {
    // ── Value type ────────────────────────────────────────────────────────
    /// `sh:class`
    Class(RdfTerm),
    /// `sh:datatype`
    Datatype(String),
    /// `sh:nodeKind`
    NodeKind(ShaclNodeKind),

    // ── Cardinality ───────────────────────────────────────────────────────
    /// `sh:minCount`
    MinCount(u64),
    /// `sh:maxCount`
    MaxCount(u64),

    // ── Value range ───────────────────────────────────────────────────────
    /// `sh:minExclusive`
    MinExclusive(RdfTerm),
    /// `sh:minInclusive`
    MinInclusive(RdfTerm),
    /// `sh:maxExclusive`
    MaxExclusive(RdfTerm),
    /// `sh:maxInclusive`
    MaxInclusive(RdfTerm),

    // ── String-based ──────────────────────────────────────────────────────
    /// `sh:minLength`
    MinLength(u64),
    /// `sh:maxLength`
    MaxLength(u64),
    /// `sh:pattern` (+ optional `sh:flags`)
    Pattern {
        pattern: String,
        flags: Option<String>,
    },
    /// `sh:languageIn`
    LanguageIn(Vec<String>),
    /// `sh:uniqueLang`
    UniqueLang(bool),

    // ── Property pair ─────────────────────────────────────────────────────
    /// `sh:equals`
    Equals(RdfTerm),
    /// `sh:disjoint`
    Disjoint(RdfTerm),
    /// `sh:lessThan`
    LessThan(RdfTerm),
    /// `sh:lessThanOrEquals`
    LessThanOrEquals(RdfTerm),

    // ── Logical ───────────────────────────────────────────────────────────
    /// `sh:not`
    Not(ShapeId),
    /// `sh:and`
    And(Vec<ShapeId>),
    /// `sh:or`
    Or(Vec<ShapeId>),
    /// `sh:xone`
    Xone(Vec<ShapeId>),

    // ── Shape-based ───────────────────────────────────────────────────────
    /// `sh:node`
    Node(ShapeId),
    /// `sh:property` (inline property shape sub-validation)
    Property(ShapeId),
    /// `sh:qualifiedValueShape` + counts
    QualifiedValue {
        shape_id: ShapeId,
        min_count: Option<u64>,
        max_count: Option<u64>,
        disjoint: bool,
    },

    // ── Other ─────────────────────────────────────────────────────────────
    /// `sh:closed` + `sh:ignoredProperties`
    Closed { ignored: Vec<RdfTerm> },
    /// `sh:hasValue`
    HasValue(RdfTerm),
    /// `sh:in`
    In(Vec<RdfTerm>),

    // ── SPARQL-based ──────────────────────────────────────────────────────
    /// `sh:sparql` constraint
    Sparql(SparqlConstraint),
    /// Custom SPARQL-based constraint component
    SparqlComponent(SparqlComponentConstraint),
}

/// A SPARQL-based constraint (`sh:sparql`).
#[derive(Debug, Clone, PartialEq)]
pub struct SparqlConstraint {
    /// The SPARQL SELECT query text.
    pub select: String,
    /// Resolved prefix declarations (prefix label → namespace IRI).
    pub prefixes: Vec<(String, String)>,
    /// Human-readable messages.
    pub messages: Vec<ShaclMessage>,
    /// Whether this constraint is deactivated.
    pub deactivated: bool,
}

/// A custom SPARQL-based constraint component invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct SparqlComponentConstraint {
    /// The constraint component IRI.
    pub component_iri: String,
    /// Parameter bindings (parameter local name → value).
    pub parameters: Vec<(String, RdfTerm)>,
    /// The source constraint component RDF term (for reporting).
    pub source_component: RdfTerm,
}

// ── Severity ─────────────────────────────────────────────────────────────────

/// SHACL validation result severity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ShaclSeverity {
    /// `sh:Violation` (most severe, default)
    #[default]
    Violation,
    /// `sh:Warning`
    Warning,
    /// `sh:Info`
    Info,
    /// A custom severity IRI.
    Custom(String),
}


impl ShaclSeverity {
    /// Return the IRI for this severity.
    pub fn as_iri(&self) -> &str {
        match self {
            ShaclSeverity::Violation => crate::validation::shacl::vocabulary::SH_VIOLATION,
            ShaclSeverity::Warning => crate::validation::shacl::vocabulary::SH_WARNING,
            ShaclSeverity::Info => crate::validation::shacl::vocabulary::SH_INFO,
            ShaclSeverity::Custom(iri) => iri.as_str(),
        }
    }

    /// Parse from an IRI string.
    pub fn from_iri(iri: &str) -> Self {
        use crate::validation::shacl::vocabulary::*;
        match iri {
            SH_VIOLATION => ShaclSeverity::Violation,
            SH_WARNING => ShaclSeverity::Warning,
            SH_INFO => ShaclSeverity::Info,
            other => ShaclSeverity::Custom(other.to_string()),
        }
    }
}

// ── NodeKind ─────────────────────────────────────────────────────────────────

/// The `sh:NodeKind` values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShaclNodeKind {
    /// `sh:IRI`
    IRI,
    /// `sh:BlankNode`
    BlankNode,
    /// `sh:Literal`
    Literal,
    /// `sh:BlankNodeOrIRI`
    BlankNodeOrIRI,
    /// `sh:BlankNodeOrLiteral`
    BlankNodeOrLiteral,
    /// `sh:IRIOrLiteral`
    IRIOrLiteral,
}

impl ShaclNodeKind {
    /// Parse from the IRI of a `sh:NodeKind` instance.
    pub fn from_iri(iri: &str) -> Option<Self> {
        use crate::validation::shacl::vocabulary::*;
        match iri {
            SH_IRI => Some(ShaclNodeKind::IRI),
            SH_BLANK_NODE => Some(ShaclNodeKind::BlankNode),
            SH_LITERAL => Some(ShaclNodeKind::Literal),
            SH_BLANK_NODE_OR_IRI => Some(ShaclNodeKind::BlankNodeOrIRI),
            SH_BLANK_NODE_OR_LITERAL => Some(ShaclNodeKind::BlankNodeOrLiteral),
            SH_IRI_OR_LITERAL => Some(ShaclNodeKind::IRIOrLiteral),
            _ => None,
        }
    }

    /// Returns `true` if the given `RdfTerm` satisfies this node kind.
    pub fn matches(&self, term: &RdfTerm) -> bool {
        match self {
            ShaclNodeKind::IRI => matches!(term, RdfTerm::Iri(_)),
            ShaclNodeKind::BlankNode => matches!(term, RdfTerm::BlankNode(_)),
            ShaclNodeKind::Literal => matches!(term, RdfTerm::Literal { .. }),
            ShaclNodeKind::BlankNodeOrIRI => {
                matches!(term, RdfTerm::Iri(_) | RdfTerm::BlankNode(_))
            }
            ShaclNodeKind::BlankNodeOrLiteral => {
                matches!(term, RdfTerm::BlankNode(_) | RdfTerm::Literal { .. })
            }
            ShaclNodeKind::IRIOrLiteral => {
                matches!(term, RdfTerm::Iri(_) | RdfTerm::Literal { .. })
            }
        }
    }
}

// ── Message ──────────────────────────────────────────────────────────────────

/// A SHACL message (optionally tagged with a language).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShaclMessage {
    /// Message text.
    pub value: String,
    /// Optional BCP47 language tag.
    pub language: Option<String>,
}

impl ShaclMessage {
    pub fn plain(text: impl Into<String>) -> Self {
        ShaclMessage {
            value: text.into(),
            language: None,
        }
    }
    pub fn lang(text: impl Into<String>, lang: impl Into<String>) -> Self {
        ShaclMessage {
            value: text.into(),
            language: Some(lang.into()),
        }
    }
}
