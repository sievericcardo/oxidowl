//! Bidirectional axiom indexes for O(1) ontology lookups.
//!
//! `AxiomIndex` provides pre-built maps from each class/individual/property
//! to the axioms that mention it, eliminating the need to scan the full axiom
//! list on every query.
//!
//! The index covers **all** logical OWL 2 axiom types — class axioms, object
//! and data property axioms (including property chains and the eight property
//! characteristics), individual axioms (SameIndividual, DifferentIndividuals,
//! positive and negative assertions), `HasKey`, `DatatypeDefinition` and SWRL
//! rules — plus a general per-type store so every axiom remains retrievable.

use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, IRI, Individual, ObjectPropertyExpression,
    axioms::{
        Axiom, AxiomTrait, AxiomType, HasKeyAxiom, NegativeDataPropertyAssertionAxiom,
        NegativeObjectPropertyAssertionAxiom, SWRLRuleAxiom,
    },
    datatypes::DatatypeDefinitionAxiom,
};
use std::collections::{HashMap, HashSet};

/// Pre-computed index over all axioms in an ontology snapshot.
///
/// Build once with `AxiomIndex::build(axioms)` and reuse across queries, or
/// maintain it incrementally with [`add_axiom`](Self::add_axiom) /
/// [`remove_axiom`](Self::remove_axiom).
#[derive(Debug, Default, Clone)]
pub struct AxiomIndex {
    /// SubClassOf axioms indexed by subclass — maps sub → list of supers
    pub subclass_by_sub: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// SubClassOf axioms indexed by superclass — maps super → list of subs
    pub subclass_by_super: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// EquivalentClasses axioms — maps each class → the other equivalent classes
    pub equivalent_by_class: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// ClassAssertion axioms indexed by class — maps class → list of individuals
    pub class_assertion_by_class: HashMap<ClassExpression, Vec<Individual>>,

    /// ClassAssertion axioms indexed by individual — maps individual → list of classes
    pub class_assertion_by_ind: HashMap<Individual, Vec<ClassExpression>>,

    /// ObjectPropertyDomain axioms indexed by property
    pub domain_by_prop: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,

    /// ObjectPropertyRange axioms indexed by property
    pub range_by_prop: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,

    /// DisjointClasses axioms — maps each class → the classes disjoint with it
    pub disjoint_by_class: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// SubObjectPropertyOf axioms indexed by sub-property (incl. property chains)
    pub sub_object_property_by_sub:
        HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,

    /// SubObjectPropertyOf axioms indexed by super-property
    pub sub_object_property_by_super:
        HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,

    /// EquivalentObjectProperties — maps each property → the other equivalent properties
    pub equivalent_object_properties_by_prop:
        HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,

    /// DisjointObjectProperties — maps each property → the disjoint properties
    pub disjoint_object_properties_by_prop:
        HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,

    /// InverseObjectProperties — maps each property → its inverse properties
    pub inverse_object_properties_by_prop:
        HashMap<ObjectPropertyExpression, Vec<ObjectPropertyExpression>>,

    /// SubDataPropertyOf axioms indexed by sub-property
    pub sub_data_property_by_sub: HashMap<DataPropertyExpression, Vec<DataPropertyExpression>>,

    /// SubDataPropertyOf axioms indexed by super-property
    pub sub_data_property_by_super: HashMap<DataPropertyExpression, Vec<DataPropertyExpression>>,

    /// EquivalentDataProperties — maps each property → the other equivalent properties
    pub equivalent_data_properties_by_prop:
        HashMap<DataPropertyExpression, Vec<DataPropertyExpression>>,

    /// DisjointDataProperties — maps each property → the disjoint properties
    pub disjoint_data_properties_by_prop:
        HashMap<DataPropertyExpression, Vec<DataPropertyExpression>>,

    /// DataPropertyDomain axioms indexed by property
    pub data_property_domain_by_prop: HashMap<DataPropertyExpression, Vec<ClassExpression>>,

    /// DataPropertyRange axioms indexed by property
    pub data_property_range_by_prop: HashMap<DataPropertyExpression, Vec<DataRange>>,

