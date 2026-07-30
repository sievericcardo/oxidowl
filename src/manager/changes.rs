//! Ontology change model — types for tracking mutations to ontologies.

use crate::ontology::{Annotation, IRI};
use crate::ontology::axioms::{Axiom, AxiomTrait};
use crate::ontology::axioms::AxiomId;

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
    AddAxiom {
        ontology_iri: IRI,
        axiom: Axiom,
    },

    /// An axiom was removed.
    RemoveAxiom {
        ontology_iri: IRI,
        axiom: Axiom,
    },

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
    OntologyId { new_iri: IRI, new_version_iri: Option<IRI> },
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
            Self::AddAxiom { .. }
                | Self::AddImport { .. }
                | Self::AddOntologyAnnotation { .. }
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
