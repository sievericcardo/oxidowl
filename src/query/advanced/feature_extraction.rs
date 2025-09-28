//! ML-driven query feature extraction for performance prediction
//!
//! This module implements sophisticated feature extractors that analyze
//! Description Logic queries to predict their execution characteristics.

use super::phase2_optimization::{QueryFeatureExtractor, QueryPerformanceDataPoint};
use super::conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
use crate::ontology::{ClassExpression, ObjectPropertyExpression};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Comprehensive feature extractor for DL queries
#[derive(Debug)]
pub struct DLQueryFeatureExtractor {
    /// Feature cache for performance
    feature_cache: HashMap<u64, Vec<f64>>,
    
    /// Configuration for feature extraction
    config: FeatureExtractionConfig,
}

/// Configuration for feature extraction
#[derive(Debug, Clone)]
pub struct FeatureExtractionConfig {
    /// Maximum depth for concept expression analysis
    pub max_concept_depth: u32,
    
    /// Include statistical features
    pub include_statistics: bool,
    
    /// Include structural features
    pub include_structure: bool,
    
    /// Include semantic features
    pub include_semantics: bool,
    
    /// Cache feature vectors
    pub enable_caching: bool,
}

impl Default for FeatureExtractionConfig {
    fn default() -> Self {
        Self {
            max_concept_depth: 10,
            include_statistics: true,
            include_structure: true,
            include_semantics: true,
            enable_caching: true,
        }
    }
}

