//! DL Clause Generation and Dumping
//!
//! This module implements DL clause generation from OWL ontologies,
//! similar to HermiT's clause dumping functionality.

use crate::{
    Error, Result,
    ontology::{
        Axiom, ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
        Ontology, OntologyRef,
    },
};
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
        
        Self {
            variable_counter: 0,
            clause_counter: 0,
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

        // Process each axiom
        for axiom in ontology.axioms() {
            let clauses = self.compile_axiom(axiom)?;
            
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
            Axiom::EquivalentObjectProperties(axiom) => self.compile_equivalent_object_properties_axiom(axiom),
            Axiom::EquivalentDataProperties(axiom) => self.compile_equivalent_data_properties_axiom(axiom),
            Axiom::DisjointObjectProperties(axiom) => self.compile_disjoint_object_properties_axiom(axiom),
            Axiom::DisjointDataProperties(axiom) => self.compile_disjoint_data_properties_axiom(axiom),
            _ => {
                // For unsupported axiom types, return empty clause set for now
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
            
            // Generate disjunctive clause for complex definition
            if let Some(disjunctive_clause) = self.compile_complex_definition(named_class, complex_expr)? {
                clauses.push(disjunctive_clause);
            }
            
            // Also generate standard implications
            clauses.extend(self.compile_equivalent_classes_standard(axiom)?);
        } else {
            // Generate bidirectional implications for each pair (standard case)
            clauses.extend(self.compile_equivalent_classes_standard(axiom)?);
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
    /// For A ≡ B ⊓ C ⊓ ..., generate: A(x) ∨ ¬B(x) ∨ ¬C(x) ∨ ... :- [body conditions]
    fn compile_complex_definition(&mut self, named_class: &ClassExpression, complex_expr: &ClassExpression) -> Result<Option<DLClause>> {
        match complex_expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                let var_x = self.fresh_variable();
                
                // Create head: NamedClass(x) ∨ ¬Conjunct1(x) ∨ ¬Conjunct2(x) ∨ ...
                let mut head_atoms = Vec::new();
                let mut body_atoms = Vec::new();
                
                // Add positive named class atom to head
                let named_atom = self.compile_class_expression_to_atom(named_class, &var_x, false)?;
                head_atoms.push(named_atom);
                
                // Process each conjunct
                for conjunct in conjuncts {
                    match conjunct {
                        ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                            // Handle ∃property.range
                            let property_name = self.object_property_expression_to_string(property);
                            let filler_name = self.extract_simple_class_name(filler);
                            
                            // Add negative restriction to head: ¬(∃property.range)(x)
                            let neg_restriction_atom = DLAtom::new_negative(
                                format!("some({},{})", property_name, filler_name),
                                vec![var_x.clone()]
                            );
                            head_atoms.push(neg_restriction_atom);
                            
                            // Add positive property assertions to body
                            let var_y = self.fresh_variable();
                            let property_atom = DLAtom::role_assertion(&property_name, &var_x, &var_y);
                            let filler_atom = DLAtom::concept_assertion(&filler_name, &var_y);
                            body_atoms.push(property_atom);
                            body_atoms.push(filler_atom);
                        }
                        ClassExpression::DataSomeValuesFrom { property, filler } => {
                            // Handle ∃dataProperty.dataRange
                            let property_name = self.data_property_expression_to_string(property);
                            let range_name = self.data_range_to_string(filler);
                            
                            let neg_restriction_atom = DLAtom::new_negative(
                                format!("dataSome({},{})", property_name, range_name),
                                vec![var_x.clone()]
                            );
                            head_atoms.push(neg_restriction_atom);
                            
                            let var_y = self.fresh_variable();
                            let property_atom = DLAtom::datatype_assertion(&property_name, &var_x, &var_y);
                            let range_atom = DLAtom::concept_assertion(&range_name, &var_y);
                            body_atoms.push(property_atom);
                            body_atoms.push(range_atom);
                        }
                        ClassExpression::ObjectHasValue { property, value } => {
                            // Handle ∃property.{individual}
                            let property_name = self.object_property_expression_to_string(property);
                            let individual_name = self.individual_to_string(value);
                            
                            let neg_restriction_atom = DLAtom::new_negative(
                                format!("hasValue({},{})", property_name, individual_name),
                                vec![var_x.clone()]
                            );
                            head_atoms.push(neg_restriction_atom);
                            
                            let property_atom = DLAtom::role_assertion(&property_name, &var_x, &individual_name);
                            body_atoms.push(property_atom);
                        }
                        _ => {
                            // For other types, add negative atom to head
                            let neg_conjunct_atom = self.compile_class_expression_to_atom(conjunct, &var_x, true)?;
                            head_atoms.push(neg_conjunct_atom);
                        }
                    }
                }
                
                if head_atoms.len() > 1 || !body_atoms.is_empty() {
                    Ok(Some(DLClause::new(
                        head_atoms,
                        body_atoms,
                        self.next_clause_id(),
                    )))
                } else {
                    Ok(None)
                }
            }
            _ => {
                // For non-intersection complex expressions, don't generate disjunctive clauses yet
                Ok(None)
            }
        }
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
            ClassExpression::ObjectUnionOf(disjuncts) => {
                // For union classes, we need to create a complex representation
                // For now, create a placeholder representation
                let predicate = format!("Union_{}", disjuncts.len());
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                // For intersection classes, we need to create a complex representation
                let predicate = format!("Intersection_{}", conjuncts.len());
                let mut atom = DLAtom::concept_assertion(&predicate, variable);
                if negate {
                    atom.is_positive = false;
                }
                Ok(atom)
            }
            _ => {
                // For other complex expressions, create a placeholder
                let predicate = "ComplexExpression".to_string();
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
        
        // Create a datatype constraint atom
        let range_name = self.data_range_to_string(&axiom.range);
        let range_atom = DLAtom::new(range_name, vec![var_y]);
        
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
