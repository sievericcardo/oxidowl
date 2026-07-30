//! RIO (RDF Input/Output) Integration Formats.
//!
//! Provides parsers and renderers for additional RDF serialization
//! formats beyond the core six.

pub mod binary_rdf;
pub mod hdt;
pub mod jsonld;
pub mod n3;
pub mod nquads;
pub mod rdf_json;
pub mod rdfa;
pub mod trig;
pub mod trix;

use crate::Result;
use crate::ontology::Ontology;

/// Common trait for RIO format parsers.
pub trait RioParser: Send + Sync {
    fn parse(&self, content: &str) -> Result<Ontology>;
}

/// Common trait for RIO format renderers.
pub trait RioRenderer: Send + Sync {
    fn serialize(&self, ontology: &Ontology) -> Result<String>;
}
