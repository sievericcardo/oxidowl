//! Manchester Syntax Renderer (Serializer)
//!
//! Renders OWL ontologies in Manchester OWL Syntax.
//! Complements the existing `ManchesterParser` in `manchester.rs`.

use crate::ontology::{
    Annotation, AnnotationValue, ClassExpression, DataPropertyExpression,
    DataRange, Individual, Literal, ObjectPropertyExpression, Ontology,
};
use crate::ontology::axioms::{
    Axiom, DisjointClassesAxiom, DisjointUnionAxiom,
    Entity, EquivalentClassesAxiom, HasKeyAxiom,
    SubClassOfAxiom,
};
use crate::Result;
use std::fmt::Write;

// ── ManchesterRenderer ───────────────────────────────────────────────────────

/// Configuration for the Manchester Syntax renderer.
#[derive(Debug, Clone)]
pub struct ManchesterRendererConfig {
    pub indent: usize,
    pub include_comments: bool,
    pub use_short_iris: bool,
}

impl Default for ManchesterRendererConfig {
    fn default() -> Self {
        Self {
            indent: 2,
            include_comments: false,
            use_short_iris: true,
        }
    }
}

/// Renders ontologies in Manchester OWL Syntax.
#[derive(Debug, Clone, Default)]
pub struct ManchesterRenderer;

