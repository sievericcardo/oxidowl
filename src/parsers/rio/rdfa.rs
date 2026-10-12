//! `RDFa` 1.1 parser core (DOM-based) and serializer — extracts RDF from
//! HTML/XHTML attribute markup.
//!
//! # Parsing (B5 Slice 4)
//!
//! The document is parsed with **html5ever** into an [`RcDom`] tree and the
//! `RDFa` Core 1.1 processing sequence (W3C Rec, §7.5 "Sequence" + §8 detail)
//! is evaluated over that DOM, depth-first:
//!
//! 1. **Evaluation context** per element: base IRI (document `<base href>`,
//!    else the caller-supplied base), parent subject, parent object,
//!    `@vocab`, prefix map (initialised with the defaults `rdf`, `rdfs`,
//!    `xsd`, `owl`, `foaf`), `@lang`/`xml:lang`, and the incomplete-triples
//!    list carried down the tree.
//! 2. **Subject/object rules**: `@about`, `@resource`, `@href`, `@src`,
//!    `@typeof`, `@property`, `@rel`, `@rev` per §7.5 steps 5–10 —
//!    including the `@rel`/`@rev` branch, the `@property` without
//!    `@content`/`@datatype` branch, anonymous (`@typeof`-only) subjects,
//!    root-element handling (empty `@about` → document base) and incomplete
//!    triple completion (§8.2) when a descendant establishes a subject.
//! 3. **Attribute values** (§7.4): CURIEs expand through the innermost
//!    `@prefix`/`xmlns:*` declaration (local `@prefix` > `@vocab`), terms
//!    expand through `@vocab`, absolute IRIs pass through, relative IRIs
//!    resolve against the current base (RFC 3986 reference resolution via
//!    [`url::Url::join`]).
//! 4. **Literals** (§8.3.1): `@property` honours `@content`,
//!    `@datatype` (typed literal; `datatype=""` → plain literal) and the
//!    current language tag (element `@lang` first, then inherited); without
//!    them the descendant text is used (whitespace preserved, as `RDFa`
//!    requires).
//! 5. **Blank nodes**: `@typeof`-only subjects, `@rel` without an object,
//!    `@about="_:label"` / `@resource="_:label"` (explicit labels stay
//!    stable within a document, §7.4.5) and safe-CURIE anonymous nodes
//!    (`@about="[]"` — see below).
//! 6. **Safe CURIEs** (§7.4): `[curie]` values in `@about`/`@resource` are
//!    evaluated inside the brackets — `[prefix:local]` expands through the
//!    innermost prefix declaration, `[_:label]` names an explicit blank
//!    node.  A value that does not resolve to an IRI (`[]`, `[unknown:x]`,
//!    `[not-a-curie]`) yields a **fresh blank node** recorded document-wide
//!    under the raw bracketed string, so the same safe CURIE always denotes
//!    the same node and later `@property`/`@typeof` resolution on that
//!    element targets it.  A bracketed value is never treated as an IRI
//!    (never falls back to the document base).
//! 7. **Initial context** (§9): the initial prefix map (`rdf`, `rdfs`,
//!    `xsd`, `owl`, `foaf`, `void`, `xhv`, `rel`, `rdfa`, `dc`, `dcterms`,
//!    `dc11`) and the initial **term map** are embedded locally (no
//!    network).  Bare terms resolve per §7.4.3: `@vocab` first, then the
//!    term map (case-sensitive, then case-insensitive), else ignored — so
//!    `rel="license"` expands to `xhv:license` without any `@vocab`.
//! 8. **`@inlist`** (§7.5 + §8.4): `@rel`/`@property` values marked
//!    `@inlist` accumulate into the evaluation context's *list mapping*
//!    instead of producing direct triples; the element that instantiates a
//!    list emits the RDF collection (`rdf:first`/`rdf:rest`/`rdf:nil`)
//!    after its children have been processed, connected to its subject.
//!    Pending `@inlist` predicates without an object become incomplete
//!    triples with direction *none* (§7.5 step 9), appending the subject a
//!    descendant establishes.
//! 9. **Vocabulary expansion** (§10, opt-in): `RdfaParser::with_vocabulary_expansion(true)`
//!    applies the `RDFa` Vocabulary Entailment rules (prp-spo1, prp-eqp1/2,
//!    cax-sco, cax-eqc1/2) to the output graph before it is mapped into the
//!    ontology.  Off by default (§10.2) and never dereferences remote
//!    vocabularies — only the triples already present in the document are
//!    used, so no network access happens at parse time.
//!
//! The evaluation first produces plain [`crate::semantics::Triple`]s, then
//! maps them into the [`Ontology`] through the **shared** RDF→OWL mapping
//! ([`crate::parsers::rdf_to_owl::map_rdf_triples`]) — the same code Turtle,
//! RDF/XML, JSON-LD, N3, `TriX`, HDT and `BinaryRDF` use.  Discovered `@prefix`
//! declarations are recorded in [`Ontology::prefixes`].
//!
//! ## Base IRI
//!
//! Resolution order: document `<base href>` (first in tree order, resolved
//! against the supplied base — HTML lets `<base>` affect even elements that
//! precede it) → caller-supplied base ([`parse_with_base`]) → the documented
//! placeholder [`DEFAULT_BASE`] when neither is present (so relative
//! references in a base-less document still resolve somewhere deterministic
//! instead of failing; callers that know the document URI should pass it).
//! An element carrying `xml:base` (XHTML host language) overrides the base
//! for itself and its descendants.  [`parse`] is the base-less convenience
//! entry point.
//!
//! ## Serializer ([`RdfaRenderer`])
//!
//! The writer converts the ontology through the shared canonical emission
//! ([`crate::semantics::owl_rdf_mapping::ontology_to_triples`]) and renders
//! one HTML block per subject: `@about`/`@typeof` on the subject element,
//! `@rel`+`@href` for resource objects, `@property` (+`@content`,
//! `@datatype`, `@lang`) for literals, `@prefix` declarations and an empty
//! `@vocab` on the root, and `@inlist` for RDF collections.  Re-parsing the
//! output reproduces the source axioms for the supported subset (asserted by
//! `tests/rdfa_tests.rs::serializer_round_trip_reproduces_core_axioms`).
//!
//! Constructs that have no `RDFa` spelling are **never dropped silently**:
//!
//! * `owl:imports` declarations are written as HTML comments carrying
//!   their IRI text;
//! * blank-node identifiers that are not stable under
//!   `sanitize_bnode_label` (they cannot round-trip through
//!   `@about="_:id"`) are written as HTML comments with their N-Triples
//!   text.
//!
//! SWRL `Axiom::Rule`s and ontology-level annotations of an ontology that
//! has no IRI **do** have a spelling: the shared triple emission attaches
//! them to the `urn:oxidowl:ontology` fallback subject (rule text under
//! `urn:oxidowl:swrlRule`) and `rdf_to_owl` decodes them back on reparse
//! (CR-011 M2).
//!
//! Documented divergences of the writer: RDF **collection cells** are
//! re-minted when re-parsed through `@inlist` (RDF lists are equivalent up
//! to blank-node renaming — the items, order and connection are preserved);
//! non-empty `@vocab` is deliberately not emitted because it would add an
//! `rdfa:usesVocabulary` triple that is not part of the source ontology.
//!
//! ## Preservation policy (never silently discard)
//!
//! Triples the shared mapping converts (declarations, class / object / data
//! property assertions, annotations, restrictions, lists, …) become axioms.
//! Triples the shared mapping is known **not** to convert are re-emitted
//! verbatim as ontology-level annotations with property
//! [`UNMAPPED_TRIPLE_PROPERTY`] (literal value = N-Triples syntax of the
//! triple).  Concretely preserved:
//!
//! * `owl:imports` triples (import resolution is intentionally a no-op in
//!   the shared mapping);
//! * structural OWL triples (`owl:onProperty`, `owl:someValuesFrom`,
//!   `rdf:first`/`rdf:rest`, …) that do **not** belong to a blank-node
//!   expression or RDF list the mapping actually reconstructs;
//! * `rdf:type` triples with a blank-node object that is not a resolvable
//!   class expression, and blank-node typing markers nobody references;
//! * `rdfs:subClassOf` / `owl:equivalentClass` / `owl:disjointWith` with a
//!   blank side that does not resolve;
//! * anything else the classifier cannot prove the mapping consumed
//!   (conservative: when in doubt, preserve).
//!
//! Structural triples that *are* consumed (restriction internals of a
//! claimed, resolvable restriction; cells of a resolved list; …) stay out of
//! the annotations, so nothing is duplicated.  The shared mapper's
//! `log::warn!` failure paths correspond 1:1 to triples this classifier
//! preserves — no silent drops remain.
//!
//! ## Documented gaps (deliberate divergences, not silent drops)
//!
//! * **`rdf:XMLLiteral`** uses concatenated descendant text and the
//!   element's children are still evaluated.  `RDFa` requires the value to be
//!   the descendant markup serialized with Exclusive XML Canonicalization
//!   (§7.5 step 10) and the recursion flag cleared; implementing C14N over
//!   the html5ever DOM (namespace/marker nodes, entity rewriting) is not
//!   feasible at low risk here, so the text form is kept and documented.
//! * **Initial context scope** — the embedded term/prefix map is the
//!   `RDFa` Core + HTML link-relation term set described above; host-language
//!   specific contexts (XHTML+RDFa's `xhv` terms for XHTML 1.x documents)
//!   are not loaded, and §9's remote context documents are never fetched.
//! * **Vocabulary expansion** never dereferences `rdfa:usesVocabulary`
//!   objects (§10.1 step 1) — it operates on the triples already in the
//!   output graph only.
//! * **Serializer**: see the *Serializer* section above (SWRL rules,
//!   `owl:imports`, unrepresentable blank-node labels and IRI-less
//!   ontology annotations are emitted as visible HTML comments; collection
//!   cells are re-minted on re-parse).
//!
//! [`RcDom`]: markup5ever_rcdom::RcDom

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use html5ever::tendril::stream::TendrilSink;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use url::Url;

use crate::Error;
use crate::Result;
use crate::ontology::{Annotation, AnnotationProperty, AnnotationValue, IRI, Literal, Ontology};
use crate::parsers::rdf_to_owl;
use crate::semantics::owl_rdf_mapping::ontology_to_triples;
use crate::semantics::{RdfTerm, Triple};

// ═══════════════════════════════════════════════════════════════════════════
// Vocabulary constants
// ═══════════════════════════════════════════════════════════════════════════

const RDF_TYPE: &str = rdf_to_owl::RDF_TYPE;
const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
const OWL_MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
const OWL_DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
const FOAF_NS: &str = "http://xmlns.com/foaf/0.1/";
const VOID_NS: &str = "http://rdfs.org/ns/void#";
/// XHTML vocabulary — the namespace historically used for `@rel` values and
/// the target of the initial-context `license` / `role` terms.
const XHV_NS: &str = "http://www.w3.org/1999/xhtml/vocab#";
/// IANA link-relation namespace — the target of the initial-context
/// link-relation terms (`about`, `alternate`, `canonical`, `describedby`,
/// `next`, `prev`, `search`, `stylesheet`, `icon`, …), exposed under the
/// conventional `rel` prefix (RFC 4287 §4.2.4.1, Atom+RDFa).
const REL_NS: &str = "http://www.iana.org/assignments/relation/";
/// `RDFa` vocabulary (`rdfa:usesVocabulary`, `rdfa:prefix`, …).
const RDFA_NS: &str = "http://www.w3.org/ns/rdfa#";
/// Dublin Core terms (`RDFa` Core initial context prefix).
const DC_NS: &str = "http://purl.org/dc/terms/";
const DC11_NS: &str = "http://purl.org/dc/elements/1.1/";

/// `rdfa:usesVocabulary` triple emitted whenever `@vocab` sets a vocabulary.
const RDFA_USES_VOCABULARY: &str = "http://www.w3.org/ns/rdfa#usesVocabulary";

/// Ontology-annotation property carrying triples the shared RDF→OWL mapping
/// cannot express, in N-Triples syntax.  Nothing is ever dropped silently.
pub const UNMAPPED_TRIPLE_PROPERTY: &str = "urn:oxidowl:rdfa:unmappedTriple";

/// Placeholder document base used when neither [`parse_with_base`] nor a
/// document `<base href>` provides one.  `.invalid` (RFC 2606) marks the
/// value as non-authoritative; pass the real document URI for production use.
pub const DEFAULT_BASE: &str = "http://example.invalid/";

/// Safety cap on DOM nesting depth during evaluation (guards pathological
/// input against stack exhaustion).  Elements below the cap are skipped with
/// a warning.  All recursive DOM walks in this module honour the same cap
/// (CR-011 B1), and [`RdfaParser::parse_with_base`] rejects documents whose
/// *actual* DOM depth exceeds it with a proper `Err` (CR-011 M4) so nothing
/// ever recurses deeper than this.
const MAX_DEPTH: usize = 1024;

