//! OWL 2 Vocabulary Constants and Namespaces Registry.
//!
//! Provides static constants for all OWL 2, RDF, RDFS, XSD built-in
//! entities, plus a comprehensive namespace registry with PrefixManager.

use crate::ontology::IRI;
use std::collections::HashMap;

// ── Namespace Strings ────────────────────────────────────────────────────────

pub const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
pub const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
pub const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
pub const DCTERMS_NS: &str = "http://purl.org/dc/terms/";
pub const DC_TYPE_NS: &str = "http://purl.org/dc/dcmitype/";
pub const SKOS_NS: &str = "http://www.w3.org/2004/02/skos/core#";
pub const SKOSXL_NS: &str = "http://www.w3.org/2008/05/skos-xl#";
pub const PROV_NS: &str = "http://www.w3.org/ns/prov#";
pub const TIME_NS: &str = "http://www.w3.org/2006/time#";
pub const SWRL_NS: &str = "http://www.w3.org/2003/11/swrl#";
pub const SWRLB_NS: &str = "http://www.w3.org/2003/11/swrlb#";
pub const FOAF_NS: &str = "http://xmlns.com/foaf/0.1/";
pub const DOAP_NS: &str = "http://usefulinc.com/ns/doap#";
pub const SIOC_NS: &str = "http://rdfs.org/sioc/ns#";
pub const OA_NS: &str = "http://www.w3.org/ns/oa#";
pub const SH_NS: &str = "http://www.w3.org/ns/shacl#";
pub const OBO_NS: &str = "http://purl.obolibrary.org/obo/";
pub const OBO_IN_OWL_NS: &str = "http://www.geneontology.org/formats/oboInOwl#";
pub const GO_NS: &str = "http://purl.obolibrary.org/obo/GO_";
pub const BFO_NS: &str = "http://purl.obolibrary.org/obo/BFO_";
pub const RO_NS: &str = "http://purl.obolibrary.org/obo/RO_";
pub const VANN_NS: &str = "http://purl.org/vocab/vann/";
pub const CC_NS: &str = "http://creativecommons.org/ns#";
pub const GEO_NS: &str = "http://www.w3.org/2003/01/geo/wgs84_pos#";
pub const ORG_NS: &str = "http://www.w3.org/ns/org#";
pub const SCHEMA_NS: &str = "http://schema.org/";
pub const WD_NS: &str = "http://www.wikidata.org/entity/";
pub const WDT_NS: &str = "http://www.wikidata.org/prop/direct/";
pub const VCARD_NS: &str = "http://www.w3.org/2006/vcard/ns#";
pub const DCAT_NS: &str = "http://www.w3.org/ns/dcat#";
pub const QB_NS: &str = "http://purl.org/linked-data/cube#";
pub const SSN_NS: &str = "http://www.w3.org/ns/ssn/";
pub const SOSA_NS: &str = "http://www.w3.org/ns/sosa/";
pub const DCAM_NS: &str = "http://purl.org/dc/dcam/";
pub const VS_NS: &str = "http://www.w3.org/2003/06/sw-vocab-status/ns#";
pub const GR_NS: &str = "http://purl.org/goodrelations/v1#";
pub const DBPEDIA_NS: &str = "http://dbpedia.org/resource/";
pub const DBP_NS: &str = "http://dbpedia.org/property/";
pub const DBO_NS: &str = "http://dbpedia.org/ontology/";
pub const SD_NS: &str = "http://www.w3.org/ns/sparql-service-description#";
pub const CSVW_NS: &str = "http://www.w3.org/ns/csvw#";
pub const VOID_NS: &str = "http://rdfs.org/ns/void#";
pub const PAV_NS: &str = "http://purl.org/pav/";

// ── OWL Vocabulary Constants ─────────────────────────────────────────────────

/// Every OWL 2 built-in entity as a string constant.
pub mod owl {
    // Classes
    pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";
    pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";

    // Object Properties
    pub const TOP_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topObjectProperty";
    pub const BOTTOM_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomObjectProperty";

    // Data Properties
    pub const TOP_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topDataProperty";
    pub const BOTTOM_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomDataProperty";

    // Class Expression Constructors
    pub const INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
    pub const UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
    pub const COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
    pub const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";

    // Restrictions
    pub const ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
    pub const SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    pub const HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
    pub const HAS_SELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";
    pub const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    pub const ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
    pub const ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";
    pub const ON_PROPERTIES: &str = "http://www.w3.org/2002/07/owl#onProperties";