    /// Properties declared FunctionalObjectProperty
    pub functional_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared InverseFunctionalObjectProperty
    pub inverse_functional_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared ReflexiveObjectProperty
    pub reflexive_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared IrreflexiveObjectProperty
    pub irreflexive_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared SymmetricObjectProperty
    pub symmetric_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared AsymmetricObjectProperty
    pub asymmetric_object_properties: HashSet<ObjectPropertyExpression>,

    /// Properties declared TransitiveObjectProperty
    pub transitive_object_properties: HashSet<ObjectPropertyExpression>,

    /// Data properties declared FunctionalDataProperty
    pub functional_data_properties: HashSet<DataPropertyExpression>,

    /// SameIndividual axioms indexed by individual — maps ind → other same individuals
    pub same_individual_by_ind: HashMap<Individual, Vec<Individual>>,

    /// DifferentIndividuals axioms indexed by individual — maps ind → other different individuals
    pub different_individuals_by_ind: HashMap<Individual, Vec<Individual>>,

    /// NegativeObjectPropertyAssertion axioms indexed by source individual
    pub negative_object_assertions_by_source:
        HashMap<Individual, Vec<NegativeObjectPropertyAssertionAxiom>>,

    /// NegativeDataPropertyAssertion axioms indexed by individual
    pub negative_data_assertions_by_ind:
        HashMap<Individual, Vec<NegativeDataPropertyAssertionAxiom>>,

    /// HasKey axioms indexed by class
    pub haskey_by_class: HashMap<ClassExpression, Vec<HasKeyAxiom>>,

    /// DatatypeDefinition axioms indexed by the IRI of the defined datatype
    pub datatype_definition_by_datatype: HashMap<IRI, Vec<DatatypeDefinitionAxiom>>,

    /// All SWRL rules
    pub rules: Vec<SWRLRuleAxiom>,

    /// General per-axiom-type store so that *every* axiom (including
    /// declarations and annotation axioms) remains retrievable by type.
    axioms_by_type: HashMap<AxiomType, Vec<Axiom>>,
}

impl AxiomIndex {
    /// Build an index from a slice of axioms.
    #[must_use]
    pub fn build(axioms: &[Axiom]) -> Self {
        let mut idx = Self::default();
        for axiom in axioms {
            idx.add_axiom(axiom.clone());
        }
        idx
    }

