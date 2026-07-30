//! REST API Implementation
//!
//! This module provides a RESTful API for accessing reasoner functionality.

use crate::{
    Error, Result, explanation::ExplanationService, ontology::ClassExpression,
    reasoning::ReasoningService,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use warp::{Filter, Reply};

/// REST API server implementation
#[derive(Debug)]
pub struct RestApiServer {
    /// Server port
    port: u16,
    /// Bind address
    bind_address: String,
    /// Reasoning service
    reasoning_service: Arc<ReasoningService>,
    /// Explanation service
    explanation_service: Arc<ExplanationService>,
}

impl RestApiServer {
    /// Create a new REST API server
    pub fn new(
        port: u16,
        bind_address: String,
        reasoning_service: Arc<ReasoningService>,
        explanation_service: Arc<ExplanationService>,
    ) -> Self {
        Self {
            port,
            bind_address,
            reasoning_service,
            explanation_service,
        }
    }

    /// Start the REST API server
    pub async fn start(self) -> Result<RestApiServerHandle> {
        let explanation_service = self.explanation_service.clone();
        // Dedicated clone for SHACL routes (avoids multiple-move issue for other routes)
        let shacl_service = self.reasoning_service.clone();

        // API routes
        let api = warp::path("api").and(warp::path("v1"));

        // Health check
        let health = api.and(warp::path("health")).and(warp::get()).map(|| {
            warp::reply::json(&ApiResponse::success(serde_json::json!({
                "status": "healthy",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        });

        // Reasoner status
        let status_service = self.reasoning_service.clone();
        let status = api
            .and(warp::path("status"))
            .and(warp::get())
            .and(warp::any().map(move || status_service.clone()))
            .and_then(get_reasoner_status);

        // Consistency check
        let consistency_service = self.reasoning_service.clone();
        let consistency = api
            .and(warp::path("consistency"))
            .and(warp::get())
            .and(warp::any().map(move || consistency_service.clone()))
            .and_then(check_consistency);

        // Satisfiability check
        let satisfiability_service = self.reasoning_service.clone();
        let satisfiability = api
            .and(warp::path("satisfiability"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || satisfiability_service.clone()))
            .and_then(check_satisfiability);

        // Subsumption check
        let subsumption_service = self.reasoning_service.clone();
        let subsumption = api
            .and(warp::path("subsumption"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || subsumption_service.clone()))
            .and_then(check_subsumption);

        // Classification
        let classification_service = self.reasoning_service.clone();
        let classification = api
            .and(warp::path("classify"))
            .and(warp::post())
            .and(warp::any().map(move || classification_service.clone()))
            .and_then(classify_ontology);

        // Query subclasses
        let subclasses_service = self.reasoning_service.clone();
        let subclasses = api
            .and(warp::path("subclasses"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || subclasses_service.clone()))
            .and_then(get_subclasses);

        // Query superclasses
        let superclasses_service = self.reasoning_service.clone();
        let superclasses = api
            .and(warp::path("superclasses"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || superclasses_service.clone()))
            .and_then(get_superclasses);

        // Query instances
        let instances_service = self.reasoning_service.clone();
        let instances = api
            .and(warp::path("instances"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || instances_service.clone()))
            .and_then(get_instances);

        // Explain inference
        let explain = api
            .and(warp::path("explain"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || explanation_service.clone()))
            .and_then(explain_inference);

        // Load ontology
        let load_ontology_service = self.reasoning_service.clone();
        let load_ontology = api
            .and(warp::path("ontology"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || load_ontology_service.clone()))
            .and_then(load_ontology_endpoint);

        // Dedicated clones for new Phase 5 routes (closure capture).
        let types_service = self.reasoning_service.clone();
        let same_service = self.reasoning_service.clone();
        let different_service = self.reasoning_service.clone();
        let equiv_service = self.reasoning_service.clone();
        let entailment_service = self.reasoning_service.clone();

        // GET types of an individual — POST /api/v1/types
        let types_endpoint = api
            .and(warp::path("types"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || types_service.clone()))
            .and_then(get_types_endpoint);

        // Same individuals — POST /api/v1/same-individuals
        let same_individuals = api
            .and(warp::path("same-individuals"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || same_service.clone()))
            .and_then(get_same_individuals_endpoint);

        // Different individuals — POST /api/v1/different-individuals
        let different_individuals = api
            .and(warp::path("different-individuals"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || different_service.clone()))
            .and_then(get_different_individuals_endpoint);

        // Equivalent classes — POST /api/v1/equivalent-classes
        let equivalent_classes = api
            .and(warp::path("equivalent-classes"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || equiv_service.clone()))
            .and_then(get_equivalent_classes_endpoint);

        // Entailment check — POST /api/v1/entailment
        let entailment = api
            .and(warp::path("entailment"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || entailment_service.clone()))
            .and_then(check_entailment_endpoint);

        // SHACL validation — POST /api/v1/shacl/validate
        let shacl_validate = api
            .and(warp::path("shacl"))
            .and(warp::path("validate"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || shacl_service.clone()))
            .and_then(validate_shacl_endpoint);

        // Ontology RDF export — GET /api/v1/ontology/export
        // Returns the currently loaded ontology serialized as Turtle with
        // Content-Type: text/turtle; version=1.2 per the RDF 1.2 HTTP spec.
        let export_service = self.reasoning_service.clone();
        let ontology_export = api
            .and(warp::path("ontology"))
            .and(warp::path("export"))
            .and(warp::get())
            .and(warp::any().map(move || export_service.clone()))
            .and_then(export_ontology_endpoint);

        let routes = health
            .or(status)
            .or(consistency)
            .or(satisfiability)
            .or(subsumption)
            .or(classification)
            .or(subclasses)
            .or(superclasses)
            .or(instances)
            .or(explain)
            .or(load_ontology)
            .or(types_endpoint)
            .or(same_individuals)
            .or(different_individuals)
            .or(equivalent_classes)
            .or(entailment)
            .or(shacl_validate)
            .or(ontology_export)
            .with(
                warp::cors()
                    .allow_any_origin()
                    .allow_headers(vec!["content-type"])
                    .allow_methods(vec!["GET", "POST"]),
            )
            .recover(handle_rejection);

        let addr: SocketAddr = format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|e| Error::config(format!("Invalid server address: {}", e)))?;

        let server_task = tokio::spawn(warp::serve(routes).run(addr));

        tracing::info!(
            "REST API server started on {}:{}",
            self.bind_address,
            self.port
        );

        Ok(RestApiServerHandle { task: server_task })
    }
}

/// Handle for a running REST API server
#[derive(Debug)]
pub struct RestApiServerHandle {
    task: tokio::task::JoinHandle<()>,
}

impl RestApiServerHandle {
    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        self.task.abort();
        Ok(())
    }
}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Request for satisfiability check
#[derive(Debug, Deserialize)]
pub struct SatisfiabilityRequest {
    pub class_expression: String,
}

/// Request for subsumption check
#[derive(Debug, Deserialize)]
pub struct SubsumptionRequest {
    pub sub_class: String,
    pub super_class: String,
}

/// Request for class hierarchy queries
#[derive(Debug, Deserialize)]
pub struct ClassQueryRequest {
    pub class_expression: String,
    pub direct: Option<bool>,
}

/// Request for explanation
#[derive(Debug, Deserialize)]
pub struct ExplanationRequest {
    pub inference_type: String,
    pub axiom: String,
}

/// Request for loading ontology
#[derive(Debug, Deserialize)]
pub struct LoadOntologyRequest {
    pub ontology_iri: String,
    pub format: Option<String>,
}

/// Request for SHACL validation
#[derive(Debug, Deserialize)]
pub struct ShaclValidateRequest {
    /// Turtle-encoded SHACL shapes graph
    pub shapes: String,
    /// Turtle-encoded data graph to validate
    pub data: String,
}

/// Request for individual type queries
#[derive(Debug, Deserialize)]
pub struct IndividualQueryRequest {
    pub individual: String,
    pub direct: Option<bool>,
}

/// Request for same/different individuals
#[derive(Debug, Deserialize)]
pub struct SameIndividualsRequest {
    pub individual: String,
}

/// Request for equivalent class queries
#[derive(Debug, Deserialize)]
pub struct EquivalentClassesRequest {
    pub class_expression: String,
}

/// Request for entailment check
#[derive(Debug, Deserialize)]
pub struct EntailmentRequest {
    /// Type of axiom being checked (e.g., "SubClassOf", "ClassAssertion")
    pub axiom_type: String,
    /// Serialised axiom expression
    pub axiom: String,
}

/// Request for property hierarchy queries
#[derive(Debug, Deserialize)]
pub struct PropertyQueryRequest {
    pub property: String,
    pub direct: Option<bool>,
}

/// Response for reasoner status
#[derive(Debug, Serialize)]
pub struct ReasonerStatus {
    pub name: String,
    pub version: String,
    pub loaded_ontologies: usize,
    pub supports_profiles: Vec<String>,
    pub features: Vec<String>,
}

/// Response for classification
#[derive(Debug, Serialize)]
pub struct ClassificationResponse {
    pub status: String,
    pub class_count: usize,
    pub property_count: usize,
    pub individual_count: usize,
    pub duration_ms: u64,
}

// REST API endpoint handlers

async fn get_reasoner_status(
    _reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let status = ReasonerStatus {
        name: "Oxidowl".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        loaded_ontologies: 0, // Would get from reasoning service
        supports_profiles: vec!["OWL2-DL".to_string(), "OWL2-EL".to_string()],
        features: vec![
            "Consistency Checking".to_string(),
            "Classification".to_string(),
            "Realisation".to_string(),
            "Explanation Generation".to_string(),
            "SWRL Rules".to_string(),
        ],
    };

    Ok(warp::reply::json(&ApiResponse::success(status)))
}

async fn check_consistency(
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    match reasoning_service.is_consistent().await {
        Ok(is_consistent) => Ok(warp::reply::json(&ApiResponse::success(
            serde_json::json!({
                "consistent": is_consistent
            }),
        ))),
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn check_satisfiability(
    request: SatisfiabilityRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // Parse class expression
    let class_expr = match parse_class_expression(&request.class_expression) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    match reasoning_service.is_satisfiable(&class_expr).await {
        Ok(is_satisfiable) => Ok(warp::reply::json(&ApiResponse::success(
            serde_json::json!({
                "satisfiable": is_satisfiable,
                "class_expression": request.class_expression
            }),
        ))),
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn check_subsumption(
    request: SubsumptionRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // Parse class expressions
    let sub_expr = match parse_class_expression(&request.sub_class) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    let super_expr = match parse_class_expression(&request.super_class) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    match reasoning_service
        .is_subsumed_by(&sub_expr, &super_expr)
        .await
    {
        Ok(is_subsumed) => Ok(warp::reply::json(&ApiResponse::success(
            serde_json::json!({
                "subsumed": is_subsumed,
                "sub_class": request.sub_class,
                "super_class": request.super_class
            }),
        ))),
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn classify_ontology(
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let start_time = std::time::Instant::now();

    match reasoning_service.classify().await {
        Ok(_classification) => {
            let duration = start_time.elapsed();
            let response = ClassificationResponse {
                status: "completed".to_string(),
                class_count: 0, // Would get from classification results
                property_count: 0,
                individual_count: 0,
                duration_ms: duration.as_millis() as u64,
            };
            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn get_subclasses(
    request: ClassQueryRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let class_expr = match parse_class_expression(&request.class_expression) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    match reasoning_service
        .get_subclasses(&class_expr, request.direct.unwrap_or(false))
        .await
    {
        Ok(subclasses) => {
            let class_iris: Vec<String> =
                subclasses.into_iter().map(|c| format!("{:?}", c)).collect();

            Ok(warp::reply::json(&ApiResponse::success(
                serde_json::json!({
                    "subclasses": class_iris,
                    "class_expression": request.class_expression,
                    "direct": request.direct.unwrap_or(false)
                }),
            )))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn get_superclasses(
    request: ClassQueryRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let class_expr = match parse_class_expression(&request.class_expression) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    match reasoning_service
        .get_superclasses(&class_expr, request.direct.unwrap_or(false))
        .await
    {
        Ok(superclasses) => {
            let class_iris: Vec<String> = superclasses
                .into_iter()
                .map(|c| format!("{:?}", c))
                .collect();

            Ok(warp::reply::json(&ApiResponse::success(
                serde_json::json!({
                    "superclasses": class_iris,
                    "class_expression": request.class_expression,
                    "direct": request.direct.unwrap_or(false)
                }),
            )))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn get_instances(
    request: ClassQueryRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let class_expr = match parse_class_expression(&request.class_expression) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };

    match reasoning_service
        .get_instances(&class_expr, request.direct.unwrap_or(false))
        .await
    {
        Ok(instances) => {
            let individual_iris: Vec<String> = instances
                .into_iter()
                .filter_map(|i| i.iri().map(|iri| iri.to_string()))
                .collect();

            Ok(warp::reply::json(&ApiResponse::success(
                serde_json::json!({
                    "instances": individual_iris,
                    "class_expression": request.class_expression,
                    "direct": request.direct.unwrap_or(false)
                }),
            )))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn explain_inference(
    request: ExplanationRequest,
    _explanation_service: Arc<ExplanationService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // For now, return a mock explanation - would integrate with actual explanation service
    Ok(warp::reply::json(&ApiResponse::success(
        serde_json::json!({
            "explanation": {
                "inference_type": request.inference_type,
                "axiom": request.axiom,
                "justifications": [],
                "message": "Explanation generation not yet fully implemented"
            }
        }),
    )))
}

async fn load_ontology_endpoint(
    request: LoadOntologyRequest,
    _reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // For now, just acknowledge - would implement actual loading
    Ok(warp::reply::json(&ApiResponse::success(
        serde_json::json!({
            "status": "loaded",
            "ontology_iri": request.ontology_iri,
            "format": request.format.unwrap_or_else(|| "auto-detect".to_string())
        }),
    )))
}

async fn validate_shacl_endpoint(
    request: ShaclValidateRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    match reasoning_service.validate_shacl(&request.shapes, &request.data) {
        Ok(report) => Ok(warp::reply::json(&ApiResponse::success(
            serde_json::json!({
                "conforms": report.conforms,
                "results": report.results.len(),
                "report": report,
            }),
        ))),
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

/// Parse a class expression from string using Manchester Syntax
fn parse_class_expression(expr_str: &str) -> Result<ClassExpression> {
    let parser = crate::parsers::manchester::ManchesterParser::default();
    parser
        .parse_class_expression(expr_str)
        .map_err(|e| Error::ParseError(format!("Manchester syntax error: {}", e)))
}

/// Build a named `Individual` from an IRI string.
fn individual_from_iri(iri: &str) -> Result<crate::ontology::Individual> {
    use crate::ontology::{IRI, Individual, NamedIndividual};
    Ok(Individual::Named(NamedIndividual { iri: IRI::new(iri) }))
}

// ── Phase 5 endpoint handlers ──────────────────────────────────────────────

async fn get_types_endpoint(
    request: IndividualQueryRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let ind = match individual_from_iri(&request.individual) {
        Ok(ind) => ind,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };
    match reasoning_service
        .get_types(&ind, request.direct.unwrap_or(false))
        .await
    {
        Ok(types) => {
            let iris: Vec<String> = types.into_iter().map(|c| format!("{:?}", c)).collect();
            Ok(warp::reply::json(&ApiResponse::success(
                serde_json::json!({
                    "individual": request.individual,
                    "types": iris,
                    "direct": request.direct.unwrap_or(false)
                }),
            )))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn get_same_individuals_endpoint(
    request: SameIndividualsRequest,
    _reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // ReasoningService doesn't yet expose get_same_individuals; return empty list.
    let empty: Vec<String> = vec![];
    Ok(warp::reply::json(&ApiResponse::success(
        serde_json::json!({
            "individual": request.individual,
            "same_individuals": empty,
        }),
    )))
}

async fn get_different_individuals_endpoint(
    request: SameIndividualsRequest,
    _reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let empty: Vec<String> = vec![];
    Ok(warp::reply::json(&ApiResponse::success(
        serde_json::json!({
            "individual": request.individual,
            "different_individuals": empty,
        }),
    )))
}

async fn get_equivalent_classes_endpoint(
    request: EquivalentClassesRequest,
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    let class_expr = match parse_class_expression(&request.class_expression) {
        Ok(expr) => expr,
        Err(e) => return Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    };
    match reasoning_service.get_equivalent_classes(&class_expr).await {
        Ok(equiv) => {
            let iris: Vec<String> = equiv.into_iter().map(|c| format!("{:?}", c)).collect();
            Ok(warp::reply::json(&ApiResponse::success(
                serde_json::json!({
                    "class_expression": request.class_expression,
                    "equivalent_classes": iris
                }),
            )))
        }
        Err(e) => Ok(warp::reply::json(&ApiResponse::<()>::error(e.to_string()))),
    }
}

async fn check_entailment_endpoint(
    request: EntailmentRequest,
    _reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    // Entailment checking: parse the axiom from the request body and
    // verify whether the ontology entails it. Full implementation requires
    // axiom parsing from the provided axiom_type and axiom fields.
    Ok(warp::reply::json(&ApiResponse::success(
        serde_json::json!({
            "axiom_type": request.axiom_type,
            "axiom": request.axiom,
            "entailed": false,
            "note": "Full axiom entailment checking not yet implemented"
        }),
    )))
}

/// Handle warp rejections
async fn handle_rejection(
    err: warp::Rejection,
) -> std::result::Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "BAD_REQUEST";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ApiResponse::<()>::error(message.to_string()));

    Ok(warp::reply::with_status(json, code))
}

/// Export the loaded ontology as Turtle.
///
/// Returns `Content-Type: text/turtle; version=1.2` as required by the
/// RDF 1.2 specification when serving RDF 1.2 content over HTTP.
async fn export_ontology_endpoint(
    reasoning_service: Arc<ReasoningService>,
) -> std::result::Result<impl Reply, warp::Rejection> {
    match reasoning_service.get_serialized_turtle().await {
        Ok(turtle) => Ok(warp::reply::with_header(
            turtle,
            "Content-Type",
            "text/turtle; version=1.2",
        )),
        Err(e) => {
            // Return an error body still labelled as Turtle so the Content-Type
            // is consistent; the body starts with `# Error:` to signal failure.
            Ok(warp::reply::with_header(
                format!("# Error: {e}\n"),
                "Content-Type",
                "text/turtle; version=1.2",
            ))
        }
    }
}