    // Cardinality
    pub const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
    pub const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
    pub const CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
    pub const MIN_QUALIFIED_CARDINALITY: &str =
        "http://www.w3.org/2002/07/owl#minQualifiedCardinality";
    pub const MAX_QUALIFIED_CARDINALITY: &str =
        "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
    pub const QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#qualifiedCardinality";

    // Annotation Properties
    pub const VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
    pub const VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
    pub const BACKWARD_COMPATIBLE: &str = "http://www.w3.org/2002/07/owl#backwardCompatibleWith";
    pub const INCOMPATIBLE: &str = "http://www.w3.org/2002/07/owl#incompatibleWith";
    pub const PRIOR_VERSION: &str = "http://www.w3.org/2002/07/owl#priorVersion";
    pub const DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
    pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";

    // Axioms
    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2002/07/owl#subClassOf";
    pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
    pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    pub const DISJOINT_UNION_OF: &str = "http://www.w3.org/2002/07/owl#disjointUnionOf";
    pub const HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
    pub const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
    pub const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
    pub const ALL_DIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";
    pub const DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
    pub const PROPERTY_CHAIN_AXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";

    // Datatypes
    pub const ON_DATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";
    pub const WITH_RESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
    pub const REAL: &str = "http://www.w3.org/2002/07/owl#real";
    pub const RATIONAL: &str = "http://www.w3.org/2002/07/owl#rational";

    // Entity Types
    pub const CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    pub const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
    pub const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
    pub const NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
    pub const ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
    pub const DATATYPE: &str = "http://www.w3.org/2002/07/owl#Datatype";
    pub const INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#Individual";
    pub const ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
    pub const AXIOM: &str = "http://www.w3.org/2002/07/owl#Axiom";
    pub const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
    pub const DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#DataRange";
    pub const ANNOTATION: &str = "http://www.w3.org/2002/07/owl#Annotation";
    pub const NEGATIVE_PROPERTY_ASSERTION: &str =
        "http://www.w3.org/2002/07/owl#NegativePropertyAssertion";

    // Property Characteristics
    pub const FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
    pub const INVERSE_FUNCTIONAL_PROPERTY: &str =
        "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
    pub const SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
    pub const ASYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
    pub const TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
    pub const REFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";
    pub const IRREFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
    pub const DEPRECATED_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DeprecatedProperty";
    pub const DEPRECATED_CLASS: &str = "http://www.w3.org/2002/07/owl#DeprecatedClass";

    // Annotation Vocabulary
    pub const ANNOTATED_SOURCE: &str = "http://www.w3.org/2002/07/owl#annotatedSource";
    pub const ANNOTATED_PROPERTY: &str = "http://www.w3.org/2002/07/owl#annotatedProperty";
    pub const ANNOTATED_TARGET: &str = "http://www.w3.org/2002/07/owl#annotatedTarget";
    pub const SOURCE_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#sourceIndividual";
    pub const ASSERTION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#assertionProperty";
    pub const TARGET_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#targetIndividual";
    pub const TARGET_VALUE: &str = "http://www.w3.org/2002/07/owl#targetValue";

    // Additional Property Axioms
    pub const INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
    pub const EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
    pub const PROPERTY_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#propertyDisjointWith";
    pub const DOMAIN: &str = "http://www.w3.org/2002/07/owl#domain";
    pub const RANGE: &str = "http://www.w3.org/2002/07/owl#range";
    pub const ALL_DISJOINT_CLASSES: &str = "http://www.w3.org/2002/07/owl#AllDisjointClasses";
    pub const ALL_DISJOINT_PROPERTIES: &str = "http://www.w3.org/2002/07/owl#AllDisjointProperties";
    pub const MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
    pub const INVERSE_OBJECT_PROPERTY_EXPRESSION: &str =
        "http://www.w3.org/2002/07/owl#inverseObjectPropertyExpression";
}

/// RDF/RDFS vocabulary constants.
pub mod rdf {
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    pub const PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
    pub const STATEMENT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement";
    pub const SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";
    pub const PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";
    pub const OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";
    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
    pub const LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
    pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
    pub const HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";
    pub const XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";
    pub const PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";
    pub const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
}

/// RDFS vocabulary constants.
pub mod rdfs {
    pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
    pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
    pub const SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    pub const SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
    pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    pub const SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
    pub const IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";
    pub const LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
    pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
    pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
    pub const DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
}

