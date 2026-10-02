//! Notation3 (N3) parser and serializer — a *real* superset of Turtle
//! (B5 Slice 2, docs/PLAN.md).
//!
//! Parsing is done by the **oxttl** N3 parser (`oxttl::n3::N3Parser` — the
//! engine behind `oxrdfio::RdfFormat::N3`), which understands the full N3
//! grammar beyond Turtle: `=>` / `<=` implications, `{ … }` formulae
//! (produced as quads whose graph name is the formula's blank node),
//! variables (`?x`), paths (`!`, `^`), `has` / `is … of` inverse verbs,
//! collections (RDF lists), `=` (`owl:sameAs`) and the `a` shorthand
//! (`rdf:type`).
//!
//! One upstream quirk is repaired here (see `repair_path_quads`): oxttl
//! expands *predicate*-position paths as if the first path item were a node,
//! emitting invalid blank-predicate quads; this module reconstructs the
//! correct `subject p1 _:m . _:m p2 object` expansion from the intermediate
//! blank-node linkage (subject/object-position paths are already correct and
//! pass through untouched).
//!
//! ## Model mapping
//!
//! | N3 construct | Ontology representation |
//! |---|---|
//! | Ground default-graph triples | The **shared** RDF→OWL mapping in [`crate::parsers::rdf_to_owl`] (identical to Turtle / RDF/XML / JSON-LD): declarations, `SubClassOf`, assertions, blank-node restriction reconstruction, RDF lists, … |
//! | `_:body log:implies _:head` where both sides are SWRL-expressible | [`Axiom::Rule`] (a [`SWRLRule`]): class / object-property / data-property atoms over variables, individuals and literals. `owl:sameAs` / `owl:differentFrom` become the corresponding SWRL atoms. N3 variables are minted as IRIs `urn:oxidowl:n3var:<name>`. |
//! | `log:implies` implications that are **not** SWRL-expressible (nested formulae, list structure, exotic predicates, …) | Preserved *losslessly* as an ontology-level annotation with property `log:implies` (`http://www.w3.org/2000/10/swap/log#implies`) whose literal value is the canonical N3 text `{ … } => { … }`. Never dropped. |
//! | Formulae that are not an implication side (standalone `{ … }`, loose variable-bearing triples) | Preserved as `log:Formula` annotations holding the quoted formula text. Never dropped. |
//! | `@forAll` / `@forSome` quantifier statements (not supported by oxttl) | Extracted by a scanner *before* parsing and preserved as `log:forAll` / `log:forSome` ontology annotations whose literal value is the raw statement text (minus the final dot). |
//!
//! All preserved annotation values are re-emitted **verbatim** by the
//! serializer, so parse → serialize → parse is idempotent for rules,
//! formulae and quantifiers.  Ontology-level annotations with ordinary
//! properties are emitted as `<ontology> <property> "value"` triples, which
//! the shared mapping routes back to `Ontology::annotations` for well-known
//! annotation properties (`rdfs:comment`, …).
//!
//! ## Serialize
//!
//! Emits valid N3: axiom triples through oxttl's `TurtleSerializer` (Turtle
//! is a syntactic subset of N3; oxttl ships no dedicated N3 *serializer* and
//! `oxrdfio` maps `RdfFormat::N3` to the Turtle serializer for the same
//! reason), followed by `{ … } => { … }` statements for [`Axiom::Rule`]
//! axioms and the verbatim preserved annotations.  Prefix declarations are
//! written as `@prefix` lines up-front so they are available to every part
//! of the document.
//!
//! Known gaps (documented, never silent): SWRL built-in atoms and rules
//! whose variables were not minted by this parser cannot be expressed as N3
//! statements and are emitted as a `# oxidowl: …` comment line; SWRL class
//! atoms with *anonymous* class expressions serialize to blank-node
//! restriction syntax which re-parses as a preserved `log:implies`
//! annotation rather than a rule.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use oxrdf::{GraphName, Literal as OxLiteral, NamedNode, TripleRef};
use oxttl::n3::{N3Parser as OxN3Parser, N3Quad, N3Term};
use oxttl::turtle::TurtleSerializer as OxTurtleSerializer;

use crate::ontology::*;
use crate::parsers::rdf_to_owl;
use crate::semantics::owl_rdf_mapping::{BlankNodeCounter, axiom_to_triples};
use crate::semantics::{RdfTerm, Triple};
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// Vocabulary constants
// ═══════════════════════════════════════════════════════════════════════════

/// `log:implies` — implication predicate and preservation annotation property.
pub const LOG_IMPLIES: &str = "http://www.w3.org/2000/10/swap/log#implies";
/// `log:Formula` — preservation annotation property for quoted formulae.
pub const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";
/// `log:forAll` — preservation annotation property for `@forAll` statements.
pub const LOG_FOR_ALL: &str = "http://www.w3.org/2000/10/swap/log#forAll";
/// `log:forSome` — preservation annotation property for `@forSome` statements.
pub const LOG_FOR_SOME: &str = "http://www.w3.org/2000/10/swap/log#forSome";

/// Ontology annotations under these properties carry verbatim N3 statement
/// text that the serializer re-emits as-is (instead of as RDF triples).
pub fn is_n3_preservation_property(iri: &str) -> bool {
    matches!(iri, LOG_IMPLIES | LOG_FORMULA | LOG_FOR_ALL | LOG_FOR_SOME)
}

/// IRI prefix used to mint SWRL variables from N3 `?name` variables.
/// Documented in the module doc; stable across parses so rules round-trip.
const N3_VARIABLE_IRI_PREFIX: &str = "urn:oxidowl:n3var:";

const RDF_TYPE: &str = rdf_to_owl::RDF_TYPE;
const RDF_FIRST: &str = rdf_to_owl::RDF_FIRST;
const RDF_REST: &str = rdf_to_owl::RDF_REST;
const RDF_NIL: &str = rdf_to_owl::RDF_NIL;
const OWL_ONTOLOGY: &str = rdf_to_owl::OWL_ONTOLOGY;
const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const OWL_DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
const OWL_VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

// ═══════════════════════════════════════════════════════════════════════════
// Parser
// ═══════════════════════════════════════════════════════════════════════════

/// N3 parser (oxttl N3 engine + shared RDF→OWL mapping + rule preservation).
#[derive(Debug, Clone, Default)]
pub struct N3Parser {
    /// Optional base IRI used to resolve relative IRIs in the document.
    base_iri: Option<String>,
}

impl N3Parser {
    #[must_use]
    pub fn new() -> Self {
        Self { base_iri: None }
    }

    /// Set a base IRI used to resolve relative IRIs (N3 `base` option).
    #[must_use]
    pub fn with_base_iri(mut self, base_iri: impl Into<String>) -> Self {
        self.base_iri = Some(base_iri.into());
        self
    }

