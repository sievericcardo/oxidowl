//! Ontology Metrics — quantitative measurements of ontology structure.

use crate::ontology::axioms::*;
use crate::ontology::{ClassExpression, Ontology};
use crate::transform::expressivity::DLExpressivityChecker;
use std::collections::{HashMap, HashSet};

/// A single metric that can be computed from an ontology.
pub trait OwlMetric {
    fn get_value(&self, ontology: &Ontology) -> f64;
    fn get_name(&self) -> &'static str;
}

// ── Individual Metrics ───────────────────────────────────────────────────────

pub struct NumberOfClasses;
impl OwlMetric for NumberOfClasses {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::Class(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfClasses"
    }
}

pub struct NumberOfObjectProperties;
impl OwlMetric for NumberOfObjectProperties {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::ObjectProperty(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfObjectProperties"
    }
}

pub struct NumberOfDataProperties;
impl OwlMetric for NumberOfDataProperties {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::DataProperty(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfDataProperties"
    }
}

pub struct NumberOfIndividuals;
impl OwlMetric for NumberOfIndividuals {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::NamedIndividual(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfIndividuals"
    }
}

pub struct NumberOfAxioms;
impl OwlMetric for NumberOfAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().len() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfAxioms"
    }
}

pub struct NumberOfLogicalAxioms;
impl OwlMetric for NumberOfLogicalAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().iter().filter(|a| a.is_logical()).count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfLogicalAxioms"
    }
}

pub struct NumberOfAnnotationAxioms;
impl OwlMetric for NumberOfAnnotationAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().iter().filter(|a| !a.is_logical()).count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfAnnotationAxioms"
    }
}

pub struct NumberOfSubClassAxioms;
impl OwlMetric for NumberOfSubClassAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::SubClassOf(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfSubClassAxioms"
    }
}

pub struct NumberOfEquivalentClassAxioms;
impl OwlMetric for NumberOfEquivalentClassAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::EquivalentClasses(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfEquivalentClassAxioms"
    }
}

pub struct NumberOfDisjointClassesAxioms;
impl OwlMetric for NumberOfDisjointClassesAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::DisjointClasses(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfDisjointClassesAxioms"
    }
}

pub struct NumberOfGCI;
impl OwlMetric for NumberOfGCI {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| {
                if let Axiom::SubClassOf(sc) = a {
                    !matches!(sc.subclass, ClassExpression::Class(_))
                } else {
                    false
                }
            })
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfGCI"
    }
}

pub struct NumberOfDatatypes;
impl OwlMetric for NumberOfDatatypes {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::Datatype(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfDatatypes"
    }
}

pub struct NumberOfAnnotationProperties;
impl OwlMetric for NumberOfAnnotationProperties {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::AnnotationProperty(_)) {
                    count += 1;
                }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfAnnotationProperties"
    }
}

pub struct MaxNamedSuperclassCount;
impl OwlMetric for MaxNamedSuperclassCount {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let subclass_map = build_subclass_map(ontology);
        let mut max_count = 0;
        for supers in subclass_map.values() {
            let named_count = supers
                .iter()
                .filter(|ce| matches!(ce, ClassExpression::Class(_)))
                .count();
            max_count = max_count.max(named_count);
        }
        max_count as f64
    }
    fn get_name(&self) -> &'static str {
        "MaxNamedSuperclassCount"
    }
}

pub struct DLExpressivityMetric;
impl OwlMetric for DLExpressivityMetric {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let checker = DLExpressivityChecker;
        let _ = checker.analyze(ontology);
        0.0
    }
    fn get_name(&self) -> &'static str {
        "DLExpressivity"
    }
}

pub struct AverageClassDepth;
impl OwlMetric for AverageClassDepth {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let classes = collect_named_classes(ontology);
        if classes.is_empty() {
            return 0.0;
        }
        let subclass_map = build_subclass_map(ontology);
        let mut total_depth = 0.0;
        let rooted = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::owl_thing(),
        });
        for cls in &classes {
            let depth = compute_depth(cls, &rooted, &subclass_map, &mut HashSet::new());
            total_depth += depth as f64;
        }
        total_depth / classes.len() as f64
    }
    fn get_name(&self) -> &'static str {
        "AverageClassDepth"
    }
}

pub struct MaximumClassDepth;
impl OwlMetric for MaximumClassDepth {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let classes = collect_named_classes(ontology);
        if classes.is_empty() {
            return 0.0;
        }
        let subclass_map = build_subclass_map(ontology);
        let rooted = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::owl_thing(),
        });
        let mut max = 0;
        for cls in &classes {
            let depth = compute_depth(cls, &rooted, &subclass_map, &mut HashSet::new());
            max = max.max(depth);
        }
        max as f64
    }
    fn get_name(&self) -> &'static str {
        "MaximumClassDepth"
    }
}

pub struct NumberOfSWRLRules;
impl OwlMetric for NumberOfSWRLRules {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::Rule(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfSWRLRules"
    }
}

pub struct NumberOfImports;
impl OwlMetric for NumberOfImports {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.imports.len() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfImports"
    }
}

