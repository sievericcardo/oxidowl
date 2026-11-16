//! DL-safe SWRL forward-chaining materialization.
//!
//! [`SwrlMaterializer`] computes the fixpoint of every [`Axiom::Rule`] in an
//! ontology over its *named-individual* `ABox`, in the same spirit as `HermiT`'s
//! DL-safe rule evaluation: the result is a set of new `ABox` assertion axioms
//! that can be merged into the ontology before classification/realisation.
//!
//! # Semantics
//!
//! 1. **Fact collection** — `ClassAssertion`, `ObjectPropertyAssertion`,
//!    `DataPropertyAssertion`, `SameIndividual` (union-find) and
//!    `DifferentIndividuals` axioms are indexed, together with every rule
//!    ([`Axiom::Rule`]) in the ontology.
//! 2. **Cheap asserted closures** — named/named `SubClassOf` and
//!    `EquivalentClasses` give the class-superclass closure;
//!    `SubObjectPropertyOf`, `EquivalentObjectProperties` and
//!    `InverseObjectProperties` give a *signed* object-property superproperty
//!    closure (a property expression is normalised to `(base IRI, polarity)`,
//!    so `inv(P)` is handled without duplicating the fact index);
//!    `SubDataPropertyOf` / `EquivalentDataProperties` give the data-property
//!    closure.  The closures are asserted-only: no tableau reasoning is used.
//! 3. **Body matching** — every rule body is matched by backtracking over the
//!    body atoms *in order*, producing variable bindings.  See
//!    [`SwrlMaterializer`] for the per-atom rules and their limits.
//! 4. **Head instantiation** — grounded head atoms become assertion axioms with
//!    fresh axiom ids and empty annotations.  Anonymous individuals are never
//!    emitted (DL-safety).
//! 5. **Fixpoint** — new facts are deduplicated by [`Axiom::content_key`]
//!    (axiom ids ignored), folded into the indexes and re-matched until no new
//!    fact appears or `max_iterations` rounds have run, so materialization
//!    always terminates (even for cyclic rules such as `A → B`, `B → A`).
//!
//! # Deliberate limitations (v1)
//!
//! * **Complex class predicates** (anything but `ClassExpression::Class`)
//!   match an asserted class only by *exact structural equality* — the
//!   asserted closure is deliberately name-only, so e.g. a body atom
//!   `∃R.C(?x)` never fires against a merely asserted `C`-style fact.
//! * **Property chains**, `SymmetricObjectProperty`, `TransitiveObjectProperty`,
//!   domain/range and cardinality consequences are *not* applied while
//!   matching; only the asserted sub-/equivalence closure is used.
//! * **Built-in atoms are filters only**: they succeed iff the registered
//!   builtin returns `SWRLValue::Boolean(true)`.  There is no output-mode
//!   binding, so a built-in atom whose arguments are still unbound skips the
//!   whole rule instance (put the binding atoms *before* the built-in in the
//!   body).  An unknown built-in logs a warning and skips the instance too —
//!   never an error.  Built-in arguments are converted with
//!   [`literal_to_swrl_value`].
//! * **`DataRangeAtom`** is a filter evaluated with
//!   [`DatatypeValidator::literal_satisfies_data_range`]; an unbound variable
//!   cannot be enumerated, so the instance is skipped.
//! * **DL-safety**: only named individuals are ever bound or emitted, and a
//!   rule that is not [`SWRLRule::is_safe`] (a head variable absent from the
//!   body) is skipped entirely with a warning — such a rule can never produce
//!   a fact.
//! * `NegativeObjectPropertyAssertion` / `NegativeDataPropertyAssertion` are
//!   not derived (they are not rule-head materialisations in v1).

use crate::Result;
use crate::core::reasoner::datatype_validation::DatatypeValidator;
use crate::ontology::{
    Axiom, Class, ClassExpression, DataPropertyExpression, IRI, Individual, Literal,
    ObjectPropertyExpression, Ontology, SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule,
    SWRLRuleAxiom, SWRLVariable,
    axioms::{
        AxiomId, AxiomTrait, ClassAssertionAxiom, DataPropertyAssertionAxiom,
        DifferentIndividualsAxiom, ObjectPropertyAssertionAxiom, SameIndividualAxiom,
    },
};
use crate::swrl::builtins::{SWRLBuiltInRegistry, SWRLValue};
use log::{debug, warn};
use std::collections::{HashMap, HashSet, VecDeque};

/// Default fixpoint bound used by [`SwrlMaterializer::new`].
const DEFAULT_MAX_ITERATIONS: usize = 1000;

/// Safety cap for the union-find root walk (the structure is acyclic by
/// construction; this only guards against a corrupted parent map).
const UNION_FIND_MAX_STEPS: u32 = 1_000_000;

/// DL-safe SWRL forward-chaining materializer.
///
/// The materializer is stateless: every [`Self::materialize`] call rebuilds
/// the fact indexes from the supplied ontology, so the same instance can be
/// reused across ontologies and repeated runs are idempotent (facts already
/// present are recognised by content key and never re-derived).
#[derive(Debug, Clone)]
pub struct SwrlMaterializer {
    /// Maximum number of fixpoint rounds.  Each round matches every rule
    /// against the facts known at the start of the round and folds the new
    /// facts in at the end; `0` disables materialization entirely.
    pub max_iterations: usize,
}

/// Result of a [`SwrlMaterializer::materialize`] run.
#[derive(Debug, Clone, Default)]
pub struct SwrlMaterialization {
    /// Newly derived `ABox` assertion axioms, in derivation order, with fresh
    /// axiom ids and empty annotations.  Never contains a duplicate of an
    /// axiom already present in the ontology (compared by content key).
    pub derived_axioms: Vec<Axiom>,
    /// Number of fired rule instances — body bindings whose head produced at
    /// least one genuinely new fact.
    pub applications: usize,
    /// Number of productive fixpoint rounds (rounds that derived at least one
    /// new fact).  The final, empty round is not counted.
    pub iterations: usize,
}

