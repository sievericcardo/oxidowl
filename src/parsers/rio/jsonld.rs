//! JSON-LD 1.1 parser and serializer (B5 Slice 1).
//!
//! The previous implementation was a hand-rolled flat JSON→assertion mapper
//! with no `@context` resolution, no expansion algorithm and a context-less
//! writer.  This module now delegates both directions to the oxigraph stack
//! (`oxjsonld` / `oxrdfio`, versions pinned to the ones already in the
//! dependency tree via `oxigraph`):
//!
//! * **Parse** — [`oxrdfio::RdfParser`] with [`oxrdfio::RdfFormat::JsonLd`]
//!   runs the real JSON-LD 1.1 *expansion* algorithm (embedded `@context`
//!   term definitions, `@vocab`, `@base`, `@language`, `@type` coercion,
//!   arrays, nested nodes, `@graph` — default and named, value objects,
//!   blank nodes, `@list`).  The resulting RDF quads are mapped into the
//!   ontology by the **shared** RDF→OWL mapping in
//!   [`crate::parsers::rdf_to_owl`] — the same code the Turtle and RDF/XML
//!   parsers use — including blank-node restriction reconstruction.
//! * **Serialize** — the ontology is converted to RDF triples with the
//!   shared OWL→RDF emission ([`crate::semantics::owl_rdf_mapping`]),
//!   serialized by `oxjsonld` (node-object grouping + an auto-generated
//!   `@context` carrying prefix definitions), then prefix-compacted against
//!   that same context so predicates/ids emit as `ex:A`-style compact IRIs
//!   and `rdf:type` becomes native `@type`.
//!
//! Remote `@context` URLs are **never fetched by default** (SSRF surface:
//! a document-controlled URL would otherwise trigger a blocking HTTP GET).
//! Opt in explicitly with [`JsonLdParser::with_remote_contexts`] (requires
//! the `http-imports` feature); otherwise oxjsonld fails with a clear
//! `LoadingRemoteContextFailed` error — remote contexts are never silently
//! ignored.

use std::collections::{BTreeSet, HashSet};

use oxrdf::{BlankNode, GraphNameRef, Literal as OxLiteral, NamedNode, Term as OxTerm, TripleRef};
use oxrdfio::{JsonLdProfileSet, LoadedDocument, RdfFormat, RdfParser};
use serde_json::Value;

use crate::ontology::{IRI, Ontology};
use crate::parsers::rdf_to_owl::{self, RDF_TYPE};
use crate::semantics::owl_rdf_mapping::ontology_to_triples;
use crate::semantics::{RdfTerm, Triple};
use crate::{Error, Result};

const RDF_TYPE_IRI: &str = RDF_TYPE;
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Upper bound on `@context` prefix entries emitted for a document.
const MAX_PREFIXES: usize = 24;

/// Remote context documents larger than this are rejected.
#[cfg(feature = "http-imports")]
const MAX_REMOTE_CONTEXT_BYTES: usize = 5 * 1024 * 1024;

// ═══════════════════════════════════════════════════════════════════════════
// Parser
// ═══════════════════════════════════════════════════════════════════════════

/// JSON-LD 1.1 parser (expansion + RDF→OWL mapping).
#[derive(Debug, Clone)]
pub struct JsonLdParser {
    /// Optional base IRI for resolving relative IRIs that the document
    /// itself does not anchor via `@base`.
    base_iri: Option<String>,
    /// Whether remote `@context`/`@import` URLs may be fetched.
    /// **Defaults to `false`** — remote fetching is an explicit opt-in
    /// (mirrors `LoaderConfig`): parsing untrusted JSON-LD must never issue
    /// HTTP requests to document-controlled URLs (CR-011 M3, SSRF).
    remote_contexts: bool,
}

