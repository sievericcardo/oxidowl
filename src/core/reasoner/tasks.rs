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
        tableau::TableauState,
    },
    ontology::{
        Axiom, ClassAssertionAxiom, ClassExpression, DataPropertyAssertionAxiom,
        DataPropertyExpression, DifferentIndividualsAxiom, Individual,
        ObjectPropertyAssertionAxiom, ObjectPropertyExpression, OntologyRef, SameIndividualAxiom,
    },
};
use log::{debug, info};
use std::collections::HashSet;
use std::time::Instant;

/// Service for basic reasoning operations (consistency, satisfiability, subsumption)
#[derive(Debug)]
pub struct ReasoningTaskService {
    pub tableau_factory: TableauFactory,
}

impl ReasoningTaskService {
    /// Create a new reasoning task service
    #[must_use]
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

        // Build the tableau directly from typed class expressions (no string round-trip)
        let tableau =
            self.tableau_factory
                .create_for_subsumption(&ontology_guard, subclass, superclass)?;

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

        // Fast path: an axiom literally present in the ontology is entailed. This
        // is never the *sole* test — the semantic checks below run for every
        // axiom type even when the axiom is absent.
        if ontology_guard.axioms.contains(axiom) {
            return Ok(true);
        }
        drop(ontology_guard);

        let result = self.check_entailment_semantic(axiom, ontology, statistics, cache)?;

        let reasoning_time = start_time.elapsed();
        statistics.add_reasoning_time(reasoning_time);