impl SwrlMaterializer {
    /// Create a materializer with the default iteration bound (1000).
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }

    /// Materialize every `Axiom::Rule` of `ontology` over its `ABox`.
    ///
    /// Returns the derived axioms plus the application/iteration statistics.
    /// The input ontology is never mutated.  Ontologies without rules return
    /// an empty result without building any index.
    pub fn materialize(&self, ontology: &Ontology) -> Result<SwrlMaterialization> {
        let rule_axioms: Vec<&SWRLRuleAxiom> = ontology
            .axioms()
            .iter()
            .filter_map(|axiom| match axiom {
                Axiom::Rule(rule) => Some(rule),
                _ => None,
            })
            .collect();

        if rule_axioms.is_empty() {
            return Ok(SwrlMaterialization::default());
        }

        // DL-safety gate: a rule with a head variable that never occurs in
        // the body cannot be grounded, so it is skipped entirely (HermiT
        // rejects such rules as non-DL-safe).
        let mut rules: Vec<&SWRLRule> = Vec::with_capacity(rule_axioms.len());
        for rule_axiom in &rule_axioms {
            if rule_axiom.rule.is_safe() {
                rules.push(&rule_axiom.rule);
            } else {
                warn!(
                    "unsafe SWRL rule (head variable not bound by the body) skipped: {:?}",
                    rule_axiom.rule
                );
            }
        }

        if rules.is_empty() {
            debug!(
                "SWRL materialization: {} rule(s), all unsafe — nothing to derive",
                rule_axioms.len()
            );
            return Ok(SwrlMaterialization::default());
        }

        let mut facts = FactBase::from_ontology(ontology);
        let closures = Closures::from_ontology(ontology);
        let mut validator = DatatypeValidator::new();
        validator.register_ontology_datatypes(ontology);
        let registry = SWRLBuiltInRegistry::new();

        let mut derived: Vec<Axiom> = Vec::new();
        let mut applications = 0usize;
        let mut iterations = 0usize;
        let mut next_id = ontology
            .axioms()
            .iter()
            .map(AxiomTrait::axiom_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        while iterations < self.max_iterations {
            let (round_facts, round_applications) = {
                let matcher = Matcher {
                    facts: &facts,
                    closures: &closures,
                    validator: &validator,
                    registry: &registry,
                };
                run_round(&matcher, &rules, &facts, &mut next_id)
            };

            if round_facts.is_empty() {
                break;
            }

            applications += round_applications;
            iterations += 1;
            for axiom in &round_facts {
                facts.insert(axiom);
            }
            derived.extend(round_facts);
        }

        debug!(
            "SWRL materialization: {} rule(s), {} application(s), {} iteration(s), {} new axiom(s)",
            rules.len(),
            applications,
            iterations,
            derived.len()
        );

        Ok(SwrlMaterialization {
            derived_axioms: derived,
            applications,
            iterations,
        })
    }
}

impl Default for SwrlMaterializer {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bindings
// ─────────────────────────────────────────────────────────────────────────────

/// A value bound to a rule variable while matching a body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BoundValue {
    Individual(Individual),
    Literal(Literal),
}

/// Variable → value substitution for one rule instance.
type Bindings = HashMap<SWRLVariable, BoundValue>;

/// Resolve an individual argument to a ground value, if it is ground.
fn ground_iarg(argument: &SWRLIArgument, bindings: &Bindings) -> Option<Individual> {
    match argument {
        SWRLIArgument::Individual(individual) => Some(individual.clone()),
        SWRLIArgument::Variable(variable) => match bindings.get(variable)? {
            BoundValue::Individual(individual) => Some(individual.clone()),
            BoundValue::Literal(_) => None,
        },
    }
}

/// Resolve an individual argument that must denote a **named** individual.
fn ground_named_iarg(argument: &SWRLIArgument, bindings: &Bindings) -> Option<Individual> {
    let individual = ground_iarg(argument, bindings)?;
    individual.is_named().then_some(individual)
}

/// Resolve a data argument to a ground literal, if it is ground.
fn ground_darg(argument: &SWRLDArgument, bindings: &Bindings) -> Option<Literal> {
    match argument {
        SWRLDArgument::Literal(literal) => Some(literal.clone()),
        SWRLDArgument::Variable(variable) => match bindings.get(variable)? {
            BoundValue::Literal(literal) => Some(literal.clone()),
            BoundValue::Individual(_) => None,
        },
    }
}

/// Convert a literal into the [`SWRLValue`] domain expected by the builtins.
///
/// The mapping is datatype-driven so the numeric/string/boolean builtins
/// (`swrlb:lessThan`, `swrlb:stringLength`, …) receive the variant they
/// pattern-match on:
///
/// | literal                                    | `SWRLValue`      |
/// |--------------------------------------------|------------------|
/// | `xsd:integer` (+ derived integer types)    | `Integer`        |
/// | `xsd:float` / `xsd:double`                 | `Float`          |
/// | `xsd:decimal`                              | `Float`          |
/// | `xsd:boolean`                              | `Boolean`        |
/// | `xsd:string`                               | `String`         |
/// | untyped (no datatype, no language tag)     | numeric parse → `Integer`/`Float`, else `String` |
/// | language-tagged / any other datatype       | `Literal` (verbatim) |
///
/// `xsd:decimal` is deliberately widened to `Float` because no registered
/// builtin consumes `SWRLValue::Decimal`; widening keeps decimal comparisons
/// working instead of failing the atom.
fn literal_to_swrl_value(literal: &Literal) -> SWRLValue {
    if literal.language.is_some() {
        return SWRLValue::Literal(literal.clone());
    }
    let datatype = literal
        .datatype
        .as_ref()
        .map(std::string::ToString::to_string);
    let value = literal.value.as_str();

    match datatype.as_deref() {
        Some(dt) if is_integer_datatype(dt) => {
            if let Ok(parsed) = value.parse::<i64>() {
                return SWRLValue::Integer(parsed);
            }
        }
        Some(dt) if is_float_datatype(dt) => {
            if let Ok(parsed) = value.parse::<f64>() {
                return SWRLValue::Float(parsed);
            }
        }
        Some("http://www.w3.org/2001/XMLSchema#boolean") => {
            if let Ok(parsed) = value.parse::<bool>() {
                return SWRLValue::Boolean(parsed);
            }
        }
        Some("http://www.w3.org/2001/XMLSchema#string") => {
            return SWRLValue::String(value.to_string());
        }
        Some(_) => return SWRLValue::Literal(literal.clone()),
        None => {
            // Untyped literal: the functional-syntax parser mints bare
            // numerics (`18`) without a datatype, so honour a numeric form
            // first and fall back to a plain string.
            if let Ok(parsed) = value.parse::<i64>() {
                return SWRLValue::Integer(parsed);
            }
            if value.contains('.')
                && let Ok(parsed) = value.parse::<f64>()
            {
                return SWRLValue::Float(parsed);
            }
            return SWRLValue::String(value.to_string());
        }
    }
    // Datatype claimed numeric but the lexical form is not well-formed:
    // fall through to the verbatim literal so the builtin sees the raw value.
    SWRLValue::Literal(literal.clone())
}

fn is_integer_datatype(datatype: &str) -> bool {
    matches!(
        datatype,
        "http://www.w3.org/2001/XMLSchema#integer"
            | "http://www.w3.org/2001/XMLSchema#int"
            | "http://www.w3.org/2001/XMLSchema#long"
            | "http://www.w3.org/2001/XMLSchema#short"
            | "http://www.w3.org/2001/XMLSchema#byte"
            | "http://www.w3.org/2001/XMLSchema#unsignedInt"
            | "http://www.w3.org/2001/XMLSchema#unsignedLong"
            | "http://www.w3.org/2001/XMLSchema#unsignedShort"
            | "http://www.w3.org/2001/XMLSchema#unsignedByte"
            | "http://www.w3.org/2001/XMLSchema#positiveInteger"
            | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            | "http://www.w3.org/2001/XMLSchema#negativeInteger"
            | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
    )
}

