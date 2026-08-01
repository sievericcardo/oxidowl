//! KRSS / KRSS2 Parser and Renderer.
//!
//! KRSS (Knowledge Representation System Specification) is an older
//! DL reasoner format. Supports both KRSS and KRSS2 variants.

use crate::Result;
use crate::ontology::axioms::{
    Axiom, ClassAssertionAxiom, DeclarationAxiom, DifferentIndividualsAxiom, DisjointClassesAxiom,
    Entity, EquivalentClassesAxiom, FunctionalObjectPropertyAxiom,
    InverseFunctionalObjectPropertyAxiom, IrreflexiveObjectPropertyAxiom,
    ObjectPropertyAssertionAxiom, ReflexiveObjectPropertyAxiom, SameIndividualAxiom,
    SubClassOfAxiom, SymmetricObjectPropertyAxiom, TransitiveObjectPropertyAxiom,
};
use crate::ontology::individuals::NamedIndividual;
use crate::ontology::{
    Class, ClassExpression, DataPropertyExpression, DataRange, Individual, ObjectProperty,
    ObjectPropertyExpression, Ontology,
};
use std::fmt::Write;

/// KRSS variant selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum KRSSVariant {
    #[default]
    KRSS,
    KRSS2,
}


// ── KRSS Parser ──────────────────────────────────────────────────────────────

use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct KRSSParser {
    input: String,
    pos: Cell<usize>,
    variant: KRSSVariant,
}

impl Default for KRSSParser {
    fn default() -> Self {
        Self::new(KRSSVariant::KRSS)
    }
}

impl KRSSParser {
    #[must_use]
    pub fn new(variant: KRSSVariant) -> Self {
        Self {
            input: String::new(),
            pos: Cell::new(0),
            variant,
        }
    }

    pub fn parse(&mut self, content: &str) -> Result<Ontology> {
        self.input = content.to_string();
        self.pos.set(0);
        let mut ontology = Ontology::new();
        while self.pos.get() < self.input.len() {
            self.skip_ws();
            if self.pos.get() >= self.input.len() {
                break;
            }
            if self.peek() != Some('(') {
                self.pos.set(self.pos.get() + 1);
                continue;
            }
            self.consume(); // '('
            self.skip_ws();
            let cmd = self.parse_symbol();
            match cmd.as_str() {
                "define-primitive-concept" => self.parse_define_primitive(&mut ontology, false)?,
                "define-concept" => self.parse_define_primitive(&mut ontology, true)?,
                "define-primitive-role" => self.parse_role_def(&mut ontology, false)?,
                "define-role" => self.parse_role_def(&mut ontology, true)?,
                "implies" => self.parse_implies(&mut ontology)?,
                "equivalent" => self.parse_equivalent(&mut ontology)?,
                "disjoint" => self.parse_disjoint(&mut ontology)?,
                "instance" => self.parse_instance(&mut ontology)?,
                "related" => self.parse_related(&mut ontology)?,
                "equal" => self.parse_equal(&mut ontology)?,
                "distinct" => self.parse_distinct(&mut ontology)?,
                "transitive" | "symmetric" | "functional" | "inverse-functional" | "reflexive"
                | "irreflexive"
                    if self.variant == KRSSVariant::KRSS2 =>
                {
                    self.parse_property_characteristic(&mut ontology, &cmd)?;
                }
                _ => { /* skip unknown */ }
            }
            self.skip_to_closing_paren();
        }
        Ok(ontology)
    }

