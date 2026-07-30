//! RIO (RDF Input/Output) Integration Formats.
//!
//! Provides parsers and renderers for additional RDF serialization
//! formats beyond the core six.

pub mod nquads;
pub mod n3;
pub mod trig;
pub mod trix;
pub mod jsonld;
pub mod rdf_json;
pub mod rdfa;
pub mod binary_rdf;
pub mod hdt;

use crate::ontology::Ontology;
use crate::Result;

/// Common trait for RIO format parsers.
pub trait RioParser: Send + Sync {
    fn parse(&self, content: &str) -> Result<Ontology>;
}

/// Common trait for RIO format renderers.
pub trait RioRenderer: Send + Sync {
    fn serialize(&self, ontology: &Ontology) -> Result<String>;
}
