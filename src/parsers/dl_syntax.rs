//! DL Syntax Parser and Renderer.
//!
//! Implements parsing and rendering of the compact Description Logic
//! notation with both Unicode and ASCII modes.

use crate::ontology::{
    Class, ClassExpression, DataPropertyExpression, DataRange, Individual,
    Literal, ObjectProperty, ObjectPropertyExpression, Ontology,
};
use crate::ontology::axioms::{
    Axiom, ClassAssertionAxiom, DeclarationAxiom,
    Entity, EquivalentClassesAxiom,
    SubClassOfAxiom,
};
use crate::ontology::individuals::NamedIndividual;
use crate::Result;

// ── DL Syntax Parser ─────────────────────────────────────────────────────────

/// DL Syntax parser supporting Unicode and ASCII token variants.
#[derive(Debug, Clone)]
pub struct DLSyntaxParser {
    input: String,
    pos: usize,
}

impl Default for DLSyntaxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DLSyntaxParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: String::new(),
            pos: 0,
        }
    }

    pub fn parse(&mut self, content: &str) -> Result<Ontology> {
        self.input = content.to_string();
        self.pos = 0;
        let mut ontology = Ontology::new();
        self.skip_whitespace();
        while self.pos < self.input.len() {
            if let Ok(axiom) = self.parse_axiom() {
                ontology.add_axiom(axiom);
            }
            self.skip_whitespace();
            if self.pos < self.input.len() && !self.at_token() {
                self.pos += 1; // Skip unknown char
            }
        }
        Ok(ontology)
    }

    fn at_token(&self) -> bool {
        self.peek().is_some_and(|c| {
            c.is_alphanumeric() || c == '(' || c == '{'
                || c == '\u{2293}' || c == '\u{2294}' || c == '\u{00AC}'
                || c == '\u{2203}' || c == '\u{2200}' || c == '\u{2291}'
                || c == '\u{2264}' || c == '\u{2265}' || c == '\u{22A4}'
                || c == '\u{22A5}' || c == '\u{2261}'
        })
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.peek().unwrap();
            if c.is_whitespace() { self.pos += c.len_utf8(); } else { break; }
        }
    }

    fn parse_axiom(&mut self) -> Result<Axiom> {
        self.skip_whitespace();
        let lhs = self.parse_concept()?;
        self.skip_whitespace();

        let op = self.peek_tok();
        match op.as_deref() {
            Some("\u{2291}") | Some("sqsubseteq") => {
                self.consume_tok(&op.unwrap());
                let rhs = self.parse_concept()?;
                Ok(Axiom::SubClassOf(SubClassOfAxiom {
                    id: 1,
                    subclass: lhs,
                    superclass: rhs,
                    annotations: vec![],
                }))
            }
            Some("\u{2261}") | Some("equiv") => {
                self.consume_tok(&op.unwrap());
                let rhs = self.parse_concept()?;
                Ok(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                    id: 1,
                    classes: vec![lhs, rhs],
                    annotations: vec![],
                }))
            }
            Some("(") => {
                self.consume().unwrap();
                let ind_name = self.parse_name();
                self.skip_whitespace();
                self.expect_char(')')?;
                Ok(Axiom::ClassAssertion(ClassAssertionAxiom {
                    id: 1,
                    class: lhs,
                    individual: Individual::Named(NamedIndividual { iri: crate::ontology::IRI::new(&ind_name) }),
                    annotations: vec![],
                }))
            }
            _ => {
                let name = self.parse_name();
                if name.is_empty() {
                    Ok(Axiom::Declaration(DeclarationAxiom {
                        id: 1,
                        entity: self.entity_from_ce(&lhs),
                    }))
                } else {
                    Ok(Axiom::Declaration(DeclarationAxiom {
                        id: 1,
                        entity: self.entity_from_ce(&lhs),
                    }))
                }
            }
        }
    }

    fn parse_concept(&mut self) -> Result<ClassExpression> {
        self.skip_whitespace();
        let c = self.peek().ok_or_else(|| self.err("Unexpected end"))?;

        match c {
            '\u{00AC}' | 'n' if self.match_keyword("not") => {
                self.consume_tok("not").or_else(|| { self.consume(); None });
                let inner = self.parse_concept()?;
                Ok(ClassExpression::ObjectComplementOf(Box::new(inner)))
            }
            '\u{2203}' | 'e' if self.match_keyword("exists") => {
                self.consume_tok("exists").or_else(|| { self.consume(); None });
                self.skip_whitespace();
                let role = self.parse_role();
                self.skip_whitespace();
                self.consume_tok(".");
                let filler = self.parse_concept()?;
                Ok(ClassExpression::ObjectSomeValuesFrom {
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: role }),
                    filler: Box::new(filler),
                })
            }
            '\u{2200}' | 'f' if self.match_keyword("forall") => {
                self.consume_tok("forall").or_else(|| { self.consume(); None });
                self.skip_whitespace();
                let role = self.parse_role();
                self.skip_whitespace();
                self.consume_tok(".");
                let filler = self.parse_concept()?;
                Ok(ClassExpression::ObjectAllValuesFrom {
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: role }),
                    filler: Box::new(filler),
                })
            }
            '\u{2265}' | '\u{2264}' | '=' | 'g' if self.match_keyword("geq") => {
                self.consume_tok("geq").or_else(|| { self.consume(); None });
                self.skip_whitespace();
                let n = self.parse_number();
                self.skip_whitespace();
                let role = self.parse_role();
                self.skip_whitespace();
                self.consume_tok(".");
                let filler = self.parse_concept()?;
                Ok(ClassExpression::ObjectMinCardinality {
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: role }),
                    cardinality: n,
                    filler: Box::new(filler),
                })
            }
            '\u{22A4}' | 'T' if self.match_keyword("Thing") || c == '\u{22A4}' => {
                self.consume_tok("Thing").or_else(|| { self.consume(); None });
                Ok(ClassExpression::Class(Class::thing()))
            }
            '\u{22A5}' | 'B' if self.match_keyword("Bottom") || c == '\u{22A5}' => {
                self.consume_tok("Bottom").or_else(|| { self.consume(); None });
                Ok(ClassExpression::Class(Class::nothing()))
            }
            '{' => {
                self.consume().unwrap();
                let mut individuals = Vec::new();
                loop {
                    self.skip_whitespace();
                    if self.peek() == Some('}') { self.consume(); break; }
                    let name = self.parse_name();
                    if !name.is_empty() {
                        individuals.push(Individual::Named(NamedIndividual { iri: crate::ontology::IRI::new(&name) }));
                    }
                    self.skip_whitespace();
                    if self.peek() == Some(',') { self.consume(); }
                }
                Ok(ClassExpression::ObjectOneOf(individuals))
            }
            '(' => {
                self.consume().unwrap();
                let inner = self.parse_concept()?;
                self.skip_whitespace();
                self.expect_char(')')?;
                Ok(inner)
            }
            _ => {
                let name = self.parse_name();
                if name.is_empty() {
                    return Err(self.err("Expected concept name"));
                }
                let base = ClassExpression::Class(Class { iri: crate::ontology::IRI::new(&name) });

                // Try binary operators
                let saved = self.pos;
                self.skip_whitespace();
                let op = self.peek_tok();
                match op.as_deref() {
                    Some("\u{2293}") | Some("and") => {
                        self.consume_tok(&op.unwrap());
                        let rhs = self.parse_concept()?;
                        Ok(ClassExpression::ObjectIntersectionOf(vec![base, rhs]))
                    }
                    Some("\u{2294}") | Some("or") => {
                        self.consume_tok(&op.unwrap());
                        let rhs = self.parse_concept()?;
                        Ok(ClassExpression::ObjectUnionOf(vec![base, rhs]))
                    }
                    _ => {
                        self.pos = saved;
                        Ok(base)
                    }
                }
            }
        }
    }

    fn parse_name(&mut self) -> String {
        self.skip_whitespace();
        let mut name = String::new();
        while self.pos < self.input.len() {
            let c = self.peek().unwrap();
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '_' || c == ':' {
                name.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        name
    }

    fn parse_role(&mut self) -> crate::ontology::IRI {
        let name = self.parse_name();
        if name.is_empty() {
            crate::ontology::IRI::new("urn:role")
        } else {
            crate::ontology::IRI::new(&name)
        }
    }

    fn parse_number(&mut self) -> u32 {
        self.skip_whitespace();
        let mut n: u32 = 0;
        while self.pos < self.input.len() {
            let c = self.peek().unwrap();
            if c.is_ascii_digit() {
                n = n * 10 + c.to_digit(10).unwrap();
                self.pos += 1;
            } else {
                break;
            }
        }
        n
    }

    fn peek_tok(&self) -> Option<String> {
        let rem = &self.input[self.pos..];
        if rem.starts_with("\u{2293}") { Some("\u{2293}".into()) }
        else if rem.starts_with("\u{2294}") { Some("\u{2294}".into()) }
        else if rem.starts_with("\u{00AC}") { Some("\u{00AC}".into()) }
        else if rem.starts_with("\u{2203}") { Some("\u{2203}".into()) }
        else if rem.starts_with("\u{2200}") { Some("\u{2200}".into()) }
        else if rem.starts_with("\u{2291}") { Some("\u{2291}".into()) }
        else if rem.starts_with("\u{2261}") { Some("\u{2261}".into()) }
        else if rem.starts_with("\u{2264}") { Some("\u{2264}".into()) }
        else if rem.starts_with("\u{2265}") { Some("\u{2265}".into()) }
        else if rem.starts_with("\u{22A4}") { Some("\u{22A4}".into()) }
        else if rem.starts_with("\u{22A5}") { Some("\u{22A5}".into()) }
        else if rem.starts_with("and") { Some("and".into()) }
        else if rem.starts_with("or") { Some("or".into()) }
        else if rem.starts_with("not") { Some("not".into()) }
        else if rem.starts_with("exists") { Some("exists".into()) }
        else if rem.starts_with("forall") { Some("forall".into()) }
        else if rem.starts_with("sqsubseteq") { Some("sqsubseteq".into()) }
        else if rem.starts_with("equiv") { Some("equiv".into()) }
        else if rem.starts_with("geq") { Some("geq".into()) }
        else if rem.starts_with("leq") { Some("leq".into()) }
        else if rem.starts_with("Thing") { Some("Thing".into()) }
        else if rem.starts_with("Bottom") { Some("Bottom".into()) }
        else if rem.starts_with(".") { Some(".".into()) }
        else { None }
    }

    fn consume_tok(&mut self, tok: &str) -> Option<()> {
        let rem = &self.input[self.pos..];
        if rem.starts_with(tok) {
            for _ in 0..tok.chars().count() {
                self.consume();
            }
            Some(())
        } else {
            None
        }
    }

    fn match_keyword(&self, kw: &str) -> bool {
        self.input[self.pos..].starts_with(kw)
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        if self.peek() == Some(expected) {
            self.consume();
            Ok(())
        } else {
            Err(self.err(&format!("Expected '{expected}'")))
        }
    }

    fn entity_from_ce(&self, expr: &ClassExpression) -> Entity {
        match expr {
            ClassExpression::Class(cls) => Entity::Class(cls.iri.clone()),
            _ => Entity::Class(crate::ontology::IRI::new("urn:concept")),
        }
    }

    fn err(&self, msg: &str) -> crate::Error {
        crate::Error::ParseError(format!("DL Syntax parse error at position {}: {msg}", self.pos))
    }
}

// ── DL Syntax Renderer ───────────────────────────────────────────────────────

/// Configuration for DL Syntax rendering.
#[derive(Debug, Clone)]
pub struct DLSyntaxConfig {
    pub use_unicode: bool,
}

impl Default for DLSyntaxConfig {
    fn default() -> Self {
        Self { use_unicode: true }
    }
}

/// Renders ontologies in Description Logic syntax.
pub struct DLSyntaxRenderer {
    config: DLSyntaxConfig,
}

impl DLSyntaxRenderer {
    #[must_use]
    pub fn new(use_unicode: bool) -> Self {
        Self {
            config: DLSyntaxConfig { use_unicode },
        }
    }

    /// Render an ontology to DL syntax string.
    pub fn serialize(&self, ontology: &Ontology) -> Result<String> {
        let mut buf = String::with_capacity(2048);
        for axiom in ontology.axioms() {
            let line = match axiom {
                Axiom::SubClassOf(a) => format!(
                    "{} {} {}\n",
                    self.render_ce(&a.subclass),
                    self.subsume(),
                    self.render_ce(&a.superclass)
                ),
                Axiom::EquivalentClasses(a) => {
                    let parts: Vec<String> = a.classes.iter().map(|c| self.render_ce(c)).collect();
                    format!("{}\n", parts.join(&format!(" {} ", self.equiv())))
                }
                Axiom::DisjointClasses(a) => {
                    let parts: Vec<String> = a.classes.iter().map(|c| self.render_ce(c)).collect();
                    format!("{} {} {} {} {}\n", parts[0], self.and(), parts[1], self.subsume(), self.bot())
                }
                Axiom::ClassAssertion(a) => format!(
                    "{}({})\n",
                    self.render_ce(&a.class),
                    self.render_individual(&a.individual)
                ),
                Axiom::ObjectPropertyAssertion(a) => format!(
                    "{}({}, {})\n",
                    self.render_ope(&a.property),
                    self.render_individual(&a.source),
                    self.render_individual(&a.target)
                ),
                Axiom::SameIndividual(a) => {
                    let parts: Vec<String> = a.individuals.iter().map(|i| self.render_individual(i)).collect();
                    format!("{} {}\n", parts.join(", "), self.equiv())
                }
                Axiom::DifferentIndividuals(a) => {
                    let parts: Vec<String> = a.individuals.iter().map(|i| self.render_individual(i)).collect();
                    format!("{} not {}\n", parts.join(", "), self.equiv())
                }
                _ => String::new(),
            };
            buf.push_str(&line);
        }
        Ok(buf)
    }

    // ── Operators (Unicode or ASCII) ─────────────────────────────────────

    fn and(&self) -> &str { if self.config.use_unicode { "\u{2293}" } else { "and" } }
    fn or(&self) -> &str { if self.config.use_unicode { "\u{2294}" } else { "or" } }
    fn not(&self) -> &str { if self.config.use_unicode { "\u{00AC}" } else { "not " } }
    fn some(&self) -> &str { if self.config.use_unicode { "\u{2203}" } else { "exists " } }
    fn all(&self) -> &str { if self.config.use_unicode { "\u{2200}" } else { "forall " } }
    fn subsume(&self) -> &str { if self.config.use_unicode { "\u{2291}" } else { " sqsubseteq " } }
    fn equiv(&self) -> &str { if self.config.use_unicode { "\u{2261}" } else { " equiv " } }
    #[allow(dead_code)]
    fn top(&self) -> &str { if self.config.use_unicode { "\u{22A4}" } else { "Thing" } }
    fn bot(&self) -> &str { if self.config.use_unicode { "\u{22A5}" } else { "Bottom" } }

    // ── Expression rendering ─────────────────────────────────────────────

    fn render_ce(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(cls) => self.name(&cls.iri.to_string()),
            ClassExpression::ObjectIntersectionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                format!("({})", parts.join(&format!(" {} ", self.and())))
            }
            ClassExpression::ObjectUnionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                format!("({})", parts.join(&format!(" {} ", self.or())))
            }
            ClassExpression::ObjectComplementOf(inner) => {
                format!("{}{}", self.not(), self.render_ce(inner))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!("{}{}.{}", self.some(), self.render_ope(property), self.render_ce(filler))
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!("{}{}.{}", self.all(), self.render_ope(property), self.render_ce(filler))
            }
            ClassExpression::ObjectHasValue { property, value } => {
                format!("{}{}.{{{}}}", self.some(), self.render_ope(property), self.render_individual(value))
            }
            ClassExpression::ObjectHasSelf { property } => {
                format!("{}{}.Self", self.some(), self.render_ope(property))
            }
            ClassExpression::ObjectMinCardinality { property, cardinality, filler } => {
                format!("\u{2265}{} {}.{}", cardinality, self.render_ope(property), self.render_ce(filler))
            }
            ClassExpression::ObjectMaxCardinality { property, cardinality, filler } => {
                format!("\u{2264}{} {}.{}", cardinality, self.render_ope(property), self.render_ce(filler))
            }
            ClassExpression::ObjectExactCardinality { property, cardinality, filler } => {
                format!("={} {}.{}", cardinality, self.render_ope(property), self.render_ce(filler))
            }
            ClassExpression::ObjectOneOf(inds) => {
                let parts: Vec<String> = inds.iter().map(|i| self.render_individual(i)).collect();
                format!("{{{}}}", parts.join(", "))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                format!("{}{}.{}", self.some(), self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                format!("{}{}.{}", self.all(), self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataHasValue { property, value } => {
                format!("{}{}.{{{}}}", self.some(), self.render_dpe(property), self.render_literal(value))
            }
            ClassExpression::DataMinCardinality { property, cardinality, filler } => {
                format!("\u{2265}{} {}.{}", cardinality, self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataMaxCardinality { property, cardinality, filler } => {
                format!("\u{2264}{} {}.{}", cardinality, self.render_dpe(property), self.render_datarange(filler))
            }
            ClassExpression::DataExactCardinality { property, cardinality, filler } => {
                format!("={} {}.{}", cardinality, self.render_dpe(property), self.render_datarange(filler))
            }
        }
    }

    fn render_ope(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(p) => self.name(&p.iri.to_string()),
            ObjectPropertyExpression::InverseObjectProperty(p) => format!("{}⁻", self.name(&p.iri.to_string())),
            ObjectPropertyExpression::PropertyChain(chain) => {
                let parts: Vec<String> = chain.iter().map(|p| self.render_ope(p)).collect();
                parts.join(" ∘ ")
            }
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
            Individual::Anonymous(a) => format!("_:{}", &a.id[..8.min(a.id.len())]),
        }
    }

    fn render_literal(&self, lit: &Literal) -> String {
        lit.value.clone()
    }

    fn render_datarange(&self, range: &DataRange) -> String {
        match range {
            DataRange::Datatype(iri) => self.name(&iri.to_string()),
            _ => "Datatype".to_string(),
        }
    }

    fn name(&self, iri: &str) -> String {
        if let Some(fragment) = iri.rsplit('#').next() {
            if !fragment.is_empty() && fragment.len() < iri.len() {
                return fragment.to_string();
            }
        }
        if let Some(last) = iri.rsplit('/').next() {
            if !last.is_empty() && last.len() < iri.len() {
                return last.to_string();
            }
        }
        iri.to_string()
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Parse DL Syntax content into an ontology.
pub fn parse(content: &str) -> Result<Ontology> {
    let mut parser = DLSyntaxParser::new();
    parser.parse(content)
}

/// Serialize an ontology to DL syntax string.
pub fn render_to_string(ontology: &Ontology, use_unicode: bool) -> Result<String> {
    let renderer = DLSyntaxRenderer::new(use_unicode);
    renderer.serialize(ontology)
}

/// Save an ontology as DL syntax to a file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let renderer = DLSyntaxRenderer::new(true);
    let content = renderer.serialize(ontology)?;
    std::fs::write(path, content).map_err(|e| crate::Error::io(format!("Failed to write DL syntax: {e}")))
}
