//! Change history — undo/redo support for ontology mutations.
//!
//! ChangeHistory records batches of [`OntologyChange`] and allows
//! them to be undone (reversed) or redone.

use super::changes::OntologyChange;

/// Records change batches for undo/redo support.
///
/// Each entry in the history is a batch of changes that were applied
/// together. Undo computes the inverse of each change in the batch
/// and returns them (caller must re-apply).
#[derive(Debug)]
pub struct ChangeHistory {
    /// All recorded change batches, oldest first.
    history: Vec<Vec<OntologyChange>>,
    /// Current position: changes at indices < `position` are "applied".
    position: usize,
    /// Maximum number of batches to retain.
    max_size: usize,
}

impl ChangeHistory {
    /// Create a new history with the given maximum size.
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_size),
            position: 0,
            max_size,
        }
    }

    /// Record a new change batch.
    /// Discards any redo history ahead of the current position.
    pub fn record(&mut self, changes: Vec<OntologyChange>) {
        self.history.truncate(self.position);
        self.history.push(changes);
        self.position = self.history.len();
        self.prune();
    }

    /// Undo the last `n` change batches. Returns the inverse changes.
    /// The caller must re-apply these inverse changes.
    #[must_use]
    pub fn undo(&mut self, n: usize) -> Vec<OntologyChange> {
        let to_undo = std::cmp::min(n, self.position);
        let mut result = Vec::new();
        for i in (self.position - to_undo..self.position).rev() {
            for change in &self.history[i] {
                result.push(change.inverse());
            }
        }
        self.position -= to_undo;
        result
    }

    /// Redo the last `n` undone change batches. Returns the changes to re-apply.
    #[must_use]
    pub fn redo(&mut self, n: usize) -> Vec<OntologyChange> {
        let to_redo = std::cmp::min(n, self.history.len() - self.position);
        let mut result = Vec::new();
        for i in self.position..self.position + to_redo {
            result.extend(self.history[i].iter().cloned());
        }
        self.position += to_redo;
        result
    }

    /// Check if any undo operations are available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    /// Check if any redo operations are available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.position < self.history.len()
    }

    /// Get the number of undo-able batches.
    #[must_use]
    pub fn undo_count(&self) -> usize {
        self.position
    }

    /// Get the number of redo-able batches.
    #[must_use]
    pub fn redo_count(&self) -> usize {
        self.history.len() - self.position
    }

    /// Total number of recorded batches (including undone ones).
    #[must_use]
    pub fn total_batches(&self) -> usize {
        self.history.len()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.history.clear();
        self.position = 0;
    }

    fn prune(&mut self) {
        if self.history.len() > self.max_size {
            let excess = self.history.len() - self.max_size;
            self.history.drain(0..excess);
            self.position = self.position.saturating_sub(excess);
        }
    }
}

impl Default for ChangeHistory {
    fn default() -> Self {
        Self::new(100)
    }
}
