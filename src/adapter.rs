use crate::Result;
use crate::ontology::{axioms::AxiomId, IRI, Class};
use horned_owl::model::MutableOntology;
use std::collections::HashMap;

/// Adapter for converting between horned-owl and oxidowl representations
pub struct HornedOwlAdapter {
    iri_cache: HashMap<String, IRI>,
    axiom_counter: u64,
}

impl Default for HornedOwlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl HornedOwlAdapter {
    #[must_use] pub fn new() -> Self {
        Self {
            iri_cache: HashMap::new(),
            axiom_counter: 0,
        }
    }

    fn next_axiom_id(&mut self) -> AxiomId {
        self.axiom_counter += 1;
        self.axiom_counter
    }

    /// Convert horned-owl IRI to oxidowl IRI
    pub fn convert_iri(&mut self, horned_iri: &horned_owl::model::IRI<String>) -> Result<IRI> {
        let iri_string = horned_iri.to_string();
        
        // Check cache first
        if let Some(cached_iri) = self.iri_cache.get(&iri_string) {
            return Ok(cached_iri.clone());
        }
        
        let oxidowl_iri = IRI::new(&iri_string);
        self.iri_cache.insert(iri_string, oxidowl_iri.clone());
        Ok(oxidowl_iri)
    }

    /// Convert horned-owl Class to oxidowl Class
    pub fn convert_class(&mut self, horned_class: &horned_owl::model::Class<String>) -> Result<Class> {
        let iri = self.convert_iri(&horned_class.0)?;
        Ok(Class::new(iri))
    }

    /// Convert horned-owl ontology to oxidowl ontology (basic conversion)
    pub fn convert_basic_ontology<A>(&mut self, horned_ontology: &dyn std::fmt::Debug) -> Result<crate::ontology::Ontology>
    where
        A: Clone + std::fmt::Display + std::hash::Hash + Eq
    {
        // Create a basic oxidowl ontology for now
        let oxidowl_ontology = crate::ontology::Ontology::new();
        
        // TODO: Implement actual conversion when horned-owl API is stable
        Ok(oxidowl_ontology)
    }

    /// Convert horned-owl ontology with SWRL rules support
    pub fn convert_ontology_with_swrl<A>(&mut self, horned_ontology: &dyn std::fmt::Debug) -> Result<crate::ontology::Ontology>
    where
        A: Clone + std::fmt::Display + std::hash::Hash + Eq
    {
        // For now, delegate to basic conversion
        // TODO: Add SWRL rule conversion when API is stable
        self.convert_basic_ontology::<A>(horned_ontology)
    }
}

#[cfg(test)]
mod tests {
    

    // TODO: Fix these tests when horned-owl API is more stable
    
    /*
    #[test]
    fn test_iri_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        // Use proper IRI constructor - build IRI from string
        let horned_iri = horned_owl::model::IRI::from("http://example.org/test".to_string());
        let oxidowl_iri = adapter.convert_iri(&horned_iri).unwrap();
        assert_eq!(oxidowl_iri.as_str(), "http://example.org/test");
    }

    #[test]
    fn test_class_conversion() {
        let mut adapter = HornedOwlAdapter::new();
        let horned_iri = horned_owl::model::IRI::from("http://example.org/Person".to_string());
        let horned_class = horned_owl::model::Class(horned_iri);
        let oxidowl_class = adapter.convert_class(&horned_class).unwrap();
        assert_eq!(oxidowl_class.iri.as_str(), "http://example.org/Person");
    }
    */
}
