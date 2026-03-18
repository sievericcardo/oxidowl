//! OWLlink Protocol Implementation
//!
//! This module implements the OWLlink protocol for reasoner communication.
//! OWLlink is a standard protocol for accessing OWL reasoners.

use crate::{Error, Result, ontology::ClassExpression, reasoning::ReasoningService};
use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use warp::{Filter, Reply};

/// OWLlink server implementation
#[derive(Debug)]
pub struct OWLlinkServer {
    /// Server port
    port: u16,
    /// Bind address
    bind_address: String,
    /// Reasoning service
    reasoning_service: Arc<ReasoningService>,
    /// Active knowledge bases
    knowledge_bases: Arc<tokio::sync::RwLock<HashMap<String, KnowledgeBase>>>,
}

impl OWLlinkServer {
    /// Create a new OWLlink server
    pub fn new(port: u16, bind_address: String, reasoning_service: Arc<ReasoningService>) -> Self {
        Self {
            port,
            bind_address,
            reasoning_service,
            knowledge_bases: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Start the OWLlink server
    pub async fn start(self) -> Result<OWLlinkServerHandle> {
        let reasoning_service = self.reasoning_service.clone();
        let knowledge_bases = self.knowledge_bases.clone();

        // OWLlink request handler
        let owllink_handler = warp::path("owllink")
            .and(warp::post())
            .and(warp::body::bytes())
            .and(warp::any().map(move || reasoning_service.clone()))
            .and(warp::any().map(move || knowledge_bases.clone()))
            .and_then(handle_owllink_request);

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::json(&serde_json::json!({"status": "ok", "protocol": "OWLlink"})));

        let routes = owllink_handler.or(health).with(
            warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["content-type"])
                .allow_methods(vec!["POST", "GET"]),
        );

        let addr: SocketAddr = format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|e| Error::config(format!("Invalid server address: {}", e)))?;

        let server_task = tokio::spawn(warp::serve(routes).run(addr));

        tracing::info!(
            "OWLlink server started on {}:{}",
            self.bind_address,
            self.port
        );

        Ok(OWLlinkServerHandle { task: server_task })
    }
}

/// Handle for a running OWLlink server
#[derive(Debug)]
pub struct OWLlinkServerHandle {
    task: tokio::task::JoinHandle<()>,
}

impl OWLlinkServerHandle {
    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        self.task.abort();
        Ok(())
    }
}

/// Knowledge base representation
#[derive(Debug, Clone)]
pub struct KnowledgeBase {
    /// Knowledge base ID
    pub id: String,
    /// Name
    pub name: Option<String>,
    /// Created timestamp
    pub created: std::time::SystemTime,
    /// Last modified timestamp
    pub modified: std::time::SystemTime,
}

/// OWLlink request message
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OWLlinkRequest {
    /// Request ID
    pub request_id: Option<String>,
    /// Knowledge base ID
    pub knowledge_base: Option<String>,
    /// Request type
    #[serde(flatten)]
    pub request_type: OWLlinkRequestType,
}

