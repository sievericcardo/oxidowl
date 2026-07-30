//! Ontology change listeners — observer pattern for ontology mutations.
//!
//! Listeners are notified after every change batch is applied to
//! the managed ontologies.

use super::changes::OntologyChange;
use crate::Error;

/// Trait for receiving notifications about ontology changes.
///
/// Implement this trait and register with [`OntologyManager::add_change_listener`]
/// to be notified of all mutations to managed ontologies.
///
/// Listeners run synchronously on the calling thread. For long-running
/// operations, spawn a background task.
pub trait OntologyChangeListener: Send + Sync {
    /// Called for every batch of changes applied.
    fn on_changes(&self, changes: &[OntologyChange]);

    /// Called after all changes in a batch have been successfully applied.
    fn on_change_applied(&self) {}

    /// Called if a change batch fails (e.g., ontology not found).
    fn on_change_failed(&self, _error: &Error) {}
}

// ── Built-in listeners ───────────────────────────────────────────────────────

/// A listener that logs changes at the given log level.
pub struct LoggingChangeListener {
    log_level: log::Level,
}

impl LoggingChangeListener {
    /// Create a logging listener at the given level.
    #[must_use]
    pub fn new(log_level: log::Level) -> Self {
        Self { log_level }
    }

    /// Create a listener at INFO level.
    #[must_use]
    pub fn info() -> Self {
        Self::new(log::Level::Info)
    }

    /// Create a listener at DEBUG level.
    #[must_use]
    pub fn debug() -> Self {
        Self::new(log::Level::Debug)
    }
}

impl OntologyChangeListener for LoggingChangeListener {
    fn on_changes(&self, changes: &[OntologyChange]) {
        if changes.is_empty() {
            return;
        }
        let adds: usize = changes.iter().filter(|c| c.is_add_change()).count();
        let removes: usize = changes.iter().filter(|c| c.is_remove_change()).count();
        let msgs = format!(
            "Ontology changes applied: +{adds} additions, -{removes} removals ({} total)",
            changes.len()
        );
        match self.log_level {
            log::Level::Error => log::error!("{msgs}"),
            log::Level::Warn => log::warn!("{msgs}"),
            log::Level::Info => log::info!("{msgs}"),
            log::Level::Debug => log::debug!("{msgs}"),
            log::Level::Trace => log::trace!("{msgs}"),
        }
    }
}

/// A no-op listener that does nothing. Useful as a default.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpChangeListener;

impl OntologyChangeListener for NoOpChangeListener {
    fn on_changes(&self, _changes: &[OntologyChange]) {}
}

pub trait MissingImportListener: Send + Sync {
    fn on_missing_import(&self, iri: &crate::ontology::IRI);
}

pub trait ImportProgressListener: Send + Sync {
    fn on_import_start(&self, iri: &crate::ontology::IRI);

    fn on_import_complete(&self, iri: &crate::ontology::IRI);
}

pub trait ReasonerChangeAwareListener: OntologyChangeListener {
    fn set_reasoner_source(&self, source: String);
}

/// Called when a change batch is vetoed (not applied).
/// This lets listeners inspect changes before they're committed.
pub trait OntologyChangesVetoedListener: Send + Sync {
    fn changes_vetoed(&self, changes: &[OntologyChange], reason: &Error);
}

/// Called when ontologies are loaded or unloaded from the manager.
pub trait OntologyLoaderListener: Send + Sync {
    fn on_ontology_loaded(&self, iri: &crate::ontology::IRI);
    fn on_ontology_unloaded(&self, iri: &crate::ontology::IRI);
}

/// Called during long-running change application to track progress.
pub trait OntologyChangeProgressListener: Send + Sync {
    fn on_changes_started(&self, total_changes: usize);
    fn on_change_applied(&self, index: usize, total: usize);
    fn on_changes_finished(&self, successful: bool);
}
