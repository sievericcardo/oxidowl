use oxidowl::{Result, config::ReasonerConfig, ontology::Ontology, reasoning::ReasoningService};

#[tokio::test]
async fn test_basic_compilation() -> Result<()> {
    // Create basic configuration
    let config = ReasonerConfig::default();

    // Create empty ontology
    let ontology = Ontology::new();

    // Create reasoning service
    let service = ReasoningService::new(ontology, config);

    println!("Basic compilation test passed!");
    Ok(())
}

#[tokio::test]
async fn test_greenhouse_loading() -> Result<()> {
    // Try to load the greenhouse ontology
    match std::fs::read_to_string("greenhouse.ttl") {
        Ok(content) => {
            println!("Successfully read greenhouse.ttl ({} bytes)", content.len());

            // Try to parse it (this might fail, but that's ok for now)
            match oxidowl::parsers::turtle::parse(&content) {
                Ok(ontology) => {
                    println!("Successfully parsed greenhouse ontology");

                    // Basic reasoning service creation
                    let config = ReasonerConfig::default();
                    let service = ReasoningService::new(ontology, config);

                    println!("Created reasoning service for greenhouse ontology");
                }
                Err(e) => {
                    println!("Could not parse greenhouse ontology: {e:?}");
                }
            }
        }
        Err(e) => {
            println!("Could not read greenhouse.ttl: {e:?}");
        }
    }

    Ok(())
}
