//! Advanced GCI Absorption with Triggered Implications
//!
//! Inspired by Konclude's `CTriggeredImplicationGCIAbsorberPreProcess`, this module
//! extends the basic clause-level absorption in `core::tableau::absorption` with
//! triggered-implication-based GCI preprocessing.  The key idea is that many GCIs of
//! the form `C ⊑ D` can be "absorbed" into a concept definition: an occurrence of `C`
//! in a tableau node immediately triggers the addition of `D` without creating a new
//! branching point.
//!
//! # Patterns handled beyond the basic absorber
//!
//! | Extra pattern | Description |
//! |---|---|
//! | `TriggeredImplication`  | `C ⊑ D` where `D` is not a primitive concept name – the implication is keyed by a trigger set |
//! | `NegativeTriggered`     | `C ⊑ ¬D` → absorption as a disjointness constraint |
//! | `ExistentialTriggered`  | `C ⊑ ∃R.D` → emit a create-edge rule keyed on trigger concept `C` |
//! | `UniversalTriggered`    | `C ⊑ ∀R.D` → emit an all-successors rule |

use crate::core::tableau::absorption::{AbsorbablePattern, ClauseAbsorber};
use crate::dl_clauses::{DLAtom, DLClause, DLClauseSet};
use std::collections::HashMap;

/// Extended absorption pattern that covers triggered GCI forms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TriggeredPattern {
    /// `C ⊑ D`  — trigger on C, add D
    TriggeredImplication { trigger: String, implied: String },
    /// `C ⊑ ¬D` — trigger on C, clash when D is also present
    NegativeTriggered { trigger: String, negated: String },
    /// `C ⊑ ∃R.D`  — trigger on C, create R-successor labelled D
    ExistentialTriggered {
        trigger: String,
        role: String,
        filler: String,
    },
    /// `C ⊑ ∀R.D`  — trigger on C, propagate D along R
    UniversalTriggered {
        trigger: String,
        role: String,
        universal_filler: String,
    },
    /// Conjunction trigger: `C₁ ⊓ C₂ ⊑ D`
    ConjunctionTriggered {
        triggers: Vec<String>,
        implied: String,
    },
}

/// Statistics for the triggered-implication absorption pass.
#[derive(Debug, Clone, Default)]
pub struct TriggeredAbsorptionStats {
    pub total_gcis: usize,
    pub triggered_absorbed: usize,
    pub negative_absorbed: usize,
    pub existential_absorbed: usize,
    pub universal_absorbed: usize,
    pub conjunction_absorbed: usize,
    pub not_absorbed: usize,
}

impl TriggeredAbsorptionStats {
    #[must_use]
    pub fn absorption_rate(&self) -> f64 {
        if self.total_gcis == 0 {
            return 0.0;
        }
        let absorbed = self.triggered_absorbed
            + self.negative_absorbed
            + self.existential_absorbed
            + self.universal_absorbed
            + self.conjunction_absorbed;
        absorbed as f64 / self.total_gcis as f64
    }
}

/// Advanced absorber that identifies triggered-implication GCI patterns.
///
/// Feed it a `DLClauseSet` (or individual clauses from GCI compilation) and
/// it will surface `TriggeredPattern` entries that the tableau can exploit
/// without branching.
pub struct TriggeredImplicationAbsorber {
    /// Patterns extracted in this pass
    pub patterns: Vec<TriggeredPattern>,
    /// Clauses that could not be absorbed by any pattern
    pub remaining: Vec<DLClause>,
    /// Statistics
    pub stats: TriggeredAbsorptionStats,
    /// Index: trigger concept → list of triggered patterns (for fast lookup)
    trigger_index: HashMap<String, Vec<usize>>,
}

impl TriggeredImplicationAbsorber {
    /// Run the absorption pass on a full clause set.
    #[must_use]
    pub fn absorb(clause_set: &DLClauseSet) -> Self {
        let mut absorber = Self {
            patterns: Vec::new(),
            remaining: Vec::new(),
            stats: TriggeredAbsorptionStats::default(),
            trigger_index: HashMap::new(),
        };

        for clause in &clause_set.deterministic_clauses {
            absorber.stats.total_gcis += 1;
            absorber.try_absorb(clause);
        }

        // Disjunctive clauses need special handling; try conjunction triggers.
        for clause in &clause_set.disjunctive_clauses {
            absorber.stats.total_gcis += 1;
            if !absorber.try_absorb_disjunctive(clause) {
                absorber.remaining.push(clause.clone());
                absorber.stats.not_absorbed += 1;
            }
        }

        absorber
    }

