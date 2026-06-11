//! Basic reasoning tasks
//!
//! This module implements fundamental reasoning operations like consistency checking,
//! satisfiability testing, and subsumption checking.

use crate::{
    Error, Result,
    cache::CacheManager,
    core::{
        lock_helpers::read_lock,
        reasoner::{
            statistics::ReasoningStatistics,
            tableau::{TableauAlgorithmInstance, TableauFactory},
        },
    },
    ontology::{ClassExpression, Individual, OntologyRef},
};
use log::{debug, info};
use std::time::Instant;

/// Service for basic reasoning operations (consistency, satisfiability, subsumption)
#[derive(Debug)]
pub struct ReasoningTaskService {
    pub tableau_factory: TableauFactory,
}

impl ReasoningTaskService {
    /// Create a new reasoning task service
    pub fn new(tableau_factory: TableauFactory) -> Self {
        Self { tableau_factory }
    }

    /// Check if the ontology is consistent
    pub fn check_consistency(
        &self,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_consistency_checks();

        info!("Checking ontology consistency");

        // Check cache first
        if let Some(cached_result) = cache.get_consistency_result(ontology) {
            debug!("Consistency result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = read_lock(ontology, "tasks: reading ontology for consistency check")?;

        // Build tableau for consistency checking
        let tableau = self
            .tableau_factory
            .create_algorithm_instance(&ontology_guard)?;

        // Run tableau algorithm
        let result = self.run_tableau_consistency_check(tableau, statistics)?;

        // Cache the result
        cache.cache_consistency_result(ontology, result);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Consistency check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if a class is satisfiable
    pub fn check_satisfiability(
        &self,
        class_iri: &str,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_satisfiability_checks();

        info!("Checking satisfiability of class: {class_iri}");

        // Handle special OWL classes
        if class_iri.contains("owl#Thing") {
            return Ok(true); // owl:Thing is always satisfiable
        }
        if class_iri.contains("owl#Nothing") {
            return Ok(false); // owl:Nothing is always unsatisfiable
        }

        // Check cache first
        if let Some(class_expr) = self.parse_class_expression(class_iri)
            && let Some(cached_result) = cache.get_satisfiability_result(&class_expr)
        {
            debug!("Satisfiability result found in cache for: {class_iri}");
            return Ok(cached_result);
        }

        let ontology_guard =
            read_lock(ontology, "tasks: reading ontology for satisfiability check")?;

        // Build tableau for satisfiability checking
        let tableau = self
            .tableau_factory
            .create_algorithm_for_satisfiability(&ontology_guard, class_iri)?;

        // Run tableau algorithm
        let result = self.run_tableau_satisfiability_check(tableau, statistics)?;

        // Cache the result
        if let Some(class_expr) = self.parse_class_expression(class_iri) {
            cache.cache_satisfiability_result(class_expr, result);
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Satisfiability check for {class_iri} completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if one class subsumes another
    pub fn check_subsumption(
        &self,
        subclass: &str,
        superclass: &str,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_subsumption_checks();

        info!("Checking subsumption: {subclass} ⊑ {superclass}");

        // Check cache first
        if let (Some(sub_expr), Some(sup_expr)) = (
            self.parse_class_expression(subclass),
            self.parse_class_expression(superclass),
        ) && let Some(cached_result) = cache.get_subsumption_result(&sub_expr, &sup_expr)
        {
            debug!("Subsumption result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = read_lock(ontology, "tasks: reading ontology for subsumption check")?;

        // Build tableau for subsumption checking
        let tableau = self.tableau_factory.create_algorithm_for_subsumption(
            &ontology_guard,
            subclass,
            superclass,
        )?;

        // Run tableau algorithm
        let result = self.run_tableau_subsumption_check(tableau, statistics)?;

        // Cache the result
        if let (Some(sub_expr), Some(sup_expr)) = (
            self.parse_class_expression(subclass),
            self.parse_class_expression(superclass),
        ) {
            cache.cache_subsumption_result(sub_expr, sup_expr, result);
        }

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Subsumption check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check if an individual is an instance of a class
    pub fn check_instance(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();

        info!("Checking instance relationship");

        // Check cache first
        if let Some(cached_result) = cache.get_instance_result(individual, class_expr) {
            debug!("Instance result found in cache");
            return Ok(cached_result);
        }

        let ontology_guard = read_lock(ontology, "tasks: reading ontology for instance check")?;

        // Build tableau for instance checking
        let tableau = self.tableau_factory.create_for_instance_check(
            &ontology_guard,
            individual,
            class_expr,
        )?;

        // Run tableau algorithm
        let result = self.run_tableau_instance_check(tableau, statistics)?;

        // Cache the result
        cache.store_instance_result(individual.clone(), class_expr.clone(), result);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Instance check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Check subsumption between two class expressions
    pub fn check_subsumption_expressions(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();
        statistics.increment_subsumption_checks();

        // Check cache first
        if let Some(cached_result) = cache.get_subsumption_result(subclass, superclass) {
            return Ok(cached_result);
        }

        let ontology_guard = read_lock(
            ontology,
            "tasks: reading ontology for subsumption expressions check",
        )?;

        // For now, convert to strings and use existing tableau methods
        let subclass_str = format!("{subclass:?}");
        let superclass_str = format!("{superclass:?}");

        let tableau = self.tableau_factory.create_algorithm_for_subsumption(
            &ontology_guard,
            &subclass_str,
            &superclass_str,
        )?;

        let result = self.run_tableau_subsumption_check(tableau, statistics)?;

        // Store in cache
        cache.cache_subsumption_result(subclass.clone(), superclass.clone(), result);

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        Ok(result)
    }

    /// Check if an axiom is entailed by the ontology
    pub fn check_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        let start_time = Instant::now();

        info!("Checking axiom entailment");

        let ontology_guard = read_lock(ontology, "tasks: reading ontology for entailment check")?;

        let result = match axiom {
            crate::ontology::axioms::Axiom::SubClassOf(subclass_axiom) => {
                // Check if subclass ⊑ superclass is entailed
                let subclass_str = format!("{:?}", subclass_axiom.subclass);
                let superclass_str = format!("{:?}", subclass_axiom.superclass);
                self.check_subsumption(&subclass_str, &superclass_str, ontology, statistics, cache)?
            }
            crate::ontology::axioms::Axiom::ClassAssertion(class_assertion) => {
                // Check if individual ∈ class is entailed
                self.check_instance(
                    &class_assertion.individual,
                    &class_assertion.class,
                    ontology,
                    statistics,
                    cache,
                )?
            }
            _ => {
                // For other axiom types, check if they are explicitly present
                ontology_guard.axioms.contains(axiom)
            }
        };

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Entailment check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    // Private helper methods

    /// Run a tableau consistency check
    fn run_tableau_consistency_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau consistency check");

        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(true),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(false),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau satisfiability check
    fn run_tableau_satisfiability_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau satisfiability check");

        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(true),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(false),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau subsumption check
    fn run_tableau_subsumption_check(
        &self,
        mut tableau: TableauAlgorithmInstance,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau subsumption check");

        // For subsumption A ⊑ B, we check if A ⊓ ¬B is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(false), // A ⊓ ¬B is satisfiable, so A ⊄ B
            crate::core::tableau::TableauState::Unsatisfiable => Ok(true), // A ⊓ ¬B is unsatisfiable, so A ⊑ B
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Run a tableau instance check
    fn run_tableau_instance_check(
        &self,
        mut tableau: Box<dyn crate::core::reasoner::tableau::TableauRunner>,
        statistics: &mut ReasoningStatistics,
    ) -> Result<bool> {
        debug!("Running tableau instance check");

        // For instance checking a ∈ C, we check if {a} ⊓ ¬C is unsatisfiable
        let result = tableau.run()?;

        // Update statistics
        statistics.update_tableau_stats(
            tableau.get_node_count() as u64,
            tableau.get_backtrack_count() as u64,
            tableau.get_max_depth(),
        );

        match result {
            crate::core::tableau::TableauState::Satisfiable => Ok(false),
            crate::core::tableau::TableauState::Unsatisfiable => Ok(true),
            crate::core::tableau::TableauState::Unknown => {
                Err(Error::reasoning("Tableau returned unknown result"))
            }
        }
    }

    /// Parse a class IRI string into a `ClassExpression`
    ///
    /// Supports simplified Manchester-style syntax:
    /// - Named classes: `<http://example.org/Class>` or `prefix:ClassName`
    /// - Intersections: `Class1 and Class2` or `Class1 ⊓ Class2`
    /// - Unions: `Class1 or Class2` or `Class1 ⊔ Class2`
    /// - Complements: `not Class` or `¬Class`
    /// - Existential: `some property Class` or `∃property.Class`
    /// - Universal: `only property Class` or `∀property.Class`
    /// - Cardinality: `min 2 property Class`, `max 5 property Class`, `exactly 3 property Class`
    /// - owl:Thing and owl:Nothing as special classes
    fn parse_class_expression(&self, class_iri: &str) -> Option<ClassExpression> {
        Self::parse_class_expr_str(class_iri)
    }

    /// Parse a class expression from a string (static helper)
    fn parse_class_expr_str(class_iri: &str) -> Option<ClassExpression> {
        let trimmed = class_iri.trim();

        // Handle empty input
        if trimmed.is_empty() {
            return None;
        }

        // Handle special OWL classes
        if trimmed == "owl:Thing" || trimmed.ends_with("#Thing") || trimmed.ends_with("/Thing") {
            return Some(ClassExpression::Class(crate::ontology::Class {
                iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Thing"),
            }));
        }
        if trimmed == "owl:Nothing"
            || trimmed.ends_with("#Nothing")
            || trimmed.ends_with("/Nothing")
        {
            return Some(ClassExpression::Class(crate::ontology::Class {
                iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#Nothing"),
            }));
        }

        // Parse complex expressions
        // Priority order: and/or > not > restrictions > atomic

        // Check for intersection (and, ⊓)
        if let Some(parts) = Self::split_by_operator(trimmed, &["and", "⊓"])
            && parts.len() >= 2
        {
            let expressions: Vec<ClassExpression> = parts
                .iter()
                .filter_map(|p| Self::parse_class_expr_str(p))
                .collect();

            if expressions.len() == parts.len() {
                return Some(ClassExpression::ObjectIntersectionOf(expressions));
            }
        }

        // Check for union (or, ⊔)
        if let Some(parts) = Self::split_by_operator(trimmed, &["or", "⊔"])
            && parts.len() >= 2
        {
            let expressions: Vec<ClassExpression> = parts
                .iter()
                .filter_map(|p| Self::parse_class_expr_str(p))
                .collect();

            if expressions.len() == parts.len() {
                return Some(ClassExpression::ObjectUnionOf(expressions));
            }
        }

        // Check for complement (not, ¬)
        if trimmed.starts_with("not ")
            && let Some(inner) = Self::parse_class_expr_str(&trimmed[4..])
        {
            return Some(ClassExpression::ObjectComplementOf(Box::new(inner)));
        }
        if trimmed.starts_with('¬')
            && let Some(inner) = Self::parse_class_expr_str(&trimmed[3..])
        {
            // ¬ is 3 bytes in UTF-8
            return Some(ClassExpression::ObjectComplementOf(Box::new(inner)));
        }

        // Check for existential restriction (some, ∃)
        if trimmed.starts_with("some ") {
            return Self::parse_restriction(trimmed, "some", true);
        }
        if trimmed.starts_with('∃') {
            return Self::parse_restriction(trimmed, "∃", true);
        }

        // Check for universal restriction (only, ∀)
        if trimmed.starts_with("only ") {
            return Self::parse_restriction(trimmed, "only", false);
        }
        if trimmed.starts_with('∀') {
            return Self::parse_restriction(trimmed, "∀", false);
        }

        // Check for cardinality restrictions
        if trimmed.starts_with("min ")
            || trimmed.starts_with("max ")
            || trimmed.starts_with("exactly ")
        {
            return Self::parse_cardinality(trimmed);
        }

        // Default: treat as named class IRI
        // Handle angle brackets: <http://example.org/Class>
        let iri_str = if trimmed.starts_with('<') && trimmed.ends_with('>') {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        Some(ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::from(iri_str.to_string()),
        }))
    }

    /// Split a string by multiple operators, handling precedence
    fn split_by_operator(s: &str, operators: &[&str]) -> Option<Vec<String>> {
        for op in operators {
            let parts: Vec<&str> = s.split(op).collect();
            if parts.len() > 1 {
                return Some(parts.iter().map(|p| p.trim().to_string()).collect());
            }
        }
        None
    }

    /// Parse a property restriction (existential or universal)
    fn parse_restriction(s: &str, keyword: &str, is_existential: bool) -> Option<ClassExpression> {
        let after_keyword = if keyword == "∃" || keyword == "∀" {
            s[keyword.len()..].trim()
        } else {
            s[keyword.len() + 1..].trim() // +1 for space
        };

        // Format: "property.Class" or "property Class"
        let parts: Vec<&str> = if after_keyword.contains('.') {
            after_keyword.splitn(2, '.').collect()
        } else {
            after_keyword.splitn(2, ' ').collect()
        };

        if parts.len() == 2 {
            let property_iri = parts[0].trim();
            let filler_str = parts[1].trim();

            // Create property expression
            let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                crate::ontology::ObjectProperty {
                    iri: crate::ontology::IRI::from(property_iri.to_string()),
                },
            );

            // Parse filler recursively
            if let Some(filler) = Self::parse_class_expr_str(filler_str) {
                return if is_existential {
                    Some(ClassExpression::ObjectSomeValuesFrom {
                        property,
                        filler: Box::new(filler),
                    })
                } else {
                    Some(ClassExpression::ObjectAllValuesFrom {
                        property,
                        filler: Box::new(filler),
                    })
                };
            }
        }

        None
    }

    /// Parse cardinality restrictions
    fn parse_cardinality(s: &str) -> Option<ClassExpression> {
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.len() >= 3 {
            let (kind, rest_idx) = if parts[0] == "min" {
                ("min", 1)
            } else if parts[0] == "max" {
                ("max", 1)
            } else if parts[0] == "exactly" {
                ("exactly", 1)
            } else {
                return None;
            };

            // Parse cardinality number
            if let Ok(cardinality) = parts[rest_idx].parse::<u32>()
                && parts.len() > rest_idx + 1
            {
                let property_iri = parts[rest_idx + 1];
                let filler_str = parts[rest_idx + 2..].join(" ");

                let property = crate::ontology::ObjectPropertyExpression::ObjectProperty(
                    crate::ontology::ObjectProperty {
                        iri: crate::ontology::IRI::from(property_iri.to_string()),
                    },
                );

                // Parse filler recursively
                if let Some(filler) = Self::parse_class_expr_str(&filler_str) {
                    return match kind {
                        "min" => Some(ClassExpression::ObjectMinCardinality {
                            cardinality,
                            property,
                            filler: Box::new(filler),
                        }),
                        "max" => Some(ClassExpression::ObjectMaxCardinality {
                            cardinality,
                            property,
                            filler: Box::new(filler),
                        }),
                        "exactly" => Some(ClassExpression::ObjectExactCardinality {
                            cardinality,
                            property,
                            filler: Box::new(filler),
                        }),
                        _ => None,
                    };
                }
            }
        }

        None
    }
}