    /// Parse an N3 document from a string.
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        self.parse_bytes(content.as_bytes())
    }

    /// Parse an N3 document from raw bytes (must be UTF-8).
    pub fn parse_bytes(&self, bytes: &[u8]) -> Result<Ontology> {
        let content = std::str::from_utf8(bytes)
            .map_err(|e| Error::ParseError(format!("N3: input is not valid UTF-8: {e}")))?;
        self.parse_str(content)
    }

    fn parse_str(&self, content: &str) -> Result<Ontology> {
        // ── 1. Extract `@forAll` / `@forSome` statements (oxttl has no
        //       support for them; they are preserved as annotations). ──────
        let (scrubbed, quantifiers) = extract_quantifier_statements(content);

        // ── 2. Parse the remaining document with oxttl. ───────────────────
        let mut parser = OxN3Parser::new();
        if let Some(base) = &self.base_iri {
            parser = parser
                .with_base_iri(base.as_str())
                .map_err(|e| Error::ParseError(format!("N3: invalid base IRI '{base}': {e}")))?;
        }
        let mut reader = parser.for_slice(scrubbed.as_bytes());
        let mut quads: Vec<N3Quad> = Vec::new();
        for quad in reader.by_ref() {
            quads.push(quad.map_err(|e| Error::ParseError(format!("N3: {e}")))?);
        }
        let prefixes: Vec<(String, String)> = reader
            .prefixes()
            .map(|(name, iri)| (name.to_owned(), iri.to_owned()))
            .collect();

        // ── 3. Repair predicate-position path expansion (`p!q`, `p^q`) and
        //       partition the quads. ────────────────────────────────────────
        let quads = repair_path_quads(quads);
        let mut ground: Vec<Triple> = Vec::new();
        let mut implications: Vec<N3Quad> = Vec::new();
        let mut formulae: HashMap<String, Vec<N3Quad>> = HashMap::new();
        let mut loose: Vec<N3Quad> = Vec::new();

        for quad in quads {
            // Formula contents: quads whose graph name is the formula's
            // blank node.
            if let GraphName::BlankNode(graph) = &quad.graph_name {
                formulae
                    .entry(graph.as_str().to_owned())
                    .or_default()
                    .push(quad);
                continue;
            }
            let is_implies = matches!(
                &quad.predicate,
                N3Term::NamedNode(p) if p.as_str() == LOG_IMPLIES
            );
            if is_implies
                && matches!(&quad.subject, N3Term::BlankNode(_))
                && matches!(&quad.object, N3Term::BlankNode(_))
            {
                implications.push(quad);
            } else if quad_has_variable(&quad) {
                loose.push(quad);
            } else {
                ground.push(n3_quad_to_triple(&quad)?);
            }
        }

        // ── 4. Ground triples → shared RDF→OWL mapping (axioms). ──────────
        let mut ontology = Ontology::new();
        for (name, iri) in &prefixes {
            // The empty name is the valid Turtle/N3 default (`@prefix : <…>`),
            // which preserved statements like `@forAll :x` depend on.
            if !name.starts_with('@') && !name.contains(':') {
                ontology.prefixes.insert(name.clone(), IRI::new(iri));
            }
        }
        rdf_to_owl::map_rdf_triples(&ground, &mut ontology)?;

        // ── 5. Implications → SWRL rules or preserved `log:implies` data. ─
        let mut consumed: HashSet<String> = HashSet::new();
        for quad in &implications {
            let body_id = blank_id(&quad.subject);
            let head_id = blank_id(&quad.object);
            let body = body_id
                .as_ref()
                .and_then(|id| formulae.get(id))
                .map_or(&[][..], |v| v.as_slice());
            let head = head_id
                .as_ref()
                .and_then(|id| formulae.get(id))
                .map_or(&[][..], |v| v.as_slice());

            let body_atoms = side_to_atoms(body, &formulae, &ontology);
            let head_atoms = side_to_atoms(head, &formulae, &ontology);
            let swrl = match (body_atoms, head_atoms) {
                (Some(body_atoms), Some(head_atoms))
                    if !(body_atoms.is_empty() && head_atoms.is_empty()) =>
                {
                    Some(SWRLRule::new(head_atoms, body_atoms))
                }
                _ => None,
            };

            if let Some(rule) = swrl {
                let id = generate_axiom_id();
                ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(id, rule)));
            } else {
                let text = render_implication(&quad.subject, &quad.object, &formulae);
                push_preserved_annotation(&mut ontology, LOG_IMPLIES, text);
            }
            if let Some(id) = body_id {
                consumed.insert(id);
            }
            if let Some(id) = head_id {
                consumed.insert(id);
            }
        }

        // ── 6. Standalone formulae → preserved `log:Formula` data. ────────
        for (id, members) in &formulae {
            if consumed.contains(id) {
                continue;
            }
            let text = render_formula(members);
            push_preserved_annotation(&mut ontology, LOG_FORMULA, text);
        }

        // ── 7. Loose variable-bearing triples → quoted as `log:Formula`. ──
        for quad in &loose {
            let text = format!("{{ {} . }}", render_quad(quad));
            push_preserved_annotation(&mut ontology, LOG_FORMULA, text);
        }

        // ── 8. `@forAll` / `@forSome` statements → preserved annotations. ─
        for quantifier in &quantifiers {
            let property = if quantifier.for_all {
                LOG_FOR_ALL
            } else {
                LOG_FOR_SOME
            };
            push_preserved_annotation(&mut ontology, property, quantifier.text.clone());
        }

        Ok(ontology)
    }
}

/// Parse an N3 document from a string using the default parser.
pub fn parse(content: &str) -> Result<Ontology> {
    N3Parser::new().parse(content)
}

/// Parse an N3 document from raw bytes using the default parser.
pub fn parse_bytes(bytes: &[u8]) -> Result<Ontology> {
    N3Parser::new().parse_bytes(bytes)
}

// ═══════════════════════════════════════════════════════════════════════════
// @forAll / @forSome extraction
// ═══════════════════════════════════════════════════════════════════════════

/// A quantifier statement extracted from the source, without the final dot.
#[derive(Debug)]
struct QuantifierStatement {
    for_all: bool,
    text: String,
}

/// Find `@forAll` / `@forSome` statements outside strings, IRIs and
/// comments, remove them from the text handed to oxttl (replacing them with
/// spaces so that line/column positions of later syntax errors still match
/// the original document) and return them for preservation as annotations.
fn extract_quantifier_statements(content: &str) -> (String, Vec<QuantifierStatement>) {
    let bytes = content.as_bytes();
    let mut scrubbed = bytes.to_vec();
    let mut found: Vec<QuantifierStatement> = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = skip_string(bytes, i),
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'<' => {
                // IRIREF: skip to the closing '>' (no spaces inside IRIs).
                while i < bytes.len() && bytes[i] != b'>' && bytes[i] != b'\n' {
                    i += 1;
                }
                i += 1;
            }
            b'@' => {
                let keyword = if matches_keyword(&bytes[i..], "@forall") {
                    Some(("@forall", true))
                } else if matches_keyword(&bytes[i..], "@forsome") {
                    Some(("@forsome", false))
                } else {
                    None
                };
                match keyword {
                    Some((kw, for_all))
                        if bytes
                            .get(i + kw.len())
                            .is_some_and(|c| c.is_ascii_whitespace() || *c == b'.') =>
                    {
                        let kw_end = i + kw.len();
                        let (next, dot) = find_statement_end(bytes, kw_end);
                        let text_end = dot.unwrap_or(next);
                        let text = content[i..text_end].trim_end_matches('.').trim().to_owned();
                        found.push(QuantifierStatement { for_all, text });
                        // Scrub the statement (keep newlines for positions).
                        for byte in &mut scrubbed[i..next] {
                            if *byte != b'\n' {
                                *byte = b' ';
                            }
                        }
                        i = next;
                    }
                    Some((kw, _)) => i += kw.len(),
                    None => i += 1,
                }
            }
            _ => i += 1,
        }
    }

    let scrubbed = String::from_utf8(scrubbed).unwrap_or_else(|_| content.to_owned());
    (scrubbed, found)
}

/// Case-insensitive keyword match at the start of `bytes`.
fn matches_keyword(bytes: &[u8], keyword: &str) -> bool {
    bytes.len() >= keyword.len() && bytes[..keyword.len()].eq_ignore_ascii_case(keyword.as_bytes())
}