    fn try_absorb(&mut self, clause: &DLClause) {
        // Attempt patterns in priority order.
        if self.try_triggered_implication(clause) {
            return;
        }
        if self.try_negative_triggered(clause) {
            return;
        }
        if self.try_existential_triggered(clause) {
            return;
        }
        if self.try_universal_triggered(clause) {
            return;
        }

        // Cannot absorb.
        self.remaining.push(clause.clone());
        self.stats.not_absorbed += 1;
    }

    fn try_absorb_disjunctive(&mut self, clause: &DLClause) -> bool {
        // A disjunctive clause A(x) → B(x) ∨ C(x) can be partially absorbed
        // when the body is a single concept: emit conjunction trigger for both heads.
        // We only absorb two-disjunct heads for now.
        if clause.body.len() == 1 && clause.head.len() == 2 && clause.body[0].arguments.len() == 1 {
            let trigger = clause.body[0].predicate.clone();
            let h0 = &clause.head[0];
            let h1 = &clause.head[1];
            if h0.arguments.len() == 1 && h1.arguments.len() == 1 {
                // This is still non-deterministic but we record it as a conjunction
                // trigger that fires when both head concepts are needed.
                let pattern = TriggeredPattern::ConjunctionTriggered {
                    triggers: vec![trigger.clone()],
                    implied: format!("{} ∨ {}", h0.predicate, h1.predicate),
                };
                let idx = self.patterns.len();
                self.patterns.push(pattern);
                self.trigger_index.entry(trigger).or_default().push(idx);
                self.stats.conjunction_absorbed += 1;
                return true;
            }
        }
        false
    }

    /// `A(x) → B(x)`  — single-concept body, single primitive concept head.
    fn try_triggered_implication(&mut self, clause: &DLClause) -> bool {
        if clause.body.len() != 1 || clause.head.len() != 1 {
            return false;
        }
        let body = &clause.body[0];
        let head = &clause.head[0];
        if body.arguments.len() == 1
            && head.arguments.len() == 1
            && body.arguments[0] == head.arguments[0]
        {
            let pattern = TriggeredPattern::TriggeredImplication {
                trigger: body.predicate.clone(),
                implied: head.predicate.clone(),
            };
            let idx = self.patterns.len();
            self.patterns.push(pattern);
            self.trigger_index
                .entry(body.predicate.clone())
                .or_default()
                .push(idx);
            self.stats.triggered_absorbed += 1;
            return true;
        }
        false
    }

    /// `A(x) ∧ neg:B(x) → ⊥`  i.e. disjointness expressed as `A ⊑ ¬B`.
    fn try_negative_triggered(&mut self, clause: &DLClause) -> bool {
        // Encoded as: body = [A(x), neg_B(x)], head = []
        // OR: body = [A(x)], head contains a negated concept marker.
        // We piggy-back on the existing disjointness pattern: body with 2 concepts,
        // empty head — interpret the second concept as the negated one.
        if clause.head.is_empty() && clause.body.len() == 2 {
            let b0 = &clause.body[0];
            let b1 = &clause.body[1];
            if b0.arguments.len() == 1
                && b1.arguments.len() == 1
                && b0.arguments[0] == b1.arguments[0]
            {
                let pattern = TriggeredPattern::NegativeTriggered {
                    trigger: b0.predicate.clone(),
                    negated: b1.predicate.clone(),
                };
                let idx = self.patterns.len();
                self.patterns.push(pattern);
                self.trigger_index
                    .entry(b0.predicate.clone())
                    .or_default()
                    .push(idx);
                // Also index on the symmetric direction.
                self.trigger_index
                    .entry(b1.predicate.clone())
                    .or_default()
                    .push(idx);
                self.stats.negative_absorbed += 1;
                return true;
            }
        }
        false
    }

