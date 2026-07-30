#[path = "helpers/mod.rs"]
mod helpers;

use helpers::*;
use oxidowl::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// OWL2Datatype Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn datatype_iri_mapping() {
    assert_eq!(
        OWL2Datatype::String.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#string"
    );
    assert_eq!(
        OWL2Datatype::Integer.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#integer"
    );
    assert_eq!(
        OWL2Datatype::Real.iri().as_str(),
        "http://www.w3.org/2002/07/owl#real"
    );
    assert_eq!(
        OWL2Datatype::Literal.iri().as_str(),
        "http://www.w3.org/2000/01/rdf-schema#Literal"
    );
}

#[test]
fn datatype_short_names() {
    assert_eq!(OWL2Datatype::String.short_name(), "string");
    assert_eq!(OWL2Datatype::Integer.short_name(), "integer");
    assert_eq!(OWL2Datatype::Boolean.short_name(), "boolean");
    assert_eq!(OWL2Datatype::DateTime.short_name(), "dateTime");
}

#[test]
fn datatype_category_numeric() {
    assert!(OWL2Datatype::Integer.is_numeric());
    assert!(OWL2Datatype::Decimal.is_numeric());
    assert!(OWL2Datatype::Float.is_numeric());
    assert!(OWL2Datatype::Double.is_numeric());
    assert!(!OWL2Datatype::String.is_numeric());
    assert!(!OWL2Datatype::Boolean.is_numeric());
}

#[test]
fn datatype_category_datetime() {
    assert!(OWL2Datatype::DateTime.is_datetime());
    assert!(OWL2Datatype::Date.is_datetime());
    assert!(OWL2Datatype::Time.is_datetime());
    assert!(!OWL2Datatype::Integer.is_datetime());
}

#[test]
fn datatype_ordered() {
    assert!(OWL2Datatype::Integer.is_ordered());
    assert!(OWL2Datatype::String.is_ordered());
    assert!(!OWL2Datatype::Boolean.is_ordered());
}

#[test]
fn datatype_subtype_hierarchy() {
    assert!(OWL2Datatype::Int.is_subtype_of(&OWL2Datatype::Integer));
    assert!(OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Decimal));
    assert!(OWL2Datatype::Decimal.is_subtype_of(&OWL2Datatype::Real));
    assert!(!OWL2Datatype::String.is_subtype_of(&OWL2Datatype::Integer));
    assert!(!OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::String));
    assert!(OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Integer));
}

#[test]
fn datatype_parent() {
    assert_eq!(
        OWL2Datatype::Integer.parent_datatype(),
        Some(OWL2Datatype::Decimal)
    );
    assert_eq!(
        OWL2Datatype::Long.parent_datatype(),
        Some(OWL2Datatype::Integer)
    );
    assert_eq!(OWL2Datatype::String.parent_datatype(), None);
}

#[test]
fn datatype_built_in() {
    assert!(OWL2Datatype::Integer.is_built_in());
    assert!(OWL2Datatype::String.is_built_in());
    assert!(OWL2Datatype::Real.is_built_in());
}

#[test]
fn datatype_validation() {
    assert!(OWL2Datatype::Integer.validate_lexical_form("42").is_ok());
    assert!(OWL2Datatype::Integer.validate_lexical_form("-17").is_ok());
    assert!(OWL2Datatype::Integer.validate_lexical_form("0").is_ok());
    assert!(OWL2Datatype::Integer.validate_lexical_form("not-a-number").is_err());
    assert!(OWL2Datatype::Boolean.validate_lexical_form("true").is_ok());
    assert!(OWL2Datatype::Boolean.validate_lexical_form("false").is_ok());
    assert!(OWL2Datatype::Boolean.validate_lexical_form("maybe").is_err());
}

#[test]
fn datatype_from_iri() {
    let string_iri = IRI::new("http://www.w3.org/2001/XMLSchema#string");
    assert_eq!(OWL2Datatype::from_iri(&string_iri), Some(OWL2Datatype::String));

    let integer_iri = IRI::new("http://www.w3.org/2001/XMLSchema#integer");
    assert_eq!(OWL2Datatype::from_iri(&integer_iri), Some(OWL2Datatype::Integer));

    let unknown_iri = IRI::new("http://example.org/MyDatatype");
    assert_eq!(OWL2Datatype::from_iri(&unknown_iri), None);
}

#[test]
fn datatype_facets_string() {
    let facets = OWL2Datatype::String.facets();
    let names: Vec<_> = facets.iter().map(|f| f.short_name()).collect();
    assert!(names.contains(&"length"));
    assert!(names.contains(&"minLength"));
    assert!(names.contains(&"maxLength"));
    assert!(names.contains(&"pattern"));
}

