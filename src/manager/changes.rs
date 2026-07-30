//! Ontology change model — types for tracking mutations to ontologies.

use crate::ontology::axioms::AxiomId;
use crate::ontology::axioms::{Axiom, AxiomTrait};
use crate::ontology::{Annotation, IRI};
use serde::{Deserialize, Serialize};

/// A document IRI or target for saving.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OntologyDocumentTarget {
    /// Local file path
    FilePath(String),
    /// URL
    Url(String),
    /// In-memory buffer
    Buffer,
}

/// Represents a single change to an ontology.
///
/// Changes are the unit of mutation in the OWL API. They track
/// exactly what was changed, on which ontology, and the nature
/// of the change (add or remove).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OntologyChange {
    /// An axiom was added.
    AddAxiom { ontology_iri: IRI, axiom: Axiom },

    /// An axiom was removed.
    RemoveAxiom { ontology_iri: IRI, axiom: Axiom },

    /// An import declaration was added.
    AddImport {
        ontology_iri: IRI,
        import: crate::import::ImportDeclaration,
    },

    /// An import declaration was removed.
    RemoveImport {
        ontology_iri: IRI,
        import: crate::import::ImportDeclaration,
    },

    /// An ontology-level annotation was added.
    AddOntologyAnnotation {
        ontology_iri: IRI,
        annotation: Annotation,
    },

    /// An ontology-level annotation was removed.
    RemoveOntologyAnnotation {
        ontology_iri: IRI,
        annotation: Annotation,
    },

    /// The ontology IRI or version IRI was changed.
    SetOntologyId {
        ontology_iri: IRI,
        new_iri: IRI,
        new_version_iri: Option<IRI>,
    },
}

// ── ChangeData — the data affected by a change ───────────────────────────────

/// The payload of an ontology change — what was actually affected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeData {
    Axiom(Axiom),
    Import(crate::import::ImportDeclaration),
    Annotation(Annotation),
    OntologyId {
        new_iri: IRI,
        new_version_iri: Option<IRI>,
    },
}

impl OntologyChange {
    /// Get the ontology IRI affected by this change.
    #[must_use]
    pub fn ontology_iri(&self) -> &IRI {
        match self {
            Self::AddAxiom { ontology_iri, .. }
            | Self::RemoveAxiom { ontology_iri, .. }
            | Self::AddImport { ontology_iri, .. }
            | Self::RemoveImport { ontology_iri, .. }
            | Self::AddOntologyAnnotation { ontology_iri, .. }
            | Self::RemoveOntologyAnnotation { ontology_iri, .. }
            | Self::SetOntologyId { ontology_iri, .. } => ontology_iri,
        }
    }

    /// Get the affected data payload.
    #[must_use]
    pub fn change_data(&self) -> ChangeData {
        match self {
            Self::AddAxiom { axiom, .. } | Self::RemoveAxiom { axiom, .. } => {
                ChangeData::Axiom(axiom.clone())
            }
            Self::AddImport { import, .. } | Self::RemoveImport { import, .. } => {
                ChangeData::Import(import.clone())
            }
            Self::AddOntologyAnnotation { annotation, .. }
            | Self::RemoveOntologyAnnotation { annotation, .. } => {
                ChangeData::Annotation(annotation.clone())
            }
            Self::SetOntologyId {
                new_iri,
                new_version_iri,
                ..
            } => ChangeData::OntologyId {
                new_iri: new_iri.clone(),
                new_version_iri: new_version_iri.clone(),
            },
        }
    }

    /// Check if this change affects axioms.
    #[must_use]
    pub fn is_axiom_change(&self) -> bool {
        matches!(self, Self::AddAxiom { .. } | Self::RemoveAxiom { .. })
    }

    /// Check if this change affects imports.
    #[must_use]
    pub fn is_import_change(&self) -> bool {
        matches!(self, Self::AddImport { .. } | Self::RemoveImport { .. })
    }

    /// Check if this change is an addition.
    #[must_use]
    pub fn is_add_change(&self) -> bool {
        matches!(
            self,
            Self::AddAxiom { .. } | Self::AddImport { .. } | Self::AddOntologyAnnotation { .. }
        )
    }

    /// Check if this change is a removal.
    #[must_use]
    pub fn is_remove_change(&self) -> bool {
        matches!(
            self,
            Self::RemoveAxiom { .. }
                | Self::RemoveImport { .. }
                | Self::RemoveOntologyAnnotation { .. }
        )
    }

    /// Get the affected axiom ID, if this is an axiom change.
    #[must_use]
    pub fn affected_axiom_id(&self) -> Option<AxiomId> {
        match self {
            Self::AddAxiom { axiom, .. } | Self::RemoveAxiom { axiom, .. } => {
                Some(axiom.axiom_id())
            }
            _ => None,
        }
    }

