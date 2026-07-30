//! OWL Ontology Manager — centralized registry for loaded ontologies.
//!
//! The `OntologyManager` is the primary entry point for working with OWL
//! ontologies. It tracks all loaded ontologies, manages imports closure,
//! applies mutations through the change system, and provides access to
//! the `DataFactory`.

pub mod changes;
pub mod history;
pub mod iri_mapper;
pub mod listeners;
pub mod loader;
pub mod sources;

#[cfg(test)]
mod tests;

use crate::factory::DataFactory;
use crate::ontology::{Ontology, OntologyRef, IRI};
use crate::Result;
use changes::OntologyChange;
use history::ChangeHistory;
use listeners::OntologyChangeListener;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

// ── ManagerConfig ────────────────────────────────────────────────────────────

/// Configuration for the OntologyManager.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Enable change history for undo/redo.
    pub enable_change_history: bool,

    /// Maximum number of change batches to retain in history.
    pub max_history_size: usize,

    /// Strategy for handling missing imports: strict = error, silent = skip.
    pub silent_missing_imports: bool,

    /// Maximum depth for recursive import resolution (to prevent infinite loops).
    pub max_import_depth: usize,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            enable_change_history: false,
            max_history_size: 100,
            silent_missing_imports: true,
            max_import_depth: 20,
        }
    }
}

// ── OntologyManager ──────────────────────────────────────────────────────────

/// Centralized registry for all loaded ontologies.
///
/// The manager tracks ontology IDs, the imports closure graph, manages
/// mutations through the change system, and provides access to the
/// shared [`DataFactory`].
///
/// # Thread Safety
///
/// The manager is wrapped in `Arc<RwLock<>>` for concurrent access.
/// Use [`OntologyManagerRef`] for shared references.
pub struct OntologyManager {
    /// All loaded ontologies, keyed by their ontology IRI.
    ontologies: HashMap<IRI, OntologyRef>,

    /// Imports dependency graph: ontology IRI → set of imported IRIs.
    imports_graph: HashMap<IRI, HashSet<IRI>>,

    /// Shared data factory for creating OWL objects.
    data_factory: DataFactory,

    /// Configuration.
    config: ManagerConfig,

    /// Registered change listeners.
    change_listeners: Vec<Box<dyn OntologyChangeListener>>,

    /// IRI-to-document mappers for resolving ontology IRIs.
    iri_mappers: Vec<Box<dyn iri_mapper::OntologyIRIMapper>>,

    /// Optional change history for undo/redo.
    change_history: Option<ChangeHistory>,
}

impl std::fmt::Debug for OntologyManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OntologyManager")
            .field("ontologies", &self.ontologies.len())
            .field("config", &self.config)
            .field("listeners", &self.change_listeners.len())
            .field("mappers", &self.iri_mappers.len())
            .finish_non_exhaustive()
    }
}

impl OntologyManager {
    /// Create a new manager with default configuration.
    #[must_use]
    pub fn new() -> Self {
        let config = ManagerConfig::default();
        Self {
            ontologies: HashMap::new(),
            imports_graph: HashMap::new(),
            data_factory: DataFactory::new(),
            change_history: if config.enable_change_history {
                Some(ChangeHistory::new(config.max_history_size))
            } else {
                None
            },
            config,
            change_listeners: Vec::new(),
            iri_mappers: Vec::new(),
        }
    }

    /// Create a new manager with a specific configuration.
    #[must_use]
    pub fn new_with_config(config: ManagerConfig) -> Self {
        let change_history = if config.enable_change_history {
            Some(ChangeHistory::new(config.max_history_size))
        } else {
            None
        };
        Self {
            ontologies: HashMap::new(),
            imports_graph: HashMap::new(),
            data_factory: DataFactory::new(),
            change_history,
            config,
            change_listeners: Vec::new(),
            iri_mappers: Vec::new(),
        }
    }

    // ── IRI Mappers ──────────────────────────────────────────────────────

    /// Set the list of IRI-to-document mappers.
    pub fn set_iri_mappers(&mut self, mappers: Vec<Box<dyn iri_mapper::OntologyIRIMapper>>) {
        self.iri_mappers = mappers;
    }