pub struct AverageNamedSuperclassCount;
impl OwlMetric for AverageNamedSuperclassCount {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let subclass_map = build_subclass_map(ontology);
        if subclass_map.is_empty() {
            return 0.0;
        }
        let total: usize = subclass_map
            .values()
            .map(|supers| {
                supers
                    .iter()
                    .filter(|ce| matches!(ce, ClassExpression::Class(_)))
                    .count()
            })
            .sum();
        (total as f64) / (subclass_map.len() as f64)
    }
    fn get_name(&self) -> &'static str {
        "AverageNamedSuperclassCount"
    }
}

pub struct NumberOfTransitivePropertyAxioms;
impl OwlMetric for NumberOfTransitivePropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::TransitiveObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfTransitivePropertyAxioms"
    }
}

pub struct NumberOfSymmetricPropertyAxioms;
impl OwlMetric for NumberOfSymmetricPropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::SymmetricObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfSymmetricPropertyAxioms"
    }
}

pub struct NumberOfAsymmetricPropertyAxioms;
impl OwlMetric for NumberOfAsymmetricPropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::AsymmetricObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfAsymmetricPropertyAxioms"
    }
}

pub struct NumberOfFunctionalPropertyAxioms;
impl OwlMetric for NumberOfFunctionalPropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::FunctionalObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfFunctionalPropertyAxioms"
    }
}

pub struct NumberOfInverseFunctionalPropertyAxioms;
impl OwlMetric for NumberOfInverseFunctionalPropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::InverseFunctionalObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfInverseFunctionalPropertyAxioms"
    }
}

pub struct NumberOfIrreflexivePropertyAxioms;
impl OwlMetric for NumberOfIrreflexivePropertyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::IrreflexiveObjectProperty(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfIrreflexivePropertyAxioms"
    }
}

pub struct NumberOfHasKeyAxioms;
impl OwlMetric for NumberOfHasKeyAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::HasKey(_)))
            .count() as f64
    }
    fn get_name(&self) -> &'static str {
        "NumberOfHasKeyAxioms"
    }
}

// ── OntologyMetrics (composite) ──────────────────────────────────────────────

/// Computes all standard metrics for an ontology.
pub struct OntologyMetrics;

impl OntologyMetrics {
    /// Compute all metrics for a single ontology.
    #[must_use]
    pub fn compute(ontology: &Ontology) -> HashMap<String, f64> {
        let metrics: Vec<Box<dyn OwlMetric>> = vec![
            Box::new(NumberOfClasses),
            Box::new(NumberOfObjectProperties),
            Box::new(NumberOfDataProperties),
            Box::new(NumberOfIndividuals),
            Box::new(NumberOfAxioms),
            Box::new(NumberOfLogicalAxioms),
            Box::new(NumberOfAnnotationAxioms),
            Box::new(NumberOfSubClassAxioms),
            Box::new(NumberOfEquivalentClassAxioms),
            Box::new(NumberOfDisjointClassesAxioms),
            Box::new(NumberOfGCI),
            Box::new(NumberOfDatatypes),
            Box::new(NumberOfAnnotationProperties),
            Box::new(NumberOfSWRLRules),
            Box::new(NumberOfImports),
            Box::new(DLExpressivityMetric),
            Box::new(MaxNamedSuperclassCount),
            Box::new(AverageNamedSuperclassCount),
            Box::new(AverageClassDepth),
            Box::new(MaximumClassDepth),
            Box::new(NumberOfTransitivePropertyAxioms),
            Box::new(NumberOfSymmetricPropertyAxioms),
            Box::new(NumberOfAsymmetricPropertyAxioms),
            Box::new(NumberOfFunctionalPropertyAxioms),
            Box::new(NumberOfInverseFunctionalPropertyAxioms),
            Box::new(NumberOfIrreflexivePropertyAxioms),
            Box::new(NumberOfHasKeyAxioms),
        ];
        let mut result = HashMap::new();
        for metric in &metrics {
            result.insert(metric.get_name().to_string(), metric.get_value(ontology));
        }
        result
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn collect_named_classes(ontology: &Ontology) -> Vec<ClassExpression> {
    let mut classes = Vec::new();
    for axiom in ontology.axioms() {
        if let Axiom::Declaration(d) = axiom {
            if let Entity::Class(iri) = &d.entity {
                classes.push(ClassExpression::Class(crate::ontology::Class {
                    iri: iri.clone(),
                }));
            }
        }
    }
    classes
}

fn build_subclass_map(ontology: &Ontology) -> HashMap<ClassExpression, Vec<ClassExpression>> {
    let mut map: HashMap<ClassExpression, Vec<ClassExpression>> = HashMap::new();
    for axiom in ontology.axioms() {
        if let Axiom::SubClassOf(a) = axiom {
            map.entry(a.subclass.clone())
                .or_default()
                .push(a.superclass.clone());
        }
    }
    map
}

fn compute_depth(
    ce: &ClassExpression,
    root: &ClassExpression,
    subclass_map: &HashMap<ClassExpression, Vec<ClassExpression>>,
    visited: &mut HashSet<ClassExpression>,
) -> usize {
    if ce == root {
        return 0;
    }
    if !visited.insert(ce.clone()) {
        return 0;
    }
    if let Some(supers) = subclass_map.get(ce) {
        let mut min_depth = usize::MAX;
        for sup in supers {
            let d = compute_depth(sup, root, subclass_map, visited);
            if d < min_depth {
                min_depth = d;
            }
        }
        if min_depth < usize::MAX {
            return min_depth + 1;
        }
    }
    0
}
