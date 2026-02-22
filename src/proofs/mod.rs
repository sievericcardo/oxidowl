//! Kani formal verification harnesses for oxidowl.
//!
//! # Running proofs
//!
//! Install Kani:
//! ```sh
//! cargo install kani-verifier
//! cargo kani setup
//! ```
//!
//! Run all proofs:
//! ```sh
//! cargo kani
//! ```
//!
//! Run a specific harness:
//! ```sh
//! cargo kani --harness dep_set_new_is_empty
//! ```
//!
//! # Organisation
//!
//! | Module            | What is proved                                                   |
//! |-------------------|------------------------------------------------------------------|
//! | `core`            | `DependencySet` algebraic laws (union, identity…)               |
//! | `ontology`        | `IRI` round-trip and equality; `Class` distinctness             |
//! | `owl_properties`  | OWL 2 property characteristics flags and consistency invariants  |
//! | `owl_expressions` | OWL 2 `ClassExpression` algebraic invariants (OWL 2 Syntax §8)  |
//! | `owl_axioms`      | OWL 2 axiom structural invariants and SWRL rule safety          |
//! | `rdf`             | RDF 1.1 `IRI`/`Individual`/`Ontology` foundational invariants   |
//! | `swrl`            | SWRL variable preservation and rule atom semantics              |

pub mod core;
pub mod ontology;
pub mod owl_axioms;
pub mod owl_expressions;
pub mod owl_properties;
pub mod rdf;
pub mod swrl;
