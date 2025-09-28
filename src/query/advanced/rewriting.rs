//! Query rewriting for OWL 2 QL profile optimization
//! 
//! This module implements the standard OWL 2 QL query rewriting algorithm
//! that transforms conjunctive queries into unions of conjunctive queries
//! that can be answered using database-style query evaluation.

use crate::ontology::{Ontology, ClassExpression, ObjectPropertyExpression, axioms::Axiom};
use crate::profiles::ql::QLValidator;
use super::conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use lru::LruCache;

/// Query rewriter for OWL 2 QL profile
pub struct QueryRewriter {
    ontology: Arc<Ontology>,
    ql_validator: QLValidator,
    rewriting_cache: LruCache<ConjunctiveQuery, Vec<ConjunctiveQuery>>,
    tbox_index: TBoxIndex,
    max_rewriting_depth: usize,
}

/// Index of TBox axioms for efficient query rewriting
#[derive(Debug, Clone)]
struct TBoxIndex {
    /// Class inclusions: C ⊑ D
    class_inclusions: HashMap<ClassExpression, Vec<ClassExpression>>,
    /// Property inclusions: P ⊑ Q
    property_inclusions: HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,
    /// Existential restrictions: ∃P.C ⊑ D
    existential_restrictions: Vec<ExistentialRestriction>,
    /// Property domains: dom(P) ⊑ C
    property_domains: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,
    /// Property ranges: range(P) ⊑ C
    property_ranges: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,
}

/// Existential restriction for query rewriting
#[derive(Debug, Clone)]
struct ExistentialRestriction {
    property: ObjectPropertyExpression,
    filler: ClassExpression,
    superclass: ClassExpression,
}

/// Error types for query rewriting
#[derive(Debug, thiserror::Error)]
pub enum RewritingError {
    #[error("Query is not in OWL 2 QL profile: {0}")]
    NotQLProfile(String),
    #[error("Rewriting depth limit exceeded: {0}")]
    DepthLimitExceeded(usize),
    #[error("Invalid query structure: {0}")]
    InvalidQuery(String),
    #[error("Ontology error: {0}")]
    OntologyError(String),
}

impl QueryRewriter {
    /// Create a new query rewriter
    pub fn new(ontology: Arc<Ontology>) -> Result<Self, RewritingError> {
        let ql_validator = QLValidator::new();
        let tbox_index = Self::build_tbox_index(&ontology)?;
        
        Ok(Self {
            ontology,
            ql_validator,
            rewriting_cache: LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
            tbox_index,
            max_rewriting_depth: 50,
        })
    }