impl Default for JsonLdParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonLdParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_iri: None,
            remote_contexts: false,
        }
    }

    /// Set a document base IRI (JSON-LD `base` option) used to resolve
    /// relative IRIs when the document has no `@base` of its own.
    #[must_use]
    pub fn with_base_iri(mut self, base_iri: impl Into<String>) -> Self {
        self.base_iri = Some(base_iri.into());
        self
    }

    /// Allow fetching remote `@context`/`@import` URLs over HTTP(S).
    ///
    /// **Opt-in and off by default** (CR-011 M3): with this disabled — the
    /// default — a remote context reference fails with a clear
    /// `LoadingRemoteContextFailed` error instead of issuing a
    /// document-controlled HTTP request.  Requires the `http-imports`
    /// feature to have any effect; size/time bounds on fetched contexts
    /// still apply.
    #[must_use]
    pub fn with_remote_contexts(mut self, enabled: bool) -> Self {
        self.remote_contexts = enabled;
        self
    }

    /// Never fetch remote `@context` URLs: only local/embedded contexts are
    /// accepted and a remote context reference fails with a clear error.
    /// (Kept for source compatibility; this is already the default.)
    #[must_use]
    pub fn without_remote_contexts(mut self) -> Self {
        self.remote_contexts = false;
        self
    }

    /// Parse a JSON-LD document from a string.
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        self.parse_bytes(content.as_bytes())
    }

    /// Parse a JSON-LD document from raw bytes (must be UTF-8 JSON).
    pub fn parse_bytes(&self, bytes: &[u8]) -> Result<Ontology> {
        let mut parser = RdfParser::from_format(RdfFormat::JsonLd {
            profile: JsonLdProfileSet::empty(),
        });
        if let Some(base) = &self.base_iri {
            parser = parser.with_base_iri(base).map_err(|e| {
                Error::ParseError(format!("JSON-LD: invalid base IRI '{base}': {e}"))
            })?;
        }

        let reader = parser.for_reader(bytes);
        // Remote contexts: fetch over HTTP only on explicit opt-in via
        // `with_remote_contexts(true)` (and only when the `http-imports`
        // feature is enabled).  Otherwise no document loader is installed
        // and oxjsonld fails with `LoadingRemoteContextFailed: No
        // LoadDocumentCallback has been set to load remote contexts` — a
        // clear error, never silence.
        #[cfg(feature = "http-imports")]
        let reader = if self.remote_contexts {
            reader.with_document_loader(remote_context_loader())
        } else {
            reader
        };

        let mut reader = reader;
        let mut triples: Vec<Triple> = Vec::new();
        for quad in reader.by_ref() {
            let quad = quad.map_err(|e| Error::ParseError(format!("JSON-LD: {e}")))?;
            // Named graphs (`@graph` with `@id`) and the default graph are
            // unioned into the single ontology triple set — an `Ontology`
            // has no named-graph structure.
            let (subject, predicate, object) = (quad.subject, quad.predicate, quad.object);
            let subject = convert_subject(&subject)?;
            let predicate = convert_named_node(&predicate)?;
            let object = convert_term(&object)?;
            triples.push(Triple {
                subject,
                predicate,
                object,
            });
        }

        let mut ontology = Ontology::new();
        // Keep the prefix definitions discovered in the document's
        // `@context` (mirrors what the Functional/Turtle parsers do), so a
        // subsequent serialization can re-emit them.
        for (name, iri) in reader.prefixes() {
            if !name.is_empty() && !name.starts_with('@') && !name.contains(':') {
                ontology.prefixes.insert(name.to_owned(), IRI::new(iri));
            }
        }
        rdf_to_owl::map_rdf_triples(&triples, &mut ontology)?;
        Ok(ontology)
    }
}

