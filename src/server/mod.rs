//! Server Interface Layer for Oxidowl
//!
//! This module provides server interfaces including OWLlink protocol,
//! SPARQL endpoint, and REST API for reasoning services.

pub mod owllink;
pub mod sparql;
pub mod rest;

use crate::{
    Error, Result,
    config::ServerConfig,
    reasoning::ReasoningService,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main server manager
#[derive(Debug)]
pub struct ServerManager {
    /// Server configuration
    config: ServerConfig,
    /// Reasoning service
    reasoning_service: Arc<ReasoningService>,
    /// Running servers
    servers: Vec<ServerHandle>,
}

impl ServerManager {
    /// Create a new server manager
    pub fn new(config: ServerConfig, reasoning_service: Arc<ReasoningService>) -> Self {
        Self {
            config,
            reasoning_service,
            servers: Vec::new(),
        }
    }

    /// Start all configured servers
    pub async fn start_all(&mut self) -> Result<()> {
        if self.config.enable_owllink {
            let owllink_server = owllink::OWLlinkServer::new(
                self.config.owllink_port,
                self.config.bind_address.clone(),
                self.reasoning_service.clone(),
            );
            let handle = owllink_server.start().await?;
            self.servers.push(ServerHandle::OWLlink(handle));
        }

        if self.config.enable_sparql {
            let sparql_server = sparql::SparqlServer::new(
                self.config.sparql_port,
                self.config.bind_address.clone(),
                self.reasoning_service.clone(),
            );
            let handle = sparql_server.start().await?;
            self.servers.push(ServerHandle::Sparql(handle));
        }

        if self.config.enable_rest_api {
            // Create a mock explanation service for now
            let explanation_service = Arc::new(crate::explanation::ExplanationService::new());
            
            let rest_server = rest::RestApiServer::new(
                self.config.rest_api_port,
                self.config.bind_address.clone(),
                self.reasoning_service.clone(),
                explanation_service,
            );
            let handle = rest_server.start().await?;
            self.servers.push(ServerHandle::Rest(handle));
        }

        tracing::info!("Started {} server(s)", self.servers.len());
        Ok(())
    }

    /// Stop all servers
    pub async fn stop_all(&mut self) -> Result<()> {
        for server in self.servers.drain(..) {
            server.stop().await?;
        }
        Ok(())
    }

    /// Get server status
    pub fn get_status(&self) -> ServerStatus {
        ServerStatus {
            running_servers: self.servers.len(),
            owllink_enabled: self.config.enable_owllink,
            sparql_enabled: self.config.enable_sparql,
            rest_api_enabled: self.config.enable_rest_api,
        }
    }
}

/// Handle for a running server
#[derive(Debug)]
pub enum ServerHandle {
    /// OWLlink server handle
    OWLlink(owllink::OWLlinkServerHandle),
    /// SPARQL server handle
    Sparql(sparql::SparqlServerHandle),
    /// REST server handle
    Rest(rest::RestApiServerHandle),
}

impl ServerHandle {
    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        match self {
            ServerHandle::OWLlink(handle) => handle.stop().await,
            ServerHandle::Sparql(handle) => handle.stop().await,
            ServerHandle::Rest(handle) => handle.stop().await,
        }
    }
}

/// Server status information
#[derive(Debug, Clone)]
pub struct ServerStatus {
    /// Number of running servers
    pub running_servers: usize,
    /// OWLlink server enabled
    pub owllink_enabled: bool,
    /// SPARQL server enabled
    pub sparql_enabled: bool,
    /// REST API enabled
    pub rest_api_enabled: bool,
}