/// Default byte budget for a single `RDFa` document (CR-011 M4): input larger
/// than this is rejected before html5ever ever sees it, bounding memory and
/// tokenizer CPU on untrusted input.  Override per parser instance with
/// [`RdfaParser::with_max_input_bytes`].
const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// Cap on the *estimated* source-tag nesting depth used as a cheap
/// pre-parse guard (CR-011 M4).  html5ever's tree construction is
/// O(depth²) on nested input (its `TreeBuilder::in_scope` walks the open
/// element stack per close tag), so a pathological ~400 000-deep document
/// burned > 4 min of CPU before reaching any of our code.  The estimate is
/// computed by a stack-free linear scan ([`estimate_source_depth`]) and is
/// deliberately far above [`MAX_DEPTH`] — it only exists to keep html5ever
/// itself fast; the precise limit is enforced on the parsed DOM.
const MAX_ESTIMATED_DEPTH: usize = 4096;

// ═══════════════════════════════════════════════════════════════════════════
// Entry points
// ═══════════════════════════════════════════════════════════════════════════

/// Parse an `RDFa` document with no caller-supplied base (see [`DEFAULT_BASE`]).
pub fn parse(content: &str) -> Result<Ontology> {
    parse_with_base(content, None)
}

/// Parse an `RDFa` document, resolving relative IRIs against `base` (which must
/// be an absolute IRI when given).  A document `<base href>` overrides the
/// supplied value, as HTML requires.
pub fn parse_with_base(content: &str, base: Option<&str>) -> Result<Ontology> {
    RdfaParser::new().parse_with_base(content, base)
}

/// `RDFa` 1.1 parser (html5ever DOM + `RDFa` Core 1.1 sequence + shared RDF→OWL
/// mapping).
#[derive(Debug, Clone)]
pub struct RdfaParser {
    /// §10.2: vocabulary expansion is opt-in and off by default.
    vocabulary_expansion: bool,
    /// Per-parser override of the input byte budget (CR-011 M4).
    max_input_bytes: usize,
}

impl Default for RdfaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RdfaParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            vocabulary_expansion: false,
            max_input_bytes: MAX_INPUT_BYTES,
        }
    }

    /// Enable `RDFa` Vocabulary Expansion (§10) on the output graph before it
    /// is mapped into the ontology.  Must be off by default (§10.2).  The
    /// expansion only reasons over triples already present in the document —
    /// no vocabulary IRI is dereferenced.
    #[must_use]
    pub fn with_vocabulary_expansion(mut self, enabled: bool) -> Self {
        self.vocabulary_expansion = enabled;
        self
    }

    /// Override the input byte budget (CR-011 M4).  Documents larger than
    /// `max_input_bytes` are rejected with a parse error before html5ever
    /// runs.  The default is [`MAX_INPUT_BYTES`].
    #[must_use]
    pub fn with_max_input_bytes(mut self, max_input_bytes: usize) -> Self {
        self.max_input_bytes = max_input_bytes;
        self
    }

    /// Whether vocabulary expansion is enabled on this parser.
    #[must_use]
    pub fn vocabulary_expansion_enabled(&self) -> bool {
        self.vocabulary_expansion
    }

    /// Parse an `RDFa` document without a caller-supplied base.
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        self.parse_with_base(content, None)
    }

    /// Parse an `RDFa` document with an optional caller-supplied base IRI.
    pub fn parse_with_base(&self, content: &str, base: Option<&str>) -> Result<Ontology> {
        // ── CR-011 M4: input guards, evaluated before html5ever runs ────
        if content.len() > self.max_input_bytes {
            return Err(Error::ParseError(format!(
                "RDFa: document is {} bytes, exceeding the {}-byte input budget \
                 (configure with RdfaParser::with_max_input_bytes)",
                content.len(),
                self.max_input_bytes
            )));
        }
        let estimated_depth = estimate_source_depth(content);
        if estimated_depth > MAX_ESTIMATED_DEPTH {
            return Err(Error::ParseError(format!(
                "RDFa: document nesting depth (est. {estimated_depth} levels) exceeds \
                 the pre-parse limit of {MAX_ESTIMATED_DEPTH}"
            )));
        }

        let supplied =
            match base {
                Some(raw) => Some(Url::parse(raw.trim()).map_err(|e| {
                    Error::ParseError(format!("RDFa: invalid base IRI '{raw}': {e}"))
                })?),
                None => None,
            };

        let dom = parse_document(RcDom::default(), ParseOpts::default()).one(content);

        // ── CR-011 B1/M4: precise depth guard on the parsed DOM ──────────
        // Computed with an explicit stack (never recursion), *before* the
        // recursive walks below run, so a deeply nested document yields a
        // proper `Err` instead of a stack-overflow abort.
        let actual_depth = dom_nesting_depth(&dom.document);
        if actual_depth > MAX_DEPTH {
            return Err(Error::ParseError(format!(
                "RDFa: document nesting depth ({actual_depth} levels) exceeds the \
                 {MAX_DEPTH}-level limit"
            )));
        }

        let scan = DocumentScan::run(&dom.document, supplied);
        let base_iri = scan.base;

        let initial = EvalContext {
            base: base_iri.clone(),
            parent_subject: RdfTerm::Iri(base_iri),
            parent_object: None,
            vocab: None,
            prefixes: initial_prefixes(),
            lang: None,
            incomplete: Vec::new(),
            list_map: ListMap::default(),
        };

        let mut evaluator = Evaluator::new(scan.explicit_labels);
        evaluator.evaluate(&dom.document, &initial, 0, false)?;

        let mut triples = evaluator.triples;
        if self.vocabulary_expansion {
            expand_vocabulary(&mut triples);
        }
        let mut ontology = Ontology::new();
        for (name, iri) in &evaluator.declared_prefixes {
            ontology
                .prefixes
                .insert(name.clone(), IRI::new(iri.as_str()));
        }
        rdf_to_owl::map_rdf_triples(&triples, &mut ontology)?;
        preserve_unmapped(&triples, &mut ontology);
        Ok(ontology)
    }
}

/// The initial prefix map of the embedded `RDFa` 1.1 initial context (§9).
/// Not recorded in [`Ontology::prefixes`] — only document-declared
/// `@prefix`/`xmlns` mappings are (they are the author's declarations).
fn initial_prefixes() -> HashMap<String, String> {
    let mut prefixes = HashMap::new();
    prefixes.insert("rdf".to_string(), RDF_NS.to_string());
    prefixes.insert("rdfs".to_string(), RDFS_NS.to_string());
    prefixes.insert("xsd".to_string(), XSD_NS.to_string());
    prefixes.insert("owl".to_string(), OWL_NS.to_string());
    prefixes.insert("foaf".to_string(), FOAF_NS.to_string());
    prefixes.insert("void".to_string(), VOID_NS.to_string());
    prefixes.insert("xhv".to_string(), XHV_NS.to_string());
    prefixes.insert("rel".to_string(), REL_NS.to_string());
    prefixes.insert("rdfa".to_string(), RDFA_NS.to_string());
    prefixes.insert("dc".to_string(), DC_NS.to_string());
    prefixes.insert("dcterms".to_string(), DC_NS.to_string());
    prefixes.insert("dc11".to_string(), DC11_NS.to_string());
    prefixes
}

/// The initial **term map** of the embedded `RDFa` 1.1 initial context (§9),
/// applied by §7.4.3 when no `@vocab` is in scope.  Sourced locally — §9
/// recommends embedding well-known initial contexts in the implementation,
/// and fetching one at parse time would be a network dependency.
///
/// * `license` / `role` — the `RDFa` Core initial-context terms
///   (`https://www.w3.org/2011/rdfa-context/rdfa-1.1`);
/// * everything else — IANA-registered link relation types under the `rel`
///   prefix, i.e. the values HTML puts in `@rel`.
fn initial_terms() -> &'static HashMap<&'static str, String> {
    use std::sync::LazyLock;
    static TERMS: LazyLock<HashMap<&'static str, String>> = LazyLock::new(|| {
        let mut terms: HashMap<&'static str, String> = HashMap::new();
        // RDFa Core initial context (xhv terms).
        terms.insert("license", format!("{XHV_NS}license"));
        terms.insert("role", format!("{XHV_NS}role"));
        // IANA link relations (the `rel` prefix).
        for term in [
            "about",
            "alternate",
            "appendix",
            "author",
            "bookmark",
            "canonical",
            "chapter",
            "cite",
            "contents",
            "copyright",
            "current",
            "describedby",
            "describes",
            "edit",
            "enclosure",
            "first",
            "glossary",
            "help",
            "icon",
            "index",
            "item",
            "last",
            "latest-version",
            "lrdd",
            "manifest",
            "micropub",
            "memento",
            "next",
            "nofollow",
            "noopener",
            "noreferrer",
            "original",
            "payment",
            "predecessor-version",
            "prefetch",
            "preload",
            "prev",
            "preview",
            "previous",
            "privacy-policy",
            "profile",
            "publication",
            "related",
            "revoke",
            "search",
            "section",
            "self",
            "service",
            "start",
            "stylesheet",
            "subsection",
            "successor-version",
            "tag",
            "terms-of-service",
            "type",
            "up",
            "version-history",
            "via",
            "webmention",
            "working-copy",
        ] {
            terms.insert(term, format!("{REL_NS}{term}"));
        }
        terms
    });
    &TERMS
}

/// §7.4.3 term resolution: `@vocab` first, then the initial term map
/// (case-sensitively, then case-insensitively), else the term is ignored.
fn expand_term_through_map(token: &str, vocab: Option<&str>) -> Option<String> {
    if let Some(vocab) = vocab {
        return Some(format!("{vocab}{token}"));
    }
    let terms = initial_terms();
    if let Some(iri) = terms.get(token) {
        return Some(iri.clone());
    }
    let lower = token.to_ascii_lowercase();
    terms.get(lower.as_str()).cloned()
}

// ═══════════════════════════════════════════════════════════════════════════
// Evaluation context
// ═══════════════════════════════════════════════════════════════════════════

/// The *list mapping* of the evaluation context (§7.2 / §7.5): a map from
/// predicate IRI to the items collected for `@inlist`.  Shared by reference
/// down the tree — an element that does not establish a new subject keeps
/// contributing to its parent's lists, exactly like the spec's "(a reference
/// of) the list mapping from the evaluation context".
type ListMap = Rc<RefCell<HashMap<String, Vec<RdfTerm>>>>;

/// A pending `@rel`/`@rev` triple whose object (or subject, for `@rev`) is
/// discovered by a descendant that establishes a new subject (§8.2).
///
/// `inlist` marks the direction-*none* form of §7.5 step 9: the predicate's
/// value is not completed as a triple but **appended to the list** bound to
/// that predicate in the given mapping.
#[derive(Debug, Clone)]
struct Incomplete {
    predicate: String,
    reverse: bool,
    inlist: Option<ListMap>,
}

/// `RDFa` evaluation context as defined in §7.2 (base, parent subject,
/// parent object, vocabulary, prefix map, language, incomplete triples,
/// list mapping).
#[derive(Debug, Clone)]
struct EvalContext {
    base: Url,
    parent_subject: RdfTerm,
    parent_object: Option<RdfTerm>,
    vocab: Option<String>,
    prefixes: HashMap<String, String>,
    lang: Option<String>,
    incomplete: Vec<Incomplete>,
    list_map: ListMap,
}

/// Result of applying the §7.5 sequence to one element: the context the
/// children run in, plus — when this element instantiated a list mapping —
/// the mapping and the subject its collections are attached to (emitted
/// after the subtree has been processed).
struct ElementStep {
    child_ctx: EvalContext,
    list_owner: Option<(ListMap, RdfTerm)>,
}

/// The `RDFa` value attributes of one element (§5).  `Option::is_some` is the
/// *presence* test the processing sequence uses — an empty value is still a
/// present attribute.
#[derive(Debug, Clone, Copy, Default)]
struct RdfaAttrs<'a> {
    about: Option<&'a str>,
    resource: Option<&'a str>,
    href: Option<&'a str>,
    src: Option<&'a str>,
    rel: Option<&'a str>,
    rev: Option<&'a str>,
    property: Option<&'a str>,
    r#typeof: Option<&'a str>,
    content: Option<&'a str>,
    datatype: Option<&'a str>,
}

