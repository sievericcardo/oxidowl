//! Negation Normal Form (NNF) converter.
//!
//! Pushes negations inward using De Morgan's laws and OWL-specific
//! transformation rules.

use crate::ontology::ClassExpression;
use crate::ontology::IRI;

/// Converts class expressions to Negation Normal Form.
#[derive(Debug, Clone, Copy, Default)]
pub struct NNFConverter;

impl NNFConverter {
    /// Convert a class expression to NNF by pushing all negations inward.
    #[must_use]
    pub fn to_nnf(&self, expr: &ClassExpression) -> ClassExpression {
        match expr {
            // Base case: named class (not negated at this level)
            ClassExpression::Class(_) => expr.clone(),

            // ¬¬C → C
            ClassExpression::ObjectComplementOf(inner) => self.complement_to_nnf(inner),

            // ¬(C ⊓ D) → ¬C ⊔ ¬D
            ClassExpression::ObjectIntersectionOf(ops) => ClassExpression::ObjectIntersectionOf(
                ops.iter().map(|op| self.to_nnf(op)).collect(),
            ),

            // ¬(C ⊔ D) → ¬C ⊓ ¬D
            ClassExpression::ObjectUnionOf(ops) => {
                ClassExpression::ObjectUnionOf(ops.iter().map(|op| self.to_nnf(op)).collect())
            }

            // ¬(∃R.C) → ∀R.¬C
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(self.to_nnf(filler)),
                }
            }

            // ¬(∀R.C) → ∃R.¬C
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(self.to_nnf(filler)),
                }
            }

            // ¬(∃R.{a}) — hasValue
            ClassExpression::ObjectHasValue { property, value } => {
                ClassExpression::ObjectHasValue {
                    property: property.clone(),
                    value: value.clone(),
                }
            }

            // ¬(∃R.Self) → ∀R.¬Self (treated as primitive)
            ClassExpression::ObjectHasSelf { property } => ClassExpression::ObjectHasSelf {
                property: property.clone(),
            },

            // Cardinality: ¬(≥ n R.C) → ≤ (n-1) R.C for n ≥ 1
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::ObjectMinCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(self.to_nnf(filler)),
            },

            // ¬(≤ n R.C) → ≥ (n+1) R.C
            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::ObjectMaxCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(self.to_nnf(filler)),
            },

            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::ObjectExactCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: Box::new(self.to_nnf(filler)),
            },

            ClassExpression::ObjectOneOf(inds) => ClassExpression::ObjectOneOf(inds.clone()),

            ClassExpression::DataSomeValuesFrom { property, filler } => {
                ClassExpression::DataSomeValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataAllValuesFrom { property, filler } => {
                ClassExpression::DataAllValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataHasValue { property, value } => ClassExpression::DataHasValue {
                property: property.clone(),
                value: value.clone(),
            },

            ClassExpression::DataMinCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::DataMinCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            },

            ClassExpression::DataMaxCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::DataMaxCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            },

            ClassExpression::DataExactCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::DataExactCardinality {
                property: property.clone(),
                cardinality: *cardinality,
                filler: filler.clone(),
            },
        }
    }

    /// Process a complement by pushing negation inward.
    fn complement_to_nnf(self, inner: &ClassExpression) -> ClassExpression {
        match inner {
            // ¬¬C → C
            ClassExpression::ObjectComplementOf(inner2) => self.to_nnf(inner2),

            // ¬(C ⊓ D) → ¬C ⊔ ¬D
            ClassExpression::ObjectIntersectionOf(ops) => ClassExpression::ObjectUnionOf(
                ops.iter()
                    .map(|op| ClassExpression::ObjectComplementOf(Box::new(op.clone())))
                    .map(|neg| self.to_nnf(&neg))
                    .collect(),
            ),

            // ¬(C ⊔ D) → ¬C ⊓ ¬D
            ClassExpression::ObjectUnionOf(ops) => ClassExpression::ObjectIntersectionOf(
                ops.iter()
                    .map(|op| ClassExpression::ObjectComplementOf(Box::new(op.clone())))
                    .map(|neg| self.to_nnf(&neg))
                    .collect(),
            ),

            // ¬(∃R.C) → ∀R.¬C
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(self.to_nnf(&ClassExpression::ObjectComplementOf(Box::new(
                        (**filler).clone(),
                    )))),
                }
            }

            // ¬(∀R.C) → ∃R.¬C
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(self.to_nnf(&ClassExpression::ObjectComplementOf(Box::new(
                        (**filler).clone(),
                    )))),
                }
            }

            // ¬(∃R.{a}) → ∀R.¬{a} = ∀R.complementOf({a})
            ClassExpression::ObjectHasValue { property, value } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(ClassExpression::ObjectComplementOf(Box::new(
                        ClassExpression::ObjectOneOf(vec![value.clone()]),
                    ))),
                }
            }

            // ¬(≥ n R.C) → ≤ (n-1) R.C for n ≥ 1
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                if *cardinality > 0 {
                    ClassExpression::ObjectMaxCardinality {
                        property: property.clone(),
                        cardinality: cardinality - 1,
                        filler: Box::new(self.to_nnf(filler)),
                    }
                } else {
                    // ¬(≥0 R.C) → ⊥
                    ClassExpression::ObjectComplementOf(Box::new(
                        ClassExpression::ObjectComplementOf(Box::new(ClassExpression::Class(
                            crate::ontology::Class {
                                iri: IRI::owl_nothing(),
                            },
                        ))),
                    ))
                }
            }

            // ¬(≤ n R.C) → ≥ (n+1) R.C
            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => ClassExpression::ObjectMinCardinality {
                property: property.clone(),
                cardinality: cardinality + 1,
                filler: Box::new(self.to_nnf(filler)),
            },

            // Named class ¬C → just wrap (already in NNF form)
            _ => ClassExpression::ObjectComplementOf(Box::new(self.to_nnf(inner))),
        }
    }
}
