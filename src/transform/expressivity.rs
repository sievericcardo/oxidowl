//! DL Expressivity Checker.
//!
//! Analyzes an ontology to determine its Description Logic expressivity
//! (ALC through SROIQV(D)).

use crate::ontology::{ClassExpression, ObjectPropertyExpression, Ontology};

/// Expressivity features detected in an ontology.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DLExpressivity {
    pub has_complement: bool,
    pub has_union: bool,
    pub has_existential: bool,
    pub has_universal: bool,
    pub has_cardinality: bool,
    pub has_qualified_cardinality: bool,
    pub has_nominals: bool,
    pub has_inverse: bool,
    pub has_transitivity: bool,
    pub has_role_hierarchy: bool,
    pub has_functional: bool,
    pub has_role_disjointness: bool,
    pub has_self: bool,
    pub has_datatype: bool,
}

impl DLExpressivity {
    /// Format as the standard expressivity string, e.g., "SROIQ(D)".
    #[must_use]
    pub fn to_name(&self) -> String {
        let mut s = String::new();

        // Base logic
        if self.has_complement && self.has_existential && self.has_universal {
            s.push_str("ALC");
        } else {
            s.push_str("AL");
        }

        // Extensions (in standard order: S H O I Q N F R V)
        if self.has_transitivity {
            s.push('S');
        }
        if self.has_role_hierarchy {
            s.push('H');
        }
        if self.has_nominals {
            s.push('O');
        }
        if self.has_inverse {
            s.push('I');
        }
        if self.has_qualified_cardinality {
            s.push('Q');
        } else if self.has_cardinality && !self.has_qualified_cardinality {
            s.push('N');
        }
        if self.has_functional {
            s.push('F');
        }
        if self.has_role_disjointness {
            s.push('R');
        }
        if self.has_self {
            s.push('V');
        }

        if self.has_datatype {
            s.push_str("(D)");
        }

        s
    }

    #[must_use]
    pub fn is_owl2_el(&self) -> bool {
        !self.has_complement
            && !self.has_union
            && !self.has_universal
            && !self.has_cardinality
            && !self.has_inverse
            && !self.has_transitivity
            && !self.has_functional
            && !self.has_self
            && !self.has_nominals
    }

    #[must_use]
    pub fn is_owl2_ql(&self) -> bool {
        !self.has_complement
            && !self.has_union
            && !self.has_transitivity
            && !self.has_functional
            && !self.has_inverse
            && !self.has_self
            && !self.has_nominals
    }

    #[must_use]
    pub fn is_owl2_rl(&self) -> bool {
        // RL allows certain constructs only in specific positions
        !self.has_complement
            && !self.has_union
            && !self.has_nominals
            && !self.has_self
            && !self.has_inverse
    }
}

/// Analyzes an ontology to determine its DL expressivity.
#[derive(Debug, Clone, Copy, Default)]
pub struct DLExpressivityChecker;

impl DLExpressivityChecker {
    /// Analyze a single ontology.
    #[must_use]
    pub fn analyze(&self, ontology: &Ontology) -> DLExpressivity {
        let mut expr = DLExpressivity::default();
        for axiom in ontology.axioms() {
            self.check_axiom(axiom, &mut expr);
        }
        expr
    }

    fn check_axiom(&self, axiom: &crate::ontology::axioms::Axiom, expr: &mut DLExpressivity) {
        use crate::ontology::axioms::Axiom;
        match axiom {
            Axiom::SubClassOf(a) => {
                self.check_ce(&a.subclass, expr);
                self.check_ce(&a.superclass, expr);
            }
            Axiom::EquivalentClasses(a) => {
                for ce in &a.classes {
                    self.check_ce(ce, expr);
                }
            }
            Axiom::DisjointClasses(a) => {
                for ce in &a.classes {
                    self.check_ce(ce, expr);
                }
            }
            Axiom::DisjointUnion(a) => {
                self.check_ce(&a.class, expr);
                for ce in &a.disjoint_classes {
                    self.check_ce(ce, expr);
                }
            }
            Axiom::SubObjectPropertyOf(a) => {
                expr.has_role_hierarchy = true;
                if let ObjectPropertyExpression::PropertyChain(_) = &a.sub_property {
                    expr.has_role_hierarchy = true;
                }
            }
            Axiom::EquivalentObjectProperties(_) => expr.has_role_hierarchy = true,
            Axiom::DisjointObjectProperties(_) => expr.has_role_disjointness = true,
            Axiom::InverseObjectProperties(_) => expr.has_inverse = true,
            Axiom::ObjectPropertyDomain(a) => self.check_ce(&a.domain, expr),
            Axiom::ObjectPropertyRange(a) => self.check_ce(&a.range, expr),
            Axiom::FunctionalObjectProperty(_) => expr.has_functional = true,
            Axiom::InverseFunctionalObjectProperty(_) => expr.has_functional = true,
            Axiom::ReflexiveObjectProperty(_) => expr.has_self = true,
            Axiom::IrreflexiveObjectProperty(_) => expr.has_self = true,
            Axiom::SymmetricObjectProperty(_) => {}
            Axiom::AsymmetricObjectProperty(_) => {}
            Axiom::TransitiveObjectProperty(_) => expr.has_transitivity = true,
            Axiom::ClassAssertion(a) => self.check_ce(&a.class, expr),
            Axiom::DataPropertyDomain(a) => {
                self.check_ce(&a.domain, expr);
                expr.has_datatype = true;
            }
            Axiom::DataPropertyRange(_) => expr.has_datatype = true,
            Axiom::SubDataPropertyOf(_) => expr.has_datatype = true,
            _ => {}
        }
    }

    fn check_ce(&self, ce: &ClassExpression, expr: &mut DLExpressivity) {
        match ce {
            ClassExpression::Class(_) => {}
            ClassExpression::ObjectIntersectionOf(ops) => {
                for op in ops {
                    self.check_ce(op, expr);
                }
            }
            ClassExpression::ObjectUnionOf(ops) => {
                expr.has_union = true;
                for op in ops {
                    self.check_ce(op, expr);
                }
            }
            ClassExpression::ObjectComplementOf(inner) => {
                expr.has_complement = true;
                self.check_ce(inner, expr);
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => {
                expr.has_existential = true;
                self.check_ce(filler, expr);
            }
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                expr.has_universal = true;
                self.check_ce(filler, expr);
            }
            ClassExpression::ObjectHasValue { .. } => {
                expr.has_existential = true;
            }
            ClassExpression::ObjectHasSelf { .. } => {
                expr.has_self = true;
            }
            ClassExpression::ObjectMinCardinality { filler, .. }
            | ClassExpression::ObjectMaxCardinality { filler, .. }
            | ClassExpression::ObjectExactCardinality { filler, .. } => {
                expr.has_cardinality = true;
                self.check_ce(filler, expr);
                if !self.is_trivial_filler(filler) {
                    expr.has_qualified_cardinality = true;
                }
            }
            ClassExpression::ObjectOneOf(_) => {
                expr.has_nominals = true;
            }
            ClassExpression::DataSomeValuesFrom { .. }
            | ClassExpression::DataAllValuesFrom { .. }
            | ClassExpression::DataHasValue { .. }
            | ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. } => {
                expr.has_datatype = true;
            }
        }
    }

    fn is_trivial_filler(&self, ce: &ClassExpression) -> bool {
        matches!(ce, ClassExpression::Class(c) if c.is_thing())
    }
}