/// Find the end of a quantifier statement starting at `from`: the first `.`
/// outside IRIs/strings that is followed by whitespace, `#`, `}` or EOF.
/// Returns `(index after the statement, Some(dot index))`; if no dot is
/// found the statement runs to EOF (`None`).
fn find_statement_end(bytes: &[u8], from: usize) -> (usize, Option<usize>) {
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => {
                while i < bytes.len() && bytes[i] != b'>' && bytes[i] != b'\n' {
                    i += 1;
                }
                i += 1;
            }
            b'"' | b'\'' => i = skip_string(bytes, i),
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'.' => {
                let next = bytes.get(i + 1);
                if next.is_none_or(|c| c.is_ascii_whitespace() || *c == b'#' || *c == b'}') {
                    return (i + 1, Some(i));
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    (bytes.len(), None)
}

/// Skip a quoted string starting at `start` (handles `'''`/`"""` long
/// forms and backslash escapes). Returns the index after the closing quote.
fn skip_string(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let len = bytes.len();
    let triple = start + 2 < len && bytes[start + 1] == quote && bytes[start + 2] == quote;
    let mut i = if triple { start + 3 } else { start + 1 };
    while i < len {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == quote => {
                if triple {
                    if i + 2 < len && bytes[i + 1] == quote && bytes[i + 2] == quote {
                        return i + 3;
                    }
                    i += 1;
                } else {
                    return i + 1;
                }
            }
            b'\n' if !triple => return i, // unterminated short string
            _ => i += 1,
        }
    }
    len
}

// ═══════════════════════════════════════════════════════════════════════════
// Predicate-position path repair (oxttl upstream bug workaround)
// ═══════════════════════════════════════════════════════════════════════════
//
// oxttl 0.2.3 expands paths in *subject*/*object* position correctly (path
// items are node sequences), but in *predicate* position it runs the same
// node-path algorithm and then plugs the final intermediate blank node in
// as the predicate, producing invalid quads such as
//
//     <p> <q> _:m .        (for `a p!q b`)
//     _:m <q> <p> .  <a> _:m <b> .      (for `a p^q b`)
//
// This pass detects the resulting blank-predicate quads ("anchors"),
// reconstructs the group of path-step quads by following the fresh
// intermediate blank nodes backwards, and rewrites the statement into the
// correct expansion
//
//     <a> <p> _:m .  _:m <q> <b> .      (for `a p!q b`)
//     <a> <p> _:m .  <b> <q> _:m .      (for `a p^q b`)
//
// which is also what OWL/RDF consumers expect.  Directions of the individual
// steps are recovered from the *linkage* (which side of a step quad carries
// the previously-seen intermediate blank node), so mixed chains such as
// `p!q^r` are reconstructed exactly.  Paths in subject/object position and
// non-path statements never produce blank predicates and pass through
// untouched.  Groups that cannot be reconstructed (upstream shape drift)
// are dropped with a warning rather than mapped as garbage.

/// Direction of one step in oxttl's path expansion output.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PathDirection {
    Forward,
    Inverse,
}

/// Rewrite predicate-position path quads (see section comment above).
fn repair_path_quads(quads: Vec<N3Quad>) -> Vec<N3Quad> {
    let anchor_indices: Vec<usize> = quads
        .iter()
        .enumerate()
        .filter(|(_, q)| matches!(q.predicate, N3Term::BlankNode(_)))
        .map(|(i, _)| i)
        .collect();
    if anchor_indices.is_empty() {
        return quads;
    }
    let anchor_set: HashSet<usize> = anchor_indices.iter().copied().collect();
    let mut consumed: HashSet<usize> = HashSet::new();
    // Reconstructed groups keyed by the anchor's path-blank predicate, so
    // object lists after a path (`a p!q b, c`) reuse the same group.
    let mut groups: HashMap<String, Vec<(usize, PathDirection)>> = HashMap::new();
    let mut replacements: HashMap<usize, Vec<N3Quad>> = HashMap::new();

    for anchor_idx in &anchor_indices {
        let anchor = &quads[*anchor_idx];
        let N3Term::BlankNode(path_blank) = &anchor.predicate else {
            continue;
        };
        let key = path_blank.as_str().to_owned();

        if let Some(group) = groups.get(&key) {
            // A later object of the same path verb: only the final step is
            // missing (the path steps themselves were already emitted).
            replacements.insert(*anchor_idx, final_step_quads(group, &quads, anchor));
            consumed.insert(*anchor_idx);
            continue;
        }

        // Walk backwards from the anchor through the intermediate blanks.
        // oxttl emits a statement's path quads in order and always before
        // the anchor, so candidates must come strictly earlier in the stream
        // than the previously found one — this also disambiguates a node
        // that appears as "previous" of a later step and "next" of an
        // earlier one.
        let mut group_rev: Vec<(usize, PathDirection)> = Vec::new();
        let mut cur_key = key.clone();
        let mut last_seen = *anchor_idx;
        let mut first_item: Option<N3Term> = None;
        loop {
            let mut candidates: Vec<(usize, PathDirection, &N3Term)> = Vec::new();
            for (j, q) in quads.iter().enumerate() {
                if j >= last_seen
                    || j == *anchor_idx
                    || anchor_set.contains(&j)
                    || consumed.contains(&j)
                    || q.graph_name != anchor.graph_name
                {
                    continue;
                }
                // The sought key sits on the *next* side of its creator:
                // object for a forward step, subject for an inverse step.
                if term_matches_blank(&q.object, &cur_key) {
                    candidates.push((j, PathDirection::Forward, &q.subject));
                }
                if term_matches_blank(&q.subject, &cur_key) {
                    candidates.push((j, PathDirection::Inverse, &q.object));
                }
            }
            match candidates.len() {
                1 => {
                    let (j, direction, prev) = candidates[0];
                    group_rev.push((j, direction));
                    last_seen = j;
                    if let N3Term::BlankNode(prev_blank) = prev {
                        cur_key = prev_blank.as_str().to_owned();
                        continue;
                    }
                    first_item = Some(prev.clone());
                    break;
                }
                // 0 or ≥2 candidates: the group cannot be reconstructed
                // unambiguously (should not happen for oxttl output).
                _ => break,
            }
        }

        let Some(first_item) = first_item else {
            log::warn!(
                "N3: dropping unrepairable path statement (blank predicate `_:{}')",
                key
            );
            consumed.insert(*anchor_idx);
            for (j, _) in group_rev {
                consumed.insert(j);
            }
            continue;
        };

        let mut group: Vec<(usize, PathDirection)> = group_rev;
        group.reverse();
        replacements.insert(
            *anchor_idx,
            reconstruct_path_quads(&group, &quads, anchor, first_item),
        );
        groups.insert(key, group.clone());
        consumed.insert(*anchor_idx);
        for (j, _) in group {
            consumed.insert(j);
        }
    }

    // Rebuild the stream: repaired statements are inserted where the anchor
    // was; consumed group/anchor quads disappear.
    let mut out = Vec::with_capacity(quads.len());
    for (i, quad) in quads.into_iter().enumerate() {
        if let Some(repaired) = replacements.remove(&i) {
            out.extend(repaired);
        } else if !consumed.contains(&i) {
            out.push(quad);
        }
    }
    out
}

fn term_matches_blank(term: &N3Term, key: &str) -> bool {
    matches!(term, N3Term::BlankNode(b) if b.as_str() == key)
}

/// `step1` + the interleaved steps of a reconstructed group.
fn reconstruct_path_quads(
    group: &[(usize, PathDirection)],
    quads: &[N3Quad],
    anchor: &N3Quad,
    first_item: N3Term,
) -> Vec<N3Quad> {
    let graph = anchor.graph_name.clone();
    let mut out = Vec::with_capacity(group.len() + 1);

    // Step 1: the statement subject reaches the first intermediate via the
    // first path item.
    let first = &quads[group[0].0];
    let mut next_term = match group[0].1 {
        PathDirection::Forward => first.object.clone(),
        PathDirection::Inverse => first.subject.clone(),
    };
    out.push(N3Quad {
        subject: anchor.subject.clone(),
        predicate: first_item,
        object: next_term.clone(),
        graph_name: graph.clone(),
    });

    // Middle steps: step j+1 carries the predicate of group quad j and
    // connects the j-th intermediate to the (j+1)-th.
    for window in group.windows(2) {
        let (gi, direction) = window[0];
        let (gj, next_direction) = window[1];
        let next_next = match next_direction {
            PathDirection::Forward => quads[gj].object.clone(),
            PathDirection::Inverse => quads[gj].subject.clone(),
        };
        let (subject, object) = match direction {
            PathDirection::Forward => (next_term.clone(), next_next.clone()),
            PathDirection::Inverse => (next_next.clone(), next_term.clone()),
        };
        out.push(N3Quad {
            subject,
            predicate: quads[gi].predicate.clone(),
            object,
            graph_name: graph.clone(),
        });
        next_term = next_next;
    }

    // Final step: last intermediate reaches the statement object.
    out.extend(final_step_quads(group, quads, anchor));
    out
}