/// Types of OWLlink requests
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum OWLlinkRequestType {
    /// Get description of reasoner
    GetDescription,
    /// Create knowledge base
    CreateKB { name: Option<String> },
    /// Release knowledge base
    ReleaseKB,
    /// Load ontology
    LoadOntology { ontology_iri: String },
    /// Check consistency
    IsConsistent,
    /// Check satisfiability
    IsSatisfiable { class_expression: String },
    /// Check subsumption
    IsSubsumedBy {
        sub_class: String,
        super_class: String,
    },
    /// Check if two classes are equivalent
    AreClassesEquivalent {
        class_a: String,
        class_b: String,
    },
    /// Check if two classes are disjoint
    AreClassesDisjoint {
        class_a: String,
        class_b: String,
    },
    /// Check if an axiom is entailed
    IsEntailed { axiom: String },
    /// Classify ontology
    Classify,
    /// Get subclasses
    GetSubClasses {
        class_expression: String,
        direct: Option<bool>,
    },
    /// Get superclasses
    GetSuperClasses {
        class_expression: String,
        direct: Option<bool>,
    },
    /// Get equivalent classes
    GetEquivalentClasses { class_expression: String },
    /// Get instances of a class
    GetInstances {
        class_expression: String,
        direct: Option<bool>,
    },
    /// Get types of an individual
    GetTypes {
        individual: String,
        direct: Option<bool>,
    },
    /// Get flattened types of an individual  
    GetFlattenedTypes { individual: String },
    /// Get all individuals same as a given individual
    GetSameIndividuals { individual: String },
    /// Get all individuals different from a given individual
    GetDifferentIndividuals { individual: String },
    /// Check if two individuals are related via an object property
    AreIndividualsRelated {
        individual_a: String,
        individual_b: String,
        role: String,
    },
    /// Get sub-object properties
    GetSubObjectProperties {
        object_property: String,
        direct: Option<bool>,
    },
    /// Get super-object properties
    GetSuperObjectProperties {
        object_property: String,
        direct: Option<bool>,
    },
    /// Get equivalent object properties
    GetEquivalentObjectProperties { object_property: String },
    /// Get sub-data properties
    GetSubDataProperties {
        data_property: String,
        direct: Option<bool>,
    },
    /// Get super-data properties
    GetSuperDataProperties {
        data_property: String,
        direct: Option<bool>,
    },
    /// Get equivalent data properties
    GetEquivalentDataProperties { data_property: String },
}

/// OWLlink response message
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OWLlinkResponse {
    /// Request ID (if provided)
    pub request_id: Option<String>,
    /// Knowledge base ID
    pub knowledge_base: Option<String>,
    /// Response content
    #[serde(flatten)]
    pub response: OWLlinkResponseType,
}

/// Types of OWLlink responses
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OWLlinkResponseType {
    /// Reasoner description
    Description {
        name: String,
        version: String,
        public_kb: Vec<String>,
        supported_datatypes: Vec<String>,
    },
    /// Knowledge base created
    KB { kb: String, name: Option<String> },
    /// Success response
    OK,
    /// Boolean result
    BooleanResponse { result: bool },
    /// Class hierarchy
    ClassHierarchy { classes: Vec<ClassNode> },
    /// Class set
    Classes { classes: Vec<String> },
    /// Individuals set
    Individuals { individuals: Vec<String> },
    /// Property set
    Properties { properties: Vec<String> },
    /// Error response
    Error { error: String, message: String },
}

/// Class node in hierarchy
#[derive(Debug, Serialize)]
pub struct ClassNode {
    /// Class IRI
    pub iri: String,
    /// Direct subclasses
    pub subclasses: Vec<ClassNode>,
}

/// Handle OWLlink requests
async fn handle_owllink_request(
    body: bytes::Bytes,
    reasoning_service: Arc<ReasoningService>,
    knowledge_bases: Arc<tokio::sync::RwLock<HashMap<String, KnowledgeBase>>>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let request_xml = String::from_utf8(body.to_vec())
        .map_err(|e| warp::reject::custom(OWLlinkError(format!("Invalid UTF-8: {}", e))))?;

    tracing::debug!("Received OWLlink request: {}", request_xml);

    // Parse OWLlink request
    let request: OWLlinkRequest = from_str(&request_xml)
        .map_err(|e| warp::reject::custom(OWLlinkError(format!("XML parse error: {}", e))))?;

    // Process request
    let response = process_owllink_request(request, reasoning_service, knowledge_bases)
        .await
        .map_err(|e| warp::reject::custom(OWLlinkError(e.to_string())))?;

    // Serialize response to XML
    let response_xml = to_string(&response).map_err(|e| {
        warp::reject::custom(OWLlinkError(format!("XML serialization error: {}", e)))
    })?;

    tracing::debug!("Sending OWLlink response: {}", response_xml);

    Ok(warp::reply::with_header(
        response_xml,
        "content-type",
        "application/xml",
    ))
}

