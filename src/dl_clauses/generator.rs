//! Main DL clause generator implementation

use crate::{
    error::Result,
    ontology::{Axiom, Ontology},
};
use log::debug;
use std::collections::HashMap;

use crate::dl_clauses::{
    axiom_compilers::AxiomCompiler,
    types::{DLClause, DLClauseSet, DLClauseStatistics},
};

/// DL clause generator that converts OWL axioms to DL clauses
pub struct DLClauseGenerator {
    pub variable_counter: u32,
    pub clause_counter: u32,
    pub definition_counter: u32, // For def:0, def:1, etc.
    pub prefixes: HashMap<String, String>,
}

impl DLClauseGenerator {
    /// Create a new DL clause generator
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();

        // Add standard prefixes
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        // Add HermiT-style internal prefixes
        prefixes.insert("def".to_string(), "internal:def#".to_string());
        prefixes.insert("all".to_string(), "internal:all#".to_string());
        prefixes.insert("nom".to_string(), "internal:nom#".to_string());

        Self {
            variable_counter: 0,
            clause_counter: 0,
            definition_counter: 0,
            prefixes,
        }
    }

    /// Generate DL clauses from an ontology
    pub fn generate_clauses(&mut self, ontology: &Ontology) -> Result<DLClauseSet> {
        let mut deterministic_clauses = Vec::new();
        let mut disjunctive_clauses = Vec::new();
        let mut abox_facts = Vec::new();

        // Extract prefixes from ontology
        self.extract_prefixes(ontology);

        // Process each axiom with enhanced compilation
        for axiom in ontology.axioms() {
            debug!("Processing axiom type: {}", self.axiom_type_name(axiom));
            let clauses = self.compile_axiom(axiom)?;

            if !clauses.is_empty() {
                debug!("Generated {} clauses from axiom", clauses.len());
            }

            for clause in clauses {
                if clause.is_fact() {
                    // Convert single-head facts to ABox facts
                    if !clause.head.is_empty() {
                        abox_facts.extend(clause.head);
                    }
                } else if clause.is_disjunctive() {
                    disjunctive_clauses.push(clause);
                } else {
                    deterministic_clauses.push(clause);
                }
            }
        }

        // Apply advanced optimizations
        debug!(
            "Applying absorption optimization to {} deterministic clauses",
            deterministic_clauses.len()
        );
        self.apply_absorption(&mut deterministic_clauses);
        debug!(
            "After absorption: {} deterministic clauses",
            deterministic_clauses.len()
        );

        // Apply structural transformation optimizations
        self.apply_structural_transformations(&mut deterministic_clauses);
        debug!(
            "After structural transformations: {} deterministic clauses",
            deterministic_clauses.len()
        );

        // Calculate statistics
        let statistics = DLClauseStatistics {
            deterministic_clause_count: deterministic_clauses.len(),
            disjunctive_clause_count: disjunctive_clauses.len(),
            disjunction_count: disjunctive_clauses
                .iter()
                .map(|c| c.head.len())
                .sum::<usize>(),
            positive_fact_count: abox_facts.iter().filter(|f| f.is_positive).count(),
            negative_fact_count: abox_facts.iter().filter(|f| !f.is_positive).count(),
        };

        Ok(DLClauseSet {
            deterministic_clauses,
            disjunctive_clauses,
            abox_facts,
            prefixes: self.prefixes.clone(),
            statistics,
        })
    }

    /// Apply absorption optimization
    /// Absorption tries to merge clauses to reduce their number and improve efficiency
    fn apply_absorption(&self, clauses: &mut Vec<DLClause>) {
        // Absorption rule: C ⊑ D and C ⊑ E becomes C ⊑ D ⊓ E
        let mut i = 0;
        while i < clauses.len() {
            let mut j = i + 1;
            while j < clauses.len() {
                if self.can_absorb(&clauses[i], &clauses[j])
                    && let Some(absorbed) = self.absorb_clauses(&clauses[i], &clauses[j])
                {
                    clauses[i] = absorbed;
                    clauses.remove(j);
                    continue;
                }
                j += 1;
            }
            i += 1;
        }
    }

    /// Check if two clauses can be absorbed
    fn can_absorb(&self, clause1: &DLClause, clause2: &DLClause) -> bool {
        // Can absorb if both have single positive body atom and same body
        clause1.body.len() == 1
            && clause2.body.len() == 1
            && clause1.body[0] == clause2.body[0]
            && clause1.head.len() == 1
            && clause2.head.len() == 1
    }

    /// Absorb two clauses into one
    fn absorb_clauses(&self, clause1: &DLClause, clause2: &DLClause) -> Option<DLClause> {
        if !self.can_absorb(clause1, clause2) {
            return None;
        }

        // Create new clause with combined head
        let mut combined_head = clause1.head.clone();
        combined_head.extend(clause2.head.clone());

        Some(DLClause::new(
            combined_head,
            clause1.body.clone(),
            format!("absorbed_{}", clause1.id),
        ))
    }

    /// Apply structural transformation optimizations
    fn apply_structural_transformations(&self, clauses: &mut Vec<DLClause>) {
        // Placeholder for structural transformations like:
        // - Simplification of redundant clauses
        // - Elimination of tautologies
        // - Constraint propagation

        let initial_count = clauses.len();

        // Remove tautologies (clauses where head and body contain the same positive atom)
        clauses.retain(|clause| {
            !clause.head.iter().any(|head_atom| {
                clause.body.iter().any(|body_atom| {
                    head_atom.predicate == body_atom.predicate
                        && head_atom.arguments == body_atom.arguments
                        && head_atom.is_positive
                        && body_atom.is_positive
                })
            })
        });

        // Remove subsumed clauses (simplified check)
        let mut i = 0;
        while i < clauses.len() {
            let mut j = i + 1;
            while j < clauses.len() {
                if self.is_subsumed(&clauses[i], &clauses[j]) {
                    clauses.remove(j);
                } else if self.is_subsumed(&clauses[j], &clauses[i]) {
                    clauses.remove(i);
                    break;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }

        let final_count = clauses.len();
        if final_count < initial_count {
            debug!(
                "Structural transformations removed {} clauses",
                initial_count - final_count
            );
        }
    }

    /// Check if clause1 is subsumed by clause2
    /// A clause C1 is subsumed by C2 if C2 is more general than C1
    fn is_subsumed(&self, clause1: &DLClause, clause2: &DLClause) -> bool {
        // Simple subsumption check: clause1 is subsumed by clause2 if:
        // - clause2's head is a subset of clause1's head
        // - clause2's body is a subset of clause1's body

        if clause2.head.len() > clause1.head.len() || clause2.body.len() > clause1.body.len() {
            return false;
        }

        // Check if all atoms in clause2's head are in clause1's head
        clause2.head.iter().all(|atom2| {
            clause1.head.iter().any(|atom1| atom1 == atom2)
        }) &&
        // Check if all atoms in clause2's body are in clause1's body
        clause2.body.iter().all(|atom2| {
            clause1.body.iter().any(|atom1| atom1 == atom2)
        })
    }

    /// Extract prefixes from the ontology
    fn extract_prefixes(&mut self, ontology: &Ontology) {
        // Add ontology base IRI if available
        if let Some(iri) = ontology.get_iri() {
            let iri_str = iri.as_str();
            if let Some(base) = iri_str.strip_suffix('#') {
                self.prefixes.insert(String::new(), format!("{base}#"));
            } else if let Some(base) = iri_str.strip_suffix('/') {
                self.prefixes.insert(String::new(), format!("{base}/"));
            }
        }

        // Extract prefixes from axioms (simplified - would need more sophisticated IRI analysis)
        for axiom in ontology.axioms() {
            self.extract_prefixes_from_axiom(axiom);
        }
    }

    /// Extract prefixes from a single axiom
    fn extract_prefixes_from_axiom(&mut self, axiom: &Axiom) {
        // Comprehensively analyze all IRIs in the axiom and extract common prefixes
        match axiom {
            Axiom::ClassAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&assertion.class) {
                    self.add_prefix_from_iri(&iri);
                }
                self.extract_prefixes_from_individual(&assertion.individual);
            }
            Axiom::SubClassOf(axiom) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.subclass) {
                    self.add_prefix_from_iri(&iri);
                }
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.superclass) {
                    self.add_prefix_from_iri(&iri);
                }
            }
            Axiom::EquivalentClasses(axiom) => {
                for class_expr in &axiom.classes {
                    if let Some(iri) = self.extract_iri_from_class_expression(class_expr) {
                        self.add_prefix_from_iri(&iri);
                    }
                }
            }
            Axiom::DisjointClasses(axiom) => {
                for class_expr in &axiom.classes {
                    if let Some(iri) = self.extract_iri_from_class_expression(class_expr) {
                        self.add_prefix_from_iri(&iri);
                    }
                }
            }
            Axiom::ObjectPropertyAssertion(assertion) => {
                self.extract_prefixes_from_object_property(&assertion.property);
                self.extract_prefixes_from_individual(&assertion.source);
                self.extract_prefixes_from_individual(&assertion.target);
            }
            Axiom::DataPropertyAssertion(assertion) => {
                self.extract_prefixes_from_data_property(&assertion.property);
                self.extract_prefixes_from_individual(&assertion.individual);
            }
            _ => {
                // Handle other axiom types as needed
            }
        }
    }

    /// Extract IRI from class expression (simplified)
    fn extract_iri_from_class_expression(
        &self,
        expr: &crate::ontology::ClassExpression,
    ) -> Option<String> {
        match expr {
            crate::ontology::ClassExpression::Class(class) => Some(class.iri.to_string()),
            _ => None,
        }
    }

    /// Add prefix from IRI
    fn add_prefix_from_iri(&mut self, iri: &str) {
        if let Some(hash_pos) = iri.rfind('#') {
            let base = &iri[..=hash_pos];
            if !self.prefixes.values().any(|v| v == base) {
                let prefix_name = format!("ns{}", self.prefixes.len());
                self.prefixes.insert(prefix_name, base.to_string());
            }
        }
    }

    /// Get human-readable axiom type name
    fn axiom_type_name(&self, axiom: &Axiom) -> &'static str {
        match axiom {
            Axiom::ClassAssertion(_) => "ClassAssertion",
            Axiom::SubClassOf(_) => "SubClassOf",
            Axiom::EquivalentClasses(_) => "EquivalentClasses",
            Axiom::DisjointClasses(_) => "DisjointClasses",
            Axiom::DisjointUnion(_) => "DisjointUnion",
            Axiom::Declaration(_) => "Declaration",
            Axiom::ObjectPropertyAssertion(_) => "ObjectPropertyAssertion",
            Axiom::NegativeObjectPropertyAssertion(_) => "NegativeObjectPropertyAssertion",
            Axiom::DataPropertyAssertion(_) => "DataPropertyAssertion",
            Axiom::NegativeDataPropertyAssertion(_) => "NegativeDataPropertyAssertion",
            Axiom::SubObjectPropertyOf(_) => "SubObjectPropertyOf",
            Axiom::EquivalentObjectProperties(_) => "EquivalentObjectProperties",
            Axiom::DisjointObjectProperties(_) => "DisjointObjectProperties",
            Axiom::InverseObjectProperties(_) => "InverseObjectProperties",
            Axiom::ObjectPropertyDomain(_) => "ObjectPropertyDomain",
            Axiom::ObjectPropertyRange(_) => "ObjectPropertyRange",
            Axiom::FunctionalObjectProperty(_) => "FunctionalObjectProperty",
            Axiom::InverseFunctionalObjectProperty(_) => "InverseFunctionalObjectProperty",
            Axiom::ReflexiveObjectProperty(_) => "ReflexiveObjectProperty",
            Axiom::IrreflexiveObjectProperty(_) => "IrreflexiveObjectProperty",
            Axiom::SymmetricObjectProperty(_) => "SymmetricObjectProperty",
            Axiom::AsymmetricObjectProperty(_) => "AsymmetricObjectProperty",
            Axiom::TransitiveObjectProperty(_) => "TransitiveObjectProperty",
            Axiom::SubDataPropertyOf(_) => "SubDataPropertyOf",
            Axiom::EquivalentDataProperties(_) => "EquivalentDataProperties",
            Axiom::DisjointDataProperties(_) => "DisjointDataProperties",
            Axiom::DataPropertyDomain(_) => "DataPropertyDomain",
            Axiom::DataPropertyRange(_) => "DataPropertyRange",
            Axiom::FunctionalDataProperty(_) => "FunctionalDataProperty",
            Axiom::HasKey(_) => "HasKey",
            Axiom::SameIndividual(_) => "SameIndividual",
            Axiom::DifferentIndividuals(_) => "DifferentIndividuals",
            Axiom::AnnotationAssertion(_) => "AnnotationAssertion",
            Axiom::SubAnnotationPropertyOf(_) => "SubAnnotationPropertyOf",
            Axiom::AnnotationPropertyDomain(_) => "AnnotationPropertyDomain",
            Axiom::AnnotationPropertyRange(_) => "AnnotationPropertyRange",
            Axiom::Rule(_) => "Rule",
            Axiom::DatatypeDefinition(_) => "DatatypeDefinition",
        }
    }

    /// Generate a fresh variable name
    pub fn fresh_variable(&mut self) -> String {
        self.variable_counter += 1;
        format!("X{}", self.variable_counter)
    }

    /// Generate a fresh clause ID
    pub fn next_clause_id(&mut self) -> String {
        self.clause_counter += 1;
        format!("clause_{}", self.clause_counter)
    }

    /// Generate a fresh definition name
    pub fn next_definition(&mut self) -> String {
        self.definition_counter += 1;
        format!("def:{}", self.definition_counter)
    }

    /// Get prefixes
    #[must_use]
    pub fn get_prefixes(&self) -> &HashMap<String, String> {
        &self.prefixes
    }

    /// Add a prefix
    pub fn add_prefix(&mut self, prefix: String, namespace: String) {
        self.prefixes.insert(prefix, namespace);
    }

    /// Compile a single axiom to DL clauses
    fn compile_axiom(&mut self, axiom: &Axiom) -> Result<Vec<DLClause>> {
        // Use the AxiomCompiler trait implementation
        AxiomCompiler::compile_axiom(self, axiom)
    }

    /// Extract prefixes from an individual
    fn extract_prefixes_from_individual(&mut self, individual: &crate::ontology::Individual) {
        match individual {
            crate::ontology::Individual::Named(named) => {
                self.add_prefix_from_iri(named.iri.as_str());
            }
            crate::ontology::Individual::Anonymous(_) => {
                // Anonymous individuals don't have IRIs to extract prefixes from
            }
        }
    }

    /// Extract prefixes from an object property
    fn extract_prefixes_from_object_property(
        &mut self,
        property: &crate::ontology::ObjectPropertyExpression,
    ) {
        match property {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                self.add_prefix_from_iri(prop.iri.as_str());
            }
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                self.add_prefix_from_iri(prop.iri.as_str());
            }
            crate::ontology::ObjectPropertyExpression::PropertyChain(chain) => {
                for prop_expr in chain {
                    self.extract_prefixes_from_object_property(prop_expr);
                }
            }
        }
    }

    /// Extract prefixes from a data property
    fn extract_prefixes_from_data_property(
        &mut self,
        property: &crate::ontology::DataPropertyExpression,
    ) {
        match property {
            crate::ontology::DataPropertyExpression::DataProperty(prop) => {
                self.add_prefix_from_iri(prop.iri.as_str());
            }
        }
    }
}

impl Default for DLClauseGenerator {
    fn default() -> Self {
        Self::new()
    }
}
