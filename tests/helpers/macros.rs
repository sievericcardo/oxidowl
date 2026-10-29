/// Build an ontology from a comma-separated list of axiom expressions.
///
/// # Example
/// ```
/// let o = make_ontology!(
///     SubClassOf(A, B),
///     ClassAssertion(A, i)
/// );
/// ```
#[macro_export]
macro_rules! make_ontology {
    ($($axiom:expr),* $(,)?) => {{
        let mut __o = oxidowl::ontology::Ontology::new();
        $(
            __o.add_axiom($axiom);
        )*
        __o
    }};
}

/// Build ontology with IRI from a list of axioms.
#[macro_export]
macro_rules! make_ontology_with_iri {
    ($iri:expr, $($axiom:expr),* $(,)?) => {{
        let mut __o = oxidowl::ontology::Ontology::new();
        __o.set_iri($iri);
        $(
            __o.add_axiom($axiom);
        )*
        __o
    }};
}

/// Assert that `$ont` contains `$axiom`.
#[macro_export]
macro_rules! assert_contains_axiom {
    ($ont:expr, $axiom:expr) => {{
        assert!(
            $ont.axioms().contains(&$axiom),
            "Ontology missing axiom: {:?}\nHas: {:?}",
            $axiom,
            $ont.axioms()
        );
    }};
}

/// Assert that `$ont` has exactly `$count` axioms.
#[macro_export]
macro_rules! assert_axiom_count {
    ($ont:expr, $count:expr) => {
        assert_eq!($ont.axioms().len(), $count, "Axiom count mismatch");
    };
}

/// Assert that two expressions are structurally equal and provide a
/// descriptive message on failure.
#[macro_export]
macro_rules! assert_ce_eq {
    ($left:expr, $right:expr) => {
        assert_eq!(
            $left, $right,
            "Class expression mismatch:\n  left:  {:?}\n  right: {:?}",
            $left, $right
        );
    };
}