    /// `A(x) → ∃R.B`  — body is one concept, head is one existential fact.
    /// Encoded as: body=[A(x)], head=[R(x, _fresh_)], plus head=[B(_fresh_)].
    /// Heuristic: detect body=1 concept atom, head=1 role atom + 1 concept atom.
    fn try_existential_triggered(&mut self, clause: &DLClause) -> bool {
        if clause.body.len() != 1 || clause.head.len() != 2 {
            return false;
        }
        let body = &clause.body[0];
        if body.arguments.len() != 1 {
            return false;
        }
        let trigger = body.predicate.clone();
        let trigger_var = &body.arguments[0];

        // Find the role atom and concept atom in head.
        let mut role_atom: Option<&DLAtom> = None;
        let mut filler_atom: Option<&DLAtom> = None;
        for h in &clause.head {
            if h.arguments.len() == 2 && &h.arguments[0] == trigger_var {
                role_atom = Some(h);
            } else if h.arguments.len() == 1 {
                filler_atom = Some(h);
            }
        }

        if let (Some(role), Some(filler)) = (role_atom, filler_atom) {
            // The filler variable should match role's second argument.
            if role.arguments.len() == 2
                && filler.arguments.len() == 1
                && role.arguments[1] == filler.arguments[0]
            {
                let pattern = TriggeredPattern::ExistentialTriggered {
                    trigger: trigger.clone(),
                    role: role.predicate.clone(),
                    filler: filler.predicate.clone(),
                };
                let idx = self.patterns.len();
                self.patterns.push(pattern);
                self.trigger_index.entry(trigger).or_default().push(idx);
                self.stats.existential_absorbed += 1;
                return true;
            }
        }
        false
    }

    /// `A(x) → ∀R.B`  — universal restriction trigger.
    /// Heuristic: body=1 concept, head=1 "all-successors" marker.
    /// We detect this via a convention: head contains a single role atom with
    /// a universally-quantified filler concept.
    fn try_universal_triggered(&mut self, clause: &DLClause) -> bool {
        if clause.body.len() != 1 || clause.head.len() != 1 {
            return false;
        }
        let body = &clause.body[0];
        let head = &clause.head[0];
        if body.arguments.len() == 1 && head.arguments.len() == 3 {
            // Convention: head arity-3 encodes ∀R.B as (trigger_var, role, filler_concept).
            let pattern = TriggeredPattern::UniversalTriggered {
                trigger: body.predicate.clone(),
                role: head.arguments[1].clone(),
                universal_filler: head.arguments[2].clone(),
            };
            let idx = self.patterns.len();
            self.patterns.push(pattern);
            self.trigger_index
                .entry(body.predicate.clone())
                .or_default()
                .push(idx);
            self.stats.universal_absorbed += 1;
            return true;
        }
        false
    }

    /// Look up all triggered-implication patterns for a concept.
    #[must_use]
    pub fn patterns_for_trigger(&self, concept: &str) -> Vec<&TriggeredPattern> {
        match self.trigger_index.get(concept) {
            Some(indices) => indices.iter().map(|&i| &self.patterns[i]).collect(),
            None => vec![],
        }
    }

    /// Merge results with a basic `ClauseAbsorber` so nothing is missed.
    #[must_use]
    pub fn merge_with_basic(&self, basic: &ClauseAbsorber) -> MergedAbsorptionResult {
        MergedAbsorptionResult {
            basic_patterns: basic.absorbed_patterns().to_vec(),
            triggered_patterns: self.patterns.clone(),
            remaining: self
                .remaining
                .iter()
                .filter(|c| basic.remaining_clauses().contains(c))
                .cloned()
                .collect(),
        }
    }
}

/// Combined result from both the basic absorber and the triggered-implication absorber.
#[derive(Debug)]
pub struct MergedAbsorptionResult {
    pub basic_patterns: Vec<AbsorbablePattern>,
    pub triggered_patterns: Vec<TriggeredPattern>,
    pub remaining: Vec<DLClause>,
}

impl MergedAbsorptionResult {
    #[must_use]
    pub fn total_absorbed(&self) -> usize {
        self.basic_patterns.len() + self.triggered_patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dl_clauses::{DLAtom, DLClauseSet};

    fn atom(pred: &str, args: &[&str]) -> DLAtom {
        DLAtom {
            predicate: pred.to_string(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
            is_positive: true,
            constraints: vec![],
        }
    }

    fn make_clause(body: Vec<DLAtom>, head: Vec<DLAtom>) -> DLClause {
        DLClause {
            body,
            head,
            id: "test".to_string(),
            variables: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn test_triggered_implication() {
        let clause = make_clause(
            vec![atom("Animal", &["x"])],
            vec![atom("LivingThing", &["x"])],
        );
        let mut set = DLClauseSet::default();
        set.deterministic_clauses.push(clause);
        let absorber = TriggeredImplicationAbsorber::absorb(&set);
        assert_eq!(absorber.stats.triggered_absorbed, 1);
        assert!(absorber.remaining.is_empty());
    }

    #[test]
    fn test_negative_triggered() {
        let clause = make_clause(vec![atom("Animal", &["x"]), atom("Plant", &["x"])], vec![]);
        let mut set = DLClauseSet::default();
        set.deterministic_clauses.push(clause);
        let absorber = TriggeredImplicationAbsorber::absorb(&set);
        assert_eq!(absorber.stats.negative_absorbed, 1);
    }
}
