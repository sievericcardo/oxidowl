use crate::ontology::axioms::*;
use crate::ontology::*;

/// Amalgamate multiple SubClassOf axioms with the same LHS into a single
/// SubClassOf with ObjectIntersectionOf on the RHS.
pub fn amalgamate_sub_class_axioms(axioms: &[SubClassOfAxiom]) -> Vec<SubClassOfAxiom> {
    use std::collections::HashMap;
    let mut grouped: HashMap<ClassExpression, Vec<ClassExpression>> = HashMap::new();
    for ax in axioms {
        grouped
            .entry(ax.subclass.clone())
            .or_default()
            .push(ax.superclass.clone());
    }
    grouped
        .into_iter()
        .map(|(sub, sups)| {
            let sup = if sups.len() == 1 {
                sups[0].clone()
            } else {
                ClassExpression::ObjectIntersectionOf(sups)
            };
            SubClassOfAxiom {
                id: 0,
                subclass: sub,
                superclass: sup,
                annotations: vec![],
            }
        })
        .collect()
}

/// Split a SubClassOf with ObjectIntersectionOf on the RHS into individual
/// SubClassOf axioms.
pub fn split_sub_class_axioms(axiom: &SubClassOfAxiom) -> Vec<SubClassOfAxiom> {
    match &axiom.superclass {
        ClassExpression::ObjectIntersectionOf(conjuncts) => conjuncts
            .iter()
            .map(|c| SubClassOfAxiom {
                id: 0,
                subclass: axiom.subclass.clone(),
                superclass: c.clone(),
                annotations: axiom.annotations.clone(),
            })
            .collect(),
        _ => vec![axiom.clone()],
    }
}

/// Convert EquivalentClasses(A, B, C) to bidirectional SubClassOf axioms:
/// SubClassOf(A, B), SubClassOf(B, A), SubClassOf(A, C), SubClassOf(C, A).
pub fn convert_equivalent_to_sub_classes(axiom: &EquivalentClassesAxiom) -> Vec<SubClassOfAxiom> {
    let mut result = vec![];
    for i in 0..axiom.classes.len() {
        for j in 0..axiom.classes.len() {
            if i != j {
                result.push(SubClassOfAxiom {
                    id: 0,
                    subclass: axiom.classes[i].clone(),
                    superclass: axiom.classes[j].clone(),
                    annotations: vec![],
                });
            }
        }
    }
    result
}

/// Convert property assertions to annotations.
pub fn convert_property_assertions_to_annotations(axioms: &[Axiom]) -> (Vec<Axiom>, Vec<Axiom>) {
    let mut removed = vec![];
    let mut added = vec![];
    for ax in axioms {
        if let Axiom::ObjectPropertyAssertion(opa) = ax {
            removed.push(ax.clone());
            let ann = Annotation::new(
                AnnotationProperty {
                    iri: opa
                        .property
                        .iri()
                        .cloned()
                        .unwrap_or_else(|| IRI::new("http://ex.org/")),
                },
                AnnotationValue::IRI(IRI::new("http://ex.org/")),
                vec![],
            );
            added.push(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                id: 0,
                subject: AnnotationSubject::IRI(IRI::new("http://ex.org/")),
                property: ann.property,
                value: ann.value,
                annotations: vec![],
            }));
        }
    }
    (removed, added)
}
