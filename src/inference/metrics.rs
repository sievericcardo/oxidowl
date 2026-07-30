//! Ontology Metrics — quantitative measurements of ontology structure.

use crate::ontology::{ClassExpression, Ontology};
use crate::ontology::axioms::*;
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
                if matches!(d.entity, Entity::Class(_)) { count += 1; }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfClasses" }
}

pub struct NumberOfObjectProperties;
impl OwlMetric for NumberOfObjectProperties {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::ObjectProperty(_)) { count += 1; }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfObjectProperties" }
}

pub struct NumberOfDataProperties;
impl OwlMetric for NumberOfDataProperties {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::DataProperty(_)) { count += 1; }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfDataProperties" }
}

pub struct NumberOfIndividuals;
impl OwlMetric for NumberOfIndividuals {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let mut count = 0;
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if matches!(d.entity, Entity::NamedIndividual(_)) { count += 1; }
            }
        }
        count as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfIndividuals" }
}

pub struct NumberOfAxioms;
impl OwlMetric for NumberOfAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().len() as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfAxioms" }
}

pub struct NumberOfLogicalAxioms;
impl OwlMetric for NumberOfLogicalAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().iter().filter(|a| a.is_logical()).count() as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfLogicalAxioms" }
}

pub struct NumberOfAnnotationAxioms;
impl OwlMetric for NumberOfAnnotationAxioms {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        ontology.axioms().iter().filter(|a| !a.is_logical()).count() as f64
    }
    fn get_name(&self) -> &'static str { "NumberOfAnnotationAxioms" }
}

pub struct AverageClassDepth;
impl OwlMetric for AverageClassDepth {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let classes = collect_named_classes(ontology);
        if classes.is_empty() { return 0.0; }
        let subclass_map = build_subclass_map(ontology);
        let mut total_depth = 0.0;
        let rooted = ClassExpression::Class(crate::ontology::Class { iri: crate::ontology::IRI::owl_thing() });
        for cls in &classes {
            let depth = compute_depth(cls, &rooted, &subclass_map, &mut HashSet::new());
            total_depth += depth as f64;
        }
        total_depth / classes.len() as f64
    }
    fn get_name(&self) -> &'static str { "AverageClassDepth" }
}

pub struct MaximumClassDepth;
impl OwlMetric for MaximumClassDepth {
    fn get_value(&self, ontology: &Ontology) -> f64 {
        let classes = collect_named_classes(ontology);
        if classes.is_empty() { return 0.0; }
        let subclass_map = build_subclass_map(ontology);
        let rooted = ClassExpression::Class(crate::ontology::Class { iri: crate::ontology::IRI::owl_thing() });
        let mut max = 0;
        for cls in &classes {
            let depth = compute_depth(cls, &rooted, &subclass_map, &mut HashSet::new());
            max = max.max(depth);
        }
        max as f64
    }
    fn get_name(&self) -> &'static str { "MaximumClassDepth" }
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
                classes.push(ClassExpression::Class(crate::ontology::Class { iri: iri.clone() }));
            }
        }
    }
    classes
}

fn build_subclass_map(ontology: &Ontology) -> HashMap<ClassExpression, Vec<ClassExpression>> {
    let mut map: HashMap<ClassExpression, Vec<ClassExpression>> = HashMap::new();
    for axiom in ontology.axioms() {
        if let Axiom::SubClassOf(a) = axiom {
            map.entry(a.subclass.clone()).or_default().push(a.superclass.clone());
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
    if ce == root { return 0; }
    if !visited.insert(ce.clone()) { return 0; }
    if let Some(supers) = subclass_map.get(ce) {
        let mut min_depth = usize::MAX;
        for sup in supers {
            let d = compute_depth(sup, root, subclass_map, visited);
            if d < min_depth { min_depth = d; }
        }
        if min_depth < usize::MAX { return min_depth + 1; }
    }
    0
}