/// The last step of a path group: `<intermediate> <last predicate> <object>`.
fn final_step_quads(
    group: &[(usize, PathDirection)],
    quads: &[N3Quad],
    anchor: &N3Quad,
) -> Vec<N3Quad> {
    // An empty path group contributes no quads (defensive: callers build
    // groups from non-empty path walks).
    let Some(&(gi, direction)) = group.last() else {
        return Vec::new();
    };
    let g = &quads[gi];
    let next_term = match direction {
        PathDirection::Forward => g.object.clone(),
        PathDirection::Inverse => g.subject.clone(),
    };
    let (subject, object) = match direction {
        PathDirection::Forward => (next_term, anchor.object.clone()),
        PathDirection::Inverse => (anchor.object.clone(), next_term),
    };
    vec![N3Quad {
        subject,
        predicate: g.predicate.clone(),
        object,
        graph_name: anchor.graph_name.clone(),
    }]
}

fn quad_has_variable(quad: &N3Quad) -> bool {
    matches!(quad.subject, N3Term::Variable(_))
        || matches!(quad.predicate, N3Term::Variable(_))
        || matches!(quad.object, N3Term::Variable(_))
}

fn blank_id(term: &N3Term) -> Option<String> {
    match term {
        N3Term::BlankNode(b) => Some(b.as_str().to_owned()),
        _ => None,
    }
}

fn n3_iri(node: &NamedNode) -> Result<url::Url> {
    url::Url::parse(node.as_str())
        .map_err(|e| Error::ParseError(format!("N3: invalid IRI '{}': {e}", node.as_str())))
}

fn n3_term_to_rdf_term(term: &N3Term) -> Result<RdfTerm> {
    match term {
        N3Term::NamedNode(node) => Ok(RdfTerm::Iri(n3_iri(node)?)),
        N3Term::BlankNode(b) => Ok(RdfTerm::BlankNode(b.as_str().to_owned())),
        N3Term::Literal(lit) => {
            let language = lit.language().map(str::to_owned);
            let datatype =
                if language.is_some() {
                    None
                } else {
                    Some(url::Url::parse(lit.datatype().as_str()).map_err(|e| {
                        Error::ParseError(format!("N3: invalid literal datatype: {e}"))
                    })?)
                };
            Ok(RdfTerm::Literal {
                value: lit.value().to_owned(),
                datatype,
                language,
                direction: None,
            })
        }
        other => Err(Error::ParseError(format!(
            "N3: unsupported RDF term in ground triple: {other}"
        ))),
    }
}

fn n3_quad_to_triple(quad: &N3Quad) -> Result<Triple> {
    let subject = match &quad.subject {
        N3Term::NamedNode(node) => RdfTerm::Iri(n3_iri(node)?),
        N3Term::BlankNode(b) => RdfTerm::BlankNode(b.as_str().to_owned()),
        other => {
            return Err(Error::ParseError(format!(
                "N3: unsupported subject term: {other}"
            )));
        }
    };
    let predicate = match &quad.predicate {
        N3Term::NamedNode(node) => RdfTerm::Iri(n3_iri(node)?),
        other => {
            return Err(Error::ParseError(format!(
                "N3: unsupported predicate term: {other}"
            )));
        }
    };
    Ok(Triple {
        subject,
        predicate,
        object: n3_term_to_rdf_term(&quad.object)?,
    })
}

/// Append an ontology-level annotation carrying preserved N3 text.
fn push_preserved_annotation(ontology: &mut Ontology, property: &str, text: String) {
    ontology.annotations.push(Annotation {
        property: AnnotationProperty {
            iri: IRI::new(property),
        },
        value: AnnotationValue::Literal(Literal::new(text)),
        annotations: vec![],
    });
}

/// Fresh axiom id for this parser (mirrors the per-parser static counters
/// used by `turtle.rs` / `functional.rs`).
fn generate_axiom_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ═══════════════════════════════════════════════════════════════════════════
// SWRL conversion of formulae
// ═══════════════════════════════════════════════════════════════════════════

/// Mint the SWRL variable IRI for an N3 variable name.
fn swrl_variable(name: &str) -> SWRLVariable {
    SWRLVariable::new(IRI::new(&format!("{N3_VARIABLE_IRI_PREFIX}{name}")))
}

/// Recover the variable name from a SWRL variable IRI minted by this parser.
/// Returns `None` for foreign variable IRIs (functional-syntax rules saved
/// as N3) — those are rendered as a comment instead.
fn n3_variable_name(var: &SWRLVariable) -> Option<&str> {
    var.iri.as_str().strip_prefix(N3_VARIABLE_IRI_PREFIX)
}

/// RDF 1.1: simple literals and explicit `xsd:string` are the same value;
/// the OWL model stores them with `datatype: None` (same normalization the
/// shared RDF→OWL mapping applies).
fn normalize_literal(lit: &Literal) -> Literal {
    let mut out = lit.clone();
    if out.language.is_none()
        && out
            .datatype
            .as_ref()
            .is_some_and(|d| d.as_str() == XSD_STRING)
    {
        out.datatype = None;
    }
    out
}

fn n3_literal_to_model(lit: &OxLiteral) -> Result<Literal> {
    let language = lit.language().map(str::to_owned);
    let datatype = if language.is_some() {
        None
    } else {
        Some(
            url::Url::parse(lit.datatype().as_str())
                .map_err(|e| Error::ParseError(format!("N3: invalid literal datatype: {e}")))?,
        )
    };
    Ok(normalize_literal(&Literal {
        value: lit.value().to_owned(),
        language,
        datatype,
    }))
}

/// Translate one side (a formula's triples) into SWRL atoms.
/// `None` = not SWRL-expressible (caller falls back to `log:implies` data).
fn side_to_atoms(
    quads: &[N3Quad],
    formulae: &HashMap<String, Vec<N3Quad>>,
    ontology: &Ontology,
) -> Option<Vec<SWRLAtom>> {
    let mut atoms = Vec::with_capacity(quads.len());
    for quad in quads {
        atoms.push(quad_to_atom(quad, formulae, ontology)?);
    }
    Some(atoms)
}