    /// Compute the inverse of this change (for undo).
    #[must_use]
    pub fn inverse(&self) -> OntologyChange {
        match self.clone() {
            Self::AddAxiom {
                ontology_iri,
                axiom,
            } => Self::RemoveAxiom {
                ontology_iri,
                axiom,
            },
            Self::RemoveAxiom {
                ontology_iri,
                axiom,
            } => Self::AddAxiom {
                ontology_iri,
                axiom,
            },
            Self::AddImport {
                ontology_iri,
                import,
            } => Self::RemoveImport {
                ontology_iri,
                import,
            },
            Self::RemoveImport {
                ontology_iri,
                import,
            } => Self::AddImport {
                ontology_iri,
                import,
            },
            Self::AddOntologyAnnotation {
                ontology_iri,
                annotation,
            } => Self::RemoveOntologyAnnotation {
                ontology_iri,
                annotation,
            },
            Self::RemoveOntologyAnnotation {
                ontology_iri,
                annotation,
            } => Self::AddOntologyAnnotation {
                ontology_iri,
                annotation,
            },
            Self::SetOntologyId {
                ontology_iri,
                new_iri,
                new_version_iri,
            } => Self::SetOntologyId {
                ontology_iri,
                new_iri,
                new_version_iri,
            },
        }
    }
}

// ── ChangeRecord (serializable change history) ───────────────────────────────

/// Serializable record of a single ontology change, preserving all information
/// needed to replay or audit the change. Models OWL API v5's OWLOntologyChangeRecord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// The type of change (e.g. "AddAxiom", "RemoveAxiom")
    pub change_type: String,
    /// The IRI of the ontology that was changed
    pub ontology_iri: String,
    /// The IRI of the axiom that was added or removed (if applicable)
    pub axiom_iri: Option<String>,
    /// The axiom in debug-format string (for auditing)
    pub axiom_debug: Option<String>,
    /// Timestamp of the change (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Sequence number within the history
    pub sequence_number: u64,
}

impl ChangeRecord {
    /// Create a ChangeRecord from an OntologyChange.
    #[must_use]
    pub fn from_change(change: &OntologyChange, sequence_number: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        match change {
            OntologyChange::AddAxiom {
                ontology_iri,
                axiom,
            } => ChangeRecord {
                change_type: "AddAxiom".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: Some(axiom.axiom_id().to_string()),
                axiom_debug: Some(format!("{axiom:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::RemoveAxiom {
                ontology_iri,
                axiom,
            } => ChangeRecord {
                change_type: "RemoveAxiom".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: Some(axiom.axiom_id().to_string()),
                axiom_debug: Some(format!("{axiom:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::AddImport {
                ontology_iri,
                import,
            } => ChangeRecord {
                change_type: "AddImport".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: None,
                axiom_debug: Some(format!("{import:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::RemoveImport {
                ontology_iri,
                import,
            } => ChangeRecord {
                change_type: "RemoveImport".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: None,
                axiom_debug: Some(format!("{import:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::AddOntologyAnnotation {
                ontology_iri,
                annotation,
            } => ChangeRecord {
                change_type: "AddOntologyAnnotation".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: None,
                axiom_debug: Some(format!("{annotation:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::RemoveOntologyAnnotation {
                ontology_iri,
                annotation,
            } => ChangeRecord {
                change_type: "RemoveOntologyAnnotation".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: None,
                axiom_debug: Some(format!("{annotation:?}")),
                timestamp_ms: now,
                sequence_number,
            },
            OntologyChange::SetOntologyId {
                ontology_iri,
                new_iri,
                new_version_iri,
            } => ChangeRecord {
                change_type: "SetOntologyId".to_string(),
                ontology_iri: ontology_iri.to_string(),
                axiom_iri: None,
                axiom_debug: Some(format!(
                    "new_iri={new_iri}, version={new_version_iri:?}"
                )),
                timestamp_ms: now,
                sequence_number,
            },
        }
    }
}

/// Serializable audit log of all changes applied to a manager.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeAuditLog {
    pub records: Vec<ChangeRecord>,
    pub next_sequence: u64,
}

impl ChangeAuditLog {
    #[must_use]
    pub fn new() -> Self {
        ChangeAuditLog {
            records: vec![],
            next_sequence: 0,
        }
    }

    pub fn record(&mut self, changes: &[OntologyChange]) {
        for change in changes {
            self.records
                .push(ChangeRecord::from_change(change, self.next_sequence));
            self.next_sequence += 1;
        }
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
