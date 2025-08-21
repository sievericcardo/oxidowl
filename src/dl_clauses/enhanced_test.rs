#[cfg(test)]
mod enhanced_clause_tests {
    use crate::dl_clauses::types::DLAtom;

    #[test]
    fn test_enhanced_atom_creation() {
        // Test the new HermiT-style atom creation methods
        let atom1 = DLAtom::at_least_cardinality(2, "hasChild", "Person", "x");
        println!("At least cardinality atom: {}", atom1);
        assert!(atom1.predicate.contains("atLeast"));

        let atom2 = DLAtom::at_most_cardinality(1, "hasSpouse", "Person", "x");
        println!("At most cardinality atom: {}", atom2);
        assert!(atom2.predicate.contains("atMost"));

        let atom3 = DLAtom::equality_constraint("x", "y");
        println!("Equality constraint atom: {}", atom3);
        assert!(atom3.predicate.contains("="));

        let atom4 = DLAtom::nominal("John", "x");
        println!("Nominal atom: {}", atom4);
        assert!(atom4.predicate.contains("John"));

        // Test with constraints
        let atom5 = DLAtom::at_least_cardinality(1, "hasAge", "integer", "x")
            .with_constraint("≥18".to_string());
        println!("Constrained atom: {}", atom5);
        assert!(!atom5.constraints.is_empty());

        println!("Enhanced clause generation is working!");
    }
}