/// oxjsonld `documentLoader` backed by the blocking reqwest client.
#[cfg(feature = "http-imports")]
fn remote_context_loader()
-> impl Fn(&str) -> std::result::Result<LoadedDocument, Box<dyn std::error::Error + Send + Sync>>
+ Send
+ Sync
+ std::panic::UnwindSafe
+ std::panic::RefUnwindSafe
+ 'static {
    |url| {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(format!(
                "remote JSON-LD contexts are only fetched over http/https, got: {url}"
            )
            .into());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!("oxidowl/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let response = client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "application/ld+json, application/json;q=0.9, */*;q=0.1",
            )
            .send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {status} fetching remote JSON-LD context {url}").into());
        }
        let content = response.bytes()?.to_vec();
        if content.len() > MAX_REMOTE_CONTEXT_BYTES {
            return Err(format!(
                "remote JSON-LD context {url} is larger than {MAX_REMOTE_CONTEXT_BYTES} bytes"
            )
            .into());
        }
        Ok(LoadedDocument {
            url: url.to_string(),
            content,
            format: RdfFormat::JsonLd {
                profile: oxjsonld::JsonLdProfile::Context.into(),
            },
        })
    }
}

// ── oxrdf → semantics term conversion ────────────────────────────────────

fn convert_iri(iri: &str) -> Result<RdfTerm> {
    url::Url::parse(iri)
        .map(RdfTerm::Iri)
        .map_err(|e| Error::ParseError(format!("JSON-LD: invalid IRI '{iri}': {e}")))
}

fn convert_named_node(node: &NamedNode) -> Result<RdfTerm> {
    convert_iri(node.as_str())
}

fn convert_subject(subject: &oxrdf::NamedOrBlankNode) -> Result<RdfTerm> {
    match subject {
        oxrdf::NamedOrBlankNode::NamedNode(node) => convert_named_node(node),
        oxrdf::NamedOrBlankNode::BlankNode(bnode) => {
            Ok(RdfTerm::BlankNode(bnode.as_str().to_string()))
        }
    }
}

fn convert_term(term: &OxTerm) -> Result<RdfTerm> {
    match term {
        OxTerm::NamedNode(node) => convert_named_node(node),
        OxTerm::BlankNode(bnode) => Ok(RdfTerm::BlankNode(bnode.as_str().to_string())),
        OxTerm::Literal(lit) => {
            let language = lit.language().map(str::to_owned);
            let datatype = if language.is_some() {
                // RDF language strings: the OWL model stores `language`
                // without the `rdf:langString` datatype (as Turtle does).
                None
            } else {
                Some(url::Url::parse(lit.datatype().as_str()).map_err(|e| {
                    Error::ParseError(format!("JSON-LD: invalid literal datatype: {e}"))
                })?)
            };
            Ok(RdfTerm::Literal {
                value: lit.value().to_owned(),
                datatype,
                language,
                direction: None,
            })
        }
    }
}

/// Parse a JSON-LD document from a string using the default parser.
pub fn parse(content: &str) -> Result<Ontology> {
    JsonLdParser::new().parse(content)
}

/// Parse a JSON-LD document from raw bytes using the default parser.
pub fn parse_bytes(bytes: &[u8]) -> Result<Ontology> {
    JsonLdParser::new().parse_bytes(bytes)
}

// ═══════════════════════════════════════════════════════════════════════════
// Serializer
// ═══════════════════════════════════════════════════════════════════════════

/// JSON-LD 1.1 serializer (OWL→RDF emission + oxjsonld + prefix compaction).
#[derive(Debug, Clone, Default)]
pub struct JsonLdRenderer;

impl JsonLdRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Serialize an ontology as a JSON-LD 1.1 document: a `@context` with
    /// prefix definitions plus a `@graph` of node objects.
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        // ── 1. Ontology → RDF triples via the shared OWL→RDF emission ───
        //    (`ontology_to_triples` — also emits SWRL rules and the
        //    IRI-less ontology annotations under the fallback subject, so
        //    nothing is silently lost on round-trip; CR-011 M2.)
        let triples = ontology_to_triples(ontology)?;

        // ── 2. `@context` prefix definitions derived from the IRIs used ──
        let prefixes = collect_prefixes(&triples, ontology);

        // ── 3. Serialize the triples with oxjsonld ────────────────────────
        let mut serializer = oxjsonld::JsonLdSerializer::new();
        for (name, iri) in &prefixes {
            serializer = serializer.with_prefix(name, iri).map_err(|e| {
                Error::ontology_parsing(format!("JSON-LD: invalid prefix IRI '{iri}': {e}"))
            })?;
        }
        let mut writer = serializer.for_writer(Vec::new());
        for t in &triples {
            let ox_triple = to_oxrdf_triple(t)?;
            writer
                .serialize_quad(TripleRef::from(&ox_triple).in_graph(GraphNameRef::DefaultGraph))
                .map_err(|e| Error::ontology_parsing(format!("JSON-LD: {e}")))?;
        }
        let bytes = writer
            .finish()
            .map_err(|e| Error::ontology_parsing(format!("JSON-LD: {e}")))?;

        // ── 4. Prefix-compaction pass against the emitted `@context` ─────
        let mut document: Value = serde_json::from_slice(&bytes).map_err(|e| {
            Error::ontology_parsing(format!("JSON-LD: invalid serializer output: {e}"))
        })?;
        compact_document(&mut document, &prefixes);
        serde_json::to_string_pretty(&document)
            .map_err(|e| Error::ontology_parsing(format!("JSON-LD: {e}")))
    }
}

/// Save an ontology as a JSON-LD file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = JsonLdRenderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| Error::io(format!("JSON-LD: {e}")))
}

// ── semantics::Triple → oxrdf::Triple ────────────────────────────────────

fn to_oxrdf_term(term: &RdfTerm) -> Result<OxTerm> {
    match term {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str())
            .map(OxTerm::from)
            .map_err(|e| Error::ontology_parsing(format!("JSON-LD: invalid IRI '{url}': {e}"))),
        RdfTerm::BlankNode(id) => BlankNode::new(id.as_str()).map(OxTerm::from).map_err(|e| {
            Error::ontology_parsing(format!(
                "JSON-LD: blank node id '{id}' is not a valid RDF blank node label: {e}"
            ))
        }),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                OxLiteral::new_language_tagged_literal(value.as_str(), lang.as_str()).map_err(
                    |e| Error::ontology_parsing(format!("JSON-LD: invalid language tag: {e}")),
                )?
            } else if let Some(dt) = datatype {
                let node = NamedNode::new(dt.as_str()).map_err(|e| {
                    Error::ontology_parsing(format!(
                        "JSON-LD: invalid literal datatype '{dt}': {e}"
                    ))
                })?;
                OxLiteral::new_typed_literal(value.as_str(), node)
            } else {
                OxLiteral::new_simple_literal(value.as_str())
            };
            Ok(OxTerm::Literal(lit))
        }
        other => Err(Error::ontology_parsing(format!(
            "JSON-LD: unsupported term for serialization: {other}"
        ))),
    }
}

fn to_oxrdf_triple(t: &Triple) -> Result<oxrdf::Triple> {
    let subject = match &t.subject {
        RdfTerm::Iri(url) => oxrdf::NamedOrBlankNode::NamedNode(
            NamedNode::new(url.as_str())
                .map_err(|e| Error::ontology_parsing(format!("JSON-LD subject IRI: {e}")))?,
        ),
        RdfTerm::BlankNode(id) => oxrdf::NamedOrBlankNode::BlankNode(
            BlankNode::new(id.as_str())
                .map_err(|e| Error::ontology_parsing(format!("JSON-LD subject blank node: {e}")))?,
        ),
        other => {
            return Err(Error::ontology_parsing(format!(
                "JSON-LD: unsupported subject term: {other}"
            )));
        }
    };
    let predicate = match &t.predicate {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str())
            .map_err(|e| Error::ontology_parsing(format!("JSON-LD predicate IRI: {e}")))?,
        other => {
            return Err(Error::ontology_parsing(format!(
                "JSON-LD: unsupported predicate term: {other}"
            )));
        }
    };
    Ok(oxrdf::Triple {
        subject,
        predicate,
        object: to_oxrdf_term(&t.object)?,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Prefix derivation + compaction
// ═══════════════════════════════════════════════════════════════════════════

/// Namespace of an IRI (everything through the last `#` or `/`), if any.
fn namespace_of(iri: &str) -> Option<String> {
    let hash = iri.rfind('#');
    let slash = iri.rfind('/');
    let cut = match (hash, slash) {
        (Some(h), Some(s)) => Some(h.max(s)),
        (Some(h), None) => Some(h),
        (None, Some(s)) => Some(s),
        (None, None) => None,
    }?;
    let ns = &iri[..=cut];
    // Reject garbage like "http:/" from scheme-relative splits.
    if url::Url::parse(ns).is_err() {
        return None;
    }
    Some(ns.to_string())
}

/// Derive a JSON-LD term name from a namespace IRI.
fn prefix_name_from_ns(ns: &str) -> String {
    let trimmed = ns.trim_end_matches(['#', '/']);
    let segment = trimmed
        .rsplit(['/', '#'])
        .find(|s| !s.is_empty())
        .unwrap_or("");
    let mut name: String = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.len() > 32 {
        name.truncate(32);
    }
    if name.is_empty() {
        return String::new();
    }
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name = format!("n{name}");
    }
    name
}

/// Build the ordered `(name, namespace)` prefix list for a document.
///
/// The four RDF/OWL vocabulary namespaces are always present (they keep the
/// `@context` non-empty so `@graph` is always emitted, and they cover
/// `rdf:type`/`owl:`/`rdfs:`/`xsd:` compaction); other namespaces are
/// derived deterministically from the IRIs used in the graph.
fn collect_prefixes(triples: &[Triple], ontology: &Ontology) -> Vec<(String, String)> {
    let standard = [
        (RDF_NS, "rdf"),
        (RDFS_NS, "rdfs"),
        (OWL_NS, "owl"),
        (XSD_NS, "xsd"),
    ];
    let mut out: Vec<(String, String)> = standard
        .iter()
        .map(|(ns, name)| ((*name).to_string(), (*ns).to_string()))
        .collect();
    let mut used: HashSet<String> = out.iter().map(|(n, _)| n.clone()).collect();
    let mut covered: HashSet<String> = out.iter().map(|(_, ns)| ns.clone()).collect();

    // Ontology prefix map (as written by the Functional/Manchester parsers).
    let mut seed: Vec<(&String, &IRI)> = ontology.prefixes.iter().collect();
    seed.sort_by(|a, b| a.0.cmp(b.0));
    for (name, iri) in seed {
        if name.is_empty()
            || name.starts_with('@')
            || name.contains(':')
            || used.contains(name)
            || covered.contains(iri.as_str())
        {
            continue;
        }
        used.insert(name.clone());
        covered.insert(iri.as_str().to_string());
        out.push((name.clone(), iri.as_str().to_string()));
    }

    let mut namespaces: BTreeSet<String> = BTreeSet::new();
    for t in triples {
        for term in [&t.subject, &t.predicate, &t.object] {
            let iri = match term {
                RdfTerm::Iri(url) => url.as_str(),
                RdfTerm::Literal {
                    datatype: Some(dt), ..
                } => dt.as_str(),
                _ => continue,
            };
            if let Some(ns) = namespace_of(iri) {
                namespaces.insert(ns);
            }
        }
    }

    let mut fallback = 1;
    for ns in namespaces {
        if standard.iter().any(|(s, _)| *s == ns) || covered.contains(&ns) {
            continue;
        }
        if out.len() >= MAX_PREFIXES {
            break;
        }
        let mut name = prefix_name_from_ns(&ns);
        if name.is_empty() {
            name = format!("ns{fallback}");
            fallback += 1;
        }
        let mut candidate = name.clone();
        let mut suffix = 2;
        while used.contains(&candidate) {
            candidate = format!("{name}_{suffix}");
            suffix += 1;
        }
        used.insert(candidate.clone());
        out.push((candidate, ns));
    }
    out
}

/// Compact a string IRI against the prefix list (`http://…#A` → `ex:A`).
/// Returns `None` when no prefix applies or the result would be ambiguous.
fn compact_iri(iri: &str, prefixes: &[(String, String)]) -> Option<String> {
    if iri.starts_with("_:") {
        return None; // blank node labels stay verbatim
    }
    // Longest matching namespace wins (most specific prefix).
    let mut sorted: Vec<&(String, String)> = prefixes.iter().collect();
    sorted.sort_by_key(|(_, ns)| std::cmp::Reverse(ns.len()));
    for (name, ns) in sorted {
        if iri.len() > ns.len() && iri.starts_with(ns.as_str()) {
            let local = &iri[ns.len()..];
            if local.is_empty() || local.starts_with("//") {
                continue;
            }
            // Compact IRIs are `name:local`; a `local` containing `:` still
            // round-trips (expansion concatenates prefix IRI + suffix), but
            // only when the name itself has no colon — guaranteed here.
            return Some(format!("{name}:{local}"));
        }
    }
    None
}

/// In-place prefix compaction of a serialized JSON-LD document.
///
/// Rewrites absolute-IRI property keys and `@id` / `@type` values to compact
/// IRIs using the `@context` that [`collect_prefixes`] generated, and
/// rewrites the expanded `rdf:type` property to native `@type`.  Both
/// rewrites expand back to exactly the original IRIs.
fn compact_document(document: &mut Value, prefixes: &[(String, String)]) {
    match document {
        Value::Object(map) => {
            // Top-level or nested node object.
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if key.starts_with('@') {
                    continue; // keywords are handled below
                }
                if key == RDF_TYPE_IRI || key == "rdf:type" {
                    if let Some(value) = map.remove(&key) {
                        let flattened = flatten_type_value(value);
                        map.insert("@type".to_owned(), compact_type_value(flattened, prefixes));
                    }
                    continue;
                }
                if let Some(compacted) = compact_iri(&key, prefixes)
                    && let Some(value) = map.remove(&key)
                {
                    map.insert(compacted, value);
                }
            }
            for (key, value) in map.iter_mut() {
                match key.as_str() {
                    "@context" => {} // never touch the context itself
                    "@id" => {
                        if let Value::String(s) = value
                            && let Some(compacted) = compact_iri(s, prefixes)
                        {
                            *s = compacted;
                        }
                    }
                    "@type" => {
                        *value = compact_type_value(std::mem::take(value), prefixes);
                    }
                    "@value" | "@language" | "@direction" | "@index" | "@reverse" => {}
                    _ => compact_document(value, prefixes),
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                compact_document(item, prefixes);
            }
        }
        _ => {}
    }
}

/// Flatten an `rdf:type` value (`[{"@id": …}]`) into the plain-string form
/// required by JSON-LD's native `@type` keyword.
fn flatten_type_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Value::Object(mut map) if map.len() == 1 => {
                        map.remove("@id").unwrap_or(Value::Null)
                    }
                    other => other,
                })
                .collect(),
        ),
        Value::Object(mut map) if map.len() == 1 => map.remove("@id").unwrap_or(Value::Null),
        other => other,
    }
}

/// Compact every string inside a native `@type` value.
fn compact_type_value(value: Value, prefixes: &[(String, String)]) -> Value {
    match value {
        Value::String(s) => Value::String(compact_iri(&s, prefixes).unwrap_or(s)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Value::String(s) => Value::String(compact_iri(&s, prefixes).unwrap_or(s)),
                    other => other,
                })
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_name_derivation() {
        assert_eq!(prefix_name_from_ns("http://example.org/"), "example.org");
        assert_eq!(prefix_name_from_ns("http://example.org/ont#"), "ont");
        assert_eq!(prefix_name_from_ns("http://www.w3.org/2002/07/owl#"), "owl");
        assert_eq!(prefix_name_from_ns("http://9site.io/"), "n9site.io");
    }

    #[test]
    fn compact_iri_longest_prefix_wins() {
        let prefixes = vec![
            ("ex".to_string(), "http://example.org/".to_string()),
            ("ont".to_string(), "http://example.org/ont#".to_string()),
        ];
        assert_eq!(
            compact_iri("http://example.org/ont#A", &prefixes).as_deref(),
            Some("ont:A")
        );
        assert_eq!(
            compact_iri("http://example.org/knows", &prefixes).as_deref(),
            Some("ex:knows")
        );
        assert_eq!(compact_iri("_:b0", &prefixes), None);
        assert_eq!(compact_iri("http://other.org/x", &prefixes), None);
    }

    #[test]
    fn parse_rejects_remote_context_without_loader() {
        let doc = r#"{"@context":"http://example.org/remote.jsonld","@id":"http://example.org/a"}"#;
        let err = JsonLdParser::new()
            .without_remote_contexts()
            .parse(doc)
            .expect_err("remote context must fail without a document loader");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("remote context")
                || msg.to_lowercase().contains("load document"),
            "error must clearly mention the remote context problem, got: {msg}"
        );
    }
}
