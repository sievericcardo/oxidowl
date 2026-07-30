use std::path::{Path, PathBuf};

use oxidowl::manager::changes::OntologyChange;
use oxidowl::manager::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::*;
use oxidowl::DataFactory;
use oxidowl::OntologyManager;
use tempfile::TempDir;

use super::df::DF;
use super::roundtrip::RoundtripHarness;

/// Shared test infrastructure — the Rust equivalent of
/// OWL API v5's `TestBase.java`.
pub struct TestBase {
    pub df: DF,
    pub manager: OntologyManager,
    pub temp_dir: TempDir,
    pub resource_dir: Option<PathBuf>,
    pub roundtrip_harness: RoundtripHarness,
}

impl TestBase {
    pub fn new() -> Self {
        let resource_dir = Self::find_resource_dir();
        TestBase {
            df: DF::new(),
            manager: OntologyManager::new(),
            temp_dir: TempDir::new().expect("Failed to create temp dir"),
            resource_dir,
            roundtrip_harness: RoundtripHarness::default(),
        }
    }

    pub fn with_config(config: ManagerConfig) -> Self {
        TestBase {
            df: DF::new(),
            manager: OntologyManager::new_with_config(config),
            temp_dir: TempDir::new().expect("Failed to create temp dir"),
            resource_dir: Self::find_resource_dir(),
            roundtrip_harness: RoundtripHarness::default(),
        }
    }