/// Dublin Core vocabulary constants.
pub mod dc {
    pub const TITLE: &str = "http://purl.org/dc/elements/1.1/title";
    pub const CREATOR: &str = "http://purl.org/dc/elements/1.1/creator";
    pub const DESCRIPTION: &str = "http://purl.org/dc/elements/1.1/description";
    pub const DATE: &str = "http://purl.org/dc/elements/1.1/date";
    pub const FORMAT: &str = "http://purl.org/dc/elements/1.1/format";
    pub const IDENTIFIER: &str = "http://purl.org/dc/elements/1.1/identifier";
    pub const LICENSE: &str = "http://purl.org/dc/elements/1.1/license";
    pub const RIGHTS: &str = "http://purl.org/dc/elements/1.1/rights";
    pub const SUBJECT: &str = "http://purl.org/dc/elements/1.1/subject";
    pub const TYPE: &str = "http://purl.org/dc/elements/1.1/type";
    pub const CONTRIBUTOR: &str = "http://purl.org/dc/elements/1.1/contributor";
    pub const PUBLISHER: &str = "http://purl.org/dc/elements/1.1/publisher";
    pub const SOURCE: &str = "http://purl.org/dc/elements/1.1/source";
}

/// XSD datatype vocabulary constants.
pub mod xsd {
    pub const FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
    pub const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
    pub const LONG: &str = "http://www.w3.org/2001/XMLSchema#long";
    pub const INT: &str = "http://www.w3.org/2001/XMLSchema#int";
    pub const SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";
    pub const BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
    pub const UNSIGNED_LONG: &str = "http://www.w3.org/2001/XMLSchema#unsignedLong";
    pub const UNSIGNED_INT: &str = "http://www.w3.org/2001/XMLSchema#unsignedInt";
    pub const UNSIGNED_SHORT: &str = "http://www.w3.org/2001/XMLSchema#unsignedShort";
    pub const UNSIGNED_BYTE: &str = "http://www.w3.org/2001/XMLSchema#unsignedByte";
    pub const POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";
    pub const NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";
    pub const NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
    pub const NON_POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";
    pub const DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
    pub const DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
    pub const TIME: &str = "http://www.w3.org/2001/XMLSchema#time";
    pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    pub const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
    pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
    pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
}

/// SKOS vocabulary constants.
pub mod skos {
    pub const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
    pub const ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";
    pub const HIDDEN_LABEL: &str = "http://www.w3.org/2004/02/skos/core#hiddenLabel";
    pub const DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";
    pub const NOTE: &str = "http://www.w3.org/2004/02/skos/core#note";
    pub const BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";
    pub const NARROWER: &str = "http://www.w3.org/2004/02/skos/core#narrower";
    pub const RELATED: &str = "http://www.w3.org/2004/02/skos/core#related";
    pub const BROAD_MATCH: &str = "http://www.w3.org/2004/02/skos/core#broadMatch";
    pub const NARROW_MATCH: &str = "http://www.w3.org/2004/02/skos/core#narrowMatch";
    pub const EXACT_MATCH: &str = "http://www.w3.org/2004/02/skos/core#exactMatch";
    pub const CLOSE_MATCH: &str = "http://www.w3.org/2004/02/skos/core#closeMatch";
    pub const RELATED_MATCH: &str = "http://www.w3.org/2004/02/skos/core#relatedMatch";
    pub const BROADER_TRANSITIVE: &str = "http://www.w3.org/2004/02/skos/core#broadTransitive";
    pub const NARROWER_TRANSITIVE: &str = "http://www.w3.org/2004/02/skos/core#narrowTransitive";
    pub const IN_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#inScheme";
    pub const HAS_TOP_CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#hasTopConcept";
    pub const TOP_CONCEPT_OF: &str = "http://www.w3.org/2004/02/skos/core#topConceptOf";
    pub const MEMBER: &str = "http://www.w3.org/2004/02/skos/core#member";
    pub const MEMBER_LIST: &str = "http://www.w3.org/2004/02/skos/core#memberList";
    pub const SEMANTIC_RELATION: &str = "http://www.w3.org/2004/02/skos/core#semanticRelation";
    pub const CHANGE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#changeNote";
    pub const EDITORIAL_NOTE: &str = "http://www.w3.org/2004/02/skos/core#editorialNote";
    pub const HISTORY_NOTE: &str = "http://www.w3.org/2004/02/skos/core#historyNote";
    pub const SCOPE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#scopeNote";
    pub const EXAMPLE: &str = "http://www.w3.org/2004/02/skos/core#example";
    pub const CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#Concept";
    pub const CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";
    pub const COLLECTION: &str = "http://www.w3.org/2004/02/skos/core#Collection";
    pub const ORDERED_COLLECTION: &str = "http://www.w3.org/2004/02/skos/core#OrderedCollection";
}

