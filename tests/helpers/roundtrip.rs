use oxidowl::ontology::*;
use oxidowl::parsers::*;

/// Roundtrip test harness for systematic format roundtrip verification.
///
/// Models the approach from OWL API v5's `AbstractRoundTrippingTestCase`
/// and `RoundtripHarness`.
pub struct RoundtripHarness {
    pub strict_mode: bool,
    pub normalize_iris: bool,
}

impl Default for RoundtripHarness {
    fn default() -> Self {
        RoundtripHarness {
            strict_mode: false,
            normalize_iris: false,
        }
    }
}

impl RoundtripHarness {
    pub fn new(strict: bool) -> Self {
        RoundtripHarness {
            strict_mode: strict,
            normalize_iris: false,
        }
    }

    pub fn with_iri_normalization(mut self) -> Self {
        self.normalize_iris = true;
        self
    }

    /// Parse → serialize → re-parse → compare axiom counts.
    ///
    /// Returns `(original_axiom_count, reparse_axiom_count)`.
    pub fn test_roundtrip(
        &self,
        content: &str,
        format: OntologyFormat,
    ) -> Result<RoundtripReport, Box<dyn std::error::Error>> {
        let orig = self.parse(content, format)?;
        let orig_count = orig.axioms().len();

        let serialized = save_to_string(&orig, format)
            .map_err(|e| format!("Serialize error: {e}"))?;

        let reparsed = self.parse(&serialized, format)?;
        let reparsed_count = reparsed.axioms().len();

        let missing: Vec<_> = orig
            .axioms()
            .iter()
            .filter(|a| !reparsed.axioms().contains(a))
            .cloned()
            .collect();

        let extra: Vec<_> = reparsed
            .axioms()
            .iter()
            .filter(|a| !orig.axioms().contains(a))
            .cloned()
            .collect();

        Ok(RoundtripReport {
            original_axiom_count: orig_count,
            reparse_axiom_count: reparsed_count,
            missing_axioms: missing.len(),
            extra_axioms: extra.len(),
            passed: missing.is_empty() && extra.is_empty(),
            missing,
            extra,
        })
    }

    /// Roundtrip through the given format and assert equality.
    pub fn roundtrip_assert(
        &self,
        content: &str,
        format: OntologyFormat,
    ) {
        let report = self.test_roundtrip(content, format).unwrap();
        assert!(
            report.passed,
            "Roundtrip {} failed: missing={}, extra={}\nMissing: {:?}\nExtra: {:?}",
            format.format_string(),
            report.missing_axioms,
            report.extra_axioms,
            report.missing,
            report.extra,
        );
    }

    /// Cross-format: parse in one format, save in another, re-parse in the
    /// second, and compare axiom counts.
    pub fn test_cross_format(
        &self,
        content: &str,
        from: OntologyFormat,
        to: OntologyFormat,
    ) -> Result<CrossFormatReport, Box<dyn std::error::Error>> {
        let orig = self.parse(content, from)?;
        let orig_count = orig.axioms().len();

        let serialized = save_to_string(&orig, to)
            .map_err(|e| format!("Serialize to {:?} error: {e}", to))?;

        let reparsed = self.parse(&serialized, to)?;
        let reparsed_count = reparsed.axioms().len();

        Ok(CrossFormatReport {
            original_axiom_count: orig_count,
            reparse_axiom_count: reparsed_count,
            passed: orig_count <= reparsed_count, // different formats may add declarations
            is_semantically_equivalent: are_ontologies_semantically_equivalent(&orig, &reparsed),
        })
    }

    fn parse(
        &self,
        content: &str,
        format: OntologyFormat,
    ) -> Result<Ontology, Box<dyn std::error::Error>> {
        let ont = match format {
            OntologyFormat::Functional => parse_functional(content)?,
            OntologyFormat::OwlXml => parse_owl_xml(content)?,
            OntologyFormat::RdfXml => parse_rdf_xml(content)?,
            OntologyFormat::Turtle => parse_turtle(content)?,
            OntologyFormat::NTriples => parse_ntriples(content)?,
            _ => {
                return Err(
                    format!("Parsing for {:?} not implemented in harness", format).into(),
                )
            }
        };
        Ok(ont)
    }
}

pub struct RoundtripReport {
    pub original_axiom_count: usize,
    pub reparse_axiom_count: usize,
    pub missing_axioms: usize,
    pub extra_axioms: usize,
    pub passed: bool,
    pub missing: Vec<oxidowl::ontology::axioms::Axiom>,
    pub extra: Vec<oxidowl::ontology::axioms::Axiom>,
}

pub struct CrossFormatReport {
    pub original_axiom_count: usize,
    pub reparse_axiom_count: usize,
    pub passed: bool,
    pub is_semantically_equivalent: bool,
}

/// Quick check: two ontologies have the same number of axioms and
/// all of ont1's axioms appear in ont2.
fn are_ontologies_semantically_equivalent(o1: &Ontology, o2: &Ontology) -> bool {
    if o1.axioms().len() != o2.axioms().len() {
        return false;
    }
    o1.axioms().iter().all(|a| o2.axioms().contains(a))
}