    /// Add a single IRI-to-document mapper.
    pub fn add_iri_mapper(&mut self, mapper: Box<dyn iri_mapper::OntologyIRIMapper>) {
        self.iri_mappers.push(mapper);
    }

    /// Resolve an ontology IRI to a document IRI using registered mappers.
    #[must_use]
    pub fn resolve_document_iri(&self, ontology_iri: &IRI) -> Option<IRI> {
        for mapper in &self.iri_mappers {
            if let Some(doc_iri) = mapper.get_document_iri(ontology_iri) {
                return Some(doc_iri);
            }
        }
        None
    }

    // ── Change Listeners ─────────────────────────────────────────────────

    /// Register a change listener that will be notified on every change.
    pub fn add_change_listener(&mut self, listener: Box<dyn OntologyChangeListener>) {
        self.change_listeners.push(listener);
    }

    /// Remove all change listeners.
    pub fn clear_listeners(&mut self) {
        self.change_listeners.clear();
    }

    // ── Ontology Registration ────────────────────────────────────────────

    /// Create a new empty ontology with the given IRI.
    pub fn create_ontology(&mut self, iri: IRI) -> OntologyRef {
        let mut ont = Ontology::new();
        ont.set_iri(iri.clone());
        let ont_ref = OntologyRef::new(RwLock::new(ont));
        self.imports_graph.entry(iri.clone()).or_default();
        self.ontologies.insert(iri.clone(), ont_ref.clone());
        ont_ref
    }

    /// Create a new ontology pre-populated with axioms.
    pub fn create_ontology_with_axioms(
        &mut self,
        iri: IRI,
        axioms: Vec<crate::ontology::axioms::Axiom>,
    ) -> OntologyRef {
        let mut ont = Ontology::new();
        ont.set_iri(iri.clone());
        for axiom in axioms {
            ont.add_axiom(axiom);
        }
        let ont_ref = OntologyRef::new(RwLock::new(ont));
        self.imports_graph.entry(iri.clone()).or_default();
        self.ontologies.insert(iri.clone(), ont_ref.clone());
        ont_ref
    }

    /// Register an already-constructed ontology.
    pub fn register_ontology(&mut self, ont_ref: OntologyRef) {
        let iri = {
            let guard = ont_ref.read().unwrap_or_else(|e| e.into_inner());
            guard.get_iri().cloned()
        };
        if let Some(iri) = iri {
            self.imports_graph.entry(iri.clone()).or_default();
            self.ontologies.insert(iri, ont_ref.clone());
        }
    }

    /// Get an ontology by its IRI.
    #[must_use]
    pub fn get_ontology(&self, iri: &IRI) -> Option<OntologyRef> {
        self.ontologies.get(iri).cloned()
    }

    /// Get all registered ontologies.
    #[must_use]
    pub fn get_ontologies(&self) -> Vec<OntologyRef> {
        self.ontologies.values().cloned().collect()
    }

    /// Check whether an ontology with the given IRI is already loaded.
    #[must_use]
    pub fn contains_ontology(&self, iri: &IRI) -> bool {
        self.ontologies.contains_key(iri)
    }

    /// Remove an ontology from the manager.
    pub fn remove_ontology(&mut self, ont_ref: &OntologyRef) -> Result<()> {
        let iri = {
            let guard = ont_ref.read().unwrap_or_else(|e| e.into_inner());
            guard.get_iri().cloned()
        };
        if let Some(iri) = iri {
            self.ontologies.remove(&iri);
            self.imports_graph.remove(&iri);
        }
        Ok(())
    }

    /// Get the number of loaded ontologies.
    #[must_use]
    pub fn ontology_count(&self) -> usize {
        self.ontologies.len()
    }

    // ── DataFactory Access ───────────────────────────────────────────────

    /// Get a reference to the shared data factory.
    #[must_use]
    pub fn get_data_factory(&self) -> &DataFactory {
        &self.data_factory
    }

    /// Get a mutable reference to the data factory.
    pub fn get_data_factory_mut(&mut self) -> &mut DataFactory {
        &mut self.data_factory
    }

    // ── Imports Closure ─────────────────────────────────────────────────

    /// Register an import dependency.
    pub fn add_import(&mut self, ontology_iri: IRI, imported_iri: IRI) {
        self.imports_graph
            .entry(ontology_iri)
            .or_default()
            .insert(imported_iri);
    }

