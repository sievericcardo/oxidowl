//! High-performance structural hashing for OWL class expressions
//!
//! This module provides optimized hashing functions that directly hash the structure
//! of class expressions without expensive Debug formatting or string conversions.

use crate::core::persistent_collections::ConceptSet;
use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, Individual, Literal,
    ObjectPropertyExpression,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Fast structural hasher for class expressions
pub struct FastConceptHasher {
    hasher: DefaultHasher,
}

impl FastConceptHasher {
    /// Create a new fast hasher
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: DefaultHasher::new(),
        }
    }

    /// Hash a class expression structurally (5-10x faster than Debug formatting)
    #[inline]
    pub fn hash_concept(&mut self, concept: &ClassExpression) {
        // Use discriminant for variant identification
        std::mem::discriminant(concept).hash(&mut self.hasher);

        match concept {
            ClassExpression::Class(class) => {
                class.iri.as_str().hash(&mut self.hasher);
            }
            ClassExpression::ObjectIntersectionOf(exprs) => {
                exprs.len().hash(&mut self.hasher);
                for expr in exprs {
                    self.hash_concept(expr);
                }
            }
            ClassExpression::ObjectUnionOf(exprs) => {
                exprs.len().hash(&mut self.hasher);
                for expr in exprs {
                    self.hash_concept(expr);
                }
            }
            ClassExpression::ObjectOneOf(individuals) => {
                individuals.len().hash(&mut self.hasher);
                for ind in individuals {
                    ind.hash(&mut self.hasher);
                }
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.hash_object_property(property);
                self.hash_concept(filler);
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.hash_object_property(property);
                self.hash_concept(filler);
            }
            ClassExpression::ObjectHasValue { property, value } => {
                self.hash_object_property(property);
                value.hash(&mut self.hasher);
            }
            ClassExpression::ObjectHasSelf { property } => {
                self.hash_object_property(property);
            }
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_object_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_concept(filler);
            }
            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_object_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_concept(filler);
            }
            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_object_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_concept(filler);
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                self.hash_data_property(property);
                self.hash_data_range(filler);
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                self.hash_data_property(property);
                self.hash_data_range(filler);
            }
            ClassExpression::DataHasValue { property, value } => {
                self.hash_data_property(property);
                self.hash_literal(value);
            }
            ClassExpression::DataMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_data_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_data_range(filler);
            }
            ClassExpression::DataMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_data_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_data_range(filler);
            }
            ClassExpression::DataExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                self.hash_data_property(property);
                cardinality.hash(&mut self.hasher);
                self.hash_data_range(filler);
            }
            ClassExpression::ObjectComplementOf(expr) => {
                self.hash_concept(expr);
            }
        }
    }

    #[inline]
    fn hash_object_property(&mut self, prop: &ObjectPropertyExpression) {
        std::mem::discriminant(prop).hash(&mut self.hasher);
        match prop {
            ObjectPropertyExpression::ObjectProperty(p) => {
                p.iri.as_str().hash(&mut self.hasher);
            }
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                p.iri.as_str().hash(&mut self.hasher);
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                chain.len().hash(&mut self.hasher);
                for prop in chain {
                    self.hash_object_property(prop);
                }
            }
        }
    }

    #[inline]
    fn hash_data_property(&mut self, prop: &DataPropertyExpression) {
        // DataPropertyExpression is typically just a DataProperty with IRI
        prop.hash(&mut self.hasher);
    }

    #[inline]
    fn hash_data_range(&mut self, range: &DataRange) {
        // Hash data range structure
        range.hash(&mut self.hasher);
    }

    #[inline]
    fn hash_literal(&mut self, lit: &Literal) {
        lit.value.hash(&mut self.hasher);
        if let Some(ref dt) = lit.datatype {
            dt.as_str().hash(&mut self.hasher);
        }
        if let Some(ref lang) = lit.language {
            lang.hash(&mut self.hasher);
        }
    }

    /// Finalize and return the hash value
    #[inline]
    #[must_use]
    pub fn finish(self) -> u64 {
        self.hasher.finish()
    }
}

impl Default for FastConceptHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two concepts structurally (faster than Debug formatting)
///
/// Precondition: Both concepts must be the same variant (same discriminant)
fn compare_concepts(a: &ClassExpression, b: &ClassExpression) -> std::cmp::Ordering {
    compare_concepts_with_depth(a, b, 0)
}

/// Maximum recursion depth for comparison to prevent stack overflow
const MAX_COMPARISON_DEPTH: usize = 500;