    /// Register a single axiom in the index.
    pub fn add_axiom(&mut self, axiom: Axiom) {
        let axiom_type = axiom.axiom_type();
        self.axioms_by_type
            .entry(axiom_type)
            .or_default()
            .push(axiom.clone());

        match &axiom {
            Axiom::SubClassOf(ax) => {
                push(
                    &mut self.subclass_by_sub,
                    ax.subclass.clone(),
                    ax.superclass.clone(),
                );
                push(
                    &mut self.subclass_by_super,
                    ax.superclass.clone(),
                    ax.subclass.clone(),
                );
            }
            Axiom::EquivalentClasses(ax) => {
                add_all_pairs(&mut self.equivalent_by_class, &ax.classes);
            }
            Axiom::DisjointUnion(ax) => {
                // DisjointUnion(A, [B, C]) ≡ A ≡ B ⊔ C (plus pairwise disjointness).
                // Every member is a subclass of the union class; the reverse (A ⊑ B)
                // is NOT entailed (A may be witnessed by C alone), so only the
                // member → union edges are indexed.
                if let ClassExpression::Class(union_cls) = &ax.class {
                    for member in &ax.disjoint_classes {
                        if let ClassExpression::Class(member_cls) = member {
                            push(
                                &mut self.subclass_by_sub,
                                ClassExpression::Class(member_cls.clone()),
                                ClassExpression::Class(union_cls.clone()),
                            );
                            push(
                                &mut self.subclass_by_super,
                                ClassExpression::Class(union_cls.clone()),
                                ClassExpression::Class(member_cls.clone()),
                            );
                        }
                    }
                }
            }
            Axiom::ClassAssertion(ax) => {
                push(
                    &mut self.class_assertion_by_class,
                    ax.class.clone(),
                    ax.individual.clone(),
                );
                push(
                    &mut self.class_assertion_by_ind,
                    ax.individual.clone(),
                    ax.class.clone(),
                );
            }
            Axiom::ObjectPropertyDomain(ax) => {
                push(
                    &mut self.domain_by_prop,
                    ax.property.clone(),
                    ax.domain.clone(),
                );
            }
            Axiom::ObjectPropertyRange(ax) => {
                push(
                    &mut self.range_by_prop,
                    ax.property.clone(),
                    ax.range.clone(),
                );
            }
            Axiom::DisjointClasses(ax) => {
                add_all_pairs(&mut self.disjoint_by_class, &ax.classes);
            }
            Axiom::SubObjectPropertyOf(ax) => {
                push(
                    &mut self.sub_object_property_by_sub,
                    ax.sub_property.clone(),
                    ax.super_property.clone(),
                );
                push(
                    &mut self.sub_object_property_by_super,
                    ax.super_property.clone(),
                    ax.sub_property.clone(),
                );
            }
            Axiom::EquivalentObjectProperties(ax) => {
                add_all_pairs(
                    &mut self.equivalent_object_properties_by_prop,
                    &ax.properties,
                );
            }
            Axiom::DisjointObjectProperties(ax) => {
                add_all_pairs(&mut self.disjoint_object_properties_by_prop, &ax.properties);
            }
            Axiom::InverseObjectProperties(ax) => {
                push(
                    &mut self.inverse_object_properties_by_prop,
                    ax.property1.clone(),
                    ax.property2.clone(),
                );
                push(
                    &mut self.inverse_object_properties_by_prop,
                    ax.property2.clone(),
                    ax.property1.clone(),
                );
            }
            Axiom::SubDataPropertyOf(ax) => {
                push(
                    &mut self.sub_data_property_by_sub,
                    ax.sub_property.clone(),
                    ax.super_property.clone(),
                );
                push(
                    &mut self.sub_data_property_by_super,
                    ax.super_property.clone(),
                    ax.sub_property.clone(),
                );
            }
            Axiom::EquivalentDataProperties(ax) => {
                add_all_pairs(&mut self.equivalent_data_properties_by_prop, &ax.properties);
            }
            Axiom::DisjointDataProperties(ax) => {
                add_all_pairs(&mut self.disjoint_data_properties_by_prop, &ax.properties);
            }
            Axiom::DataPropertyDomain(ax) => {
                push(
                    &mut self.data_property_domain_by_prop,
                    ax.property.clone(),
                    ax.domain.clone(),
                );
            }
            Axiom::DataPropertyRange(ax) => {
                push(
                    &mut self.data_property_range_by_prop,
                    ax.property.clone(),
                    ax.range.clone(),
                );
            }
            Axiom::FunctionalObjectProperty(ax) => {
                self.functional_object_properties
                    .insert(ax.property.clone());
            }
            Axiom::InverseFunctionalObjectProperty(ax) => {
                self.inverse_functional_object_properties
                    .insert(ax.property.clone());
            }
            Axiom::ReflexiveObjectProperty(ax) => {
                self.reflexive_object_properties.insert(ax.property.clone());
            }
            Axiom::IrreflexiveObjectProperty(ax) => {
                self.irreflexive_object_properties
                    .insert(ax.property.clone());
            }
            Axiom::SymmetricObjectProperty(ax) => {
                self.symmetric_object_properties.insert(ax.property.clone());
            }
            Axiom::AsymmetricObjectProperty(ax) => {
                self.asymmetric_object_properties
                    .insert(ax.property.clone());
            }
            Axiom::TransitiveObjectProperty(ax) => {
                self.transitive_object_properties
                    .insert(ax.property.clone());
            }
            Axiom::FunctionalDataProperty(ax) => {
                self.functional_data_properties.insert(ax.property.clone());
            }
            Axiom::SameIndividual(ax) => {
                add_all_pairs(&mut self.same_individual_by_ind, &ax.individuals);
            }
            Axiom::DifferentIndividuals(ax) => {
                add_all_pairs(&mut self.different_individuals_by_ind, &ax.individuals);
            }
            Axiom::NegativeObjectPropertyAssertion(ax) => {
                push(
                    &mut self.negative_object_assertions_by_source,
                    ax.source.clone(),
                    ax.clone(),
                );
            }
            Axiom::NegativeDataPropertyAssertion(ax) => {
                push(
                    &mut self.negative_data_assertions_by_ind,
                    ax.individual.clone(),
                    ax.clone(),
                );
            }
            Axiom::HasKey(ax) => {
                push(&mut self.haskey_by_class, ax.class.clone(), ax.clone());
            }
            Axiom::DatatypeDefinition(ax) => {
                push(
                    &mut self.datatype_definition_by_datatype,
                    ax.datatype.clone(),
                    ax.clone(),
                );
            }
            Axiom::Rule(ax) => {
                self.rules.push(ax.clone());
            }
            _ => {}
        }
    }