fn quad_to_atom(
    quad: &N3Quad,
    formulae: &HashMap<String, Vec<N3Quad>>,
    ontology: &Ontology,
) -> Option<SWRLAtom> {
    let N3Term::NamedNode(predicate) = &quad.predicate else {
        return None; // variable / non-IRI predicate
    };
    let predicate_iri = predicate.as_str();
    if matches!(predicate_iri, RDF_FIRST | RDF_REST | RDF_NIL)
        || rdf_to_owl::is_structural_owl_predicate(predicate_iri)
    {
        return None; // list / structural structure is not expressible
    }
    let first = term_to_i_argument(&quad.subject)?;

    if predicate_iri == RDF_TYPE {
        let N3Term::NamedNode(class) = &quad.object else {
            return None; // `?x rdf:type ?c` or a blank/nested formula class
        };
        return Some(SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new(class.as_str()))),
            argument: first,
        });
    }
    if predicate_iri == OWL_SAME_AS {
        return Some(SWRLAtom::SameIndividualAtom {
            first_argument: first,
            second_argument: term_to_i_argument(&quad.object)?,
        });
    }
    if predicate_iri == OWL_DIFFERENT_FROM {
        return Some(SWRLAtom::DifferentIndividualsAtom {
            first_argument: first,
            second_argument: term_to_i_argument(&quad.object)?,
        });
    }

    match &quad.object {
        N3Term::Literal(lit) => Some(SWRLAtom::DataPropertyAtom {
            predicate: DataPropertyExpression::DataProperty(DataProperty {
                iri: IRI::new(predicate_iri),
            }),
            first_argument: first,
            second_argument: SWRLDArgument::Literal(n3_literal_to_model(lit).ok()?),
        }),
        N3Term::Variable(var) => {
            if rdf_to_owl::is_data_property(predicate_iri, ontology) {
                Some(SWRLAtom::DataPropertyAtom {
                    predicate: DataPropertyExpression::DataProperty(DataProperty {
                        iri: IRI::new(predicate_iri),
                    }),
                    first_argument: first,
                    second_argument: SWRLDArgument::Variable(swrl_variable(var.as_str())),
                })
            } else {
                Some(SWRLAtom::ObjectPropertyAtom {
                    predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: IRI::new(predicate_iri),
                    }),
                    first_argument: first,
                    second_argument: SWRLIArgument::Variable(swrl_variable(var.as_str())),
                })
            }
        }
        N3Term::NamedNode(node) => Some(SWRLAtom::ObjectPropertyAtom {
            predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                iri: IRI::new(predicate_iri),
            }),
            first_argument: first,
            second_argument: SWRLIArgument::Individual(Individual::Named(NamedIndividual {
                iri: IRI::new(node.as_str()),
            })),
        }),
        N3Term::BlankNode(b) => {
            if formulae.contains_key(b.as_str()) {
                None // nested formula — not SWRL-expressible
            } else {
                Some(SWRLAtom::ObjectPropertyAtom {
                    predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: IRI::new(predicate_iri),
                    }),
                    first_argument: first,
                    second_argument: SWRLIArgument::Individual(Individual::Anonymous(
                        AnonymousIndividual {
                            id: b.as_str().to_owned(),
                        },
                    )),
                })
            }
        }
    }
}

fn term_to_i_argument(term: &N3Term) -> Option<SWRLIArgument> {
    match term {
        N3Term::Variable(var) => Some(SWRLIArgument::Variable(swrl_variable(var.as_str()))),
        N3Term::NamedNode(node) => Some(SWRLIArgument::Individual(Individual::Named(
            NamedIndividual {
                iri: IRI::new(node.as_str()),
            },
        ))),
        N3Term::BlankNode(b) => Some(SWRLIArgument::Individual(Individual::Anonymous(
            AnonymousIndividual {
                id: b.as_str().to_owned(),
            },
        ))),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Rendering (used by the preserved-annotation values and the serializer)
// ═══════════════════════════════════════════════════════════════════════════

fn escape_n3_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c => out.push(c),
        }
    }
    out
}

fn render_literal_parts(value: &str, language: Option<&str>, datatype: &str) -> String {
    let escaped = escape_n3_string(value);
    if let Some(lang) = language {
        format!("\"{escaped}\"@{lang}")
    } else if datatype == XSD_STRING || datatype == rdf_to_owl::RDFS_LITERAL {
        format!("\"{escaped}\"")
    } else {
        format!("\"{escaped}\"^^<{datatype}>")
    }
}

/// Render an OWL-model literal as N3.
fn render_model_literal(lit: &Literal) -> String {
    render_literal_parts(
        &lit.value,
        lit.language.as_deref(),
        lit.datatype.as_ref().map_or(XSD_STRING, |d| d.as_str()),
    )
}

/// Render an N3 term as N3 text (full IRIs — no prefix dependence, so
/// preserved annotation values are self-contained).
fn render_term(term: &N3Term) -> String {
    match term {
        N3Term::Literal(lit) => {
            render_literal_parts(lit.value(), lit.language(), lit.datatype().as_str())
        }
        other => format!("{other}"), // NamedNode → `<iri>`, BlankNode → `_:b`, Variable → `?x`
    }
}

fn render_quad(quad: &N3Quad) -> String {
    format!(
        "{} {} {}",
        render_term(&quad.subject),
        render_term(&quad.predicate),
        render_term(&quad.object)
    )
}

/// Render `{ s p o . … }` for a formula's member quads.
fn render_formula(members: &[N3Quad]) -> String {
    if members.is_empty() {
        return "{ }".to_owned();
    }
    let mut out = String::from("{ ");
    for (i, quad) in members.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&render_quad(quad));
        out.push_str(" .");
    }
    out.push_str(" }");
    out
}

fn render_side(term: &N3Term, formulae: &HashMap<String, Vec<N3Quad>>) -> String {
    if let Some(id) = blank_id(term)
        && let Some(members) = formulae.get(&id)
    {
        return render_formula(members);
    }
    render_term(term)
}

/// Canonical `{ body } => { head }` text for a preserved implication.
fn render_implication(
    body: &N3Term,
    head: &N3Term,
    formulae: &HashMap<String, Vec<N3Quad>>,
) -> String {
    format!(
        "{} => {}",
        render_side(body, formulae),
        render_side(head, formulae)
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Serializer
// ═══════════════════════════════════════════════════════════════════════════

/// N3 serializer: Turtle (a subset of N3) for the axiom triples, plus
/// `{ … } => { … }` statements for rules and verbatim preserved annotations.
#[derive(Debug, Clone, Default)]
pub struct N3Renderer;

impl N3Renderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        // ── 1. Ontology → RDF triples via the shared OWL→RDF emission ────
        let mut counter = BlankNodeCounter::new();
        let mut triples: Vec<Triple> = Vec::new();
        // Annotations are attached to the ontology IRI, or — when the
        // ontology has none — to the well-known fallback subject so they
        // survive the round-trip instead of being dropped (CR-011 M2).
        let annotation_subject = match ontology.get_iri() {
            Some(iri) => {
                let subject = RdfTerm::Iri(
                    url::Url::parse(iri.as_str())
                        .map_err(|e| Error::ontology_parsing(format!("N3: ontology IRI: {e}")))?,
                );
                triples.push(Triple {
                    subject: subject.clone(),
                    predicate: RdfTerm::Iri(
                        url::Url::parse(RDF_TYPE)
                            .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?,
                    ),
                    object: RdfTerm::Iri(
                        url::Url::parse(OWL_ONTOLOGY)
                            .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?,
                    ),
                });
                if let Some(version) = ontology.id.get_version_iri() {
                    triples.push(Triple {
                        subject: subject.clone(),
                        predicate: RdfTerm::Iri(
                            url::Url::parse(OWL_VERSION_IRI)
                                .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?,
                        ),
                        object: RdfTerm::Iri(
                            url::Url::parse(version.as_str())
                                .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?,
                        ),
                    });
                }
                subject
            }
            None => RdfTerm::Iri(
                url::Url::parse(crate::semantics::owl_rdf_mapping::ONTOLOGY_FALLBACK_IRI).map_err(
                    |e| Error::ontology_parsing(format!("N3: fallback subject IRI: {e}")),
                )?,
            ),
        };
        for annotation in &ontology.annotations {
            if is_n3_preservation_property(annotation.property.iri.as_str()) {
                continue; // re-emitted verbatim below
            }
            triples.push(Triple {
                subject: annotation_subject.clone(),
                predicate: RdfTerm::Iri(
                    url::Url::parse(annotation.property.iri.as_str()).map_err(|e| {
                        Error::ontology_parsing(format!("N3: annotation property IRI: {e}"))
                    })?,
                ),
                object: annotation_value_to_term(&annotation.value)?,
            });
        }
        for axiom in ontology.axioms() {
            triples.extend(axiom_to_triples(axiom, &mut counter));
        }

        // ── 2. Prefix declarations (valid ones only) ──────────────────────
        let mut prefix_lines = String::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        let mut seen_iris: HashSet<String> = HashSet::new();
        let standard = [
            ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
            ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
            ("owl", "http://www.w3.org/2002/07/owl#"),
            ("xsd", "http://www.w3.org/2001/XMLSchema#"),
        ];
        let mut all_prefixes: Vec<(String, String)> = ontology
            .prefixes
            .iter()
            .map(|(n, i)| (n.clone(), i.as_str().to_owned()))
            .collect();
        all_prefixes.sort();
        for (name, iri) in standard {
            if !all_prefixes.iter().any(|(n, i)| n == name || i == iri) {
                all_prefixes.push((name.to_owned(), iri.to_owned()));
            }
        }
        for (name, iri) in &all_prefixes {
            if !valid_prefix_name(name) || !valid_prefix_iri(iri) {
                continue;
            }
            if !seen_names.insert(name.clone()) || !seen_iris.insert(iri.clone()) {
                continue;
            }
            prefix_lines.push_str(&format!("@prefix {name}: <{iri}> .\n"));
        }

        // ── 3. Serialize the axiom triples with oxttl's Turtle serializer ─
        let serializer = OxTurtleSerializer::new();
        let mut writer = serializer.for_writer(Vec::new());
        for triple in &triples {
            let ox_triple = to_oxrdf_triple(triple)?;
            writer
                .serialize_triple(TripleRef::from(&ox_triple))
                .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?;
        }
        let bytes = writer
            .finish()
            .map_err(|e| Error::ontology_parsing(format!("N3: {e}")))?;
        let body = String::from_utf8(bytes)
            .map_err(|e| Error::ontology_parsing(format!("N3: invalid serializer output: {e}")))?;

        // ── 4. Rules as `{ … } => { … }` statements ──────────────────────
        let mut extras = String::new();
        for axiom in ontology.axioms() {
            if let Axiom::Rule(rule_axiom) = axiom {
                match render_swrl_rule(&rule_axiom.rule) {
                    Some(text) => extras.push_str(&format!("{text} .\n")),
                    None => extras.push_str(&format!(
                        "# oxidowl: SWRL rule not representable as N3: {:?}\n",
                        rule_axiom.rule
                    )),
                }
            }
        }

        // ── 5. Preserved N3 data, verbatim ────────────────────────────────
        for annotation in &ontology.annotations {
            if is_n3_preservation_property(annotation.property.iri.as_str())
                && let AnnotationValue::Literal(lit) = &annotation.value
            {
                let text = lit.value.trim();
                if !text.is_empty() {
                    extras.push_str(text);
                    extras.push_str(" .\n");
                }
            }
        }

        Ok(format!("{prefix_lines}{body}{extras}"))
    }
}