/// PROV-O vocabulary constants (extended).
pub mod prov {
    pub const ENTITY: &str = "http://www.w3.org/ns/prov#Entity";
    pub const ACTIVITY: &str = "http://www.w3.org/ns/prov#Activity";
    pub const AGENT: &str = "http://www.w3.org/ns/prov#Agent";
    pub const WAS_GENERATED_BY: &str = "http://www.w3.org/ns/prov#wasGeneratedBy";
    pub const WAS_DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
    pub const WAS_ATTRIBUTED_TO: &str = "http://www.w3.org/ns/prov#wasAttributedTo";
    pub const WAS_ASSOCIATED_WITH: &str = "http://www.w3.org/ns/prov#wasAssociatedWith";
    pub const ACTED_ON_BEHALF_OF: &str = "http://www.w3.org/ns/prov#actedOnBehalfOf";
    pub const WAS_INFORMED_BY: &str = "http://www.w3.org/ns/prov#wasInformedBy";
    pub const USED: &str = "http://www.w3.org/ns/prov#used";
    pub const STARTED_AT_TIME: &str = "http://www.w3.org/ns/prov#startedAtTime";
    pub const ENDED_AT_TIME: &str = "http://www.w3.org/ns/prov#endedAtTime";
    pub const WAS_INVALIDATED_BY: &str = "http://www.w3.org/ns/prov#wasInvalidatedBy";
    pub const WAS_QUOTED_FROM: &str = "http://www.w3.org/ns/prov#wasQuotedFrom";
    pub const WAS_REVISION_OF: &str = "http://www.w3.org/ns/prov#wasRevisionOf";
    pub const HAD_PRIMARY_SOURCE: &str = "http://www.w3.org/ns/prov#hadPrimarySource";
    pub const ALTERNATE_OF: &str = "http://www.w3.org/ns/prov#alternateOf";
    pub const SPECIALIZATION_OF: &str = "http://www.w3.org/ns/prov#specializationOf";
    pub const WAS_INFLUENCED_BY: &str = "http://www.w3.org/ns/prov#wasInfluencedBy";
    pub const WAS_ENDED_BY: &str = "http://www.w3.org/ns/prov#wasEndedBy";
    pub const WAS_STARTED_BY: &str = "http://www.w3.org/ns/prov#wasStartedBy";
    pub const AT_LOCATION: &str = "http://www.w3.org/ns/prov#atLocation";
    pub const GENERATED: &str = "http://www.w3.org/ns/prov#generated";
    pub const INVALIDATED: &str = "http://www.w3.org/ns/prov#invalidated";
    pub const HAD_ROLE: &str = "http://www.w3.org/ns/prov#hadRole";
    pub const HAD_PLAN: &str = "http://www.w3.org/ns/prov#hadPlan";
    pub const HAD_ACTIVITY: &str = "http://www.w3.org/ns/prov#hadActivity";
    pub const HAD_MEMBER: &str = "http://www.w3.org/ns/prov#hadMember";
    pub const COLLECTION: &str = "http://www.w3.org/ns/prov#Collection";
    pub const EMPTY_COLLECTION: &str = "http://www.w3.org/ns/prov#EmptyCollection";
    pub const BUNDLE: &str = "http://www.w3.org/ns/prov#Bundle";
    pub const PERSON: &str = "http://www.w3.org/ns/prov#Person";
    pub const ORGANIZATION: &str = "http://www.w3.org/ns/prov#Organization";
    pub const SOFTWARE_AGENT: &str = "http://www.w3.org/ns/prov#SoftwareAgent";
    pub const DELEGATION: &str = "http://www.w3.org/ns/prov#Delegation";
    pub const DERIVATION: &str = "http://www.w3.org/ns/prov#Derivation";
    pub const END: &str = "http://www.w3.org/ns/prov#End";
    pub const GENERATION: &str = "http://www.w3.org/ns/prov#Generation";
    pub const INFLUENCE: &str = "http://www.w3.org/ns/prov#Influence";
    pub const INVALIDATION: &str = "http://www.w3.org/ns/prov#Invalidation";
    pub const START: &str = "http://www.w3.org/ns/prov#Start";
    pub const USAGE: &str = "http://www.w3.org/ns/prov#Usage";
    pub const COMMUNICATION: &str = "http://www.w3.org/ns/prov#Communication";
    pub const PRIMARY_SOURCE: &str = "http://www.w3.org/ns/prov#PrimarySource";
    pub const QUOTATION: &str = "http://www.w3.org/ns/prov#Quotation";
    pub const REVISION: &str = "http://www.w3.org/ns/prov#Revision";
    pub const LOCATION: &str = "http://www.w3.org/ns/prov#Location";
    pub const PLAN: &str = "http://www.w3.org/ns/prov#Plan";
    pub const ROLE: &str = "http://www.w3.org/ns/prov#Role";
    pub const VALUE: &str = "http://www.w3.org/ns/prov#value";
    pub const AT_TIME: &str = "http://www.w3.org/ns/prov#atTime";
}