    /// Deregister a single axiom from the index.
    pub fn remove_axiom(&mut self, axiom: &Axiom) {
        if let Some(vec) = self.axioms_by_type.get_mut(&axiom.axiom_type()) {
            vec.retain(|a| a != axiom);
            if vec.is_empty() {
                self.axioms_by_type.remove(&axiom.axiom_type());
            }
        }

        match axiom {
            Axiom::SubClassOf(ax) => {
                remove(&mut self.subclass_by_sub, &ax.subclass, &ax.superclass);
                remove(&mut self.subclass_by_super, &ax.superclass, &ax.subclass);
            }
            Axiom::EquivalentClasses(ax) => {
                remove_all_pairs(&mut self.equivalent_by_class, &ax.classes);
            }
            Axiom::ClassAssertion(ax) => {
                remove(
                    &mut self.class_assertion_by_class,
                    &ax.class,
                    &ax.individual,
                );
                remove(&mut self.class_assertion_by_ind, &ax.individual, &ax.class);
            }
            Axiom::ObjectPropertyDomain(ax) => {
                remove(&mut self.domain_by_prop, &ax.property, &ax.domain);
            }
            Axiom::ObjectPropertyRange(ax) => {
                remove(&mut self.range_by_prop, &ax.property, &ax.range);
            }
            Axiom::DisjointClasses(ax) => {
                remove_all_pairs(&mut self.disjoint_by_class, &ax.classes);
            }
            Axiom::SubObjectPropertyOf(ax) => {
                remove(
                    &mut self.sub_object_property_by_sub,
                    &ax.sub_property,
                    &ax.super_property,
                );
                remove(
                    &mut self.sub_object_property_by_super,
                    &ax.super_property,
                    &ax.sub_property,
                );
            }
            Axiom::EquivalentObjectProperties(ax) => {
                remove_all_pairs(
                    &mut self.equivalent_object_properties_by_prop,
                    &ax.properties,
                );
            }
            Axiom::DisjointObjectProperties(ax) => {
                remove_all_pairs(&mut self.disjoint_object_properties_by_prop, &ax.properties);
            }
            Axiom::InverseObjectProperties(ax) => {
                remove(
                    &mut self.inverse_object_properties_by_prop,
                    &ax.property1,
                    &ax.property2,
                );
                remove(
                    &mut self.inverse_object_properties_by_prop,
                    &ax.property2,
                    &ax.property1,
                );
            }
            Axiom::SubDataPropertyOf(ax) => {
                remove(
                    &mut self.sub_data_property_by_sub,
                    &ax.sub_property,
                    &ax.super_property,
                );
                remove(
                    &mut self.sub_data_property_by_super,
                    &ax.super_property,
                    &ax.sub_property,
                );
            }
            Axiom::EquivalentDataProperties(ax) => {
                remove_all_pairs(&mut self.equivalent_data_properties_by_prop, &ax.properties);
            }
            Axiom::DisjointDataProperties(ax) => {
                remove_all_pairs(&mut self.disjoint_data_properties_by_prop, &ax.properties);
            }
            Axiom::DataPropertyDomain(ax) => {
                remove(
                    &mut self.data_property_domain_by_prop,
                    &ax.property,
                    &ax.domain,
                );
            }
            Axiom::DataPropertyRange(ax) => {
                remove(
                    &mut self.data_property_range_by_prop,
                    &ax.property,
                    &ax.range,
                );
            }
            Axiom::FunctionalObjectProperty(ax) => {
                self.functional_object_properties.remove(&ax.property);
            }
            Axiom::InverseFunctionalObjectProperty(ax) => {
                self.inverse_functional_object_properties
                    .remove(&ax.property);
            }
            Axiom::ReflexiveObjectProperty(ax) => {
                self.reflexive_object_properties.remove(&ax.property);
            }
            Axiom::IrreflexiveObjectProperty(ax) => {
                self.irreflexive_object_properties.remove(&ax.property);
            }
            Axiom::SymmetricObjectProperty(ax) => {
                self.symmetric_object_properties.remove(&ax.property);
            }
            Axiom::AsymmetricObjectProperty(ax) => {
                self.asymmetric_object_properties.remove(&ax.property);
            }
            Axiom::TransitiveObjectProperty(ax) => {
                self.transitive_object_properties.remove(&ax.property);
            }
            Axiom::FunctionalDataProperty(ax) => {
                self.functional_data_properties.remove(&ax.property);
            }
            Axiom::SameIndividual(ax) => {
                remove_all_pairs(&mut self.same_individual_by_ind, &ax.individuals);
            }
            Axiom::DifferentIndividuals(ax) => {
                remove_all_pairs(&mut self.different_individuals_by_ind, &ax.individuals);
            }
            Axiom::NegativeObjectPropertyAssertion(ax) => {
                remove_axiom_val(
                    &mut self.negative_object_assertions_by_source,
                    &ax.source,
                    ax,
                );
            }
            Axiom::NegativeDataPropertyAssertion(ax) => {
                remove_axiom_val(
                    &mut self.negative_data_assertions_by_ind,
                    &ax.individual,
                    ax,
                );
            }
            Axiom::HasKey(ax) => {
                remove_axiom_val(&mut self.haskey_by_class, &ax.class, ax);
            }
            Axiom::DatatypeDefinition(ax) => {
                remove_axiom_val(&mut self.datatype_definition_by_datatype, &ax.datatype, ax);
            }
            Axiom::Rule(ax) => {
                self.rules.retain(|r| r != ax);
            }
            _ => {}
        }
    }

