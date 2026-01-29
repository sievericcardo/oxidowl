//! Saturation rules for deterministic consequence computation

use crate::{
    Result,
    ontology::{
        ClassExpression, IRI, Ontology,
        axioms::Axiom,
    },
};
use super::node::SaturationNode;

/// Trait for saturation rules
pub trait SaturationRule: Send + Sync {
    /// Get the name of this rule
    fn name(&self) -> &str;

    /// Check if this rule is applicable to the given node
    fn is_applicable(&self, node: &SaturationNode, ontology: &Ontology) -> bool;

    /// Apply this rule to the node, returning true if changes were made
    fn apply(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool>;

    /// Get the determinism level (0-100, higher means more deterministic)
    fn determinism_level(&self) -> u8;
}

/// Collection of all saturation rules
pub struct SaturationRuleSet {
    rules: Vec<Box<dyn SaturationRule>>,
}

impl std::fmt::Debug for SaturationRuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaturationRuleSet")
            .field("rules_count", &self.rules.len())
            .finish()
    }
}

impl SaturationRuleSet {
    /// Create a new rule set with standard OWL 2 DL saturation rules
    pub fn new_owl2_dl() -> Self {
        let rules: Vec<Box<dyn SaturationRule>> = vec![
            Box::new(ConjunctionRule),
            Box::new(SubClassOfRule),
            Box::new(EquivalentClassRule),
            Box::new(UniversalRestrictionRule),
            Box::new(DomainRule),
            Box::new(RangeRule),
            Box::new(TransitivePropertyRule),
            Box::new(PropertyChainRule),
            Box::new(InversePropertyRule),
        ];

        Self { rules }
    }

    /// Apply all applicable rules to a node
    pub fn apply_all(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool> {
        let mut changed = false;

        for rule in &self.rules {
            if rule.is_applicable(node, ontology) {
                if rule.apply(node, ontology)? {
                    changed = true;
                }
            }
        }

        Ok(changed)
    }

    /// Get all rules
    pub fn rules(&self) -> &[Box<dyn SaturationRule>] {
        &self.rules
    }
}

/// Rule for handling conjunctions (AND): C ⊓ D ⊑ C, C ⊓ D ⊑ D
#[derive(Debug)]
pub struct ConjunctionRule;

impl SaturationRule for ConjunctionRule {
    fn name(&self) -> &str {
        "Conjunction"
    }

    fn is_applicable(&self, node: &SaturationNode, _ontology: &Ontology) -> bool {
        node.saturated_concepts.iter().any(|c| matches!(c, ClassExpression::ObjectIntersectionOf(_)))
    }

