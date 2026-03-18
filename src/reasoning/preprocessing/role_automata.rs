//! Role Chain Automata Transformation
//!
//! Inspired by Konclude's `CRoleChainAutomataTransformationPreProcess`.
//!
//! Complex role chains (SubObjectPropertyOf ObjectPropertyChain) and transitive
//! roles are compiled into finite automata so that tableau expansion rules can
//! check role-chain satisfaction in O(1) via state transitions rather than by
//! recursive lookup.
//!
//! # Representation
//!
//! Each role or role chain is compiled into a non-deterministic finite automaton
//! (NFA) with:
//! - A unique initial state.
//! - A unique final (accepting) state.
//! - Transitions labelled by atomic roles.
//!
//! During tableau expansion, following a role `R` from node `n` to `n'`
//! advances the automaton state.  A node pair `(n, n')` satisfies the complex
//! role `P` iff the automaton for `P` can reach an accepting state.
//!
//! # Transitive closure (⊤⁺ optimization)
//!
//! Transitive roles `Trans(R)` get a self-loop on their single intermediate state.

use std::collections::{HashMap, HashSet};

/// A state in a role automaton.
pub type StateId = usize;

/// A transition in a role automaton.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: StateId,
    /// The atomic role that fires this transition.
    pub role: String,
    pub to: StateId,
}

/// A finite automaton encoding a role (chain).
#[derive(Debug, Clone)]
pub struct RoleAutomaton {
    /// Role or chain label this automaton recognises.
    pub role_name: String,
    pub states: usize,
    pub initial: StateId,
    pub accepting: HashSet<StateId>,
    pub transitions: Vec<Transition>,
}

impl RoleAutomaton {
    /// Create an automaton for a single atomic role.
    /// `q0 --R--> q1` with q1 as the accepting state.
    #[must_use]
    pub fn for_atomic_role(role: &str) -> Self {
        Self {
            role_name: role.to_string(),
            states: 2,
            initial: 0,
            accepting: {
                let mut s = HashSet::new();
                s.insert(1);
                s
            },
            transitions: vec![Transition { from: 0, role: role.to_string(), to: 1 }],
        }
    }

    /// Create an automaton for a role chain  R₁ ∘ R₂ ∘ … ∘ Rₙ.
    /// `q0 --R₁--> q1 --R₂--> q2 … --Rₙ--> qₙ`.
    #[must_use]
    pub fn for_chain(name: &str, chain: &[&str]) -> Self {
        let n = chain.len();
        let mut transitions = Vec::with_capacity(n);
        for (i, role) in chain.iter().enumerate() {
            transitions.push(Transition {
                from: i,
                role: role.to_string(),
                to: i + 1,
            });
        }
        let mut accepting = HashSet::new();
        accepting.insert(n);
        Self {
            role_name: name.to_string(),
            states: n + 1,
            initial: 0,
            accepting,
            transitions,
        }
    }

    /// Create an automaton for a transitive role `Trans(R)`.
    /// `q0 --R--> q0` (self-loop), with q0 also being accepting after at least
    /// one step.  We model this with:
    /// `q0 --R--> q1  q1 --R--> q1`  (q1 accepting).
    #[must_use]
    pub fn for_transitive_role(role: &str) -> Self {
        let mut accepting = HashSet::new();
        accepting.insert(1);
        Self {
            role_name: format!("{role}+"),
            states: 2,
            initial: 0,
            accepting,
            transitions: vec![
                Transition { from: 0, role: role.to_string(), to: 1 },
                Transition { from: 1, role: role.to_string(), to: 1 }, // self-loop
            ],
        }
    }

    /// Execute the automaton on a sequence of roles, returning `true` if accepted.
    #[must_use]
    pub fn accepts(&self, roles: &[&str]) -> bool {
        let mut current: HashSet<StateId> = {
            let mut s = HashSet::new();
            s.insert(self.initial);
            s
        };
        // epsilon-closure is trivial (no epsilon-transitions in our automata).
        for role in roles {
            let mut next = HashSet::new();
            for &state in &current {
                for t in &self.transitions {
                    if t.from == state && t.role == *role {
                        next.insert(t.to);
                    }
                }
            }
            current = next;
            if current.is_empty() {
                return false;
            }
        }
        current.iter().any(|s| self.accepting.contains(s))
    }

    /// Compute the set of states reachable by one transition labelled `role`.
    #[must_use]
    pub fn step(&self, states: &HashSet<StateId>, role: &str) -> HashSet<StateId> {
        let mut next = HashSet::new();
        for &s in states {
            for t in &self.transitions {
                if t.from == s && t.role == role {
                    next.insert(t.to);
                }
            }
        }
        next
    }

    /// Check whether any state in `states` is accepting.
    #[must_use]
    pub fn any_accepting(&self, states: &HashSet<StateId>) -> bool {
        states.iter().any(|s| self.accepting.contains(s))
    }
}

/// Registry of all role automata, keyed by the complex role name.
#[derive(Debug, Default)]
pub struct RoleAutomataRegistry {
    automata: HashMap<String, RoleAutomaton>,
}