    fn parse_define_primitive(&self, ontology: &mut Ontology, defined: bool) -> Result<()> {
        self.skip_ws();
        let name = self.parse_symbol();
        self.skip_ws();
        if self.peek() == Some(')') {
            return Ok(());
        }
        let expr = self.parse_concept()?;
        let cls = Class {
            iri: crate::ontology::IRI::new(&name),
        };
        ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
            id: 1,
            entity: Entity::Class(cls.iri.clone()),
        }));
        if defined {
            ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                id: 1,
                classes: vec![ClassExpression::Class(cls), expr],
                annotations: vec![],
            }));
        } else {
            ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                id: 1,
                subclass: ClassExpression::Class(cls),
                superclass: expr,
                annotations: vec![],
            }));
        }
        Ok(())
    }

    fn parse_role_def(&self, ontology: &mut Ontology, _defined: bool) -> Result<()> {
        self.skip_ws();
        let name = self.parse_symbol();
        let iri = crate::ontology::IRI::new(&name);
        ontology.add_axiom(Axiom::Declaration(DeclarationAxiom {
            id: 1,
            entity: Entity::ObjectProperty(iri.clone()),
        }));
        self.skip_ws();
        while self.peek() == Some(':') {
            self.consume(); // ':'
            self.skip_ws();
            let parent = self.parse_symbol();
            ontology.add_axiom(Axiom::SubObjectPropertyOf(
                crate::ontology::axioms::SubObjectPropertyOfAxiom {
                    id: 1,
                    sub_property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: iri.clone(),
                    }),
                    super_property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: crate::ontology::IRI::new(&parent),
                    }),
                    annotations: vec![],
                },
            ));
            self.skip_ws();
        }
        Ok(())
    }

    fn parse_implies(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let sub = self.parse_concept()?;
        self.skip_ws();
        let sup = self.parse_concept()?;
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        }));
        Ok(())
    }

    fn parse_equivalent(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let a = self.parse_concept()?;
        self.skip_ws();
        let b = self.parse_concept()?;
        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 1,
            classes: vec![a, b],
            annotations: vec![],
        }));
        Ok(())
    }

    fn parse_disjoint(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let a = self.parse_concept()?;
        self.skip_ws();
        let b = self.parse_concept()?;
        ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
            id: 1,
            classes: vec![a, b],
            annotations: vec![],
        }));
        Ok(())
    }

    fn parse_instance(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let ind_name = self.parse_symbol();
        self.skip_ws();
        let concept = self.parse_concept()?;
        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            class: concept,
            individual: Individual::Named(NamedIndividual {
                iri: crate::ontology::IRI::new(&ind_name),
            }),
            annotations: vec![],
        }));
        Ok(())
    }

    fn parse_related(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let src = self.parse_symbol();
        self.skip_ws();
        let role = self.parse_symbol();
        self.skip_ws();
        let tgt = self.parse_symbol();
        ontology.add_axiom(Axiom::ObjectPropertyAssertion(
            ObjectPropertyAssertionAxiom {
                id: 1,
                property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                    iri: crate::ontology::IRI::new(&role),
                }),
                source: Individual::Named(NamedIndividual {
                    iri: crate::ontology::IRI::new(&src),
                }),
                target: Individual::Named(NamedIndividual {
                    iri: crate::ontology::IRI::new(&tgt),
                }),
                annotations: vec![],
            },
        ));
        Ok(())
    }

    fn parse_equal(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let mut inds = Vec::new();
        while self.peek() != Some(')') && self.pos.get() < self.input.len() {
            let name = self.parse_symbol();
            if !name.is_empty() {
                inds.push(Individual::Named(NamedIndividual {
                    iri: crate::ontology::IRI::new(&name),
                }));
            }
            self.skip_ws();
        }
        if inds.len() >= 2 {
            ontology.add_axiom(Axiom::SameIndividual(SameIndividualAxiom {
                id: 1,
                individuals: inds,
                annotations: vec![],
            }));
        }
        Ok(())
    }

    fn parse_distinct(&self, ontology: &mut Ontology) -> Result<()> {
        self.skip_ws();
        let mut inds = Vec::new();
        while self.peek() != Some(')') && self.pos.get() < self.input.len() {
            let name = self.parse_symbol();
            if !name.is_empty() {
                inds.push(Individual::Named(NamedIndividual {
                    iri: crate::ontology::IRI::new(&name),
                }));
            }
            self.skip_ws();
        }
        if inds.len() >= 2 {
            ontology.add_axiom(Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                id: 1,
                individuals: inds,
                annotations: vec![],
            }));
        }
        Ok(())
    }

    fn parse_property_characteristic(
        &self,
        ontology: &mut Ontology,
        characteristic: &str,
    ) -> Result<()> {
        self.skip_ws();
        let name = self.parse_symbol();
        let iri = crate::ontology::IRI::new(&name);
        let prop_expr = ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri });
        let axiom = match characteristic {
            "transitive" => Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom {
                id: 1,
                property: prop_expr,
                annotations: vec![],
            }),
            "symmetric" => Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom {
                id: 1,
                property: prop_expr,
                annotations: vec![],
            }),
            "functional" => Axiom::FunctionalObjectProperty(FunctionalObjectPropertyAxiom {
                id: 1,
                property: prop_expr,
                annotations: vec![],
            }),
            "inverse-functional" => {
                Axiom::InverseFunctionalObjectProperty(InverseFunctionalObjectPropertyAxiom {
                    id: 1,
                    property: prop_expr,
                    annotations: vec![],
                })
            }
            "reflexive" => Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom {
                id: 1,
                property: prop_expr,
                annotations: vec![],
            }),
            "irreflexive" => Axiom::IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom {
                id: 1,
                property: prop_expr,
                annotations: vec![],
            }),
            _ => return Ok(()),
        };
        ontology.add_axiom(axiom);
        Ok(())
    }

    // ── Concept expression parser ────────────────────────────────────────

    fn parse_concept(&self) -> Result<ClassExpression> {
        self.skip_ws();
        if self.peek() == Some('(') {
            self.consume();
            self.skip_ws();
            let op = self.parse_symbol();
            let result = match op.as_str() {
                "and" => {
                    let a = self.parse_concept()?;
                    let b = self.parse_concept()?;
                    ClassExpression::ObjectIntersectionOf(vec![a, b])
                }
                "or" => {
                    let a = self.parse_concept()?;
                    let b = self.parse_concept()?;
                    ClassExpression::ObjectUnionOf(vec![a, b])
                }
                "not" => {
                    let inner = self.parse_concept()?;
                    ClassExpression::ObjectComplementOf(Box::new(inner))
                }
                "some" => {
                    let role = self.parse_symbol();
                    let filler = self.parse_concept()?;
                    ClassExpression::ObjectSomeValuesFrom {
                        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                            iri: crate::ontology::IRI::new(&role),
                        }),
                        filler: Box::new(filler),
                    }
                }
                "all" => {
                    let role = self.parse_symbol();
                    let filler = self.parse_concept()?;
                    ClassExpression::ObjectAllValuesFrom {
                        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                            iri: crate::ontology::IRI::new(&role),
                        }),
                        filler: Box::new(filler),
                    }
                }
                "at-least" => {
                    let n = self.parse_number();
                    let role = self.parse_symbol();
                    let filler = self.parse_concept()?;
                    ClassExpression::ObjectMinCardinality {
                        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                            iri: crate::ontology::IRI::new(&role),
                        }),
                        cardinality: n,
                        filler: Box::new(filler),
                    }
                }
                "at-most" => {
                    let n = self.parse_number();
                    let role = self.parse_symbol();
                    let filler = self.parse_concept()?;
                    ClassExpression::ObjectMaxCardinality {
                        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                            iri: crate::ontology::IRI::new(&role),
                        }),
                        cardinality: n,
                        filler: Box::new(filler),
                    }
                }
                "exactly" => {
                    let n = self.parse_number();
                    let role = self.parse_symbol();
                    let filler = self.parse_concept()?;
                    ClassExpression::ObjectExactCardinality {
                        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                            iri: crate::ontology::IRI::new(&role),
                        }),
                        cardinality: n,
                        filler: Box::new(filler),
                    }
                }
                "one-of" if self.variant == KRSSVariant::KRSS2 => {
                    let mut inds = Vec::new();
                    while self.peek() != Some(')') {
                        let name = self.parse_symbol();
                        if !name.is_empty() {
                            inds.push(Individual::Named(NamedIndividual {
                                iri: crate::ontology::IRI::new(&name),
                            }));
                        }
                        self.skip_ws();
                    }
                    ClassExpression::ObjectOneOf(inds)
                }
                "inverse" if self.variant == KRSSVariant::KRSS2 => {
                    let name = self.parse_symbol();
                    let _prop = ObjectPropertyExpression::InverseObjectProperty(ObjectProperty {
                        iri: crate::ontology::IRI::new(&name),
                    });
                    ClassExpression::Class(Class {
                        iri: crate::ontology::IRI::new("urn:dummy"),
                    })
                }
                _ => ClassExpression::Class(Class {
                    iri: crate::ontology::IRI::new(&op),
                }),
            };
            self.skip_to_closing_paren();
            Ok(result)
        } else {
            let name = self.parse_symbol();
            if name.is_empty() {
                Ok(ClassExpression::Class(Class::thing()))
            } else {
                Ok(ClassExpression::Class(Class {
                    iri: crate::ontology::IRI::new(&name),
                }))
            }
        }
    }

    fn parse_number(&self) -> u32 {
        self.skip_ws();
        let mut n: u32 = 0;
        while self.pos.get() < self.input.len() {
            let p = self.pos.get();
            let c = self.input.chars().nth(p).unwrap();
            if c.is_ascii_digit() {
                n = n * 10 + c.to_digit(10).unwrap();
                self.pos.set(p + c.len_utf8());
            } else {
                break;
            }
        }
        n
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn peek(&self) -> Option<char> {
        self.input[self.pos.get()..].chars().next()
    }

    fn consume(&self) {
        let p = self.pos.get();
        if p < self.input.len() {
            let c = self.input[p..].chars().next().unwrap();
            self.pos.set(p + c.len_utf8());
        }
    }

    fn skip_ws(&self) {
        while self.pos.get() < self.input.len() {
            let p = self.pos.get();
            let c = self.input.chars().nth(p).unwrap();
            if c.is_whitespace() {
                self.pos.set(p + c.len_utf8());
            } else {
                break;
            }
        }
    }

    fn skip_to_closing_paren(&self) {
        let mut depth = 1;
        while self.pos.get() < self.input.len() && depth > 0 {
            let p = self.pos.get();
            let c = self.input.chars().nth(p).unwrap();
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    self.pos.set(p + 1);
                    return;
                }
            }
            self.pos.set(p + c.len_utf8());
        }
    }

    fn parse_symbol(&self) -> String {
        self.skip_ws();
        let mut name = String::new();
        while self.pos.get() < self.input.len() {
            let p = self.pos.get();
            let c = self.input.chars().nth(p).unwrap();
            if c == '(' || c == ')' || c == ':' {
                break;
            }
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.pos.set(p + c.len_utf8());
            } else if c.is_whitespace() && !name.is_empty() {
                break;
            } else {
                self.pos.set(p + c.len_utf8());
                break;
            }
        }
        name
    }
}

