//! Server Interface Layer for Oxidowl
//!
//! This module provides server interfaces including OWLlink protocol,
//! SPARQL endpoint, and REST API for reasoning services.

pub mod owllink;
#[cfg(feature = "sparql")]
pub mod sparql;
pub mod rest;

// Re-export main types for convenience
pub use rest::RestApiServer;
pub use owllink::OWLlinkServer;
#[cfg(feature = "sparql")]
pub use sparql::SparqlServer;

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

    /// Create a new server manager with default configuration
    pub fn with_defaults(reasoning_service: Arc<ReasoningService>) -> Self {
        Self::new(ServerConfig::default(), reasoning_service)
    }

    /// Create a new server manager with custom port
    pub fn with_port(reasoning_service: Arc<ReasoningService>, port: u16) -> Self {
        let mut config = ServerConfig::default();
        config.enable_server = true;
        config.port = port;
        config.rest_api_port = port;
        Self::new(config, reasoning_service)
    }

    /// Create a new server manager with custom bind address and port
    pub fn with_bind_and_port(
        reasoning_service: Arc<ReasoningService>,
        bind_address: String,
        port: u16,
    ) -> Self {
        let mut config = ServerConfig::default();
        config.enable_server = true;
        config.bind_address = bind_address;
        config.port = port;
        config.rest_api_port = port;
        Self::new(config, reasoning_service)
    }

    /// Enable specific server types
    pub fn enable_owllink(&mut self, enabled: bool) -> &mut Self {
        self.config.enable_owllink = enabled;
        self
    }

    /// Enable SPARQL server
    pub fn enable_sparql(&mut self, enabled: bool) -> &mut Self {
        self.config.enable_sparql = enabled;
        self
    }

    /// Enable REST API
    pub fn enable_rest_api(&mut self, enabled: bool) -> &mut Self {
        self.config.enable_rest_api = enabled;
        self
    }

    /// Set OWLlink port
    pub fn set_owllink_port(&mut self, port: u16) -> &mut Self {
        self.config.owllink_port = port;
        self
    }

    /// Set SPARQL port
    pub fn set_sparql_port(&mut self, port: u16) -> &mut Self {
        self.config.sparql_port = port;
        self
    }

    /// Set REST API port
    pub fn set_rest_api_port(&mut self, port: u16) -> &mut Self {
        self.config.rest_api_port = port;
        self
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

        #[cfg(feature = "sparql")]
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
    #[cfg(feature = "sparql")]
    Sparql(sparql::SparqlServerHandle),
    /// REST server handle
    Rest(rest::RestApiServerHandle),
}

impl ServerHandle {
    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        match self {
            ServerHandle::OWLlink(handle) => handle.stop().await,
            #[cfg(feature = "sparql")]
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