//! Example of starting the Oxidowl reasoner with a web server
//!
//! This example demonstrates how to:
//! 1. Load an ontology
//! 2. Start a web server for remote access (OWLlink, SPARQL, REST API)
//! 3. Configure server ports and bind addresses
//!
//! Usage:
//! ```bash
//! # Run with default settings (requires 'server' feature)
//! cargo run --example server_example --features server -- path/to/ontology.owl
//!
//! # Run on a custom port
//! cargo run --example server_example --features server -- path/to/ontology.owl --port 9090
//! ```

use clap::Parser;
use oxidowl::{OntologyFormat, Reasoner, ReasonerConfig, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "server_example")]
#[command(about = "Example of running Oxidowl with a web server")]
struct Args {
    /// Path to the ontology file
    ontology: PathBuf,

    /// Server port (default: 8080)
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Bind address (default: 127.0.0.1)
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,

    /// Enable OWLlink server
    #[arg(long)]
    owllink: bool,

    /// OWLlink port (default: 8081)
    #[arg(long, default_value = "8081")]
    owllink_port: u16,
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<()> {
    use std::sync::Arc;

    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    println!("Oxidowl Server Example");
    println!("======================\n");

    // Create reasoner with default configuration
    let mut config = ReasonerConfig::default();

    // Configure server settings
    config.server.enable_server = true;
    config.server.rest_api_port = args.port;
    config.server.bind_address = args.bind.clone();

    if args.owllink {
        config.server.enable_owllink = true;
        config.server.owllink_port = args.owllink_port;
    }

    println!("Creating reasoner...");
    let mut reasoner = Reasoner::new(config.clone())?;

    // Load the ontology
    println!("Loading ontology from: {}", args.ontology.display());
    reasoner.load_ontology_from_file(&args.ontology, OntologyFormat::Auto)?;
    println!("Ontology loaded successfully!\n");

    // Perform initial consistency check
    println!("Performing consistency check...");
    let is_consistent = reasoner.is_consistent()?;
    println!(
        "Ontology is {}\n",
        if is_consistent {
            "consistent"
        } else {
            "inconsistent"
        }
    );

    // Create and start the server
    println!("Starting web server...");

    let ontology = reasoner
        .get_ontology()
        .ok_or_else(|| oxidowl::Error::io("No ontology loaded".to_string()))?;

    let ontology_clone = ontology
        .read()
        .map_err(|_| oxidowl::Error::io("Failed to acquire ontology read lock".to_string()))?
        .clone();

    let reasoning_service = Arc::new(oxidowl::reasoning::ReasoningService::new(
        ontology_clone,
        config.clone(),
    ));

    let mut server_manager = oxidowl::ServerManager::new(config.server.clone(), reasoning_service);

    server_manager.start_all().await?;

    println!("\n✓ Server started successfully!");
    println!("\nServer endpoints:");
    println!("  REST API: http://{}:{}", args.bind, args.port);
    if args.owllink {
        println!("  OWLlink: http://{}:{}", args.bind, args.owllink_port);
    }
    println!("\nPress Ctrl+C to stop the server...\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| oxidowl::Error::io(format!("Failed to listen for shutdown signal: {}", e)))?;

    println!("\nShutting down servers...");
    server_manager.stop_all().await?;
    println!("Servers stopped successfully.");

    Ok(())
}

#[cfg(not(feature = "server"))]
fn main() {
    eprintln!("This example requires the 'server' feature to be enabled.");
    eprintln!("Run with: cargo run --example server_example --features server");
    std::process::exit(1);
}
