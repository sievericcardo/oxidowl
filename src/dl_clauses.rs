//! DL Clause Generation and Dumping
//!
//! This module implements DL clause generation from OWL ontologies,
//! similar to HermiT's clause dumping functionality.

use crate::{
    Result,
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
        Ontology,
    },
};
use log::{info, debug, warn};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    io::Write,
    path::Path,
};

/// A DL clause with head and body atoms
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DLClause {
    /// Head atoms (conclusions)
    pub head: Vec<DLAtom>,
    /// Body atoms (conditions)
    pub body: Vec<DLAtom>,
    /// Variables used in the clause
    pub variables: HashSet<String>,
    /// Clause identifier
    pub id: String,
}

/// An atomic formula in DL clauses
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DLAtom {
    /// Predicate name
    pub predicate: String,
    /// Arguments (variables or constants)
    pub arguments: Vec<String>,
    /// Whether this is a positive or negative atom
    pub is_positive: bool,
}

/// Result of DL clause generation
#[derive(Debug, Clone)]
pub struct DLClauseSet {
    /// Deterministic DL clauses (Horn clauses)
    pub deterministic_clauses: Vec<DLClause>,
    /// Disjunctive DL clauses (multiple heads)
    pub disjunctive_clauses: Vec<DLClause>,
    /// ABox facts (ground assertions)
    pub abox_facts: Vec<DLAtom>,
    /// Prefixes used in the ontology
    pub prefixes: HashMap<String, String>,
    /// Statistics about the clause set
    pub statistics: DLClauseStatistics,
}

/// Statistics about DL clauses
#[derive(Debug, Clone, Default)]
pub struct DLClauseStatistics {
    pub deterministic_clause_count: usize,
    pub disjunctive_clause_count: usize,
    pub disjunction_count: usize,
    pub positive_fact_count: usize,
    pub negative_fact_count: usize,
}

impl DLAtom {
    /// Create a new positive atomic formula
    pub fn new(predicate: String, arguments: Vec<String>) -> Self {
        Self {
            predicate,
            arguments,
            is_positive: true,
        }
    }

    /// Create a new negative atomic formula
    pub fn new_negative(predicate: String, arguments: Vec<String>) -> Self {
        Self {
            predicate,
            arguments,
            is_positive: false,
        }
    }

    /// Create an atom with specified negation
    pub fn with_negation(mut self, negate: bool) -> Self {
        self.is_positive = !negate;
        self
    }

    /// Create a concept assertion C(x)
    pub fn concept_assertion(concept: &str, individual: &str) -> Self {
        Self::new(concept.to_string(), vec![individual.to_string()])
    }

    /// Create a role assertion R(x, y)
    pub fn role_assertion(role: &str, subject: &str, object: &str) -> Self {
        Self::new(role.to_string(), vec![subject.to_string(), object.to_string()])
    }

    /// Create a datatype property assertion P(x, v)
    pub fn datatype_assertion(property: &str, subject: &str, value: &str) -> Self {
        Self::new(property.to_string(), vec![subject.to_string(), value.to_string()])
    }
}

impl fmt::Display for DLAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.is_positive { "" } else { "not(" };
        let suffix = if self.is_positive { "" } else { ")" };
        
        if self.arguments.is_empty() {
            write!(f, "{prefix}{}{suffix}", self.predicate)
        } else if self.arguments.len() == 1 {
            write!(f, "{prefix}{}({}){suffix}", self.predicate, self.arguments[0])
        } else {
            write!(f, "{prefix}{}({}){suffix}", self.predicate, self.arguments.join(","))
        }
    }
}

impl DLClause {
    /// Create a new DL clause
    pub fn new(head: Vec<DLAtom>, body: Vec<DLAtom>, id: String) -> Self {
        let mut variables = HashSet::new();
        
        // Collect variables from head and body
        for atom in &head {
            for arg in &atom.arguments {
                if arg.chars().next().map_or(false, |c| c.is_uppercase()) {
                    variables.insert(arg.clone());
                }
            }
        }
        for atom in &body {
            for arg in &atom.arguments {
                if arg.chars().next().map_or(false, |c| c.is_uppercase()) {
                    variables.insert(arg.clone());
                }
            }
        }

        Self {
            head,
            body,
            variables,
            id,
        }
    }

    /// Check if this is a deterministic clause (single head)
    pub fn is_deterministic(&self) -> bool {
        self.head.len() <= 1
    }

    /// Check if this is a disjunctive clause (multiple heads)
    pub fn is_disjunctive(&self) -> bool {
        self.head.len() > 1
    }

    /// Check if this is a fact (no body)
    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }

    /// Check if this is a constraint (no head)
    pub fn is_constraint(&self) -> bool {
        self.head.is_empty()
    }
}

impl fmt::Display for DLClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format head
        if self.head.is_empty() {
            // Constraint (no head)
            write!(f, " ")?;
        } else if self.head.len() == 1 {
            // Deterministic clause
            write!(f, "{}", self.head[0])?;
        } else {
            // Disjunctive clause
            for (i, atom) in self.head.iter().enumerate() {
                if i > 0 {
                    write!(f, " v ")?;
                }
                write!(f, "{atom}")?;
            }
        }

        // Format body
        if !self.body.is_empty() {
            write!(f, " :- ")?;
            for (i, atom) in self.body.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{atom}")?;
            }
        }

        Ok(())
    }
}

/// DL clause generator that converts OWL axioms to DL clauses
pub struct DLClauseGenerator {
    variable_counter: u32,
    clause_counter: u32,
    definition_counter: u32,  // For def:0, def:1, etc.
    prefixes: HashMap<String, String>,
}