impl ManchesterRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Serialize an ontology to Manchester Syntax string.
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let config = ManchesterRendererConfig::default();
        self.serialize_with_config(ontology, &config)
    }

    /// Serialize with a specific configuration.
    pub fn serialize_with_config(
        &self,
        ontology: &Ontology,
        config: &ManchesterRendererConfig,
    ) -> Result<String> {
        let mut buf = String::with_capacity(4096);

        if let Some(iri) = ontology.get_iri() {
            writeln!(buf, "Ontology: <{iri}>\n").ok();
        }

        if let Some(viri) = &ontology.version_iri {
            writeln!(buf, "VersionIRI: <{viri}>\n").ok();
        }

        for imp in &ontology.imports {
            writeln!(buf, "Import: <{imp}>\n").ok();
        }

        for ann in &ontology.annotations {
            writeln!(buf, "Annotations: {}", self.render_annotation(ann)).ok();
        }

        let prefix = " ".repeat(config.indent);

        // Group axioms by kind for frame-style output
        self.render_declarations(ontology, &mut buf, &prefix);
        self.render_class_frames(ontology, &mut buf, &prefix);
        self.render_object_property_frames(ontology, &mut buf, &prefix);
        self.render_data_property_frames(ontology, &mut buf, &prefix);
        self.render_individual_frames(ontology, &mut buf, &prefix);
        self.render_swrl_rules(ontology, &mut buf, &prefix);

        Ok(buf)
    }

    // ── Frame renderers ──────────────────────────────────────────────────

    fn render_declarations(&self, ontology: &Ontology, buf: &mut String, _prefix: &str) {
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(decl) = axiom {
                let _ = match &decl.entity {
                    Entity::Class(iri) => writeln!(buf, "Class: {iri}\n"),
                    Entity::ObjectProperty(iri) => writeln!(buf, "ObjectProperty: {iri}\n"),
                    Entity::DataProperty(iri) => writeln!(buf, "DataProperty: {iri}\n"),
                    Entity::AnnotationProperty(iri) => writeln!(buf, "AnnotationProperty: {iri}\n"),
                    Entity::NamedIndividual(iri) => writeln!(buf, "Individual: {iri}\n"),
                    Entity::Datatype(iri) => writeln!(buf, "Datatype: {iri}\n"),
                };
            }
        }
    }

    fn render_class_frames(&self, ontology: &Ontology, buf: &mut String, prefix: &str) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::SubClassOf(a) => Some(format!(
                    "{prefix}SubClassOf: {}",
                    self.render_subclassof(a)
                )),
                Axiom::EquivalentClasses(a) => Some(format!(
                    "{prefix}EquivalentTo: {}",
                    self.render_equivalent_classes(a)
                )),
                Axiom::DisjointClasses(a) => Some(format!(
                    "{prefix}DisjointWith: {}",
                    self.render_disjoint_classes(a)
                )),
                Axiom::DisjointUnion(a) => Some(format!(
                    "{prefix}DisjointUnionOf: {}",
                    self.render_disjoint_union(a)
                )),
                Axiom::HasKey(a) => Some(format!(
                    "{prefix}HasKey: {} {}",
                    self.render_ce(&a.class),
                    self.render_has_key(a)
                )),
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{s}\n");
            }
        }
    }

    fn render_object_property_frames(&self, ontology: &Ontology, buf: &mut String, prefix: &str) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::ObjectPropertyDomain(a) => Some(format!(
                    "{prefix}Domain: {}",
                    self.render_ce(&a.domain)
                )),
                Axiom::ObjectPropertyRange(a) => Some(format!(
                    "{prefix}Range: {}",
                    self.render_ce(&a.range)
                )),
                Axiom::SubObjectPropertyOf(a) => Some(format!(
                    "{prefix}SubPropertyOf: {}",
                    self.render_ope(&a.super_property)
                )),
                Axiom::EquivalentObjectProperties(a) => {
                    let props: Vec<String> = a.properties.iter().map(|p| self.render_ope(p)).collect();
                    Some(format!("{prefix}EquivalentTo: {}", props.join(", ")))
                }
                Axiom::InverseObjectProperties(a) => Some(format!(
                    "{prefix}InverseOf: {}",
                    self.render_ope(&a.property2)
                )),
                Axiom::DisjointObjectProperties(a) => {
                    let props: Vec<String> = a.properties.iter().map(|p| self.render_ope(p)).collect();
                    Some(format!("{prefix}DisjointWith: {}", props.join(", ")))
                }
                Axiom::FunctionalObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Functional"))
                }
                Axiom::InverseFunctionalObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: InverseFunctional"))
                }
                Axiom::ReflexiveObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Reflexive"))
                }
                Axiom::IrreflexiveObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Irreflexive"))
                }
                Axiom::SymmetricObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Symmetric"))
                }
                Axiom::AsymmetricObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Asymmetric"))
                }
                Axiom::TransitiveObjectProperty(_) => {
                    Some(format!("{prefix}Characteristics: Transitive"))
                }
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{s}\n");
            }
        }
    }

    fn render_data_property_frames(&self, ontology: &Ontology, buf: &mut String, prefix: &str) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::DataPropertyDomain(a) => Some(format!(
                    "{prefix}Domain: {}",
                    self.render_ce(&a.domain)
                )),
                Axiom::DataPropertyRange(a) => Some(format!(
                    "{prefix}Range: {}",
                    self.render_datarange(&a.range)
                )),
                Axiom::SubDataPropertyOf(a) => Some(format!(
                    "{prefix}SubPropertyOf: {}",
                    self.render_dpe(&a.super_property)
                )),
                Axiom::EquivalentDataProperties(a) => {
                    let props: Vec<String> = a.properties.iter().map(|p| self.render_dpe(p)).collect();
                    Some(format!("{prefix}EquivalentTo: {}", props.join(", ")))
                }
                Axiom::DisjointDataProperties(a) => {
                    let props: Vec<String> = a.properties.iter().map(|p| self.render_dpe(p)).collect();
                    Some(format!("{prefix}DisjointWith: {}", props.join(", ")))
                }
                Axiom::FunctionalDataProperty(_) => {
                    Some(format!("{prefix}Characteristics: Functional"))
                }
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{s}\n");
            }
        }
    }

    fn render_individual_frames(&self, ontology: &Ontology, buf: &mut String, prefix: &str) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::ClassAssertion(a) => Some(format!(
                    "{prefix}Types: {}",
                    self.render_ce(&a.class)
                )),
                Axiom::ObjectPropertyAssertion(a) => Some(format!(
                    "{prefix}Facts: {} {}",
                    self.render_ope(&a.property),
                    self.render_individual(&a.target)
                )),
                Axiom::DataPropertyAssertion(a) => Some(format!(
                    "{prefix}Facts: {} {}",
                    self.render_dpe(&a.property),
                    self.render_literal(&a.value)
                )),
                Axiom::SameIndividual(a) => {
                    let inds: Vec<String> = a.individuals.iter().map(|i| self.render_individual(i)).collect();
                    Some(format!("{prefix}SameAs: {}", inds.join(", ")))
                }
                Axiom::DifferentIndividuals(a) => {
                    let inds: Vec<String> = a.individuals.iter().map(|i| self.render_individual(i)).collect();
                    Some(format!("{prefix}DifferentFrom: {}", inds.join(", ")))
                }
                Axiom::NegativeObjectPropertyAssertion(a) => Some(format!(
                    "{prefix}Facts: not ({} {})",
                    self.render_ope(&a.property),
                    self.render_individual(&a.target)
                )),
                Axiom::NegativeDataPropertyAssertion(a) => Some(format!(
                    "{prefix}Facts: not ({} {})",
                    self.render_dpe(&a.property),
                    self.render_literal(&a.value)
                )),
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{s}\n");
            }
        }
    }

    fn render_swrl_rules(&self, ontology: &Ontology, buf: &mut String, prefix: &str) {
        for axiom in ontology.axioms() {
            if let Axiom::Rule(rule_ax) = axiom {
                let head: Vec<String> = rule_ax
                    .rule
                    .head
                    .iter()
                    .map(|a| self.render_swrl_atom(a))
                    .collect();
                let body: Vec<String> = rule_ax
                    .rule
                    .body
                    .iter()
                    .map(|a| self.render_swrl_atom(a))
                    .collect();
                let _ = writeln!(
                    buf,
                    "{prefix}Rule: {} :- {}",
                    head.join(", "),
                    body.join(", ")
                );
            }
        }
    }

    // ── Axiom sub-renderers ──────────────────────────────────────────────

    fn render_subclassof(&self, ax: &SubClassOfAxiom) -> String {
        format!(
            "{} SubClassOf: {}",
            self.render_ce(&ax.subclass),
            self.render_ce(&ax.superclass)
        )
    }

    fn render_equivalent_classes(&self, ax: &EquivalentClassesAxiom) -> String {
        ax.classes
            .iter()
            .map(|c| self.render_ce(c))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_disjoint_classes(&self, ax: &DisjointClassesAxiom) -> String {
        ax.classes
            .iter()
            .map(|c| self.render_ce(c))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_disjoint_union(&self, ax: &DisjointUnionAxiom) -> String {
        let parts: Vec<String> = ax.disjoint_classes.iter().map(|c| self.render_ce(c)).collect();
        format!("{} DisjointUnionOf: {}", self.render_ce(&ax.class), parts.join(", "))
    }

    fn render_has_key(&self, ax: &HasKeyAxiom) -> String {
        let mut parts: Vec<String> = ax.object_properties.iter().map(|p| self.render_ope(p)).collect();
        parts.extend(ax.data_properties.iter().map(|p| self.render_dpe(p)));
        parts.join(", ")
    }

    // ── Class expression rendering ───────────────────────────────────────

    /// Render a class expression in Manchester syntax.
    pub fn render_ce(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(cls) => cls.iri.to_string(),
            ClassExpression::ObjectIntersectionOf(ops) => {
                let inner: Vec<String> = ops.iter().map(|op| self.render_ce_with_prec(op, 1)).collect();
                inner.join(" and ")
            }
            ClassExpression::ObjectUnionOf(ops) => {
                let inner: Vec<String> = ops.iter().map(|op| self.render_ce_with_prec(op, 0)).collect();
                inner.join(" or ")
            }
            ClassExpression::ObjectComplementOf(inner) => {
                format!("not {}", self.render_ce_with_prec(inner, 0))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!("{} some {}", self.render_ope(property), self.render_ce_with_prec(filler, 0))
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!("{} only {}", self.render_ope(property), self.render_ce_with_prec(filler, 0))
            }
            ClassExpression::ObjectHasValue { property, value } => {
                format!("{} value {}", self.render_ope(property), self.render_individual(value))
            }
            ClassExpression::ObjectHasSelf { property } => {
                format!("{} Self", self.render_ope(property))
            }
            ClassExpression::ObjectMinCardinality { property, cardinality, filler } => {
                format!(
                    "{} min {} {}",
                    self.render_ope(property),
                    cardinality,
                    self.render_ce_with_prec(filler, 0)
                )
            }
            ClassExpression::ObjectMaxCardinality { property, cardinality, filler } => {
                format!(
                    "{} max {} {}",
                    self.render_ope(property),
                    cardinality,
                    self.render_ce_with_prec(filler, 0)
                )
            }
            ClassExpression::ObjectExactCardinality { property, cardinality, filler } => {
                format!(
                    "{} exactly {} {}",
                    self.render_ope(property),
                    cardinality,
                    self.render_ce_with_prec(filler, 0)
                )
            }
            ClassExpression::ObjectOneOf(inds) => {
                let inner: Vec<String> = inds.iter().map(|i| self.render_individual(i)).collect();
                format!("{{{}}}", inner.join(", "))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                format!("{} some {}", self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                format!("{} only {}", self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataHasValue { property, value } => {
                format!("{} value {}", self.render_dpe(property), self.render_literal(value))
            }
            ClassExpression::DataMinCardinality { property, cardinality, filler } => {
                format!(
                    "{} min {} {}",
                    self.render_dpe(property),
                    cardinality,
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataMaxCardinality { property, cardinality, filler } => {
                format!(
                    "{} max {} {}",
                    self.render_dpe(property),
                    cardinality,
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataExactCardinality { property, cardinality, filler } => {
                format!(
                    "{} exactly {} {}",
                    self.render_dpe(property),
                    cardinality,
                    self.render_datarange(filler)
                )
            }
        }
    }

    fn render_ce_with_prec(&self, expr: &ClassExpression, min_prec: u8) -> String {
        let needs_parens = match expr {
            ClassExpression::ObjectUnionOf(_) => min_prec > 0,
            ClassExpression::ObjectIntersectionOf(_) => min_prec > 1,
            ClassExpression::ObjectComplementOf(_) => min_prec > 0,
            _ => false,
        };
        let rendered = self.render_ce(expr);
        if needs_parens {
            format!("({rendered})")
        } else {
            rendered
        }
    }

    // ── Helper renderers ─────────────────────────────────────────────────

    fn render_ope(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(p) => p.iri.to_string(),
            ObjectPropertyExpression::InverseObjectProperty(p) => format!("inverse ({})", p.iri),
            ObjectPropertyExpression::PropertyChain(chain) => {
                let parts: Vec<String> = chain.iter().map(|p| self.render_ope(p)).collect();
                format!("o {}", parts.join(" o "))
            }
        }
    }

    fn render_dpe(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(p) => p.iri.to_string(),
        }
    }

    fn render_individual(&self, ind: &Individual) -> String {
        match ind {
            Individual::Named(n) => n.iri.to_string(),
            Individual::Anonymous(a) => format!("_:{a:?}"),
        }
    }

    fn render_literal(&self, lit: &Literal) -> String {
        if let Some(lang) = &lit.language {
            format!("\"{}\"@{lang}", lit.value)
        } else if let Some(dt) = &lit.datatype {
            format!("\"{}\"^^<{dt}>", lit.value)
        } else {
            format!("\"{}\"", lit.value)
        }
    }

    fn render_datarange(&self, range: &DataRange) -> String {
        match range {
            DataRange::Datatype(iri) => iri.to_string(),
            DataRange::DataIntersectionOf(rs) => {
                let parts: Vec<String> = rs.iter().map(|r| self.render_datarange(r)).collect();
                parts.join(" and ")
            }
            DataRange::DataUnionOf(rs) => {
                let parts: Vec<String> = rs.iter().map(|r| self.render_datarange(r)).collect();
                parts.join(" or ")
            }
            DataRange::DataComplementOf(r) => {
                format!("not {}", self.render_datarange(r))
            }
            DataRange::DataOneOf(lits) => {
                let parts: Vec<String> = lits.iter().map(|l| self.render_literal(l)).collect();
                format!("{{{}}}", parts.join(", "))
            }
            DataRange::DatatypeRestriction { datatype, restrictions } => {
                let facets: Vec<String> = restrictions
                    .iter()
                    .map(|f| format!("{} {}", f.facet, self.render_literal(&f.value)))
                    .collect();
                format!("{}[{}]", datatype, facets.join(", "))
            }
        }
    }

    fn render_annotation(&self, ann: &Annotation) -> String {
        let val = match &ann.value {
            AnnotationValue::IRI(iri) => iri.to_string(),
            AnnotationValue::Literal(lit) => self.render_literal(lit),
            AnnotationValue::AnonymousIndividual(a) => format!("_:{a:?}"),
        };
        format!("{} {}", ann.property.iri, val)
    }

    fn render_swrl_atom(&self, atom: &crate::ontology::axioms::SWRLAtom) -> String {
        match atom {
            crate::ontology::axioms::SWRLAtom::ClassAtom { predicate, argument } => {
                format!("ClassAtom({}, {})", self.render_ce(predicate), self.render_swrl_arg_i(argument))
            }
            crate::ontology::axioms::SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                format!(
                    "ObjectPropertyAtom({} {} {})",
                    self.render_ope(predicate),
                    self.render_swrl_arg_i(first_argument),
                    self.render_swrl_arg_i(second_argument)
                )
            }
            crate::ontology::axioms::SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                format!(
                    "DataPropertyAtom({} {} {})",
                    self.render_dpe(predicate),
                    self.render_swrl_arg_i(first_argument),
                    self.render_swrl_arg_d(second_argument)
                )
            }
            crate::ontology::axioms::SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| self.render_swrl_arg_d(a)).collect();
                format!("BuiltInAtom({}, {})", predicate, args.join(", "))
            }
            crate::ontology::axioms::SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => {
                format!(
                    "SameIndividualAtom({}, {})",
                    self.render_swrl_arg_i(first_argument),
                    self.render_swrl_arg_i(second_argument)
                )
            }
            crate::ontology::axioms::SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                format!(
                    "DifferentIndividualsAtom({}, {})",
                    self.render_swrl_arg_i(first_argument),
                    self.render_swrl_arg_i(second_argument)
                )
            }
            crate::ontology::axioms::SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => {
                format!(
                    "DataRangeAtom({}, {})",
                    self.render_datarange(predicate),
                    self.render_swrl_arg_d(argument)
                )
            }
        }
    }

    fn render_swrl_arg_i(&self, arg: &crate::ontology::axioms::SWRLIArgument) -> String {
        match arg {
            crate::ontology::axioms::SWRLIArgument::Individual(ind) => self.render_individual(ind),
            crate::ontology::axioms::SWRLIArgument::Variable(var) => format!("?{}", var.iri),
        }
    }

    fn render_swrl_arg_d(&self, arg: &crate::ontology::axioms::SWRLDArgument) -> String {
        match arg {
            crate::ontology::axioms::SWRLDArgument::Literal(lit) => self.render_literal(lit),
            crate::ontology::axioms::SWRLDArgument::Variable(var) => format!("?{}", var.iri),
        }
    }
}
