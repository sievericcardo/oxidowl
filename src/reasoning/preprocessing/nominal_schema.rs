//! Nominal Schema Support (SROIQV extension)
//!
//! Implements Konclude's `CNominalSchemaTemplateExtractionPreProcess` and
//! `CFullNominalSchemaGroundingPreProcess` for SROIQV(D) reasoning.
//!
//! A **nominal schema** is a parametric nominal `{z}` where `z` is a concept
//! variable (distinct from individual names). They allow axioms like:
//!
//! ```text
//! ∀z. Human(z) → ∃hasMother.({z} ⊓ Female)
//! ```
//!
//! During preprocessing:
//! 1. **Template extraction**: scan axioms for nominal schema occurrences and
//!    record them as `NominalSchemaTemplate` entries.
//! 2. **Grounding**: for each known individual `a`, substitute `z := a` to get
//!    a ground axiom.
//!
//! # Note
//! Full SROIQV support is an advanced feature.  This module provides the
//! data structures and preprocessing pipeline; integration with the main
//! tableau requires wiring into `core::tableau::executor`.

use std::collections::{HashMap};

/// A concept variable used inside a nominal schema (e.g., `z` in `{z}`).
pub type ConceptVariable = String;

/// A nominal schema template: a pattern that can be instantiated by substituting
/// concept variables with concrete individuals.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NominalSchemaTemplate {
    /// Unique identifier for this template (e.g. the axiom IRI or hash).
    pub id: String,
    /// The concept variables appearing in the schema (e.g. `["z"]`).
    pub variables: Vec<ConceptVariable>,
    /// The GCI body expressed as a list of "concept-or-nominal-schema" atoms.
    /// Each element is either a concept name `"Animal"` or a schema variable `"?z"`.
    pub body_atoms: Vec<TemplateAtom>,
    /// The GCI head atoms (same representation).
    pub head_atoms: Vec<TemplateAtom>,
}

/// An atom in a nominal schema template.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemplateAtom {
    /// A ground concept atom: `Concept(x)`.
    Concept { concept: String, var: String },
    /// A nominal schema atom: `{z}(x)` — the nominal is the individual bound to `z`.
    NominalSchema { schema_var: ConceptVariable, node_var: String },
    /// A role atom: `R(x, y)`.
    Role { role: String, from: String, to: String },
}

/// A grounded instance of a nominal schema template, where all concept
/// variables have been replaced by concrete individuals.
#[derive(Debug, Clone)]
pub struct GroundedNominalSchema {
    pub template_id: String,
    /// The substitution used: variable name → individual IRI.
    pub substitution: HashMap<ConceptVariable, String>,
    /// The resulting ground body atoms.
    pub ground_body: Vec<GroundAtom>,
    /// The resulting ground head atoms.
    pub ground_head: Vec<GroundAtom>,
}

/// A ground atom (with individual constants rather than variables).
#[derive(Debug, Clone)]
pub struct GroundAtom {
    pub predicate: String,
    pub arguments: Vec<String>,
}

/// Extracts nominal schema templates from an axiom stream.
#[derive(Debug, Default)]
pub struct NominalSchemaExtractor {
    pub templates: Vec<NominalSchemaTemplate>,
    pub stats: NominalSchemaStats,
}

/// Statistics for nominal schema processing.
#[derive(Debug, Clone, Default)]
pub struct NominalSchemaStats {
    pub templates_found: usize,
    pub individuals_used: usize,
    pub groundings_produced: usize,
}

impl NominalSchemaExtractor {
    /// Register a new nominal schema template.
    pub fn add_template(&mut self, template: NominalSchemaTemplate) {
        self.stats.templates_found += 1;
        self.templates.push(template);
    }

    /// Ground all templates with respect to a set of known individuals.
    ///
    /// Returns one grounded axiom per (template × individual-binding) combination.
    #[must_use]
    pub fn ground_all(&self, individuals: &[String]) -> Vec<GroundedNominalSchema> {
        let mut result = Vec::new();

        for template in &self.templates {
            if template.variables.is_empty() {
                continue;
            }

            // Generate all assignments of individuals to variables.
            let assignments = generate_assignments(&template.variables, individuals);
            for assignment in assignments {
                let grounded = ground_template(template, &assignment);
                result.push(grounded);
            }
        }

        result
    }
}