    /// Remove an import dependency.
    pub fn remove_import(&mut self, ontology_iri: &IRI, imported_iri: &IRI) {
        if let Some(imports) = self.imports_graph.get_mut(ontology_iri) {
            imports.remove(imported_iri);
        }
    }

    /// Compute the imports closure for an ontology (all transitive imports).
    /// Performs BFS with cycle detection and depth limiting.
    #[must_use]
    pub fn get_imports_closure(&self, ont_ref: &OntologyRef) -> Result<Vec<OntologyRef>> {
        let start_iri = {
            let guard = ont_ref.read().unwrap_or_else(|e| e.into_inner());
            guard.get_iri().cloned()
        };

        let Some(start_iri) = start_iri else {
            return Ok(vec![ont_ref.clone()]);
        };

        let mut result: Vec<OntologyRef> = vec![ont_ref.clone()];
        let mut visited: HashSet<IRI> = HashSet::new();
        visited.insert(start_iri.clone());
        let mut queue: VecDeque<IRI> = VecDeque::new();
        queue.push_back(start_iri);

        let mut depth = 0;
        while let Some(current) = queue.pop_front() {
            depth += 1;
            if depth > self.config.max_import_depth {
                break;
            }
            if let Some(imports) = self.imports_graph.get(&current) {
                for imported_iri in imports {
                    if !visited.contains(imported_iri) {
                        visited.insert(imported_iri.clone());
                        queue.push_back(imported_iri.clone());
                        if let Some(imported_ont) = self.ontologies.get(imported_iri) {
                            result.push(imported_ont.clone());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Refresh the imports closure graph from the actual owl:imports declarations
    /// in the ontologies.
    pub fn refresh_imports_closure(&mut self) {
        for (iri, ont_ref) in &self.ontologies {
            let guard = ont_ref.read().unwrap_or_else(|e| e.into_inner());
            let entry = self.imports_graph.entry(iri.clone()).or_default();
            entry.clear();
            for import_iri in &guard.imports {
                entry.insert(import_iri.clone());
            }
        }
    }

    /// Detect import cycles.
    #[must_use]
    pub fn detect_import_cycles(&self) -> Vec<Vec<IRI>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut path = Vec::new();

        for iri in self.imports_graph.keys() {
            if !visited.contains(iri) {
                Self::dfs_cycles(
                    iri,
                    &self.imports_graph,
                    &mut visited,
                    &mut stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }
        cycles
    }

    fn dfs_cycles(
        current: &IRI,
        graph: &HashMap<IRI, HashSet<IRI>>,
        visited: &mut HashSet<IRI>,
        stack: &mut HashSet<IRI>,
        path: &mut Vec<IRI>,
        cycles: &mut Vec<Vec<IRI>>,
    ) {
        visited.insert(current.clone());
        stack.insert(current.clone());
        path.push(current.clone());

        if let Some(deps) = graph.get(current) {
            for dep in deps {
                if stack.contains(dep) {
                    if let Some(cycle_start) = path.iter().position(|iri| iri == dep) {
                        cycles.push(path[cycle_start..].to_vec());
                    }
                } else if !visited.contains(dep) {
                    Self::dfs_cycles(dep, graph, visited, stack, path, cycles);
                }
            }
        }

        path.pop();
        stack.remove(current);
    }

    // ── Config ───────────────────────────────────────────────────────────

    /// Get the manager configuration.
    #[must_use]
    pub fn config(&self) -> &ManagerConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut ManagerConfig {
        &mut self.config
    }

    // ── Change Application ───────────────────────────────────────────────
    // Delegated to changes module; core wiring is here.

    /// Apply a single change to the managed ontologies.
    pub fn apply_change(&mut self, change: OntologyChange) -> Result<()> {
        self.apply_changes(&[change])
    }

    /// Apply a batch of changes atomically.
    pub fn apply_changes(&mut self, changes: &[OntologyChange]) -> Result<()> {
        // Phase 1: apply all changes to the data model
        for change in changes {
            self.apply_single_change(change)?;
        }

        // Phase 2: update imports graph
        self.refresh_imports_closure();

        // Phase 3: record in history
        if let Some(ref mut history) = self.change_history {
            history.record(changes.to_vec());
        }

        // Phase 4: notify listeners
        for listener in &self.change_listeners {
            listener.on_changes(changes);
        }
        for listener in &self.change_listeners {
            listener.on_change_applied();
        }

        Ok(())
    }

    /// Try to apply a batch of changes — rolls back on failure.
    pub fn try_apply_changes(&mut self, changes: &[OntologyChange]) -> Result<()> {
        match self.apply_changes(changes) {
            Ok(()) => Ok(()),
            Err(e) => {
                for listener in &self.change_listeners {
                    listener.on_change_failed(&e);
                }
                Err(e)
            }
        }
    }

    /// Apply a single change to the data model.
    fn apply_single_change(&mut self, change: &OntologyChange) -> Result<()> {
        let ont_ref = self
            .get_ontology(change.ontology_iri())
            .ok_or_else(|| {
                crate::Error::InvalidInput {
                    message: format!(
                        "Ontology not found: {}",
                        change.ontology_iri()
                    ),
                }
            })?;

        let mut guard = ont_ref.write().unwrap_or_else(|e| e.into_inner());
        match change {
            OntologyChange::AddAxiom { axiom, .. } => {
                guard.add_axiom(axiom.clone());
            }
            OntologyChange::RemoveAxiom { axiom, .. } => {
                guard.remove_axiom(axiom);
            }
            OntologyChange::AddImport { import, .. } => {
                guard.imports.push(import.imported_ontology_iri.clone());
            }
            OntologyChange::RemoveImport { import, .. } => {
                guard
                    .imports
                    .retain(|iri| iri != &import.imported_ontology_iri);
            }
            OntologyChange::AddOntologyAnnotation { annotation, .. } => {
                guard.annotations.push(annotation.clone());
            }
            OntologyChange::RemoveOntologyAnnotation { annotation, .. } => {
                guard.annotations.retain(|a| a != annotation);
            }
            OntologyChange::SetOntologyId {
                new_iri,
                new_version_iri,
                ..
            } => {
                if let Some(old_iri) = guard.get_iri().cloned() {
                    self.ontologies.remove(&old_iri);
                }
                guard.set_iri(new_iri.clone());
                if let Some(viri) = new_version_iri {
                    guard.set_version_iri(Some(viri.clone()));
                }
                self.ontologies.insert(new_iri.clone(), ont_ref.clone());
            }
        }
        drop(guard);

        Ok(())
    }

    // ── Undo / Redo ──────────────────────────────────────────────────────

    /// Undo the last `n` change batches.
    pub fn undo(&mut self, n: usize) -> Result<Vec<OntologyChange>> {
        let Some(ref mut history) = self.change_history else {
            return Err(crate::Error::Unsupported {
                message: "Change history is not enabled".to_string(),
            });
        };
        let inverted = history.undo(n);
        Ok(inverted)
    }

    /// Redo the last undone change batches.
    pub fn redo(&mut self, n: usize) -> Result<Vec<OntologyChange>> {
        let Some(ref mut history) = self.change_history else {
            return Err(crate::Error::Unsupported {
                message: "Change history is not enabled".to_string(),
            });
        };
        let reapplied = history.redo(n);
        Ok(reapplied)
    }

    // ── Reasoner Integration ─────────────────────────────────────────────

    /// Create a tableau-based reasoner for the given ontology.
    pub fn create_reasoner(&self, ontology: &OntologyRef) -> Result<Box<dyn crate::reasoner_api::OWLReasoner>> {
        use crate::reasoner_api::ReasonerFactory;
        let factory = crate::reasoner_api::TableauReasonerFactory;
        let config = crate::reasoner_api::OWLReasonerConfiguration::default();
        factory.create_reasoner(ontology, &config)
    }

    /// Create a reasoner with a specific factory.
    pub fn create_reasoner_with_factory(
        &self,
        ontology: &OntologyRef,
        factory: &dyn crate::reasoner_api::ReasonerFactory,
    ) -> Result<Box<dyn crate::reasoner_api::OWLReasoner>> {
        let config = crate::reasoner_api::OWLReasonerConfiguration::default();
        factory.create_reasoner(ontology, &config)
    }
}

impl Default for OntologyManager {
    fn default() -> Self {
        Self::new()
    }
}
