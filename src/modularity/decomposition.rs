//! Atomic Decomposition — atom-level modular structure.

use crate::ontology::axioms::Axiom;
use crate::ontology::{Ontology, IRI};
use std::collections::{HashMap, HashSet};

/// A set of axioms that always appear together in modules.
#[derive(Debug, Clone)]
pub struct Atom {
    pub id: usize,
    pub axiom_positions: HashSet<usize>,
    pub dependencies: HashSet<usize>,
    pub dependent_atoms: HashSet<usize>,
}

impl Atom {
    /// Get all axioms in this atom from the decomposition.
    #[must_use]
    pub fn get_axioms(&self, decomposition: &AtomicDecomposition) -> Vec<Axiom> {
        self.axiom_positions.iter()
            .filter_map(|&pos| decomposition.axioms.get(pos).cloned())
            .collect()
    }

    /// Bottom atom: has dependencies (depends on other atoms).
    #[must_use]
    pub fn is_bot_atom(&self) -> bool { !self.dependencies.is_empty() }

    /// Top atom: nothing depends on it, no dependencies.
    #[must_use]
    pub fn is_top_atom(&self) -> bool { self.dependencies.is_empty() }

    /// Number of axioms in this atom.
    #[must_use]
    pub fn get_size(&self) -> usize { self.axiom_positions.len() }
}

/// The atomic decomposition of an ontology.
/// Each atom is a set of axioms that always appear together in any module.
#[derive(Debug, Clone)]
pub struct AtomicDecomposition {
    pub atoms: Vec<Atom>,
    pub axioms: Vec<Axiom>,
    pub axiom_to_atom: HashMap<usize, usize>,
    /// Signature of each atom.
    pub signatures: HashMap<usize, HashSet<IRI>>,
}

impl AtomicDecomposition {
    #[must_use]
    pub fn new(axioms: Vec<Axiom>) -> Self {
        Self { atoms: Vec::new(), axioms, axiom_to_atom: HashMap::new(), signatures: HashMap::new() }
    }

    /// Number of atoms in the decomposition.
    #[must_use]
    pub fn atom_count(&self) -> usize { self.atoms.len() }

    /// Get axiom count.
    #[must_use]
    pub fn axiom_count(&self) -> usize { self.axioms.len() }

    /// Get all axioms belonging to a given atom.
    #[must_use]
    pub fn get_atom_axioms(&self, atom_id: usize) -> Vec<&Axiom> {
        self.atoms.get(atom_id)
            .map(|atom| {
                atom.axiom_positions.iter()
                    .filter_map(|&pos| self.axioms.get(pos))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the atom ID for an axiom position.
    #[must_use]
    pub fn atom_for_position(&self, pos: usize) -> Option<usize> {
        self.axiom_to_atom.get(&pos).copied()
    }

    /// Get all axioms that depend on this atom.
    #[must_use]
    pub fn dependent_atoms(&self, atom_id: usize) -> HashSet<usize> {
        self.atoms.get(atom_id).map(|a| a.dependent_atoms.clone()).unwrap_or_default()
    }
}

/// Compute a simple atomic decomposition by axiom co-occurrence in signatures.
/// Groups axioms that share entities into connected components (atoms).
pub fn compute_atomic_decomposition(ontology: &Ontology) -> AtomicDecomposition {
    let axioms = ontology.axioms().to_vec();
    if axioms.is_empty() { return AtomicDecomposition::new(vec![]); }

    let wrappers: Vec<crate::modularity::AxiomWrapper> = axioms.iter().enumerate()
        .map(|(i, ax)| crate::modularity::AxiomWrapper {
            position: i,
            signature: crate::modularity::axiom_signature(ax),
        })
        .collect();

    // Build entity → axiom positions index
    let mut sig_index = crate::modularity::SigIndex::new();
    sig_index.build(&wrappers);

    // Compute adjacency: two axioms are adjacent if their signatures overlap
    let n = axioms.len();
    let mut uf = UnionFind::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            if wrappers[i].signature.iter().any(|iri| wrappers[j].signature.contains(iri)) {
                uf.union(i, j);
            }
        }
    }

    // Group by connected components
    let mut root_to_atom: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        let root = uf.find(i);
        root_to_atom[root].push(i);
    }

    let mut decomposition = AtomicDecomposition::new(axioms);
    let mut atom_id = 0;
    for component in &root_to_atom {
        if component.is_empty() { continue; }
        let positions: HashSet<usize> = component.iter().copied().collect();
        for &pos in &positions {
            decomposition.axiom_to_atom.insert(pos, atom_id);
        }
        let sig: HashSet<IRI> = positions.iter()
            .flat_map(|&pos| wrappers[pos].signature.iter().cloned())
            .collect();
        decomposition.signatures.insert(atom_id, sig);
        decomposition.atoms.push(Atom {
            id: atom_id,
            axiom_positions: positions,
            dependencies: HashSet::new(),
            dependent_atoms: HashSet::new(),
        });
        atom_id += 1;
    }

    // Compute dependencies: atom A depends on B if sig(B) ∩ sig(A) ≠ ∅
    for a in 0..atom_id {
        let sig_a = decomposition.signatures.get(&a).cloned().unwrap_or_default();
        for b in 0..atom_id {
            if a == b { continue; }
            let sig_b = decomposition.signatures.get(&b).cloned().unwrap_or_default();
            if sig_a.iter().any(|iri| sig_b.contains(iri)) {
                if let Some(atom) = decomposition.atoms.get_mut(a) {
                    atom.dependencies.insert(b);
                }
                if let Some(atom) = decomposition.atoms.get_mut(b) {
                    atom.dependent_atoms.insert(a);
                }
            }
        }
    }

    decomposition
}

// ── Union-Find ───────────────────────────────────────────────────────────────

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry { return; }
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else if self.rank[rx] > self.rank[ry] {
            self.parent[ry] = rx;
        } else {
            self.parent[ry] = rx;
            self.rank[rx] += 1;
        }
    }
}