impl<'a> RdfaAttrs<'a> {
    fn from_slice(attrs: &'a [(String, String)]) -> Self {
        let get = |name: &str| -> Option<&'a str> {
            attrs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
        };
        Self {
            about: get("about"),
            resource: get("resource"),
            href: get("href"),
            src: get("src"),
            rel: get("rel"),
            rev: get("rev"),
            property: get("property"),
            r#typeof: get("typeof"),
            content: get("content"),
            datatype: get("datatype"),
        }
    }

    fn has_relrev(&self) -> bool {
        self.rel.is_some() || self.rev.is_some()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Document base + explicit blank-node pre-scan
// ═══════════════════════════════════════════════════════════════════════════

/// One pre-evaluation pass over the document: the base IRI and every
/// explicitly authored blank-node label (§7.4.5 requires generated nodes
/// never to collide with explicitly referenced ones — so the labels are
/// reserved *before* any node is minted instead of being renamed after the
/// fact, which would break `@about="_:id"` round-trips through the writer).
struct DocumentScan {
    base: Url,
    explicit_labels: HashSet<String>,
    base_found: bool,
}

impl DocumentScan {
    fn run(root: &Handle, supplied: Option<Url>) -> Self {
        let fallback =
            supplied.unwrap_or_else(|| Url::parse(DEFAULT_BASE).expect("DEFAULT_BASE is valid"));
        let mut scan = Self {
            base: fallback.clone(),
            explicit_labels: HashSet::new(),
            base_found: false,
        };
        scan.walk(root, &fallback, 0);
        scan
    }

    fn walk(&mut self, node: &Handle, fallback: &Url, depth: usize) {
        // CR-011 B1: never recurse past the shared depth cap.  Documents this
        // deep are rejected up front by `parse_with_base`, so this is a
        // defence-in-depth backstop — truncating is always preferable to an
        // unbounded recursion (which aborts the whole process).
        if depth > MAX_DEPTH {
            log::warn!("RDFa: pre-scan depth cap ({MAX_DEPTH}) reached; truncating subtree");
            return;
        }
        if let NodeData::Element { name, attrs, .. } = &node.data {
            let attrs = attrs.borrow();
            let local: &str = &name.local;
            if local.eq_ignore_ascii_case("base")
                && !self.base_found
                && let Some(attr) = attrs
                    .iter()
                    .find(|a| a.name.local.as_ref().eq_ignore_ascii_case("href"))
            {
                match fallback.join(attr.value.trim()) {
                    Ok(url) => {
                        self.base = url;
                        self.base_found = true;
                    }
                    Err(e) => log::warn!("RDFa: ignoring unresolvable <base href>: {e}"),
                }
            }
            for attr in attrs.iter() {
                let full = attribute_full_name(&attr.name);
                let is_reference = full == "about" || full == "resource";
                if is_reference && let Some(label) = explicit_label_of(attr.value.trim()) {
                    self.explicit_labels.insert(label);
                }
            }
        }
        let children = node.children.borrow().clone();
        // Mirror `Evaluator::evaluate`'s depth accounting: only elements
        // increment the depth (the document node's children are level 0).
        let child_depth = if matches!(&node.data, NodeData::Element { .. }) {
            depth + 1
        } else {
            depth
        };
        for child in &children {
            self.walk(child, fallback, child_depth);
        }
    }
}

/// The explicit `_:label` an `@about`/`@resource` value refers to, if any —
/// both `_:label` and the safe-CURIE form `[_:label]`.
fn explicit_label_of(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let inner = if raw.starts_with('[') && raw.ends_with(']') && raw.len() >= 2 {
        raw[1..raw.len() - 1].trim()
    } else {
        raw
    };
    inner
        .strip_prefix("_:")
        .filter(|label| !label.is_empty())
        .map(sanitize_bnode_label)
}

// ═══════════════════════════════════════════════════════════════════════════
// Evaluator
// ═══════════════════════════════════════════════════════════════════════════

struct Evaluator {
    /// Deduplicated triples in document order.
    triples: Vec<Triple>,
    seen: HashSet<Triple>,
    bnode_counter: u64,
    /// Prefixes declared by the document (`@prefix` / `xmlns`), recorded on
    /// the resulting ontology.
    declared_prefixes: HashMap<String, String>,
    /// Blank-node labels explicitly authored in the document (§7.4.5) —
    /// generated labels skip over them.
    explicit_labels: HashSet<String>,
    /// Resolutions of bracketed safe-CURIE values, keyed by the raw
    /// bracketed string: `[]` / `[unresolvable]` → the blank node minted for
    /// it, `[prefix:local]` → the expanded IRI.  Recorded so the same value
    /// always denotes the same node for the rest of the document (and for
    /// later `@property`/`@typeof` resolution on elements carrying it).
    safe_curies: HashMap<String, RdfTerm>,
}

impl Evaluator {
    fn new(explicit_labels: HashSet<String>) -> Self {
        Self {
            triples: Vec::new(),
            seen: HashSet::new(),
            bnode_counter: 0,
            declared_prefixes: HashMap::new(),
            explicit_labels,
            safe_curies: HashMap::new(),
        }
    }

    fn push(&mut self, subject: RdfTerm, predicate: &str, object: RdfTerm) {
        let Ok(predicate) = Url::parse(predicate) else {
            log::warn!("RDFa: skipping triple with non-absolute predicate '{predicate}'");
            return;
        };
        let triple = Triple {
            subject,
            predicate: RdfTerm::Iri(predicate),
            object,
        };
        if self.seen.insert(triple.clone()) {
            self.triples.push(triple);
        }
    }

    fn fresh_bnode(&mut self) -> RdfTerm {
        loop {
            self.bnode_counter += 1;
            let id = format!("rdfa_gen_{}", self.bnode_counter);
            if !self.explicit_labels.contains(&id) {
                return RdfTerm::BlankNode(id);
            }
        }
    }

    /// Blank node for an explicit `_:label` CURIE (§7.4.5): stable within the
    /// document.  Collisions with generated labels cannot occur —
    /// [`Self::fresh_bnode`] skips every pre-scanned explicit label.
    fn explicit_bnode(&mut self, label: &str) -> RdfTerm {
        RdfTerm::BlankNode(sanitize_bnode_label(label))
    }

    fn evaluate(
        &mut self,
        node: &Handle,
        ctx: &EvalContext,
        depth: usize,
        is_root: bool,
    ) -> Result<()> {
        match &node.data {
            NodeData::Document => {
                // The document element (the first element child, always
                // `<html>` after html5ever's tree construction) acts as if it
                // had an empty @about → the document base IRI.
                let children = node.children.borrow().clone();
                let mut seen_element = false;
                for child in &children {
                    let is_doc_element =
                        !seen_element && matches!(&child.data, NodeData::Element { .. });
                    seen_element |= is_doc_element;
                    self.evaluate(child, ctx, depth, is_doc_element)?;
                }
            }
            NodeData::Element {
                name,
                attrs,
                template_contents,
                ..
            } => {
                if depth > MAX_DEPTH {
                    log::warn!(
                        "RDFa: DOM nesting deeper than {MAX_DEPTH}; skipping subtree at <{}>",
                        &*name.local
                    );
                    return Ok(());
                }
                let attrs: Vec<(String, String)> = attrs
                    .borrow()
                    .iter()
                    .map(|a| (attribute_full_name(&a.name), a.value.to_string()))
                    .collect();
                let template = template_contents.borrow().clone();
                let step = self.eval_element(node, &attrs, ctx, is_root)?;
                let children = node.children.borrow().clone();
                for child in &children {
                    self.evaluate(child, &step.child_ctx, depth + 1, false)?;
                }
                // `<template>` children live outside the regular child list
                // in html5ever's rcdom; evaluate them in the same context.
                if let Some(template) = template {
                    let inner = template.children.borrow().clone();
                    for child in &inner {
                        self.evaluate(child, &step.child_ctx, depth + 1, false)?;
                    }
                }
                // §7.5 final step: the element that instantiated a list
                // mapping emits its RDF collections (§8.4) once its whole
                // subtree has contributed to them.
                if let Some((map, subject)) = &step.list_owner {
                    self.generate_list_triples(map, subject);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Emit the RDF collection triples for every predicate in a list mapping
    /// this element owns (§7.5 "Finally", §8.4): an empty list connects
    /// `rdf:nil` directly, otherwise each item gets an `rdf:first` cell whose
    /// `rdf:rest` chains to the next cell (last → `rdf:nil`) and the subject
    /// is connected to the first cell.
    fn generate_list_triples(&mut self, map: &ListMap, subject: &RdfTerm) {
        let entries: Vec<(String, Vec<RdfTerm>)> = {
            let borrowed = map.borrow();
            borrowed
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let nil =
            RdfTerm::Iri(Url::parse(rdf_to_owl::RDF_NIL).expect("rdf:nil is a valid absolute IRI"));
        for (predicate, items) in entries {
            if items.is_empty() {
                self.push(subject.clone(), &predicate, nil.clone());
                continue;
            }
            let cells: Vec<RdfTerm> = items.iter().map(|_| self.fresh_bnode()).collect();
            for (i, item) in items.iter().enumerate() {
                self.push(cells[i].clone(), rdf_to_owl::RDF_FIRST, item.clone());
                let rest = if i + 1 < cells.len() {
                    cells[i + 1].clone()
                } else {
                    nil.clone()
                };
                self.push(cells[i].clone(), rdf_to_owl::RDF_REST, rest);
            }
            self.push(subject.clone(), &predicate, cells[0].clone());
        }
    }

    /// Apply the `RDFa` Core 1.1 sequence (§7.5) to one element and return the
    /// evaluation context its children run in (§7.5 final step), plus the
    /// list mapping this element owns (if any) to be emitted after the
    /// children have been processed.
    fn eval_element(
        &mut self,
        node: &Handle,
        attrs: &[(String, String)],
        ctx: &EvalContext,
        is_root: bool,
    ) -> Result<ElementStep> {
        let get = |name: &str| -> Option<&str> {
            attrs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
        };
        let rdfa = RdfaAttrs::from_slice(attrs);

        // ── Local context values (initialised from the incoming context) ──
        let mut prefixes = ctx.prefixes.clone();
        let mut vocab = ctx.vocab.clone();
        let mut lang = ctx.lang.clone();
        let mut base = ctx.base.clone();
        let mut local_incomplete: Vec<Incomplete> = Vec::new();
        let mut list_map: ListMap = ctx.list_map.clone();

        // ── `xml:base` (XHTML host language): overrides the base for this
        //    element and its descendants ──────────────────────────────────
        if let Some(raw) = get("xml:base") {
            let raw = raw.trim();
            if raw.is_empty() {
                // An empty xml:base resolves to the containing base — keep it.
            } else {
                match base.join(raw) {
                    Ok(url) => base = url,
                    Err(e) => log::warn!("RDFa: ignoring unresolvable xml:base='{raw}': {e}"),
                }
            }
        }

        // ── Step 2: @vocab (§8.1) ────────────────────────────────────────
        if let Some(raw) = get("vocab") {
            let raw = raw.trim();
            if raw.is_empty() {
                vocab = None;
            } else if let Some(iri) = resolve_iri(raw, &base) {
                self.push(
                    RdfTerm::Iri(base.clone()),
                    RDFA_USES_VOCABULARY,
                    RdfTerm::Iri(iri.clone()),
                );
                vocab = Some(iri.to_string());
            } else {
                log::warn!("RDFa: ignoring unresolvable @vocab='{raw}'");
            }
        }

        // ── Step 3: prefix mappings — @xmlns first, then @prefix (§7.5) ──
        for (key, value) in attrs {
            if key == "xmlns" {
                prefixes.insert(String::new(), value.clone());
                self.declared_prefixes.insert(String::new(), value.clone());
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                prefixes.insert(prefix.to_lowercase(), value.clone());
                self.declared_prefixes
                    .insert(prefix.to_lowercase(), value.clone());
            }
        }
        if let Some(raw) = get("prefix") {
            parse_prefix_declaration(raw, &mut prefixes, &mut self.declared_prefixes);
        }

        // ── Step 4: language (@lang / xml:lang) ──────────────────────────
        if let Some(raw) = get("lang").or_else(|| get("xml:lang")) {
            let trimmed = raw.trim();
            lang = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        let has_typeof = rdfa.r#typeof.is_some();
        let inlist = get("inlist").is_some();
        let mut new_subject: Option<RdfTerm> = None;
        let mut current_object: Option<RdfTerm> = None;
        let mut typed_resource: Option<RdfTerm> = None;
        let mut skip = false;

        if rdfa.has_relrev() {
            // ── §7.5 step 6: element carries @rel / @rev ────────────────
            if let Some(raw) = rdfa.about {
                new_subject = Some(self.resolve_resource(raw, &base, &prefixes));
            }
            if has_typeof && rdfa.about.is_some() {
                typed_resource.clone_from(&new_subject);
            }
            if new_subject.is_none() {
                if is_root {
                    new_subject = Some(RdfTerm::Iri(base.clone()));
                } else if let Some(parent_object) = ctx.parent_object.clone() {
                    new_subject = Some(parent_object);
                }
            }
            current_object = match self.object_from_attrs(rdfa, &base, &prefixes) {
                Some(term) => Some(term),
                None if has_typeof && rdfa.about.is_none() => Some(self.fresh_bnode()),
                None => None,
            };
            if has_typeof && rdfa.about.is_none() {
                typed_resource.clone_from(&current_object);
            }
        } else if rdfa.property.is_some() && rdfa.content.is_none() && rdfa.datatype.is_none() {
            // ── §7.5 step 5a: @property without @content/@datatype ──
            if let Some(raw) = rdfa.about {
                new_subject = Some(self.resolve_resource(raw, &base, &prefixes));
            } else if is_root {
                new_subject = Some(RdfTerm::Iri(base.clone()));
            } else {
                new_subject.clone_from(&ctx.parent_object);
            }
            if has_typeof {
                if rdfa.about.is_some() || is_root {
                    typed_resource.clone_from(&new_subject);
                } else {
                    typed_resource = Some(match self.object_from_attrs(rdfa, &base, &prefixes) {
                        Some(term) => term,
                        None => self.fresh_bnode(),
                    });
                    // `current object resource` tracks the typed resource
                    // so descendants chain off it (§7.5 step 5a).
                    current_object.clone_from(&typed_resource);
                }
            }
        } else {
            // ── §7.5 step 5b: everything else ───────────────────────
            // Presence decides the rule (`@about` → `@resource` →
            // `@href` → `@src`); an ignored/invalid value behaves like
            // an empty one (§7.4) → document base.
            if let Some(subject) = self.subject_from_attrs(rdfa, &base, &prefixes) {
                new_subject = Some(subject);
            } else if is_root {
                new_subject = Some(RdfTerm::Iri(base.clone()));
            } else if has_typeof {
                new_subject = Some(self.fresh_bnode());
            } else if let Some(parent_object) = ctx.parent_object.clone() {
                new_subject = Some(parent_object);
                if rdfa.property.is_none() {
                    // Plain wrapper element: descendants keep operating on
                    // the incoming subject context (§7.5 skip flag).
                    skip = true;
                }
            }
            if has_typeof {
                typed_resource.clone_from(&new_subject);
            }
        }

        // ── §7.5: a new subject different from the parent object starts a
        //    fresh local list mapping (this element then owns the lists its
        //    subtree creates and emits them in the final step). ──────────
        let owns_list_map = new_subject
            .as_ref()
            .is_some_and(|subject| Some(subject) != ctx.parent_object.as_ref());
        if owns_list_map {
            list_map = Rc::new(RefCell::new(HashMap::new()));
        }

        // ── Step 7: @typeof → rdf:type triples on the typed resource ────
        if let (Some(typed), Some(raw)) = (&typed_resource, rdfa.r#typeof) {
            for token in raw.split_whitespace() {
                if let Some(type_iri) = expand_term(token, &base, &prefixes, vocab.as_deref()) {
                    match Url::parse(&type_iri) {
                        Ok(url) => self.push(typed.clone(), RDF_TYPE, RdfTerm::Iri(url)),
                        Err(e) => {
                            log::warn!("RDFa: ignoring invalid @typeof value '{type_iri}': {e}");
                        }
                    }
                }
            }
        }

        // ── Step 8/9: @rel / @rev against the current object resource ───
        if let (Some(object), Some(subject)) = (current_object.clone(), new_subject.clone()) {
            if let Some(raw) = rdfa.rel {
                for token in raw.split_whitespace() {
                    if let Some(predicate) = expand_term(token, &base, &prefixes, vocab.as_deref())
                    {
                        if inlist {
                            // §7.5 step 9: @inlist collects the object into
                            // the list for this predicate instead of
                            // generating a direct triple.
                            list_map
                                .borrow_mut()
                                .entry(predicate)
                                .or_default()
                                .push(object.clone());
                        } else {
                            self.push(subject.clone(), &predicate, object.clone());
                        }
                    }
                }
            }
            if let Some(raw) = rdfa.rev {
                // @rev keeps generating reverse triples — RDFa applies
                // @inlist to @rel and @property only.
                for token in raw.split_whitespace() {
                    if let Some(predicate) = expand_term(token, &base, &prefixes, vocab.as_deref())
                    {
                        self.push(object.clone(), &predicate, subject.clone());
                    }
                }
            }
        } else if rdfa.has_relrev() {
            // Predicates with no object yet → incomplete triples (§7.5 step
            // 9); a fresh bnode becomes the parent object so the pending
            // predicates have something to connect to.
            if let Some(raw) = rdfa.rel {
                for token in raw.split_whitespace() {
                    if let Some(predicate) = expand_term(token, &base, &prefixes, vocab.as_deref())
                    {
                        if inlist {
                            // Direction *none*: instantiate the list now, a
                            // descendant's new subject is appended to it.
                            list_map.borrow_mut().entry(predicate.clone()).or_default();
                            local_incomplete.push(Incomplete {
                                predicate,
                                reverse: false,
                                inlist: Some(list_map.clone()),
                            });
                        } else {
                            local_incomplete.push(Incomplete {
                                predicate,
                                reverse: false,
                                inlist: None,
                            });
                        }
                    }
                }
            }
            if let Some(raw) = rdfa.rev {
                for token in raw.split_whitespace() {
                    if let Some(predicate) = expand_term(token, &base, &prefixes, vocab.as_deref())
                    {
                        local_incomplete.push(Incomplete {
                            predicate,
                            reverse: true,
                            inlist: None,
                        });
                    }
                }
            }
            current_object = Some(self.fresh_bnode());
        }

        // ── Step 10: @property current property value (§8.3.1) ──────────
        if let Some(property) = rdfa.property {
            let predicates: Vec<String> = property
                .split_whitespace()
                .filter_map(|token| expand_term(token, &base, &prefixes, vocab.as_deref()))
                .collect();
            if !predicates.is_empty() {
                let value = self.property_value(
                    node,
                    &base,
                    &prefixes,
                    &lang,
                    vocab.as_deref(),
                    rdfa,
                    &typed_resource,
                );
                match &new_subject {
                    Some(subject) => {
                        for predicate in &predicates {
                            if inlist {
                                list_map
                                    .borrow_mut()
                                    .entry(predicate.clone())
                                    .or_default()
                                    .push(value.clone());
                            } else {
                                self.push(subject.clone(), predicate, value.clone());
                            }
                        }
                    }
                    None => {
                        log::warn!("RDFa: @property dropped — the element has no subject");
                    }
                }
            }
        }

        // ── Step 11: complete the incoming incomplete triples (§8.2) ────
        if !skip && let Some(subject) = &new_subject {
            for incomplete in &ctx.incomplete {
                if let Some(map) = &incomplete.inlist {
                    // Direction *none* (§7.5 step 9): the new subject is
                    // added to the pending list.
                    map.borrow_mut()
                        .entry(incomplete.predicate.clone())
                        .or_default()
                        .push(subject.clone());
                } else if incomplete.reverse {
                    self.push(
                        subject.clone(),
                        &incomplete.predicate,
                        ctx.parent_subject.clone(),
                    );
                } else {
                    self.push(
                        ctx.parent_subject.clone(),
                        &incomplete.predicate,
                        subject.clone(),
                    );
                }
            }
        }

        // ── Step 12: evaluation context for the children ────────────────
        let child_ctx = if skip {
            // §7.5: copy of the *incoming* context with the local language
            // and prefix values (the local @vocab is carried too — a
            // documented, usability-motivated refinement of the spec text).
            EvalContext {
                base: base.clone(),
                parent_subject: ctx.parent_subject.clone(),
                parent_object: ctx.parent_object.clone(),
                incomplete: ctx.incomplete.clone(),
                vocab,
                prefixes,
                lang,
                list_map: list_map.clone(),
            }
        } else {
            EvalContext {
                base,
                parent_subject: new_subject
                    .clone()
                    .unwrap_or_else(|| ctx.parent_subject.clone()),
                parent_object: Some(
                    current_object
                        .clone()
                        .or(new_subject.clone())
                        .unwrap_or_else(|| ctx.parent_subject.clone()),
                ),
                incomplete: local_incomplete,
                vocab,
                prefixes,
                lang,
                list_map: list_map.clone(),
            }
        };

        // ── §7.5 final step is run by the caller (after the children):
        //    only the mapping this element instantiated gets emitted. ─────
        let list_owner = if owns_list_map && !Rc::ptr_eq(&list_map, &ctx.list_map) {
            let subject = new_subject
                .clone()
                .unwrap_or_else(|| ctx.parent_subject.clone());
            Some((list_map, subject))
        } else {
            None
        };
        Ok(ElementStep {
            child_ctx,
            list_owner,
        })
    }

    /// Subject candidates of the element (§7.5 step 5b presence order):
    /// `@about` → `@resource` → `@href` → `@src`.  `None` when none is
    /// present (the caller then applies the root / `@typeof` / parent-object
    /// fallbacks).
    fn subject_from_attrs(
        &mut self,
        attrs: RdfaAttrs<'_>,
        base: &Url,
        prefixes: &HashMap<String, String>,
    ) -> Option<RdfTerm> {
        if let Some(raw) = attrs.about {
            return Some(self.resolve_resource(raw, base, prefixes));
        }
        if let Some(raw) = attrs.resource {
            return Some(self.resolve_resource(raw, base, prefixes));
        }
        if let Some(raw) = attrs.href {
            return Some(RdfTerm::Iri(resolve_href(raw, base)));
        }
        if let Some(raw) = attrs.src {
            return Some(RdfTerm::Iri(resolve_href(raw, base)));
        }
        None
    }

    /// Object candidates of the element (§7.5 step 6 / §8.3.1 rule 5 order):
    /// `@resource` → `@href` → `@src`.  `None` when none is present.
    fn object_from_attrs(
        &mut self,
        attrs: RdfaAttrs<'_>,
        base: &Url,
        prefixes: &HashMap<String, String>,
    ) -> Option<RdfTerm> {
        if let Some(raw) = attrs.resource {
            return Some(self.resolve_resource(raw, base, prefixes));
        }
        if let Some(raw) = attrs.href {
            return Some(RdfTerm::Iri(resolve_href(raw, base)));
        }
        if let Some(raw) = attrs.src {
            return Some(RdfTerm::Iri(resolve_href(raw, base)));
        }
        None
    }

    /// Literal or IRI object for a `@property` (§8.3.1 rule order):
    /// `@datatype` → `@content` → `@resource`/`@href`/`@src` (only without
    /// `@rel`/`@rev`) → typed resource → plain literal from descendant text.
    ///
    /// `lang` is the *current* language (the element's own `@lang` already
    /// folded in), as §8.3.1 requires for the plain-literal rules.
    fn property_value(
        &mut self,
        node: &Handle,
        base: &Url,
        prefixes: &HashMap<String, String>,
        lang: &Option<String>,
        vocab: Option<&str>,
        attrs: RdfaAttrs<'_>,
        typed_resource: &Option<RdfTerm>,
    ) -> RdfTerm {
        // 1–3: @datatype (typed literal; `datatype=""` → plain literal).
        if let Some(raw) = attrs.datatype {
            let text = match attrs.content {
                Some(content) => content.to_string(),
                None => text_content(node),
            };
            return match expand_term(raw.trim(), base, prefixes, vocab) {
                // Typed literal (including rdf:XMLLiteral — the text is
                // concatenated descendant text; canonicalization is a gap).
                Some(datatype) => RdfTerm::Literal {
                    value: text,
                    datatype: Url::parse(&datatype).ok(),
                    language: None,
                    direction: None,
                },
                // `datatype=""` (or an ignored value) → plain literal with
                // the current language (§8.3.1 rule 2).
                None => RdfTerm::Literal {
                    value: text,
                    datatype: None,
                    language: lang.clone(),
                    direction: None,
                },
            };
        }
        // 4: plain literal from @content
        if let Some(content) = attrs.content {
            return RdfTerm::Literal {
                value: content.to_string(),
                datatype: None,
                language: lang.clone(),
                direction: None,
            };
        }
        // 5: IRI object from @resource / @href / @src (only when @rel/@rev
        // are absent, §8.3.1 rule 5)
        if !attrs.has_relrev()
            && let Some(resource) = self.object_from_attrs(attrs, base, prefixes)
        {
            return resource;
        }
        // 6: the typed resource (anonymous @typeof subject, no @about)
        if attrs.r#typeof.is_some()
            && attrs.about.is_none()
            && let Some(typed) = typed_resource
        {
            return typed.clone();
        }
        // 7: plain literal from the descendant text
        RdfTerm::Literal {
            value: text_content(node),
            datatype: None,
            language: lang.clone(),
            direction: None,
        }
    }

    /// `@about` / `@resource` value (`SafeCURIEorCURIEorIRI`), §7.4:
    ///
    /// * `[prefix:local]` / `[_:label]` → the bracketed CURIE is evaluated
    ///   and expanded (an IRI, or the named blank node);
    /// * any other bracketed value that does not resolve to an IRI (`[]`,
    ///   `[unknown:x]`, `[not-a-curie]`) → a **fresh blank node**, recorded
    ///   document-wide under the raw value so later occurrences (and the
    ///   element's own `@property`/`@typeof`) refer to the same node.
    ///   Bracketed values are never treated as IRIs — in particular they
    ///   never fall back to the document base.
    /// * `_:label` (unbracketed) → the named blank node (§7.4.5).
    /// * anything else → CURIE through a declared prefix, else IRI
    ///   reference against the base; an unresolvable value behaves like an
    ///   empty one (§7.4) → the document base.
    fn resolve_resource(
        &mut self,
        raw: &str,
        base: &Url,
        prefixes: &HashMap<String, String>,
    ) -> RdfTerm {
        let raw = raw.trim();
        if raw.starts_with('[') && raw.ends_with(']') && raw.len() >= 2 {
            if let Some(known) = self.safe_curies.get(raw) {
                return known.clone();
            }
            let inner = raw[1..raw.len() - 1].trim();
            let resolved = if inner.is_empty() {
                None
            } else if let Some(label) = inner.strip_prefix("_:") {
                (!label.is_empty()).then(|| self.explicit_bnode(label))
            } else {
                expand_curie(inner, base, prefixes).map(RdfTerm::Iri)
            };
            let term = resolved.unwrap_or_else(|| {
                let node = self.fresh_bnode();
                log::debug!("RDFa: safe-CURIE '{raw}' resolved to blank node {node:?}");
                node
            });
            self.safe_curies.insert(raw.to_string(), term.clone());
            return term;
        }
        // `_:label` — explicit blank node reference (§7.4.5).
        if let Some(label) = raw.strip_prefix("_:") {
            return self.explicit_bnode(label);
        }
        // A declared CURIE prefix wins over plain-IRI interpretation.
        if let Some(url) = expand_curie(raw, base, prefixes) {
            return RdfTerm::Iri(url);
        }
        if let Some(url) = resolve_iri(raw, base) {
            RdfTerm::Iri(url)
        } else {
            log::warn!("RDFa: ignoring unresolvable IRI reference '{raw}'");
            RdfTerm::Iri(base.clone())
        }
    }
}

/// Expand a CURIE (`prefix:local`) through a declared prefix mapping,
/// resolving the result against `base` (a relative namespace IRI is allowed,
/// §7.4).  `None` when the prefix is unknown, the value is not a CURIE, or
/// the expansion does not resolve.
fn expand_curie(raw: &str, base: &Url, prefixes: &HashMap<String, String>) -> Option<Url> {
    let colon = raw.find(':')?;
    let prefix = raw[..colon].to_lowercase();
    let ns = prefixes.get(&prefix)?;
    let expanded = format!("{}{}", ns, &raw[colon + 1..]);
    resolve_iri(&expanded, base)
}

// ═══════════════════════════════════════════════════════════════════════════
// Term / CURIE / IRI expansion
// ═══════════════════════════════════════════════════════════════════════════

/// Expand a `TERMorCURIEorAbsIRI` attribute value (`@property`, `@rel`,
/// `@rev`, `@typeof`, `@datatype`).
///
/// Order (§7.4.4 + task rule: local `@prefix`/`xmlns` > `@vocab`):
/// 1. `_:label` → ignored (a blank node cannot be a predicate/datatype);
/// 2. declared CURIE prefix → namespace + local part, resolved against base;
/// 3. absolute IRI → used verbatim;
/// 4. bare term → `@vocab` + term, else the initial-context term map
///    (case-sensitive, then case-insensitive — §7.4.3), else ignored;
/// 5. otherwise → `None` (the value is ignored, per §7.4).
fn expand_term(
    token: &str,
    base: &Url,
    prefixes: &HashMap<String, String>,
    vocab: Option<&str>,
) -> Option<String> {
    let token = token.trim();
    if token.is_empty() || token.starts_with("_:") {
        return None;
    }
    if let Some(colon) = token.find(':') {
        let prefix = token[..colon].to_lowercase();
        if let Some(ns) = prefixes.get(&prefix) {
            let expanded = format!("{}{}", ns, &token[colon + 1..]);
            return resolve_iri(&expanded, base).map(|u| u.to_string());
        }
        // Unknown prefix: usable only if it is already an absolute IRI.
        if let Ok(url) = Url::parse(token) {
            return Some(url.to_string());
        }
        return None;
    }
    if is_rdfa_term(token) {
        let expanded = expand_term_through_map(token, vocab)?;
        return Some(expanded);
    }
    None
}

/// `RDFa` term syntax (§7.4.3): `NCNameStartChar termChar*` with `termChar =
/// (NameChar - ':') | '/'`.  Approximated with the ASCII subset.
fn is_rdfa_term(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
}

/// Resolve an IRI reference against `base` (RFC 3986): absolute references
/// pass through, relative ones are joined onto the base.
fn resolve_iri(raw: &str, base: &Url) -> Option<Url> {
    let raw = raw.trim();
    if let Ok(url) = Url::parse(raw) {
        return Some(url);
    }
    base.join(raw).ok()
}

/// `@href` / `@src` resolution: an unresolvable reference is ignored and
/// behaves like an empty one (§7.4) → the document base, with a warning.
fn resolve_href(raw: &str, base: &Url) -> Url {
    resolve_iri(raw, base).unwrap_or_else(|| {
        log::warn!("RDFa: ignoring unresolvable IRI reference '{raw}'");
        base.clone()
    })
}

/// Parse an `@prefix="ex: http://example.org/ dc: http://purl.org/dc/"`
/// declaration into the local prefix map.  Prefix names are lower-cased
/// (§7.5) and namespace IRIs are stored verbatim — a relative namespace is
/// resolved against the base only when the expanded CURIE is used.
fn parse_prefix_declaration(
    value: &str,
    prefixes: &mut HashMap<String, String>,
    declared: &mut HashMap<String, String>,
) {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        let Some(colon) = token.find(':') else {
            i += 1;
            continue;
        };
        let name = &token[..colon];
        let rest = &token[colon + 1..];
        // A token whose "rest" opens like an absolute IRI (`http://…`) is a
        // stray IRI, not a `name: IRI` pair.
        if rest.starts_with("//") {
            i += 1;
            continue;
        }
        if !name.is_empty()
            && !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            i += 1;
            continue;
        }
        if rest.is_empty() {
            // `ex: http://…` — the namespace is the next whitespace token.
            if i + 1 < tokens.len() {
                let ns = tokens[i + 1];
                prefixes.insert(name.to_lowercase(), ns.to_string());
                declared.insert(name.to_lowercase(), ns.to_string());
                i += 2;
            } else {
                i += 1;
            }
        } else {
            // `ex:http://…` — namespace in the same token.
            prefixes.insert(name.to_lowercase(), rest.to_string());
            declared.insert(name.to_lowercase(), rest.to_string());
            i += 1;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RDFa Vocabulary Expansion (§10) — opt-in, local
// ═══════════════════════════════════════════════════════════════════════════

/// Apply `RDFa` Vocabulary Expansion (§10) to the output graph in place.
///
/// This is the §10.1.1 *`RDFa` Vocabulary Entailment* — the six OWL 2 RL
/// rules `prp-spo1`, `prp-eqp1`, `prp-eqp2`, `cax-sco`, `cax-eqc1`,
/// `cax-eqc2` — evaluated to a fixpoint over the triples the document
/// produced.  Two deliberate restrictions keep it network-free and
/// deterministic:
///
/// * the *vocabulary graph* of §10.1 step 1 is **not** populated by
///   dereferencing `rdfa:usesVocabulary` objects; only vocabulary facts
///   already present in the output graph (typically declared inline in the
///   document) take part in the derivation;
/// * triples that exist only in the vocabulary graph are never added to the
///   output (§10.1 step 2 already forbids this).
///
/// Per §10.2 this runs only when the caller opted in
/// ([`RdfaParser::with_vocabulary_expansion`]).
fn expand_vocabulary(triples: &mut Vec<Triple>) {
    // Schema facts of the entailment (sub/eq property, sub/eq class) taken
    // from the output graph itself.  `equivalent*` is stored in both
    // directions because OWL equivalence is symmetric (prp-eqp2 / cax-eqc2).
    let mut sub_property: Vec<(String, String)> = Vec::new();
    let mut equivalent_property: Vec<(String, String)> = Vec::new();
    let mut sub_class: Vec<(String, String)> = Vec::new();
    let mut equivalent_class: Vec<(String, String)> = Vec::new();

    for t in triples.iter() {
        let (RdfTerm::Iri(subject), RdfTerm::Iri(predicate), RdfTerm::Iri(object)) =
            (&t.subject, &t.predicate, &t.object)
        else {
            continue;
        };
        let subject = subject.as_str().to_string();
        let object = object.as_str().to_string();
        match predicate.as_str() {
            RDFS_SUBPROPERTY_OF => sub_property.push((subject, object)),
            OWL_EQUIVALENT_PROPERTY => {
                equivalent_property.push((subject.clone(), object.clone()));
                equivalent_property.push((object, subject));
            }
            rdf_to_owl::RDFS_SUBCLASS_OF => sub_class.push((subject, object)),
            rdf_to_owl::OWL_EQUIVALENT_CLASS => {
                equivalent_class.push((subject.clone(), object.clone()));
                equivalent_class.push((object, subject));
            }
            _ => {}
        }
    }

    if sub_property.is_empty()
        && equivalent_property.is_empty()
        && sub_class.is_empty()
        && equivalent_class.is_empty()
    {
        return;
    }

    let absolute = |iri: &str| Url::parse(iri).ok();
    let mut seen: HashSet<Triple> = triples.iter().cloned().collect();
    loop {
        let mut added: Vec<Triple> = Vec::new();
        for t in triples.iter() {
            let RdfTerm::Iri(predicate) = &t.predicate else {
                continue;
            };
            if predicate.as_str() == RDF_TYPE {
                // cax-sco / cax-eqc: `x rdf:type C` with `C ⊑ D` / `C ≡ D`.
                if let RdfTerm::Iri(class) = &t.object {
                    let class = class.as_str();
                    for (_, sup) in sub_class
                        .iter()
                        .chain(equivalent_class.iter())
                        .filter(|(sub, _)| sub.as_str() == class)
                        .filter_map(|(_, sup)| absolute(sup).map(|url| (sup, url)))
                    {
                        added.push(Triple {
                            subject: t.subject.clone(),
                            predicate: t.predicate.clone(),
                            object: RdfTerm::Iri(sup),
                        });
                    }
                }
                continue;
            }
            // prp-spo1 / prp-eqp1: `s p o` with `p ⊑ q` / `p ≡ q`.
            let predicate = predicate.as_str();
            for (_, sup) in sub_property
                .iter()
                .chain(equivalent_property.iter())
                .filter(|(sub, _)| sub.as_str() == predicate)
                .filter_map(|(_, sup)| absolute(sup).map(|url| (sup, url)))
            {
                added.push(Triple {
                    subject: t.subject.clone(),
                    predicate: RdfTerm::Iri(sup),
                    object: t.object.clone(),
                });
            }
        }
        let mut grew = false;
        for triple in added {
            if seen.insert(triple.clone()) {
                grew = true;
                triples.push(triple);
            }
        }
        if !grew {
            break;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DOM helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Full attribute name (`prefix:local` when a prefix survived parsing).
fn attribute_full_name(name: &markup5ever::QualName) -> String {
    match &name.prefix {
        Some(prefix) => format!("{}:{}", prefix, name.local),
        None => name.local.to_string(),
    }
}

/// Concatenate all descendant text nodes (§8.3.1), whitespace preserved.
fn text_content(node: &Handle) -> String {
    fn walk(node: &Handle, out: &mut String, depth: usize) {
        // CR-011 B1: same `MAX_DEPTH` discipline as `Evaluator::evaluate` —
        // a node at the evaluation cap has no descendants in the evaluated
        // document (deeper subtrees are rejected or skipped), and this cap
        // keeps the walk stack-safe even if it is ever reached.
        if depth > MAX_DEPTH {
            log::warn!("RDFa: text-content depth cap ({MAX_DEPTH}) reached; truncating");
            return;
        }
        match &node.data {
            NodeData::Text { contents } => out.push_str(&contents.borrow()),
            NodeData::Element { .. } => {
                let children = node.children.borrow().clone();
                for child in &children {
                    walk(child, out, depth + 1);
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    walk(node, &mut out, 0);
    out
}

/// Maximum *element* nesting depth of a parsed document, computed with an
/// explicit work-list stack — never recursion, so it is safe on documents
/// of any depth.  The depth accounting matches `Evaluator::evaluate`
/// (the document's element child is level 0).
///
/// Used as the precise post-parse guard in `RdfaParser::parse_with_base`
/// (CR-011 B1/M4): a document deeper than [`MAX_DEPTH`] is rejected with an
/// `Err` *before* any recursive walk touches it.
fn dom_nesting_depth(root: &Handle) -> usize {
    let mut stack: Vec<(Handle, usize)> = Vec::new();
    stack.push((root.clone(), 0));
    let mut max_depth = 0usize;
    while let Some((node, depth)) = stack.pop() {
        let is_element = matches!(&node.data, NodeData::Element { .. });
        if is_element {
            max_depth = max_depth.max(depth);
        }
        // Only elements increment the depth for their children (mirrors
        // `Evaluator::evaluate` and `DocumentScan::walk`).
        let child_depth = if is_element { depth + 1 } else { depth };
        for child in node.children.borrow().iter() {
            stack.push((child.clone(), child_depth));
        }
        if let NodeData::Element {
            template_contents, ..
        } = &node.data
            && let Some(template) = template_contents.borrow().as_ref()
        {
            for child in template.children.borrow().iter() {
                stack.push((child.clone(), child_depth));
            }
        }
    }
    max_depth
}

/// First position of `needle` in `haystack`, comparing ASCII bytes
/// case-insensitively.  Used by [`estimate_source_depth`] for raw-text
/// close-tag searches without allocating a lowercased copy of the document.
fn find_ascii_ci(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    'outer: for start in 0..=(haystack.len() - needle.len()) {
        for (offset, &n) in needle.iter().enumerate() {
            if !haystack[start + offset].eq_ignore_ascii_case(&n) {
                continue 'outer;
            }
        }
        return Some(start);
    }
    None
}

/// HTML5 void elements (never have children and never increase nesting).
const HTML_VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Raw-text elements whose content is not markup (their inner `<…>` strings
/// must not count towards the nesting estimate).
const HTML_RAW_TEXT_ELEMENTS: [&str; 7] = [
    "script", "style", "textarea", "title", "xmp", "noembed", "noframes",
];

/// Cheap, stack-free estimate of the maximum element nesting depth of an
/// HTML source string (CR-011 M4 pre-parse guard).
///
/// Runs in O(input) time with no allocation beyond the rare raw-text
/// search.  It is *conservative by design*: html5ever's implicit close
/// rules (`<p>`, `<li>`, table cells …) can make the real DOM shallower
/// than the source, so the estimate may over-count — which is why
/// [`MAX_ESTIMATED_DEPTH`] sits far above the enforced [`MAX_DEPTH`] and
/// why the parsed DOM gets its own precise check.
fn estimate_source_depth(content: &str) -> usize {
    let bytes = content.as_bytes();
    let mut depth: usize = 0;
    let mut max_depth: usize = 0;
    let mut cursor: usize = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }
        // `<!-- comment -->` — skip wholesale.
        if content[cursor..].starts_with("<!--") {
            cursor = match content[cursor + 4..].find("-->") {
                Some(end) => cursor + 4 + end + 3,
                None => bytes.len(),
            };
            continue;
        }
        // `<!doctype …>`, `<?xml …?>`, CDATA and other `<!…>`/`<?…>` forms.
        if cursor + 1 < bytes.len() && (bytes[cursor + 1] == b'!' || bytes[cursor + 1] == b'?') {
            cursor = match content[cursor..].find('>') {
                Some(end) => cursor + end + 1,
                None => bytes.len(),
            };
            continue;
        }
        // Closing tag.
        if cursor + 1 < bytes.len() && bytes[cursor + 1] == b'/' {
            depth = depth.saturating_sub(1);
            cursor = match content[cursor..].find('>') {
                Some(end) => cursor + end + 1,
                None => bytes.len(),
            };
            continue;
        }
        // Start tag must begin with an ASCII alpha (anything else — e.g.
        // `a < byte` in text — is not a tag).
        if cursor + 1 >= bytes.len() || !bytes[cursor + 1].is_ascii_alphabetic() {
            cursor += 1;
            continue;
        }
        let name_start = cursor + 1;
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'-')
        {
            name_end += 1;
        }
        let name = content[name_start..name_end].to_ascii_lowercase();
        // Scan to the tag's `>`, honouring quoted attribute values that may
        // contain `>`.
        let mut scan = name_end;
        let mut quote: Option<u8> = None;
        let mut self_closing = false;
        let mut terminated = false;
        while scan < bytes.len() {
            let byte = bytes[scan];
            match quote {
                Some(quote_char) => {
                    if byte == quote_char {
                        quote = None;
                    }
                }
                None => {
                    if byte == b'"' || byte == b'\'' {
                        quote = Some(byte);
                    } else if byte == b'>' {
                        self_closing = scan > name_start && bytes[scan - 1] == b'/';
                        terminated = true;
                        break;
                    }
                }
            }
            scan += 1;
        }
        if !terminated {
            break; // unterminated tag — nothing left to count
        }
        if !HTML_VOID_ELEMENTS.contains(&name.as_str()) && !self_closing {
            depth += 1;
            max_depth = max_depth.max(depth);
            // Raw-text elements swallow their content verbatim: jump to the
            // matching close tag so tag-like strings inside JS/CSS don't
            // inflate the estimate.
            if HTML_RAW_TEXT_ELEMENTS.contains(&name.as_str()) {
                match find_ascii_ci(
                    &content.as_bytes()[scan + 1..],
                    format!("</{name}").as_bytes(),
                ) {
                    Some(p) => cursor = scan + 1 + p,
                    // Unterminated raw-text element: the rest of the
                    // document is its text content — nothing left to scan.
                    None => break,
                }
                continue;
            }
        }
        cursor = scan + 1;
    }
    max_depth
}

/// Make a label safe for an RDF blank node identifier.
fn sanitize_bnode_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("anon");
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════════
// Preservation of triples the shared mapping cannot convert
// ═══════════════════════════════════════════════════════════════════════════

/// Re-emit every triple the shared mapping does not consume as an
/// ontology-level [`UNMAPPED_TRIPLE_PROPERTY`] annotation (N-Triples text).
/// The classifier mirrors `map_rdf_triples`'s drop decisions, biased
/// towards *preserving* whenever consumption cannot be proven.
fn preserve_unmapped(triples: &[Triple], ontology: &mut Ontology) {
    let index = GraphIndex::build(triples);
    for triple in triples {
        if index.is_mapped(triple) {
            continue;
        }
        ontology.annotations.push(Annotation {
            property: AnnotationProperty {
                iri: IRI::new(UNMAPPED_TRIPLE_PROPERTY),
            },
            value: AnnotationValue::Literal(Literal {
                value: ntriples_syntax(triple),
                datatype: None,
                language: None,
            }),
            annotations: vec![],
        });
    }
}

/// Blank-node indexes needed to decide what `map_rdf_triples` consumes.
struct GraphIndex {
    /// blank id → its outgoing `(predicate IRI, object)` pairs.
    out: HashMap<String, Vec<(String, RdfTerm)>>,
    /// Blank nodes sitting in a class-expression position (object of
    /// `rdf:type` / `rdfs:subClassOf` / `owl:equivalentClass` /
    /// `owl:disjointWith` / restriction filler …, or subject of a
    /// class-expression predicate), i.e. nodes the mapper will attempt to
    /// resolve.
    claimed: HashSet<String>,
    /// Blank nodes that are cells of a *successfully resolved* RDF list
    /// rooted at a list-valued OWL predicate.
    cells: HashSet<String>,
    /// IRIs declared as `owl:DatatypeProperty` (mirrors the mapper).
    data_properties: HashSet<String>,
}

impl GraphIndex {
    fn build(triples: &[Triple]) -> Self {
        let mut out: HashMap<String, Vec<(String, RdfTerm)>> = HashMap::new();
        let mut claimed: HashSet<String> = HashSet::new();
        let mut data_properties: HashSet<String> = HashSet::new();

        for triple in triples {
            let RdfTerm::Iri(predicate) = &triple.predicate else {
                continue;
            };
            let predicate = predicate.as_str();
            if let RdfTerm::BlankNode(id) = &triple.subject {
                out.entry(id.clone())
                    .or_default()
                    .push((predicate.to_string(), triple.object.clone()));
            }
            if predicate == rdf_to_owl::RDF_TYPE {
                if let (RdfTerm::Iri(subject), RdfTerm::Iri(object)) =
                    (&triple.subject, &triple.object)
                    && object.as_str() == rdf_to_owl::OWL_DATATYPE_PROPERTY
                {
                    data_properties.insert(subject.as_str().to_string());
                }
                if let RdfTerm::BlankNode(id) = &triple.object {
                    claimed.insert(id.clone());
                }
            }
        }

        // Expression positions that trigger `resolve_class_expression` /
        // `resolve_data_range` on a blank node.
        for triple in triples {
            let RdfTerm::Iri(predicate) = &triple.predicate else {
                continue;
            };
            if matches!(
                predicate.as_str(),
                rdf_to_owl::RDFS_SUBCLASS_OF
                    | rdf_to_owl::OWL_EQUIVALENT_CLASS
                    | rdf_to_owl::OWL_DISJOINT_WITH
                    | rdf_to_owl::OWL_SOME_VALUES_FROM
                    | rdf_to_owl::OWL_ALL_VALUES_FROM
                    | rdf_to_owl::OWL_COMPLEMENT_OF
                    | rdf_to_owl::OWL_ON_CLASS
                    | rdf_to_owl::OWL_ON_DATA_RANGE
                    | rdf_to_owl::OWL_ON_DATATYPE
            ) {
                for side in [&triple.subject, &triple.object] {
                    if let RdfTerm::BlankNode(id) = side {
                        claimed.insert(id.clone());
                    }
                }
            }
        }

        // Cells (and blank items) of lists rooted at list-valued predicates
        // the mapper resolves.
        let list_heads = [
            rdf_to_owl::OWL_INTERSECTION_OF,
            rdf_to_owl::OWL_UNION_OF,
            rdf_to_owl::OWL_ONE_OF,
            rdf_to_owl::OWL_DISJOINT_UNION_OF,
            rdf_to_owl::OWL_PROPERTY_CHAIN_AXIOM,
            rdf_to_owl::OWL_HAS_KEY,
        ];
        let mut cells: HashSet<String> = HashSet::new();
        for triple in triples {
            let RdfTerm::Iri(predicate) = &triple.predicate else {
                continue;
            };
            if !list_heads.contains(&predicate.as_str()) {
                continue;
            }
            let RdfTerm::BlankNode(head) = &triple.object else {
                continue;
            };
            let mut current = RdfTerm::BlankNode(head.clone());
            let mut visited: HashSet<String> = HashSet::new();
            let mut collected: Vec<String> = Vec::new();
            let mut items: Vec<RdfTerm> = Vec::new();
            let resolved = loop {
                match &current {
                    RdfTerm::Iri(url) => break url.as_str() == rdf_to_owl::RDF_NIL,
                    RdfTerm::BlankNode(id) => {
                        if !visited.insert(id.clone()) {
                            break false;
                        }
                        collected.push(id.clone());
                        let Some(first) = find_predicate(&out, id, rdf_to_owl::RDF_FIRST) else {
                            break false;
                        };
                        items.push(first.clone());
                        match find_predicate(&out, id, rdf_to_owl::RDF_REST) {
                            Some(rest) => current = rest.clone(),
                            None => break false,
                        }
                    }
                    _ => break false,
                }
            };
            if resolved {
                cells.extend(collected);
                for item in items {
                    if let RdfTerm::BlankNode(id) = item {
                        claimed.insert(id);
                    }
                }
            }
        }

        Self {
            out,
            claimed,
            cells,
            data_properties,
        }
    }

    fn find<'a>(&'a self, id: &str, predicate: &str) -> Option<&'a RdfTerm> {
        find_predicate(&self.out, id, predicate)
    }

    /// Mirror of `rdf_to_owl::resolve_list`: the chain must reach `rdf:nil`.
    fn list_resolves(&self, head: &RdfTerm) -> bool {
        let mut current = head.clone();
        let mut visited: HashSet<String> = HashSet::new();
        loop {
            match &current {
                RdfTerm::Iri(url) => return url.as_str() == rdf_to_owl::RDF_NIL,
                RdfTerm::BlankNode(id) => {
                    if !visited.insert(id.clone()) {
                        return false;
                    }
                    if self.find(id, rdf_to_owl::RDF_FIRST).is_none() {
                        return false;
                    }
                    match self.find(id, rdf_to_owl::RDF_REST) {
                        Some(rest) => current = rest.clone(),
                        None => return false,
                    }
                }
                _ => return false,
            }
        }
    }

    fn term_resolvable(&self, term: &RdfTerm) -> bool {
        match term {
            RdfTerm::Iri(_) => true,
            RdfTerm::BlankNode(id) => self.expr_resolvable(id),
            _ => false,
        }
    }

    /// Mirror of `rdf_to_owl::resolve_class_expression` for blank nodes.
    fn expr_resolvable(&self, id: &str) -> bool {
        let entries = match self.out.get(id) {
            Some(entries) if !entries.is_empty() => entries,
            _ => return false,
        };
        let has = |p: &str| entries.iter().any(|(q, _)| q == p);
        if has(rdf_to_owl::OWL_ON_PROPERTY) {
            return self.restriction_resolvable(id);
        }
        if has(rdf_to_owl::OWL_INTERSECTION_OF) {
            return self
                .find(id, rdf_to_owl::OWL_INTERSECTION_OF)
                .is_some_and(|head| self.list_resolves(head));
        }
        if has(rdf_to_owl::OWL_UNION_OF) {
            return self
                .find(id, rdf_to_owl::OWL_UNION_OF)
                .is_some_and(|head| self.list_resolves(head));
        }
        if let Some(inner) = self.find(id, rdf_to_owl::OWL_COMPLEMENT_OF) {
            return self.term_resolvable(inner);
        }
        if has(rdf_to_owl::OWL_ONE_OF) {
            return self
                .find(id, rdf_to_owl::OWL_ONE_OF)
                .is_some_and(|head| self.list_resolves(head));
        }
        false
    }

    /// Mirror of `rdf_to_owl::resolve_restriction`'s success conditions.
    fn restriction_resolvable(&self, id: &str) -> bool {
        let Some(property) = self.find(id, rdf_to_owl::OWL_ON_PROPERTY) else {
            return false;
        };
        let RdfTerm::Iri(property) = property else {
            return false;
        };
        let property = property.as_str();
        let has_data_range = self.find(id, rdf_to_owl::OWL_ON_DATA_RANGE).is_some();
        let is_data_prop = self.data_properties.contains(property) || has_data_range;

        for predicate in [
            rdf_to_owl::OWL_SOME_VALUES_FROM,
            rdf_to_owl::OWL_ALL_VALUES_FROM,
        ] {
            if let Some(filler) = self.find(id, predicate) {
                // Mirror of `filler_is_data`: blank fillers are always class
                // expressions; literal fillers are data (and `resolve_data_range`
                // rejects them); IRI fillers depend on the datatype check.
                let filler_is_data = has_data_range
                    || self.data_properties.contains(property)
                    || match filler {
                        RdfTerm::Literal { .. } => true,
                        RdfTerm::Iri(url) => is_datatype_iri(url.as_str()),
                        _ => false,
                    };
                return match filler {
                    RdfTerm::Iri(_) => true, // named class or datatype IRI
                    RdfTerm::BlankNode(filler_id) => {
                        if filler_is_data {
                            self.data_range_resolvable(filler_id)
                        } else {
                            self.expr_resolvable(filler_id)
                        }
                    }
                    // `resolve_data_range(Literal)` is `None` upstream.
                    _ => false,
                };
            }
        }
        if let Some(value) = self.find(id, rdf_to_owl::OWL_HAS_VALUE) {
            return match value {
                RdfTerm::Literal { .. } => true,
                RdfTerm::BlankNode(_) => !is_data_prop,
                RdfTerm::Iri(_) => !is_data_prop,
                _ => false,
            };
        }
        if let Some(flag) = self.find(id, rdf_to_owl::OWL_HAS_SELF)
            && matches!(flag, RdfTerm::Literal { value, .. } if value == "true")
        {
            return true;
        }
        let pairs = [
            (
                rdf_to_owl::OWL_MIN_QUALIFIED_CARDINALITY,
                rdf_to_owl::OWL_MIN_CARDINALITY,
            ),
            (
                rdf_to_owl::OWL_MAX_QUALIFIED_CARDINALITY,
                rdf_to_owl::OWL_MAX_CARDINALITY,
            ),
            (
                rdf_to_owl::OWL_QUALIFIED_CARDINALITY,
                rdf_to_owl::OWL_CARDINALITY,
            ),
        ];
        for (qualified, unqualified) in pairs {
            let term = match self.find(id, qualified) {
                Some(term) => term,
                None => match self.find(id, unqualified) {
                    Some(term) => term,
                    None => continue,
                },
            };
            let RdfTerm::Literal { value, .. } = term else {
                return false;
            };
            if value.parse::<u32>().is_err() {
                return false;
            }
            if is_data_prop {
                return match self.find(id, rdf_to_owl::OWL_ON_DATA_RANGE) {
                    Some(range) => self.term_resolvable(range),
                    None => true,
                };
            }
            return match self.find(id, rdf_to_owl::OWL_ON_CLASS) {
                Some(class) => self.term_resolvable(class),
                None => true,
            };
        }
        false
    }

    /// Mirror of `rdf_to_owl::resolve_data_range` for blank nodes.
    fn data_range_resolvable(&self, id: &str) -> bool {
        let entries = match self.out.get(id) {
            Some(entries) if !entries.is_empty() => entries,
            _ => return false,
        };
        let has = |p: &str| entries.iter().any(|(q, _)| q == p);
        if has(rdf_to_owl::OWL_ON_DATATYPE) {
            let base_ok = matches!(
                self.find(id, rdf_to_owl::OWL_ON_DATATYPE),
                Some(RdfTerm::Iri(_))
            );
            let facets_ok = self
                .find(id, rdf_to_owl::OWL_WITH_RESTRICTIONS)
                .is_some_and(|head| {
                    self.list_resolves(head) && list_items_all_literals(&self.out, head)
                });
            return base_ok && facets_ok;
        }
        if has(rdf_to_owl::OWL_INTERSECTION_OF) {
            return self
                .find(id, rdf_to_owl::OWL_INTERSECTION_OF)
                .is_some_and(|head| self.list_resolves(head));
        }
        if has(rdf_to_owl::OWL_UNION_OF) {
            return self
                .find(id, rdf_to_owl::OWL_UNION_OF)
                .is_some_and(|head| self.list_resolves(head));
        }
        if let Some(inner) = self.find(id, rdf_to_owl::OWL_COMPLEMENT_OF) {
            return match inner {
                RdfTerm::Iri(_) => true,
                RdfTerm::BlankNode(inner_id) => self.data_range_resolvable(inner_id),
                _ => false,
            };
        }
        if has(rdf_to_owl::OWL_ONE_OF) {
            return self.find(id, rdf_to_owl::OWL_ONE_OF).is_some_and(|head| {
                self.list_resolves(head) && list_has_literal_item(&self.out, head)
            });
        }
        false
    }

    /// Does `map_rdf_triples` consume this triple?  `false` → the triple is
    /// preserved as an annotation instead of being dropped.
    fn is_mapped(&self, triple: &Triple) -> bool {
        let RdfTerm::Iri(predicate) = &triple.predicate else {
            return false; // non-IRI predicates are invalid RDF (unreachable)
        };
        let predicate = predicate.as_str();
        let subject_id = match &triple.subject {
            RdfTerm::BlankNode(id) => Some(id.as_str()),
            _ => None,
        };
        let object_id = match &triple.object {
            RdfTerm::BlankNode(id) => Some(id.as_str()),
            _ => None,
        };

        if predicate == OWL_IMPORTS {
            return false; // import handling is a documented no-op
        }

        match predicate {
            rdf_to_owl::RDF_TYPE => match &triple.object {
                RdfTerm::Iri(object) => match subject_id {
                    // Named subject → declaration / ontology / class assertion.
                    None => true,
                    // Blank subject: a custom class becomes an anonymous class
                    // assertion; well-known vocabulary is a structural marker
                    // only if the node is claimed as an expression.
                    Some(id) => !is_wellknown_vocab(object.as_str()) || self.claimed.contains(id),
                },
                // Blank object: consumed only when the subject is a named
                // individual and the expression resolves.
                RdfTerm::BlankNode(object) => subject_id.is_none() && self.expr_resolvable(object),
                _ => false,
            },
            rdf_to_owl::RDFS_SUBCLASS_OF
            | rdf_to_owl::OWL_EQUIVALENT_CLASS
            | rdf_to_owl::OWL_DISJOINT_WITH => {
                let subject_ok = subject_id.is_none_or(|id| self.expr_resolvable(id));
                let object_ok = object_id.is_none_or(|id| self.expr_resolvable(id));
                subject_ok && object_ok
            }
            _ if rdf_to_owl::is_structural_owl_predicate(predicate) => {
                self.structural_consumed(triple, predicate, subject_id)
            }
            // Everything else (assertions, annotations, rdfs:domain/range,
            // owl:sameAs, …) flows through `process_term_triple`, which maps
            // literal / IRI / blank objects for IRI and blank subjects.
            _ => true,
        }
    }

    fn structural_consumed(
        &self,
        triple: &Triple,
        predicate: &str,
        subject_id: Option<&str>,
    ) -> bool {
        let object_list_resolves = match &triple.object {
            RdfTerm::BlankNode(head) => self.list_resolves(&RdfTerm::BlankNode(head.clone())),
            _ => false,
        };
        match predicate {
            rdf_to_owl::RDF_FIRST | rdf_to_owl::RDF_REST | rdf_to_owl::RDF_NIL => {
                subject_id.is_some_and(|id| self.cells.contains(id))
            }
            rdf_to_owl::OWL_INTERSECTION_OF | rdf_to_owl::OWL_UNION_OF | rdf_to_owl::OWL_ONE_OF => {
                // Resolved only as part of a claimed blank-node expression.
                subject_id.is_some_and(|id| self.claimed.contains(id)) && object_list_resolves
            }
            rdf_to_owl::OWL_DISJOINT_UNION_OF | rdf_to_owl::OWL_PROPERTY_CHAIN_AXIOM => {
                // The mapper's arms require a *named* subject.
                subject_id.is_none() && object_list_resolves
            }
            rdf_to_owl::OWL_HAS_KEY => {
                let subject_ok = match subject_id {
                    None => true,
                    Some(id) => self.claimed.contains(id) && self.expr_resolvable(id),
                };
                subject_ok && object_list_resolves
            }
            rdf_to_owl::OWL_COMPLEMENT_OF => {
                subject_id.is_some_and(|id| self.claimed.contains(id))
                    && self.term_resolvable(&triple.object)
            }
            rdf_to_owl::OWL_WITH_RESTRICTIONS | rdf_to_owl::OWL_ON_DATATYPE => {
                // Data-range internals: only consumed when the enclosing
                // blank data range actually resolves.
                subject_id.is_some_and(|id| self.data_range_resolvable(id))
            }
            OWL_DISTINCT_MEMBERS | OWL_MEMBERS => false,
            _ => {
                // Restriction internals (`owl:onProperty`,
                // `owl:someValuesFrom`, cardinalities, …): consumed only
                // when the enclosing restriction resolves.
                match subject_id {
                    Some(id) => self.claimed.contains(id) && self.restriction_resolvable(id),
                    None => false,
                }
            }
        }
    }
}

fn find_predicate<'a>(
    out: &'a HashMap<String, Vec<(String, RdfTerm)>>,
    id: &str,
    predicate: &str,
) -> Option<&'a RdfTerm> {
    out.get(id)?
        .iter()
        .find(|(p, _)| p == predicate)
        .map(|(_, object)| object)
}

/// Every `rdf:first` object of the list rooted at `head` is a literal
/// (facet lists of `owl:withRestrictions`).
fn list_items_all_literals(out: &HashMap<String, Vec<(String, RdfTerm)>>, head: &RdfTerm) -> bool {
    let mut current = head.clone();
    let mut visited: HashSet<String> = HashSet::new();
    loop {
        match &current {
            RdfTerm::Iri(url) => return url.as_str() == rdf_to_owl::RDF_NIL,
            RdfTerm::BlankNode(id) => {
                if !visited.insert(id.clone()) {
                    return false;
                }
                match find_predicate(out, id, rdf_to_owl::RDF_FIRST) {
                    Some(RdfTerm::Literal { .. }) => {}
                    _ => return false,
                }
                match find_predicate(out, id, rdf_to_owl::RDF_REST) {
                    Some(rest) => current = rest.clone(),
                    None => return false,
                }
            }
            _ => return false,
        }
    }
}

/// Does the list rooted at `head` contain at least one literal item?
fn list_has_literal_item(out: &HashMap<String, Vec<(String, RdfTerm)>>, head: &RdfTerm) -> bool {
    let mut current = head.clone();
    let mut visited: HashSet<String> = HashSet::new();
    loop {
        match &current {
            RdfTerm::Iri(url) => return url.as_str() == rdf_to_owl::RDF_NIL,
            RdfTerm::BlankNode(id) => {
                if !visited.insert(id.clone()) {
                    return false;
                }
                if matches!(
                    find_predicate(out, id, rdf_to_owl::RDF_FIRST),
                    Some(RdfTerm::Literal { .. })
                ) {
                    return true;
                }
                match find_predicate(out, id, rdf_to_owl::RDF_REST) {
                    Some(rest) => current = rest.clone(),
                    None => return false,
                }
            }
            _ => return false,
        }
    }
}

/// Well-known RDF/RDFS/OWL vocabulary (mirrors the shared mapper's private
/// helper): `rdf:type` objects under these namespaces are structural markers
/// rather than class assertions when the subject is a blank node.
fn is_wellknown_vocab(iri: &str) -> bool {
    iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
        || iri.starts_with("http://www.w3.org/2000/01/rdf-schema#")
        || iri.starts_with("http://www.w3.org/2002/07/owl#")
}

/// XSD datatypes (mirrors the shared mapper's `is_datatype_iri`).
fn is_datatype_iri(iri: &str) -> bool {
    iri.starts_with("http://www.w3.org/2001/XMLSchema#")
        || iri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#Literal"
        || iri == "http://www.w3.org/2000/01/rdf-schema#Literal"
}

/// N-Triples rendering of a triple (used for preservation annotations).
fn ntriples_syntax(triple: &Triple) -> String {
    format!(
        "{} {} {} .",
        ntriples_term(&triple.subject),
        ntriples_term(&triple.predicate),
        ntriples_term(&triple.object)
    )
}

fn ntriples_term(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(url) => format!("<{url}>"),
        RdfTerm::BlankNode(id) => format!("_:{id}"),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let mut out = format!("\"{}\"", escape_ntriples(value));
            if let Some(language) = language {
                out.push_str(&format!("@{language}"));
            } else if let Some(datatype) = datatype {
                out.push_str(&format!("^^<{datatype}>"));
            }
            out
        }
        other => format!("{other:?}"),
    }
}

fn escape_ntriples(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════════
// Serializer — HTML5 + RDFa writer
// ═══════════════════════════════════════════════════════════════════════════

/// All triples of one subject, ready to be rendered as a single `RDFa` block.
#[derive(Debug)]
struct SubjectBlock {
    subject: RdfTerm,
    /// `rdf:type` objects that are IRIs — emitted as `@typeof`.
    types: Vec<String>,
    /// `(predicate IRI, object)` — emitted as `@rel`/`@property` children.
    children: Vec<(String, RdfTerm)>,
    /// `(predicate IRI, item)` — an RDF collection emitted with `@inlist`.
    lists: Vec<(String, RdfTerm)>,
}

/// `RDFa` 1.1 (HTML5) writer.
///
/// The ontology is converted with the shared canonical emission
/// ([`ontology_to_triples`]) and every triple is rendered as `RDFa` markup:
/// `@about`/`@typeof` on the subject block, `@rel`+`@href` for resource
/// objects, `@property` (+`@content`, `@datatype`, `@lang`) for literals,
/// `@inlist` for RDF collections, plus an `@prefix` block and an empty
/// `@vocab` on the root element.  `RdfaParser` re-parses that markup back
/// into the same triples, hence the same axioms (see
/// `tests/rdfa_tests.rs::serializer_round_trip_reproduces_core_axioms`).
///
/// Constructs without an `RDFa` spelling are written as HTML comments instead
/// of being dropped: `owl:imports` declarations, literals carrying an RDF
/// 1.2 base direction, and blank-node labels that are not stable under the
/// parser's label sanitiser.  SWRL rules and IRI-less ontology annotations
/// are emitted as triples under the `urn:oxidowl:` preservation vocabulary
/// (CR-011 M2).  RDF collection cells are the one structural
/// loss: re-parsing `@inlist` mints fresh cell nodes (the items, their
/// order and the head connection are preserved; RDF lists are equivalent up
/// to blank-node renaming).
#[derive(Debug, Clone, Default)]
pub struct RdfaRenderer;

impl RdfaRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Serialize `ontology` as an HTML5 document annotated with `RDFa` 1.1.
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let triples = ontology_to_triples(ontology)?;
        let mut notes: Vec<String> = Vec::new();

        // ── Constructs with no RDFa spelling → visible HTML comments ────
        //    (SWRL rules and IRI-less ontology annotations now *do* have a
        //    spelling: [`ontology_to_triples`] emits them under the
        //    `urn:oxidowl:` preservation vocabulary, CR-011 M2.)
        for import in &ontology.imports {
            notes.push(format!(
                "unsupported import declaration: <{}>",
                import.imported_ontology_iri
            ));
        }

        // ── RDF collection detection (drives `@inlist`) ──────────────────
        let mut outgoing: HashMap<String, Vec<(String, RdfTerm)>> = HashMap::new();
        for triple in &triples {
            if let (RdfTerm::BlankNode(id), RdfTerm::Iri(predicate)) =
                (&triple.subject, &triple.predicate)
            {
                outgoing
                    .entry(id.clone())
                    .or_default()
                    .push((predicate.as_str().to_string(), triple.object.clone()));
            }
        }
        let mut referenced: HashMap<String, usize> = HashMap::new();
        for triple in &triples {
            if let RdfTerm::BlankNode(id) = &triple.object {
                *referenced.entry(id.clone()).or_default() += 1;
            }
        }

        // Blank subject → the indices of every triple it participates in
        // (used to consume a collection's cells in O(1) per cell).
        let mut by_subject: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, triple) in triples.iter().enumerate() {
            if let RdfTerm::BlankNode(id) = &triple.subject {
                by_subject.entry(id.clone()).or_default().push(index);
            }
        }

        let mut cell_triples: HashSet<usize> = HashSet::new();
        let mut collections: HashMap<usize, Vec<RdfTerm>> = HashMap::new();
        for (index, triple) in triples.iter().enumerate() {
            if let (RdfTerm::Iri(_), RdfTerm::BlankNode(head)) = (&triple.predicate, &triple.object)
                && referenced.get(head.as_str()) == Some(&1)
                && let Some((items, cells)) = walk_collection(&outgoing, head.as_str())
            {
                for cell in &cells {
                    cell_triples.extend(
                        by_subject
                            .get(cell)
                            .into_iter()
                            .flatten()
                            .filter(|other| **other != index),
                    );
                }
                collections.insert(index, items);
            }
        }

        // ── Group the remaining triples per subject ──────────────────────
        let mut blocks: Vec<SubjectBlock> = Vec::new();
        let mut block_index: HashMap<String, usize> = HashMap::new();
        for (index, triple) in triples.iter().enumerate() {
            if cell_triples.contains(&index) {
                continue;
            }
            let RdfTerm::Iri(predicate) = &triple.predicate else {
                continue;
            };
            let predicate = predicate.as_str().to_string();
            if is_directional_literal(&triple.object) {
                notes.push(format!(
                    "unsupported literal (base direction cannot be expressed in RDFa): {}",
                    ntriples_syntax(triple)
                ));
                continue;
            }
            if !subject_is_expressible(&triple.subject) || !object_is_expressible(&triple.object) {
                notes.push(format!(
                    "unsupported term (cannot round-trip through RDFa): {}",
                    ntriples_syntax(triple)
                ));
                continue;
            }
            let key = format!("{:?}", triple.subject);
            let position = if let Some(position) = block_index.get(&key) {
                *position
            } else {
                blocks.push(SubjectBlock {
                    subject: triple.subject.clone(),
                    types: Vec::new(),
                    children: Vec::new(),
                    lists: Vec::new(),
                });
                let position = blocks.len() - 1;
                block_index.insert(key, position);
                position
            };
            let block = &mut blocks[position];
            if let Some(items) = collections.remove(&index) {
                for item in items {
                    block.lists.push((predicate.clone(), item));
                }
                continue;
            }
            if predicate == RDF_TYPE
                && let RdfTerm::Iri(object) = &triple.object
            {
                let iri = object.as_str().to_string();
                if !block.types.contains(&iri) {
                    block.types.push(iri);
                }
                continue;
            }
            block.children.push((predicate, triple.object.clone()));
        }

        // ── Render ───────────────────────────────────────────────────────
        let mut buf = String::from("<!DOCTYPE html>\n<html vocab=\"\"");
        buf.push_str(&format!(
            " prefix=\"{}\">\n",
            self.prefix_declaration(ontology)
        ));
        buf.push_str("<head>\n<title>OxidOWL RDFa 1.1 export</title>\n</head>\n<body>\n");
        for note in &notes {
            buf.push_str(&format!("<!-- {} -->\n", html_comment_safe(note)));
        }
        for block in &blocks {
            buf.push_str("  <div");
            buf.push_str(&format!(
                " about=\"{}\"",
                Self::attribute_value(&subject_attribute(&block.subject))
            ));
            if !block.types.is_empty() {
                buf.push_str(&format!(
                    " typeof=\"{}\"",
                    Self::attribute_value(&block.types.join(" "))
                ));
            }
            buf.push_str(">\n");
            for (predicate, object) in &block.children {
                let predicate = Self::attribute_value(predicate);
                match object {
                    RdfTerm::Iri(target) => {
                        buf.push_str(&format!(
                            "    <a rel=\"{predicate}\" href=\"{}\"></a>\n",
                            Self::attribute_value(target.as_str())
                        ));
                    }
                    RdfTerm::BlankNode(target) => {
                        buf.push_str(&format!(
                            "    <span property=\"{predicate}\" resource=\"_:{target}\"></span>\n"
                        ));
                    }
                    RdfTerm::Literal { .. } => {
                        buf.push_str(&format!(
                            "    {}\n",
                            Self::literal_element(predicate, object, false)
                        ));
                    }
                    other => {
                        log::warn!("RDFa: skipping unsupported object term {other:?}");
                    }
                }
            }
            for (predicate, item) in &block.lists {
                let predicate = Self::attribute_value(predicate);
                match item {
                    RdfTerm::Iri(target) => {
                        buf.push_str(&format!(
                            "    <a rel=\"{predicate}\" inlist=\"\" href=\"{}\"></a>\n",
                            Self::attribute_value(target.as_str())
                        ));
                    }
                    RdfTerm::BlankNode(target) => {
                        buf.push_str(&format!(
                            "    <span property=\"{predicate}\" inlist=\"\" resource=\"_:{target}\"></span>\n"
                        ));
                    }
                    RdfTerm::Literal { .. } => {
                        buf.push_str(&format!(
                            "    {}\n",
                            Self::literal_element(predicate, item, true)
                        ));
                    }
                    other => {
                        log::warn!("RDFa: skipping unsupported list item term {other:?}");
                    }
                }
            }
            buf.push_str("  </div>\n");
        }
        buf.push_str("</body>\n</html>\n");
        Ok(buf)
    }

    /// The `@prefix` block: the writer's standard RDFa/OWL vocabulary plus
    /// every prefix the ontology declares (author declarations win).
    fn prefix_declaration(&self, ontology: &Ontology) -> String {
        let mut merged: Vec<(String, String)> = vec![
            ("rdf".to_string(), RDF_NS.to_string()),
            ("rdfs".to_string(), RDFS_NS.to_string()),
            ("owl".to_string(), OWL_NS.to_string()),
            ("xsd".to_string(), XSD_NS.to_string()),
            ("foaf".to_string(), FOAF_NS.to_string()),
            ("xhv".to_string(), XHV_NS.to_string()),
        ];
        for (name, iri) in &ontology.prefixes {
            let name = name.to_lowercase();
            let valid = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
            if !valid {
                continue;
            }
            if let Some(entry) = merged.iter_mut().find(|(existing, _)| *existing == name) {
                entry.1 = iri.as_str().to_string();
            } else {
                merged.push((name, iri.as_str().to_string()));
            }
        }
        merged
            .iter()
            .map(|(name, iri)| format!("{name}: {iri}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// `<span property=… content=…>` (optionally `@inlist`), with the
    /// literal's `@datatype` / `@lang` attributes.
    fn literal_element(predicate: String, object: &RdfTerm, inlist: bool) -> String {
        let RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } = object
        else {
            unreachable!("literal_element is only called for literals");
        };
        let mut out = format!(
            "<span property=\"{}\"{} content=\"{}\"",
            predicate,
            if inlist { " inlist=\"\"" } else { "" },
            Self::attribute_value(value)
        );
        if let Some(datatype) = datatype {
            out.push_str(&format!(
                " datatype=\"{}\"",
                Self::attribute_value(datatype.as_str())
            ));
        } else if let Some(language) = language {
            out.push_str(&format!(" lang=\"{}\"", Self::attribute_value(language)));
        }
        out.push_str("></span>");
        out
    }

    /// Escape an attribute value (html5ever unescapes entities on parse, so
    /// this is lossless).
    fn attribute_value(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

/// `@about`/`@resource`/`@href` spelling of a term: the absolute IRI, or
/// `_:label` for a blank node whose label survives the parser's sanitiser.
fn subject_attribute(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(url) => url.as_str().to_string(),
        RdfTerm::BlankNode(id) => format!("_:{id}"),
        other => format!("{other:?}"),
    }
}

/// RDF 1.2 `dirLangString` literals carry a base direction `RDFa` has no
/// attribute for — they are reported instead of being written lossily.
fn is_directional_literal(term: &RdfTerm) -> bool {
    matches!(
        term,
        RdfTerm::Literal {
            direction: Some(_),
            ..
        }
    )
}

fn subject_is_expressible(term: &RdfTerm) -> bool {
    match term {
        RdfTerm::Iri(_) => true,
        RdfTerm::BlankNode(id) => sanitize_bnode_label(id) == *id,
        _ => false,
    }
}

fn object_is_expressible(term: &RdfTerm) -> bool {
    match term {
        RdfTerm::Iri(_) | RdfTerm::Literal { .. } => true,
        RdfTerm::BlankNode(id) => sanitize_bnode_label(id) == *id,
        _ => false,
    }
}

/// Walk a complete `rdf:first`/`rdf:rest` chain rooted at `head`, returning
/// the items in order and the cell labels of the chain.  `None` when the
/// chain is broken, cyclic, `rdf:nil`-terminated without items, or when a
/// cell carries anything besides `rdf:first`/`rdf:rest`.
fn walk_collection(
    outgoing: &HashMap<String, Vec<(String, RdfTerm)>>,
    head: &str,
) -> Option<(Vec<RdfTerm>, Vec<String>)> {
    let nil = Url::parse(rdf_to_owl::RDF_NIL).ok()?;
    let mut items = Vec::new();
    let mut cells = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut current = head.to_string();
    loop {
        if !visited.insert(current.clone()) {
            return None;
        }
        cells.push(current.clone());
        let entries = outgoing.get(&current)?;
        let mut first: Option<&RdfTerm> = None;
        let mut rest: Option<&RdfTerm> = None;
        for (predicate, object) in entries {
            if predicate == rdf_to_owl::RDF_FIRST && first.is_none() {
                first = Some(object);
            } else if predicate == rdf_to_owl::RDF_REST && rest.is_none() {
                rest = Some(object);
            } else {
                return None; // extra or duplicate cell triple
            }
        }
        items.push(first?.clone());
        match rest? {
            RdfTerm::Iri(url) if *url == nil => return Some((items, cells)),
            RdfTerm::BlankNode(next) => current = next.clone(),
            _ => return None,
        }
    }
}

/// Make text safe inside `<!-- … -->` (no `--`, no trailing `-`).
fn html_comment_safe(text: &str) -> String {
    let mut out = text.replace("--", "- -");
    while out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = RdfaRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("RDFa: {e}")))
}
