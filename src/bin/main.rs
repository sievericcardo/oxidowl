//! Main executable for the Oxidowl reasoner
//!
//! This binary provides a command-line interface
// supporting all major reasoning tasks and server modes.

use clap::{Parser, Subcommand, ValueEnum};
use oxidowl::{
    config::ReasonerConfig,
    core::reasoner::Reasoner,
    ontology::OntologyFormat,
    Result,
};
use std::{
    fs,
    path::PathBuf,
    time::Instant,
};
use tracing::{error, info, Level};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "oxidowl")]
#[command(about = "A tableau-based reasoner for the Description Logic SROIQV(D)")]
#[command(version = oxidowl::VERSION)]
#[command(author = "Team Oxidowl")]
#[command(long_about = None)]
struct Cli {
    /// Configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Check ontology consistency
    Consistency {
        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Perform ontology classification
    Classification {
        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output file for class hierarchy
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Check class satisfiability
    Satisfiability {
        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Class IRI to check
        #[arg(short = 'x', long, value_name = "IRI")]
        class_iri: String,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Perform individual realization
    Realization {
        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output file for realization results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Process OWLlink request file
    OwlLinkFile {
        /// Input OWLlink request file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output OWLlink response file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /*
    /*
    /// Start OWLlink HTTP server
    OwlLinkServer {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,
    },
    */
    */

    /// Process SPARQL file
    SparqlFile {
        /// Input SPARQL file
        #[arg(short, long, value_name = "FILE")]
        sparql: PathBuf,

        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Output results file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /*
    /*
    /// Start SPARQL HTTP server
    SparqlServer {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,
    },
    */
    */
}

#[derive(ValueEnum, Clone, Debug)]
enum InputFormat {
    /// OWL 2 XML format
    OwlXml,
    /// OWL 2 Functional Syntax
    Functional,
    /// RDF/XML format
    RdfXml,
    /// Turtle format
    Turtle,
    /// N-Triples format
    NTriples,
    /// Auto-detect format
    Auto,
}

impl From<InputFormat> for OntologyFormat {
    fn from(format: InputFormat) -> Self {
        match format {
            InputFormat::OwlXml => OntologyFormat::OwlXml,
            InputFormat::Functional => OntologyFormat::Functional,
            InputFormat::RdfXml => OntologyFormat::RdfXml,
            InputFormat::Turtle => OntologyFormat::Turtle,
            InputFormat::NTriples => OntologyFormat::NTriples,
            InputFormat::Auto => OntologyFormat::Auto,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    setup_logging(cli.verbose, cli.quiet);

    // Print version information
    if !cli.quiet {
        println!("{}", oxidowl::version_info());
        println!("Starting Oxidowl ...");
    }

    // Load configuration
    let config = load_configuration(cli.config.as_deref())?;

    // Execute command
    let start_time = Instant::now();
    let result = execute_command(cli.command, config).await;
    let elapsed = start_time.elapsed();

    match result {
        Ok(_) => {
            if !cli.quiet {
                info!("Operation completed successfully in {:?}", elapsed);
                println!("Stopping Oxidowl ...");
            }
            Ok(())
        }
        Err(e) => {
            error!("Operation failed: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn setup_logging(verbosity: u8, quiet: bool) {
    let level = if quiet {
        Level::ERROR
    } else {
        match verbosity {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };

    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
}

fn load_configuration(config_path: Option<&std::path::Path>) -> Result<ReasonerConfig> {
    if let Some(path) = config_path {
        info!("Loading configuration from: {}", path.display());
        ReasonerConfig::load_from_file(path)
    } else {
        Ok(ReasonerConfig::default())
    }
}

async fn execute_command(command: Commands, config: ReasonerConfig) -> Result<()> {
    match command {
        Commands::Consistency { input, output, format } => {
            execute_consistency_check(input, output, format, config).await
        }
        Commands::Classification { input, output, format } => {
            execute_classification(input, output, format, config).await
        }
        Commands::Satisfiability { input, class_iri, output, format } => {
            execute_satisfiability_check(input, class_iri, output, format, config).await
        }
        Commands::Realization { input, output, format } => {
            execute_realization(input, output, format, config).await
        }
        Commands::OwlLinkFile { input, output } => {
            execute_owllink_file(input, output, config).await
        }
        /*
        /*
        Commands::OwlLinkServer { port, bind } => {
            execute_owllink_server(port, bind, config).await
        }
        */
        */
        Commands::SparqlFile { sparql, input, output } => {
            execute_sparql_file(sparql, input, output, config).await
        }
        /*
        /*
        Commands::SparqlServer { port, bind } => {
            execute_sparql_server(port, bind, config).await
        }
        */
        */
    }
}

async fn execute_consistency_check(
    input: PathBuf,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Performing consistency check on: {}", input.display());

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map(Into::into).unwrap_or(OntologyFormat::Auto);
    
    reasoner.load_ontology_from_file(&input, ontology_format)?;
    
    let is_consistent = reasoner.is_consistent()?;
    
    info!("Consistency check result: {}", is_consistent);
    
    if let Some(output_path) = output {
        let result = if is_consistent { "consistent" } else { "inconsistent" };
        fs::write(output_path, result)?;
    } else {
        println!("Result: {}", if is_consistent { "consistent" } else { "inconsistent" });
    }
    
    Ok(())
}

async fn execute_classification(
    input: PathBuf,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Performing classification on: {}", input.display());

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map(Into::into).unwrap_or(OntologyFormat::Auto);
    
    reasoner.load_ontology_from_file(&input, ontology_format)?;
    
    let hierarchy = reasoner.classify()?;
    
    info!("Classification completed");
    
    if let Some(output_path) = output {
        hierarchy.save_to_file(output_path)?;
    } else {
        println!("Classification completed. Use -o to save results.");
    }
    
    Ok(())
}

async fn execute_satisfiability_check(
    input: PathBuf,
    class_iri: String,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Checking satisfiability of class: {}", class_iri);

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map(Into::into).unwrap_or(OntologyFormat::Auto);
    
    reasoner.load_ontology_from_file(&input, ontology_format)?;
    
    let is_satisfiable = reasoner.is_class_satisfiable(&class_iri)?;
    
    info!("Satisfiability result: {}", is_satisfiable);
    
    if let Some(output_path) = output {
        let result = if is_satisfiable { "satisfiable" } else { "unsatisfiable" };
        fs::write(output_path, result)?;
    } else {
        println!("Result: {}", if is_satisfiable { "satisfiable" } else { "unsatisfiable" });
    }
    
    Ok(())
}

async fn execute_realization(
    input: PathBuf,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Performing realization on: {}", input.display());

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map(Into::into).unwrap_or(OntologyFormat::Auto);
    
    reasoner.load_ontology_from_file(&input, ontology_format)?;
    
    let realization = reasoner.realize()?;
    
    info!("Realization completed");
    
    if let Some(output_path) = output {
        realization.save_to_file(output_path)?;
    } else {
        println!("Realization completed. Use -o to save results.");
    }
    
    Ok(())
}

async fn execute_owllink_file(
    input: PathBuf,
    output: Option<PathBuf>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Processing OWLlink file: {}", input.display());
    
    let owllink_content = fs::read_to_string(&input)?;
    let reasoner = Reasoner::new(config)?;
    
    // Process OWLlink request
    let response = reasoner.process_owllink_request(&owllink_content)?;
    
    if let Some(output_path) = output {
        fs::write(output_path, response)?;
    } else {
        println!("{}", response);
    }
    
    Ok(())
}

/*
/*
async fn execute_owllink_server(
    port: u16,
    bind: String,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Starting OWLlink server on {}:{}", bind, port);
    
    let server = OwlLinkServer::new(config)?;
    server.start(&bind, port).await?;
    
    Ok(())
}
*/
*/

async fn execute_sparql_file(
    sparql: PathBuf,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Processing SPARQL file: {}", sparql.display());
    
    let sparql_content = fs::read_to_string(&sparql)?;
    let mut reasoner = Reasoner::new(config)?;
    
    // Load ontology if provided
    if let Some(ontology_file) = input {
        reasoner.load_ontology_from_file(&ontology_file, OntologyFormat::Auto)?;
    }
    
    // Execute SPARQL query
    let results = reasoner.execute_sparql_query(&sparql_content)?;
    
    if let Some(output_path) = output {
        fs::write(output_path, results)?;
    } else {
        println!("{}", results);
    }
    
    Ok(())
}

/*
/*
async fn execute_sparql_server(
    port: u16,
    bind: String,
    config: ReasonerConfig,
) -> Result<()> {
    info!("Starting SPARQL server on {}:{}", bind, port);
    
    let server = SparqlServer::new(config)?;
    server.start(&bind, port).await?;
    
    Ok(())
}
*/
*/