    /// Clear the whole index.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Return every indexed axiom of the given type.
    #[must_use]
    pub fn get(&self, axiom_type: AxiomType) -> &[Axiom] {
        self.axioms_by_type
            .get(&axiom_type)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Return every indexed axiom, grouped by type.
    #[must_use]
    pub fn by_type(&self) -> &HashMap<AxiomType, Vec<Axiom>> {
        &self.axioms_by_type
    }

    // ── Typed query helpers ─────────────────────────────────────────────────

    /// All `SubObjectPropertyOf` axioms (including property-chain axioms).
    #[must_use]
    pub fn sub_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::SubObjectPropertyOf)
    }

    /// All `SubDataPropertyOf` axioms.
    #[must_use]
    pub fn sub_data_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::SubDataPropertyOf)
    }

    /// All `EquivalentObjectProperties` axioms.
    #[must_use]
    pub fn equivalent_object_properties_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::EquivalentObjectProperties)
    }

    /// All `EquivalentDataProperties` axioms.
    #[must_use]
    pub fn equivalent_data_properties_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::EquivalentDataProperties)
    }

    /// All `DisjointObjectProperties` axioms.
    #[must_use]
    pub fn disjoint_object_properties_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DisjointObjectProperties)
    }

    /// All `DisjointDataProperties` axioms.
    #[must_use]
    pub fn disjoint_data_properties_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DisjointDataProperties)
    }

    /// All `InverseObjectProperties` axioms.
    #[must_use]
    pub fn inverse_object_properties_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::InverseObjectProperties)
    }

    /// All `ObjectPropertyDomain` axioms.
    #[must_use]
    pub fn object_property_domain_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::ObjectPropertyDomain)
    }

    /// All `ObjectPropertyRange` axioms.
    #[must_use]
    pub fn object_property_range_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::ObjectPropertyRange)
    }

    /// All `DataPropertyDomain` axioms.
    #[must_use]
    pub fn data_property_domain_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DataPropertyDomain)
    }

    /// All `DataPropertyRange` axioms.
    #[must_use]
    pub fn data_property_range_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DataPropertyRange)
    }

    /// All `FunctionalObjectProperty` axioms.
    #[must_use]
    pub fn functional_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::FunctionalObjectProperty)
    }

    /// All `InverseFunctionalObjectProperty` axioms.
    #[must_use]
    pub fn inverse_functional_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::InverseFunctionalObjectProperty)
    }

    /// All `ReflexiveObjectProperty` axioms.
    #[must_use]
    pub fn reflexive_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::ReflexiveObjectProperty)
    }

    /// All `IrreflexiveObjectProperty` axioms.
    #[must_use]
    pub fn irreflexive_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::IrreflexiveObjectProperty)
    }

    /// All `SymmetricObjectProperty` axioms.
    #[must_use]
    pub fn symmetric_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::SymmetricObjectProperty)
    }

    /// All `AsymmetricObjectProperty` axioms.
    #[must_use]
    pub fn asymmetric_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::AsymmetricObjectProperty)
    }

    /// All `TransitiveObjectProperty` axioms.
    #[must_use]
    pub fn transitive_object_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::TransitiveObjectProperty)
    }

    /// All `FunctionalDataProperty` axioms.
    #[must_use]
    pub fn functional_data_property_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::FunctionalDataProperty)
    }

    /// All `SameIndividual` axioms.
    #[must_use]
    pub fn same_individual_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::SameIndividual)
    }

    /// All `DifferentIndividuals` axioms.
    #[must_use]
    pub fn different_individuals_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DifferentIndividuals)
    }

    /// All `NegativeObjectPropertyAssertion` axioms.
    #[must_use]
    pub fn negative_object_property_assertion_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::NegativeObjectPropertyAssertion)
    }

    /// All `NegativeDataPropertyAssertion` axioms.
    #[must_use]
    pub fn negative_data_property_assertion_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::NegativeDataPropertyAssertion)
    }

    /// All `HasKey` axioms.
    #[must_use]
    pub fn has_key_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::HasKey)
    }

    /// All `DatatypeDefinition` axioms.
    #[must_use]
    pub fn datatype_definition_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::DatatypeDefinition)
    }

    /// All SWRL `Rule` axioms.
    #[must_use]
    pub fn rule_axioms(&self) -> &[Axiom] {
        self.get(AxiomType::Rule)
    }

    // ── Bidirectional lookups (class reasoning) ─────────────────────────────

    /// Look up all direct superclasses of a class expression.
    #[must_use]
    pub fn direct_superclasses(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.subclass_by_sub
            .get(class)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Look up all direct subclasses of a class expression.
    #[must_use]
    pub fn direct_subclasses(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.subclass_by_super
            .get(class)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Look up all equivalent classes.
    #[must_use]
    pub fn equivalent_classes(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.equivalent_by_class
            .get(class)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
    }
}

// ── Small internal helpers ────────────────────────────────────────────────────

fn push<K, V>(map: &mut HashMap<K, Vec<V>>, key: K, value: V)
where
    K: std::hash::Hash + Eq,
{
    map.entry(key).or_default().push(value);
}

fn remove<K, V>(map: &mut HashMap<K, Vec<V>>, key: &K, value: &V)
where
    K: std::hash::Hash + Eq,
    V: PartialEq,
{
    if let Some(vec) = map.get_mut(key) {
        vec.retain(|v| v != value);
        if vec.is_empty() {
            map.remove(key);
        }
    }
}

/// For a symmetric axiom (EquivalentClasses, DisjointClasses, SameIndividual,
/// DifferentIndividuals, Equivalent/DisjointObjectProperties, ...) add each
/// element as a key pointing at every *other* element.
fn add_all_pairs<K>(map: &mut HashMap<K, Vec<K>>, values: &[K])
where
    K: std::hash::Hash + Eq + Clone,
{
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            if i != j {
                push(map, a.clone(), b.clone());
            }
        }
    }
}

fn remove_all_pairs<K>(map: &mut HashMap<K, Vec<K>>, values: &[K])
where
    K: std::hash::Hash + Eq + Clone + PartialEq,
{
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            if i != j {
                remove(map, a, b);
            }
        }
    }
}

/// Remove a specific stored axiom value from a per-key list.
fn remove_axiom_val<K, V>(map: &mut HashMap<K, Vec<V>>, key: &K, axiom: &V)
where
    K: std::hash::Hash + Eq,
    V: PartialEq,
{
    if let Some(vec) = map.get_mut(key) {
        vec.retain(|v| v != axiom);
        if vec.is_empty() {
            map.remove(key);
        }
    }
}