/// Save an N3 ontology to a file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = N3Renderer::new().serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| Error::io(format!("N3: {e}")))
}

fn annotation_value_to_term(value: &AnnotationValue) -> Result<RdfTerm> {
    match value {
        AnnotationValue::IRI(iri) => Ok(RdfTerm::Iri(
            url::Url::parse(iri.as_str())
                .map_err(|e| Error::ontology_parsing(format!("N3: annotation IRI: {e}")))?,
        )),
        AnnotationValue::AnonymousIndividual(anon) => Ok(RdfTerm::BlankNode(anon.id.clone())),
        AnnotationValue::Literal(lit) => Ok(RdfTerm::Literal {
            value: lit.value.clone(),
            datatype: lit.datatype.clone(),
            language: lit.language.clone(),
            direction: None,
        }),
    }
}

fn valid_prefix_name(name: &str) -> bool {
    if name.is_empty() {
        return true; // default `:` prefix
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn valid_prefix_iri(iri: &str) -> bool {
    !iri.is_empty()
        && !iri.contains(['<', '>', '"', '{', '}', '|', '^', '`'])
        && !iri.chars().any(char::is_whitespace)
}

// ── SWRL → N3 rendering ───────────────────────────────────────────────────

fn render_swrl_i_argument(arg: &SWRLIArgument) -> Option<String> {
    match arg {
        SWRLIArgument::Variable(var) => Some(render_swrl_variable(var)),
        SWRLIArgument::Individual(Individual::Named(n)) => Some(format!("<{}>", n.iri.as_str())),
        SWRLIArgument::Individual(Individual::Anonymous(a)) => Some(format!("_:{}", a.id)),
    }
}

fn render_swrl_d_argument(arg: &SWRLDArgument) -> Option<String> {
    match arg {
        SWRLDArgument::Variable(var) => Some(render_swrl_variable(var)),
        SWRLDArgument::Literal(lit) => Some(render_model_literal(lit)),
    }
}

/// Render a SWRL variable. Variables minted by this parser render as
/// `?name`; foreign variable IRIs (functional-syntax rules saved as N3)
/// render as `?<sanitized local part>` — an approximation documented in the
/// module doc.
fn render_swrl_variable(var: &SWRLVariable) -> String {
    if let Some(name) = n3_variable_name(var) {
        return format!("?{name}");
    }
    let local = var
        .iri
        .as_str()
        .rsplit(['#', '/', ':'])
        .find(|s| !s.is_empty())
        .unwrap_or("var");
    let sanitized: String = local
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("?{sanitized}")
}

fn render_restriction(
    on_property: &ObjectProperty,
    filler_predicate: &str,
    filler: &str,
) -> String {
    format!(
        "[ a <http://www.w3.org/2002/07/owl#Restriction> ; \
         <http://www.w3.org/2002/07/owl#onProperty> <{}> ; \
         <{filler_predicate}> {filler} ]",
        on_property.iri.as_str()
    )
}

fn render_class_expression(ce: &ClassExpression) -> Option<String> {
    match ce {
        ClassExpression::Class(c) => Some(format!("<{}>", c.iri.as_str())),
        ClassExpression::ObjectSomeValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(p),
            filler,
        } => match filler.as_ref() {
            ClassExpression::Class(c) => Some(render_restriction(
                p,
                "http://www.w3.org/2002/07/owl#someValuesFrom",
                &format!("<{}>", c.iri.as_str()),
            )),
            _ => None,
        },
        ClassExpression::ObjectAllValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(p),
            filler,
        } => match filler.as_ref() {
            ClassExpression::Class(c) => Some(render_restriction(
                p,
                "http://www.w3.org/2002/07/owl#allValuesFrom",
                &format!("<{}>", c.iri.as_str()),
            )),
            _ => None,
        },
        ClassExpression::ObjectHasValue {
            property: ObjectPropertyExpression::ObjectProperty(p),
            value,
        } => {
            let term = match value {
                Individual::Named(n) => format!("<{}>", n.iri.as_str()),
                Individual::Anonymous(a) => format!("_:{}", a.id),
            };
            Some(render_restriction(
                p,
                "http://www.w3.org/2002/07/owl#hasValue",
                &term,
            ))
        }
        ClassExpression::ObjectHasSelf {
            property: ObjectPropertyExpression::ObjectProperty(p),
        } => Some(render_restriction(
            p,
            "http://www.w3.org/2002/07/owl#hasSelf",
            "true",
        )),
        ClassExpression::ObjectMinCardinality {
            property: ObjectPropertyExpression::ObjectProperty(p),
            cardinality,
            filler,
        } => render_cardinality_restriction(p, "min", *cardinality, filler),
        ClassExpression::ObjectMaxCardinality {
            property: ObjectPropertyExpression::ObjectProperty(p),
            cardinality,
            filler,
        } => render_cardinality_restriction(p, "max", *cardinality, filler),
        ClassExpression::ObjectExactCardinality {
            property: ObjectPropertyExpression::ObjectProperty(p),
            cardinality,
            filler,
        } => render_cardinality_restriction(p, "exact", *cardinality, filler),
        _ => None,
    }
}

