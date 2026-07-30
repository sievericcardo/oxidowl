use std::sync::Arc;

use oxidowl::factory::providers::AxiomCreationProvider;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::DataFactory;

/// DSL-style data factory helper for concise test ontology construction.
///
/// Models the pattern from OWL API v5's `DF.java` — provides short-form
/// constructors for entities, class expressions, axioms, annotations,
/// and literals so that test code reads declaratively.
pub struct DF {
    pub df: DataFactory,
}

impl Default for DF {
    fn default() -> Self {
        Self::new()
    }
}

impl DF {
    pub fn new() -> Self {
        DF {
            df: DataFactory::new(),
        }
    }

    // ── IRI helpers ─────────────────────────────────────────────────────────

    pub fn iri<S: AsRef<str>>(&self, s: S) -> IRI {
        IRI::new(s.as_ref())
    }

    // ── Entity constructors ─────────────────────────────────────────────────

    pub fn class<S: AsRef<str>>(&self, iri: S) -> Class {
        self.df.get_class(&IRI::new(iri.as_ref()))
    }

    pub fn object_property<S: AsRef<str>>(&self, iri: S) -> ObjectProperty {
        self.df.get_object_property(&IRI::new(iri.as_ref()))
    }

    pub fn data_property<S: AsRef<str>>(&self, iri: S) -> DataProperty {
        self.df.get_data_property(&IRI::new(iri.as_ref()))
    }

    pub fn named_individual<S: AsRef<str>>(&self, iri: S) -> NamedIndividual {
        self.df.get_named_individual(&IRI::new(iri.as_ref()))
    }

    pub fn annotation_property<S: AsRef<str>>(&self, iri: S) -> AnnotationProperty {
        self.df.get_annotation_property(&IRI::new(iri.as_ref()))
    }

    pub fn anonymous_individual(&self) -> AnonymousIndividual {
        self.df.get_anonymous_individual()
    }

    pub fn datatype_iri<S: AsRef<str>>(&self, iri: S) -> Datatype {
        self.df.get_datatype(&IRI::new(iri.as_ref()))
    }

    pub fn entity(&self, iri: &IRI, entity_type: &EntityType) -> Entity {
        self.df.get_entity(iri, entity_type)
    }

    // ── Individual helpers ──────────────────────────────────────────────────

    pub fn named<S: AsRef<str>>(&self, iri: S) -> Individual {
        Individual::Named(self.named_individual(iri))
    }

    pub fn anon(&self) -> Individual {
        Individual::Anonymous(self.anonymous_individual())
    }

    // ── Property expression helpers ─────────────────────────────────────────