impl DLQueryFeatureExtractor {
    /// Create a new DL query feature extractor
    pub fn new() -> Self {
        Self {
            feature_cache: HashMap::new(),
            config: FeatureExtractionConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: FeatureExtractionConfig) -> Self {
        Self {
            feature_cache: HashMap::new(),
            config,
        }
    }
    
    /// Calculate query complexity hash for caching
    fn query_hash(&self, query: &ConjunctiveQuery) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.answer_variables.len().hash(&mut hasher);
        query.body_atoms.len().hash(&mut hasher);
        
        // Hash key structural elements
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { class_expression, .. } => {
                    self.hash_class_expression(class_expression, &mut hasher);
                }
                QueryAtom::ObjectPropertyAtom { property, .. } => {
                    self.hash_property_expression(property, &mut hasher);
                }
                _ => {
                    std::mem::discriminant(atom).hash(&mut hasher);
                }
            }
        }
        
        hasher.finish()
    }
    
    /// Hash class expression structure
    fn hash_class_expression(&self, expr: &ClassExpression, hasher: &mut impl Hasher) {
        match expr {
            ClassExpression::Class(class) => {
                "Class".hash(hasher);
                class.iri.to_string().hash(hasher);
            }
            ClassExpression::ObjectIntersectionOf(exprs) => {
                "ObjectIntersectionOf".hash(hasher);
                exprs.len().hash(hasher);
                for expr in exprs {
                    self.hash_class_expression(expr, hasher);
                }
            }
            ClassExpression::ObjectUnionOf(exprs) => {
                "ObjectUnionOf".hash(hasher);
                exprs.len().hash(hasher);
                for expr in exprs {
                    self.hash_class_expression(expr, hasher);
                }
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                "ObjectSomeValuesFrom".hash(hasher);
                self.hash_property_expression(property, hasher);
                self.hash_class_expression(filler, hasher);
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                "ObjectAllValuesFrom".hash(hasher);
                self.hash_property_expression(property, hasher);
                self.hash_class_expression(filler, hasher);
            }
            _ => {
                std::mem::discriminant(expr).hash(hasher);
            }
        }
    }
    
    /// Hash property expression structure
    fn hash_property_expression(&self, expr: &ObjectPropertyExpression, hasher: &mut impl Hasher) {
        match expr {
            ObjectPropertyExpression::ObjectProperty(prop) => {
                prop.iri.to_string().hash(hasher);
            }
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                "InverseObjectProperty".hash(hasher);
                prop.iri.to_string().hash(hasher);
            }
            ObjectPropertyExpression::PropertyChain(props) => {
                "PropertyChain".hash(hasher);
                props.len().hash(hasher);
                for prop in props {
                    self.hash_property_expression(prop, hasher);
                }
            }
        }
    }
    
    /// Extract structural features from the query
    fn extract_structural_features(&self, query: &ConjunctiveQuery) -> Vec<f64> {
        let mut features = Vec::new();
        
        // Basic structural features
        features.push(query.body_atoms.len() as f64); // Number of atoms
        features.push(query.answer_variables.len() as f64); // Number of head variables
        
        // Variable analysis
        let all_variables = self.collect_all_variables(query);
        features.push(all_variables.len() as f64); // Total variables
        
        let shared_variables = self.count_shared_variables(query);
        features.push(shared_variables as f64); // Variables appearing in multiple atoms
        
        // Atom type distribution
        let (class_atoms, prop_atoms, other_atoms) = self.count_atom_types(query);
        features.push(class_atoms as f64);
        features.push(prop_atoms as f64);
        features.push(other_atoms as f64);
        
        // Join structure analysis
        let join_complexity = self.calculate_join_complexity(query);
        features.push(join_complexity);
        
        // Query tree depth (for nested expressions)
        let max_depth = self.calculate_max_expression_depth(query);
        features.push(max_depth as f64);
        
        features
    }
    
    /// Extract semantic features from query concepts
    fn extract_semantic_features(&self, query: &ConjunctiveQuery) -> Vec<f64> {
        let mut features = Vec::new();
        
        // Concept complexity analysis
        let (simple_concepts, complex_concepts) = self.analyze_concept_complexity(query);
        features.push(simple_concepts as f64);
        features.push(complex_concepts as f64);
        
        // Restriction analysis
        let (existential_restrictions, universal_restrictions) = self.count_restrictions(query);
        features.push(existential_restrictions as f64);
        features.push(universal_restrictions as f64);
        
        // Boolean operator usage
        let (intersections, unions, complements) = self.count_boolean_operators(query);
        features.push(intersections as f64);
        features.push(unions as f64);
        features.push(complements as f64);
        
        // Cardinality features
        let cardinality_restrictions = self.count_cardinality_restrictions(query);
        features.push(cardinality_restrictions as f64);
        
        features
    }
    
    /// Extract statistical features (would require ontology statistics)
    fn extract_statistical_features(&self, query: &ConjunctiveQuery) -> Vec<f64> {
        let mut features = Vec::new();
        
        // Placeholder for statistical features that would be computed from ontology
        // These would include:
        // - Average class instance counts
        // - Property selectivity estimates
        // - Hierarchy depth statistics
        // - Domain/range complexity
        
        // For now, use heuristic estimates
        features.push(self.estimate_selectivity(query));
        features.push(self.estimate_result_size(query));
        features.push(self.estimate_join_cost(query));
        
        features
    }
    
    // Helper methods for feature extraction
    
    fn collect_all_variables(&self, query: &ConjunctiveQuery) -> HashSet<QueryVariable> {
        let mut variables = HashSet::new();
        
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
                QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                    variables.insert(subject.clone());
                    variables.insert(object.clone());
                }
                QueryAtom::DataPropertyAtom { subject, literal, .. } => {
                    variables.insert(subject.clone());
                    variables.insert(literal.clone());
                }
                QueryAtom::SameIndividualAtom { left, right } |
                QueryAtom::DifferentIndividualsAtom { left, right } => {
                    variables.insert(left.clone());
                    variables.insert(right.clone());
                }
                QueryAtom::ConcreteIndividualAtom { variable, .. } |
                QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                    variables.insert(variable.clone());
                }
            }
        }
        
        variables
    }
    
    fn count_shared_variables(&self, query: &ConjunctiveQuery) -> usize {
        let mut variable_counts = HashMap::new();
        
        for atom in &query.body_atoms {
            let atom_vars = match atom {
                QueryAtom::ClassAtom { variable, .. } => vec![variable.clone()],
                QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                    vec![subject.clone(), object.clone()]
                }
                QueryAtom::DataPropertyAtom { subject, literal, .. } => {
                    vec![subject.clone(), literal.clone()]
                }
                QueryAtom::SameIndividualAtom { left, right } |
                QueryAtom::DifferentIndividualsAtom { left, right } => {
                    vec![left.clone(), right.clone()]
                }
                QueryAtom::ConcreteIndividualAtom { variable, .. } |
                QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                    vec![variable.clone()]
                }
            };
            
            for var in atom_vars {
                *variable_counts.entry(var).or_insert(0) += 1;
            }
        }
        
        variable_counts.values().filter(|&&count| count > 1).count()
    }
    
    fn count_atom_types(&self, query: &ConjunctiveQuery) -> (usize, usize, usize) {
        let mut class_atoms = 0;
        let mut prop_atoms = 0;
        let mut other_atoms = 0;
        
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { .. } => class_atoms += 1,
                QueryAtom::ObjectPropertyAtom { .. } | QueryAtom::DataPropertyAtom { .. } => prop_atoms += 1,
                _ => other_atoms += 1,
            }
        }
        
        (class_atoms, prop_atoms, other_atoms)
    }
    
    fn calculate_join_complexity(&self, query: &ConjunctiveQuery) -> f64 {
        let num_atoms = query.body_atoms.len();
        if num_atoms <= 1 {
            return 0.0;
        }
        
        // Simplified join complexity based on number of joins
        let potential_joins = (num_atoms * (num_atoms - 1)) / 2;
        let shared_vars = self.count_shared_variables(query);
        
        if shared_vars > 0 {
            potential_joins as f64 / shared_vars as f64
        } else {
            potential_joins as f64 // Cartesian product complexity
        }
    }
    
    fn calculate_max_expression_depth(&self, query: &ConjunctiveQuery) -> u32 {
        query.body_atoms
            .iter()
            .map(|atom| match atom {
                QueryAtom::ClassAtom { class_expression, .. } => {
                    self.class_expression_depth(class_expression, 0)
                }
                _ => 1,
            })
            .max()
            .unwrap_or(0)
    }
    
    fn class_expression_depth(&self, expr: &ClassExpression, current_depth: u32) -> u32 {
        if current_depth >= self.config.max_concept_depth {
            return current_depth;
        }
        
        match expr {
            ClassExpression::Class(_) => current_depth + 1,
            ClassExpression::ObjectIntersectionOf(exprs) | 
            ClassExpression::ObjectUnionOf(exprs) => {
                exprs.iter()
                    .map(|e| self.class_expression_depth(e, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } |
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                self.class_expression_depth(filler, current_depth + 1)
            }
            ClassExpression::ObjectComplementOf(expr) => {
                self.class_expression_depth(expr, current_depth + 1)
            }
            _ => current_depth + 1,
        }
    }
    
    fn analyze_concept_complexity(&self, query: &ConjunctiveQuery) -> (usize, usize) {
        let mut simple_concepts = 0;
        let mut complex_concepts = 0;
        
        for atom in &query.body_atoms {
            if let QueryAtom::ClassAtom { class_expression, .. } = atom {
                if self.is_simple_concept(class_expression) {
                    simple_concepts += 1;
                } else {
                    complex_concepts += 1;
                }
            }
        }
        
        (simple_concepts, complex_concepts)
    }
    
    fn is_simple_concept(&self, expr: &ClassExpression) -> bool {
        matches!(expr, ClassExpression::Class(_))
    }
    
    fn count_restrictions(&self, query: &ConjunctiveQuery) -> (usize, usize) {
        let mut existential = 0;
        let mut universal = 0;
        
        for atom in &query.body_atoms {
            if let QueryAtom::ClassAtom { class_expression, .. } = atom {
                self.count_restrictions_recursive(class_expression, &mut existential, &mut universal);
            }
        }
        
        (existential, universal)
    }
    
    fn count_restrictions_recursive(&self, expr: &ClassExpression, existential: &mut usize, universal: &mut usize) {
        match expr {
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => {
                *existential += 1;
                self.count_restrictions_recursive(filler, existential, universal);
            }
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                *universal += 1;
                self.count_restrictions_recursive(filler, existential, universal);
            }
            ClassExpression::ObjectIntersectionOf(exprs) | 
            ClassExpression::ObjectUnionOf(exprs) => {
                for expr in exprs {
                    self.count_restrictions_recursive(expr, existential, universal);
                }
            }
            ClassExpression::ObjectComplementOf(expr) => {
                self.count_restrictions_recursive(expr, existential, universal);
            }
            _ => {}
        }
    }
    
    fn count_boolean_operators(&self, query: &ConjunctiveQuery) -> (usize, usize, usize) {
        let mut intersections = 0;
        let mut unions = 0;
        let mut complements = 0;
        
        for atom in &query.body_atoms {
            if let QueryAtom::ClassAtom { class_expression, .. } = atom {
                self.count_boolean_operators_recursive(class_expression, &mut intersections, &mut unions, &mut complements);
            }
        }
        
        (intersections, unions, complements)
    }
    
    fn count_boolean_operators_recursive(
        &self, 
        expr: &ClassExpression, 
        intersections: &mut usize, 
        unions: &mut usize, 
        complements: &mut usize
    ) {
        match expr {
            ClassExpression::ObjectIntersectionOf(exprs) => {
                *intersections += 1;
                for expr in exprs {
                    self.count_boolean_operators_recursive(expr, intersections, unions, complements);
                }
            }
            ClassExpression::ObjectUnionOf(exprs) => {
                *unions += 1;
                for expr in exprs {
                    self.count_boolean_operators_recursive(expr, intersections, unions, complements);
                }
            }
            ClassExpression::ObjectComplementOf(expr) => {
                *complements += 1;
                self.count_boolean_operators_recursive(expr, intersections, unions, complements);
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } |
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                self.count_boolean_operators_recursive(filler, intersections, unions, complements);
            }
            _ => {}
        }
    }
    
    fn count_cardinality_restrictions(&self, query: &ConjunctiveQuery) -> usize {
        // Placeholder - would count ObjectMinCardinality, ObjectMaxCardinality, etc.
        // These would be added to the ClassExpression enum if not already present
        0
    }
    
    // Statistical estimation methods (heuristic-based)
    
    fn estimate_selectivity(&self, query: &ConjunctiveQuery) -> f64 {
        // Heuristic selectivity estimation
        let base_selectivity = 0.1;
        let atom_count = query.body_atoms.len() as f64;
        
        // More atoms generally mean lower selectivity
        base_selectivity / (1.0 + atom_count * 0.1)
    }
    
    fn estimate_result_size(&self, query: &ConjunctiveQuery) -> f64 {
        // Heuristic result size estimation
        let base_size = 100.0;
        let selectivity = self.estimate_selectivity(query);
        
        base_size * selectivity
    }
    
    fn estimate_join_cost(&self, query: &ConjunctiveQuery) -> f64 {
        // Heuristic join cost estimation
        let join_complexity = self.calculate_join_complexity(query);
        let atom_count = query.body_atoms.len() as f64;
        
        join_complexity * atom_count.powi(2)
    }
}