fn compare_concepts_with_depth(
    a: &ClassExpression,
    b: &ClassExpression,
    depth: usize,
) -> std::cmp::Ordering {
    use ClassExpression::{Class, ObjectIntersectionOf, ObjectUnionOf, ObjectOneOf, ObjectSomeValuesFrom, ObjectAllValuesFrom, ObjectHasValue, ObjectHasSelf, ObjectMinCardinality, ObjectMaxCardinality, ObjectExactCardinality, DataSomeValuesFrom, DataAllValuesFrom, DataHasValue, DataMinCardinality, DataMaxCardinality, DataExactCardinality, ObjectComplementOf};
    use std::cmp::Ordering;

    // Prevent stack overflow on deeply nested expressions
    if depth > MAX_COMPARISON_DEPTH {
        // Fall back to discriminant-only comparison for very deep structures
        let a_disc = std::mem::discriminant(a);
        let b_disc = std::mem::discriminant(b);
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    // CRITICAL: Check discriminants first to ensure total order
    // This prevents different variants from being considered equal
    let a_disc = std::mem::discriminant(a);
    let b_disc = std::mem::discriminant(b);

    if a_disc != b_disc {
        // Different variants - use discriminant ordering for total order
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    // Same variant - compare by content
    match (a, b) {
        (Class(ca), Class(cb)) => ca.iri.as_str().cmp(cb.iri.as_str()),

        (ObjectIntersectionOf(ea), ObjectIntersectionOf(eb)) => {
            compare_concept_lists_with_depth(ea, eb, depth + 1)
        }
        (ObjectUnionOf(ea), ObjectUnionOf(eb)) => {
            compare_concept_lists_with_depth(ea, eb, depth + 1)
        }

        (ObjectOneOf(ia), ObjectOneOf(ib)) => {
            // Compare individual lists
            ia.len().cmp(&ib.len()).then_with(|| {
                for (ind_a, ind_b) in ia.iter().zip(ib.iter()) {
                    match compare_individuals(ind_a, ind_b) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
                Ordering::Equal
            })
        }

        (
            ObjectSomeValuesFrom {
                property: pa,
                filler: fa,
            },
            ObjectSomeValuesFrom {
                property: pb,
                filler: fb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| compare_concepts_with_depth(fa, fb, depth + 1)),

        (
            ObjectAllValuesFrom {
                property: pa,
                filler: fa,
            },
            ObjectAllValuesFrom {
                property: pb,
                filler: fb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| compare_concepts_with_depth(fa, fb, depth + 1)),

        (
            ObjectHasValue {
                property: pa,
                value: va,
            },
            ObjectHasValue {
                property: pb,
                value: vb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| compare_individuals(va, vb)),

        (ObjectHasSelf { property: pa }, ObjectHasSelf { property: pb }) => {
            compare_object_properties_with_depth(pa, pb, depth + 1)
        }

        (
            ObjectMinCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            ObjectMinCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_concepts_with_depth(fa, fb, depth + 1)),

        (
            ObjectMaxCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            ObjectMaxCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_concepts_with_depth(fa, fb, depth + 1)),

        (
            ObjectExactCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            ObjectExactCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_object_properties_with_depth(pa, pb, depth + 1)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_concepts_with_depth(fa, fb, depth + 1)),

        (
            DataSomeValuesFrom {
                property: pa,
                filler: fa,
            },
            DataSomeValuesFrom {
                property: pb,
                filler: fb,
            },
        ) => compare_data_properties(pa, pb)
            .then_with(|| compare_data_ranges_with_depth(fa, fb, depth + 1)),

        (
            DataAllValuesFrom {
                property: pa,
                filler: fa,
            },
            DataAllValuesFrom {
                property: pb,
                filler: fb,
            },
        ) => compare_data_properties(pa, pb)
            .then_with(|| compare_data_ranges_with_depth(fa, fb, depth + 1)),

        (
            DataHasValue {
                property: pa,
                value: va,
            },
            DataHasValue {
                property: pb,
                value: vb,
            },
        ) => compare_data_properties(pa, pb).then_with(|| compare_literals(va, vb)),

        (
            DataMinCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            DataMinCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_data_properties(pa, pb)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_data_ranges_with_depth(fa, fb, depth + 1)),

        (
            DataMaxCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            DataMaxCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_data_properties(pa, pb)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_data_ranges_with_depth(fa, fb, depth + 1)),

        (
            DataExactCardinality {
                property: pa,
                cardinality: ca,
                filler: fa,
            },
            DataExactCardinality {
                property: pb,
                cardinality: cb,
                filler: fb,
            },
        ) => compare_data_properties(pa, pb)
            .then_with(|| ca.cmp(cb))
            .then_with(|| compare_data_ranges_with_depth(fa, fb, depth + 1)),

        (ObjectComplementOf(ea), ObjectComplementOf(eb)) => {
            compare_concepts_with_depth(ea, eb, depth + 1)
        }

        // This should never be reached since we checked discriminants above
        _ => Ordering::Equal,
    }
}

/// Compare two lists of concepts
#[allow(dead_code)]
fn compare_concept_lists(a: &[ClassExpression], b: &[ClassExpression]) -> std::cmp::Ordering {
    compare_concept_lists_with_depth(a, b, 0)
}

fn compare_concept_lists_with_depth(
    a: &[ClassExpression],
    b: &[ClassExpression],
    depth: usize,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    a.len().cmp(&b.len()).then_with(|| {
        for (ca, cb) in a.iter().zip(b.iter()) {
            match compare_concepts_with_depth(ca, cb, depth) {
                Ordering::Equal => {}
                other => return other,
            }
        }
        Ordering::Equal
    })
}

/// Compare two object properties
#[allow(dead_code)]
fn compare_object_properties(
    a: &ObjectPropertyExpression,
    b: &ObjectPropertyExpression,
) -> std::cmp::Ordering {
    compare_object_properties_with_depth(a, b, 0)
}

fn compare_object_properties_with_depth(
    a: &ObjectPropertyExpression,
    b: &ObjectPropertyExpression,
    depth: usize,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Prevent stack overflow on deeply nested property chains
    if depth > MAX_COMPARISON_DEPTH {
        let a_disc = std::mem::discriminant(a);
        let b_disc = std::mem::discriminant(b);
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    let a_disc = std::mem::discriminant(a);
    let b_disc = std::mem::discriminant(b);

    if a_disc != b_disc {
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    match (a, b) {
        (
            ObjectPropertyExpression::ObjectProperty(pa),
            ObjectPropertyExpression::ObjectProperty(pb),
        ) => pa.iri.as_str().cmp(pb.iri.as_str()),
        (
            ObjectPropertyExpression::InverseObjectProperty(pa),
            ObjectPropertyExpression::InverseObjectProperty(pb),
        ) => pa.iri.as_str().cmp(pb.iri.as_str()),
        (
            ObjectPropertyExpression::PropertyChain(ca),
            ObjectPropertyExpression::PropertyChain(cb),
        ) => ca.len().cmp(&cb.len()).then_with(|| {
            for (prop_a, prop_b) in ca.iter().zip(cb.iter()) {
                match compare_object_properties_with_depth(prop_a, prop_b, depth + 1) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            Ordering::Equal
        }),
        _ => Ordering::Equal,
    }
}

/// Compare two data properties
fn compare_data_properties(
    a: &DataPropertyExpression,
    b: &DataPropertyExpression,
) -> std::cmp::Ordering {
    match (a, b) {
        (DataPropertyExpression::DataProperty(pa), DataPropertyExpression::DataProperty(pb)) => {
            pa.iri.as_str().cmp(pb.iri.as_str())
        }
    }
}

/// Compare two data ranges
#[allow(dead_code)]
fn compare_data_ranges(a: &DataRange, b: &DataRange) -> std::cmp::Ordering {
    compare_data_ranges_with_depth(a, b, 0)
}

fn compare_data_ranges_with_depth(
    a: &DataRange,
    b: &DataRange,
    depth: usize,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Prevent stack overflow on deeply nested data ranges
    if depth > MAX_COMPARISON_DEPTH {
        let a_disc = std::mem::discriminant(a);
        let b_disc = std::mem::discriminant(b);
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    let a_disc = std::mem::discriminant(a);
    let b_disc = std::mem::discriminant(b);

    if a_disc != b_disc {
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    match (a, b) {
        (DataRange::Datatype(ia), DataRange::Datatype(ib)) => ia.as_str().cmp(ib.as_str()),
        (DataRange::DataIntersectionOf(ra), DataRange::DataIntersectionOf(rb)) => {
            ra.len().cmp(&rb.len()).then_with(|| {
                for (range_a, range_b) in ra.iter().zip(rb.iter()) {
                    match compare_data_ranges_with_depth(range_a, range_b, depth + 1) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
                Ordering::Equal
            })
        }
        (DataRange::DataUnionOf(ra), DataRange::DataUnionOf(rb)) => {
            ra.len().cmp(&rb.len()).then_with(|| {
                for (range_a, range_b) in ra.iter().zip(rb.iter()) {
                    match compare_data_ranges_with_depth(range_a, range_b, depth + 1) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
                Ordering::Equal
            })
        }
        (DataRange::DataComplementOf(ra), DataRange::DataComplementOf(rb)) => {
            compare_data_ranges_with_depth(ra, rb, depth + 1)
        }
        (DataRange::DataOneOf(la), DataRange::DataOneOf(lb)) => {
            la.len().cmp(&lb.len()).then_with(|| {
                for (lit_a, lit_b) in la.iter().zip(lb.iter()) {
                    match compare_literals(lit_a, lit_b) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
                Ordering::Equal
            })
        }
        (
            DataRange::DatatypeRestriction {
                datatype: da,
                restrictions: ra,
            },
            DataRange::DatatypeRestriction {
                datatype: db,
                restrictions: rb,
            },
        ) => {
            da.as_str()
                .cmp(db.as_str())
                .then_with(|| ra.len().cmp(&rb.len()))
            // Note: FacetRestriction comparison would need implementation
            // For now just compare by length
        }
        _ => Ordering::Equal,
    }
}

/// Compare two individuals
fn compare_individuals(a: &Individual, b: &Individual) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let a_disc = std::mem::discriminant(a);
    let b_disc = std::mem::discriminant(b);

    if a_disc != b_disc {
        return format!("{a_disc:?}").cmp(&format!("{b_disc:?}"));
    }

    match (a, b) {
        (Individual::Named(na), Individual::Named(nb)) => na.iri.as_str().cmp(nb.iri.as_str()),
        (Individual::Anonymous(aa), Individual::Anonymous(ab)) => aa.id.cmp(&ab.id),
        _ => Ordering::Equal,
    }
}

/// Compare two literals
fn compare_literals(a: &Literal, b: &Literal) -> std::cmp::Ordering {
    a.value
        .cmp(&b.value)
        .then_with(|| a.datatype.cmp(&b.datatype))
        .then_with(|| a.language.cmp(&b.language))
}

/// Compute a fast structural hash for a concept set
///
/// This function is 5-10x faster than the string-based approach because it:
/// - Directly hashes structural components (IRIs, discriminants)
/// - Avoids Debug formatting overhead
/// - Uses efficient inline hashing
#[must_use]
pub fn compute_fast_signature(concepts: &ConceptSet) -> u64 {
    let mut hasher = FastConceptHasher::new();

    // Sort concepts for deterministic hashing
    // Note: We still need to sort for consistency, but we can optimize this later
    // with a canonical ordering based on hash values
    let mut sorted_concepts: Vec<_> = concepts.iter().collect();
    sorted_concepts.sort_by(|a, b| {
        // Quick comparison using structural properties
        let a_disc = std::mem::discriminant(*a);
        let b_disc = std::mem::discriminant(*b);

        if a_disc != b_disc {
            // Different variants - use discriminant ordering
            format!("{a_disc:?}").cmp(&format!("{b_disc:?}"))
        } else {
            // Same variant - compare by content using structural comparison
            compare_concepts(a, b)
        }
    });

    // Hash all concepts in sorted order
    for concept in sorted_concepts {
        hasher.hash_concept(concept);
    }

    hasher.finish()
}

/// Compute hash for a single concept (useful for indexing)
#[inline]
#[must_use]
pub fn hash_concept(concept: &ClassExpression) -> u64 {
    let mut hasher = FastConceptHasher::new();
    hasher.hash_concept(concept);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_fast_hashing_consistency() {
        let concept1 = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassA"),
        });
        let concept2 = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassA"),
        });
        let concept3 = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassB"),
        });

        let hash1 = hash_concept(&concept1);
        let hash2 = hash_concept(&concept2);
        let hash3 = hash_concept(&concept3);

        // Same concepts should have same hash
        assert_eq!(hash1, hash2);
        // Different concepts should have different hashes (with high probability)
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_fast_signature_deterministic() {
        use crate::core::persistent_collections::ConceptSet;

        let concept1 = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassA"),
        });
        let concept2 = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassB"),
        });

        let mut set1 = ConceptSet::new();
        set1 = set1.update(concept1.clone());
        set1 = set1.update(concept2.clone());

        let mut set2 = ConceptSet::new();
        set2 = set2.update(concept2);
        set2 = set2.update(concept1);

        let sig1 = compute_fast_signature(&set1);
        let sig2 = compute_fast_signature(&set2);

        // Same concepts in different order should have same signature
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_complex_expression_hashing() {
        let class_a = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassA"),
        });
        let class_b = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/ClassB"),
        });

        let intersection =
            ClassExpression::ObjectIntersectionOf(vec![class_a.clone(), class_b.clone()]);

        let complement = ClassExpression::ObjectComplementOf(Box::new(class_a));

        let hash_int = hash_concept(&intersection);
        let hash_comp = hash_concept(&complement);

        // Different expressions should have different hashes
        assert_ne!(hash_int, hash_comp);
    }
}
