//! OBO Format Parser and Writer.
//!
//! OBO (Open Biomedical Ontologies) is widely used in bio-ontologies
//! like the Gene Ontology. This module provides parsing, serialization,
//! and OBO↔OWL conversion.

pub mod converter;
pub mod parser;
pub mod writer;

pub use converter::{Obo2Owl, Owl2Obo};
pub use parser::{OBOParser, OBOParserConfig, parse};
pub use writer::{OBOOutputConfig, OBOWriter, save_file};