impl QueryFeatureExtractor for DLQueryFeatureExtractor {
    fn extract_features(&self, query: &ConjunctiveQuery) -> Vec<f64> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let query_hash = self.query_hash(query);
            if let Some(cached_features) = self.feature_cache.get(&query_hash) {
                return cached_features.clone();
            }
        }
        
        let mut features = Vec::new();
        
        // Extract different types of features based on configuration
        if self.config.include_structure {
            features.extend(self.extract_structural_features(query));
        }
        
        if self.config.include_semantics {
            features.extend(self.extract_semantic_features(query));
        }
        
        if self.config.include_statistics {
            features.extend(self.extract_statistical_features(query));
        }
        
        // Cache the result if caching is enabled
        if self.config.enable_caching {
            let query_hash = self.query_hash(query);
            // Note: In a real implementation, we'd need mutable access to cache
            // This would require interior mutability (RefCell/Mutex)
        }
        
        features
    }
    
    fn feature_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        
        if self.config.include_structure {
            names.extend([
                "num_atoms",
                "num_head_variables", 
                "total_variables",
                "shared_variables",
                "class_atoms",
                "property_atoms", 
                "other_atoms",
                "join_complexity",
                "max_expression_depth",
            ].iter().map(|s| s.to_string()));
        }
        
        if self.config.include_semantics {
            names.extend([
                "simple_concepts",
                "complex_concepts",
                "existential_restrictions",
                "universal_restrictions", 
                "intersections",
                "unions",
                "complements",
                "cardinality_restrictions",
            ].iter().map(|s| s.to_string()));
        }
        
        if self.config.include_statistics {
            names.extend([
                "estimated_selectivity",
                "estimated_result_size",
                "estimated_join_cost",
            ].iter().map(|s| s.to_string()));
        }
        
        names
    }
}