    fn find_resource_dir() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("ontologies-test"),
            PathBuf::from("../ontologies-test"),
            PathBuf::from("../../ontologies-test"),
        ];
        for c in &candidates {
            if c.exists() && c.is_dir() {
                return Some(c.clone());
            }
        }
        None
    }

    pub fn resources(&self) -> &Path {
        self.resource_dir
            .as_ref()
            .expect("ontologies-test directory not found")
            .as_path()
    }

    pub fn create_manager(&self) -> OntologyManager {
        OntologyManager::new()
    }

    pub fn create_data_factory(&self) -> DataFactory {
        DataFactory::new()
    }

    pub fn create_ontology(&mut self, iri: &str) -> OntologyRef {
        self.manager.create_ontology(IRI::new(iri))
    }

    pub fn create_anon_ontology(&mut self) -> OntologyRef {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ANON_COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = ANON_COUNTER.fetch_add(1, Ordering::Relaxed);
        let iri = IRI::new(&format!("urn:anon:{id}"));
        self.manager.create_ontology(iri)
    }

    // ── Loading helpers ─────────────────────────────────────────────────────

    pub fn load_test_ontology<S: AsRef<str>>(
        &mut self,
        filename: S,
    ) -> Result<OntologyRef, String> {
        let path = self.resources().join(filename.as_ref());
        let format = Self::detect_format(&path);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        self.load_from_string(&content, format)
    }

    pub fn load_from_string(
        &mut self,
        content: &str,
        format: OntologyFormat,
    ) -> Result<OntologyRef, String> {
        let ontology = Self::parse_string(content, format)?;
        let ontology_iri = ontology
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:anon:loaded"));
        Ok(self
            .manager
            .create_ontology_with_axioms(ontology_iri, ontology.axioms().to_vec()))
    }

    pub fn load_and_get_ontology(
        &self,
        content: &str,
        format: OntologyFormat,
    ) -> Result<Ontology, String> {
        Self::parse_string(content, format)
    }

    fn parse_string(content: &str, format: OntologyFormat) -> Result<Ontology, String> {
        match format {
            OntologyFormat::Functional => parse_functional(content).map_err(|e| format!("{e}")),
            OntologyFormat::OwlXml => parse_owl_xml(content).map_err(|e| format!("{e}")),
            OntologyFormat::RdfXml => parse_rdf_xml(content).map_err(|e| format!("{e}")),
            OntologyFormat::Turtle => parse_turtle(content).map_err(|e| format!("{e}")),
            OntologyFormat::NTriples => parse_ntriples(content).map_err(|e| format!("{e}")),
            _ => Err(format!("Parsing for {:?} not supported", format)),
        }
    }

    pub fn load_from_text(
        &mut self,
        content: &str,
        format: OntologyFormat,
    ) -> Result<OntologyRef, String> {
        self.load_from_string(content, format)
    }

    pub fn detect_format(path: &Path) -> OntologyFormat {
        match path.extension().and_then(|e| e.to_str()) {
            Some("ofn") | Some("fss") => OntologyFormat::Functional,
            Some("owx") | Some("owlx") => OntologyFormat::OwlXml,
            Some("rdf") | Some("xml") => OntologyFormat::RdfXml,
            Some("ttl") => OntologyFormat::Turtle,
            Some("nt") => OntologyFormat::NTriples,
            Some("omn") | Some("mn") => OntologyFormat::Manchester,
            Some("obo") => OntologyFormat::Obo,
            Some("jsonld") | Some("json") => OntologyFormat::JsonLd,
            _ => OntologyFormat::RdfXml,
        }
    }

    // ── Saving helpers ──────────────────────────────────────────────────────

    pub fn save_to_string(
        &self,
        ontology: &Ontology,
        format: OntologyFormat,
    ) -> Result<String, String> {
        save_to_string(ontology, format).map_err(|e| format!("{e}"))
    }

    pub fn save_ontology_to_file<P: AsRef<Path>>(
        &self,
        ontology: &Ontology,
        path: P,
        format: OntologyFormat,
    ) -> Result<(), String> {
        save_file(ontology, path, format).map_err(|e| format!("{e}"))
    }

    pub fn save_to_temp_file(
        &self,
        ontology: &Ontology,
        format: OntologyFormat,
    ) -> Result<PathBuf, String> {
        let ext = match format {
            OntologyFormat::Functional => "ofn",
            OntologyFormat::OwlXml => "owx",
            OntologyFormat::RdfXml => "rdf",
            OntologyFormat::Turtle => "ttl",
            OntologyFormat::NTriples => "nt",
            OntologyFormat::Manchester => "omn",
            _ => "owl",
        };
        let file_path = self.temp_dir.path().join(format!("roundtrip.{ext}"));
        save_file(ontology, &file_path, format).map_err(|e| format!("{e}"))?;
        Ok(file_path)
    }

    // ── Roundtrip helpers ───────────────────────────────────────────────────

    pub fn round_trip(
        &mut self,
        ontology: &Ontology,
        format: OntologyFormat,
    ) -> Result<Ontology, String> {
        let serialized = self.save_to_string(ontology, format)?;
        self.load_and_get_ontology(&serialized, format)
    }

    pub fn round_trip_and_compare(
        &mut self,
        ontology: &Ontology,
        format: OntologyFormat,
    ) -> Result<(), String> {
        let reloaded = self.round_trip(ontology, format)?;
        super::assertions::assert_ontologies_axiom_equal(ontology, &reloaded);
        Ok(())
    }

    pub fn cross_format_roundtrip(
        &mut self,
        ontology: &Ontology,
        fmt_from: OntologyFormat,
        fmt_to: OntologyFormat,
    ) -> Result<(), String> {
        let serialized = self.save_to_string(ontology, fmt_from)?;
        let reloaded = self.load_and_get_ontology(&serialized, fmt_to)?;
        super::assertions::assert_ontologies_axiom_equal(ontology, &reloaded);
        Ok(())
    }

    pub fn plain_equal(
        &mut self,
        ontology: &Ontology,
        compare_input: bool,
    ) -> Result<(), String> {
        let o1 = self.round_trip(ontology, OntologyFormat::RdfXml)?;
        let o2 = self.round_trip(ontology, OntologyFormat::Functional)?;
        if compare_input {
            super::assertions::assert_ontologies_axiom_equal(ontology, &o1);
        }
        super::assertions::assert_ontologies_axiom_equal(&o1, &o2);
        Ok(())
    }

    // ── Manager helpers ─────────────────────────────────────────────────────

    pub fn create_onto(&mut self, iri: &str) -> OntologyRef {
        self.manager.create_ontology(IRI::new(iri))
    }

    pub fn apply_add_axiom(&mut self, ont_ref: &OntologyRef, axiom: Axiom) {
        let iri = ont_ref.read().unwrap().get_iri().cloned().unwrap();
        self.manager
            .apply_change(OntologyChange::AddAxiom {
                ontology_iri: iri,
                axiom,
            });
    }

    pub fn core_roundtrip_formats() -> Vec<OntologyFormat> {
        vec![
            OntologyFormat::RdfXml,
            OntologyFormat::Functional,
            OntologyFormat::OwlXml,
            OntologyFormat::Turtle,
        ]
    }

    pub fn all_roundtrip_formats() -> Vec<OntologyFormat> {
        vec![
            OntologyFormat::RdfXml,
            OntologyFormat::Functional,
            OntologyFormat::OwlXml,
            OntologyFormat::Turtle,
            OntologyFormat::NTriples,
        ]
    }
}

impl Default for TestBase {
    fn default() -> Self {
        Self::new()
    }
}