/// OWL-Time vocabulary constants (extended).
pub mod time {
    pub const TEMPORAL_ENTITY: &str = "http://www.w3.org/2006/time#TemporalEntity";
    pub const INSTANT: &str = "http://www.w3.org/2006/time#Instant";
    pub const INTERVAL: &str = "http://www.w3.org/2006/time#Interval";
    pub const PROPER_INTERVAL: &str = "http://www.w3.org/2006/time#ProperInterval";
    pub const BEFORE: &str = "http://www.w3.org/2006/time#before";
    pub const AFTER: &str = "http://www.w3.org/2006/time#after";
    pub const MEETS: &str = "http://www.w3.org/2006/time#meets";
    pub const MET_BY: &str = "http://www.w3.org/2006/time#metBy";
    pub const OVERLAPS: &str = "http://www.w3.org/2006/time#overlaps";
    pub const OVERLAPPED_BY: &str = "http://www.w3.org/2006/time#overlappedBy";
    pub const STARTS: &str = "http://www.w3.org/2006/time#starts";
    pub const STARTED_BY: &str = "http://www.w3.org/2006/time#startedBy";
    pub const DURING: &str = "http://www.w3.org/2006/time#during";
    pub const CONTAINS: &str = "http://www.w3.org/2006/time#contains";
    pub const FINISHES: &str = "http://www.w3.org/2006/time#finishes";
    pub const FINISHED_BY: &str = "http://www.w3.org/2006/time#finishedBy";
    pub const EQUALS: &str = "http://www.w3.org/2006/time#equals";
    pub const HAS_BEGINNING: &str = "http://www.w3.org/2006/time#hasBeginning";
    pub const HAS_END: &str = "http://www.w3.org/2006/time#hasEnd";
    pub const IN_XSD_DATE_TIME: &str = "http://www.w3.org/2006/time#inXSDDateTime";
    pub const IN_XSD_DATE_TIME_STAMP: &str = "http://www.w3.org/2006/time#inXSDDateTimeStamp";
    pub const IN_XSD_DATE: &str = "http://www.w3.org/2006/time#inXSDDate";
    pub const IN_XSD_G_YEAR: &str = "http://www.w3.org/2006/time#inXSDgYear";
    pub const IN_XSD_G_YEAR_MONTH: &str = "http://www.w3.org/2006/time#inXSDgYearMonth";
    pub const NUMERIC_POSITION: &str = "http://www.w3.org/2006/time#numericPosition";
    pub const NOMINAL_POSITION: &str = "http://www.w3.org/2006/time#nominalPosition";
    pub const HAS_TIME_INSTANT: &str = "http://www.w3.org/2006/time#hasTimeInstant";
    pub const IN_TEMPORAL_POSITION: &str = "http://www.w3.org/2006/time#inTemporalPosition";
    pub const DAY_OF_WEEK: &str = "http://www.w3.org/2006/time#DayOfWeek";
    pub const DISJOINT: &str = "http://www.w3.org/2006/time#intervalDisjoint";
    pub const IN: &str = "http://www.w3.org/2006/time#intervalIn";
    pub const HAS_DURATION: &str = "http://www.w3.org/2006/time#hasDuration";
    pub const HAS_TEMPORAL_DURATION: &str = "http://www.w3.org/2006/time#hasTemporalDuration";
    pub const HAS_DURATION_DESCRIPTION: &str = "http://www.w3.org/2006/time#hasDurationDescription";
    pub const HAS_DATE_TIME_DESCRIPTION: &str =
        "http://www.w3.org/2006/time#hasDateTimeDescription";
    pub const UNIT_TYPE: &str = "http://www.w3.org/2006/time#unitType";
    pub const HAS_TRS: &str = "http://www.w3.org/2006/time#hasTRS";
    pub const YEAR: &str = "http://www.w3.org/2006/time#year";
    pub const MONTH: &str = "http://www.w3.org/2006/time#month";
    pub const DAY: &str = "http://www.w3.org/2006/time#day";
    pub const HOUR: &str = "http://www.w3.org/2006/time#hour";
    pub const MINUTE: &str = "http://www.w3.org/2006/time#minute";
    pub const SECOND: &str = "http://www.w3.org/2006/time#second";
    pub const DURATION: &str = "http://www.w3.org/2006/time#Duration";
    pub const DURATION_DESCRIPTION: &str = "http://www.w3.org/2006/time#DurationDescription";
    pub const TEMPORAL_POSITION: &str = "http://www.w3.org/2006/time#TemporalPosition";
    pub const TEMPORAL_DURATION: &str = "http://www.w3.org/2006/time#TemporalDuration";
    pub const TRS: &str = "http://www.w3.org/2006/time#TRS";
    pub const DATE_TIME_INTERVAL: &str = "http://www.w3.org/2006/time#DateTimeInterval";
    pub const DATE_TIME_DESCRIPTION: &str = "http://www.w3.org/2006/time#DateTimeDescription";
    pub const MONTH_OF_YEAR: &str = "http://www.w3.org/2006/time#MonthOfYear";
    pub const TIME_ZONE: &str = "http://www.w3.org/2006/time#TimeZone";
    pub const TIME_POSITION: &str = "http://www.w3.org/2006/time#TimePosition";
}