    fn apply(&self, node: &mut SaturationNode, _ontology: &Ontology) -> Result<bool> {
        let mut new_concepts = Vec::new();

        for concept in &node.saturated_concepts.clone() {
            if let ClassExpression::ObjectIntersectionOf(conjuncts) = concept {
                for conjunct in conjuncts {
                    if !node.saturated_concepts.contains(conjunct) {
                        new_concepts.push(conjunct.clone());
                    }
                }
            }
        }

        if !new_concepts.is_empty() {
            node.add_saturated_concepts(new_concepts);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn determinism_level(&self) -> u8 {
        100 // Fully deterministic
    }
}

/// Rule for SubClassOf axioms: C ⊑ D, individual:C ⊢ individual:D
#[derive(Debug)]
pub struct SubClassOfRule;

impl SaturationRule for SubClassOfRule {
    fn name(&self) -> &str {
        "SubClassOf"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(a, Axiom::SubClassOf(_)))
    }

    fn apply(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool> {
        let mut new_concepts = Vec::new();

        for axiom in ontology.axioms() {
            if let Axiom::SubClassOf(subclass_axiom) = axiom {
                // If the node contains the subclass, add the superclass
                if node.saturated_concepts.contains(&subclass_axiom.subclass) {
                    if !node.saturated_concepts.contains(&subclass_axiom.superclass) {
                        new_concepts.push(subclass_axiom.superclass.clone());
                        node.add_direct_subsumer(subclass_axiom.superclass.clone());
                    }
                }
            }
        }

        if !new_concepts.is_empty() {
            node.add_saturated_concepts(new_concepts);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn determinism_level(&self) -> u8 {
        100 // Fully deterministic
    }
}

/// Rule for EquivalentClasses axioms: C ≡ D ⊢ C ⊑ D, D ⊑ C
#[derive(Debug)]
pub struct EquivalentClassRule;

impl SaturationRule for EquivalentClassRule {
    fn name(&self) -> &str {
        "EquivalentClasses"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(a, Axiom::EquivalentClasses(_)))
    }

    fn apply(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool> {
        let mut new_concepts = Vec::new();

        for axiom in ontology.axioms() {
            if let Axiom::EquivalentClasses(equiv_axiom) = axiom {
                // Check if any equivalent class is present
                for class in &equiv_axiom.classes {
                    if node.saturated_concepts.contains(class) {
                        // Add all other equivalent classes
                        for other_class in &equiv_axiom.classes {
                            if class != other_class && !node.saturated_concepts.contains(other_class) {
                                new_concepts.push(other_class.clone());
                            }
                        }
                    }
                }
            }
        }

        if !new_concepts.is_empty() {
            node.add_saturated_concepts(new_concepts);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn determinism_level(&self) -> u8 {
        100 // Fully deterministic
    }
}

/// Rule for universal restrictions: ∀R.C absorption
#[derive(Debug)]
pub struct UniversalRestrictionRule;

impl SaturationRule for UniversalRestrictionRule {
    fn name(&self) -> &str {
        "UniversalRestriction"
    }

    fn is_applicable(&self, node: &SaturationNode, _ontology: &Ontology) -> bool {
        node.saturated_concepts.iter().any(|c| matches!(c, ClassExpression::ObjectAllValuesFrom { .. }))
    }

    fn apply(&self, node: &mut SaturationNode, _ontology: &Ontology) -> Result<bool> {
        let mut changed = false;

        for concept in &node.saturated_concepts.clone() {
            if let ClassExpression::ObjectAllValuesFrom { property, filler } = concept {
                // Extract property IRI
                if let Some(property_iri) = extract_property_iri(property) {
                    node.add_universal(property_iri, (**filler).clone());
                    changed = true;
                }
            }
        }

        Ok(changed)
    }

    fn determinism_level(&self) -> u8 {
        95 // Mostly deterministic
    }
}

/// Rule for domain axioms: Domain(R, C) ⊢ ∃R.⊤ ⊑ C
#[derive(Debug)]
pub struct DomainRule;

impl SaturationRule for DomainRule {
    fn name(&self) -> &str {
        "Domain"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(
            a,
            Axiom::ObjectPropertyDomain(_) | Axiom::DataPropertyDomain(_)
        ))
    }

    fn apply(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool> {
        let mut new_concepts = Vec::new();

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ObjectPropertyDomain(domain_axiom) => {
                    // Check if node has existential with this property
                    if let Some(property_iri) = extract_property_iri(&domain_axiom.property) {
                        for existential in &node.existentials {
                            if existential.property == property_iri {
                                if !node.saturated_concepts.contains(&domain_axiom.domain) {
                                    new_concepts.push(domain_axiom.domain.clone());
                                }
                            }
                        }
                    }
                }
                Axiom::DataPropertyDomain(domain_axiom) => {
                    // Similar handling for data properties
                    if let Some(property_iri) = extract_data_property_iri(&domain_axiom.property) {
                        // Check data property usage
                        if !node.saturated_concepts.contains(&domain_axiom.domain) {
                            // Add domain if property is used
                            // new_concepts.push(domain_axiom.domain.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        if !new_concepts.is_empty() {
            node.add_saturated_concepts(new_concepts);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn determinism_level(&self) -> u8 {
        100 // Fully deterministic
    }
}

/// Rule for range axioms: Range(R, C) ⊢ ⊤ ⊑ ∀R.C
#[derive(Debug)]
pub struct RangeRule;

impl SaturationRule for RangeRule {
    fn name(&self) -> &str {
        "Range"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(
            a,
            Axiom::ObjectPropertyRange(_) | Axiom::DataPropertyRange(_)
        ))
    }

    fn apply(&self, node: &mut SaturationNode, ontology: &Ontology) -> Result<bool> {
        let mut changed = false;

        for axiom in ontology.axioms() {
            if let Axiom::ObjectPropertyRange(range_axiom) = axiom {
                if let Some(property_iri) = extract_property_iri(&range_axiom.property) {
                    // Add universal restriction for this property
                    node.add_universal(property_iri, range_axiom.range.clone());
                    changed = true;
                }
            }
        }

        Ok(changed)
    }

    fn determinism_level(&self) -> u8 {
        95 // Mostly deterministic
    }
}

/// Rule for transitive properties
#[derive(Debug)]
pub struct TransitivePropertyRule;

impl SaturationRule for TransitivePropertyRule {
    fn name(&self) -> &str {
        "TransitiveProperty"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(a, Axiom::TransitiveObjectProperty(_)))
    }

    fn apply(&self, _node: &mut SaturationNode, _ontology: &Ontology) -> Result<bool> {
        // Transitive property handling requires edge processing
        // This is a placeholder for the full implementation
        Ok(false)
    }

    fn determinism_level(&self) -> u8 {
        80 // Partially deterministic
    }
}

/// Rule for property chains
#[derive(Debug)]
pub struct PropertyChainRule;

impl SaturationRule for PropertyChainRule {
    fn name(&self) -> &str {
        "PropertyChain"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(a, Axiom::SubObjectPropertyOf(_)))
    }

    fn apply(&self, _node: &mut SaturationNode, _ontology: &Ontology) -> Result<bool> {
        // Property chain handling requires edge processing
        // This is a placeholder for the full implementation
        Ok(false)
    }

    fn determinism_level(&self) -> u8 {
        70 // Partially deterministic
    }
}

/// Rule for inverse properties
#[derive(Debug)]
pub struct InversePropertyRule;

impl SaturationRule for InversePropertyRule {
    fn name(&self) -> &str {
        "InverseProperty"
    }

    fn is_applicable(&self, _node: &SaturationNode, ontology: &Ontology) -> bool {
        ontology.axioms().iter().any(|a| matches!(a, Axiom::InverseObjectProperties(_)))
    }

    fn apply(&self, _node: &mut SaturationNode, _ontology: &Ontology) -> Result<bool> {
        // Inverse property handling requires edge processing
        // This is a placeholder for the full implementation
        Ok(false)
    }

    fn determinism_level(&self) -> u8 {
        90 // Mostly deterministic
    }
}

// Helper functions

fn extract_property_iri(property_expr: &crate::ontology::ObjectPropertyExpression) -> Option<IRI> {
    use crate::ontology::ObjectPropertyExpression;
    match property_expr {
        ObjectPropertyExpression::ObjectProperty(prop) => Some(prop.iri.clone()),
        ObjectPropertyExpression::InverseObjectProperty(prop) => Some(prop.iri.clone()),
        ObjectPropertyExpression::PropertyChain(_) => None, // Property chains don't have a single IRI
    }
}

fn extract_data_property_iri(property_expr: &crate::ontology::DataPropertyExpression) -> Option<IRI> {
    use crate::ontology::DataPropertyExpression;
    match property_expr {
        DataPropertyExpression::DataProperty(prop) => Some(prop.iri.clone()),
    }
}
