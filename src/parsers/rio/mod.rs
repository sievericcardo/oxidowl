//! RIO (RDF Input/Output) Integration Formats.
//!
//! Provides parsers and renderers for additional RDF serialization
//! formats beyond the core six.  Each module exposes free `parse` /
//! `save_file` functions and a `*Parser` / `*Renderer` type dispatched
//! through the format registry (`ParserFactory` / `OntologyFormat`).

pub mod binary_rdf;
pub mod hdt;
pub mod jsonld;
pub mod n3;
pub mod nquads;
pub mod rdf_json;
pub mod rdfa;
mod rcdom;
pub mod trig;
pub mod trix;