impl Default for DLQueryFeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};
    
    #[test]
    fn test_feature_extraction() {
        let extractor = DLQueryFeatureExtractor::new();
        let feature_names = extractor.feature_names();
        
        // Verify that we have the expected number of features
        assert!(!feature_names.is_empty());
        assert!(feature_names.contains(&"num_atoms".to_string()));
        assert!(feature_names.contains(&"join_complexity".to_string()));
    }
    
    #[test]
    fn test_query_hash() {
        let extractor = DLQueryFeatureExtractor::new();
        
        // Create a simple query for testing
        let query = ConjunctiveQuery {
            answer_variables: vec![],
            body_atoms: vec![
                QueryAtom::ClassAtom {
                    class_expression: ClassExpression::Class(
                        Class::new(IRI::new("http://example.org/Person"))
                    ),
                    variable: QueryVariable { 
                        name: "x".to_string(), 
                        var_type: crate::query::advanced::conjunctive::VariableType::Individual 
                    },
                }
            ],
            constraints: crate::query::advanced::conjunctive::QueryConstraints::default(),
            metadata: crate::query::advanced::conjunctive::QueryMetadata::default(),
        };
        
        let hash1 = extractor.query_hash(&query);
        let hash2 = extractor.query_hash(&query);
        
        // Same query should produce same hash
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_concept_complexity_analysis() {
        let extractor = DLQueryFeatureExtractor::new();
        
        // Test simple concept
        let simple_concept = ClassExpression::Class(
            Class::new(IRI::new("http://example.org/Person"))
        );
        assert!(extractor.is_simple_concept(&simple_concept));
        
        // Test depth calculation
        let depth = extractor.class_expression_depth(&simple_concept, 0);
        assert_eq!(depth, 1);
    }
}