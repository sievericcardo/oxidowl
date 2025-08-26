use oxidowl::ontology::Ontology;
use oxidowl::validation::owl2_dl::OWL2DLValidator;
use oxidowl::ontology::datatypes::DatatypeManager;
use oxidowl::parsers::manchester::ManchesterParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owl2_dl_validator() {
        let ontology = Ontology::new();
        let mut validator = OWL2DLValidator::new(ontology);
        
        // Empty ontology should validate successfully
        let result = validator.validate();
        assert!(result.is_ok(), "Empty ontology should validate: {:?}", result);
        
        println!("✓ OWL 2 DL Validator test passed");
    }

    #[test]
    fn test_datatype_manager() {
        let manager = DatatypeManager::new();
        
        // Should be able to create manager
        println!("✓ Datatype Manager creation test passed");
    }

    #[test]
    fn test_manchester_parser() {
        let config = oxidowl::parsers::manchester::ManchesterParserConfig::default();
        let parser = ManchesterParser::new(config);
        
        // Should be able to create parser
        println!("✓ Manchester Parser creation test passed");
    }
}
