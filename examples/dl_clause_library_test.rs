use oxidowl::Result;
use oxidowl::config::ReasonerConfig;
use oxidowl::core::reasoner::Reasoner;
use oxidowl::ontology::OntologyFormat;
use std::path::Path;

fn main() -> Result<()> {
    // Create a reasoner
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;

    // Load an ontology
    let ontology_path = Path::new("greenhouse.owx");
    if ontology_path.exists() {
        reasoner.load_ontology_from_file(ontology_path, OntologyFormat::OwlXml)?;

        // Get DL clauses as a string
        let dl_clauses_output = reasoner.get_dl_clauses_string()?;
        println!("DL Clauses generated programmatically:");
        println!("{}", dl_clauses_output);

        // Save to file
        reasoner.save_dl_clauses("library_test_dl_clauses.txt")?;
        println!("\nDL clauses also saved to library_test_dl_clauses.txt");
    } else {
        println!("greenhouse.owx not found, skipping test");
    }

    Ok(())
}