#[test]
fn datatype_facets_numeric() {
    let facets = OWL2Datatype::Integer.facets();
    let names: Vec<_> = facets.iter().map(|f| f.short_name()).collect();
    assert!(names.contains(&"minInclusive"));
    assert!(names.contains(&"maxInclusive"));
    assert!(names.contains(&"minExclusive"));
    assert!(names.contains(&"maxExclusive"));
    assert!(names.contains(&"totalDigits"));
}

#[test]
fn datatype_parse_literal_integer() {
    let lit = OWL2Datatype::Integer.parse_literal("42");
    assert_eq!(lit.value, "42");
    assert_eq!(lit.datatype.as_ref().unwrap().as_str(), "http://www.w3.org/2001/XMLSchema#integer");
}

#[test]
fn datatype_parse_literal_string() {
    let lit = OWL2Datatype::String.parse_literal("hello");
    assert_eq!(lit.value, "hello");
}

// ══════════════════════════════════════════════════════════════════════════════
// DatatypeManager Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn datatype_manager_recognized() {
    let manager = DatatypeManager::new();
    assert!(manager.is_recognized_datatype(&OWL2Datatype::String.iri()));
    assert!(manager.is_recognized_datatype(&OWL2Datatype::Integer.iri()));
    assert!(!manager.is_recognized_datatype(&IRI::new("http://example.org/Unknown")));
}

#[test]
fn datatype_manager_subtype() {
    let manager = DatatypeManager::new();
    assert!(manager.is_subtype_of(&OWL2Datatype::Int, &OWL2Datatype::Integer));
    assert!(manager.is_subtype_of(&OWL2Datatype::Integer, &OWL2Datatype::Decimal));
    assert!(!manager.is_subtype_of(&OWL2Datatype::String, &OWL2Datatype::Integer));
}

#[test]
fn datatype_manager_subtypes() {
    let manager = DatatypeManager::new();
    let subtypes = manager.get_subtypes(&OWL2Datatype::Integer);
    // Should contain some numeric subtypes — exact set depends on implementation
    assert!(!subtypes.is_empty(), "Integer should have subtypes");
}

// ══════════════════════════════════════════════════════════════════════════════
// OWLFacet Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn facet_iris() {
    assert_eq!(
        OWLFacet::XsdLength.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#length"
    );
    assert_eq!(
        OWLFacet::XsdPattern.iri().as_str(),
        "http://www.w3.org/2001/XMLSchema#pattern"
    );
}

#[test]
fn facet_short_names() {
    assert_eq!(OWLFacet::XsdLength.short_name(), "length");
    assert_eq!(OWLFacet::XsdMinInclusive.short_name(), "minInclusive");
    assert_eq!(OWLFacet::RdfLangRange.short_name(), "langRange");
}

// ══════════════════════════════════════════════════════════════════════════════
// ConstrainingFacet Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn constraining_facet_applicable_length_to_string() {
    assert!(ConstrainingFacet::Length.is_applicable_to(&OWL2Datatype::String));
    assert!(!ConstrainingFacet::Length.is_applicable_to(&OWL2Datatype::Integer));
}

#[test]
fn constraining_facet_applicable_min_max_to_numeric() {
    assert!(ConstrainingFacet::MinInclusive.is_applicable_to(&OWL2Datatype::Integer));
    assert!(ConstrainingFacet::MaxInclusive.is_applicable_to(&OWL2Datatype::Integer));
    assert!(ConstrainingFacet::MinInclusive.is_applicable_to(&OWL2Datatype::Decimal));
    assert!(ConstrainingFacet::TotalDigits.is_applicable_to(&OWL2Datatype::Integer));
}

// ══════════════════════════════════════════════════════════════════════════════
// Literal Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn literal_plain() {
    let lit = Literal::new("hello".to_string());
    assert_eq!(lit.value, "hello");
    assert!(lit.language.is_none());
    assert!(lit.datatype.is_none());
}

#[test]
fn literal_with_language() {
    let lit = Literal::with_language("bonjour".to_string(), "fr".to_string());
    assert_eq!(lit.value, "bonjour");
    assert_eq!(lit.language.as_deref(), Some("fr"));
}

#[test]
fn literal_with_datatype() {
    let lit = Literal::with_datatype(
        "42".to_string(),
        IRI::new("http://www.w3.org/2001/XMLSchema#integer"),
    );
    assert_eq!(lit.value, "42");
    assert_eq!(
        lit.datatype.as_ref().unwrap().as_str(),
        "http://www.w3.org/2001/XMLSchema#integer"
    );
}