// ── Namespaces Registry ──────────────────────────────────────────────────────

/// Registry of well-known namespace prefixes (50+ prefixes).
pub struct Namespaces;

impl Namespaces {
    /// Get all well-known namespace (prefix, IRI) pairs.
    #[must_use]
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("owl", OWL_NS),
            ("rdf", RDF_NS),
            ("rdfs", RDFS_NS),
            ("xsd", XSD_NS),
            ("xml", XML_NS),
            ("dc", DC_NS),
            ("dc11", DC_NS),
            ("dcterms", DCTERMS_NS),
            ("dctype", DC_TYPE_NS),
            ("skos", SKOS_NS),
            ("skosxl", SKOSXL_NS),
            ("prov", PROV_NS),
            ("time", TIME_NS),
            ("swrl", SWRL_NS),
            ("swrlb", SWRLB_NS),
            ("foaf", FOAF_NS),
            ("doap", DOAP_NS),
            ("sioc", SIOC_NS),
            ("oa", OA_NS),
            ("sh", SH_NS),
            ("obo", OBO_NS),
            ("oboInOwl", OBO_IN_OWL_NS),
            ("go", GO_NS),
            ("bfo", BFO_NS),
            ("ro", RO_NS),
            ("vann", VANN_NS),
            ("cc", CC_NS),
            ("geo", GEO_NS),
            ("org", ORG_NS),
            ("schema", SCHEMA_NS),
            ("wd", WD_NS),
            ("wdt", WDT_NS),
            ("vcard", VCARD_NS),
            ("dcat", DCAT_NS),
            ("qb", QB_NS),
            ("ssn", SSN_NS),
            ("sosa", SOSA_NS),
            ("dct", DCTERMS_NS),
            ("dcam", DCAM_NS),
            ("vs", VS_NS),
            ("gr", GR_NS),
            ("dbpedia", DBPEDIA_NS),
            ("dbp", DBP_NS),
            ("dbo", DBO_NS),
            ("sd", SD_NS),
            ("csvw", CSVW_NS),
            ("void", VOID_NS),
            ("pav", PAV_NS),
        ]
    }

    /// Get the IRI for a well-known prefix, if known.
    #[must_use]
    pub fn get_iri(prefix: &str) -> Option<&'static str> {
        Self::all()
            .iter()
            .find(|(p, _)| *p == prefix)
            .map(|(_, iri)| *iri)
    }

    /// Get the prefix for a well-known IRI, if known.
    #[must_use]
    pub fn get_prefix(iri: &str) -> Option<&'static str> {
        Self::all().iter().find(|(_, i)| *i == iri).map(|(p, _)| *p)
    }
}