// ── KRSS Renderer ────────────────────────────────────────────────────────────

/// Configuration for KRSS rendering.
#[derive(Debug, Clone)]
pub struct KRSSConfig {
    pub variant: KRSSVariant,
    pub indent: usize,
}

impl Default for KRSSConfig {
    fn default() -> Self {
        Self {
            variant: KRSSVariant::KRSS,
            indent: 2,
        }
    }
}

/// Renders ontologies in KRSS syntax.
pub struct KRSSRenderer {
    config: KRSSConfig,
}

impl KRSSRenderer {
    #[must_use]
    pub fn new(variant: KRSSVariant) -> Self {
        Self {
            config: KRSSConfig { variant, indent: 2 },
        }
    }

    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::with_capacity(4096);
        let _prefix = " ".repeat(self.config.indent);

        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(a) => {
                    let _ = writeln!(
                        buf,
                        "(implies {} {})",
                        self.render_ce(&a.subclass),
                        self.render_ce(&a.superclass)
                    );
                }
                Axiom::EquivalentClasses(a) if a.classes.len() >= 2 => {
                    let _ = writeln!(
                        buf,
                        "(equivalent {} {})",
                        self.render_ce(&a.classes[0]),
                        self.render_ce(&a.classes[1])
                    );
                }
                Axiom::DisjointClasses(a) if a.classes.len() >= 2 => {
                    let _ = writeln!(
                        buf,
                        "(disjoint {} {})",
                        self.render_ce(&a.classes[0]),
                        self.render_ce(&a.classes[1])
                    );
                }
                Axiom::ClassAssertion(a) => {
                    let _ = writeln!(
                        buf,
                        "(instance {} {})",
                        self.render_individual(&a.individual),
                        self.render_ce(&a.class)
                    );
                }
                Axiom::ObjectPropertyAssertion(a) => {
                    let _ = writeln!(
                        buf,
                        "(related {} {} {})",
                        self.render_individual(&a.source),
                        self.render_ope(&a.property),
                        self.render_individual(&a.target)
                    );
                }
                Axiom::SameIndividual(a) => {
                    let parts: Vec<String> = a
                        .individuals
                        .iter()
                        .map(|i| self.render_individual(i))
                        .collect();
                    let _ = writeln!(buf, "(equal {})", parts.join(" "));
                }
                Axiom::DifferentIndividuals(a) => {
                    let parts: Vec<String> = a
                        .individuals
                        .iter()
                        .map(|i| self.render_individual(i))
                        .collect();
                    let _ = writeln!(buf, "(distinct {})", parts.join(" "));
                }
                Axiom::Declaration(decl) => {
                    if let Entity::ObjectProperty(iri) = &decl.entity {
                        let _ =
                            writeln!(buf, "(define-primitive-role {name})", name = iri.as_str());
                    }
                }
                Axiom::TransitiveObjectProperty(a) if self.config.variant == KRSSVariant::KRSS2 => {
                    let _ = writeln!(buf, "(transitive {})", self.render_ope(&a.property));
                }
                Axiom::SymmetricObjectProperty(a) if self.config.variant == KRSSVariant::KRSS2 => {
                    let _ = writeln!(buf, "(symmetric {})", self.render_ope(&a.property));
                }
                Axiom::FunctionalObjectProperty(a) if self.config.variant == KRSSVariant::KRSS2 => {
                    let _ = writeln!(buf, "(functional {})", self.render_ope(&a.property));
                }
                Axiom::ReflexiveObjectProperty(a) if self.config.variant == KRSSVariant::KRSS2 => {
                    let _ = writeln!(buf, "(reflexive {})", self.render_ope(&a.property));
                }
                _ => {}
            }
        }
        Ok(buf)
    }

    // ── Expression rendering ─────────────────────────────────────────────

    fn render_ce(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(cls) => self.name(&cls.iri.to_string()),
            ClassExpression::ObjectIntersectionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                format!("(and {})", parts.join(" "))
            }
            ClassExpression::ObjectUnionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                format!("(or {})", parts.join(" "))
            }
            ClassExpression::ObjectComplementOf(inner) => {
                format!("(not {})", self.render_ce(inner))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!(
                    "(some {} {})",
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!(
                    "(all {} {})",
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectHasValue { property, value } => {
                format!(
                    "(some {} {})",
                    self.render_ope(property),
                    self.render_individual(value)
                )
            }
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "(at-least {} {} {})",
                    cardinality,
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "(at-most {} {} {})",
                    cardinality,
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "(exactly {} {} {})",
                    cardinality,
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectOneOf(inds) => {
                let parts: Vec<String> = inds.iter().map(|i| self.render_individual(i)).collect();
                format!("(one-of {})", parts.join(" "))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                format!(
                    "(some {} {})",
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                format!(
                    "(all {} {})",
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            _ => format!("(concept {})", self.name("unknown")),
        }
    }

    fn render_ope(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(p) => self.name(&p.iri.to_string()),
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                format!("(inv {})", self.name(&p.iri.to_string()))
            }
            _ => self.name("role"),
        }
    }

    fn render_dpe(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(p) => self.name(&p.iri.to_string()),
        }
    }

    fn render_individual(&self, ind: &Individual) -> String {
        match ind {
            Individual::Named(n) => self.name(&n.iri.to_string()),
            Individual::Anonymous(a) => a.id.clone(),
        }
    }

    fn render_datarange(&self, range: &DataRange) -> String {
        match range {
            DataRange::Datatype(iri) => self.name(&iri.to_string()),
            _ => "datatype".to_string(),
        }
    }

    fn name(&self, iri: &str) -> String {
        if let Some(fragment) = iri.rsplit('#').next()
            && !fragment.is_empty() && fragment.len() < iri.len() {
                return fragment.to_string();
            }
        if let Some(last) = iri.rsplit('/').next()
            && !last.is_empty() && last.len() < iri.len() {
                return last.to_string();
            }
        iri.to_string()
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Parse KRSS content into an ontology.
pub fn parse(content: &str) -> Result<Ontology> {
    let mut parser = KRSSParser::new(KRSSVariant::KRSS);
    parser.parse(content)
}

/// Parse KRSS2 content into an ontology.
pub fn parse_krss2(content: &str) -> Result<Ontology> {
    let mut parser = KRSSParser::new(KRSSVariant::KRSS2);
    parser.parse(content)
}

/// Serialize ontology to KRSS string.
pub fn render_to_string(ontology: &Ontology) -> Result<String> {
    let renderer = KRSSRenderer::new(KRSSVariant::KRSS);
    renderer.serialize(ontology)
}

/// Serialize ontology to KRSS2 string.
pub fn render_to_string_krss2(ontology: &Ontology) -> Result<String> {
    let renderer = KRSSRenderer::new(KRSSVariant::KRSS2);
    renderer.serialize(ontology)
}

/// Save ontology as KRSS to a file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let renderer = KRSSRenderer::new(KRSSVariant::KRSS);
    let content = renderer.serialize(ontology)?;
    std::fs::write(path, content)
        .map_err(|e| crate::Error::io(format!("Failed to write KRSS: {e}")))
}

/// Save ontology as KRSS2 to a file.
pub fn save_file_krss2<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let renderer = KRSSRenderer::new(KRSSVariant::KRSS2);
    let content = renderer.serialize(ontology)?;
    std::fs::write(path, content)
        .map_err(|e| crate::Error::io(format!("Failed to write KRSS2: {e}")))
}