    pub fn obj_prop<S: AsRef<str>>(&self, iri: S) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(self.object_property(iri))
    }

    pub fn inv_obj_prop<S: AsRef<str>>(&self, iri: S) -> ObjectPropertyExpression {
        ObjectPropertyExpression::InverseObjectProperty(self.object_property(iri))
    }

    pub fn data_prop<S: AsRef<str>>(&self, iri: S) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(self.data_property(iri))
    }

    // ── Class expression constructors ───────────────────────────────────────

    pub fn class_ce<S: AsRef<str>>(&self, iri: S) -> ClassExpression {
        ClassExpression::Class(self.class(iri))
    }

    pub fn owl_thing(&self) -> ClassExpression {
        ClassExpression::Class(Class::thing())
    }

    pub fn owl_nothing(&self) -> ClassExpression {
        ClassExpression::Class(Class::nothing())
    }

    pub fn intersection_of(&self, operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectIntersectionOf(operands)
    }

    pub fn union_of(&self, operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectUnionOf(operands)
    }

    pub fn complement_of(&self, operand: ClassExpression) -> ClassExpression {
        ClassExpression::ObjectComplementOf(Box::new(operand))
    }

    pub fn one_of(&self, individuals: Vec<Individual>) -> ClassExpression {
        ClassExpression::ObjectOneOf(individuals)
    }

    pub fn some_values_from(
        &self,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    pub fn all_values_from(
        &self,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    pub fn has_value(
        &self,
        property: ObjectPropertyExpression,
        value: Individual,
    ) -> ClassExpression {
        ClassExpression::ObjectHasValue { property, value }
    }

    pub fn has_self(&self, property: ObjectPropertyExpression) -> ClassExpression {
        ClassExpression::ObjectHasSelf { property }
    }

    pub fn min_cardinality(
        &self,
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMinCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    pub fn max_cardinality(
        &self,
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMaxCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    pub fn exact_cardinality(
        &self,
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectExactCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    pub fn data_some_values_from(
        &self,
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataSomeValuesFrom { property, filler }
    }

    pub fn data_all_values_from(
        &self,
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataAllValuesFrom { property, filler }
    }

    pub fn data_has_value(
        &self,
        property: DataPropertyExpression,
        value: Literal,
    ) -> ClassExpression {
        ClassExpression::DataHasValue { property, value }
    }

    // ── Literal constructors ────────────────────────────────────────────────

    pub fn literal<S: AsRef<str>>(&self, value: S) -> Literal {
        Literal::new(value.as_ref().to_string())
    }

    pub fn int_literal(&self, value: i64) -> Literal {
        self.df.get_integer_literal(value)
    }

    pub fn bool_literal(&self, value: bool) -> Literal {
        self.df.get_boolean_literal(value)
    }

    pub fn double_literal(&self, value: f64) -> Literal {
        self.df.get_double_literal(value)
    }

    pub fn lang_literal<S: AsRef<str>>(&self, value: S, lang: S) -> Literal {
        Literal::with_language(value.as_ref().to_string(), lang.as_ref().to_string())
    }

    pub fn typed_literal<S: AsRef<str>>(&self, value: S, datatype: &str) -> Literal {
        Literal::with_datatype(
            value.as_ref().to_string(),
            IRI::new(datatype),
        )
    }

    // ── DataRange helpers ───────────────────────────────────────────────────

    pub fn data_range_dt<S: AsRef<str>>(&self, iri: S) -> DataRange {
        DataRange::Datatype(IRI::new(iri.as_ref()))
    }

    pub fn data_range_integer(&self) -> DataRange {
        DataRange::Datatype(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))
    }

    pub fn data_range_boolean(&self) -> DataRange {
        DataRange::Datatype(IRI::new("http://www.w3.org/2001/XMLSchema#boolean"))
    }

    pub fn top_datatype(&self) -> DataRange {
        DataRange::Datatype(IRI::new("http://www.w3.org/2000/01/rdf-schema#Literal"))
    }

    // ── Annotation helpers ──────────────────────────────────────────────────

    pub fn ann<S: AsRef<str>>(
        &self,
        property_iri: S,
        value_text: S,
    ) -> Annotation {
        let prop = self.annotation_property(property_iri);
        let val = AnnotationValue::Literal(self.literal(value_text));
        Annotation::new(prop, val, vec![])
    }

    pub fn ann_iri<S1: AsRef<str>, S2: AsRef<str>>(
        &self,
        property_iri: S1,
        value_iri: S2,
    ) -> Annotation {
        let prop = self.annotation_property(property_iri);
        let val = AnnotationValue::IRI(IRI::new(value_iri.as_ref()));
        Annotation::new(prop, val, vec![])
    }

    pub fn ann_with_annotations(
        &self,
        property: AnnotationProperty,
        value: AnnotationValue,
        annotations: Vec<Annotation>,
    ) -> Annotation {
        Annotation::new(property, value, annotations)
    }

    /// rdfs:label shortcut
    pub fn rdfs_label<S: AsRef<str>>(&self, value: S) -> Annotation {
        let prop = AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label"),
        };
        let val = AnnotationValue::Literal(self.literal(value));
        Annotation::new(prop, val, vec![])
    }

    /// rdfs:comment shortcut
    pub fn rdfs_comment<S: AsRef<str>>(&self, value: S) -> Annotation {
        let prop = AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#comment"),
        };
        let val = AnnotationValue::Literal(self.literal(value));
        Annotation::new(prop, val, vec![])
    }

    // ── Axiom constructors ──────────────────────────────────────────────────

    pub fn next_id(&self) -> AxiomId {
        self.df.next_id()
    }

    pub fn declaration_axiom(&self, entity: Entity) -> Axiom {
        let ax = self.df.get_declaration_axiom(entity);
        Axiom::Declaration(ax)
    }

    pub fn sub_class_of(
        &self,
        subclass: ClassExpression,
        superclass: ClassExpression,
    ) -> Axiom {
        let ax = self.df.get_sub_class_of_axiom(subclass, superclass);
        Axiom::SubClassOf(ax)
    }

    pub fn equivalent_classes(&self, classes: Vec<ClassExpression>) -> Axiom {
        let ax = self.df.get_equivalent_classes_axiom(classes);
        Axiom::EquivalentClasses(ax)
    }

    pub fn disjoint_classes(&self, classes: Vec<ClassExpression>) -> Axiom {
        let ax = self.df.get_disjoint_classes_axiom(classes);
        Axiom::DisjointClasses(ax)
    }

    pub fn disjoint_union(
        &self,
        class: ClassExpression,
        disjoint_classes: Vec<ClassExpression>,
    ) -> Axiom {
        let ax = self
            .df
            .get_disjoint_union_axiom(class, disjoint_classes);
        Axiom::DisjointUnion(ax)
    }

    pub fn class_assertion(
        &self,
        class: ClassExpression,
        individual: Individual,
    ) -> Axiom {
        let ax = self
            .df
            .get_class_assertion_axiom(class, individual);
        Axiom::ClassAssertion(ax)
    }

    pub fn object_property_assertion(
        &self,
        property: ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    ) -> Axiom {
        let ax = self
            .df
            .get_object_property_assertion_axiom(property, source, target);
        Axiom::ObjectPropertyAssertion(ax)
    }

    pub fn data_property_assertion(
        &self,
        property: DataPropertyExpression,
        individual: Individual,
        value: Literal,
    ) -> Axiom {
        let ax = self
            .df
            .get_data_property_assertion_axiom(property, individual, value);
        Axiom::DataPropertyAssertion(ax)
    }

    pub fn negative_object_property_assertion(
        &self,
        property: ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_negative_object_property_assertion_axiom(
            &self.df,
            property,
            source,
            target,
            vec![],
        );
        Axiom::NegativeObjectPropertyAssertion(ax)
    }

    pub fn negative_data_property_assertion(
        &self,
        property: DataPropertyExpression,
        individual: Individual,
        value: Literal,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_negative_data_property_assertion_axiom(
            &self.df,
            property,
            individual,
            value,
            vec![],
        );
        Axiom::NegativeDataPropertyAssertion(ax)
    }

    pub fn same_individual(&self, individuals: Vec<Individual>) -> Axiom {
        let ax = self.df.get_same_individual_axiom(individuals);
        Axiom::SameIndividual(ax)
    }

    pub fn different_individuals(&self, individuals: Vec<Individual>) -> Axiom {
        let ax = self.df.get_different_individuals_axiom(individuals);
        Axiom::DifferentIndividuals(ax)
    }

    // ── Object Property Axioms ─────────────────────────────────────────────

    pub fn sub_object_property_of(
        &self,
        sub: ObjectPropertyExpression,
        sup: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self.df.get_sub_object_property_of_axiom(sub, sup);
        Axiom::SubObjectPropertyOf(ax)
    }

    pub fn equivalent_object_properties(
        &self,
        properties: Vec<ObjectPropertyExpression>,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_equivalent_object_properties_axiom(
            &self.df,
            properties,
            vec![],
        );
        Axiom::EquivalentObjectProperties(ax)
    }

    pub fn functional_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_functional_object_property_axiom(property);
        Axiom::FunctionalObjectProperty(ax)
    }

    pub fn transitive_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_transitive_object_property_axiom(property);
        Axiom::TransitiveObjectProperty(ax)
    }

    pub fn symmetric_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_symmetric_object_property_axiom(property);
        Axiom::SymmetricObjectProperty(ax)
    }

    pub fn asymmetric_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_asymmetric_object_property_axiom(property);
        Axiom::AsymmetricObjectProperty(ax)
    }

    pub fn reflexive_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_reflexive_object_property_axiom(property);
        Axiom::ReflexiveObjectProperty(ax)
    }

    pub fn irreflexive_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_irreflexive_object_property_axiom(property);
        Axiom::IrreflexiveObjectProperty(ax)
    }

    pub fn inverse_functional_object_property(
        &self,
        property: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_inverse_functional_object_property_axiom(property);
        Axiom::InverseFunctionalObjectProperty(ax)
    }

    pub fn inverse_object_properties(
        &self,
        p1: ObjectPropertyExpression,
        p2: ObjectPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_inverse_object_properties_axiom(p1, p2);
        Axiom::InverseObjectProperties(ax)
    }

    pub fn object_property_domain(
        &self,
        property: ObjectPropertyExpression,
        domain: ClassExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_object_property_domain_axiom(property, domain);
        Axiom::ObjectPropertyDomain(ax)
    }

    pub fn object_property_range(
        &self,
        property: ObjectPropertyExpression,
        range: ClassExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_object_property_range_axiom(property, range);
        Axiom::ObjectPropertyRange(ax)
    }

    pub fn disjoint_object_properties(
        &self,
        properties: Vec<ObjectPropertyExpression>,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_disjoint_object_properties_axiom(
            &self.df,
            properties,
            vec![],
        );
        Axiom::DisjointObjectProperties(ax)
    }

    // ── Data Property Axioms ────────────────────────────────────────────────

    pub fn sub_data_property_of(
        &self,
        sub: DataPropertyExpression,
        sup: DataPropertyExpression,
    ) -> Axiom {
        let ax = self.df.get_sub_data_property_of_axiom(sub, sup);
        Axiom::SubDataPropertyOf(ax)
    }

    pub fn equivalent_data_properties(
        &self,
        properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_equivalent_data_properties_axiom(
            &self.df,
            properties,
            vec![],
        );
        Axiom::EquivalentDataProperties(ax)
    }

    pub fn functional_data_property(
        &self,
        property: DataPropertyExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_functional_data_property_axiom(property);
        Axiom::FunctionalDataProperty(ax)
    }

    pub fn data_property_domain(
        &self,
        property: DataPropertyExpression,
        domain: ClassExpression,
    ) -> Axiom {
        let ax = self
            .df
            .get_data_property_domain_axiom(property, domain);
        Axiom::DataPropertyDomain(ax)
    }

    pub fn data_property_range(
        &self,
        property: DataPropertyExpression,
        range: DataRange,
    ) -> Axiom {
        let ax = self
            .df
            .get_data_property_range_axiom(property, range);
        Axiom::DataPropertyRange(ax)
    }

    pub fn disjoint_data_properties(
        &self,
        properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_disjoint_data_properties_axiom(
            &self.df,
            properties,
            vec![],
        );
        Axiom::DisjointDataProperties(ax)
    }

    // ── Annotation Axioms ──────────────────────────────────────────────────

    pub fn annotation_assertion(
        &self,
        property: AnnotationProperty,
        subject: IRI,
        value_text: &str,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_annotation_assertion_axiom(
            &self.df,
            property,
            AnnotationSubject::IRI(subject),
            AnnotationValue::Literal(self.literal(value_text)),
            vec![],
        );
        Axiom::AnnotationAssertion(ax)
    }

    pub fn annotation_assertion_iri(
        &self,
        property: AnnotationProperty,
        subject: IRI,
        value_iri: IRI,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_annotation_assertion_axiom(
            &self.df,
            property,
            AnnotationSubject::IRI(subject),
            AnnotationValue::IRI(value_iri),
            vec![],
        );
        Axiom::AnnotationAssertion(ax)
    }

    pub fn sub_annotation_property_of(
        &self,
        sub: AnnotationProperty,
        sup: AnnotationProperty,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_sub_annotation_property_of_axiom(
            &self.df, sub, sup, vec![],
        );
        Axiom::SubAnnotationPropertyOf(ax)
    }

    // ── HasKey ──────────────────────────────────────────────────────────────

    pub fn has_key(
        &self,
        class: ClassExpression,
        object_properties: Vec<ObjectPropertyExpression>,
        data_properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        let ax = AxiomCreationProvider::make_has_key_axiom(
            &self.df,
            class,
            object_properties,
            data_properties,
            vec![],
        );
        Axiom::HasKey(ax)
    }

    // ── Ontology builders ───────────────────────────────────────────────────

    pub fn new_ontology(&self) -> Ontology {
        Ontology::new()
    }

    pub fn new_ontology_with_iri<S: AsRef<str>>(&self, iri: S) -> Ontology {
        let mut o = Ontology::new();
        o.set_iri(IRI::new(iri.as_ref()));
        o
    }

    /// Build an ontology from a list of axioms, auto-declaring entities
    pub fn build_ontology(&self, axioms: Vec<Axiom>) -> Ontology {
        let mut o = Ontology::new();
        for ax in axioms {
            o.add_axiom(ax);
        }
        o
    }

    /// Build an ontology from axioms and set its IRI
    pub fn build_ontology_with_iri<S: AsRef<str>>(
        &self,
        iri: S,
        axioms: Vec<Axiom>,
    ) -> Ontology {
        let mut o = Ontology::new();
        o.set_iri(IRI::new(iri.as_ref()));
        for ax in axioms {
            o.add_axiom(ax);
        }
        o
    }

    // ── Convenience: auto-declare entities ─────────────────────────────────

    /// Declare all undeclared non-builtin entities in the ontology.
    /// Scans axioms directly instead of relying on `signature()`.
    pub fn auto_declare(&self, ontology: &mut Ontology) {
        let mut declared_classes = std::collections::HashSet::new();
        let mut declared_obj_props = std::collections::HashSet::new();
        let mut declared_data_props = std::collections::HashSet::new();
        let mut declared_inds = std::collections::HashSet::new();
        let mut declared_ann_props = std::collections::HashSet::new();

        for ax in ontology.axioms() {
            match ax {
                Axiom::Declaration(d) => match &d.entity {
                    Entity::Class(iri) => { declared_classes.insert(iri.clone()); }
                    Entity::ObjectProperty(iri) => { declared_obj_props.insert(iri.clone()); }
                    Entity::DataProperty(iri) => { declared_data_props.insert(iri.clone()); }
                    Entity::NamedIndividual(iri) => { declared_inds.insert(iri.clone()); }
                    Entity::AnnotationProperty(iri) => { declared_ann_props.insert(iri.clone()); }
                    _ => {}
                },
                _ => {}
            }
        }

        for ax in ontology.axioms().to_vec() {
            self.collect_entity_iris(&ax, &mut declared_classes, &mut declared_obj_props,
                &mut declared_data_props, &mut declared_inds, &mut declared_ann_props);
        }

        for iri in &declared_classes {
            if !iri.is_owl_thing() && !iri.is_owl_nothing() {
                ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
                    id: self.df.next_id(),
                    entity: Entity::Class(iri.clone()),
                }));
            }
        }
        for iri in &declared_obj_props {
            ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
                id: self.df.next_id(),
                entity: Entity::ObjectProperty(iri.clone()),
            }));
        }
        for iri in &declared_data_props {
            ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
                id: self.df.next_id(),
                entity: Entity::DataProperty(iri.clone()),
            }));
        }
        for iri in &declared_inds {
            ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
                id: self.df.next_id(),
                entity: Entity::NamedIndividual(iri.clone()),
            }));
        }
        for iri in &declared_ann_props {
            ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
                id: self.df.next_id(),
                entity: Entity::AnnotationProperty(iri.clone()),
            }));
        }
    }

    fn collect_entity_iris(
        &self,
        axiom: &Axiom,
        classes: &mut std::collections::HashSet<IRI>,
        obj_props: &mut std::collections::HashSet<IRI>,
        data_props: &mut std::collections::HashSet<IRI>,
        individuals: &mut std::collections::HashSet<IRI>,
        ann_props: &mut std::collections::HashSet<IRI>,
    ) {
        match axiom {
            Axiom::SubClassOf(a) => {
                self.collect_ce_iris(&a.subclass, classes, obj_props, data_props, individuals, ann_props);
                self.collect_ce_iris(&a.superclass, classes, obj_props, data_props, individuals, ann_props);
            }
            Axiom::EquivalentClasses(a) => {
                for ce in &a.classes {
                    self.collect_ce_iris(ce, classes, obj_props, data_props, individuals, ann_props);
                }
            }
            Axiom::DisjointClasses(a) => {
                for ce in &a.classes {
                    self.collect_ce_iris(ce, classes, obj_props, data_props, individuals, ann_props);
                }
            }
            Axiom::DisjointUnion(a) => {
                self.collect_ce_iris(&a.class, classes, obj_props, data_props, individuals, ann_props);
                for ce in &a.disjoint_classes {
                    self.collect_ce_iris(ce, classes, obj_props, data_props, individuals, ann_props);
                }
            }
            Axiom::ClassAssertion(a) => {
                self.collect_ce_iris(&a.class, classes, obj_props, data_props, individuals, ann_props);
                self.collect_ind_iris(&a.individual, individuals);
            }
            Axiom::ObjectPropertyAssertion(a) => {
                self.collect_ope_iris(&a.property, obj_props);
                self.collect_ind_iris(&a.source, individuals);
                self.collect_ind_iris(&a.target, individuals);
            }
            Axiom::DataPropertyAssertion(a) => {
                self.collect_dpe_iris(&a.property, data_props);
                self.collect_ind_iris(&a.individual, individuals);
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                self.collect_ope_iris(&a.property, obj_props);
                self.collect_ind_iris(&a.source, individuals);
                self.collect_ind_iris(&a.target, individuals);
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                self.collect_dpe_iris(&a.property, data_props);
                self.collect_ind_iris(&a.individual, individuals);
            }
            Axiom::SubObjectPropertyOf(a) => {
                self.collect_ope_iris(&a.sub_property, obj_props);
                self.collect_ope_iris(&a.super_property, obj_props);
            }
            Axiom::EquivalentObjectProperties(a) => {
                for p in &a.properties { self.collect_ope_iris(p, obj_props); }
            }
            Axiom::DisjointObjectProperties(a) => {
                for p in &a.properties { self.collect_ope_iris(p, obj_props); }
            }
            Axiom::InverseObjectProperties(a) => {
                self.collect_ope_iris(&a.property1, obj_props);
                self.collect_ope_iris(&a.property2, obj_props);
            }
            Axiom::ObjectPropertyDomain(a) => {
                self.collect_ope_iris(&a.property, obj_props);
                self.collect_ce_iris(&a.domain, classes, obj_props, data_props, individuals, ann_props);
            }
            Axiom::ObjectPropertyRange(a) => {
                self.collect_ope_iris(&a.property, obj_props);
                self.collect_ce_iris(&a.range, classes, obj_props, data_props, individuals, ann_props);
            }
            Axiom::FunctionalObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::InverseFunctionalObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::ReflexiveObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::IrreflexiveObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::SymmetricObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::AsymmetricObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::TransitiveObjectProperty(a) => { self.collect_ope_iris(&a.property, obj_props); }
            Axiom::SubDataPropertyOf(a) => {
                self.collect_dpe_iris(&a.sub_property, data_props);
                self.collect_dpe_iris(&a.super_property, data_props);
            }
            Axiom::EquivalentDataProperties(a) => {
                for p in &a.properties { self.collect_dpe_iris(p, data_props); }
            }
            Axiom::DisjointDataProperties(a) => {
                for p in &a.properties { self.collect_dpe_iris(p, data_props); }
            }
            Axiom::DataPropertyDomain(a) => {
                self.collect_dpe_iris(&a.property, data_props);
                self.collect_ce_iris(&a.domain, classes, obj_props, data_props, individuals, ann_props);
            }
            Axiom::DataPropertyRange(a) => {
                self.collect_dpe_iris(&a.property, data_props);
            }
            Axiom::FunctionalDataProperty(a) => { self.collect_dpe_iris(&a.property, data_props); }
            Axiom::SameIndividual(a) => {
                for ind in &a.individuals { self.collect_ind_iris(ind, individuals); }
            }
            Axiom::DifferentIndividuals(a) => {
                for ind in &a.individuals { self.collect_ind_iris(ind, individuals); }
            }
            Axiom::AnnotationAssertion(a) => {
                ann_props.insert(a.property.iri.clone());
            }
            Axiom::SubAnnotationPropertyOf(a) => {
                ann_props.insert(a.sub_property.iri.clone());
                ann_props.insert(a.super_property.iri.clone());
            }
            Axiom::AnnotationPropertyDomain(a) => {
                ann_props.insert(a.property.iri.clone());
            }
            Axiom::AnnotationPropertyRange(a) => {
                ann_props.insert(a.property.iri.clone());
            }
            Axiom::HasKey(a) => {
                self.collect_ce_iris(&a.class, classes, obj_props, data_props, individuals, ann_props);
                for p in &a.object_properties { self.collect_ope_iris(p, obj_props); }
                for p in &a.data_properties { self.collect_dpe_iris(p, data_props); }
            }
            _ => {}
        }
    }

    fn collect_ce_iris(
        &self,
        ce: &ClassExpression,
        classes: &mut std::collections::HashSet<IRI>,
        obj_props: &mut std::collections::HashSet<IRI>,
        data_props: &mut std::collections::HashSet<IRI>,
        individuals: &mut std::collections::HashSet<IRI>,
        ann_props: &mut std::collections::HashSet<IRI>,
    ) {
        match ce {
            ClassExpression::Class(c) => {
                classes.insert(c.iri.clone());
            }
            ClassExpression::ObjectIntersectionOf(ces)
            | ClassExpression::ObjectUnionOf(ces) => {
                for c in ces {
                    self.collect_ce_iris(c, classes, obj_props, data_props, individuals, ann_props);
                }
            }
            ClassExpression::ObjectComplementOf(c) => {
                self.collect_ce_iris(c, classes, obj_props, data_props, individuals, ann_props);
            }
            ClassExpression::ObjectOneOf(inds) => {
                for ind in inds { self.collect_ind_iris(ind, individuals); }
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler }
            | ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.collect_ope_iris(property, obj_props);
                self.collect_ce_iris(filler, classes, obj_props, data_props, individuals, ann_props);
            }
            ClassExpression::ObjectHasValue { property, value } => {
                self.collect_ope_iris(property, obj_props);
                self.collect_ind_iris(value, individuals);
            }
            ClassExpression::ObjectHasSelf { property } => {
                self.collect_ope_iris(property, obj_props);
            }
            ClassExpression::ObjectMinCardinality { property, filler, .. }
            | ClassExpression::ObjectMaxCardinality { property, filler, .. }
            | ClassExpression::ObjectExactCardinality { property, filler, .. } => {
                self.collect_ope_iris(property, obj_props);
                self.collect_ce_iris(filler, classes, obj_props, data_props, individuals, ann_props);
            }
            ClassExpression::DataSomeValuesFrom { property, .. }
            | ClassExpression::DataAllValuesFrom { property, .. } => {
                self.collect_dpe_iris(property, data_props);
            }
            ClassExpression::DataHasValue { property, .. } => {
                self.collect_dpe_iris(property, data_props);
            }
            ClassExpression::DataMinCardinality { property, .. }
            | ClassExpression::DataMaxCardinality { property, .. }
            | ClassExpression::DataExactCardinality { property, .. } => {
                self.collect_dpe_iris(property, data_props);
            }
        }
    }

    fn collect_ope_iris(
        &self,
        ope: &ObjectPropertyExpression,
        obj_props: &mut std::collections::HashSet<IRI>,
    ) {
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => { obj_props.insert(p.iri.clone()); }
            ObjectPropertyExpression::InverseObjectProperty(p) => { obj_props.insert(p.iri.clone()); }
            ObjectPropertyExpression::PropertyChain(props) => {
                for p in props { self.collect_ope_iris(p, obj_props); }
            }
        }
    }

    fn collect_dpe_iris(
        &self,
        dpe: &DataPropertyExpression,
        data_props: &mut std::collections::HashSet<IRI>,
    ) {
        match dpe {
            DataPropertyExpression::DataProperty(p) => { data_props.insert(p.iri.clone()); }
        }
    }

    fn collect_ind_iris(
        &self,
        ind: &Individual,
        individuals: &mut std::collections::HashSet<IRI>,
    ) {
        if let Individual::Named(ni) = ind {
            individuals.insert(ni.iri.clone());
        }
    }

    // ── Common ontology fixtures ────────────────────────────────────────────

    /// A simple A ⊑ B, B ⊑ C chain with a class assertion
    pub fn simple_chain_ontology(&self) -> Ontology {
        let a = self.class_ce("http://ex.org/A");
        let b = self.class_ce("http://ex.org/B");
        let c = self.class_ce("http://ex.org/C");
        let i = self.named("http://ex.org/ind");
        let mut o = Ontology::new();
        o.set_iri(IRI::new("http://ex.org/TestOnt"));
        o.add_axiom(self.sub_class_of(a.clone(), b.clone()));
        o.add_axiom(self.sub_class_of(b.clone(), c.clone()));
        o.add_axiom(self.class_assertion(a.clone(), i));
        self.auto_declare(&mut o);
        o
    }

    /// An ontology with a contradictory pattern: C ⊑ A, C ⊑ ¬A
    pub fn contradictory_ontology(&self) -> Ontology {
        let a = self.class_ce("http://ex.org/A");
        let c = self.class_ce("http://ex.org/C");
        let not_a = self.complement_of(a.clone());
        let mut o = Ontology::new();
        o.set_iri(IRI::new("http://ex.org/Contradictory"));
        o.add_axiom(self.sub_class_of(c.clone(), a.clone()));
        o.add_axiom(self.sub_class_of(c.clone(), not_a));
        self.auto_declare(&mut o);
        o
    }

    // ── Creates Entity from IRI and type ────────────────────────────────────

    pub fn make_entity<S: AsRef<str>>(
        &self,
        iri: S,
        entity_type: EntityType,
    ) -> Entity {
        let iri = IRI::new(iri.as_ref());
        match entity_type {
            EntityType::Class => Entity::Class(iri),
            EntityType::ObjectProperty => Entity::ObjectProperty(iri),
            EntityType::DataProperty => Entity::DataProperty(iri),
            EntityType::AnnotationProperty => Entity::AnnotationProperty(iri),
            EntityType::NamedIndividual => Entity::NamedIndividual(iri),
            EntityType::Datatype => Entity::Datatype(iri),
        }
    }

    /// Wrap as Arc<RwLock<Ontology>>
    pub fn onto_ref(&self, ontology: Ontology) -> OntologyRef {
        Arc::new(std::sync::RwLock::new(ontology))
    }
}

// ── Common vocabulary constants ─────────────────────────────────────────────

/// Predefined IRIs matching the OWL API DF.java convention
pub mod pred {
    use super::*;

    pub const EX: &str = "http://example.org/";
    pub const TEST: &str = "http://test.org/";
    pub const OWL: &str = "http://www.w3.org/2002/07/owl#";
    pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    pub const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
    pub const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    pub fn ex_iri(local: &str) -> IRI {
        IRI::new(&format!("{EX}{local}"))
    }

    pub fn test_iri(local: &str) -> IRI {
        IRI::new(&format!("{TEST}{local}"))
    }
}