/// Generate all mappings from variables to individuals (cartesian product).
fn generate_assignments(
    variables: &[ConceptVariable],
    individuals: &[String],
) -> Vec<HashMap<ConceptVariable, String>> {
    if variables.is_empty() || individuals.is_empty() {
        return vec![HashMap::new()];
    }

    let mut result = vec![HashMap::new()];

    for var in variables {
        let mut expanded = Vec::new();
        for existing in &result {
            for ind in individuals {
                let mut assignment = existing.clone();
                assignment.insert(var.clone(), ind.clone());
                expanded.push(assignment);
            }
        }
        result = expanded;
    }

    result
}

/// Substitute variables in a template to produce a grounded schema.
fn ground_template(
    template: &NominalSchemaTemplate,
    assignment: &HashMap<ConceptVariable, String>,
) -> GroundedNominalSchema {
    let ground_body = template
        .body_atoms
        .iter()
        .filter_map(|atom| ground_atom(atom, assignment))
        .collect();
    let ground_head = template
        .head_atoms
        .iter()
        .filter_map(|atom| ground_atom(atom, assignment))
        .collect();

    GroundedNominalSchema {
        template_id: template.id.clone(),
        substitution: assignment.clone(),
        ground_body,
        ground_head,
    }
}

/// Ground a single template atom.
fn ground_atom(atom: &TemplateAtom, assignment: &HashMap<ConceptVariable, String>) -> Option<GroundAtom> {
    match atom {
        TemplateAtom::Concept { concept, var } => Some(GroundAtom {
            predicate: concept.clone(),
            arguments: vec![var.clone()],
        }),
        TemplateAtom::NominalSchema { schema_var, node_var } => {
            // Replace the nominal schema with the concrete individual.
            let individual = assignment.get(schema_var)?;
            Some(GroundAtom {
                predicate: format!("Nominal_{individual}"),
                arguments: vec![node_var.clone()],
            })
        }
        TemplateAtom::Role { role, from, to } => Some(GroundAtom {
            predicate: role.clone(),
            arguments: vec![from.clone(), to.clone()],
        }),
    }
}

/// Grounding pass: takes extracted templates and a set of individuals,
/// produces a flat list of ground axioms.
pub struct NominalSchemaGrounder {
    individuals: Vec<String>,
}

impl NominalSchemaGrounder {
    #[must_use]
    pub fn new(individuals: Vec<String>) -> Self {
        Self { individuals }
    }

    /// Ground all templates extracted by the extractor.
    pub fn ground(
        &self,
        extractor: &NominalSchemaExtractor,
        stats: &mut NominalSchemaStats,
    ) -> Vec<GroundedNominalSchema> {
        stats.individuals_used = self.individuals.len();
        let groundings = extractor.ground_all(&self.individuals);
        stats.groundings_produced = groundings.len();
        groundings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grounding_single_variable() {
        let template = NominalSchemaTemplate {
            id: "t1".to_string(),
            variables: vec!["z".to_string()],
            body_atoms: vec![
                TemplateAtom::Concept { concept: "Human".to_string(), var: "x".to_string() },
            ],
            head_atoms: vec![
                TemplateAtom::NominalSchema { schema_var: "z".to_string(), node_var: "x".to_string() },
            ],
        };

        let mut extractor = NominalSchemaExtractor::default();
        extractor.add_template(template);

        let individuals = vec!["Alice".to_string(), "Bob".to_string()];
        let groundings = extractor.ground_all(&individuals);

        // One grounding per individual.
        assert_eq!(groundings.len(), 2);
        let preds: Vec<&str> = groundings.iter().map(|g| g.ground_head[0].predicate.as_str()).collect();
        assert!(preds.contains(&"Nominal_Alice"));
        assert!(preds.contains(&"Nominal_Bob"));
    }

    #[test]
    fn test_assignments_count() {
        let vars = vec!["z1".to_string(), "z2".to_string()];
        let inds = vec!["a".to_string(), "b".to_string()];
        let assignments = generate_assignments(&vars, &inds);
        // 2 variables × 2 individuals = 4 assignments.
        assert_eq!(assignments.len(), 4);
    }
}
