//! Main executable for the Oxidowl reasoner
//!
//! This binary provides a command-line interface
// supporting all major reasoning tasks and server modes.

use clap::{Parser, Subcommand, ValueEnum};
use oxidowl::{Result, config::ReasonerConfig, core::reasoner::Reasoner, ontology::OntologyFormat};
use std::{fs, path::PathBuf, time::Instant};
use tracing::{Level, error, info};
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
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,

        /// Class IRI to check for satisfiability (default: check overall consistency)
        #[arg(short = 'x', long, value_name = "IRI")]
        class_iri: Option<String>,
    },

    /// Perform ontology classification
    Classification {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Default namespace for entity resolution
        #[arg(short, long, value_name = "NAMESPACE")]
        namespace: Option<String>,

        /// Output file for class hierarchy
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,

        /// Pretty print the hierarchy with proper indentation
        #[arg(short = 'P', long)]
        pretty_print: bool,
    },

    /// Classify object properties
    ClassifyObjectProperties {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Output file for object property hierarchy
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,

        /// Pretty print the hierarchy
        #[arg(short = 'P', long)]
        pretty_print: bool,
    },

    /// Classify data properties
    ClassifyDataProperties {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Output file for data property hierarchy
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,

        /// Pretty print the hierarchy
        #[arg(short = 'P', long)]
        pretty_print: bool,
    },

    /// Check class satisfiability
    Satisfiability {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

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

    /// Get subclasses of a class
    Subclasses {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Class IRI to get subclasses for
        #[arg(short = 'x', long, value_name = "IRI")]
        class_iri: String,

        /// Return only direct subclasses
        #[arg(short = 'd', long)]
        direct: bool,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Get superclasses of a class
    Superclasses {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Class IRI to get superclasses for
        #[arg(short = 'x', long, value_name = "IRI")]
        class_iri: String,

        /// Return only direct superclasses
        #[arg(short = 'd', long)]
        direct: bool,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Get equivalent classes of a class
    EquivalentClasses {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Class IRI to get equivalent classes for
        #[arg(short = 'x', long, value_name = "IRI")]
        class_iri: String,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Get unsatisfiable classes
    UnsatisfiableClasses {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Check entailment between premise and conclusion ontologies
    CheckEntailment {
        /// Premise ontology file
        #[arg(long, value_name = "FILE")]
        premise: PathBuf,

        /// Conclusion ontology file
        #[arg(long, value_name = "FILE")]
        conclusion: PathBuf,

        /// Output file for results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Print available prefixes
    PrintPrefixes {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Dump DL clauses (similar to `HermiT`)
    DumpClauses {
        /// Input ontology files
        #[arg(short, long, value_name = "FILE")]
        input: Vec<PathBuf>,

        /// Output file for DL clauses
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,

        /// Pretty print the clauses
        #[arg(short = 'P', long)]
        pretty_print: bool,
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

    /// Execute DL query
    Query {
        /// Input ontology file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// DL query string (Manchester Syntax)
        #[arg(short, long, value_name = "QUERY")]
        query: String,

        /// Default namespace for query parsing
        #[arg(short, long, value_name = "NAMESPACE")]
        namespace: Option<String>,

        /// Output file for query results
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Input format
        #[arg(short, long, value_enum)]
        format: Option<InputFormat>,
    },

    /// Process `OWLlink` request file
    OwlLinkFile {
        /// Input `OWLlink` request file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output `OWLlink` response file
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
        Ok(()) => {
            if !cli.quiet {
                info!("Operation completed successfully in {:?}", elapsed);
                println!("Stopping Oxidowl ...");
            }
            Ok(())
        }
        Err(e) => {
            error!("Operation failed: {}", e);
            eprintln!("Error: {e}");
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

/// Helper function to extract class names from complex class expressions
fn extract_class_names_from_expression(expr: &oxidowl::ontology::ClassExpression) -> String {
    match expr {
        oxidowl::ontology::ClassExpression::Class(class) => {
            // Extract just the class name from the IRI
            let iri_str = class.iri.to_string();
            if let Some(name) = iri_str.split('#').next_back() {
                name.to_string()
            } else if let Some(name) = iri_str.split('/').next_back() {
                name.to_string()
            } else {
                iri_str
            }
        }
        oxidowl::ontology::ClassExpression::ObjectUnionOf(union_classes) => {
            let class_names: Vec<String> = union_classes
                .iter()
                .map(extract_class_names_from_expression)
                .collect();
            class_names.join(" or ")
        }
        oxidowl::ontology::ClassExpression::ObjectIntersectionOf(intersection_classes) => {
            let class_names: Vec<String> = intersection_classes
                .iter()
                .map(extract_class_names_from_expression)
                .collect();
            class_names.join(" and ")
        }
        _ => format!("{expr:?}"),
    }
}

async fn execute_command(command: Commands, config: ReasonerConfig) -> Result<()> {
    match command {
        Commands::Consistency {
            input,
            output,
            format,
            class_iri,
        } => execute_consistency_check(input, output, format, class_iri, config).await,
        Commands::Classification {
            input,
            namespace,
            output,
            format,
            pretty_print,
        } => execute_classification(input, namespace, output, format, pretty_print, config).await,
        Commands::ClassifyObjectProperties {
            input,
            output,
            format,
            pretty_print,
        } => {
            execute_object_property_classification(input, output, format, pretty_print, config)
                .await
        }
        Commands::ClassifyDataProperties {
            input,
            output,
            format,
            pretty_print,
        } => {
            execute_data_property_classification(input, output, format, pretty_print, config).await
        }
        Commands::Satisfiability {
            input,
            class_iri,
            output,
            format,
        } => execute_satisfiability_check(input, class_iri, output, format, config).await,
        Commands::Subclasses {
            input,
            class_iri,
            direct,
            output,
            format,
        } => execute_subclasses_query(input, class_iri, direct, output, format, config).await,
        Commands::Superclasses {
            input,
            class_iri,
            direct,
            output,
            format,
        } => execute_superclasses_query(input, class_iri, direct, output, format, config).await,
        Commands::EquivalentClasses {
            input,
            class_iri,
            output,
            format,
        } => execute_equivalent_classes_query(input, class_iri, output, format, config).await,
        Commands::UnsatisfiableClasses {
            input,
            output,
            format,
        } => execute_unsatisfiable_classes_query(input, output, format, config).await,
        Commands::CheckEntailment {
            premise,
            conclusion,
            output,
            format,
        } => execute_entailment_check(premise, conclusion, output, format, config).await,
        Commands::PrintPrefixes { input, format } => {
            execute_print_prefixes(input, format, config).await
        }
        Commands::DumpClauses {
            input,
            output,
            format,
            pretty_print,
        } => execute_dump_clauses(input, output, format, pretty_print, config).await,
        Commands::Realization {
            input,
            output,
            format,
        } => execute_realization(input, output, format, config).await,
        Commands::Query {
            input,
            query,
            namespace,
            output,
            format,
        } => {
            let input_str = input
                .to_str()
                .ok_or_else(|| oxidowl::Error::io("Invalid input path encoding".to_string()))?;

            let output_str = output
                .as_ref()
                .map(|p| {
                    p.to_str().ok_or_else(|| {
                        oxidowl::Error::io("Invalid output path encoding".to_string())
                    })
                })
                .transpose()?;

            execute_dl_query(
                input_str,
                &query,
                namespace.as_deref(),
                output_str,
                "json", // Always use JSON for now
                config,
            )
            .await
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
        Commands::SparqlFile {
            sparql,
            input,
            output,
        } => execute_sparql_file(sparql, input, output, config).await, /*
                                                                       /*
                                                                       Commands::SparqlServer { port, bind } => {
                                                                           execute_sparql_server(port, bind, config).await
                                                                       }
                                                                       */
                                                                       */
    }
}

async fn execute_consistency_check(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    class_iri: Option<String>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Performing consistency check on {} file(s)", input.len());

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let result = if let Some(class_iri) = class_iri {
        info!("Checking satisfiability of class: {}", class_iri);
        reasoner.is_class_satisfiable(&class_iri)?
    } else {
        info!("Checking overall ontology consistency");
        reasoner.is_consistent()?
    };

    info!("Consistency check result: {}", result);

    if let Some(output_path) = output {
        let result_text = if result { "consistent" } else { "inconsistent" };
        fs::write(output_path, result_text)?;
    } else {
        println!(
            "Result: {}",
            if result { "consistent" } else { "inconsistent" }
        );
    }

    Ok(())
}

async fn execute_classification(
    input: Vec<PathBuf>,
    namespace: Option<String>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    pretty_print: bool,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Performing classification on {} file(s)", input.len());

    if let Some(ref ns) = namespace {
        info!("Using default namespace: {}", ns);
    }

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let hierarchy = reasoner.classify()?;

    info!("Classification completed");

    if let Some(output_path) = output {
        if pretty_print {
            hierarchy.save_to_file_pretty_print(output_path)?;
        } else {
            hierarchy.save_to_file(output_path)?;
        }
    } else {
        println!("Classification completed. Use -o to save results.");
        if pretty_print {
            println!("\nClass Hierarchy:");
            print_hierarchy_pretty(&hierarchy);
        }
    }

    Ok(())
}

async fn execute_satisfiability_check(
    input: Vec<PathBuf>,
    class_iri: String,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Checking satisfiability of class: {}", class_iri);

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let is_satisfiable = reasoner.is_class_satisfiable(&class_iri)?;

    info!("Satisfiability result: {}", is_satisfiable);

    if let Some(output_path) = output {
        let result = if is_satisfiable {
            "satisfiable"
        } else {
            "unsatisfiable"
        };
        fs::write(output_path, result)?;
    } else {
        println!(
            "Result: {}",
            if is_satisfiable {
                "satisfiable"
            } else {
                "unsatisfiable"
            }
        );
    }

    Ok(())
}

async fn execute_dl_query(
    ontology_file: &str,
    query: &str,
    namespace: Option<&str>,
    output_file: Option<&str>,
    format: &str,
    config: ReasonerConfig,
) -> Result<()> {
    // Create reasoner and load ontology
    let mut reasoner = Reasoner::new(config.clone())?;
    reasoner.load_ontology_from_file(ontology_file, OntologyFormat::Auto)?;

    // Get the ontology from the reasoner
    let ontology = reasoner.get_ontology()?;

    // Create reasoning service and query engine
    let ontology_clone = ontology
        .read()
        .map_err(|_| oxidowl::Error::io("Failed to acquire ontology read lock".to_string()))?
        .clone();
    let reasoning_service = oxidowl::reasoning::ReasoningService::new(ontology_clone, config);

    // Create query engine with optional namespace
    let query_engine = if let Some(ns) = namespace {
        oxidowl::query::DLQueryEngine::new_with_namespace(reasoning_service, ns.to_string())
    } else {
        // Try to auto-detect namespace from ontology IRI, fallback to default
        let ontology_guard = ontology
            .read()
            .map_err(|_| oxidowl::Error::io("Failed to acquire ontology read lock".to_string()))?;
        let default_namespace = ontology_guard
            .get_iri()
            .map(|iri| {
                let iri_str = iri.as_str();
                if iri_str.ends_with('#') {
                    iri_str.to_string()
                } else if iri_str.ends_with('/') {
                    iri_str.to_string()
                } else {
                    format!("{iri_str}#")
                }
            })
            .unwrap_or_else(|| "http://example.org/ontology#".to_string());

        oxidowl::query::DLQueryEngine::new_with_namespace(reasoning_service, default_namespace)
    };

    // Execute the query
    let result = query_engine.execute_query(query).await?;

    // Format and output the result
    let output = match format {
        "json" => {
            // Extract readable class names for JSON output
            let classes_vec: Vec<String> = if let Some(ref classes) = result.classes {
                classes
                    .iter()
                    .map(extract_class_names_from_expression)
                    .collect()
            } else {
                Vec::new()
            };

            if result.classes.is_some() {
                format!(
                    "{{\"query\": \"{}\", \"classes\": {:?}, \"execution_time\": \"{:?}\"}}",
                    query, classes_vec, result.execution_time
                )
            } else {
                format!(
                    "{{\"query\": \"{}\", \"result\": \"No results\", \"execution_time\": \"{:?}\"}}",
                    query, result.execution_time
                )
            }
        }
        "text" => format!("{result}"), // Use Display format instead of Debug
        _ => {
            return Err(oxidowl::Error::io(
                "Unsupported format. Use 'json' or 'text'".to_string(),
            ));
        }
    };

    // Write to file or stdout
    if let Some(file_path) = output_file {
        std::fs::write(file_path, output)?;
        println!("Query result saved to {file_path}");
    } else {
        println!("{output}");
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
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

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
        println!("{response}");
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
        println!("{results}");
    }

    Ok(())
}

async fn execute_object_property_classification(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    pretty_print: bool,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!(
        "Performing object property classification on {} file(s)",
        input.len()
    );

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let property_hierarchy = reasoner.classify_object_properties()?;

    info!("Object property classification completed");

    if let Some(output_path) = output {
        if pretty_print {
            property_hierarchy.save_to_file_pretty_print(output_path)?;
        } else {
            property_hierarchy.save_to_file(output_path)?;
        }
    } else {
        println!("Object property classification completed. Use -o to save results.");
        if pretty_print {
            println!("\nObject Property Hierarchy:");
            print_property_hierarchy_pretty(&property_hierarchy);
        }
    }

    Ok(())
}

async fn execute_data_property_classification(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    pretty_print: bool,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!(
        "Performing data property classification on {} file(s)",
        input.len()
    );

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let property_hierarchy = reasoner.classify_data_properties()?;

    info!("Data property classification completed");

    if let Some(output_path) = output {
        if pretty_print {
            property_hierarchy.save_to_file_pretty_print(output_path)?;
        } else {
            property_hierarchy.save_to_file(output_path)?;
        }
    } else {
        println!("Data property classification completed. Use -o to save results.");
        if pretty_print {
            println!("\nData Property Hierarchy:");
            print_property_hierarchy_pretty(&property_hierarchy);
        }
    }

    Ok(())
}

async fn execute_subclasses_query(
    input: Vec<PathBuf>,
    class_iri: String,
    direct: bool,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!(
        "Getting {}subclasses of: {}",
        if direct { "direct " } else { "" },
        class_iri
    );

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let class_expr = oxidowl::ontology::ClassExpression::Class(oxidowl::ontology::Class {
        iri: oxidowl::ontology::IRI::new(&class_iri).to_url()?.into(),
    });

    let subclasses = reasoner.get_subclasses(&class_expr, direct)?;

    info!("Found {} subclasses", subclasses.len());

    let result = format_class_list(&subclasses);

    if let Some(output_path) = output {
        fs::write(output_path, result)?;
    } else {
        println!("Subclasses of {class_iri}:");
        for class in &subclasses {
            println!("  {}", format_class_expression(class));
        }
    }

    Ok(())
}

async fn execute_superclasses_query(
    input: Vec<PathBuf>,
    class_iri: String,
    direct: bool,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!(
        "Getting {}superclasses of: {}",
        if direct { "direct " } else { "" },
        class_iri
    );

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let class_expr = oxidowl::ontology::ClassExpression::Class(oxidowl::ontology::Class {
        iri: oxidowl::ontology::IRI::new(&class_iri).to_url()?.into(),
    });

    let superclasses = reasoner.get_superclasses(&class_expr, direct)?;

    info!("Found {} superclasses", superclasses.len());

    let result = format_class_list(&superclasses);

    if let Some(output_path) = output {
        fs::write(output_path, result)?;
    } else {
        println!("Superclasses of {class_iri}:");
        for class in &superclasses {
            println!("  {}", format_class_expression(class));
        }
    }

    Ok(())
}

async fn execute_equivalent_classes_query(
    input: Vec<PathBuf>,
    class_iri: String,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Getting equivalent classes of: {}", class_iri);

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let class_expr = oxidowl::ontology::ClassExpression::Class(oxidowl::ontology::Class {
        iri: oxidowl::ontology::IRI::new(&class_iri).to_url()?.into(),
    });

    let equivalent_classes = reasoner.get_equivalent_classes(&class_expr)?;

    info!("Found {} equivalent classes", equivalent_classes.len());

    let result = format_class_list(&equivalent_classes);

    if let Some(output_path) = output {
        fs::write(output_path, result)?;
    } else {
        println!("Equivalent classes of {class_iri}:");
        for class in &equivalent_classes {
            println!("  {}", format_class_expression(class));
        }
    }

    Ok(())
}

async fn execute_unsatisfiable_classes_query(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Finding unsatisfiable classes");

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let unsatisfiable_classes = reasoner.get_unsatisfiable_classes()?;

    info!(
        "Found {} unsatisfiable classes",
        unsatisfiable_classes.len()
    );

    let result = format_class_list(&unsatisfiable_classes);

    if let Some(output_path) = output {
        fs::write(output_path, result)?;
    } else {
        println!("Unsatisfiable classes:");
        for class in &unsatisfiable_classes {
            println!("  {}", format_class_expression(class));
        }
    }

    Ok(())
}

async fn execute_entailment_check(
    premise: PathBuf,
    conclusion: PathBuf,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    info!(
        "Checking entailment: {} |= {}",
        premise.display(),
        conclusion.display()
    );

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    let entails = reasoner.check_entailment(&premise, &conclusion, ontology_format)?;

    info!("Entailment result: {}", entails);

    let result = if entails {
        "entails"
    } else {
        "does not entail"
    };

    if let Some(output_path) = output {
        fs::write(output_path, result)?;
    } else {
        println!(
            "Result: {} {} {}",
            premise.display(),
            result,
            conclusion.display()
        );
    }

    Ok(())
}

async fn execute_print_prefixes(
    input: Vec<PathBuf>,
    format: Option<InputFormat>,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Printing available prefixes");

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    let prefixes = reasoner.get_prefixes()?;

    println!("Available prefixes:");
    for (prefix, iri) in &prefixes {
        println!("  {prefix} = {iri}");
    }

    Ok(())
}

async fn execute_dump_clauses(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    format: Option<InputFormat>,
    pretty_print: bool,
    config: ReasonerConfig,
) -> Result<()> {
    if input.is_empty() {
        return Err(oxidowl::Error::io("No input files provided".to_string()));
    }

    info!("Dumping DL clauses");

    let mut reasoner = Reasoner::new(config)?;
    let ontology_format = format.map_or(OntologyFormat::Auto, Into::into);

    // Load multiple ontologies if provided
    for file in &input {
        info!("Loading ontology: {}", file.display());
        reasoner.load_ontology_from_file(file, ontology_format)?;
    }

    // Generate DL clauses
    let clause_set = reasoner.dump_dl_clauses()?;

    // Output clauses
    if let Some(output_file) = output {
        clause_set.save_to_file(&output_file)?;
        info!("DL clauses saved to: {}", output_file.display());
    } else {
        // Print to stdout
        let output_str = if pretty_print {
            clause_set.to_hermit_format()
        } else {
            clause_set.to_hermit_format()
        };
        println!("{output_str}");
    }

    println!("DL clause generation completed:");
    println!(
        "  Deterministic clauses: {}",
        clause_set.statistics.deterministic_clause_count
    );
    println!(
        "  Disjunctive clauses: {}",
        clause_set.statistics.disjunctive_clause_count
    );
    println!(
        "  ABox facts: {}",
        clause_set.statistics.positive_fact_count + clause_set.statistics.negative_fact_count
    );

    Ok(())
}

// Helper functions for formatting output

fn print_hierarchy_pretty(hierarchy: &oxidowl::core::reasoner::ClassificationResult) {
    // Simple pretty printing - in a full implementation this would be more sophisticated
    for (class, superclasses) in &hierarchy.hierarchy {
        println!("{}", format_class_expression(class));
        for superclass in superclasses {
            println!("  ⊑ {}", format_class_expression(superclass));
        }
    }
}

fn print_property_hierarchy_pretty(
    hierarchy: &oxidowl::core::reasoner::PropertyClassificationResult,
) {
    // Placeholder for property hierarchy printing
    println!("Property hierarchy (detailed implementation needed)");
}

fn format_class_expression(expr: &oxidowl::ontology::ClassExpression) -> String {
    match expr {
        oxidowl::ontology::ClassExpression::Class(class) => {
            let iri_str = class.iri.to_string();
            if let Some(name) = iri_str.split('#').next_back() {
                name.to_string()
            } else if let Some(name) = iri_str.split('/').next_back() {
                name.to_string()
            } else {
                iri_str
            }
        }
        _ => format!("{expr:?}"),
    }
}

fn format_class_list(classes: &[oxidowl::ontology::ClassExpression]) -> String {
    classes
        .iter()
        .map(format_class_expression)
        .collect::<Vec<_>>()
        .join("\n")
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