    /// Set maximum rewriting depth
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_rewriting_depth = depth;
    }

    /// Rewrite a conjunctive query for OWL 2 QL
    pub fn rewrite_for_ql(&mut self, query: &ConjunctiveQuery) -> Result<Vec<ConjunctiveQuery>, RewritingError> {
        // Check if query is already cached
        if let Some(cached) = self.rewriting_cache.get(query) {
            return Ok(cached.clone());
        }

        // Validate that the query is compatible with QL profile
        self.validate_ql_compatibility(query)?;

        // Perform query rewriting
        let rewritten_queries = self.perform_rewriting(query)?;
        
        // Cache the result
        self.rewriting_cache.put(query.clone(), rewritten_queries.clone());
        
        Ok(rewritten_queries)
    }

    /// Perform the actual query rewriting algorithm
    fn perform_rewriting(&self, query: &ConjunctiveQuery) -> Result<Vec<ConjunctiveQuery>, RewritingError> {
        let mut rewritten_queries = vec![query.clone()];
        let mut work_queue = VecDeque::new();
        work_queue.push_back((query.clone(), 0));
        let mut processed = HashSet::new();

        while let Some((current_query, depth)) = work_queue.pop_front() {
            if depth >= self.max_rewriting_depth {
                return Err(RewritingError::DepthLimitExceeded(depth));
            }

            // Skip if already processed
            let query_hash = self.compute_query_hash(&current_query);
            if processed.contains(&query_hash) {
                continue;
            }
            processed.insert(query_hash);

            // Try to expand each atom in the query
            for (atom_idx, atom) in current_query.body_atoms.iter().enumerate() {
                let expansions = self.expand_atom(atom, &current_query)?;
                
                for expansion in expansions {
                    let mut new_query = current_query.clone();
                    new_query.body_atoms[atom_idx] = expansion;
                    
                    // Add to results and work queue if not already present
                    let new_hash = self.compute_query_hash(&new_query);
                    if !processed.contains(&new_hash) {
                        rewritten_queries.push(new_query.clone());
                        work_queue.push_back((new_query, depth + 1));
                    }
                }
            }

            // Try to add new atoms based on TBox axioms
            let new_atoms = self.generate_new_atoms(&current_query)?;
            for new_atom in new_atoms {
                let mut new_query = current_query.clone();
                new_query.body_atoms.push(new_atom);
                
                let new_hash = self.compute_query_hash(&new_query);
                if !processed.contains(&new_hash) {
                    rewritten_queries.push(new_query.clone());
                    work_queue.push_back((new_query, depth + 1));
                }
            }
        }

        // Remove redundant queries
        Ok(self.remove_redundant_queries(rewritten_queries))
    }

    /// Expand a single query atom using TBox axioms
    fn expand_atom(&self, atom: &QueryAtom, _context: &ConjunctiveQuery) -> Result<Vec<QueryAtom>, RewritingError> {
        let mut expansions = Vec::new();

        match atom {
            QueryAtom::ClassAtom { variable, class_expression } => {
                // Look for class inclusions: if C ⊑ D and we have D(x), add C(x)
                if let Some(subclasses) = self.tbox_index.class_inclusions.get(class_expression) {
                    for subclass in subclasses {
                        expansions.push(QueryAtom::ClassAtom {
                            variable: variable.clone(),
                            class_expression: subclass.clone(),
                        });
                    }
                }

                // Look for existential restrictions
                for restriction in &self.tbox_index.existential_restrictions {
                    if restriction.superclass == *class_expression {
                        // Add P(x, y) and C(y) atoms
                        let fresh_var = self.generate_fresh_variable("y");
                        expansions.push(QueryAtom::ObjectPropertyAtom {
                            subject: variable.clone(),
                            property: restriction.property.clone(),
                            object: fresh_var.clone(),
                        });
                        // Note: This would typically require creating a new query with the additional atom
                    }
                }
            }

            QueryAtom::ObjectPropertyAtom { subject, property, object } => {
                // Look for property inclusions: if P ⊑ Q and we have Q(x,y), add P(x,y)
                if let Some(subproperties) = self.tbox_index.property_inclusions.get(property) {
                    for subproperty in subproperties {
                        expansions.push(QueryAtom::ObjectPropertyAtom {
                            subject: subject.clone(),
                            property: subproperty.clone(),
                            object: object.clone(),
                        });
                    }
                }

                // Add domain and range constraints
                if let Some(domains) = self.tbox_index.property_domains.get(property) {
                    for domain in domains {
                        expansions.push(QueryAtom::ClassAtom {
                            variable: subject.clone(),
                            class_expression: domain.clone(),
                        });
                    }
                }

                if let Some(ranges) = self.tbox_index.property_ranges.get(property) {
                    for range in ranges {
                        expansions.push(QueryAtom::ClassAtom {
                            variable: object.clone(),
                            class_expression: range.clone(),
                        });
                    }
                }
            }

            _ => {
                // Other atom types don't need rewriting in QL
            }
        }

        Ok(expansions)
    }

    /// Generate new atoms that can be added to the query based on TBox axioms
    fn generate_new_atoms(&self, query: &ConjunctiveQuery) -> Result<Vec<QueryAtom>, RewritingError> {
        let mut new_atoms = Vec::new();
        let query_variables: HashSet<_> = query.get_all_variables();

        // For each pair of variables in the query, check if we can add property atoms
        for var1 in &query_variables {
            for var2 in &query_variables {
                if var1 != var2 && var1.is_individual() && var2.is_individual() {
                    // Check if there are axioms that would justify adding a property between these variables
                    for (property, _) in &self.tbox_index.property_inclusions {
                        new_atoms.push(QueryAtom::ObjectPropertyAtom {
                            subject: var1.clone(),
                            property: property.clone(),
                            object: var2.clone(),
                        });
                    }
                }
            }
        }

        Ok(new_atoms)
    }

    /// Remove redundant queries from the result set
    fn remove_redundant_queries(&self, queries: Vec<ConjunctiveQuery>) -> Vec<ConjunctiveQuery> {
        let mut unique_queries = Vec::new();
        let mut seen_hashes = HashSet::new();

        for query in queries {
            let hash = self.compute_query_hash(&query);
            if !seen_hashes.contains(&hash) {
                seen_hashes.insert(hash);
                unique_queries.push(query);
            }
        }

        // Further remove subsumed queries
        self.remove_subsumed_queries(unique_queries)
    }

    /// Remove queries that are subsumed by other queries
    fn remove_subsumed_queries(&self, queries: Vec<ConjunctiveQuery>) -> Vec<ConjunctiveQuery> {
        let mut result = Vec::new();

        for query in &queries {
            let mut is_subsumed = false;
            
            for other_query in &queries {
                if query != other_query && self.query_subsumes(other_query, query) {
                    is_subsumed = true;
                    break;
                }
            }
            
            if !is_subsumed {
                result.push(query.clone());
            }
        }

        result
    }

    /// Check if query1 subsumes query2
    fn query_subsumes(&self, _query1: &ConjunctiveQuery, _query2: &ConjunctiveQuery) -> bool {
        // Simplified subsumption check
        // In practice, this would involve checking if there's a homomorphism
        // from query1 to query2 that preserves answer variables
        false // Conservative: don't remove any queries for now
    }

    /// Generate a fresh variable name
    fn generate_fresh_variable(&self, prefix: &str) -> QueryVariable {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        QueryVariable::individual(format!("{}_{}", prefix, count))
    }

    /// Compute a hash for a query to check for duplicates
    fn compute_query_hash(&self, query: &ConjunctiveQuery) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash answer variables
        for var in &query.answer_variables {
            var.hash(&mut hasher);
        }
        
        // Hash body atoms (order-independent)
        let mut atom_hashes: Vec<u64> = query.body_atoms.iter().map(|atom| {
            let mut atom_hasher = DefaultHasher::new();
            // Simple hash based on atom structure
            std::mem::discriminant(atom).hash(&mut atom_hasher);
            atom_hasher.finish()
        }).collect();
        atom_hashes.sort();
        
        for hash in atom_hashes {
            hash.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    /// Validate that the query is compatible with OWL 2 QL profile
    fn validate_ql_compatibility(&self, query: &ConjunctiveQuery) -> Result<(), RewritingError> {
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { class_expression, .. } => {
                    if !self.is_ql_class_expression(class_expression) {
                        return Err(RewritingError::NotQLProfile(
                            format!("Complex class expression not allowed in QL: {}", class_expression)
                        ));
                    }
                }
                QueryAtom::ObjectPropertyAtom { .. } |
                QueryAtom::DataPropertyAtom { .. } |
                QueryAtom::SameIndividualAtom { .. } |
                QueryAtom::DifferentIndividualsAtom { .. } |
                QueryAtom::ConcreteIndividualAtom { .. } |
                QueryAtom::ConcreteLiteralAtom { .. } => {
                    // These are allowed in QL
                }
            }
        }
        Ok(())
    }

    /// Check if a class expression is allowed in OWL 2 QL
    fn is_ql_class_expression(&self, expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => {
                matches!(**filler, ClassExpression::Class(_))
            }
            _ => false, // Other expressions not allowed in QL query atoms
        }
    }

    /// Build TBox index for efficient query rewriting
    fn build_tbox_index(ontology: &Ontology) -> Result<TBoxIndex, RewritingError> {
        let mut index = TBoxIndex {
            class_inclusions: HashMap::new(),
            property_inclusions: HashMap::new(),
            existential_restrictions: Vec::new(),
            property_domains: HashMap::new(),
            property_ranges: HashMap::new(),
        };

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(axiom) => {
                    index.class_inclusions
                        .entry(axiom.superclass.clone())
                        .or_insert_with(Vec::new)
                        .push(axiom.subclass.clone());
                }
                Axiom::SubObjectPropertyOf(axiom) => {
                    index.property_inclusions
                        .entry(axiom.super_property.clone())
                        .or_insert_with(Vec::new)
                        .push(axiom.sub_property.clone());
                }
                Axiom::ObjectPropertyDomain(axiom) => {
                    index.property_domains
                        .entry(axiom.property.clone())
                        .or_insert_with(Vec::new)
                        .push(axiom.domain.clone());
                }
                Axiom::ObjectPropertyRange(axiom) => {
                    index.property_ranges
                        .entry(axiom.property.clone())
                        .or_insert_with(Vec::new)
                        .push(axiom.range.clone());
                }
                _ => {
                    // Handle other axiom types as needed
                }
            }
        }

        Ok(index)
    }
}