/// Process an OWLlink request
async fn process_owllink_request(
    request: OWLlinkRequest,
    reasoning_service: Arc<ReasoningService>,
    knowledge_bases: Arc<tokio::sync::RwLock<HashMap<String, KnowledgeBase>>>,
) -> Result<OWLlinkResponse> {
    let response_type = match request.request_type {
        OWLlinkRequestType::GetDescription => OWLlinkResponseType::Description {
            name: "Oxidowl".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            public_kb: vec![],
            supported_datatypes: vec![
                "http://www.w3.org/2001/XMLSchema#string".to_string(),
                "http://www.w3.org/2001/XMLSchema#integer".to_string(),
                "http://www.w3.org/2001/XMLSchema#boolean".to_string(),
            ],
        },

        OWLlinkRequestType::CreateKB { name } => {
            let kb_id = uuid::Uuid::new_v4().to_string();
            let kb = KnowledgeBase {
                id: kb_id.clone(),
                name: name.clone(),
                created: std::time::SystemTime::now(),
                modified: std::time::SystemTime::now(),
            };

            let mut kbs = knowledge_bases.write().await;
            kbs.insert(kb_id.clone(), kb);

            OWLlinkResponseType::KB { kb: kb_id, name }
        }

        OWLlinkRequestType::ReleaseKB => {
            if let Some(kb_id) = &request.knowledge_base {
                let mut kbs = knowledge_bases.write().await;
                kbs.remove(kb_id);
            }
            OWLlinkResponseType::OK
        }

        OWLlinkRequestType::LoadOntology { ontology_iri: _ } => {
            // For now, just acknowledge - in practice would load the ontology
            OWLlinkResponseType::OK
        }

        OWLlinkRequestType::IsConsistent => {
            let is_consistent = reasoning_service
                .is_consistent()
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            OWLlinkResponseType::BooleanResponse {
                result: is_consistent,
            }
        }

        OWLlinkRequestType::IsSatisfiable { class_expression } => {
            // Parse class expression and check satisfiability
            let class_expr = parse_class_expression(&class_expression)?;
            let is_satisfiable = reasoning_service
                .is_satisfiable(&class_expr)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            OWLlinkResponseType::BooleanResponse {
                result: is_satisfiable,
            }
        }

        OWLlinkRequestType::IsSubsumedBy {
            sub_class,
            super_class,
        } => {
            let sub_expr = parse_class_expression(&sub_class)?;
            let super_expr = parse_class_expression(&super_class)?;

            let is_subsumed = reasoning_service
                .is_subsumed_by(&sub_expr, &super_expr)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            OWLlinkResponseType::BooleanResponse {
                result: is_subsumed,
            }
        }

        OWLlinkRequestType::Classify => {
            let _classification = reasoning_service
                .classify()
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            // For now, return empty hierarchy - would build from classification results
            OWLlinkResponseType::ClassHierarchy { classes: vec![] }
        }

        OWLlinkRequestType::GetSubClasses {
            class_expression,
            direct,
        } => {
            let class_expr = parse_class_expression(&class_expression)?;
            let subclasses = reasoning_service
                .get_subclasses(&class_expr, direct.unwrap_or(false))
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            let class_iris: Vec<String> =
                subclasses.into_iter().map(|c| format!("{:?}", c)).collect();

            OWLlinkResponseType::Classes {
                classes: class_iris,
            }
        }

        OWLlinkRequestType::GetSuperClasses {
            class_expression,
            direct,
        } => {
            let class_expr = parse_class_expression(&class_expression)?;
            let superclasses = reasoning_service
                .get_superclasses(&class_expr, direct.unwrap_or(false))
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            let class_iris: Vec<String> = superclasses
                .into_iter()
                .map(|c| format!("{:?}", c))
                .collect();

            OWLlinkResponseType::Classes {
                classes: class_iris,
            }
        }

        OWLlinkRequestType::GetInstances {
            class_expression,
            direct,
        } => {
            let class_expr = parse_class_expression(&class_expression)?;
            let instances = reasoning_service
                .get_instances(&class_expr, direct.unwrap_or(false))
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;

            let individual_iris: Vec<String> = instances
                .into_iter()
                .filter_map(|i| i.iri().map(|iri| iri.to_string()))
                .collect();

            OWLlinkResponseType::Individuals {
                individuals: individual_iris,
            }
        }

        OWLlinkRequestType::AreClassesEquivalent { class_a, class_b } => {
            let expr_a = parse_class_expression(&class_a)?;
            let expr_b = parse_class_expression(&class_b)?;
            let result = reasoning_service
                .is_equivalent_to(&expr_a, &expr_b)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;
            OWLlinkResponseType::BooleanResponse { result }
        }

        OWLlinkRequestType::AreClassesDisjoint { class_a, class_b } => {
            let expr_a = parse_class_expression(&class_a)?;
            let expr_b = parse_class_expression(&class_b)?;
            let result = reasoning_service
                .is_disjoint_with(&expr_a, &expr_b)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;
            OWLlinkResponseType::BooleanResponse { result }
        }

        OWLlinkRequestType::IsEntailed { axiom: _ } => {
            // Entailment checking is complex; acknowledge and return false for now.
            // A full implementation would parse the axiom and delegate to the reasoner.
            OWLlinkResponseType::BooleanResponse { result: false }
        }

        OWLlinkRequestType::GetEquivalentClasses { class_expression } => {
            let class_expr = parse_class_expression(&class_expression)?;
            let equiv = reasoning_service
                .get_equivalent_classes(&class_expr)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;
            let class_iris: Vec<String> = equiv
                .into_iter()
                .map(|c| format!("{:?}", c))
                .collect();
            OWLlinkResponseType::Classes { classes: class_iris }
        }

        OWLlinkRequestType::GetTypes { individual, direct } => {
            let ind = parse_individual(&individual)?;
            let types = reasoning_service
                .get_types(&ind, direct.unwrap_or(false))
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;
            let class_iris: Vec<String> = types.into_iter().map(|c| format!("{:?}", c)).collect();
            OWLlinkResponseType::Classes { classes: class_iris }
        }

        OWLlinkRequestType::GetFlattenedTypes { individual } => {
            let ind = parse_individual(&individual)?;
            let types = reasoning_service
                .get_types(&ind, false)
                .await
                .map_err(|e| Error::ReasoningError(e.to_string()))?;
            let class_iris: Vec<String> = types.into_iter().map(|c| format!("{:?}", c)).collect();
            OWLlinkResponseType::Classes { classes: class_iris }
        }

        OWLlinkRequestType::GetSameIndividuals { individual: _ } => {
            // Not yet implemented in ReasoningService; return empty.
            OWLlinkResponseType::Individuals { individuals: vec![] }
        }

        OWLlinkRequestType::GetDifferentIndividuals { individual: _ } => {
            // Not yet implemented in ReasoningService; return empty.
            OWLlinkResponseType::Individuals { individuals: vec![] }
        }

        OWLlinkRequestType::AreIndividualsRelated {
            individual_a,
            individual_b: _,
            role: _,
        } => {
            // Evaluate via object property values query.
            // For now return false — full implementation requires property parsing.
            let _ = individual_a;
            OWLlinkResponseType::BooleanResponse { result: false }
        }

        OWLlinkRequestType::GetSubObjectProperties { object_property: _, direct: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }

        OWLlinkRequestType::GetSuperObjectProperties { object_property: _, direct: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }

        OWLlinkRequestType::GetEquivalentObjectProperties { object_property: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }

        OWLlinkRequestType::GetSubDataProperties { data_property: _, direct: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }

        OWLlinkRequestType::GetSuperDataProperties { data_property: _, direct: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }

        OWLlinkRequestType::GetEquivalentDataProperties { data_property: _ } => {
            OWLlinkResponseType::Properties { properties: vec![] }
        }
    };

    Ok(OWLlinkResponse {
        request_id: request.request_id,
        knowledge_base: request.knowledge_base,
        response: response_type,
    })
}

/// Parse a class expression from string using Manchester Syntax
fn parse_class_expression(expr_str: &str) -> Result<ClassExpression> {
    let parser = crate::parsers::manchester::ManchesterParser::default();
    parser
        .parse_class_expression(expr_str)
        .map_err(|e| Error::ParseError(format!("Manchester syntax error: {}", e)))
}

/// Build a named `Individual` from an IRI string.
fn parse_individual(iri: &str) -> Result<crate::ontology::Individual> {
    use crate::ontology::{Individual, IRI, NamedIndividual};
    Ok(Individual::Named(NamedIndividual { iri: IRI::new(iri) }))
}

/// OWLlink error for warp rejection
#[derive(Debug)]
struct OWLlinkError(String);

impl warp::reject::Reject for OWLlinkError {}