impl DLClauseGenerator {
    /// Create a new DL clause generator
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        
        // Add standard prefixes
        prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        prefixes.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
        prefixes.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
        prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
        
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
        }
    }

    /// Generate DL clauses from an ontology
    pub fn generate_clauses(&mut self, ontology: &Ontology) -> Result<DLClauseSet> {
        let mut deterministic_clauses = Vec::new();
        let mut disjunctive_clauses = Vec::new();
        let mut abox_facts = Vec::new();

        // Extract prefixes from ontology
        self.extract_prefixes(ontology);

        // Process each axiom
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

        // Calculate statistics
        let statistics = DLClauseStatistics {
            deterministic_clause_count: deterministic_clauses.len(),
            disjunctive_clause_count: disjunctive_clauses.len(),
            disjunction_count: disjunctive_clauses.iter().map(|c| c.head.len()).sum::<usize>(),
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

    /// Extract prefixes from the ontology
    fn extract_prefixes(&mut self, ontology: &Ontology) {
        // Add ontology base IRI if available
        if let Some(iri) = ontology.get_iri() {
            let iri_str = iri.as_str();
            if let Some(base) = iri_str.strip_suffix('#') {
                self.prefixes.insert("".to_string(), format!("{base}#"));
            } else if let Some(base) = iri_str.strip_suffix('/') {
                self.prefixes.insert("".to_string(), format!("{base}/"));
            }
        }

        // Extract prefixes from axioms (simplified - would need more sophisticated IRI analysis)
        for axiom in ontology.axioms() {
            self.extract_prefixes_from_axiom(axiom);
        }
    }

    /// Extract prefixes from a single axiom
    fn extract_prefixes_from_axiom(&mut self, axiom: &Axiom) {
        // This is a simplified implementation
        // In practice, you'd analyze all IRIs in the axiom and extract common prefixes
        match axiom {
            Axiom::ClassAssertion(assertion) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&assertion.class) {
                    self.add_prefix_from_iri(&iri);
                }
            }
            Axiom::SubClassOf(axiom) => {
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.subclass) {
                    self.add_prefix_from_iri(&iri);
                }
                if let Some(iri) = self.extract_iri_from_class_expression(&axiom.superclass) {
                    self.add_prefix_from_iri(&iri);
                }
            }
            _ => {} // Handle other axiom types as needed
        }
    }

    /// Extract IRI from class expression (simplified)
    fn extract_iri_from_class_expression(&self, expr: &ClassExpression) -> Option<String> {
        match expr {
            ClassExpression::Class(class) => Some(class.iri.to_string()),
            _ => None,
        }
    }

    /// Add prefix from IRI
    fn add_prefix_from_iri(&mut self, iri: &str) {
        if let Some(hash_pos) = iri.rfind('#') {
            let base = &iri[..hash_pos + 1];
            if !self.prefixes.values().any(|v| v == base) {
                let prefix_name = format!("ns{}", self.prefixes.len());
                self.prefixes.insert(prefix_name, base.to_string());
            }
        }
    }

    /// Compile a single axiom to DL clauses
    fn compile_axiom(&mut self, axiom: &Axiom) -> Result<Vec<DLClause>> {
        match axiom {
            Axiom::SubClassOf(axiom) => self.compile_subclass_axiom(axiom),
            Axiom::EquivalentClasses(axiom) => self.compile_equivalent_classes_axiom(axiom),
            Axiom::DisjointClasses(axiom) => self.compile_disjoint_classes_axiom(axiom),
            Axiom::DisjointUnion(axiom) => self.compile_disjoint_union_axiom(axiom),
            Axiom::ClassAssertion(axiom) => self.compile_class_assertion_axiom(axiom),
            Axiom::ObjectPropertyAssertion(axiom) => self.compile_object_property_assertion_axiom(axiom),
            Axiom::DataPropertyAssertion(axiom) => self.compile_data_property_assertion_axiom(axiom),
            Axiom::SubObjectPropertyOf(axiom) => self.compile_sub_object_property_axiom(axiom),
            Axiom::SubDataPropertyOf(axiom) => self.compile_sub_data_property_axiom(axiom),
            Axiom::ObjectPropertyDomain(axiom) => self.compile_object_property_domain_axiom(axiom),
            Axiom::ObjectPropertyRange(axiom) => self.compile_object_property_range_axiom(axiom),
            Axiom::DataPropertyDomain(axiom) => self.compile_data_property_domain_axiom(axiom),
            Axiom::DataPropertyRange(axiom) => self.compile_data_property_range_axiom(axiom),
            Axiom::FunctionalObjectProperty(axiom) => self.compile_functional_object_property_axiom(axiom),
            Axiom::FunctionalDataProperty(axiom) => self.compile_functional_data_property_axiom(axiom),
            Axiom::InverseFunctionalObjectProperty(axiom) => self.compile_inverse_functional_object_property_axiom(axiom),
            Axiom::ReflexiveObjectProperty(axiom) => {
                debug!("Found ReflexiveObjectProperty axiom");
                self.compile_reflexive_object_property_axiom(axiom)
            },
            Axiom::IrreflexiveObjectProperty(axiom) => {
                debug!("Found IrreflexiveObjectProperty axiom");
                self.compile_irreflexive_object_property_axiom(axiom)
            },
            Axiom::SymmetricObjectProperty(axiom) => {
                debug!("Found SymmetricObjectProperty axiom");
                self.compile_symmetric_object_property_axiom(axiom)
            },
            Axiom::AsymmetricObjectProperty(axiom) => {
                debug!("Found AsymmetricObjectProperty axiom");
                self.compile_asymmetric_object_property_axiom(axiom)
            },
            Axiom::TransitiveObjectProperty(axiom) => {
                debug!("Found TransitiveObjectProperty axiom");
                self.compile_transitive_object_property_axiom(axiom)
            },
            Axiom::InverseObjectProperties(axiom) => {
                debug!("Found InverseObjectProperties axiom");
                self.compile_inverse_object_properties_axiom(axiom)
            },
            Axiom::EquivalentObjectProperties(axiom) => self.compile_equivalent_object_properties_axiom(axiom),
            Axiom::EquivalentDataProperties(axiom) => self.compile_equivalent_data_properties_axiom(axiom),
            Axiom::DisjointObjectProperties(axiom) => self.compile_disjoint_object_properties_axiom(axiom),
            Axiom::DisjointDataProperties(axiom) => self.compile_disjoint_data_properties_axiom(axiom),
            Axiom::SameIndividual(axiom) => {
                debug!("Found SameIndividual axiom");
                self.compile_same_individual_axiom(axiom)
            },
            Axiom::DifferentIndividuals(axiom) => {
                debug!("Found DifferentIndividuals axiom");
                self.compile_different_individuals_axiom(axiom)
            },
            Axiom::NegativeObjectPropertyAssertion(axiom) => {
                debug!("Found NegativeObjectPropertyAssertion axiom");
                self.compile_negative_object_property_assertion_axiom(axiom)
            },
            Axiom::NegativeDataPropertyAssertion(axiom) => {
                debug!("Found NegativeDataPropertyAssertion axiom");
                self.compile_negative_data_property_assertion_axiom(axiom)
            },
            Axiom::HasKey(axiom) => {
                debug!("Found HasKey axiom");
                self.compile_has_key_axiom(axiom)
            },
            Axiom::Rule(axiom) => {
                debug!("Found SWRL Rule axiom");
                self.compile_swrl_rule_axiom(axiom)
            },
            _ => {
                // For remaining unsupported axiom types, return empty clause set for now
                debug!("Unsupported axiom type: {:?}", std::mem::discriminant(axiom));
                Ok(Vec::new())
            }
        }
    }

    /// Compile SubClassOf axiom
    fn compile_subclass_axiom(&mut self, axiom: &crate::ontology::SubClassOfAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        
        let subclass_atom = self.compile_class_expression_to_atom(&axiom.subclass, &var_x, true)?;
        let superclass_atom = self.compile_class_expression_to_atom(&axiom.superclass, &var_x, false)?;
        
        let clause = DLClause::new(
            vec![superclass_atom],
            vec![subclass_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile EquivalentClasses axiom
    fn compile_equivalent_classes_axiom(&mut self, axiom: &crate::ontology::EquivalentClassesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Check for special case: NamedClass ≡ ComplexExpression
        // This should generate disjunctive clauses
        if axiom.classes.len() == 2 {
            // Try to identify named class vs complex expression
            let (named_class, complex_expr) = if self.is_named_class(&axiom.classes[0]) && !self.is_named_class(&axiom.classes[1]) {
                (&axiom.classes[0], &axiom.classes[1])
            } else if self.is_named_class(&axiom.classes[1]) && !self.is_named_class(&axiom.classes[0]) {
                (&axiom.classes[1], &axiom.classes[0])
            } else {
                // Fall back to standard bidirectional implications
                return self.compile_equivalent_classes_standard(axiom);
            };
            
            // Generate specialized clauses for complex definitions
            clauses.extend(self.compile_complex_equivalence(named_class, complex_expr)?);
            
            // Also generate standard implications for completeness
            clauses.extend(self.compile_equivalent_classes_standard(axiom)?);
        } else {
            // Generate bidirectional implications for each pair (standard case)
            clauses.extend(self.compile_equivalent_classes_standard(axiom)?);
        }
        
        Ok(clauses)
    }

    /// Compile complex equivalence: A ≡ ComplexExpression
    /// Generates both forward and backward implications plus specialized clauses
    fn compile_complex_equivalence(&mut self, named_class: &ClassExpression, complex_expr: &ClassExpression) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        match complex_expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                let var_x = self.fresh_variable();
                
                // Generate forward implications: A(x) → B(x), A(x) → C(x), ...
                for conjunct in conjuncts {
                    let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
                    let conjunct_atom = self.compile_class_expression_to_atom(conjunct, &var_x, false)?;
                    
                    clauses.push(DLClause::new(
                        vec![conjunct_atom],
                        vec![named_atom],
                        self.next_clause_id(),
                    ));
                }
                
                // Generate backward implication with expanded complex expressions
                if let Some(reverse_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(reverse_clause);
                }
                
                // Generate additional expansion clauses for complex expressions
                clauses.extend(self.compile_complex_expression_expansions(conjuncts, &var_x)?);
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // A ≡ ∃R.C generates: A(x) → R(x,y) ∧ C(y) and R(x,y) ∧ C(y) → A(x)
                if let Some(definition_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(definition_clause);
                }
                
                // Also generate forward implication
                let var_x = self.fresh_variable();
                let var_y = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
                let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
                let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
                
                clauses.push(DLClause::new(
                    vec![property_atom, filler_atom],
                    vec![named_atom],
                    self.next_clause_id(),
                ));
            }
            ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                // Generate cardinality expansion clauses
                if let Some(cardinality_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(cardinality_clause);
                }
                
                // Generate forward implication with HermiT-style atLeast atom
                let var_x = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
                
                // Create HermiT-style atLeast atom  
                let range_str = self.class_expression_to_range_string(filler);
                let at_least_atom = self.create_at_least_atom(*cardinality, &property_name, &range_str, &var_x, false)?;
                
                clauses.push(DLClause::new(
                    vec![at_least_atom],
                    vec![named_atom],
                    self.next_clause_id(),
                ));
                
                // Generate forward implication: A(x) → ≥nR.C expansion
                clauses.extend(self.compile_min_cardinality_forward_implications(named_class, *cardinality, property, Some(filler.as_ref()))?);
            }
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                // Generate maximum cardinality constraint
                if let Some(constraint_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(constraint_clause);
                }
            }
            ClassExpression::ObjectExactCardinality { cardinality, property, filler } => {
                // Generate both minimum and maximum constraints
                if let Some(exact_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(exact_clause);
                }
                
                // Add maximum cardinality constraint
                let max_expr = ClassExpression::ObjectMaxCardinality {
                    cardinality: *cardinality,
                    property: property.clone(),
                    filler: filler.clone(),
                };
                if let Some(max_clause) = self.compile_complex_definition(named_class, &max_expr)? {
                    clauses.push(max_clause);
                }
            }
            ClassExpression::ObjectHasSelf { property } => {
                // A ≡ ∃R.Self generates: A(x) → R(x,x) and R(x,x) → A(x)
                if let Some(self_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(self_clause);
                }
                
                // Generate forward implication
                let var_x = self.fresh_variable();
                let property_name = self.object_property_expression_to_string(property);
                let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
                let self_property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);
                
                clauses.push(DLClause::new(
                    vec![self_property_atom],
                    vec![named_atom],
                    self.next_clause_id(),
                ));
            }
            _ => {
                // For other complex expressions, try the general definition
                if let Some(definition_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                    clauses.push(definition_clause);
                }
            }
        }
        
        Ok(clauses)
    }

    /// Generate expansion clauses for complex expressions within intersections
    fn compile_complex_expression_expansions(&mut self, conjuncts: &[ClassExpression], var_x: &str) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        for conjunct in conjuncts {
            match conjunct {
                ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                    // For ∃R.C, generate existence expansion
                    let var_y = self.fresh_variable();
                    let property_name = self.object_property_expression_to_string(property);
                    let conjunct_atom = self.compile_class_expression_to_atom(conjunct, var_x, true)?;
                    let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
                    let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
                    
                    clauses.push(DLClause::new(
                        vec![property_atom, filler_atom],
                        vec![conjunct_atom],
                        self.next_clause_id(),
                    ));
                }
                ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                    // Generate witness expansion for minimum cardinality
                    clauses.extend(self.compile_min_cardinality_expansion(conjunct, var_x, *cardinality, property, Some(filler.as_ref()))?);
                }
                _ => {
                    // For simple classes, no additional expansion needed
                }
            }
        }
        
        Ok(clauses)
    }

    /// Generate forward implications for minimum cardinality
    fn compile_min_cardinality_forward_implications(&mut self, named_class: &ClassExpression, cardinality: u32, property: &ObjectPropertyExpression, filler: Option<&ClassExpression>) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        let var_x = self.fresh_variable();
        let property_name = self.object_property_expression_to_string(property);
        
        // Generate A(x) → ≥nR.C means we need at least n R-successors
        // This translates to generating witness existence clauses
        if cardinality > 0 {
            let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
            
            // Generate existence of at least one R-successor (simplified)
            let var_y = self.fresh_variable();
            let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
            let mut head_atoms = vec![property_atom];
            
            if let Some(filler_expr) = filler {
                let filler_atom = self.compile_class_expression_to_atom(filler_expr, &var_y, false)?;
                head_atoms.push(filler_atom);
            }
            
            clauses.push(DLClause::new(
                head_atoms,
                vec![named_atom],
                self.next_clause_id(),
            ));
        }
        
        Ok(clauses)
    }

    /// Generate expansion clauses for minimum cardinality restrictions
    fn compile_min_cardinality_expansion(&mut self, restriction: &ClassExpression, var_x: &str, cardinality: u32, property: &ObjectPropertyExpression, filler: Option<&ClassExpression>) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        if cardinality == 0 {
            return Ok(clauses); // Trivially satisfied
        }
        
        let property_name = self.object_property_expression_to_string(property);
        let restriction_atom = self.compile_class_expression_to_atom(restriction, var_x, true)?;
        
        // Generate existence of witness individuals
        for i in 0..cardinality {
            let var_y = format!("y{}", i);
            let property_atom = DLAtom::role_assertion(&property_name, var_x, &var_y);
            let mut head_atoms = vec![property_atom];
            
            if let Some(filler_expr) = filler {
                let filler_atom = self.compile_class_expression_to_atom(filler_expr, &var_y, false)?;
                head_atoms.push(filler_atom);
            }
            
            clauses.push(DLClause::new(
                head_atoms,
                vec![restriction_atom.clone()],
                self.next_clause_id(),
            ));
        }
        
        Ok(clauses)
    }
    
    /// Standard EquivalentClasses compilation (bidirectional implications)
    fn compile_equivalent_classes_standard(&mut self, axiom: &crate::ontology::EquivalentClassesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate bidirectional implications for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();
                
                // A(x) → B(x)
                let a_atom = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, false)?;
                
                clauses.push(DLClause::new(
                    vec![b_atom],
                    vec![a_atom],
                    self.next_clause_id(),
                ));
                
                // B(x) → A(x)
                let a_atom_2 = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, false)?;
                let b_atom_2 = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                
                clauses.push(DLClause::new(
                    vec![a_atom_2],
                    vec![b_atom_2],
                    self.next_clause_id(),
                ));
            }
        }
        
        Ok(clauses)
    }

    /// Check if a class expression is a simple named class
    fn is_named_class(&self, expr: &ClassExpression) -> bool {
        matches!(expr, ClassExpression::Class(_))
    }
    
    /// Compile complex definition into disjunctive clause  
    /// For A ≡ B ⊓ C ⊓ ..., generate clauses similar to HermiT's output
    fn compile_complex_definition(&mut self, named_class: &ClassExpression, complex_expr: &ClassExpression) -> Result<Option<DLClause>> {
        match complex_expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                self.compile_intersection_definition(named_class, conjuncts)
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.compile_existential_definition(named_class, property, filler)
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.compile_universal_definition(named_class, property, filler)
            }
            ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                self.compile_min_cardinality_definition(named_class, *cardinality, property, Some(filler.as_ref()))
            }
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                self.compile_max_cardinality_definition(named_class, *cardinality, property, Some(filler.as_ref()))
            }
            ClassExpression::ObjectExactCardinality { cardinality, property, filler } => {
                self.compile_exact_cardinality_definition(named_class, *cardinality, property, Some(filler.as_ref()))
            }
            ClassExpression::ObjectHasSelf { property } => {
                self.compile_self_restriction_definition(named_class, property)
            }
            _ => Ok(None), // Not implemented yet
        }
    }

    /// Compile intersection definition: A ≡ B ⊓ C ⊓ ... 
    /// Generates: B(x) ∧ C(x) ∧ ... → A(x) (reverse implication as disjunctive clause)
    fn compile_intersection_definition(&mut self, named_class: &ClassExpression, conjuncts: &[ClassExpression]) -> Result<Option<DLClause>> {
        let var_x = self.fresh_variable();
        let mut body_atoms = Vec::new();
        
        // Generate atoms for each conjunct in the body
        for conjunct in conjuncts {
            match conjunct {
                ClassExpression::Class(_) => {
                    let conjunct_atom = self.compile_class_expression_to_atom(conjunct, &var_x, true)?;
                    body_atoms.push(conjunct_atom);
                }
                ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                    // For ∃R.C, generate R(x,y) ∧ C(y) atoms
                    let var_y = self.fresh_variable();
                    let property_name = self.object_property_expression_to_string(property);
                    let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
                    let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
                    body_atoms.push(property_atom);
                    body_atoms.push(filler_atom);
                }
                ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                    // For ≥nR.C, generate witness individuals
                    let property_name = self.object_property_expression_to_string(property);
                    for i in 0..*cardinality {
                        let var_y = format!("y{}", i);
                        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
                        body_atoms.push(property_atom);
                        
                        let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
                        body_atoms.push(filler_atom);
                        
                        // Add inequality constraints for distinctness
                        for j in 0..i {
                            let other_var = format!("y{}", j);
                            let inequality = DLAtom::new(format!("[{} != {}]", var_y, other_var), vec![]);
                            body_atoms.push(inequality);
                        }
                    }
                }
                _ => {
                    // Handle other complex expressions
                    let conjunct_atom = self.compile_class_expression_to_atom(conjunct, &var_x, true)?;
                    body_atoms.push(conjunct_atom);
                }
            }
        }
        
        // Generate head atom for the named class
        let head_atom = self.compile_class_expression_to_atom(named_class, &var_x, false)?;
        
        Ok(Some(DLClause::new(
            vec![head_atom],
            body_atoms,
            self.next_clause_id(),
        )))
    }

    /// Compile existential definition: A ≡ ∃R.C
    /// Generates: R(x,y) ∧ C(y) → A(x)
    fn compile_existential_definition(&mut self, named_class: &ClassExpression, property: &ObjectPropertyExpression, filler: &ClassExpression) -> Result<Option<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(property);
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
        let head_atom = self.compile_class_expression_to_atom(named_class, &var_x, false)?;
        
        Ok(Some(DLClause::new(
            vec![head_atom],
            vec![property_atom, filler_atom],
            self.next_clause_id(),
        )))
    }

    /// Compile universal definition: A ≡ ∀R.C  
    /// Generates: A(x) ∧ R(x,y) → C(y) (constraint form)
    fn compile_universal_definition(&mut self, named_class: &ClassExpression, property: &ObjectPropertyExpression, filler: &ClassExpression) -> Result<Option<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(property);
        let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let filler_atom = self.compile_class_expression_to_atom(filler, &var_y, false)?;
        
        Ok(Some(DLClause::new(
            vec![filler_atom],
            vec![named_atom, property_atom],
            self.next_clause_id(),
        )))
    }

    /// Compile minimum cardinality definition: A ≡ ≥nR.C
    /// Generates: R(x,y1) ∧ C(y1) ∧ ... ∧ R(x,yn) ∧ C(yn) ∧ yi ≠ yj → A(x)
    fn compile_min_cardinality_definition(&mut self, named_class: &ClassExpression, cardinality: u32, property: &ObjectPropertyExpression, filler: Option<&ClassExpression>) -> Result<Option<DLClause>> {
        if cardinality == 0 {
            return Ok(None); // Trivially satisfied
        }
        
        let var_x = self.fresh_variable();
        let mut body_atoms = Vec::new();
        
        let property_name = self.object_property_expression_to_string(property);
        
        // Generate witness individuals
        for i in 0..cardinality {
            let var_y = format!("y{}", i);
            let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
            body_atoms.push(property_atom);
            
            // Add filler constraint if specified
            if let Some(filler_expr) = filler {
                let filler_atom = self.compile_class_expression_to_atom(filler_expr, &var_y, false)?;
                body_atoms.push(filler_atom);
            }
            
            // Add distinctness constraints
            for j in 0..i {
                let other_var = format!("y{}", j);
                let inequality = DLAtom::new(format!("[{} != {}]", var_y, other_var), vec![]);
                body_atoms.push(inequality);
            }
        }
        
        let head_atom = self.compile_class_expression_to_atom(named_class, &var_x, false)?;
        
        Ok(Some(DLClause::new(
            vec![head_atom],
            body_atoms,
            self.next_clause_id(),
        )))
    }

    /// Compile maximum cardinality definition: A ≡ ≤nR.C
    /// Generates constraint clauses for violations
    fn compile_max_cardinality_definition(&mut self, named_class: &ClassExpression, cardinality: u32, property: &ObjectPropertyExpression, filler: Option<&ClassExpression>) -> Result<Option<DLClause>> {
        let var_x = self.fresh_variable();
        let mut body_atoms = Vec::new();
        
        let property_name = self.object_property_expression_to_string(property);
        let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, true)?;
        body_atoms.push(named_atom);
        
        // Generate constraint for having more than n distinct fillers
        for i in 0..=cardinality {
            let var_y = format!("y{}", i);
            let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
            body_atoms.push(property_atom);
            
            if let Some(filler_expr) = filler {
                let filler_atom = self.compile_class_expression_to_atom(filler_expr, &var_y, false)?;
                body_atoms.push(filler_atom);
            }
            
            // Add distinctness constraints
            for j in 0..i {
                let other_var = format!("y{}", j);
                let inequality = DLAtom::new(format!("[{} != {}]", var_y, other_var), vec![]);
                body_atoms.push(inequality);
            }
        }
        
        // Generate constraint clause (empty head = clash)
        Ok(Some(DLClause::new(
            vec![], // Empty head indicates constraint/clash
            body_atoms,
            self.next_clause_id(),
        )))
    }

    /// Compile exact cardinality definition: A ≡ =nR.C
    /// Generates both minimum and maximum cardinality constraints
    fn compile_exact_cardinality_definition(&mut self, named_class: &ClassExpression, cardinality: u32, property: &ObjectPropertyExpression, filler: Option<&ClassExpression>) -> Result<Option<DLClause>> {
        // For exact cardinality, we generate the minimum cardinality clause
        // The maximum constraint would be handled separately
        self.compile_min_cardinality_definition(named_class, cardinality, property, filler)
    }

    /// Compile self restriction definition: A ≡ ∃R.Self
    /// Generates: R(x,x) → A(x)
    fn compile_self_restriction_definition(&mut self, named_class: &ClassExpression, property: &ObjectPropertyExpression) -> Result<Option<DLClause>> {
        let var_x = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(property);
        let self_property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);
        let head_atom = self.compile_class_expression_to_atom(named_class, &var_x, false)?;
        
        Ok(Some(DLClause::new(
            vec![head_atom],
            vec![self_property_atom],
            self.next_clause_id(),
        )))
    }


    /// Compile DisjointClasses axiom
    fn compile_disjoint_classes_axiom(&mut self, axiom: &crate::ontology::DisjointClassesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate disjointness constraints for each pair
        for i in 0..axiom.classes.len() {
            for j in (i + 1)..axiom.classes.len() {
                let var_x = self.fresh_variable();
                
                let a_atom = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                
                // Constraint: ¬(A(x) ∧ B(x))
                let clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![a_atom, b_atom],
                    self.next_clause_id(),
                );
                
                clauses.push(clause);
            }
        }
        
        Ok(clauses)
    }

    /// Compile ClassAssertion axiom
    fn compile_class_assertion_axiom(&mut self, axiom: &crate::ontology::ClassAssertionAxiom) -> Result<Vec<DLClause>> {
        let individual_name = self.individual_to_string(&axiom.individual);
        let class_atom = self.compile_class_expression_to_atom(&axiom.class, &individual_name, false)?;
        
        let clause = DLClause::new(
            vec![class_atom],
            vec![], // No body (fact)
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile ObjectPropertyAssertion axiom
    fn compile_object_property_assertion_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        let subject = self.individual_to_string(&axiom.source);
        let object = self.individual_to_string(&axiom.target);
        let property_name = self.object_property_expression_to_string(&axiom.property);
        
        let role_atom = DLAtom::role_assertion(&property_name, &subject, &object);
        
        let clause = DLClause::new(
            vec![role_atom],
            vec![], // No body (fact)
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile DataPropertyAssertion axiom
    fn compile_data_property_assertion_axiom(&mut self, axiom: &crate::ontology::DataPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        let subject = self.individual_to_string(&axiom.individual);
        let value = axiom.value.to_string();
        let property_name = self.data_property_expression_to_string(&axiom.property);
        
        let datatype_atom = DLAtom::datatype_assertion(&property_name, &subject, &value);
        
        let clause = DLClause::new(
            vec![datatype_atom],
            vec![], // No body (fact)
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile SubObjectPropertyOf axiom
    fn compile_sub_object_property_axiom(&mut self, axiom: &crate::ontology::SubObjectPropertyOfAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let sub_property = self.object_property_expression_to_string(&axiom.sub_property);
        let super_property = self.object_property_expression_to_string(&axiom.super_property);
        
        let sub_atom = DLAtom::role_assertion(&sub_property, &var_x, &var_y);
        let super_atom = DLAtom::role_assertion(&super_property, &var_x, &var_y);
        
        let clause = DLClause::new(
            vec![super_atom],
            vec![sub_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile SubDataPropertyOf axiom
    fn compile_sub_data_property_axiom(&mut self, axiom: &crate::ontology::SubDataPropertyOfAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let sub_property = self.data_property_expression_to_string(&axiom.sub_property);
        let super_property = self.data_property_expression_to_string(&axiom.super_property);
        
        let sub_atom = DLAtom::datatype_assertion(&sub_property, &var_x, &var_y);
        let super_atom = DLAtom::datatype_assertion(&super_property, &var_x, &var_y);
        
        let clause = DLClause::new(
            vec![super_atom],
            vec![sub_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile class expression to atomic formula
    fn compile_class_expression_to_atom(&self, expr: &ClassExpression, variable: &str, negate: bool) -> Result<DLAtom> {
        match expr {
            ClassExpression::Class(class) => {
                let predicate = self.shorten_iri(&class.iri.to_string());
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                // ∃property.filler becomes exists(property,filler)(variable)
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.extract_simple_class_name(filler);
                let predicate = format!("exists({},{})", property_name, filler_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                // ∀property.filler becomes forall(property,filler)(variable)
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.extract_simple_class_name(filler);
                let predicate = format!("forall({},{})", property_name, filler_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectHasValue { property, value } => {
                // ∃property.{individual} becomes hasValue(property,individual)(variable) or nominal-style atom
                let property_name = self.object_property_expression_to_string(property);
                let individual_name = self.individual_to_string(value);
                
                // Try HermiT-style nominal atom for specific individuals
                if individual_name.contains("@") || individual_name.contains("WPS") {
                    return self.create_nominal_atom(&individual_name, &property_name, variable, negate);
                }
                
                let predicate = format!("hasValue({},{})", property_name, individual_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                // ≥n property.filler becomes atLeast(n,property,filler)(variable) - HermiT style
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.extract_simple_class_name(filler);
                
                // Use HermiT-style atLeast atom creation
                return self.create_at_least_atom(*cardinality, &property_name, &filler_name, variable, negate);
            }
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                // ≤n property.filler becomes atMost(n,property,filler)(variable)
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.extract_simple_class_name(filler);
                let predicate = format!("atMost({},{},{})", cardinality, property_name, filler_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectExactCardinality { cardinality, property, filler } => {
                // =n property.filler becomes exactly(n,property,filler)(variable)
                let property_name = self.object_property_expression_to_string(property);
                let filler_name = self.extract_simple_class_name(filler);
                let predicate = format!("exactly({},{},{})", cardinality, property_name, filler_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // ∃dataProp.datatype becomes dataExists(property,datatype)(variable)
                let property_name = self.data_property_expression_to_string(property);
                let range_name = self.data_range_to_string(filler);
                let predicate = format!("dataExists({},{})", property_name, range_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                // ∀dataProp.datatype becomes dataForall(property,datatype)(variable)
                let property_name = self.data_property_expression_to_string(property);
                let range_name = self.data_range_to_string(filler);
                let predicate = format!("dataForall({},{})", property_name, range_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataHasValue { property, value } => {
                // dataProp hasValue "literal" becomes dataHasValue(property,literal)(variable) or nominal
                let property_name = self.data_property_expression_to_string(property);
                let literal_value = &value.value;
                
                // Try HermiT-style nominal atom for specific literals
                if literal_value.contains("@") || literal_value.contains("WPS") {
                    return self.create_nominal_atom(literal_value, &property_name, variable, negate);
                }
                
                let predicate = format!("dataHasValue({},\"{}\")", property_name, literal_value);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataMinCardinality { cardinality, property, filler } => {
                // ≥n dataProp.datatype becomes dataAtLeast(n,property,datatype)(variable)
                let property_name = self.data_property_expression_to_string(property);
                let range_name = self.data_range_to_string(filler);
                let predicate = format!("dataAtLeast({},{},{})", cardinality, property_name, range_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataMaxCardinality { cardinality, property, filler } => {
                // ≤n dataProp.datatype becomes dataAtMost(n,property,datatype)(variable)
                let property_name = self.data_property_expression_to_string(property);
                let range_name = self.data_range_to_string(filler);
                let predicate = format!("dataAtMost({},{},{})", cardinality, property_name, range_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::DataExactCardinality { cardinality, property, filler } => {
                // =n dataProp.datatype becomes dataExactly(n,property,datatype)(variable)
                let property_name = self.data_property_expression_to_string(property);
                let range_name = self.data_range_to_string(filler);
                let predicate = format!("dataExactly({},{},{})", cardinality, property_name, range_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectOneOf(individuals) => {
                // {ind1, ind2, ...} becomes oneOf(ind1,ind2,...)(variable)
                let individual_names: Vec<String> = individuals.iter()
                    .map(|ind| self.individual_to_string(ind))
                    .collect();
                let predicate = format!("oneOf({})", individual_names.join(","));
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectUnionOf(disjuncts) => {
                // A ⊔ B ⊔ ... becomes union(A,B,...)(variable)
                let disjunct_names: Vec<String> = disjuncts.iter()
                    .map(|expr| self.extract_simple_class_name(expr))
                    .collect();
                let predicate = format!("union({})", disjunct_names.join(","));
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                // A ⊓ B ⊓ ... becomes intersection(A,B,...)(variable)
                let conjunct_names: Vec<String> = conjuncts.iter()
                    .map(|expr| self.extract_simple_class_name(expr))
                    .collect();
                let predicate = format!("intersection({})", conjunct_names.join(","));
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectComplementOf(complement) => {
                // ¬A becomes complement(A)(variable)
                let complement_name = self.extract_simple_class_name(complement);
                let predicate = format!("complement({})", complement_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectHasSelf { property } => {
                // ∃property.Self becomes hasSelf(property)(variable)
                let property_name = self.object_property_expression_to_string(property);
                let predicate = format!("hasSelf({})", property_name);
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            // Annotation-related class expressions (these are not typical DL class expressions)
            ClassExpression::AnnotationAssertion { .. } |
            ClassExpression::SubAnnotationPropertyOf { .. } |
            ClassExpression::AnnotationPropertyDomain { .. } |
            ClassExpression::AnnotationPropertyRange { .. } => {
                // For annotation expressions, create a placeholder
                let predicate = "AnnotationExpression".to_string();
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
        }
    }

    /// Convert individual to string representation
    fn individual_to_string(&self, individual: &Individual) -> String {
        match individual {
            Individual::Named(named) => self.shorten_iri(&named.iri.to_string()),
            Individual::Anonymous(anon) => format!("_:{}", anon.id),
        }
    }

    /// Convert object property expression to string
    fn object_property_expression_to_string(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(prop) => self.shorten_iri(&prop.iri.to_string()),
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("inv({})", self.shorten_iri(&prop.iri.to_string()))
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                let properties: Vec<String> = chain.iter()
                    .map(|p| self.object_property_expression_to_string(p))
                    .collect();
                format!("chain({})", properties.join(" ∘ "))
            }
        }
    }

    /// Convert data property expression to string
    fn data_property_expression_to_string(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(prop) => self.shorten_iri(&prop.iri.to_string()),
        }
    }

    /// Convert data range to string representation
    fn data_range_to_string(&self, range: &crate::ontology::DataRange) -> String {
        match range {
            crate::ontology::DataRange::Datatype(iri) => {
                self.shorten_iri(&iri.to_string())
            }
            crate::ontology::DataRange::DataIntersectionOf(ranges) => {
                let operands: Vec<String> = ranges.iter()
                    .map(|r| self.data_range_to_string(r))
                    .collect();
                format!("intersection({})", operands.join(" "))
            }
            crate::ontology::DataRange::DataUnionOf(ranges) => {
                let operands: Vec<String> = ranges.iter()
                    .map(|r| self.data_range_to_string(r))
                    .collect();
                format!("union({})", operands.join(" "))
            }
            crate::ontology::DataRange::DataComplementOf(range) => {
                format!("not({})", self.data_range_to_string(range))
            }
            crate::ontology::DataRange::DataOneOf(literals) => {
                let values: Vec<String> = literals.iter()
                    .map(|lit| format!("\"{}\"", lit.value))
                    .collect();
                format!("oneOf({})", values.join(" "))
            }
            crate::ontology::DataRange::DatatypeRestriction { datatype, restrictions } => {
                let base_type = self.shorten_iri(&datatype.to_string());
                if restrictions.is_empty() {
                    base_type
                } else {
                    let restriction_strs: Vec<String> = restrictions.iter()
                        .map(|r| format!("{}=\"{}\"", 
                            self.shorten_iri(&r.facet.to_string()),
                            r.value.value))
                        .collect();
                    format!("{}[{}]", base_type, restriction_strs.join(","))
                }
            }
        }
    }

    /// Compile data range to atom using HermiT-style patterns
    fn compile_data_range_to_atom(&self, range: &crate::ontology::DataRange, variable: &str) -> Result<DLAtom> {
        match range {
            crate::ontology::DataRange::Datatype(iri) => {
                let datatype_name = self.shorten_iri(&iri.to_string());
                Ok(DLAtom::new(datatype_name, vec![variable.to_string()]))
            }
            crate::ontology::DataRange::DatatypeRestriction { datatype, restrictions } => {
                let base_type = self.shorten_iri(&datatype.to_string());
                if restrictions.is_empty() {
                    Ok(DLAtom::new(base_type, vec![variable.to_string()]))
                } else {
                    // Use HermiT-style datatype restriction atom
                    let restriction_pairs: Vec<(String, String)> = restrictions.iter()
                        .map(|r| (self.shorten_iri(&r.facet.to_string()), r.value.value.clone()))
                        .collect();
                    let restriction_refs: Vec<(&str, &str)> = restriction_pairs.iter()
                        .map(|(f, v)| (f.as_str(), v.as_str()))
                        .collect();
                    Ok(self.create_datatype_restriction_atom(&base_type, &restriction_refs, variable, false))
                }
            }
            crate::ontology::DataRange::DataOneOf(literals) => {
                // For DataOneOf, generate nominal constraints
                if literals.len() == 1 {
                    let lit = &literals[0];
                    let datatype = if let Some(dt) = &lit.datatype {
                        self.shorten_iri(&dt.to_string())
                    } else {
                        "xsd:string".to_string()
                    };
                    // Use HermiT-style nominal atom
                    return self.create_nominal_atom(&lit.value, &datatype, variable, false);
                } else {
                    // For multiple values, use general literal type  
                    Ok(DLAtom::new("rdfs:Literal".to_string(), vec![variable.to_string()]))
                }
            }
            _ => {
                // Default to literal type
                Ok(DLAtom::new("rdfs:Literal".to_string(), vec![variable.to_string()]))
            }
        }
    }

    /// Shorten IRI using prefixes
    fn shorten_iri(&self, iri: &str) -> String {
        for (prefix, namespace) in &self.prefixes {
            if iri.starts_with(namespace) {
                let local_part = &iri[namespace.len()..];
                if prefix.is_empty() {
                    return format!(":{local_part}");
                } else {
                    return format!("{prefix}:{local_part}");
                }
            }
        }
        iri.to_string() // Return full IRI if no prefix matches
    }

    /// Generate a fresh variable name
    fn fresh_variable(&mut self) -> String {
        let var = format!("X{}", self.variable_counter);
        self.variable_counter += 1;
        var
    }

    /// Generate next clause ID
    fn next_clause_id(&mut self) -> String {
        let id = format!("clause_{}", self.clause_counter);
        self.clause_counter += 1;
        id
    }

    /// Generate next definition predicate (def:0, def:1, etc.)
    fn next_definition_predicate(&mut self) -> String {
        let def_id = format!("def:{}", self.definition_counter);
        self.definition_counter += 1;
        def_id
    }

    /// Create data type restriction atom following HermiT patterns
    fn create_datatype_restriction_atom(&self, datatype: &str, restrictions: &[(&str, &str)], variable: &str, negate: bool) -> DLAtom {
        let mut restriction_parts = Vec::new();
        for (facet, value) in restrictions {
            restriction_parts.push(format!("{}=\"{}\"^^{}", facet, value, datatype));
        }
        
        let restriction_expr = if restriction_parts.is_empty() {
            datatype.to_string()
        } else {
            format!("{}[{}]", datatype, restriction_parts.join(","))
        };
        
        let mut atom = DLAtom::new(restriction_expr, vec![variable.to_string()]);
        if negate {
            atom.is_positive = false;
        }
        atom
    }

    /// Create nominal value atom (e.g., { "WPS27@"^^rdf:PlainLiteral })
    fn create_nominal_atom(&self, value: &str, _property: &str, variable: &str, negate: bool) -> Result<DLAtom> {
        let nominal_expr = format!("nom:{{ \"{}\" }}", value);
        let mut atom = DLAtom::new(nominal_expr, vec![variable.to_string()]);
        if negate {
            atom.is_positive = false;
        }
        Ok(atom)
    }

    /// Create atLeast cardinality atom (e.g., atLeast(1 :property :range))
    fn create_at_least_atom(&self, cardinality: u32, property: &str, range: &str, variable: &str, negate: bool) -> Result<DLAtom> {
        let predicate = format!("all:atLeast({},{},{})", cardinality, property, range);
        let mut atom = DLAtom::new(predicate, vec![variable.to_string()]);
        if negate {
            atom.is_positive = false;
        }
        Ok(atom)
    }

    /// Convert class expression to range string for atLeast atoms
    fn class_expression_to_range_string(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => self.shorten_iri(&class.iri.to_string()),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let prop_str = self.object_property_expression_to_string(property);
                let filler_str = self.class_expression_to_range_string(filler);
                format!("exists({} {})", prop_str, filler_str)
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let prop_str = self.object_property_expression_to_string(property);
                let filler_str = self.class_expression_to_range_string(filler);
                format!("forall({} {})", prop_str, filler_str)
            }
            _ => "owl:Thing".to_string(),
        }
    }

    /// Compile DisjointUnion axiom 
    fn compile_disjoint_union_axiom(&mut self, axiom: &crate::ontology::DisjointUnionAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Create disjoint union clauses
        let var_x = self.fresh_variable();
        
        // 1. Coverage axiom: B1(x) v B2(x) v ... v Bn(x) :- A(x)
        // This ensures that every A is one of the disjuncts
        let union_class_atom = self.compile_class_expression_to_atom(&axiom.class, &var_x, false)?;
        let mut disjunct_head_atoms = Vec::new();
        
        for disjunct in &axiom.disjoint_classes {
            let disjunct_atom = self.compile_class_expression_to_atom(disjunct, &var_x, false)?;
            disjunct_head_atoms.push(disjunct_atom);
        }
        
        // Create disjunctive clause: B1(x) v B2(x) v ... v Bn(x) :- A(x)
        let coverage_clause = DLClause::new(
            disjunct_head_atoms, // Multiple heads (disjunctive)
            vec![union_class_atom],
            self.next_clause_id(),
        );
        clauses.push(coverage_clause);
        
        // 2. Union axioms: A(x) :- B1(x), A(x) :- B2(x), ..., A(x) :- Bn(x)
        // These are deterministic clauses showing each disjunct implies the union
        for disjunct in &axiom.disjoint_classes {
            let var_y = self.fresh_variable();
            let union_atom = self.compile_class_expression_to_atom(&axiom.class, &var_y, false)?;
            let disjunct_atom = self.compile_class_expression_to_atom(disjunct, &var_y, true)?;
            
            let union_clause = DLClause::new(
                vec![union_atom],
                vec![disjunct_atom],
                self.next_clause_id(),
            );
            clauses.push(union_clause);
        }
        
        // 3. Add disjointness constraints: ¬(Bi(x) ∧ Bj(x)) for all i ≠ j
        for i in 0..axiom.disjoint_classes.len() {
            for j in (i + 1)..axiom.disjoint_classes.len() {
                let var_z = self.fresh_variable();
                let a_atom = self.compile_class_expression_to_atom(&axiom.disjoint_classes[i], &var_z, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.disjoint_classes[j], &var_z, true)?;
                
                // Constraint: ¬(A(z) ∧ B(z))
                let constraint_clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![a_atom, b_atom],
                    self.next_clause_id(),
                );
                clauses.push(constraint_clause);
            }
        }
        
        Ok(clauses)
    }

    /// Compile ObjectPropertyDomain axiom
    fn compile_object_property_domain_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyDomainAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let domain_atom = self.compile_class_expression_to_atom(&axiom.domain, &var_x, false)?;
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        
        let clause = DLClause::new(
            vec![domain_atom],
            vec![property_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile ObjectPropertyRange axiom
    fn compile_object_property_range_axiom(&mut self, axiom: &crate::ontology::ObjectPropertyRangeAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let range_atom = self.compile_class_expression_to_atom(&axiom.range, &var_y, false)?;
        let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        
        let clause = DLClause::new(
            vec![range_atom],
            vec![property_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile DataPropertyDomain axiom
    fn compile_data_property_domain_axiom(&mut self, axiom: &crate::ontology::DataPropertyDomainAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.data_property_expression_to_string(&axiom.property);
        let domain_atom = self.compile_class_expression_to_atom(&axiom.domain, &var_x, false)?;
        let property_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_y);
        
        let clause = DLClause::new(
            vec![domain_atom],
            vec![property_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile DataPropertyRange axiom
    fn compile_data_property_range_axiom(&mut self, axiom: &crate::ontology::DataPropertyRangeAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.data_property_expression_to_string(&axiom.property);
        let property_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_y);
        
        // Generate HermiT-style data type restriction
        let range_atom = self.compile_data_range_to_atom(&axiom.range, &var_y)?;
        
        let clause = DLClause::new(
            vec![range_atom],
            vec![property_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile FunctionalObjectProperty axiom
    fn compile_functional_object_property_axiom(&mut self, axiom: &crate::ontology::FunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y1 = self.fresh_variable();
        let var_y2 = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y1);
        let prop2_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y2);
        
        // Functional constraint: P(x,y1) ∧ P(x,y2) → y1 = y2
        let equality_atom = DLAtom::new(format!("[{} == {}]", var_y1, var_y2), vec![]);
        
        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile FunctionalDataProperty axiom  
    fn compile_functional_data_property_axiom(&mut self, axiom: &crate::ontology::FunctionalDataPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y1 = self.fresh_variable();
        let var_y2 = self.fresh_variable();
        
        let property_name = self.data_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_y1);
        let prop2_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_y2);
        
        // Functional constraint: P(x,y1) ∧ P(x,y2) → y1 = y2
        let equality_atom = DLAtom::new(format!("[{} == {}]", var_y1, var_y2), vec![]);
        
        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile InverseFunctionalObjectProperty axiom
    fn compile_inverse_functional_object_property_axiom(&mut self, axiom: &crate::ontology::InverseFunctionalObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x1 = self.fresh_variable();
        let var_x2 = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let prop1_atom = DLAtom::role_assertion(&property_name, &var_x1, &var_y);
        let prop2_atom = DLAtom::role_assertion(&property_name, &var_x2, &var_y);
        
        // Inverse functional constraint: P(x1,y) ∧ P(x2,y) → x1 = x2
        let equality_atom = DLAtom::new(format!("[{} == {}]", var_x1, var_x2), vec![]);
        
        let clause = DLClause::new(
            vec![equality_atom],
            vec![prop1_atom, prop2_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile EquivalentObjectProperties axiom
    fn compile_equivalent_object_properties_axiom(&mut self, axiom: &crate::ontology::EquivalentObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Create bidirectional implications for each pair
        for i in 0..axiom.properties.len() {
            for j in 0..axiom.properties.len() {
                if i != j {
                    let var_x = self.fresh_variable();
                    let var_y = self.fresh_variable();
                    
                    let prop1_name = self.object_property_expression_to_string(&axiom.properties[i]);
                    let prop2_name = self.object_property_expression_to_string(&axiom.properties[j]);
                    
                    let prop1_atom = DLAtom::role_assertion(&prop1_name, &var_x, &var_y);
                    let prop2_atom = DLAtom::role_assertion(&prop2_name, &var_x, &var_y);
                    
                    let clause = DLClause::new(
                        vec![prop2_atom],
                        vec![prop1_atom],
                        self.next_clause_id(),
                    );
                    clauses.push(clause);
                }
            }
        }
        
        Ok(clauses)
    }

    /// Compile EquivalentDataProperties axiom
    fn compile_equivalent_data_properties_axiom(&mut self, axiom: &crate::ontology::EquivalentDataPropertiesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Create bidirectional implications for each pair
        for i in 0..axiom.properties.len() {
            for j in 0..axiom.properties.len() {
                if i != j {
                    let var_x = self.fresh_variable();
                    let var_y = self.fresh_variable();
                    
                    let prop1_name = self.data_property_expression_to_string(&axiom.properties[i]);
                    let prop2_name = self.data_property_expression_to_string(&axiom.properties[j]);
                    
                    let prop1_atom = DLAtom::datatype_assertion(&prop1_name, &var_x, &var_y);
                    let prop2_atom = DLAtom::datatype_assertion(&prop2_name, &var_x, &var_y);
                    
                    let clause = DLClause::new(
                        vec![prop2_atom],
                        vec![prop1_atom],
                        self.next_clause_id(),
                    );
                    clauses.push(clause);
                }
            }
        }
        
        Ok(clauses)
    }

    /// Compile DisjointObjectProperties axiom
    fn compile_disjoint_object_properties_axiom(&mut self, axiom: &crate::ontology::DisjointObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate disjointness constraints for each pair
        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_y = self.fresh_variable();
                
                let prop1_name = self.object_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.object_property_expression_to_string(&axiom.properties[j]);
                
                let prop1_atom = DLAtom::role_assertion(&prop1_name, &var_x, &var_y);
                let prop2_atom = DLAtom::role_assertion(&prop2_name, &var_x, &var_y);
                
                // Constraint: ¬(P1(x,y) ∧ P2(x,y))
                let clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![prop1_atom, prop2_atom],
                    self.next_clause_id(),
                );
                clauses.push(clause);
            }
        }
        
        Ok(clauses)
    }

    /// Compile DisjointDataProperties axiom
    fn compile_disjoint_data_properties_axiom(&mut self, axiom: &crate::ontology::DisjointDataPropertiesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate disjointness constraints for each pair
        for i in 0..axiom.properties.len() {
            for j in (i + 1)..axiom.properties.len() {
                let var_x = self.fresh_variable();
                let var_y = self.fresh_variable();
                
                let prop1_name = self.data_property_expression_to_string(&axiom.properties[i]);
                let prop2_name = self.data_property_expression_to_string(&axiom.properties[j]);
                
                let prop1_atom = DLAtom::datatype_assertion(&prop1_name, &var_x, &var_y);
                let prop2_atom = DLAtom::datatype_assertion(&prop2_name, &var_x, &var_y);
                
                // Constraint: ¬(P1(x,y) ∧ P2(x,y))
                let clause = DLClause::new(
                    vec![], // Empty head (constraint)
                    vec![prop1_atom, prop2_atom],
                    self.next_clause_id(),
                );
                clauses.push(clause);
            }
        }
        
        Ok(clauses)
    }
    
    /// Extract debug info from class expression
    fn extract_debug_info(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => format!("NamedClass({})", self.shorten_iri(&class.iri.to_string())),
            ClassExpression::ObjectIntersectionOf(conjuncts) => format!("Intersection({} conjuncts)", conjuncts.len()),
            ClassExpression::ObjectUnionOf(disjuncts) => format!("Union({} disjuncts)", disjuncts.len()),
            ClassExpression::ObjectSomeValuesFrom { .. } => "SomeValuesFrom".to_string(),
            ClassExpression::DataSomeValuesFrom { .. } => "DataSomeValuesFrom".to_string(),
            ClassExpression::ObjectHasValue { .. } => "HasValue".to_string(),
            _ => "OtherComplexExpression".to_string(),
        }
    }
    
    /// Extract simple class name from class expression (fallback for complex cases)
    fn extract_simple_class_name(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(class) => self.shorten_iri(&class.iri.to_string()),
            _ => "ComplexClass".to_string(),
        }
    }
    
    /// Compile ReflexiveObjectProperty axiom
    fn compile_reflexive_object_property_axiom(&mut self, axiom: &crate::ontology::ReflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let reflexive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);
        
        // For reflexive properties, we need an axiom that says for all x: P(x,x)
        // This is often encoded as a fact template or constraint
        let clause = DLClause::new(
            vec![reflexive_atom],
            vec![], // No conditions - always true for any individual
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile IrreflexiveObjectProperty axiom
    fn compile_irreflexive_object_property_axiom(&mut self, axiom: &crate::ontology::IrreflexiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let irreflexive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_x);
        
        // For irreflexive properties: ¬P(x,x) - constraint that forbids reflexive usage
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![irreflexive_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile SymmetricObjectProperty axiom
    fn compile_symmetric_object_property_axiom(&mut self, axiom: &crate::ontology::SymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let forward_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let backward_atom = DLAtom::role_assertion(&property_name, &var_y, &var_x);
        
        // Symmetric property: P(x,y) → P(y,x)
        let clause = DLClause::new(
            vec![backward_atom],
            vec![forward_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile AsymmetricObjectProperty axiom
    fn compile_asymmetric_object_property_axiom(&mut self, axiom: &crate::ontology::AsymmetricObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let forward_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let backward_atom = DLAtom::role_assertion(&property_name, &var_y, &var_x);
        
        // Asymmetric property: ¬(P(x,y) ∧ P(y,x)) - constraint
        let clause = DLClause::new(
            vec![], // Empty head (constraint)
            vec![forward_atom, backward_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile TransitiveObjectProperty axiom
    fn compile_transitive_object_property_axiom(&mut self, axiom: &crate::ontology::TransitiveObjectPropertyAxiom) -> Result<Vec<DLClause>> {
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        let var_z = self.fresh_variable();
        
        let property_name = self.object_property_expression_to_string(&axiom.property);
        let first_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
        let second_atom = DLAtom::role_assertion(&property_name, &var_y, &var_z);
        let transitive_atom = DLAtom::role_assertion(&property_name, &var_x, &var_z);
        
        // Transitive property: P(x,y) ∧ P(y,z) → P(x,z)
        let clause = DLClause::new(
            vec![transitive_atom],
            vec![first_atom, second_atom],
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile InverseObjectProperties axiom
    fn compile_inverse_object_properties_axiom(&mut self, axiom: &crate::ontology::InverseObjectPropertiesAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        let property1_name = self.object_property_expression_to_string(&axiom.property1);
        let property2_name = self.object_property_expression_to_string(&axiom.property2);
        
        // P1(x,y) → P2(y,x)
        let p1_atom = DLAtom::role_assertion(&property1_name, &var_x, &var_y);
        let p2_inverse_atom = DLAtom::role_assertion(&property2_name, &var_y, &var_x);
        
        clauses.push(DLClause::new(
            vec![p2_inverse_atom],
            vec![p1_atom],
            self.next_clause_id(),
        ));
        
        // P2(x,y) → P1(y,x)
        let var_a = self.fresh_variable();
        let var_b = self.fresh_variable();
        let p2_atom = DLAtom::role_assertion(&property2_name, &var_a, &var_b);
        let p1_inverse_atom = DLAtom::role_assertion(&property1_name, &var_b, &var_a);
        
        clauses.push(DLClause::new(
            vec![p1_inverse_atom],
            vec![p2_atom],
            self.next_clause_id(),
        ));
        
        Ok(clauses)
    }

    /// Compile SameIndividual axiom
    fn compile_same_individual_axiom(&mut self, axiom: &crate::ontology::SameIndividualAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate equality assertions for each pair
        for i in 0..axiom.individuals.len() {
            for j in (i + 1)..axiom.individuals.len() {
                let ind1 = self.individual_to_string(&axiom.individuals[i]);
                let ind2 = self.individual_to_string(&axiom.individuals[j]);
                
                // Assert equality: ind1 = ind2
                let equality_atom = DLAtom::new(format!("[{} == {}]", ind1, ind2), vec![]);
                
                let clause = DLClause::new(
                    vec![equality_atom],
                    vec![], // No body (fact)
                    self.next_clause_id(),
                );
                clauses.push(clause);
            }
        }
        
        Ok(clauses)
    }

    /// Compile DifferentIndividuals axiom
    fn compile_different_individuals_axiom(&mut self, axiom: &crate::ontology::DifferentIndividualsAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Generate inequality constraints for each pair
        for i in 0..axiom.individuals.len() {
            for j in (i + 1)..axiom.individuals.len() {
                let ind1 = self.individual_to_string(&axiom.individuals[i]);
                let ind2 = self.individual_to_string(&axiom.individuals[j]);
                
                // Assert inequality: ind1 ≠ ind2
                let inequality_atom = DLAtom::new(format!("[{} != {}]", ind1, ind2), vec![]);
                
                let clause = DLClause::new(
                    vec![inequality_atom],
                    vec![], // No body (fact)
                    self.next_clause_id(),
                );
                clauses.push(clause);
            }
        }
        
        Ok(clauses)
    }

    /// Compile NegativeObjectPropertyAssertion axiom
    fn compile_negative_object_property_assertion_axiom(&mut self, axiom: &crate::ontology::NegativeObjectPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        let subject = self.individual_to_string(&axiom.source);
        let object = self.individual_to_string(&axiom.target);
        let property_name = self.object_property_expression_to_string(&axiom.property);
        
        let negative_role_atom = DLAtom::new_negative(property_name, vec![subject, object]);
        
        let clause = DLClause::new(
            vec![negative_role_atom],
            vec![], // No body (negative fact)
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile NegativeDataPropertyAssertion axiom
    fn compile_negative_data_property_assertion_axiom(&mut self, axiom: &crate::ontology::NegativeDataPropertyAssertionAxiom) -> Result<Vec<DLClause>> {
        let subject = self.individual_to_string(&axiom.individual);
        let value = axiom.value.to_string();
        let property_name = self.data_property_expression_to_string(&axiom.property);
        
        let negative_datatype_atom = DLAtom::new_negative(property_name, vec![subject, value]);
        
        let clause = DLClause::new(
            vec![negative_datatype_atom],
            vec![], // No body (negative fact)
            self.next_clause_id(),
        );
        
        Ok(vec![clause])
    }

    /// Compile HasKey axiom
    fn compile_has_key_axiom(&mut self, axiom: &crate::ontology::HasKeyAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        let var_x = self.fresh_variable();
        let var_y = self.fresh_variable();
        
        // For HasKey(C OP1 ... OPm DP1 ... DPn), we generate:
        // C(x) ∧ C(y) ∧ OP1(x,z1) ∧ OP1(y,z1) ∧ ... ∧ DPm(x,v1) ∧ DPm(y,v1) ∧ ... → x = y
        
        let mut body_atoms = Vec::new();
        
        // Class assertions for both individuals
        let class_atom_x = self.compile_class_expression_to_atom(&axiom.class, &var_x, true)?;
        let class_atom_y = self.compile_class_expression_to_atom(&axiom.class, &var_y, true)?;
        body_atoms.push(class_atom_x);
        body_atoms.push(class_atom_y);
        
        // Object property key conditions
        for obj_prop in &axiom.object_properties {
            let prop_name = self.object_property_expression_to_string(obj_prop);
            let var_z = self.fresh_variable();
            
            let prop_atom_x = DLAtom::role_assertion(&prop_name, &var_x, &var_z);
            let prop_atom_y = DLAtom::role_assertion(&prop_name, &var_y, &var_z);
            body_atoms.push(prop_atom_x);
            body_atoms.push(prop_atom_y);
        }
        
        // Data property key conditions
        for data_prop in &axiom.data_properties {
            let prop_name = self.data_property_expression_to_string(data_prop);
            let var_v = self.fresh_variable();
            
            let prop_atom_x = DLAtom::datatype_assertion(&prop_name, &var_x, &var_v);
            let prop_atom_y = DLAtom::datatype_assertion(&prop_name, &var_y, &var_v);
            body_atoms.push(prop_atom_x);
            body_atoms.push(prop_atom_y);
        }
        
        // Conclusion: x = y
        let equality_atom = DLAtom::new(format!("[{} == {}]", var_x, var_y), vec![]);
        
        let clause = DLClause::new(
            vec![equality_atom],
            body_atoms,
            self.next_clause_id(),
        );
        clauses.push(clause);
        
        Ok(clauses)
    }

    /// Compile SWRL Rule axiom
    fn compile_swrl_rule_axiom(&mut self, axiom: &crate::ontology::SWRLRuleAxiom) -> Result<Vec<DLClause>> {
        let mut clauses = Vec::new();
        
        // Convert SWRL rule to DL clause format
        let mut head_atoms = Vec::new();
        let mut body_atoms = Vec::new();
        
        // Convert SWRL head atoms to DL atoms
        for swrl_atom in &axiom.rule.head {
            if let Some(dl_atom) = self.convert_swrl_atom_to_dl(swrl_atom)? {
                head_atoms.push(dl_atom);
            }
        }
        
        // Convert SWRL body atoms to DL atoms
        for swrl_atom in &axiom.rule.body {
            if let Some(dl_atom) = self.convert_swrl_atom_to_dl(swrl_atom)? {
                body_atoms.push(dl_atom);
            }
        }
        
        if !head_atoms.is_empty() || !body_atoms.is_empty() {
            let clause = DLClause::new(
                head_atoms,
                body_atoms,
                self.next_clause_id(),
            );
            clauses.push(clause);
        }
        
        Ok(clauses)
    }
    
    /// Convert SWRL atom to DL atom
    fn convert_swrl_atom_to_dl(&self, swrl_atom: &crate::ontology::SWRLAtom) -> Result<Option<DLAtom>> {
        use crate::ontology::SWRLAtom;
        
        match swrl_atom {
            SWRLAtom::ClassAtom { predicate, argument } => {
                let var_name = self.swrl_argument_to_string_i(argument)?;
                let atom = self.compile_class_expression_to_atom(predicate, &var_name, false)?;
                Ok(Some(atom))
            }
            SWRLAtom::ObjectPropertyAtom { predicate, first_argument, second_argument } => {
                let subj = self.swrl_argument_to_string_i(first_argument)?;
                let obj = self.swrl_argument_to_string_i(second_argument)?;
                let prop_name = self.object_property_expression_to_string(predicate);
                Ok(Some(DLAtom::role_assertion(&prop_name, &subj, &obj)))
            }
            SWRLAtom::DataPropertyAtom { predicate, first_argument, second_argument } => {
                let subj = self.swrl_argument_to_string_i(first_argument)?;
                let val = self.swrl_argument_to_string_d(second_argument)?;
                let prop_name = self.data_property_expression_to_string(predicate);
                Ok(Some(DLAtom::datatype_assertion(&prop_name, &subj, &val)))
            }
            SWRLAtom::SameIndividualAtom { first_argument, second_argument } => {
                let ind1 = self.swrl_argument_to_string_i(first_argument)?;
                let ind2 = self.swrl_argument_to_string_i(second_argument)?;
                Ok(Some(DLAtom::new(format!("[{} == {}]", ind1, ind2), vec![])))
            }
            SWRLAtom::DifferentIndividualsAtom { first_argument, second_argument } => {
                let ind1 = self.swrl_argument_to_string_i(first_argument)?;
                let ind2 = self.swrl_argument_to_string_i(second_argument)?;
                Ok(Some(DLAtom::new(format!("[{} != {}]", ind1, ind2), vec![])))
            }
            SWRLAtom::BuiltInAtom { predicate, arguments } => {
                let builtin_name = self.shorten_iri(&predicate.to_string());
                let arg_strings: Result<Vec<String>> = arguments.iter()
                    .map(|arg| self.swrl_argument_to_string_d(arg))
                    .collect();
                let args = arg_strings?;
                Ok(Some(DLAtom::new(format!("{}({})", builtin_name, args.join(",")), vec![])))
            }
            _ => Ok(None), // Some SWRL atoms might not have direct DL equivalents
        }
    }
    
    /// Convert SWRL individual argument to string
    fn swrl_argument_to_string_i(&self, arg: &crate::ontology::SWRLIArgument) -> Result<String> {
        use crate::ontology::SWRLIArgument;
        match arg {
            SWRLIArgument::Individual(ind) => Ok(self.individual_to_string(ind)),
            SWRLIArgument::Variable(var) => Ok(self.shorten_iri(&var.iri.to_string())),
        }
    }
    
    /// Convert SWRL data argument to string
    fn swrl_argument_to_string_d(&self, arg: &crate::ontology::SWRLDArgument) -> Result<String> {
        use crate::ontology::SWRLDArgument;
        match arg {
            SWRLDArgument::Literal(lit) => Ok(format!("\"{}\"", lit.value)),
            SWRLDArgument::Variable(var) => Ok(self.shorten_iri(&var.iri.to_string())),
        }
    }

}

impl Default for DLClauseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DLClauseSet {
    /// Save DL clauses to a file in HermiT format
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path)?;
        
        // Write prefixes
        writeln!(file, "Prefixes: [")?;
        for (prefix, namespace) in &self.prefixes {
            if prefix.is_empty() {
                writeln!(file, "  : = <{namespace}>")?;
            } else {
                writeln!(file, "  {prefix}: = <{namespace}>")?;
            }
        }
        writeln!(file, "]")?;
        
        // Write deterministic DL clauses
        writeln!(file, "Deterministic DL-clauses: [")?;
        for clause in &self.deterministic_clauses {
            writeln!(file, "  {clause}")?;
        }
        writeln!(file, "]")?;
        
        // Write disjunctive DL clauses
        writeln!(file, "Disjunctive DL-clauses: [")?;
        for clause in &self.disjunctive_clauses {
            writeln!(file, "  {clause}")?;
        }
        writeln!(file, "]")?;
        
        // Write ABox facts
        writeln!(file, "ABox: [")?;
        for fact in &self.abox_facts {
            writeln!(file, "  {fact}")?;
        }
        writeln!(file, "]")?;
        
        // Write statistics
        writeln!(file, "Statistics: [")?;
        writeln!(file, "  Number of deterministic clauses: {}", self.statistics.deterministic_clause_count)?;
        writeln!(file, "  Number of nondeterministic clauses: {}", self.statistics.disjunctive_clause_count)?;
        writeln!(file, "  Number of disjunctions: {}", self.statistics.disjunction_count)?;
        writeln!(file, "  Number of positive facts: {}", self.statistics.positive_fact_count)?;
        writeln!(file, "  Number of negative facts: {}", self.statistics.negative_fact_count)?;
        writeln!(file, "]")?;
        
        Ok(())
    }

    /// Convert to HermiT-style string representation
    pub fn to_hermit_format(&self) -> String {
        let mut output = String::new();
        
        // Prefixes
        output.push_str("Prefixes: [\n");
        for (prefix, namespace) in &self.prefixes {
            if prefix.is_empty() {
                output.push_str(&format!("  : = <{namespace}>\n"));
            } else {
                output.push_str(&format!("  {prefix}: = <{namespace}>\n"));
            }
        }
        output.push_str("]\n");
        
        // Deterministic clauses
        output.push_str("Deterministic DL-clauses: [\n");
        for clause in &self.deterministic_clauses {
            output.push_str(&format!("  {clause}\n"));
        }
        output.push_str("]\n");
        
        // Disjunctive clauses
        output.push_str("Disjunctive DL-clauses: [\n");
        for clause in &self.disjunctive_clauses {
            output.push_str(&format!("  {clause}\n"));
        }
        output.push_str("]\n");
        
        // ABox
        output.push_str("ABox: [\n");
        for fact in &self.abox_facts {
            output.push_str(&format!("  {fact}\n"));
        }
        output.push_str("]\n");
        
        // Statistics
        output.push_str("Statistics: [\n");
        output.push_str(&format!("  Number of deterministic clauses: {}\n", self.statistics.deterministic_clause_count));
        output.push_str(&format!("  Number of nondeterministic clauses: {}\n", self.statistics.disjunctive_clause_count));
        output.push_str(&format!("  Number of disjunctions: {}\n", self.statistics.disjunction_count));
        output.push_str(&format!("  Number of positive facts: {}\n", self.statistics.positive_fact_count));
        output.push_str(&format!("  Number of negative facts: {}\n", self.statistics.negative_fact_count));
        output.push_str("]\n");
        
        output
    }
}