        info!("Entailment check completed in {reasoning_time:?}: {result}");
        Ok(result)
    }

    /// Semantic entailment for every axiom type (workstream A7).
    ///
    /// Strategy per axiom type:
    /// - **Typed tableau reduction** (O ⊨ α iff O ⊓ ¬α is inconsistent): used for
    ///   `SubClassOf` / `ClassAssertion` (via the existing subsumption / instance
    ///   checks) and `EquivalentClasses` (mutual subsumption).
    /// - **Completed-tableau closure checks**: property-characteristic and
    ///   assertion entailments are answered by running the tableau over a
    ///   *witness-augmented* copy of the ontology and inspecting the materialised
    ///   closure. For `SubObjectPropertyOf`, `Transitive`, `Symmetric`,
    ///   `Reflexive`, `ObjectPropertyDomain/Range` the witness creates a fresh
    ///   edge/individual so the check is non-vacuous; the `RBox` pass (A5) then
    ///   materialises the required closure iff the axiom is entailed.
    /// - **Consistency reductions** (witness + contradiction): `Asymmetric`,
    ///   `Irreflexive`, `Functional`, `InverseFunctional`,
    ///   `DisjointObjectProperties`, `DifferentIndividuals` are answered by
    ///   adding witnesses that would clash iff the axiom is *not* entailed.
    /// - **Assertion closure**: `ObjectPropertyAssertion`,
    ///   `DataPropertyAssertion`, negative assertions and `SameIndividual` are
    ///   answered over the completed tableau's edge set / nominal merges.
    /// - **Data-property hierarchy closure**: `SubDataPropertyOf`,
    ///   `EquivalentDataProperties`, `DisjointDataProperties`,
    ///   `DataPropertyDomain/Range`, `FunctionalDataProperty` use the transitive
    ///   closure of the asserted data-property hierarchy (OWL 2 DL has no data
    ///   property chains, inverses or non-RBox role axioms, so the asserted
    ///   hierarchy closure is complete for these).
    /// - **`HasKey` / non-logical axioms**: presence-only (fast path).
    pub fn check_entailment_semantic(
        &self,
        axiom: &Axiom,
        ontology: &OntologyRef,
        statistics: &mut ReasoningStatistics,
        cache: &mut CacheManager,
    ) -> Result<bool> {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                // Check if subclass ⊑ superclass is entailed (typed path, no string round-trip)
                self.check_subsumption_expressions(
                    &subclass_axiom.subclass,
                    &subclass_axiom.superclass,
                    ontology,
                    statistics,
                    cache,
                )
            }
            Axiom::ClassAssertion(class_assertion) => {
                // Check if individual ∈ class is entailed
                self.check_instance(
                    &class_assertion.individual,
                    &class_assertion.class,
                    ontology,
                    statistics,
                    cache,
                )
            }
            Axiom::EquivalentClasses(eq) => {
                // O ⊨ EquivalentClasses([C1..Cn]) iff every pair is mutually subsumed.
                for i in 0..eq.classes.len() {
                    for j in (i + 1)..eq.classes.len() {
                        if !self.check_subsumption_expressions(
                            &eq.classes[i],
                            &eq.classes[j],
                            ontology,
                            statistics,
                            cache,
                        )? {
                            return Ok(false);
                        }
                        if !self.check_subsumption_expressions(
                            &eq.classes[j],
                            &eq.classes[i],
                            ontology,
                            statistics,
                            cache,
                        )? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            _ => self.check_property_axiom_entailment(axiom, ontology),
        }
    }

    /// Semantic entailment for property / individual / data axioms that cannot be
    /// reduced to the existing typed subsumption / instance checks.
    fn check_property_axiom_entailment(
        &self,
        axiom: &Axiom,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        match axiom {
            // ── Object property axioms ────────────────────────────────────
            Axiom::SubObjectPropertyOf(a) => {
                self.entails_sub_object_property(&a.sub_property, &a.super_property, ontology)
            }
            Axiom::EquivalentObjectProperties(a) => {
                // Mutual sub-property in both directions for every pair.
                for i in 0..a.properties.len() {
                    for j in (i + 1)..a.properties.len() {
                        if !self.entails_sub_object_property(
                            &a.properties[i],
                            &a.properties[j],
                            ontology,
                        )? {
                            return Ok(false);
                        }
                        if !self.entails_sub_object_property(
                            &a.properties[j],
                            &a.properties[i],
                            ontology,
                        )? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            Axiom::TransitiveObjectProperty(a) => self.entails_transitive(&a.property, ontology),
            Axiom::SymmetricObjectProperty(a) => self.entails_symmetric(&a.property, ontology),
            Axiom::ReflexiveObjectProperty(a) => self.entails_reflexive(&a.property, ontology),
            Axiom::AsymmetricObjectProperty(a) => self.entails_asymmetric(&a.property, ontology),
            Axiom::IrreflexiveObjectProperty(a) => self.entails_irreflexive(&a.property, ontology),
            Axiom::FunctionalObjectProperty(a) => self.entails_functional(&a.property, ontology),
            Axiom::InverseFunctionalObjectProperty(a) => {
                self.entails_inverse_functional(&a.property, ontology)
            }
            Axiom::DisjointObjectProperties(a) => {
                self.entails_disjoint_object_properties(&a.properties, ontology)
            }
            Axiom::InverseObjectProperties(a) => {
                self.entails_inverse_pair(&a.property1, &a.property2, ontology)
            }
            Axiom::ObjectPropertyDomain(a) => {
                self.entails_object_domain(&a.property, &a.domain, ontology)
            }
            Axiom::ObjectPropertyRange(a) => {
                self.entails_object_range(&a.property, &a.range, ontology)
            }
            // ── Data property axioms ──────────────────────────────────────
            Axiom::SubDataPropertyOf(a) => {
                self.entails_sub_data_property(&a.sub_property, &a.super_property, ontology)
            }
            Axiom::EquivalentDataProperties(a) => {
                for i in 0..a.properties.len() {
                    for j in (i + 1)..a.properties.len() {
                        if !self.entails_sub_data_property(
                            &a.properties[i],
                            &a.properties[j],
                            ontology,
                        )? {
                            return Ok(false);
                        }
                        if !self.entails_sub_data_property(
                            &a.properties[j],
                            &a.properties[i],
                            ontology,
                        )? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            Axiom::DisjointDataProperties(a) => {
                self.entails_disjoint_data_properties(&a.properties, ontology)
            }
            Axiom::DataPropertyDomain(a) => {
                self.entails_data_domain(&a.property, &a.domain, ontology)
            }
            Axiom::DataPropertyRange(a) => self.entails_data_range(&a.property, &a.range, ontology),
            Axiom::FunctionalDataProperty(a) => self.entails_functional_data(&a.property, ontology),
            // ── Individual axioms ─────────────────────────────────────────
            Axiom::SameIndividual(a) => self.entails_same_individual(&a.individuals, ontology),
            Axiom::DifferentIndividuals(a) => {
                self.entails_different_individuals(&a.individuals, ontology)
            }
            Axiom::ObjectPropertyAssertion(a) => {
                self.entails_object_property_assertion(a, ontology)
            }
            Axiom::DataPropertyAssertion(a) => self.entails_data_property_assertion(a, ontology),
            Axiom::NegativeObjectPropertyAssertion(a) => {
                self.entails_negative_object_property_assertion(a, ontology)
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                self.entails_negative_data_property_assertion(a, ontology)
            }
            // HasKey / annotations / declarations / SWRL / DatatypeDefinition:
            // no clean semantic closure test (HasKey's key-derived equalities are
            // already materialised by A5 and surfaced through SameIndividual).
            _ => Ok(false),
        }
    }

    // Private helper methods

    // ── Semantic entailment helpers (Workstream A7) ──────────────────────────

    /// Witness namespace for fresh individuals used in entailment checks. The
    /// namespace is deliberately out of the way so it cannot collide with real
    /// ontology individuals.
    const WITNESS_NS: &'static str = "http://www.oxidowl.org/a7/witness#";

    /// Fresh named witness individual for a tableau-entailment reduction.
    fn fresh_individual(&self) -> Individual {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        Individual::named(crate::ontology::IRI::new(&format!(
            "{}{n}",
            Self::WITNESS_NS
        )))
    }

    /// Run a completed tableau over the ontology (with optional witness axioms)
    /// and return the completed instance for inspection.
    fn run_witness_tableau(
        &self,
        ontology: &crate::ontology::Ontology,
        witnesses: &[Axiom],
    ) -> Result<TableauAlgorithmInstance> {
        let mut witness_ontology = ontology.clone();
        for witness in witnesses {
            witness_ontology.add_axiom(witness.clone());
        }
        let mut tableau = self
            .tableau_factory
            .create_algorithm_instance(&witness_ontology)?;
        let state = tableau.run()?;
        if state == TableauState::Unknown {
            return Err(Error::reasoning("Tableau returned unknown result"));
        }
        Ok(tableau)
    }

    /// Whether the ontology augmented with the witness axioms is inconsistent.
    fn is_inconsistent_with(
        &self,
        ontology: &crate::ontology::Ontology,
        witnesses: &[Axiom],
    ) -> Result<bool> {
        let tableau = self.run_witness_tableau(ontology, witnesses)?;
        Ok(!tableau.is_satisfiable())
    }

    /// Build an `ObjectPropertyAssertion` witness axiom.
    fn object_assertion(
        &self,
        property: ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    ) -> Axiom {
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: 0,
            property,
            source,
            target,
            annotations: Vec::new(),
        })
    }

    /// Build a `ClassAssertion` witness axiom.
    fn class_assertion(&self, individual: Individual, class: ClassExpression) -> Axiom {
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            class,
            individual,
            annotations: Vec::new(),
        })
    }

    /// Build a `SameIndividual` witness axiom.
    fn same_individual(&self, a: Individual, b: Individual) -> Axiom {
        Axiom::SameIndividual(SameIndividualAxiom {
            id: 0,
            individuals: vec![a, b],
            annotations: Vec::new(),
        })
    }

    /// Build a `DifferentIndividuals` witness axiom.
    fn different_individuals(&self, a: Individual, b: Individual) -> Axiom {
        Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
            id: 0,
            individuals: vec![a, b],
            annotations: Vec::new(),
        })
    }

    // ── Top / bottom property markers ────────────────────────────────────────

    fn is_top_object_property(prop: &ObjectPropertyExpression) -> bool {
        matches!(
            prop,
            ObjectPropertyExpression::ObjectProperty(p)
                if p.iri.as_str() == "http://www.w3.org/2002/07/owl#topObjectProperty"
        )
    }

    fn is_bottom_object_property(prop: &ObjectPropertyExpression) -> bool {
        matches!(
            prop,
            ObjectPropertyExpression::ObjectProperty(p)
                if p.iri.as_str() == "http://www.w3.org/2002/07/owl#bottomObjectProperty"
        )
    }

    fn is_top_data_property(prop: &DataPropertyExpression) -> bool {
        matches!(
            prop,
            DataPropertyExpression::DataProperty(p)
                if p.iri.as_str() == "http://www.w3.org/2002/07/owl#topDataProperty"
        )
    }

    fn is_bottom_data_property(prop: &DataPropertyExpression) -> bool {
        matches!(
            prop,
            DataPropertyExpression::DataProperty(p)
                if p.iri.as_str() == "http://www.w3.org/2002/07/owl#bottomDataProperty"
        )
    }

    fn is_owl_thing(class: &ClassExpression) -> bool {
        matches!(class, ClassExpression::Class(c) if c.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing")
    }

    // ── Object property characteristic entailment ────────────────────────────

    /// `O ⊨ R ⊑ S`: run the tableau over `O ∪ {R(w1,w2)}` (fresh witness pair)
    /// and check that the `RBox` closure materialised an `S(w1,w2)` edge. A
    /// `PropertyChain` sub-property is expanded into a witness path
    /// `R1(w0,w1), …, Rn(w(n-1),wn)` and the check closes to `S(w0,wn)`.
    ///
    /// Top/bottom shortcuts: `R ⊑ owl:topObjectProperty` and
    /// `owl:bottomObjectProperty ⊑ R` are always entailed.
    fn entails_sub_object_property(
        &self,
        sub: &ObjectPropertyExpression,
        sup: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_top_object_property(sup) || Self::is_bottom_object_property(sub) || sub == sup {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for sub-object-property entailment",
        )?;
        let (witnesses, (from, to)) = if let ObjectPropertyExpression::PropertyChain(chain) = sub {
            let mut nodes: Vec<Individual> =
                (0..=chain.len()).map(|_| self.fresh_individual()).collect();
            let mut witnesses = Vec::new();
            for (i, role) in chain.iter().enumerate() {
                let source = nodes[i].clone();
                let target = nodes[i + 1].clone();
                witnesses.push(self.object_assertion(role.clone(), source, target));
            }
            let from = nodes.remove(0);
            let to = nodes.pop().expect("chain has at least one role");
            (witnesses, (from, to))
        } else {
            let w1 = self.fresh_individual();
            let w2 = self.fresh_individual();
            let witness = self.object_assertion(sub.clone(), w1.clone(), w2.clone());
            (vec![witness], (w1, w2))
        };
        let tableau = self.run_witness_tableau(&guard, &witnesses)?;
        if !tableau.is_satisfiable() {
            return Ok(true); // inconsistent ontology: vacuous entailment
        }
        let snapshot = tableau.snapshot();
        let result = match (snapshot.node_of(&from), snapshot.node_of(&to)) {
            (Some(a), Some(b)) => snapshot.has_edge(a, b, &sup.to_string()),
            _ => false,
        };
        Ok(result)
    }

    /// `O ⊨ Transitive(R)`: witness chain `R(w1,w2), R(w2,w3)` must close to
    /// `R(w1,w3)`.
    fn entails_transitive(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        // owl:topObjectProperty is transitive; owl:bottomObjectProperty is
        // vacuously transitive (no edges at all).
        if Self::is_top_object_property(property) || Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for transitive entailment",
        )?;
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let w3 = self.fresh_individual();
        let witnesses = vec![
            self.object_assertion(property.clone(), w1.clone(), w2.clone()),
            self.object_assertion(property.clone(), w2.clone(), w3.clone()),
        ];
        let tableau = self.run_witness_tableau(&guard, &witnesses)?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match (snapshot.node_of(&w1), snapshot.node_of(&w3)) {
            (Some(a), Some(b)) => snapshot.has_edge(a, b, &property.to_string()),
            _ => false,
        };
        Ok(result)
    }

    /// `O ⊨ Symmetric(R)`: witness `R(w1,w2)` must close to `R(w2,w1)`.
    fn entails_symmetric(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        // owl:topObjectProperty is symmetric; owl:bottomObjectProperty is
        // vacuously symmetric (no edges at all).
        if Self::is_top_object_property(property) || Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(ontology, "tasks: reading ontology for symmetric entailment")?;
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let witness = self.object_assertion(property.clone(), w1.clone(), w2.clone());
        let tableau = self.run_witness_tableau(&guard, &[witness])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match (snapshot.node_of(&w1), snapshot.node_of(&w2)) {
            (Some(a), Some(b)) => snapshot.has_edge(b, a, &property.to_string()),
            _ => false,
        };
        Ok(result)
    }

    /// `O ⊨ Reflexive(R)`: witness individual `w` must carry an `R(w,w)` self-loop.
    /// The witness is added to the ontology (as `w:owl:Thing`) so the reflexive
    /// `RBox` rule sees a real individual node.
    fn entails_reflexive(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_top_object_property(property) {
            return Ok(true); // owl:topObjectProperty is reflexive by definition
        }
        let guard = read_lock(ontology, "tasks: reading ontology for reflexive entailment")?;
        let w1 = self.fresh_individual();
        let witness = self.class_assertion(w1.clone(), ClassExpression::thing());
        let tableau = self.run_witness_tableau(&guard, &[witness])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match snapshot.node_of(&w1) {
            Some(a) => snapshot.has_edge(a, a, &property.to_string()),
            None => false,
        };
        Ok(result)
    }

    /// `O ⊨ Asymmetric(R)`: `O ∪ {R(w1,w2), R(w2,w1)}` is inconsistent.
    ///
    /// `owl:bottomObjectProperty` is vacuously asymmetric (no edges at all).
    fn entails_asymmetric(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for asymmetric entailment",
        )?;
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let witnesses = vec![
            self.object_assertion(property.clone(), w1.clone(), w2.clone()),
            self.object_assertion(property.clone(), w2, w1),
        ];
        self.is_inconsistent_with(&guard, &witnesses)
    }

    /// `O ⊨ Irreflexive(R)`: `O ∪ {R(w,w)}` is inconsistent.
    ///
    /// `owl:bottomObjectProperty` is vacuously irreflexive (no edges at all).
    fn entails_irreflexive(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for irreflexive entailment",
        )?;
        let w1 = self.fresh_individual();
        let witness = self.object_assertion(property.clone(), w1.clone(), w1);
        self.is_inconsistent_with(&guard, &[witness])
    }

    /// `O ⊨ Functional(R)`: `O ∪ {R(w1,w2), R(w1,w3), w2≠w3}` is inconsistent.
    ///
    /// `owl:bottomObjectProperty` is vacuously functional (no edges at all).
    fn entails_functional(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for functional entailment",
        )?;
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let w3 = self.fresh_individual();
        let witnesses = vec![
            self.object_assertion(property.clone(), w1.clone(), w2.clone()),
            self.object_assertion(property.clone(), w1, w3.clone()),
            self.different_individuals(w2, w3),
        ];
        self.is_inconsistent_with(&guard, &witnesses)
    }

    /// `O ⊨ InverseFunctional(R)`: `O ∪ {R(w1,w3), R(w2,w3), w1≠w2}` is inconsistent.
    ///
    /// `owl:bottomObjectProperty` is vacuously inverse-functional (no edges).
    fn entails_inverse_functional(
        &self,
        property: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for inverse-functional entailment",
        )?;
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let w3 = self.fresh_individual();
        let witnesses = vec![
            self.object_assertion(property.clone(), w1.clone(), w3.clone()),
            self.object_assertion(property.clone(), w2.clone(), w3),
            self.different_individuals(w1, w2),
        ];
        self.is_inconsistent_with(&guard, &witnesses)
    }

    /// `O ⊨ DisjointObjectProperties([P1..Pn])`: every pair `(Pi,Pj)` must satisfy
    /// `O ∪ {Pi(w1,w2), Pj(w1,w2)}` inconsistent.
    fn entails_disjoint_object_properties(
        &self,
        properties: &[ObjectPropertyExpression],
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for disjoint-property entailment",
        )?;
        for i in 0..properties.len() {
            for j in (i + 1)..properties.len() {
                if Self::is_bottom_object_property(&properties[i])
                    || Self::is_bottom_object_property(&properties[j])
                {
                    continue; // owl:bottomObjectProperty is disjoint with everything
                }
                if Self::is_top_object_property(&properties[i])
                    || Self::is_top_object_property(&properties[j])
                {
                    return Ok(false); // nothing is disjoint with owl:topObjectProperty
                }
                let w1 = self.fresh_individual();
                let w2 = self.fresh_individual();
                let witnesses = vec![
                    self.object_assertion(properties[i].clone(), w1.clone(), w2.clone()),
                    self.object_assertion(properties[j].clone(), w1, w2),
                ];
                if !self.is_inconsistent_with(&guard, &witnesses)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `O ⊨ InverseObjectProperties(R,S)`: witness `R(w1,w2)` closes to `S(w2,w1)`
    /// and witness `S(w1,w2)` closes to `R(w2,w1)`.
    fn entails_inverse_pair(
        &self,
        r: &ObjectPropertyExpression,
        s: &ObjectPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for inverse-pair entailment",
        )?;
        // Direction R → S⁻.
        let w1 = self.fresh_individual();
        let w2 = self.fresh_individual();
        let witness = self.object_assertion(r.clone(), w1.clone(), w2.clone());
        let tableau = self.run_witness_tableau(&guard, &[witness])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let forward = match (snapshot.node_of(&w1), snapshot.node_of(&w2)) {
            (Some(a), Some(b)) => snapshot.has_edge(b, a, &s.to_string()),
            _ => false,
        };
        if !forward {
            return Ok(false);
        }
        // Direction S → R⁻.
        let w3 = self.fresh_individual();
        let w4 = self.fresh_individual();
        let witness2 = self.object_assertion(s.clone(), w3.clone(), w4.clone());
        let tableau2 = self.run_witness_tableau(&guard, &[witness2])?;
        if !tableau2.is_satisfiable() {
            return Ok(true);
        }
        let snapshot2 = tableau2.snapshot();
        Ok(match (snapshot2.node_of(&w3), snapshot2.node_of(&w4)) {
            (Some(a), Some(b)) => snapshot2.has_edge(b, a, &r.to_string()),
            _ => false,
        })
    }

    /// `O ⊨ ObjectPropertyDomain(R,C)`: run the tableau over `O ∪ {w:∃R.⊤}`; every
    /// R-edge source (including the witness `w`) must be labelled `C`. The `RBox`
    /// domain rule materialises `C` on `w` iff the domain is entailed.
    fn entails_object_domain(
        &self,
        property: &ObjectPropertyExpression,
        domain: &ClassExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_owl_thing(domain) || Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for object-domain entailment",
        )?;
        let w = self.fresh_individual();
        let some = ClassExpression::some_values_from(property.clone(), ClassExpression::thing());
        let witness = self.class_assertion(w.clone(), some);
        let tableau = self.run_witness_tableau(&guard, &[witness])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match snapshot.node_of(&w) {
            Some(n) => tableau.node_labeled_as(n, domain),
            None => false,
        };
        Ok(result)
    }

    /// `O ⊨ ObjectPropertyRange(R,C)`: run the tableau over `O ∪ {x:∃R.{w}}`; the
    /// R-edge target `w` must be labelled `C`. The `RBox` range rule materialises
    /// `C` on `w` iff the range is entailed.
    fn entails_object_range(
        &self,
        property: &ObjectPropertyExpression,
        range: &ClassExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_owl_thing(range) || Self::is_bottom_object_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for object-range entailment",
        )?;
        let x = self.fresh_individual();
        let w = self.fresh_individual();
        let has_value = ClassExpression::ObjectHasValue {
            property: property.clone(),
            value: w.clone(),
        };
        let witness = self.class_assertion(x, has_value);
        let tableau = self.run_witness_tableau(&guard, &[witness])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match snapshot.node_of(&w) {
            Some(n) => tableau.node_labeled_as(n, range),
            None => false,
        };
        Ok(result)
    }

    // ── Data property hierarchy entailment ───────────────────────────────────

    /// Transitive closure of the asserted data-property hierarchy:
    /// `SubDataPropertyOf` and `EquivalentDataProperties`. Complete for OWL 2 DL
    /// (data properties have no chains, inverses or other `RBox` role axioms).
    fn data_property_ancestors(
        &self,
        ontology: &crate::ontology::Ontology,
        property: &DataPropertyExpression,
    ) -> HashSet<DataPropertyExpression> {
        let mut ancestors: HashSet<DataPropertyExpression> = HashSet::new();
        let mut stack: Vec<DataPropertyExpression> = vec![property.clone()];
        while let Some(current) = stack.pop() {
            for axiom in ontology.axioms() {
                match axiom {
                    Axiom::SubDataPropertyOf(a) => {
                        if a.sub_property == current && ancestors.insert(a.super_property.clone()) {
                            stack.push(a.super_property.clone());
                        }
                    }
                    Axiom::EquivalentDataProperties(a) if a.properties.contains(&current) => {
                        for q in &a.properties {
                            if q != &current && ancestors.insert(q.clone()) {
                                stack.push(q.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        ancestors
    }

    /// Whether `sub ⊑* sup` in the asserted data-property hierarchy.
    fn is_sub_data_property(
        &self,
        ontology: &crate::ontology::Ontology,
        sub: &DataPropertyExpression,
        sup: &DataPropertyExpression,
    ) -> bool {
        sub == sup || self.data_property_ancestors(ontology, sub).contains(sup)
    }

    /// `O ⊨ SubDataPropertyOf(P,Q)`: `Q` is an ancestor of `P` in the asserted
    /// data-property hierarchy (top/bottom shortcuts apply).
    fn entails_sub_data_property(
        &self,
        sub: &DataPropertyExpression,
        sup: &DataPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_top_data_property(sup) || Self::is_bottom_data_property(sub) || sub == sup {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for sub-data-property entailment",
        )?;
        Ok(self.data_property_ancestors(&guard, sub).contains(sup))
    }

    /// `O ⊨ DisjointDataProperties([P1..Pn])`: for every pair, both must be
    /// sub-properties of distinct members of an asserted disjoint data-property
    /// set (bottom is disjoint with everything; top with nothing).
    fn entails_disjoint_data_properties(
        &self,
        properties: &[DataPropertyExpression],
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for disjoint-data-property entailment",
        )?;
        for i in 0..properties.len() {
            for j in (i + 1)..properties.len() {
                if Self::is_bottom_data_property(&properties[i])
                    || Self::is_bottom_data_property(&properties[j])
                {
                    continue;
                }
                if Self::is_top_data_property(&properties[i])
                    || Self::is_top_data_property(&properties[j])
                {
                    return Ok(false);
                }
                let pi = properties[i].clone();
                let pj = properties[j].clone();
                let mut disjoint = false;
                for axiom in guard.axioms() {
                    if let Axiom::DisjointDataProperties(a) = axiom {
                        for m in 0..a.properties.len() {
                            for n in (m + 1)..a.properties.len() {
                                if self.is_sub_data_property(&guard, &pi, &a.properties[m])
                                    && self.is_sub_data_property(&guard, &pj, &a.properties[n])
                                {
                                    disjoint = true;
                                }
                            }
                        }
                    }
                }
                if !disjoint {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `O ⊨ DataPropertyDomain(P,C)`: some ancestor `P'` of `P` has an asserted
    /// `DataPropertyDomain(P', C)`.
    fn entails_data_domain(
        &self,
        property: &DataPropertyExpression,
        domain: &ClassExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_owl_thing(domain) || Self::is_bottom_data_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for data-domain entailment",
        )?;
        for axiom in guard.axioms() {
            if let Axiom::DataPropertyDomain(a) = axiom
                && self.is_sub_data_property(&guard, property, &a.property)
                && &a.domain == domain
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `O ⊨ DataPropertyRange(P,R)`: some ancestor `P'` of `P` has an asserted
    /// `DataPropertyRange(P', R)`.
    fn entails_data_range(
        &self,
        property: &DataPropertyExpression,
        range: &crate::ontology::DataRange,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_data_property(property) {
            return Ok(true); // bottom data property has no values
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for data-range entailment",
        )?;
        for axiom in guard.axioms() {
            if let Axiom::DataPropertyRange(a) = axiom
                && self.is_sub_data_property(&guard, property, &a.property)
                && &a.range == range
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `O ⊨ FunctionalDataProperty(P)`: some ancestor `P'` of `P` is asserted
    /// functional (bottom data property is trivially functional).
    fn entails_functional_data(
        &self,
        property: &DataPropertyExpression,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        if Self::is_bottom_data_property(property) {
            return Ok(true);
        }
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for functional-data entailment",
        )?;
        for axiom in guard.axioms() {
            if let Axiom::FunctionalDataProperty(a) = axiom
                && self.is_sub_data_property(&guard, property, &a.property)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ── Individual axiom entailment ──────────────────────────────────────────

    /// `O ⊨ SameIndividual([a1..an])`: all individuals collapse to the same node
    /// in the completed tableau (nominal merging materialises `SameIndividual`,
    /// functional/inverse-functional, and `HasKey` equalities).
    fn entails_same_individual(
        &self,
        individuals: &[Individual],
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for same-individual entailment",
        )?;
        let tableau = self.run_witness_tableau(&guard, &[])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let mut target: Option<usize> = None;
        for individual in individuals {
            let Some(node) = snapshot.node_of(individual) else {
                return Ok(false); // an individual absent from the ABox cannot be same-as
            };
            match target {
                None => target = Some(node),
                Some(t) if t != node => return Ok(false),
                _ => {}
            }
        }
        Ok(true)
    }

    /// `O ⊨ DifferentIndividuals([a1..an])`: every pair `(a,b)` must satisfy
    /// `O ∪ {SameIndividual(a,b)}` inconsistent.
    fn entails_different_individuals(
        &self,
        individuals: &[Individual],
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for different-individuals entailment",
        )?;
        for i in 0..individuals.len() {
            for j in (i + 1)..individuals.len() {
                if individuals[i] == individuals[j] {
                    return Ok(false); // a ≠ a is unsatisfiable, never entailed
                }
                let same = self.same_individual(individuals[i].clone(), individuals[j].clone());
                if !self.is_inconsistent_with(&guard, &[same])? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `O ⊨ ObjectPropertyAssertion(R,a,b)`: the completed tableau has an
    /// `R`-edge between `a`'s and `b`'s nodes.
    fn entails_object_property_assertion(
        &self,
        assertion: &ObjectPropertyAssertionAxiom,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for object-assertion entailment",
        )?;
        let tableau = self.run_witness_tableau(&guard, &[])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match (
            snapshot.node_of(&assertion.source),
            snapshot.node_of(&assertion.target),
        ) {
            (Some(s), Some(t)) => snapshot.has_edge(s, t, &assertion.property.to_string()),
            _ => false,
        };
        Ok(result)
    }

    /// `O ⊨ DataPropertyAssertion(P,a,v)`: the completed tableau has a `P`-edge
    /// from `a`'s node to a data successor whose value equals `v`.
    fn entails_data_property_assertion(
        &self,
        assertion: &DataPropertyAssertionAxiom,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for data-assertion entailment",
        )?;
        let tableau = self.run_witness_tableau(&guard, &[])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let result = match snapshot.node_of(&assertion.individual) {
            Some(n) => {
                snapshot.has_data_edge_value(n, &assertion.property.to_string(), &assertion.value)
            }
            None => false,
        };
        Ok(result)
    }

    /// `O ⊨ NegativeObjectPropertyAssertion(¬R,a,b)`: the completed tableau has
    /// no `R`-edge between `a` and `b`, and `a` and `b` are not forced equal.
    fn entails_negative_object_property_assertion(
        &self,
        assertion: &crate::ontology::NegativeObjectPropertyAssertionAxiom,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for negative-object-assertion entailment",
        )?;
        let tableau = self.run_witness_tableau(&guard, &[])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let Some(s) = snapshot.node_of(&assertion.source) else {
            return Ok(false);
        };
        let Some(t) = snapshot.node_of(&assertion.target) else {
            return Ok(false);
        };
        if s == t {
            return Ok(false); // a ≡ b: ¬R(a,b) (which requires a ≠ b) is not entailed
        }
        Ok(!snapshot.has_edge(s, t, &assertion.property.to_string()))
    }

    /// `O ⊨ NegativeDataPropertyAssertion(¬P,a,v)`: the completed tableau has no
    /// `P`-edge from `a` to a value equal to `v`.
    fn entails_negative_data_property_assertion(
        &self,
        assertion: &crate::ontology::NegativeDataPropertyAssertionAxiom,
        ontology: &OntologyRef,
    ) -> Result<bool> {
        let guard = read_lock(
            ontology,
            "tasks: reading ontology for negative-data-assertion entailment",
        )?;
        let tableau = self.run_witness_tableau(&guard, &[])?;
        if !tableau.is_satisfiable() {
            return Ok(true);
        }
        let snapshot = tableau.snapshot();
        let Some(n) = snapshot.node_of(&assertion.individual) else {
            return Ok(false);
        };
        Ok(!snapshot.has_data_edge_value(n, &assertion.property.to_string(), &assertion.value))
    }

    // Legacy helpers below this point

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
        mut tableau: Box<dyn crate::core::reasoner::tableau::TableauRunner>,
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