// ── PrefixManager ────────────────────────────────────────────────────────────

/// Manages prefix ↔ IRI mappings for an ontology.
pub struct PrefixManager {
    known: HashMap<String, String>,
    custom: HashMap<String, String>,
    reverse: HashMap<String, String>,
}

impl Default for PrefixManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PrefixManager {
    /// Create with all well-known namespaces pre-loaded.
    #[must_use]
    pub fn new() -> Self {
        let mut known = HashMap::new();
        for (prefix, iri) in Namespaces::all() {
            known.insert(prefix.to_string(), iri.to_string());
        }
        Self {
            known,
            custom: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Add a custom prefix → IRI mapping.
    pub fn add_prefix(&mut self, prefix: &str, iri: &str) {
        self.custom.insert(prefix.to_string(), iri.to_string());
        self.reverse.insert(iri.to_string(), prefix.to_string());
    }

    /// Remove a prefix.
    pub fn remove_prefix(&mut self, prefix: &str) {
        if let Some(iri) = self.custom.remove(prefix) {
            self.reverse.remove(&iri);
        }
    }

    /// Expand a prefixed name: "owl:Thing" → "http://www.w3.org/2002/07/owl#Thing"
    #[must_use]
    pub fn expand(&self, prefixed: &str) -> Option<String> {
        let (pref, local) = prefixed.split_once(':')?;
        let ns = self.custom.get(pref).or_else(|| self.known.get(pref))?;
        Some(format!("{ns}{local}"))
    }

    /// Shorten an IRI: "http://www.w3.org/2002/07/owl#Thing" → "owl:Thing"
    #[must_use]
    pub fn shorten(&self, iri: &str) -> Option<String> {
        // Try custom prefixes first, then known
        for (prefix, ns) in self.custom.iter().chain(self.known.iter()) {
            if let Some(local) = iri.strip_prefix(ns.as_str())
                && !local.is_empty()
            {
                return Some(format!("{prefix}:{local}"));
            }
        }
        None
    }

    /// Get all prefix declarations.
    #[must_use]
    pub fn get_prefixes(&self) -> HashMap<String, String> {
        let mut map = self.known.clone();
        map.extend(self.custom.clone());
        map
    }
}

// ── OWL 2 Built-in IRI Helpers ───────────────────────────────────────────────

// Keep existing constants for backward compatibility
pub const OWL_THING_STR: &str = owl::THING;
pub const OWL_NOTHING_STR: &str = owl::NOTHING;

impl IRI {
    #[must_use]
    #[inline]
    pub fn owl_thing() -> Self {
        IRI::new(owl::THING)
    }

    #[must_use]
    #[inline]
    pub fn owl_nothing() -> Self {
        IRI::new(owl::NOTHING)
    }

    #[must_use]
    #[inline]
    pub fn is_owl_thing(&self) -> bool {
        self.as_str() == owl::THING
    }

    #[must_use]
    #[inline]
    pub fn is_owl_nothing(&self) -> bool {
        self.as_str() == owl::NOTHING
    }

    /// Check if this IRI has an absolute scheme (contains `://`)
    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.as_str().contains("://")
    }

    /// Extract the scheme component (e.g., "http", "urn", "file")
    #[must_use]
    pub fn get_scheme(&self) -> Option<&str> {
        self.as_str().split("://").next()
    }

    /// Get the namespace (up to and including the last # or /)
    #[must_use]
    pub fn get_namespace(&self) -> IRI {
        let s = self.as_str();
        if let Some(pos) = s.rfind('#') {
            IRI::new(&s[..=pos])
        } else if let Some(pos) = s.rfind('/') {
            IRI::new(&s[..=pos])
        } else {
            self.clone()
        }
    }

    /// Get the fragment portion after # (local name)
    #[must_use]
    pub fn get_fragment(&self) -> Option<&str> {
        self.as_str()
            .rfind('#')
            .map(|pos| &self.as_str()[pos + 1..])
    }

    /// Resolve a relative IRI against this base IRI
    #[must_use]
    pub fn resolve(&self, relative: &str) -> IRI {
        if relative.contains("://") {
            return IRI::new(relative);
        }
        let base = self.as_str();
        let idx = base.rfind('#').unwrap_or(base.len());
        let base_without_frag = &base[..idx];
        IRI::new(&format!("{base_without_frag}{relative}"))
    }

    /// Check if this IRI uses OWL/RDF/XSD reserved vocabulary namespace
    #[must_use]
    pub fn is_reserved_vocabulary(&self) -> bool {
        let s = self.as_str();
        s.starts_with("http://www.w3.org/2002/07/owl#")
            || s.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
            || s.starts_with("http://www.w3.org/2000/01/rdf-schema#")
            || s.starts_with("http://www.w3.org/2001/XMLSchema#")
    }

    /// Check if this IRI is a built-in annotation property
    #[must_use]
    pub fn is_builtin_annotation_property(&self) -> bool {
        let s = self.as_str();
        s == owl::VERSION_IRI
            || s == owl::VERSION_INFO
            || s == owl::DEPRECATED
            || s == owl::BACKWARD_COMPATIBLE
            || s == owl::INCOMPATIBLE
            || s == owl::PRIOR_VERSION
            || s == rdfs::LABEL
            || s == rdfs::COMMENT
            || s == rdfs::SEE_ALSO
            || s == rdfs::IS_DEFINED_BY
    }

    /// Return the angle-bracket-quoted IRI form
    #[must_use]
    pub fn to_quoted_string(&self) -> String {
        format!("<{}>", self.as_str())
    }

    /// Return the N-Triples IRI form (angle-bracket-quoted)
    #[must_use]
    pub fn to_ntriples_string(&self) -> String {
        self.to_quoted_string()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owl_thing() {
        let thing = IRI::owl_thing();
        assert!(thing.is_owl_thing());
        assert!(!thing.is_owl_nothing());
    }

    #[test]
    fn test_owl_nothing() {
        let nothing = IRI::owl_nothing();
        assert!(nothing.is_owl_nothing());
        assert!(!nothing.is_owl_thing());
    }

    #[test]
    fn test_vocabulary_equality() {
        let thing1 = IRI::owl_thing();
        let thing2 = IRI::new(OWL_THING_STR);
        assert_eq!(thing1, thing2);
    }

    #[test]
    fn test_namespaces_all() {
        let all = Namespaces::all();
        assert!(all.len() >= 48);
        assert!(all.iter().any(|(p, _)| *p == "owl"));
        assert!(all.iter().any(|(p, _)| *p == "rdf"));
        assert!(all.iter().any(|(p, _)| *p == "rdfs"));
        assert!(all.iter().any(|(p, _)| *p == "xsd"));
    }

    #[test]
    fn test_namespaces_get_iri() {
        assert_eq!(Namespaces::get_iri("owl"), Some(OWL_NS));
        assert_eq!(Namespaces::get_iri("skos"), Some(SKOS_NS));
        assert_eq!(Namespaces::get_iri("nonexistent"), None);
    }

    #[test]
    fn test_prefix_manager_expand() {
        let pm = PrefixManager::new();
        assert_eq!(
            pm.expand("owl:Thing"),
            Some("http://www.w3.org/2002/07/owl#Thing".to_string())
        );
        assert_eq!(pm.expand("unknown:Thing"), None);
    }

    #[test]
    fn test_prefix_manager_shorten() {
        let pm = PrefixManager::new();
        let shortened = pm.shorten("http://www.w3.org/2002/07/owl#Thing");
        assert_eq!(shortened, Some("owl:Thing".to_string()));
    }

    #[test]
    fn test_owl_constants() {
        assert_eq!(owl::THING, "http://www.w3.org/2002/07/owl#Thing");
        assert_eq!(
            owl::SUB_CLASS_OF,
            "http://www.w3.org/2002/07/owl#subClassOf"
        );
        assert_eq!(owl::IMPORTS, "http://www.w3.org/2002/07/owl#imports");
    }

    #[test]
    fn test_rdf_constants() {
        assert_eq!(rdf::TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        assert_eq!(
            rdf::LANG_STRING,
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
        );
    }

    #[test]
    fn test_dc_constants() {
        assert_eq!(dc::TITLE, "http://purl.org/dc/elements/1.1/title");
    }

    #[test]
    fn test_skos_constants() {
        assert_eq!(
            skos::PREF_LABEL,
            "http://www.w3.org/2004/02/skos/core#prefLabel"
        );
    }

    #[test]
    fn test_prefix_manager_custom() {
        let mut pm = PrefixManager::new();
        pm.add_prefix("ex", "http://example.org/");
        assert_eq!(
            pm.expand("ex:Test"),
            Some("http://example.org/Test".to_string())
        );
        assert_eq!(
            pm.shorten("http://example.org/Test"),
            Some("ex:Test".to_string())
        );
    }
}
