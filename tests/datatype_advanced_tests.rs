#[path = "helpers/mod.rs"]
mod helpers;

use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::datatypes::DatatypeDefinitionAxiom;
use oxidowl::ontology::*;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn xsd_iri(local: &str) -> IRI {
    IRI::new(&format!("{XSD}{local}"))
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 DataRange Construction Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Create DataComplementOf(Integer) and verify structure.
#[test]
fn test_datatype_complement() {
    let dt = xsd_iri("integer");
    let complement = DataRange::DataComplementOf(Box::new(DataRange::Datatype(dt)));
    match &complement {
        DataRange::DataComplementOf(inner) => match inner.as_ref() {
            DataRange::Datatype(dt_iri) => {
                assert!(dt_iri.as_str().contains("integer"));
            }
            _ => panic!("Expected Datatype variant"),
        },
        _ => panic!("Expected DataComplementOf"),
    }
}

/// Create DataIntersectionOf([Integer, NonNegativeInteger]).
#[test]
fn test_datatype_intersection() {
    let integer = DataRange::Datatype(xsd_iri("integer"));
    let non_neg = DataRange::Datatype(xsd_iri("nonNegativeInteger"));
    let intersection = DataRange::DataIntersectionOf(vec![integer, non_neg]);

    match &intersection {
        DataRange::DataIntersectionOf(ranges) => {
            assert_eq!(ranges.len(), 2, "Should contain exactly 2 data ranges");
            assert!(matches!(&ranges[0], DataRange::Datatype(_)));
            assert!(matches!(&ranges[1], DataRange::Datatype(_)));
        }
        _ => panic!("Expected DataIntersectionOf"),
    }
}

/// Create DataUnionOf([String, Integer]).
#[test]
fn test_datatype_union() {
    let string_dt = DataRange::Datatype(xsd_iri("string"));
    let integer_dt = DataRange::Datatype(xsd_iri("integer"));
    let union = DataRange::DataUnionOf(vec![string_dt, integer_dt]);

    match &union {
        DataRange::DataUnionOf(ranges) => {
            assert_eq!(ranges.len(), 2, "Should contain exactly 2 data ranges");
        }
        _ => panic!("Expected DataUnionOf"),
    }
}

/// Create DatatypeRestriction with facet restrictions.
#[test]
fn test_datatype_restriction() {
    let integer = xsd_iri("integer");
    let facet_min = FacetRestriction {
        facet: xsd_iri("minInclusive"),
        value: Literal::with_datatype("0".to_string(), xsd_iri("integer")),
    };
    let facet_max = FacetRestriction {
        facet: xsd_iri("maxInclusive"),
        value: Literal::with_datatype("100".to_string(), xsd_iri("integer")),
    };

    let restriction = DataRange::DatatypeRestriction {
        datatype: integer,
        restrictions: vec![facet_min, facet_max],
    };

    match &restriction {
        DataRange::DatatypeRestriction {
            datatype,
            restrictions,
        } => {
            assert!(datatype.as_str().contains("integer"));
            assert_eq!(restrictions.len(), 2);
            assert!(restrictions[0].facet.as_str().contains("minInclusive"));
            assert!(restrictions[1].facet.as_str().contains("maxInclusive"));
        }
        _ => panic!("Expected DatatypeRestriction"),
    }
}

/// Create DataOneOf with literal values.
#[test]
fn test_datatype_one_of() {
    let lit1 = Literal::with_datatype("1".to_string(), xsd_iri("integer"));
    let lit2 = Literal::with_datatype("2".to_string(), xsd_iri("integer"));
    let lit3 = Literal::with_datatype("3".to_string(), xsd_iri("integer"));

    let one_of = DataRange::DataOneOf(vec![lit1, lit2, lit3]);

    match &one_of {
        DataRange::DataOneOf(literals) => {
            assert_eq!(literals.len(), 3, "Should contain exactly 3 literals");
            assert_eq!(literals[0].value, "1");
            assert_eq!(literals[1].value, "2");
            assert_eq!(literals[2].value, "3");
        }
        _ => panic!("Expected DataOneOf"),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 OWL2Datatype IRI Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Verify all OWL2Datatype variants map to correct IRIs.
#[test]
fn test_all_owl2_datatype_iris() {
    // Core XSD datatypes
    assert_eq!(
        OWL2Datatype::String.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#string"
    );
    assert_eq!(
        OWL2Datatype::Boolean.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#boolean"
    );
    assert_eq!(
        OWL2Datatype::Decimal.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#decimal"
    );
    assert_eq!(
        OWL2Datatype::Float.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#float"
    );
    assert_eq!(
        OWL2Datatype::Double.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#double"
    );
    assert_eq!(
        OWL2Datatype::DateTime.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#dateTime"
    );
    assert_eq!(
        OWL2Datatype::Time.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#time"
    );
    assert_eq!(
        OWL2Datatype::Date.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#date"
    );
    assert_eq!(
        OWL2Datatype::GYearMonth.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#gYearMonth"
    );
    assert_eq!(
        OWL2Datatype::GYear.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#gYear"
    );
    assert_eq!(
        OWL2Datatype::GMonthDay.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#gMonthDay"
    );
    assert_eq!(
        OWL2Datatype::GDay.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#gDay"
    );
    assert_eq!(
        OWL2Datatype::GMonth.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#gMonth"
    );
    assert_eq!(
        OWL2Datatype::Duration.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#duration"
    );
    assert_eq!(
        OWL2Datatype::DateTimeStamp.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#dateTimeStamp"
    );
    assert_eq!(
        OWL2Datatype::Base64Binary.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#base64Binary"
    );
    assert_eq!(
        OWL2Datatype::HexBinary.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#hexBinary"
    );
    assert_eq!(
        OWL2Datatype::AnyURI.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#anyURI"
    );

    // Numeric hierarchy
    assert_eq!(
        OWL2Datatype::Integer.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#integer"
    );
    assert_eq!(
        OWL2Datatype::NonNegativeInteger.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
    );
    assert_eq!(
        OWL2Datatype::PositiveInteger.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#positiveInteger"
    );
    assert_eq!(
        OWL2Datatype::NegativeInteger.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#negativeInteger"
    );
    assert_eq!(
        OWL2Datatype::Long.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#long"
    );
    assert_eq!(
        OWL2Datatype::Int.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#int"
    );
    assert_eq!(
        OWL2Datatype::Short.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#short"
    );
    assert_eq!(
        OWL2Datatype::Byte.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#byte"
    );
    assert_eq!(
        OWL2Datatype::UnsignedLong.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#unsignedLong"
    );
    assert_eq!(
        OWL2Datatype::UnsignedInt.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#unsignedInt"
    );
    assert_eq!(
        OWL2Datatype::UnsignedShort.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#unsignedShort"
    );
    assert_eq!(
        OWL2Datatype::UnsignedByte.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#unsignedByte"
    );

    // RDF datatypes
    assert_eq!(
        OWL2Datatype::XMLLiteral.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral"
    );
    assert_eq!(
        OWL2Datatype::Literal.iri().as_str(),
        "http://www.w3.org/2000/01/rdf-schema#Literal"
    );
    assert_eq!(
        OWL2Datatype::PlainLiteral.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral"
    );

    // OWL 2 specific
    assert_eq!(
        OWL2Datatype::Real.iri().as_str(),
        "http://www.w3.org/2002/07/owl#real"
    );
    assert_eq!(
        OWL2Datatype::Rational.iri().as_str(),
        "http://www.w3.org/2002/07/owl#rational"
    );

    // Extended string subtypes
    assert_eq!(
        OWL2Datatype::NormalizedString.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#normalizedString"
    );
    assert_eq!(
        OWL2Datatype::Token.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#token"
    );
    assert_eq!(
        OWL2Datatype::Language.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#language"
    );
    assert_eq!(
        OWL2Datatype::Name.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#Name"
    );
    assert_eq!(
        OWL2Datatype::NCName.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#NCName"
    );
    assert_eq!(
        OWL2Datatype::NMTOKEN.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#NMTOKEN"
    );
    assert_eq!(
        OWL2Datatype::NMTOKENS.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#NMTOKENS"
    );

    // Duration subtypes
    assert_eq!(
        OWL2Datatype::DayTimeDuration.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#dayTimeDuration"
    );
    assert_eq!(
        OWL2Datatype::YearMonthDuration.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#yearMonthDuration"
    );

    // RDF extended
    assert_eq!(
        OWL2Datatype::LangString.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
    );
    assert_eq!(
        OWL2Datatype::RdfText.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML"
    );

    // OWL extended aliases
    assert_eq!(
        OWL2Datatype::RdfPlainLiteral.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral"
    );
    assert_eq!(
        OWL2Datatype::RdfXMLLiteral.iri().as_str(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 Datatype Subtype Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Verify transitive subtype: Integer < Decimal < Real.
#[test]
fn test_datatype_subtype_transitive() {
    assert!(OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Decimal));
    assert!(OWL2Datatype::Decimal.is_subtype_of(&OWL2Datatype::Real));
    assert!(
        OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Real),
        "Integer should be transitive subtype of Real"
    );

    assert!(OWL2Datatype::Int.is_subtype_of(&OWL2Datatype::Long));
    assert!(OWL2Datatype::Long.is_subtype_of(&OWL2Datatype::Integer));
    assert!(OWL2Datatype::Int.is_subtype_of(&OWL2Datatype::Integer));

    assert!(!OWL2Datatype::String.is_subtype_of(&OWL2Datatype::Integer));
    assert!(!OWL2Datatype::Real.is_subtype_of(&OWL2Datatype::Integer));
    assert!(OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Integer));

    assert!(OWL2Datatype::UnsignedInt.is_subtype_of(&OWL2Datatype::UnsignedLong));
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 Facet Restriction Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Length facet restriction on string datatype.
#[test]
fn test_facet_restriction_length() {
    let string_dt = xsd_iri("string");
    let length_iri = xsd_iri("length");
    let length_value = Literal::with_datatype("10".to_string(), xsd_iri("integer"));

    let fr = FacetRestriction {
        facet: length_iri.clone(),
        value: length_value.clone(),
    };

    assert!(
        fr.facet.as_str().contains("length"),
        "Facet IRI should contain 'length': {}",
        fr.facet.as_str()
    );
    assert_eq!(fr.value.value, "10");
    assert!(
        fr.value.datatype.is_some(),
        "Length value should have integer datatype"
    );

    let restriction = DataRange::DatatypeRestriction {
        datatype: string_dt,
        restrictions: vec![fr],
    };

    match &restriction {
        DataRange::DatatypeRestriction {
            datatype,
            restrictions,
        } => {
            assert!(datatype.as_str().contains("string"));
            assert_eq!(restrictions.len(), 1);
            assert!(restrictions[0].facet.as_str().contains("length"));
            assert_eq!(restrictions[0].value.value, "10");
        }
        _ => panic!("Expected DatatypeRestriction"),
    }
}

/// Min/Max inclusive facet restriction on numeric datatype.
#[test]
fn test_facet_restriction_min_max() {
    let integer = xsd_iri("integer");
    let min_inc = FacetRestriction {
        facet: xsd_iri("minInclusive"),
        value: Literal::with_datatype("0".to_string(), xsd_iri("integer")),
    };
    let max_exc = FacetRestriction {
        facet: xsd_iri("maxExclusive"),
        value: Literal::with_datatype("100".to_string(), xsd_iri("integer")),
    };

    let restriction = DataRange::DatatypeRestriction {
        datatype: integer,
        restrictions: vec![min_inc, max_exc],
    };

    match &restriction {
        DataRange::DatatypeRestriction { restrictions, .. } => {
            assert_eq!(restrictions.len(), 2);
            assert!(restrictions[0].facet.as_str().contains("minInclusive"));
            assert_eq!(restrictions[0].value.value, "0");
            assert!(restrictions[1].facet.as_str().contains("maxExclusive"));
            assert_eq!(restrictions[1].value.value, "100");
        }
        _ => panic!("Expected DatatypeRestriction"),
    }
}

/// Pattern restriction on string datatype.
#[test]
fn test_facet_restriction_pattern() {
    let string_dt = xsd_iri("string");
    let pattern_fr = FacetRestriction {
        facet: xsd_iri("pattern"),
        value: Literal::new("[A-Z][a-z]+".to_string()),
    };

    let restriction = DataRange::DatatypeRestriction {
        datatype: string_dt,
        restrictions: vec![pattern_fr],
    };

    match &restriction {
        DataRange::DatatypeRestriction {
            datatype,
            restrictions,
        } => {
            assert!(datatype.as_str().contains("string"));
            assert_eq!(restrictions.len(), 1);
            assert!(restrictions[0].facet.as_str().contains("pattern"));
            assert_eq!(restrictions[0].value.value, "[A-Z][a-z]+");
        }
        _ => panic!("Expected DatatypeRestriction"),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 Literal Construction Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Scientific notation literal creation.
#[test]
fn test_literal_scientific_notation() {
    let lit1 = Literal::with_datatype("1.5e10".to_string(), xsd_iri("double"));
    assert_eq!(lit1.value, "1.5e10");
    assert!(lit1.datatype.is_some());
    assert!(lit1.language.is_none());

    let lit2 = Literal::with_datatype("2.3E-4".to_string(), xsd_iri("double"));
    assert_eq!(lit2.value, "2.3E-4");
    assert!(lit2.datatype.is_some());

    let lit3 = Literal::with_datatype("3.14".to_string(), xsd_iri("decimal"));
    assert_eq!(lit3.value, "3.14");

    let lit4 = Literal::with_datatype("-9.81".to_string(), xsd_iri("float"));
    assert_eq!(lit4.value, "-9.81");
}

/// String escaping in literal values.
#[test]
fn test_literal_escaping() {
    let newline = Literal::new("line1\nline2".to_string());
    assert_eq!(newline.value, "line1\nline2");
    assert!(newline.value.contains('\n'));

    let quote = Literal::new("He said \"hello\"".to_string());
    assert_eq!(quote.value, "He said \"hello\"");
    assert!(quote.value.contains('"'));

    let backslash = Literal::new("path\\to\\file".to_string());
    assert_eq!(backslash.value, "path\\to\\file");
    assert!(backslash.value.contains('\\'));

    let tab = Literal::new("col1\tcol2".to_string());
    assert_eq!(tab.value, "col1\tcol2");
    assert!(tab.value.contains('\t'));

    let unicode = Literal::new("cafe\u{0301}".to_string());
    assert_eq!(unicode.value, "cafe\u{0301}");
}

/// Language tag normalization (case handling).
#[test]
fn test_language_tag_normalization() {
    let lit_en = Literal::with_language("hello".to_string(), "en".to_string());
    assert_eq!(lit_en.language.as_deref(), Some("en"));
    assert_eq!(lit_en.value, "hello");

    let lit_en_upper = Literal::with_language("hello".to_string(), "EN".to_string());
    assert_eq!(lit_en_upper.language.as_deref(), Some("EN"));
    assert_ne!(
        lit_en.language, lit_en_upper.language,
        "Language tags 'en' and 'EN' are case-sensitive in structural form"
    );

    let lit_en_gb = Literal::with_language("colour".to_string(), "en-GB".to_string());
    assert_eq!(lit_en_gb.language.as_deref(), Some("en-GB"));

    let lit_fr = Literal::with_language("bonjour".to_string(), "fr".to_string());
    assert_eq!(lit_fr.language.as_deref(), Some("fr"));
}

/// Plain literal vs xsd:string type folding.
#[test]
fn test_plain_literal_type_folding() {
    let plain = Literal::new("abc".to_string());
    assert!(plain.language.is_none());
    assert!(plain.datatype.is_none());
    assert_eq!(plain.value, "abc");

    let _xsd_string_url = xsd_iri("string").to_url().ok();
    let typed = Literal::with_datatype("abc".to_string(), xsd_iri("string"));
    assert!(typed.language.is_none());
    assert!(typed.datatype.is_some());
    assert_eq!(typed.value, "abc");

    assert_eq!(
        plain.value, typed.value,
        "Plain and xsd:string-typed literals should have same lexical value"
    );
    assert_ne!(
        plain.datatype, typed.datatype,
        "Plain literal has no datatype; xsd:string-typed has a datatype URL"
    );
    assert_eq!(
        plain.language, typed.language,
        "Neither should have a language tag"
    );

    let plain2 = Literal::new("xyz".to_string());
    assert_ne!(
        plain2.value, typed.value,
        "Different lexical values should not match"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.10 DatatypeDefinitionAxiom Test
// ══════════════════════════════════════════════════════════════════════════════

/// Construct a DatatypeDefinition axiom with a custom datatype.
#[test]
fn test_datatype_definition_in_ontology() {
    use horned_owl::model::Build;

    let id: AxiomId = 42;
    let b = Build::new_string();

    let custom_dt = b.datatype("http://example.org/myInteger");
    let integer_range = horned_owl::model::DataRange::Datatype(
        b.datatype("http://www.w3.org/2001/XMLSchema#integer"),
    );

    let axiom = DatatypeDefinitionAxiom::new(id, custom_dt.into(), integer_range, vec![]);

    assert_eq!(axiom.id, 42);
    assert!(axiom.datatype.as_ref().contains("myInteger"));
    assert!(axiom.annotations.is_empty());

    match &axiom.data_range {
        horned_owl::model::DataRange::Datatype(dt) => {
            assert!(dt.as_ref().contains("integer"));
        }
        _ => panic!("Expected Datatype range"),
    }

    let axiom2 = DatatypeDefinitionAxiom {
        id: 43,
        datatype: b.iri("http://example.org/myString").into(),
        data_range: horned_owl::model::DataRange::Datatype(
            b.datatype("http://www.w3.org/2001/XMLSchema#string"),
        ),
        annotations: vec![],
    };
    assert_eq!(axiom2.id, 43);
    assert!(axiom2.datatype.as_ref().contains("myString"));
}