fn render_cardinality_restriction(
    property: &ObjectProperty,
    kind: &str,
    cardinality: u32,
    filler: &ClassExpression,
) -> Option<String> {
    let on_property = format!(
        "<http://www.w3.org/2002/07/owl#onProperty> <{}>",
        property.iri.as_str()
    );
    let cardinality_predicate = match kind {
        "min" => "minQualifiedCardinality",
        "max" => "maxQualifiedCardinality",
        _ => "qualifiedCardinality",
    };
    let cardinality_term =
        format!("\"{cardinality}\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>");
    match filler {
        ClassExpression::Class(c) if c.iri.as_str() == rdf_to_owl::OWL_THING => {
            let unqualified = match kind {
                "min" => "minCardinality",
                "max" => "maxCardinality",
                _ => "cardinality",
            };
            Some(format!(
                "[ a <http://www.w3.org/2002/07/owl#Restriction> ; {on_property} ; \
                 <http://www.w3.org/2002/07/owl#{unqualified}> {cardinality_term} ]"
            ))
        }
        ClassExpression::Class(c) => Some(format!(
            "[ a <http://www.w3.org/2002/07/owl#Restriction> ; {on_property} ; \
             <http://www.w3.org/2002/07/owl#{cardinality_predicate}> {cardinality_term} ; \
             <http://www.w3.org/2002/07/owl#onClass> <{}> ]",
            c.iri.as_str()
        )),
        _ => None,
    }
}

/// Render one SWRL atom as one (or, for property chains, several) N3
/// statement(s) without the trailing dot. `None` = not representable.
fn render_swrl_atom(atom: &SWRLAtom) -> Option<String> {
    match atom {
        SWRLAtom::ClassAtom {
            predicate,
            argument,
        } => {
            let subject = render_swrl_i_argument(argument)?;
            let class = render_class_expression(predicate)?;
            Some(format!("{subject} a {class}"))
        }
        SWRLAtom::ObjectPropertyAtom {
            predicate,
            first_argument,
            second_argument,
        } => {
            let first = render_swrl_i_argument(first_argument)?;
            let second = render_swrl_i_argument(second_argument)?;
            match predicate {
                ObjectPropertyExpression::ObjectProperty(p) => {
                    Some(format!("{first} <{}> {second}", p.iri.as_str()))
                }
                ObjectPropertyExpression::InverseObjectProperty(p) => {
                    // P⁻(x,y) ≡ P(y,x)
                    Some(format!("{second} <{}> {first}", p.iri.as_str()))
                }
                ObjectPropertyExpression::PropertyChain(chain) => {
                    if chain.is_empty() {
                        return None;
                    }
                    let mut parts = vec![first];
                    for (i, link) in chain.iter().enumerate() {
                        let ObjectPropertyExpression::ObjectProperty(p) = link else {
                            return None; // nested chains/inverses: give up
                        };
                        if i + 1 == chain.len() {
                            parts.push(format!("<{}>", p.iri.as_str()));
                            parts.push(second.clone());
                        } else {
                            static CHAIN_COUNTER: AtomicU64 = AtomicU64::new(0);
                            let fresh = CHAIN_COUNTER.fetch_add(1, Ordering::Relaxed);
                            parts.push(format!("<{}>", p.iri.as_str()));
                            parts.push(format!("_:swrlchain{fresh}"));
                        }
                    }
                    Some(parts.join(" "))
                }
            }
        }
        SWRLAtom::DataPropertyAtom {
            predicate: DataPropertyExpression::DataProperty(p),
            first_argument,
            second_argument,
        } => {
            let first = render_swrl_i_argument(first_argument)?;
            let second = render_swrl_d_argument(second_argument)?;
            Some(format!("{first} <{}> {second}", p.iri.as_str()))
        }
        SWRLAtom::DataRangeAtom {
            predicate,
            argument,
        } => {
            let DataRange::Datatype(dt) = predicate else {
                return None;
            };
            let subject = render_swrl_d_argument(argument)?;
            Some(format!("{subject} a <{}>", dt.as_str()))
        }
        SWRLAtom::SameIndividualAtom {
            first_argument,
            second_argument,
        } => {
            let first = render_swrl_i_argument(first_argument)?;
            let second = render_swrl_i_argument(second_argument)?;
            Some(format!("{first} <{OWL_SAME_AS}> {second}"))
        }
        SWRLAtom::DifferentIndividualsAtom {
            first_argument,
            second_argument,
        } => {
            let first = render_swrl_i_argument(first_argument)?;
            let second = render_swrl_i_argument(second_argument)?;
            Some(format!("{first} <{OWL_DIFFERENT_FROM}> {second}"))
        }
        // SWRL built-ins have no triple form in N3.
        SWRLAtom::BuiltInAtom { .. } => None,
    }
}

/// Render a whole SWRL rule as `{ body } => { head }` (no trailing dot).
/// `None` = some atom is not representable (caller emits a comment).
fn render_swrl_rule(rule: &SWRLRule) -> Option<String> {
    let mut body = Vec::with_capacity(rule.body.len());
    for atom in &rule.body {
        body.push(render_swrl_atom(atom)?);
    }
    let mut head = Vec::with_capacity(rule.head.len());
    for atom in &rule.head {
        head.push(render_swrl_atom(atom)?);
    }
    Some(format!(
        "{} => {}",
        render_atom_list(&body),
        render_atom_list(&head)
    ))
}

/// `{ a . b . }` for a list of rendered atoms; `{ }` when empty.
fn render_atom_list(atoms: &[String]) -> String {
    if atoms.is_empty() {
        return "{ }".to_owned();
    }
    format!("{{ {} . }}", atoms.join(" . "))
}

// ── semantics::Triple → oxrdf::Triple ────────────────────────────────────

fn to_oxrdf_term(term: &RdfTerm) -> Result<oxrdf::Term> {
    match term {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str())
            .map(Into::into)
            .map_err(|e| Error::ontology_parsing(format!("N3: invalid IRI '{url}': {e}"))),
        RdfTerm::BlankNode(id) => oxrdf::BlankNode::new(id.as_str())
            .map(Into::into)
            .map_err(|e| {
                Error::ontology_parsing(format!("N3: invalid blank node label '{id}': {e}"))
            }),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                OxLiteral::new_language_tagged_literal(value.as_str(), lang.as_str()).map_err(
                    |e| Error::ontology_parsing(format!("N3: invalid language tag: {e}")),
                )?
            } else if let Some(dt) = datatype {
                let node = NamedNode::new(dt.as_str()).map_err(|e| {
                    Error::ontology_parsing(format!("N3: invalid literal datatype '{dt}': {e}"))
                })?;
                OxLiteral::new_typed_literal(value.as_str(), node)
            } else {
                OxLiteral::new_simple_literal(value.as_str())
            };
            Ok(lit.into())
        }
        other => Err(Error::ontology_parsing(format!(
            "N3: unsupported term for serialization: {other}"
        ))),
    }
}