impl RoleAutomataRegistry {
    /// Register an atomic role.
    pub fn register_atomic(&mut self, role: &str) {
        self.automata
            .entry(role.to_string())
            .or_insert_with(|| RoleAutomaton::for_atomic_role(role));
    }

    /// Register a transitive role.
    pub fn register_transitive(&mut self, role: &str) {
        let key = format!("{role}+");
        self.automata
            .entry(key)
            .or_insert_with(|| RoleAutomaton::for_transitive_role(role));
    }

    /// Register a role chain `name := R₁ ∘ R₂ ∘ … ∘ Rₙ`.
    pub fn register_chain(&mut self, name: &str, chain: &[&str]) {
        self.automata
            .entry(name.to_string())
            .or_insert_with(|| RoleAutomaton::for_chain(name, chain));
    }

    /// Look up an automaton by role name.
    #[must_use]
    pub fn get(&self, role: &str) -> Option<&RoleAutomaton> {
        self.automata.get(role)
    }

    /// Check whether a sequence of concrete roles satisfies the named complex role.
    #[must_use]
    pub fn satisfies(&self, complex_role: &str, role_sequence: &[&str]) -> bool {
        match self.automata.get(complex_role) {
            Some(automaton) => automaton.accepts(role_sequence),
            None => {
                // Fall back: treat as atomic role — sequence must be [complex_role].
                role_sequence.len() == 1 && role_sequence[0] == complex_role
            }
        }
    }

    /// Compute all complex roles that a given concrete role participates in.
    #[must_use]
    pub fn roles_involving(&self, atomic_role: &str) -> Vec<&str> {
        self.automata
            .values()
            .filter(|a| a.transitions.iter().any(|t| t.role == atomic_role))
            .map(|a| a.role_name.as_str())
            .collect()
    }
}

/// Summary of the transformation pass.
#[derive(Debug, Default)]
pub struct RoleAutomataStats {
    pub atomic_roles: usize,
    pub transitive_roles: usize,
    pub chain_roles: usize,
}

/// Input specification for building the registry from ontology axioms.
#[derive(Debug, Default, Clone)]
pub struct RoleAxioms {
    pub atomic_roles: Vec<String>,
    pub transitive_roles: Vec<String>,
    /// (chain_name, vec_of_components)
    pub role_chains: Vec<(String, Vec<String>)>,
}

/// Build a `RoleAutomataRegistry` from a set of role axioms.
#[must_use]
pub fn build_registry(axioms: &RoleAxioms) -> (RoleAutomataRegistry, RoleAutomataStats) {
    let mut registry = RoleAutomataRegistry::default();
    let mut stats = RoleAutomataStats::default();

    for role in &axioms.atomic_roles {
        registry.register_atomic(role);
        stats.atomic_roles += 1;
    }

    for role in &axioms.transitive_roles {
        registry.register_transitive(role);
        stats.transitive_roles += 1;
    }

    for (name, chain) in &axioms.role_chains {
        let refs: Vec<&str> = chain.iter().map(std::string::String::as_str).collect();
        registry.register_chain(name, &refs);
        stats.chain_roles += 1;
    }

    (registry, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_role_accepts() {
        let automaton = RoleAutomaton::for_atomic_role("hasPart");
        assert!(automaton.accepts(&["hasPart"]));
        assert!(!automaton.accepts(&["isPartOf"]));
        assert!(!automaton.accepts(&["hasPart", "hasPart"]));
    }

    #[test]
    fn test_chain_accepts() {
        let automaton = RoleAutomaton::for_chain("hasGrandParent", &["hasParent", "hasParent"]);
        assert!(automaton.accepts(&["hasParent", "hasParent"]));
        assert!(!automaton.accepts(&["hasParent"]));
        assert!(!automaton.accepts(&["hasParent", "hasParent", "hasParent"]));
    }

    #[test]
    fn test_transitive_role_accepts() {
        let automaton = RoleAutomaton::for_transitive_role("hasAncestor");
        assert!(automaton.accepts(&["hasAncestor"]));
        assert!(automaton.accepts(&["hasAncestor", "hasAncestor"]));
        assert!(automaton.accepts(&["hasAncestor", "hasAncestor", "hasAncestor"]));
        assert!(!automaton.accepts(&[]));
    }

    #[test]
    fn test_registry() {
        let axioms = RoleAxioms {
            atomic_roles: vec!["hasParent".to_string()],
            transitive_roles: vec!["hasAncestor".to_string()],
            role_chains: vec![
                ("hasGrandParent".to_string(), vec!["hasParent".to_string(), "hasParent".to_string()])
            ],
        };
        let (registry, stats) = build_registry(&axioms);
        assert_eq!(stats.atomic_roles, 1);
        assert_eq!(stats.transitive_roles, 1);
        assert_eq!(stats.chain_roles, 1);
        assert!(registry.satisfies("hasGrandParent", &["hasParent", "hasParent"]));
        assert!(registry.satisfies("hasAncestor+", &["hasAncestor", "hasAncestor"]));
    }
}
