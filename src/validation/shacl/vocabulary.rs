//! SHACL vocabulary constants.
//!
//! All IRIs defined by the W3C SHACL specification, plus helper utilities for
//! building SHACL IRIs from local names.

/// The SHACL namespace URI.
pub const SH_NS: &str = "http://www.w3.org/ns/shacl#";

/// Helper: prepend the SHACL namespace to a local name.
#[inline]
pub fn sh(local: &str) -> String {
    format!("{SH_NS}{local}")
}

// ── Core shape and report vocabulary ────────────────────────────────────────

pub const SH_NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";
pub const SH_PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";
pub const SH_SHAPE: &str = "http://www.w3.org/ns/shacl#Shape";

pub const SH_TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";
pub const SH_TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";
pub const SH_TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";
pub const SH_TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";

pub const SH_PATH: &str = "http://www.w3.org/ns/shacl#path";
pub const SH_DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";
pub const SH_SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";
pub const SH_MESSAGE: &str = "http://www.w3.org/ns/shacl#message";
pub const SH_NAME: &str = "http://www.w3.org/ns/shacl#name";
pub const SH_DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";
pub const SH_PROPERTY: &str = "http://www.w3.org/ns/shacl#property";
pub const SH_ORDER: &str = "http://www.w3.org/ns/shacl#order";
pub const SH_GROUP: &str = "http://www.w3.org/ns/shacl#group";
pub const SH_DEFAULT_VALUE: &str = "http://www.w3.org/ns/shacl#defaultValue";

// ── Severity ────────────────────────────────────────────────────────────────

pub const SH_VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";
pub const SH_WARNING: &str = "http://www.w3.org/ns/shacl#Warning";
pub const SH_INFO: &str = "http://www.w3.org/ns/shacl#Info";

// ── Validation report ────────────────────────────────────────────────────────

pub const SH_VALIDATION_REPORT: &str = "http://www.w3.org/ns/shacl#ValidationReport";
pub const SH_VALIDATION_RESULT: &str = "http://www.w3.org/ns/shacl#ValidationResult";
pub const SH_CONFORMS: &str = "http://www.w3.org/ns/shacl#conforms";
pub const SH_RESULT: &str = "http://www.w3.org/ns/shacl#result";
pub const SH_RESULT_SEVERITY: &str = "http://www.w3.org/ns/shacl#resultSeverity";
pub const SH_FOCUS_NODE: &str = "http://www.w3.org/ns/shacl#focusNode";
pub const SH_RESULT_PATH: &str = "http://www.w3.org/ns/shacl#resultPath";
pub const SH_VALUE: &str = "http://www.w3.org/ns/shacl#value";
pub const SH_SOURCE_SHAPE: &str = "http://www.w3.org/ns/shacl#sourceShape";
pub const SH_SOURCE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#sourceConstraintComponent";
pub const SH_RESULT_MESSAGE: &str = "http://www.w3.org/ns/shacl#resultMessage";
pub const SH_DETAIL: &str = "http://www.w3.org/ns/shacl#detail";
pub const SH_SHAPES_GRAPH_WELL_FORMED: &str =
    "http://www.w3.org/ns/shacl#shapesGraphWellFormed";

// ── Value type constraints ───────────────────────────────────────────────────

pub const SH_CLASS: &str = "http://www.w3.org/ns/shacl#class";
pub const SH_DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";
pub const SH_NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";

pub const SH_IRI: &str = "http://www.w3.org/ns/shacl#IRI";
pub const SH_BLANK_NODE: &str = "http://www.w3.org/ns/shacl#BlankNode";
pub const SH_LITERAL: &str = "http://www.w3.org/ns/shacl#Literal";
pub const SH_BLANK_NODE_OR_IRI: &str = "http://www.w3.org/ns/shacl#BlankNodeOrIRI";
pub const SH_BLANK_NODE_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#BlankNodeOrLiteral";
pub const SH_IRI_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#IRIOrLiteral";

// ── Value type constraint components ────────────────────────────────────────

pub const SH_CLASS_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#ClassConstraintComponent";
pub const SH_DATATYPE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";
pub const SH_NODE_KIND_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#NodeKindConstraintComponent";

// ── Cardinality constraints ──────────────────────────────────────────────────

pub const SH_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";
pub const SH_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";

pub const SH_MIN_COUNT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MinCountConstraintComponent";
pub const SH_MAX_COUNT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MaxCountConstraintComponent";

// ── Value range constraints ──────────────────────────────────────────────────

pub const SH_MIN_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#minExclusive";
pub const SH_MIN_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#minInclusive";
pub const SH_MAX_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxExclusive";
pub const SH_MAX_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxInclusive";

pub const SH_MIN_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent";
pub const SH_MIN_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";
pub const SH_MAX_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent";
pub const SH_MAX_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent";

// ── String-based constraints ─────────────────────────────────────────────────

pub const SH_MIN_LENGTH: &str = "http://www.w3.org/ns/shacl#minLength";
pub const SH_MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#maxLength";
pub const SH_PATTERN: &str = "http://www.w3.org/ns/shacl#pattern";
pub const SH_FLAGS: &str = "http://www.w3.org/ns/shacl#flags";
pub const SH_LANGUAGE_IN: &str = "http://www.w3.org/ns/shacl#languageIn";
pub const SH_UNIQUE_LANG: &str = "http://www.w3.org/ns/shacl#uniqueLang";