fn to_oxrdf_triple(triple: &Triple) -> Result<oxrdf::Triple> {
    let subject = match &triple.subject {
        RdfTerm::Iri(url) => oxrdf::NamedOrBlankNode::NamedNode(
            NamedNode::new(url.as_str())
                .map_err(|e| Error::ontology_parsing(format!("N3: subject IRI: {e}")))?,
        ),
        RdfTerm::BlankNode(id) => oxrdf::NamedOrBlankNode::BlankNode(
            oxrdf::BlankNode::new(id.as_str())
                .map_err(|e| Error::ontology_parsing(format!("N3: subject blank node: {e}")))?,
        ),
        other => {
            return Err(Error::ontology_parsing(format!(
                "N3: unsupported subject term: {other}"
            )));
        }
    };
    let predicate = match &triple.predicate {
        RdfTerm::Iri(url) => NamedNode::new(url.as_str())
            .map_err(|e| Error::ontology_parsing(format!("N3: predicate IRI: {e}")))?,
        other => {
            return Err(Error::ontology_parsing(format!(
                "N3: unsupported predicate term: {other}"
            )));
        }
    };
    Ok(oxrdf::Triple {
        subject,
        predicate,
        object: to_oxrdf_term(&triple.object)?,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantifier_statements_are_scrubbed_and_preserved() {
        let doc = concat!(
            "@prefix ex: <http://example.org/> .\n",
            "# @forAll inside a comment stays a comment\n",
            "\"@forAll :x\" stays a string .\n",
            "@forAll :a :b .\n",
            "ex:i ex:knows ex:j .\n",
            "@FORSOME ex:k .\n",
        );
        let (scrubbed, found) = extract_quantifier_statements(doc);
        assert_eq!(found.len(), 2, "two quantifier statements: {found:?}");
        assert!(found[0].for_all, "first is @forAll");
        assert_eq!(found[0].text, "@forAll :a :b");
        assert!(!found[1].for_all, "second is @forSome");
        assert_eq!(found[1].text, "@FORSOME ex:k");
        assert!(
            scrubbed.contains("\"@forAll :x\" stays a string"),
            "quantifier text inside a string must not be scrubbed"
        );
        assert!(
            scrubbed.contains("# @forAll inside a comment stays a comment"),
            "quantifier text inside a comment must not be scrubbed"
        );
        assert!(
            !scrubbed.contains("@forAll :a"),
            "the statement itself must be scrubbed: {scrubbed:?}"
        );
        assert_eq!(
            scrubbed.lines().count(),
            doc.lines().count(),
            "line numbers must be preserved"
        );
    }

    #[test]
    fn n3_parse_a_shorthand_maps_to_rdf_type() {
        let ont = parse(concat!(
            "@prefix ex: <http://example.org/> .\n",
            "ex:A a ex:Named .\n",
        ))
        .expect("parse");
        assert!(
            ont.axioms()
                .iter()
                .any(|ax| matches!(ax, Axiom::ClassAssertion(_))),
            "`a` shorthand must map to rdf:type; axioms: {:?}",
            ont.axioms()
        );
    }

    #[test]
    fn render_swrl_rule_produces_reparsable_text() {
        let var_x = swrl_variable("x");
        let rule = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/B"))),
                argument: SWRLIArgument::Variable(var_x.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/A"))),
                argument: SWRLIArgument::Variable(var_x),
            }],
        );
        let text = render_swrl_rule(&rule).expect("render");
        assert_eq!(
            text,
            "{ ?x a <http://example.org/A> . } => { ?x a <http://example.org/B> . }"
        );
    }

    // ── Path repair ────────────────────────────────────────────────────────

    fn nn(iri: &str) -> N3Term {
        N3Term::NamedNode(NamedNode::new_unchecked(iri))
    }
    fn bb(id: &str) -> N3Term {
        N3Term::BlankNode(oxrdf::BlankNode::new_unchecked(id))
    }
    fn q(s: N3Term, p: N3Term, o: N3Term) -> N3Quad {
        N3Quad {
            subject: s,
            predicate: p,
            object: o,
            graph_name: GraphName::DefaultGraph,
        }
    }

    const A: &str = "http://example.org/a";
    const B: &str = "http://example.org/b";
    const P: &str = "http://example.org/p";
    const QQ: &str = "http://example.org/q";
    const R: &str = "http://example.org/r";
    const D_END: &str = "http://example.org/d";

    #[test]
    fn path_repair_forward_chain() {
        // `a p!q b` — oxttl emits (p q m) (a m b).
        let raw = vec![q(nn(P), nn(QQ), bb("m")), q(nn(A), bb("m"), nn(B))];
        let repaired = repair_path_quads(raw);
        assert_eq!(
            repaired,
            vec![q(nn(A), nn(P), bb("m")), q(bb("m"), nn(QQ), nn(B)),],
            "a p!q b must expand to `a p _:m . _:m q b`"
        );
    }

    #[test]
    fn path_repair_inverse_chain() {
        // `a p^q b` — oxttl emits (m q p) (a m b).
        let raw = vec![q(bb("m"), nn(QQ), nn(P)), q(nn(A), bb("m"), nn(B))];
        let repaired = repair_path_quads(raw);
        assert_eq!(
            repaired,
            vec![q(nn(A), nn(P), bb("m")), q(nn(B), nn(QQ), bb("m")),],
            "a p^q b must expand to `a p _:m . b q _:m`"
        );
    }

    #[test]
    fn path_repair_mixed_chain() {
        // `a p!q^r b` — oxttl emits (p q m1) (m2 r m1) (a m2 b).
        let raw = vec![
            q(nn(P), nn(QQ), bb("m1")),
            q(bb("m2"), nn(R), bb("m1")),
            q(nn(A), bb("m2"), nn(B)),
        ];
        let repaired = repair_path_quads(raw);
        assert_eq!(
            repaired,
            vec![
                q(nn(A), nn(P), bb("m1")),
                q(bb("m1"), nn(QQ), bb("m2")),
                q(nn(B), nn(R), bb("m2")),
            ],
            "a p!q^r b must expand to `a p _:m1 . _:m1 q _:m2 . b r _:m2`"
        );
    }

    #[test]
    fn path_repair_object_list_reuses_group() {
        // `a p!q b, c` — two anchors share the same path blank node.
        let raw = vec![
            q(nn(P), nn(QQ), bb("m")),
            q(nn(A), bb("m"), nn(B)),
            q(nn(A), bb("m"), nn("c-node")),
        ];
        let repaired = repair_path_quads(raw);
        assert_eq!(
            repaired,
            vec![
                q(nn(A), nn(P), bb("m")),
                q(bb("m"), nn(QQ), nn(B)),
                q(bb("m"), nn(QQ), nn("c-node")),
            ],
        );
    }

    #[test]
    fn path_repair_leaves_other_quads_untouched() {
        // Subject-position paths are correct upstream and must not change.
        let raw = vec![q(nn(A), nn(B), bb("m")), q(bb("m"), nn(P), nn(R))];
        let repaired = repair_path_quads(raw.clone());
        assert_eq!(repaired, raw);
        // No anchors → identity for everything else too.
        let plain = vec![q(nn(A), nn(P), nn(B))];
        assert_eq!(repair_path_quads(plain.clone()), plain);
    }

    #[test]
    fn path_quads_map_to_axioms_end_to_end() {
        let ont = parse(concat!(
            "@prefix ex: <http://example.org/> .\n",
            "ex:a ex:p!ex:q ex:b .\n",
            "ex:c ex:p^ex:q ex:d .\n",
        ))
        .expect("parse");
        // Each path expands to two ObjectPropertyAssertions bridged by an
        // anonymous intermediate individual.
        let paths: Vec<_> = ont
            .axioms()
            .iter()
            .filter_map(|ax| match ax {
                Axiom::ObjectPropertyAssertion(o) => Some(o.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(paths.len(), 4, "two paths → four assertions: {paths:?}");
        assert!(
            paths.iter().all(|o| {
                matches!(o.source, Individual::Anonymous(_))
                    || matches!(o.target, Individual::Anonymous(_))
            }),
            "every path assertion must involve the anonymous intermediate; got {paths:?}"
        );
        // The endpoints survive: a→…→b and the inverse path anchored at c/d
        // (`c p^q d` expands to `c p _:m` + `d q _:m`).
        assert!(
            paths
                .iter()
                .any(|o| matches!(&o.source, Individual::Named(n)
                if n.iri.as_str() == A)),
            "path start `a` must be the subject of the first step"
        );
        assert!(
            paths
                .iter()
                .any(|o| matches!(&o.source, Individual::Named(n)
                if n.iri.as_str() == D_END)),
            "the inverse step must be anchored at `d` as its subject"
        );
        assert!(
            paths
                .iter()
                .any(|o| matches!(&o.target, Individual::Named(n)
                if n.iri.as_str() == B)),
            "path end `b` must be reached by the forward step"
        );
    }
}
