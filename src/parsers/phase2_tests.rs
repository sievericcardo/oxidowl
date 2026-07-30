#[cfg(test)]
mod phase2_tests {
    use crate::ontology::{
        Class, ClassExpression, DataProperty, DataPropertyExpression, DataRange, Individual,
        Literal, ObjectProperty, ObjectPropertyExpression, Ontology, OntologyFormat, OntologyRef, IRI,
    };
    use crate::ontology::axioms::{
        Axiom, ClassAssertionAxiom, DifferentIndividualsAxiom, DisjointClassesAxiom,
        EquivalentClassesAxiom, ObjectPropertyAssertionAxiom, SameIndividualAxiom,
        SubClassOfAxiom, SubObjectPropertyOfAxiom, TransitiveObjectPropertyAxiom,
    };
    use crate::ontology::individuals::NamedIndividual;
    use crate::parsers::{
        manchester_renderer::ManchesterRenderer,
        latex::LatexRenderer,
        dl_syntax::{DLSyntaxParser, DLSyntaxRenderer},
        krss::{KRSSParser, KRSSRenderer, KRSSVariant},
    };
    use crate::Result;

    fn make_test_ontology() -> Ontology {
        let mut o = Ontology::new();
        let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
        let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
        let c = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/C") });
        o.set_iri(IRI::new("http://ex.org/TestOnt"));
        o.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1, subclass: a.clone(), superclass: b.clone(), annotations: vec![],
        }));
        o.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 2, classes: vec![b.clone(), c.clone()], annotations: vec![],
        }));
        o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 3,
            class: a.clone(),
            individual: Individual::Named(NamedIndividual { iri: IRI::new("http://ex.org/ind") }),
            annotations: vec![],
        }));
        o
    }

    // ── Manchester Renderer Tests ────────────────────────────────────────────

    #[test]
    fn test_manchester_serialize_basic() -> Result<()> {
        let o = make_test_ontology();
        let renderer = ManchesterRenderer::new();
        let output = renderer.serialize(&o)?;
        assert!(output.contains("Ontology:"));
        assert!(output.contains("SubClassOf:"));
        assert!(output.contains("EquivalentTo:"));
        assert!(output.contains("Types:"));
        Ok(())
    }

    #[test]
    fn test_manchester_serialize_empty() -> Result<()> {
        let o = Ontology::new();
        let renderer = ManchesterRenderer::new();
        let output = renderer.serialize(&o)?;
        assert!(!output.contains("Class:"));
        Ok(())
    }

    #[test]
    fn test_manchester_serialize_class_expression() {
        let renderer = ManchesterRenderer::new();
        let class = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
        let rendered = renderer.render_ce(&class);
        assert!(rendered.contains("http://ex.org/A"));
    }

    #[test]
    fn test_manchester_serialize_intersection() {
        let renderer = ManchesterRenderer::new();
        let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
        let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
        let intersection = ClassExpression::ObjectIntersectionOf(vec![a, b]);
        let rendered = renderer.render_ce(&intersection);
        assert!(rendered.contains(" and "));
    }

    #[test]
    fn test_manchester_serialize_some_values() {
        let renderer = ManchesterRenderer::new();
        let c = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/C") });
        let some = ClassExpression::ObjectSomeValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: IRI::new("http://ex.org/prop") }),
            filler: Box::new(c),
        };
        let rendered = renderer.render_ce(&some);
        assert!(rendered.contains("some"));
    }

    #[test]
    fn test_manchester_serialize_cardinality() {
        let renderer = ManchesterRenderer::new();
        let c = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/C") });
        let min = ClassExpression::ObjectMinCardinality {
            property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: IRI::new("http://ex.org/prop") }),
            cardinality: 2,
            filler: Box::new(c),
        };
        let rendered = renderer.render_ce(&min);
        assert!(rendered.contains("min 2"));
    }

    // ── LaTeX Renderer Tests ─────────────────────────────────────────────────

    #[test]
    fn test_latex_serialize_document() -> Result<()> {
        let o = make_test_ontology();
        let config = crate::parsers::latex::LatexConfig::default();
        let renderer = LatexRenderer::new();
        let output = renderer.render_document(&o, &config)?;
        assert!(output.contains("\\documentclass"));
        assert!(output.contains("\\begin{document}"));
        assert!(output.contains("\\sqsubseteq"));
        assert!(output.contains("\\end{document}"));
        Ok(())
    }

    #[test]
    fn test_latex_serialize_to_file() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::latex::save_file(&o, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("\\documentclass"));
        Ok(())
    }

    // ── DL Syntax Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_dl_syntax_render_basic() -> Result<()> {
        let o = make_test_ontology();
        let renderer = DLSyntaxRenderer::new(true);
        let output = renderer.serialize(&o)?;
        assert!(output.contains("\u{2291}")); // subsume
        Ok(())
    }

    #[test]
    fn test_dl_syntax_render_ascii() -> Result<()> {
        let o = make_test_ontology();
        let renderer = DLSyntaxRenderer::new(false);
        let output = renderer.serialize(&o)?;
        assert!(output.contains("sqsubseteq"));
        Ok(())
    }

    #[test]
    fn test_dl_syntax_parse_basic() -> Result<()> {
        let mut parser = DLSyntaxParser::new();
        let o = parser.parse("A \u{2291} B")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_dl_syntax_parse_intersection() -> Result<()> {
        let mut parser = DLSyntaxParser::new();
        let o = parser.parse("C \u{2291} A \u{2293} B")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_dl_syntax_parse_one_of() -> Result<()> {
        let mut parser = DLSyntaxParser::new();
        let o = parser.parse("A \u{2291} {a, b, c}")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_dl_syntax_save_file() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::dl_syntax::save_file(&o, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("\u{2291}"));
        Ok(())
    }

    // ── KRSS Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_krss_render_basic() -> Result<()> {
        let o = make_test_ontology();
        let renderer = KRSSRenderer::new(KRSSVariant::KRSS);
        let output = renderer.serialize(&o)?;
        assert!(output.contains("implies"));
        Ok(())
    }

    #[test]
    fn test_krss_parse_basic() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(implies A B)")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_parse_implies() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(implies A (and B C))")?;
        assert_eq!(o.axioms().len(), 1);
        Ok(())
    }

    #[test]
    fn test_krss_parse_instance() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(instance a C)")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_parse_related() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(related a R b)")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_parse_disjoint() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(disjoint A B)")?;
        assert_eq!(o.axioms().len(), 1);
        Ok(())
    }

    #[test]
    fn test_krss_parse_define_primitive() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(define-primitive-concept A (and B C))")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_parse_define_concept() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(define-concept D (some R C))")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_parse_equal() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(equal a b c)")?;
        assert_eq!(o.axioms().len(), 1);
        Ok(())
    }

    #[test]
    fn test_krss_parse_distinct() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(distinct a b)")?;
        assert_eq!(o.axioms().len(), 1);
        Ok(())
    }

    #[test]
    fn test_krss_parse_role_def() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS);
        let o = parser.parse("(define-primitive-role R :parent P)")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss2_parse_characteristic() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS2);
        let o = parser.parse("(transitive R)")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss2_parse_one_of() -> Result<()> {
        let mut parser = KRSSParser::new(KRSSVariant::KRSS2);
        let o = parser.parse("(define-concept E (one-of a b c))")?;
        assert!(!o.axioms().is_empty());
        Ok(())
    }

    #[test]
    fn test_krss_save_file() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::krss::save_file(&o, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("implies"));
        Ok(())
    }

    // ── Format Dispatch Tests ─────────────────────────────────────────────────

    #[test]
    fn test_save_manchester_via_dispatch() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::save_file(&o, tmp.path(), OntologyFormat::Manchester)?;
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("SubClassOf:"));
        Ok(())
    }

    #[test]
    fn test_save_latex_via_dispatch() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::save_file(&o, tmp.path(), OntologyFormat::Latex)?;
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("\\documentclass"));
        Ok(())
    }

    #[test]
    fn test_save_dl_via_dispatch() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::save_file(&o, tmp.path(), OntologyFormat::DL)?;
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("\u{2291}"));
        Ok(())
    }

    #[test]
    fn test_save_krss_via_dispatch() -> Result<()> {
        let o = make_test_ontology();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::parsers::save_file(&o, tmp.path(), OntologyFormat::Krss)?;
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("implies"));
        Ok(())
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(OntologyFormat::Latex.extension(), "tex");
        assert_eq!(OntologyFormat::DL.extension(), "dl");
        assert_eq!(OntologyFormat::Krss.extension(), "krss");
        assert_eq!(OntologyFormat::Krss2.extension(), "krss2");
    }

    #[test]
    fn test_format_detection() {
        assert_eq!(OntologyFormat::from_extension("tex"), Some(OntologyFormat::Latex));
        assert_eq!(OntologyFormat::from_extension("dl"), Some(OntologyFormat::DL));
        assert_eq!(OntologyFormat::from_extension("krss"), Some(OntologyFormat::Krss));
        assert_eq!(OntologyFormat::from_extension("krss2"), Some(OntologyFormat::Krss2));
    }

    #[test]
    fn test_format_strings() {
        assert_eq!(OntologyFormat::Latex.format_string(), "latex");
        assert_eq!(OntologyFormat::DL.format_string(), "dl");
        assert_eq!(OntologyFormat::Krss.format_string(), "krss");
        assert_eq!(OntologyFormat::Krss2.format_string(), "krss2");
    }
}