pub const SH_MIN_LENGTH_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MinLengthConstraintComponent";
pub const SH_MAX_LENGTH_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";
pub const SH_PATTERN_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#PatternConstraintComponent";
pub const SH_LANGUAGE_IN_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#LanguageInConstraintComponent";
pub const SH_UNIQUE_LANG_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#UniqueLangConstraintComponent";

// ── Property pair constraints ────────────────────────────────────────────────

pub const SH_EQUALS: &str = "http://www.w3.org/ns/shacl#equals";
pub const SH_DISJOINT: &str = "http://www.w3.org/ns/shacl#disjoint";
pub const SH_LESS_THAN: &str = "http://www.w3.org/ns/shacl#lessThan";
pub const SH_LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#lessThanOrEquals";

pub const SH_EQUALS_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#EqualsConstraintComponent";
pub const SH_DISJOINT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#DisjointConstraintComponent";
pub const SH_LESS_THAN_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#LessThanConstraintComponent";
pub const SH_LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent";

// ── Logical constraints ──────────────────────────────────────────────────────

pub const SH_NOT: &str = "http://www.w3.org/ns/shacl#not";
pub const SH_AND: &str = "http://www.w3.org/ns/shacl#and";
pub const SH_OR: &str = "http://www.w3.org/ns/shacl#or";
pub const SH_XONE: &str = "http://www.w3.org/ns/shacl#xone";

pub const SH_NOT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#NotConstraintComponent";
pub const SH_AND_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#AndConstraintComponent";
pub const SH_OR_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#OrConstraintComponent";
pub const SH_XONE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#XoneConstraintComponent";

// ── Shape-based constraints ──────────────────────────────────────────────────

pub const SH_NODE: &str = "http://www.w3.org/ns/shacl#node";
pub const SH_PROPERTY_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#property";
pub const SH_QUALIFIED_VALUE_SHAPE: &str = "http://www.w3.org/ns/shacl#qualifiedValueShape";
pub const SH_QUALIFIED_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMinCount";
pub const SH_QUALIFIED_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMaxCount";
pub const SH_QUALIFIED_VALUE_SHAPES_DISJOINT: &str =
    "http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint";

pub const SH_NODE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#NodeConstraintComponent";
pub const SH_PROPERTY_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#PropertyConstraintComponent";
pub const SH_QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent";
pub const SH_QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent";

// ── Other constraints ────────────────────────────────────────────────────────

pub const SH_CLOSED: &str = "http://www.w3.org/ns/shacl#closed";
pub const SH_IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";
pub const SH_HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";
pub const SH_IN: &str = "http://www.w3.org/ns/shacl#in";

pub const SH_CLOSED_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#ClosedConstraintComponent";
pub const SH_HAS_VALUE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#HasValueConstraintComponent";
pub const SH_IN_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#InConstraintComponent";

// ── SPARQL constraints / components ─────────────────────────────────────────

pub const SH_SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";
pub const SH_SELECT: &str = "http://www.w3.org/ns/shacl#select";
pub const SH_ASK: &str = "http://www.w3.org/ns/shacl#ask";
pub const SH_PREFIXES: &str = "http://www.w3.org/ns/shacl#prefixes";
pub const SH_DECLARE: &str = "http://www.w3.org/ns/shacl#declare";
pub const SH_PREFIX: &str = "http://www.w3.org/ns/shacl#prefix";
pub const SH_NAMESPACE: &str = "http://www.w3.org/ns/shacl#namespace";
pub const SH_LABEL_TEMPLATE: &str = "http://www.w3.org/ns/shacl#labelTemplate";

pub const SH_SPARQL_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#SPARQLConstraintComponent";
pub const SH_SPARQL_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#SPARQLConstraint";
pub const SH_SPARQL_ASK_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLAskValidator";
pub const SH_SPARQL_SELECT_VALIDATOR: &str =
    "http://www.w3.org/ns/shacl#SPARQLSelectValidator";
pub const SH_SPARQL_CONSTRAINT_COMPONENT_TYPE: &str =
    "http://www.w3.org/ns/shacl#SPARQLConstraintComponent";

pub const SH_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#ConstraintComponent";
pub const SH_PARAMETER: &str = "http://www.w3.org/ns/shacl#parameter";
pub const SH_VALIDATOR: &str = "http://www.w3.org/ns/shacl#validator";
pub const SH_NODE_VALIDATOR: &str = "http://www.w3.org/ns/shacl#nodeValidator";
pub const SH_PROPERTY_VALIDATOR: &str = "http://www.w3.org/ns/shacl#propertyValidator";
pub const SH_OPTIONAL: &str = "http://www.w3.org/ns/shacl#optional";

// ── Path vocabulary ──────────────────────────────────────────────────────────

pub const SH_INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";
pub const SH_ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";
pub const SH_ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";
pub const SH_ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";
pub const SH_ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";

// ── Entailment ───────────────────────────────────────────────────────────────

pub const SH_ENTAILMENT: &str = "http://www.w3.org/ns/shacl#entailment";

// ── RDF vocabulary used by SHACL ─────────────────────────────────────────────

pub const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
pub const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
pub const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

pub const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
pub const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
pub const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

pub const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";

pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
pub const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
pub const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
pub const XSD_DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
pub const XSD_DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
pub const XSD_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
