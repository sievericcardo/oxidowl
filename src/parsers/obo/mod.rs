//! OBO Format Parser and Writer.
//!
//! OBO (Open Biomedical Ontologies) is widely used in bio-ontologies
//! like the Gene Ontology. This module provides parsing, serialization,
//! and OBO↔OWL conversion.

pub mod parser;
pub mod writer;
pub mod converter;

pub use parser::{OBOParser, OBOParserConfig, parse};
pub use writer::{OBOWriter, OBOOutputConfig, save_file};
pub use converter::{Obo2Owl, Owl2Obo};
