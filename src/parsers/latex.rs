//! LaTeX Renderer for OWL ontologies.
//!
//! Renders OWL ontologies in LaTeX format for documentation and papers.

use crate::Result;
use crate::ontology::axioms::Axiom;
use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, Individual, Literal,
    ObjectPropertyExpression, Ontology,
};
use std::fmt::Write;

/// Configuration for the LaTeX renderer.
#[derive(Debug, Clone)]
pub struct LatexConfig {
    pub custom_preamble: Option<String>,
    pub numbered: bool,
}

impl Default for LatexConfig {
    fn default() -> Self {
        Self {
            custom_preamble: None,
            numbered: true,
        }
    }
}

/// Renders ontologies in LaTeX mathematical notation.
#[derive(Debug, Clone, Default)]
pub struct LatexRenderer;

impl LatexRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Render a complete LaTeX document for the ontology.
    pub fn render_document(&self, ontology: &Ontology, config: &LatexConfig) -> Result<String> {
        let mut buf = String::with_capacity(4096);

        // Preamble
        buf.push_str(
            r"\documentclass{article}
\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{hyperref}
",
        );
        if let Some(ref custom) = config.custom_preamble {
            buf.push_str(custom);
            buf.push('\n');
        }
        buf.push_str(
            r"\begin{document}
",
        );

        // Title
        if let Some(iri) = ontology.get_iri() {
            let _ = writeln!(buf, "\\title{{{iri}}}");
        }
        buf.push_str(
            r"\maketitle
",
        );

        // Ontology IRI
        if let Some(iri) = ontology.get_iri() {
            let _ = writeln!(buf, "\\textbf{{Ontology IRI:}} \\url{{{iri}}}\\\\");
        }
        if let Some(viri) = &ontology.id.version_iri {
            let _ = writeln!(buf, "\\textbf{{Version IRI:}} \\url{{{viri}}}\\\\");
        }

        // Imports
        if !ontology.imports.is_empty() {
            buf.push_str("\\textbf{Imports:}\n\\begin{itemize}\n");
            for imp in &ontology.imports {
                let _ = writeln!(buf, "\\item \\url{{{}}}", imp.imported_ontology_iri);
            }
            buf.push_str("\\end{itemize}\n");
        }

        // Group axioms for readable sections
        let axioms = ontology.axioms();
        let has_classes = axioms.iter().any(|a| {
            matches!(
                a,
                Axiom::SubClassOf(_)
                    | Axiom::EquivalentClasses(_)
                    | Axiom::DisjointClasses(_)
                    | Axiom::DisjointUnion(_)
            )
        });
        let has_objprops = axioms.iter().any(|a| {
            matches!(
                a,
                Axiom::SubObjectPropertyOf(_)
                    | Axiom::EquivalentObjectProperties(_)
                    | Axiom::ObjectPropertyDomain(_)
                    | Axiom::ObjectPropertyRange(_)
            )
        });
        let has_dataprops = axioms.iter().any(|a| {
            matches!(
                a,
                Axiom::SubDataPropertyOf(_)
                    | Axiom::DataPropertyDomain(_)
                    | Axiom::DataPropertyRange(_)
            )
        });
        let has_inds = axioms.iter().any(|a| {
            matches!(
                a,
                Axiom::ClassAssertion(_)
                    | Axiom::ObjectPropertyAssertion(_)
                    | Axiom::DataPropertyAssertion(_)
                    | Axiom::SameIndividual(_)
                    | Axiom::DifferentIndividuals(_)
            )
        });

        if has_classes {
            buf.push_str("\n\\section{Class Axioms}\n");
            self.render_class_axioms(ontology, &mut buf, config.numbered);
        }
        if has_objprops {
            buf.push_str("\n\\section{Object Property Axioms}\n");
            self.render_object_property_axioms(ontology, &mut buf, config.numbered);
        }
        if has_dataprops {
            buf.push_str("\n\\section{Data Property Axioms}\n");
            self.render_data_property_axioms(ontology, &mut buf, config.numbered);
        }
        if has_inds {
            buf.push_str("\n\\section{Individual Assertions}\n");
            self.render_individual_assertions(ontology, &mut buf, config.numbered);
        }

        buf.push_str("\n\\end{document}\n");
        Ok(buf)
    }

    // ── Section renderers ─────────────────────────────────────────────────

    fn render_class_axioms(&self, ontology: &Ontology, buf: &mut String, numbered: bool) {
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(a) => {
                    let _ = writeln!(
                        buf,
                        "{}",
                        self.format_formula(
                            numbered,
                            &format!(
                                "{} \\sqsubseteq {}",
                                self.render_ce(&a.subclass),
                                self.render_ce(&a.superclass)
                            )
                        )
                    );
                }
                Axiom::EquivalentClasses(a) => {
                    let parts: Vec<String> = a.classes.iter().map(|c| self.render_ce(c)).collect();
                    let _ = writeln!(
                        buf,
                        "{}",
                        self.format_formula(numbered, &parts.join(" \\equiv "))
                    );
                }
                Axiom::DisjointClasses(a) => {
                    let parts: Vec<String> = a.classes.iter().map(|c| self.render_ce(c)).collect();
                    let _ = writeln!(
                        buf,
                        "{}",
                        self.format_formula(
                            numbered,
                            &format!("{} \\sqcap {} \\sqsubseteq \\bot", parts[0], parts[1])
                        )
                    );
                }
                Axiom::DisjointUnion(a) => {
                    let parts: Vec<String> = a
                        .disjoint_classes
                        .iter()
                        .map(|c| self.render_ce(c))
                        .collect();
                    let _ = writeln!(
                        buf,
                        "{}",
                        self.format_formula(
                            numbered,
                            &format!(
                                "{} \\equiv {}",
                                self.render_ce(&a.class),
                                parts.join(" \\mathbin{\\dot\\sqcup} ")
                            )
                        )
                    );
                }
                _ => {}
            }
        }
    }

    fn render_object_property_axioms(&self, ontology: &Ontology, buf: &mut String, numbered: bool) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::SubObjectPropertyOf(a) => Some(format!(
                    "{} \\sqsubseteq {}",
                    self.render_ope(&a.sub_property),
                    self.render_ope(&a.super_property)
                )),
                Axiom::ObjectPropertyDomain(a) => Some(format!(
                    "\\exists {}.\\top \\sqsubseteq {}",
                    self.render_ope(&a.property),
                    self.render_ce(&a.domain)
                )),
                Axiom::ObjectPropertyRange(a) => Some(format!(
                    "\\top \\sqsubseteq \\forall {}.{}",
                    self.render_ope(&a.property),
                    self.render_ce(&a.range)
                )),
                Axiom::FunctionalObjectProperty(a) => Some(format!(
                    "\\top \\sqsubseteq \\mathop{{\\leq}} 1 {}",
                    self.render_ope(&a.property)
                )),
                Axiom::TransitiveObjectProperty(a) => Some(format!(
                    "\\textit{{Trans}}({})",
                    self.render_ope(&a.property)
                )),
                Axiom::SymmetricObjectProperty(a) => {
                    Some(format!("\\textit{{Sym}}({})", self.render_ope(&a.property)))
                }
                Axiom::InverseObjectProperties(a) => Some(format!(
                    "{} \\equiv {}",
                    self.render_ope(&a.property1),
                    format!("{}^-", self.render_ope(&a.property2))
                )),
                Axiom::ReflexiveObjectProperty(a) => {
                    Some(format!("\\textit{{Ref}}({})", self.render_ope(&a.property)))
                }
                Axiom::IrreflexiveObjectProperty(a) => Some(format!(
                    "\\textit{{Irref}}({})",
                    self.render_ope(&a.property)
                )),
                Axiom::AsymmetricObjectProperty(a) => Some(format!(
                    "\\textit{{Asym}}({})",
                    self.render_ope(&a.property)
                )),
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{}", self.format_formula(numbered, &s));
            }
        }
    }

    fn render_data_property_axioms(&self, ontology: &Ontology, buf: &mut String, numbered: bool) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::SubDataPropertyOf(a) => Some(format!(
                    "{} \\sqsubseteq {}",
                    self.render_dpe(&a.sub_property),
                    self.render_dpe(&a.super_property)
                )),
                Axiom::DataPropertyDomain(a) => Some(format!(
                    "\\exists {}.\\top \\sqsubseteq {}",
                    self.render_dpe(&a.property),
                    self.render_ce(&a.domain)
                )),
                Axiom::DataPropertyRange(a) => Some(format!(
                    "\\top \\sqsubseteq \\forall {}.{}",
                    self.render_dpe(&a.property),
                    self.render_datarange(&a.range)
                )),
                Axiom::FunctionalDataProperty(a) => Some(format!(
                    "\\top \\sqsubseteq \\mathop{{\\leq}} 1 {}",
                    self.render_dpe(&a.property)
                )),
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{}", self.format_formula(numbered, &s));
            }
        }
    }

    fn render_individual_assertions(&self, ontology: &Ontology, buf: &mut String, numbered: bool) {
        for axiom in ontology.axioms() {
            let rendered = match axiom {
                Axiom::ClassAssertion(a) => Some(format!(
                    "{} \\in {}",
                    self.render_individual(&a.individual),
                    self.render_ce(&a.class)
                )),
                Axiom::ObjectPropertyAssertion(a) => Some(format!(
                    "({}, {}) : {}",
                    self.render_individual(&a.source),
                    self.render_individual(&a.target),
                    self.render_ope(&a.property)
                )),
                Axiom::DataPropertyAssertion(a) => Some(format!(
                    "({}, {}) : {}",
                    self.render_individual(&a.individual),
                    self.render_literal(&a.value),
                    self.render_dpe(&a.property)
                )),
                Axiom::SameIndividual(a) => {
                    let parts: Vec<String> = a
                        .individuals
                        .iter()
                        .map(|i| self.render_individual(i))
                        .collect();
                    Some(parts.join(" \\equiv "))
                }
                Axiom::DifferentIndividuals(a) => {
                    let parts: Vec<String> = a
                        .individuals
                        .iter()
                        .map(|i| self.render_individual(i))
                        .collect();
                    Some(parts.join(" \\neq "))
                }
                Axiom::NegativeObjectPropertyAssertion(a) => Some(format!(
                    "\\neg ({} : {})",
                    self.render_individual(&a.source),
                    self.render_ope(&a.property)
                )),
                Axiom::NegativeDataPropertyAssertion(a) => Some(format!(
                    "\\neg ({} : {})",
                    self.render_individual(&a.individual),
                    self.render_dpe(&a.property)
                )),
                _ => None,
            };
            if let Some(s) = rendered {
                let _ = writeln!(buf, "{}", self.format_formula(numbered, &s));
            }
        }
    }

    // ── LaTeX rendering helpers ───────────────────────────────────────────

    fn format_formula(&self, numbered: bool, formula: &str) -> String {
        if numbered {
            format!("\\begin{{equation}}\n  {formula}\n\\end{{equation}}")
        } else {
            format!("\\[\n  {formula}\n\\]")
        }
    }

    pub fn render_ce(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(cls) => self.escape_iri(&cls.iri.to_string()),
            ClassExpression::ObjectIntersectionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                parts.join(" \\sqcap ")
            }
            ClassExpression::ObjectUnionOf(ops) => {
                let parts: Vec<String> = ops.iter().map(|op| self.render_ce(op)).collect();
                parts.join(" \\sqcup ")
            }
            ClassExpression::ObjectComplementOf(inner) => {
                format!("\\neg {}", self.render_ce(inner))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                format!(
                    "\\exists {}.{}",
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                format!(
                    "\\forall {}.{}",
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectHasValue { property, value } => {
                format!(
                    "\\exists {}.\\{{{}}}",
                    self.render_ope(property),
                    self.render_individual(value)
                )
            }
            ClassExpression::ObjectHasSelf { property } => {
                format!("\\exists {}.\\textit{{Self}}", self.render_ope(property))
            }
            ClassExpression::ObjectMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "\\mathop{{\\geq}} {} {}.{}",
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
                    "\\mathop{{\\leq}} {} {}.{}",
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
                    "= {} {}.{}",
                    cardinality,
                    self.render_ope(property),
                    self.render_ce(filler)
                )
            }
            ClassExpression::ObjectOneOf(inds) => {
                let parts: Vec<String> = inds.iter().map(|i| self.render_individual(i)).collect();
                format!("\\{{{}}}", parts.join(", "))
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                format!(
                    "\\exists {}.{}",
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                format!(
                    "\\forall {}.{}",
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataHasValue { property, value } => {
                format!(
                    "\\exists {}.\\{{{}}}",
                    self.render_dpe(property),
                    self.render_literal(value)
                )
            }
            ClassExpression::DataMinCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "\\mathop{{\\geq}} {} {}.{}",
                    cardinality,
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataMaxCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "\\mathop{{\\leq}} {} {}.{}",
                    cardinality,
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
            ClassExpression::DataExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                format!(
                    "= {} {}.{}",
                    cardinality,
                    self.render_dpe(property),
                    self.render_datarange(filler)
                )
            }
        }
    }

    fn render_ope(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(p) => self.escape_iri(&p.iri.to_string()),
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                format!("{}^-", self.escape_iri(&p.iri.to_string()))
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                let parts: Vec<String> = chain.iter().map(|p| self.render_ope(p)).collect();
                parts.join(" \\circ ")
            }
        }
    }

    fn render_dpe(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(p) => self.escape_iri(&p.iri.to_string()),
        }
    }

    fn render_individual(&self, ind: &Individual) -> String {
        match ind {
            Individual::Named(n) => self.escape_iri(&n.iri.to_string()),
            Individual::Anonymous(a) => format!("\\_\\{{{}\\_id\\}}", id_short(&a.id)),
        }
    }

    fn render_literal(&self, lit: &Literal) -> String {
        if lit
            .value
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
        {
            lit.value.clone()
        } else if let Some(lang) = &lit.language {
            format!("\\text{{\"{}\"@\\textit{{{lang}}}}}", lit.value)
        } else {
            format!("\\text{{\"{}\"}}", lit.value)
        }
    }

    fn render_datarange(&self, range: &DataRange) -> String {
        match range {
            DataRange::Datatype(iri) => self.escape_iri(&iri.to_string()),
            DataRange::DataIntersectionOf(rs) => {
                let parts: Vec<String> = rs.iter().map(|r| self.render_datarange(r)).collect();
                parts.join(" \\sqcap ")
            }
            DataRange::DataUnionOf(rs) => {
                let parts: Vec<String> = rs.iter().map(|r| self.render_datarange(r)).collect();
                parts.join(" \\sqcup ")
            }
            DataRange::DataComplementOf(r) => format!("\\neg {}", self.render_datarange(r)),
            DataRange::DataOneOf(lits) => {
                let parts: Vec<String> = lits.iter().map(|l| self.render_literal(l)).collect();
                format!("\\{{{}}}", parts.join(", "))
            }
            DataRange::DatatypeRestriction { datatype, .. } => {
                self.escape_iri(&datatype.to_string())
            }
        }
    }

    fn escape_iri(&self, iri: &str) -> String {
        iri.replace('_', "\\_")
            .replace('#', "\\#")
            .replace('%', "\\%")
            .replace('$', "\\$")
            .replace('&', "\\&")
            .replace('~', "\\textasciitilde{}")
    }
}

fn id_short(id: &str) -> String {
    if id.len() <= 8 {
        id.to_string()
    } else {
        id[..8].to_string()
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Serialize an ontology to a LaTeX document string.
pub fn serialize(ontology: &Ontology) -> Result<String> {
    let renderer = LatexRenderer::new();
    renderer.render_document(ontology, &LatexConfig::default())
}

/// Save ontology to a .tex file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let content = serialize(ontology)?;
    std::fs::write(path, content)
        .map_err(|e| crate::Error::io(format!("Failed to write LaTeX: {e}")))
}