fn is_float_datatype(datatype: &str) -> bool {
    matches!(
        datatype,
        "http://www.w3.org/2001/XMLSchema#decimal"
            | "http://www.w3.org/2001/XMLSchema#float"
            | "http://www.w3.org/2001/XMLSchema#double"
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Union-find over named individuals (sameAs)
// ─────────────────────────────────────────────────────────────────────────────

/// Simple union-find over [`Individual`]s induced by `SameIndividual` axioms.
#[derive(Debug, Default, Clone)]
struct UnionFind {
    parent: HashMap<Individual, Individual>,
}

impl UnionFind {
    /// Representative of `x` (returns `x` itself when it never occurred in a
    /// `SameIndividual` axiom).  `union` only ever links a root under another
    /// root, so the walk is acyclic; `find` therefore needs no `&mut`.
    fn find(&self, x: &Individual) -> Individual {
        let mut current = x.clone();
        for _ in 0..UNION_FIND_MAX_STEPS {
            match self.parent.get(&current) {
                Some(next) if *next != current => current = next.clone(),
                _ => break,
            }
        }
        current
    }

    /// Merge the equivalence classes of `a` and `b`.  The lexicographically
    /// smaller representative wins so the outcome does not depend on argument
    /// order beyond the axiom order itself.
    fn union(&mut self, a: &Individual, b: &Individual) {
        let left = self.find(a);
        let right = self.find(b);
        if left == right {
            return;
        }
        if left.to_string() <= right.to_string() {
            self.parent.insert(right, left);
        } else {
            self.parent.insert(left, right);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fact index
// ─────────────────────────────────────────────────────────────────────────────

/// Indexed `ABox` facts of one materialization run.
#[derive(Debug, Default)]
struct FactBase {
    /// Named asserted class IRI → individuals asserted in exactly that class.
    class_by_iri: HashMap<IRI, Vec<Individual>>,
    /// Complex asserted class expressions → individuals.  Only matched by
    /// exact structural equality against a complex body predicate.
    class_by_expr: Vec<(ClassExpression, Individual)>,
    /// Normalised object-property edges: base property IRI → `(source, target)`.
    /// An assertion with an inverse property expression has its endpoints
    /// swapped, so every stored edge reads `base(source, target)`.
    obj_edges: HashMap<IRI, Vec<(Individual, Individual)>>,
    /// Data-property assertions: base property IRI → `(individual, literal)`.
    data_facts: HashMap<IRI, Vec<(Individual, Literal)>>,
    /// Every named individual known to the `ABox` (body matching / `owl:Thing`).
    individuals: HashSet<Individual>,
    /// Equivalence classes induced by `SameIndividual`.
    same: UnionFind,
    /// Raw `DifferentIndividuals` pairs; normalised through the union-find at
    /// query time so `a ≡ c`, `c ≠ b` still yields `a ≠ b`.
    different: Vec<(Individual, Individual)>,
    /// Content keys of every fact axiom already present (asserted *or*
    /// previously derived) — the deduplication set.
    known: HashSet<String>,
}

impl FactBase {
    /// Index every assertion axiom of `ontology` (rules and `TBox` axioms are
    /// handled by [`Closures`]).
    fn from_ontology(ontology: &Ontology) -> Self {
        let mut base = Self::default();
        for axiom in ontology.axioms() {
            if matches!(
                axiom,
                Axiom::ClassAssertion(_)
                    | Axiom::ObjectPropertyAssertion(_)
                    | Axiom::DataPropertyAssertion(_)
                    | Axiom::SameIndividual(_)
                    | Axiom::DifferentIndividuals(_)
            ) {
                base.insert(axiom);
            }
        }
        base
    }

    /// Add one fact axiom to the index (idempotent by content key).
    fn insert(&mut self, axiom: &Axiom) {
        self.known.insert(axiom.content_key());
        match axiom {
            Axiom::ClassAssertion(assertion) => {
                let Some(individual) = named(&assertion.individual) else {
                    return;
                };
                self.individuals.insert(individual.clone());
                match &assertion.class {
                    ClassExpression::Class(class) => {
                        self.class_by_iri
                            .entry(class.iri.clone())
                            .or_default()
                            .push(individual);
                    }
                    other => self.class_by_expr.push((other.clone(), individual)),
                }
            }
            Axiom::ObjectPropertyAssertion(assertion) => {
                let Some(source) = named(&assertion.source) else {
                    return;
                };
                let Some(target) = named(&assertion.target) else {
                    return;
                };
                self.individuals.insert(source.clone());
                self.individuals.insert(target.clone());
                if let Some((iri, positive)) = normalise_property(&assertion.property) {
                    // `inv(P)(s, t)` is stored as `P(t, s)`.
                    let edge = if positive {
                        (source, target)
                    } else {
                        (target, source)
                    };
                    self.obj_edges.entry(iri).or_default().push(edge);
                }
            }
            Axiom::DataPropertyAssertion(assertion) => {
                let Some(individual) = named(&assertion.individual) else {
                    return;
                };
                self.individuals.insert(individual.clone());
                let DataPropertyExpression::DataProperty(property) = &assertion.property;
                self.data_facts
                    .entry(property.iri.clone())
                    .or_default()
                    .push((individual, assertion.value.clone()));
            }
            Axiom::SameIndividual(assertion) => {
                let mut members: Vec<Individual> = Vec::new();
                for individual in &assertion.individuals {
                    if let Some(named_individual) = named(individual) {
                        self.individuals.insert(named_individual.clone());
                        members.push(named_individual);
                    }
                }
                if let Some((first, rest)) = members.split_first() {
                    for other in rest {
                        self.same.union(first, other);
                    }
                }
            }
            Axiom::DifferentIndividuals(assertion) => {
                let members: Vec<Individual> =
                    assertion.individuals.iter().filter_map(named).collect();
                for individual in &members {
                    self.individuals.insert(individual.clone());
                }
                for (i, left) in members.iter().enumerate() {
                    for right in members.iter().skip(i + 1) {
                        self.different.push((left.clone(), right.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether `a` and `b` are known to be the same individual.
    fn equivalent(&self, a: &Individual, b: &Individual) -> bool {
        self.same.find(a) == self.same.find(b)
    }

    /// Whether `a` and `b` are *explicitly* known to be different individuals.
    fn different(&self, a: &Individual, b: &Individual) -> bool {
        let left = self.same.find(a);
        let right = self.same.find(b);
        if left == right {
            return false;
        }
        self.different.iter().any(|(x, y)| {
            let (rx, ry) = (self.same.find(x), self.same.find(y));
            (rx == left && ry == right) || (rx == right && ry == left)
        })
    }

    /// All known members of `x`'s equivalence class, `x` first.
    fn same_as_members(&self, x: &Individual) -> Vec<Individual> {
        let representative = self.same.find(x);
        let mut members = vec![x.clone()];
        for individual in &self.individuals {
            if individual != x && self.same.find(individual) == representative {
                members.push(individual.clone());
            }
        }
        members
    }
}

/// The named form of an individual, or `None` for anonymous individuals
/// (DL-safety: rules only ever range over named individuals).
fn named(individual: &Individual) -> Option<Individual> {
    individual.is_named().then(|| individual.clone())
}

/// Append `individual` unless it is already present (first occurrence wins,
/// so the candidate order stays deterministic).
fn push_unique(out: &mut Vec<Individual>, seen: &mut HashSet<Individual>, individual: Individual) {
    if seen.insert(individual.clone()) {
        out.push(individual);
    }
}

/// Normalise an object-property expression to `(base IRI, positive?)`.
/// Returns `None` for property chains (unsupported in v1).
fn normalise_property(property: &ObjectPropertyExpression) -> Option<(IRI, bool)> {
    match property {
        ObjectPropertyExpression::ObjectProperty(prop) => Some((prop.iri.clone(), true)),
        ObjectPropertyExpression::InverseObjectProperty(prop) => Some((prop.iri.clone(), false)),
        ObjectPropertyExpression::PropertyChain(_) => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Asserted closure indexes
// ─────────────────────────────────────────────────────────────────────────────

/// Cheap, asserted-only super-property/subclass closures.
#[derive(Debug, Default)]
struct Closures {
    /// Named class IRI → every superclass reachable through named/named
    /// `SubClassOf` and `EquivalentClasses` (reflexive + transitive, plus the
    /// implicit `⊑ owl:Thing` edge).
    class_super: HashMap<IRI, HashSet<IRI>>,
    /// Named object property IRI → the signed `(base, positive)` super-property
    /// nodes reachable from it (reflexive + transitive).  A reachable
    /// `(q, false)` node means `p ⊑ inv(q)`.
    obj_super: HashMap<IRI, HashSet<(IRI, bool)>>,
    /// Named data property IRI → super-property IRIs (reflexive + transitive).
    data_super: HashMap<IRI, HashSet<IRI>>,
}

impl Closures {
    fn from_ontology(ontology: &Ontology) -> Self {
        let mut class_edges: HashMap<IRI, HashSet<IRI>> = HashMap::new();
        let mut obj_edges: HashMap<(IRI, bool), HashSet<(IRI, bool)>> = HashMap::new();
        let mut obj_nodes: HashSet<(IRI, bool)> = HashSet::new();
        let mut data_edges: HashMap<IRI, HashSet<IRI>> = HashMap::new();

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(axiom) => {
                    if let (ClassExpression::Class(sub), ClassExpression::Class(sup)) =
                        (&axiom.subclass, &axiom.superclass)
                    {
                        class_edges
                            .entry(sub.iri.clone())
                            .or_default()
                            .insert(sup.iri.clone());
                    }
                }
                Axiom::EquivalentClasses(axiom) => {
                    let named: Vec<&Class> = axiom
                        .classes
                        .iter()
                        .filter_map(|expr| match expr {
                            ClassExpression::Class(class) => Some(class),
                            _ => None,
                        })
                        .collect();
                    for left in &named {
                        for right in &named {
                            if left.iri != right.iri {
                                class_edges
                                    .entry(left.iri.clone())
                                    .or_default()
                                    .insert(right.iri.clone());
                            }
                        }
                    }
                }
                Axiom::SubObjectPropertyOf(axiom) => {
                    if let (Some(sub), Some(sup)) = (
                        normalise_property(&axiom.sub_property),
                        normalise_property(&axiom.super_property),
                    ) {
                        add_obj_sub(&mut obj_edges, &mut obj_nodes, sub, sup);
                    }
                }
                Axiom::EquivalentObjectProperties(axiom) => {
                    let props: Vec<(IRI, bool)> = axiom
                        .properties
                        .iter()
                        .filter_map(normalise_property)
                        .collect();
                    for left in &props {
                        for right in &props {
                            if left != right {
                                add_obj_sub(
                                    &mut obj_edges,
                                    &mut obj_nodes,
                                    left.clone(),
                                    right.clone(),
                                );
                            }
                        }
                    }
                }
                Axiom::InverseObjectProperties(axiom) => {
                    // `p1 ≡ inv(p2)` gives four signed edges:
                    // p1 ⊑ inv(p2), inv(p2) ⊑ p1 and their inverted mirrors.
                    if let (Some(left), Some(right)) = (
                        normalise_property(&axiom.property1),
                        normalise_property(&axiom.property2),
                    ) {
                        let inverse_right = (right.0, !right.1);
                        add_obj_sub(
                            &mut obj_edges,
                            &mut obj_nodes,
                            left.clone(),
                            inverse_right.clone(),
                        );
                        add_obj_sub(&mut obj_edges, &mut obj_nodes, inverse_right, left);
                    }
                }
                Axiom::SubDataPropertyOf(axiom) => {
                    let (
                        DataPropertyExpression::DataProperty(sub),
                        DataPropertyExpression::DataProperty(sup),
                    ) = (&axiom.sub_property, &axiom.super_property);
                    data_edges
                        .entry(sub.iri.clone())
                        .or_default()
                        .insert(sup.iri.clone());
                }
                Axiom::EquivalentDataProperties(axiom) => {
                    let props: Vec<&IRI> = axiom
                        .properties
                        .iter()
                        .map(|expr| match expr {
                            DataPropertyExpression::DataProperty(prop) => &prop.iri,
                        })
                        .collect();
                    for left in &props {
                        for right in &props {
                            if left != right {
                                data_edges
                                    .entry((*left).clone())
                                    .or_default()
                                    .insert((*right).clone());
                            }
                        }
                    }
                }
                Axiom::ObjectPropertyAssertion(axiom) => {
                    if let Some((iri, positive)) = normalise_property(&axiom.property) {
                        // Asserted properties are reflexive super-properties
                        // of themselves, even without any TBox axiom.
                        obj_nodes.insert((iri.clone(), positive));
                        obj_nodes.insert((iri, !positive));
                    }
                }
                _ => {}
            }
        }

        // Implicit `⊑ owl:Thing` for every class seen in a subsumption edge.
        let thing = IRI::owl_thing();
        for supers in class_edges.values_mut() {
            supers.insert(thing.clone());
        }

        Self {
            class_super: transitive_class_closure(class_edges),
            obj_super: transitive_obj_closure(obj_edges, obj_nodes),
            data_super: transitive_data_closure(data_edges),
        }
    }

    /// `subclass ⊑ superclass` under the asserted named-class closure.
    fn is_class_sub(&self, subclass: &IRI, superclass: &IRI) -> bool {
        subclass == superclass
            || superclass.is_owl_thing()
            || self
                .class_super
                .get(subclass)
                .is_some_and(|supers| supers.contains(superclass))
    }

    /// `sub ⊑ sup` under the asserted named-data-property closure.
    fn is_data_sub(&self, sub: &IRI, sup: &IRI) -> bool {
        sub == sup || self.data_super.get(sub).is_some_and(|s| s.contains(sup))
    }

    /// Whether the object property `from` is a sub-property of the signed
    /// node `(to, positive)`.
    fn obj_reaches(&self, from: &IRI, to: &IRI, positive: bool) -> bool {
        // Reflexive fallback: an asserted property is a sub-property of
        // itself even when no TBox axiom mentions it.
        if from == to && positive {
            return true;
        }
        self.obj_super
            .get(from)
            .is_some_and(|reach| reach.contains(&(to.clone(), positive)))
    }
}

/// `sub ⊑ super`, mirrored as `inv(sub) ⊑ inv(super)` — the signed form of
/// the property-hierarchy edges.
fn add_obj_sub(
    edges: &mut HashMap<(IRI, bool), HashSet<(IRI, bool)>>,
    nodes: &mut HashSet<(IRI, bool)>,
    sub: (IRI, bool),
    sup: (IRI, bool),
) {
    let sub_mirror = (sub.0.clone(), !sub.1);
    let sup_mirror = (sup.0.clone(), !sup.1);
    add_obj_edge(edges, nodes, sub, sup);
    add_obj_edge(edges, nodes, sub_mirror, sup_mirror);
}

fn add_obj_edge(
    edges: &mut HashMap<(IRI, bool), HashSet<(IRI, bool)>>,
    nodes: &mut HashSet<(IRI, bool)>,
    from: (IRI, bool),
    to: (IRI, bool),
) {
    nodes.insert(from.clone());
    nodes.insert(to.clone());
    edges.entry(from).or_default().insert(to);
}

fn transitive_class_closure(edges: HashMap<IRI, HashSet<IRI>>) -> HashMap<IRI, HashSet<IRI>> {
    let mut all: HashSet<IRI> = edges.keys().cloned().collect();
    for supers in edges.values() {
        all.extend(supers.iter().cloned());
    }
    let mut closure: HashMap<IRI, HashSet<IRI>> = HashMap::new();
    for start in &all {
        let mut visited: HashSet<IRI> = HashSet::new();
        let mut queue: VecDeque<IRI> = VecDeque::new();
        visited.insert(start.clone());
        queue.push_back(start.clone());
        while let Some(current) = queue.pop_front() {
            if let Some(supers) = edges.get(&current) {
                for super_class in supers {
                    if visited.insert(super_class.clone()) {
                        queue.push_back(super_class.clone());
                    }
                }
            }
        }
        closure.insert(start.clone(), visited);
    }
    closure
}

fn transitive_data_closure(edges: HashMap<IRI, HashSet<IRI>>) -> HashMap<IRI, HashSet<IRI>> {
    transitive_class_closure(edges)
}

fn transitive_obj_closure(
    edges: HashMap<(IRI, bool), HashSet<(IRI, bool)>>,
    nodes: HashSet<(IRI, bool)>,
) -> HashMap<IRI, HashSet<(IRI, bool)>> {
    let mut all: HashSet<(IRI, bool)> = nodes;
    for (from, tos) in &edges {
        all.insert(from.clone());
        all.extend(tos.iter().cloned());
    }

    let mut reach: HashMap<(IRI, bool), HashSet<(IRI, bool)>> = HashMap::new();
    for start in &all {
        let mut visited: HashSet<(IRI, bool)> = HashSet::new();
        let mut queue: VecDeque<(IRI, bool)> = VecDeque::new();
        visited.insert(start.clone());
        queue.push_back(start.clone());
        while let Some(current) = queue.pop_front() {
            if let Some(nexts) = edges.get(&current) {
                for next in nexts {
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        reach.insert(start.clone(), visited);
    }

    // Keep only the entries reachable from a *positive* base property — every
    // stored fact edge is already normalised to a positive base IRI.
    let mut positive: HashMap<IRI, HashSet<(IRI, bool)>> = HashMap::new();
    for (iri, polarity) in &all {
        if *polarity && let Some(reached) = reach.get(&(iri.clone(), true)) {
            positive.insert(iri.clone(), reached.clone());
        }
    }
    positive
}

// ─────────────────────────────────────────────────────────────────────────────
// Body matching
// ─────────────────────────────────────────────────────────────────────────────

/// Read-only matcher bundling the indexes of one materialization run.
struct Matcher<'a> {
    facts: &'a FactBase,
    closures: &'a Closures,
    validator: &'a DatatypeValidator,
    registry: &'a SWRLBuiltInRegistry,
}

/// One fixpoint round: match every rule, instantiate every head, keep the
/// facts that are genuinely new.  Returns `(new facts, fired applications)`.
fn run_round(
    matcher: &Matcher<'_>,
    rules: &[&SWRLRule],
    facts: &FactBase,
    next_id: &mut AxiomId,
) -> (Vec<Axiom>, usize) {
    let mut round_facts: Vec<Axiom> = Vec::new();
    let mut round_keys: HashSet<String> = HashSet::new();
    let mut round_applications = 0usize;

    for rule in rules {
        for bindings in matcher.match_body(rule) {
            let mut produced = false;
            for head_atom in &rule.head {
                let Some(axiom) = instantiate_head(head_atom, &bindings, next_id) else {
                    continue;
                };
                let key = axiom.content_key();
                if facts.known.contains(&key) || round_keys.contains(&key) {
                    continue;
                }
                round_keys.insert(key);
                round_facts.push(axiom);
                produced = true;
            }
            if produced {
                round_applications += 1;
            }
        }
    }

    (round_facts, round_applications)
}

impl Matcher<'_> {
    /// All bindings satisfying `rule.body`, computed by backtracking over the
    /// body atoms in order.  An empty body yields the single empty binding
    /// (ground rules fire unconditionally).
    fn match_body(&self, rule: &SWRLRule) -> Vec<Bindings> {
        let mut current: Vec<Bindings> = vec![Bindings::new()];
        for atom in &rule.body {
            let mut next: Vec<Bindings> = Vec::new();
            for bindings in &current {
                self.extend(atom, bindings, &mut next);
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }
        current
    }

    /// Extend one partial binding with every solution of `atom`.
    fn extend(&self, atom: &SWRLAtom, bindings: &Bindings, out: &mut Vec<Bindings>) {
        match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => {
                for candidate in self.class_candidates(predicate) {
                    self.bind_iarg(argument, &candidate, bindings, out);
                }
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => self.extend_object(predicate, first_argument, second_argument, bindings, out),
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => self.extend_data(predicate, first_argument, second_argument, bindings, out),
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => {
                // Filter atom: both a literal constant and a bound variable
                // can be evaluated; an unbound variable cannot be enumerated.
                let literal = match argument {
                    SWRLDArgument::Literal(literal) => Some(literal),
                    SWRLDArgument::Variable(variable) => match bindings.get(variable) {
                        Some(BoundValue::Literal(literal)) => Some(literal),
                        _ => None,
                    },
                };
                if let Some(literal) = literal
                    && self
                        .validator
                        .literal_satisfies_data_range(literal, predicate)
                        .unwrap_or(false)
                {
                    out.push(bindings.clone());
                }
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => self.extend_same(first_argument, second_argument, bindings, out),
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                let (Some(left), Some(right)) = (
                    ground_iarg(first_argument, bindings),
                    ground_iarg(second_argument, bindings),
                ) else {
                    // DifferentFrom cannot bind an unbound variable (the
                    // candidate set is not enumerable in v1).
                    return;
                };
                if self.facts.different(&left, &right) {
                    out.push(bindings.clone());
                }
            }
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => self.extend_builtin(predicate, arguments, bindings, out),
        }
    }

    /// Individuals matching a class atom predicate.
    ///
    /// * named predicate → individuals whose *asserted named* class is the
    ///   predicate itself or a named subclass of it (asserted closure);
    ///   `owl:Thing` matches every individual known to the `ABox`;
    /// * complex predicate → only individuals asserting a *structurally equal*
    ///   complex class expression (documented v1 limitation).
    fn class_candidates(&self, predicate: &ClassExpression) -> Vec<Individual> {
        let mut seen: HashSet<Individual> = HashSet::new();
        let mut candidates: Vec<Individual> = Vec::new();

        match predicate {
            ClassExpression::Class(class) if class.is_thing() => {
                for individual in &self.facts.individuals {
                    push_unique(&mut candidates, &mut seen, individual.clone());
                }
            }
            ClassExpression::Class(class) => {
                for (iri, individuals) in &self.facts.class_by_iri {
                    if self.closures.is_class_sub(iri, &class.iri) {
                        for individual in individuals {
                            push_unique(&mut candidates, &mut seen, individual.clone());
                        }
                    }
                }
            }
            other => {
                for (asserted, individual) in &self.facts.class_by_expr {
                    if asserted == other {
                        push_unique(&mut candidates, &mut seen, individual.clone());
                    }
                }
            }
        }

        // Deterministic order regardless of `HashMap` iteration order.
        candidates.sort_by_key(std::string::ToString::to_string);
        candidates
    }

    /// Bind/verify one individual argument against `value`.
    fn bind_iarg(
        &self,
        argument: &SWRLIArgument,
        value: &Individual,
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        match argument {
            SWRLIArgument::Individual(constant) => {
                if self.facts.equivalent(constant, value) {
                    out.push(bindings.clone());
                }
            }
            SWRLIArgument::Variable(variable) => match bindings.get(variable) {
                None => {
                    let mut extended = bindings.clone();
                    extended.insert(variable.clone(), BoundValue::Individual(value.clone()));
                    out.push(extended);
                }
                Some(BoundValue::Individual(bound)) => {
                    if self.facts.equivalent(bound, value) {
                        out.push(bindings.clone());
                    }
                }
                // Type clash: an individual atom can never satisfy a
                // literal-bound variable.
                Some(BoundValue::Literal(_)) => {}
            },
        }
    }

    /// Bind/verify one data argument against `value`.
    fn bind_darg(
        &self,
        argument: &SWRLDArgument,
        value: &Literal,
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        match argument {
            SWRLDArgument::Literal(constant) => {
                if constant == value {
                    out.push(bindings.clone());
                }
            }
            SWRLDArgument::Variable(variable) => match bindings.get(variable) {
                None => {
                    let mut extended = bindings.clone();
                    extended.insert(variable.clone(), BoundValue::Literal(value.clone()));
                    out.push(extended);
                }
                Some(BoundValue::Literal(bound)) => {
                    if bound == value {
                        out.push(bindings.clone());
                    }
                }
                Some(BoundValue::Individual(_)) => {}
            },
        }
    }

    /// Match an object-property atom against the normalised edge index.
    ///
    /// For every asserted edge `p0(s, t)` whose property can reach the atom's
    /// base property `q` (either directly or as `inv(q)`), the atom's
    /// arguments are bound to the correspondingly oriented endpoints.
    fn extend_object(
        &self,
        predicate: &ObjectPropertyExpression,
        first: &SWRLIArgument,
        second: &SWRLIArgument,
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        let (base, atom_positive) = match predicate {
            ObjectPropertyExpression::ObjectProperty(property) => (property.iri.clone(), true),
            ObjectPropertyExpression::InverseObjectProperty(property) => {
                (property.iri.clone(), false)
            }
            // Property chains are not supported as rule predicates in v1.
            ObjectPropertyExpression::PropertyChain(_) => return,
        };

        let mut bases: Vec<&IRI> = self.facts.obj_edges.keys().collect();
        bases.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        for p0 in bases {
            // Orientations of `q(u, v)` that follow from `p0(s, t)`.
            let mut orientations: Vec<bool> = Vec::new();
            if self.closures.obj_reaches(p0, &base, true) {
                orientations.push(false); // q(s, t)
            }
            if self.closures.obj_reaches(p0, &base, false) {
                orientations.push(true); // q(t, s)
            }
            if orientations.is_empty() {
                continue;
            }
            for &flipped in &orientations {
                for (source, target) in &self.facts.obj_edges[p0] {
                    let (u, v) = if flipped {
                        (target, source)
                    } else {
                        (source, target)
                    };
                    // The atom denotes `q(x, y)` (positive) or `q(y, x)`.
                    let (x, y) = if atom_positive { (u, v) } else { (v, u) };
                    let mut step: Vec<Bindings> = Vec::new();
                    self.bind_iarg(first, x, bindings, &mut step);
                    for partial in &step {
                        self.bind_iarg(second, y, partial, out);
                    }
                }
            }
        }
    }

    /// Match a data-property atom against the data assertion index.
    fn extend_data(
        &self,
        predicate: &DataPropertyExpression,
        first: &SWRLIArgument,
        second: &SWRLDArgument,
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        let DataPropertyExpression::DataProperty(property) = predicate;
        let mut bases: Vec<&IRI> = self.facts.data_facts.keys().collect();
        bases.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        for p0 in bases {
            if !self.closures.is_data_sub(p0, &property.iri) {
                continue;
            }
            for (individual, literal) in &self.facts.data_facts[p0] {
                let mut step: Vec<Bindings> = Vec::new();
                self.bind_iarg(first, individual, bindings, &mut step);
                for partial in &step {
                    self.bind_darg(second, literal, partial, out);
                }
            }
        }
    }

    /// Match a `sameAs` atom through the union-find.  With one side already
    /// ground the other (unbound) variable is bound to every known member of
    /// the equivalence class; with neither side ground the atom cannot start a
    /// match.
    fn extend_same(
        &self,
        first: &SWRLIArgument,
        second: &SWRLIArgument,
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        let left = ground_iarg(first, bindings);
        let right = ground_iarg(second, bindings);
        match (left, right) {
            (Some(left), Some(right)) => {
                if self.facts.equivalent(&left, &right) {
                    out.push(bindings.clone());
                }
            }
            (Some(left), None) => {
                if let SWRLIArgument::Variable(variable) = second {
                    for member in self.facts.same_as_members(&left) {
                        let mut extended = bindings.clone();
                        extended.insert(variable.clone(), BoundValue::Individual(member));
                        out.push(extended);
                    }
                }
            }
            (None, Some(right)) => {
                if let SWRLIArgument::Variable(variable) = first {
                    for member in self.facts.same_as_members(&right) {
                        let mut extended = bindings.clone();
                        extended.insert(variable.clone(), BoundValue::Individual(member));
                        out.push(extended);
                    }
                }
            }
            (None, None) => {}
        }
    }

    /// Evaluate a built-in atom as a filter (see the module documentation
    /// for the output-mode limitation).
    fn extend_builtin(
        &self,
        predicate: &IRI,
        arguments: &[SWRLDArgument],
        bindings: &Bindings,
        out: &mut Vec<Bindings>,
    ) {
        if !self.registry.is_registered(predicate) {
            warn!("unknown SWRL built-in {predicate}; skipping rule instance");
            return;
        }

        let mut values: Vec<SWRLValue> = Vec::with_capacity(arguments.len());
        for argument in arguments {
            match argument {
                SWRLDArgument::Literal(literal) => values.push(literal_to_swrl_value(literal)),
                SWRLDArgument::Variable(variable) => match bindings.get(variable) {
                    Some(BoundValue::Individual(individual)) => {
                        values.push(SWRLValue::Individual(individual.clone()));
                    }
                    Some(BoundValue::Literal(literal)) => {
                        values.push(literal_to_swrl_value(literal));
                    }
                    // No output-mode binding in v1: an unbound argument makes
                    // the whole rule instance a no-op.
                    None => return,
                },
            }
        }

        match self.registry.execute(predicate, &values) {
            Ok(SWRLValue::Boolean(true)) => out.push(bindings.clone()),
            Ok(_) => {}
            Err(error) => {
                warn!("SWRL built-in {predicate} failed: {error}; skipping rule instance");
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Head instantiation
// ─────────────────────────────────────────────────────────────────────────────

/// Turn one bound head atom into a ground assertion axiom.
///
/// Returns `None` when the atom cannot be grounded (an unbound variable, a
/// literal bound to an individual position, an anonymous individual), when it
/// is a filter rather than an assertion (`DataRangeAtom`, `BuiltInAtom`), or
/// when it carries no information (`SameIndividual(a, a)`).
fn instantiate_head(atom: &SWRLAtom, bindings: &Bindings, next_id: &mut AxiomId) -> Option<Axiom> {
    let id = *next_id;
    let axiom = match atom {
        SWRLAtom::ClassAtom {
            predicate,
            argument,
        } => Axiom::ClassAssertion(ClassAssertionAxiom {
            id,
            individual: ground_named_iarg(argument, bindings)?,
            class: predicate.clone(),
            annotations: Vec::new(),
        }),
        SWRLAtom::ObjectPropertyAtom {
            predicate,
            first_argument,
            second_argument,
        } => {
            // A property chain is not a legal assertion property.
            normalise_property(predicate)?;
            Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                id,
                source: ground_named_iarg(first_argument, bindings)?,
                target: ground_named_iarg(second_argument, bindings)?,
                property: predicate.clone(),
                annotations: Vec::new(),
            })
        }
        SWRLAtom::DataPropertyAtom {
            predicate,
            first_argument,
            second_argument,
        } => Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
            id,
            individual: ground_named_iarg(first_argument, bindings)?,
            property: predicate.clone(),
            value: ground_darg(second_argument, bindings)?,
            annotations: Vec::new(),
        }),
        SWRLAtom::SameIndividualAtom {
            first_argument,
            second_argument,
        } => {
            let mut individuals = vec![
                ground_named_iarg(first_argument, bindings)?,
                ground_named_iarg(second_argument, bindings)?,
            ];
            // `sameAs(a, a)` carries no information; never emit it.
            if individuals[0] == individuals[1] {
                return None;
            }
            individuals.sort_by_key(std::string::ToString::to_string);
            Axiom::SameIndividual(SameIndividualAxiom {
                id,
                individuals,
                annotations: Vec::new(),
            })
        }
        SWRLAtom::DifferentIndividualsAtom {
            first_argument,
            second_argument,
        } => {
            let mut individuals = vec![
                ground_named_iarg(first_argument, bindings)?,
                ground_named_iarg(second_argument, bindings)?,
            ];
            // `differentFrom(a, a)` would assert an inconsistency; the body
            // can never produce it anyway (the union-find rejects it).
            if individuals[0] == individuals[1] {
                return None;
            }
            individuals.sort_by_key(std::string::ToString::to_string);
            Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                id,
                individuals,
                annotations: Vec::new(),
            })
        }
        // Filters, not assertions.
        SWRLAtom::DataRangeAtom { .. } | SWRLAtom::BuiltInAtom { .. } => return None,
    };
    *next_id += 1;
    Some(axiom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{DataProperty, NamedIndividual, ObjectProperty};

    fn class(name: &str) -> ClassExpression {
        ClassExpression::Class(Class::new(IRI::new(name)))
    }

    fn named_ind(name: &str) -> Individual {
        Individual::Named(NamedIndividual {
            iri: IRI::new(name),
        })
    }

    fn var(name: &str) -> SWRLVariable {
        SWRLVariable::new(IRI::new(name))
    }

    fn class_atom(class: ClassExpression, argument: SWRLIArgument) -> SWRLAtom {
        SWRLAtom::ClassAtom {
            predicate: class,
            argument,
        }
    }

    fn xsd(name: &str) -> IRI {
        IRI::new(&format!("http://www.w3.org/2001/XMLSchema#{name}"))
    }

    #[test]
    fn literal_conversion_follows_the_datatype() {
        let integer = Literal::with_datatype("10".to_string(), xsd("integer"));
        assert_eq!(literal_to_swrl_value(&integer), SWRLValue::Integer(10));

        let string = Literal::with_datatype("abc".to_string(), xsd("string"));
        assert_eq!(
            literal_to_swrl_value(&string),
            SWRLValue::String("abc".into())
        );

        // Bare numeric literal as minted by the functional-syntax parser.
        assert_eq!(
            literal_to_swrl_value(&Literal::new("18".to_string())),
            SWRLValue::Integer(18)
        );
        assert_eq!(
            literal_to_swrl_value(&Literal::new("hello".to_string())),
            SWRLValue::String("hello".into())
        );
    }

    #[test]
    fn unsafe_rule_is_skipped_without_panicking() {
        let mut ontology = Ontology::new();
        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            individual: named_ind("http://ex.org/alice"),
            class: class("http://ex.org/Person"),
            annotations: Vec::new(),
        }));
        // Head variable `?y` never occurs in the body → not DL-safe.
        let unsafe_rule = SWRLRule::new(
            vec![class_atom(
                class("http://ex.org/Student"),
                SWRLIArgument::Variable(var("http://ex.org/y")),
            )],
            vec![class_atom(
                class("http://ex.org/Person"),
                SWRLIArgument::Variable(var("http://ex.org/x")),
            )],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(2, unsafe_rule)));

        let result = SwrlMaterializer::new()
            .materialize(&ontology)
            .expect("materialization must not fail");
        assert!(result.derived_axioms.is_empty());
        assert_eq!(result.applications, 0);
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn class_rule_materializes_to_a_class_assertion() {
        let mut ontology = Ontology::new();
        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            individual: named_ind("http://ex.org/alice"),
            class: class("http://ex.org/Person"),
            annotations: Vec::new(),
        }));
        let rule = SWRLRule::new(
            vec![class_atom(
                class("http://ex.org/Student"),
                SWRLIArgument::Variable(var("http://ex.org/x")),
            )],
            vec![class_atom(
                class("http://ex.org/Person"),
                SWRLIArgument::Variable(var("http://ex.org/x")),
            )],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(2, rule)));

        let result = SwrlMaterializer::new()
            .materialize(&ontology)
            .expect("materialization");
        assert_eq!(result.iterations, 1);
        assert_eq!(result.applications, 1);
        assert_eq!(result.derived_axioms.len(), 1);
        match &result.derived_axioms[0] {
            Axiom::ClassAssertion(assertion) => {
                assert_eq!(assertion.class, class("http://ex.org/Student"));
                assert_eq!(assertion.individual, named_ind("http://ex.org/alice"));
            }
            other => panic!("expected a class assertion, got {other:?}"),
        }

        // A second run over the materialized ontology derives nothing new.
        let mut materialized = ontology;
        for axiom in result.derived_axioms {
            materialized.add_axiom(axiom);
        }
        let second = SwrlMaterializer::new()
            .materialize(&materialized)
            .expect("second materialization");
        assert!(second.derived_axioms.is_empty());
        assert_eq!(second.applications, 0);
    }

    #[test]
    fn object_property_inverse_atom_matches_the_reverse_edge() {
        let mut ontology = Ontology::new();
        let parent = ObjectPropertyExpression::InverseObjectProperty(ObjectProperty {
            iri: IRI::new("http://ex.org/hasParent"),
        });
        ontology.add_axiom(Axiom::ObjectPropertyAssertion(
            ObjectPropertyAssertionAxiom {
                id: 1,
                source: named_ind("http://ex.org/alice"),
                target: named_ind("http://ex.org/bob"),
                property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                    iri: IRI::new("http://ex.org/hasParent"),
                }),
                annotations: Vec::new(),
            },
        ));
        // inv(hasParent)(alice, bob) ⇔ hasParent(bob, alice): the rule body
        // must still match the asserted hasParent(alice, bob) edge.
        let rule = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: class("http://ex.org/Child"),
                argument: SWRLIArgument::Variable(var("http://ex.org/y")),
            }],
            vec![SWRLAtom::ObjectPropertyAtom {
                predicate: parent,
                first_argument: SWRLIArgument::Variable(var("http://ex.org/y")),
                second_argument: SWRLIArgument::Variable(var("http://ex.org/x")),
            }],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(2, rule)));

        let result = SwrlMaterializer::new()
            .materialize(&ontology)
            .expect("materialization");
        assert_eq!(result.iterations, 1);
        assert!(
            result
                .derived_axioms
                .iter()
                .any(|axiom| matches!(axiom, Axiom::ClassAssertion(a)
                if a.class == class("http://ex.org/Child")
                    && a.individual == named_ind("http://ex.org/bob")))
        );
    }

    #[test]
    fn data_property_rule_can_use_a_builtin_filter() {
        let mut ontology = Ontology::new();
        let age = DataPropertyExpression::DataProperty(DataProperty {
            iri: IRI::new("http://ex.org/age"),
        });
        let age_atom = |id: AxiomId, individual: Individual, value: Literal| {
            Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                id,
                individual,
                property: age.clone(),
                value,
                annotations: Vec::new(),
            })
        };
        ontology.add_axiom(age_atom(
            1,
            named_ind("http://ex.org/alice"),
            Literal::with_datatype("10".to_string(), xsd("integer")),
        ));
        ontology.add_axiom(age_atom(
            2,
            named_ind("http://ex.org/bob"),
            Literal::with_datatype("20".to_string(), xsd("integer")),
        ));
        let rule = SWRLRule::new(
            vec![class_atom(
                class("http://ex.org/Minor"),
                SWRLIArgument::Variable(var("http://ex.org/x")),
            )],
            vec![
                SWRLAtom::DataPropertyAtom {
                    predicate: age,
                    first_argument: SWRLIArgument::Variable(var("http://ex.org/x")),
                    second_argument: SWRLDArgument::Variable(var("http://ex.org/age")),
                },
                SWRLAtom::BuiltInAtom {
                    predicate: IRI::new("http://www.w3.org/2003/11/swrlb#lessThan"),
                    arguments: vec![
                        SWRLDArgument::Variable(var("http://ex.org/age")),
                        SWRLDArgument::Literal(Literal::new("18".to_string())),
                    ],
                },
            ],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(3, rule)));

        let result = SwrlMaterializer::new()
            .materialize(&ontology)
            .expect("materialization");
        assert_eq!(result.applications, 1, "only alice (age 10) is a minor");
        assert!(
            result
                .derived_axioms
                .iter()
                .any(|axiom| matches!(axiom, Axiom::ClassAssertion(a)
                if a.class == class("http://ex.org/Minor")
                    && a.individual == named_ind("http://ex.org/alice")))
        );
        assert!(!result.derived_axioms.iter().any(|axiom| matches!(
            axiom,
            Axiom::ClassAssertion(a) if a.individual == named_ind("http://ex.org/bob")
        )));
    }

    #[test]
    fn cyclic_rules_terminate_at_the_iteration_bound() {
        let mut ontology = Ontology::new();
        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            individual: named_ind("http://ex.org/alice"),
            class: class("http://ex.org/A"),
            annotations: Vec::new(),
        }));
        for (rule_id, (head, body)) in [
            (10u64, ("http://ex.org/B", "http://ex.org/A")),
            (11u64, ("http://ex.org/A", "http://ex.org/B")),
        ] {
            let rule = SWRLRule::new(
                vec![class_atom(
                    class(head),
                    SWRLIArgument::Variable(var("http://ex.org/x")),
                )],
                vec![class_atom(
                    class(body),
                    SWRLIArgument::Variable(var("http://ex.org/x")),
                )],
            );
            ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(rule_id, rule)));
        }

        let mut materializer = SwrlMaterializer::new();
        materializer.max_iterations = 3;
        let result = materializer
            .materialize(&ontology)
            .expect("cyclic rules must terminate");
        assert!(result.iterations <= 3);
        assert!(result
            .derived_axioms
            .iter()
            .all(|axiom| !matches!(axiom, Axiom::ClassAssertion(a) if a.class == class("http://ex.org/A"))));
    }
}
