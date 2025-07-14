//! OWL 2 DL Concepts and Class Expressions
//! 
//! This module implements OWL 2 DL class expressions and concept representation
//! following the OWL 2 specification structure.

use std::collections::{HashMap, HashSet};

/// Identifier for concepts
pub type ConceptId = u64;

/// Named OWL classes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Class {
    pub iri: crate::ontologies::IRI,
}

impl Class {
    pub fn new(iri: crate::ontologies::IRI) -> Self {
        Self { iri }
    }

    pub fn thing() -> Self {
        Self::new(crate::ontologies::IRI::new("http://www.w3.org/2002/07/owl#Thing"))
    }

    pub fn nothing() -> Self {
        Self::new(crate::ontologies::IRI::new("http://www.w3.org/2002/07/owl#Nothing"))
    }

    pub fn is_thing(&self) -> bool {
        self.iri == crate::ontologies::IRI::new("http://www.w3.org/2002/07/owl#Thing")
    }

    pub fn is_nothing(&self) -> bool {
        self.iri == crate::ontologies::IRI::new("http://www.w3.org/2002/07/owl#Nothing")
    }
}

/// OWL 2 DL Class Expression
#[derive(Debug, Clone, PartialEq)]
pub enum ClassExpression {
    /// Named class
    Class(Class),

    /// Intersection of class expressions (ObjectIntersectionOf)
    ObjectIntersectionOf(Vec<ClassExpression>),

    /// Union of class expressions (ObjectUnionOf)
    ObjectUnionOf(Vec<ClassExpression>),

    /// Object property restriction (ObjectSomeValuesFrom)
    ObjectSomeValuesFrom {
        property: crate::ontologies::ObjectPropertyExpression,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (ObjectAllValuesFrom)
    ObjectAllValuesFrom {
        property: crate::ontologies::ObjectPropertyExpression,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (ObjectHasValue)
    ObjectHasValue {
        property: crate::ontologies::ObjectPropertyExpression,
        value: crate::ontologies::Individual,
    },

    /// Object property restriction (ObjectHasSelf)
    ObjectHasSelf {
        property: crate::ontologies::ObjectPropertyExpression,
    },

    /// Object property restriction (ObjectMinCardinality)
    ObjectMinCardinality {
        property: crate::ontologies::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (ObjectMaxCardinality)
    ObjectMaxCardinality {
        property: crate::ontologies::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Object property restriction (ObjectExactCardinality)
    ObjectExactCardinality {
        property: crate::ontologies::ObjectPropertyExpression,
        cardinality: u32,
        filler: Box<ClassExpression>,
    },

    /// Data property restriction (DataSomeValuesFrom)
    DataSomeValuesFrom {
        property: crate::ontologies::DataPropertyExpression,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (DataAllValuesFrom)
    DataAllValuesFrom {
        property: crate::ontologies::DataPropertyExpression,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (DataHasValue)
    DataHasValue {
        property: crate::ontologies::DataPropertyExpression,
        value: crate::ontologies::Literal,
    },

    /// Data property restriction (DataMinCardinality)
    DataMinCardinality {
        property: crate::ontologies::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (DataMaxCardinality)
    DataMaxCardinality {
        property: crate::ontologies::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Data property restriction (DataExactCardinality)
    DataExactCardinality {
        property: crate::ontologies::DataPropertyExpression,
        cardinality: u32,
        filler: crate::ontology::DataRange,
    },

    /// Negation of a class expression (ObjectComplementOf)

    ObjectComplementOf(Box<ClassExpression>),

    /// Annotation assertion (AnnotationAssertion)
    AnnotationAssertion {
        property: crate::ontologies::AnnotationPropertyExpression,
        subject: crate::ontologies::IRI,
        value: crate::ontologies::Literal,
    },

    /// Sub-annotation property of (SubAnnotationPropertyOf)
    SubAnnotationPropertyOf {
        sub_property: crate::ontologies::AnnotationPropertyExpression,
        super_property: crate::ontologies::AnnotationPropertyExpression,
    },

    /// Annotation property domain (AnnotationPropertyDomain)
    AnnotationPropertyDomain {
        property: crate::ontologies::AnnotationPropertyExpression,
        domain: ClassExpression,
    },

    /// Annotation property range (AnnotationPropertyRange)
    AnnotationPropertyRange {
        property: crate::ontologies::AnnotationPropertyExpression,
        range: ClassExpression,
    },
}

impl ClassExpression {
    /// Create a class expression from a named class
    pub fn class(iri: crate::ontologies::IRI) -> Self {
        ClassExpression::Class(Class::new(iri))
    }

    /// Create the OWL Thing class expression
    pub fn thing() -> Self {
        ClassExpression::Class(Class::thing())
    }

    /// Create the OWL Nothing class expression
    pub fn nothing() -> Self {
        ClassExpression::Class(Class::nothing())
    }

    /// Create an intersection of class expressions
    pub fn intersection_of(expressions: Vec<ClassExpression>) -> Self {
        if expression.is_empty() {
            Self::thing() // Intersection of nothing is Thing
        } else if expressions.len() == 1 {
            expressions.into_iter().next().unwrap() // Single expression
        } else {
            ClassExpression::ObjectIntersectionOf(expressions)
        }
    }

    /// Create a union of class expressions
    pub fn union_of(expressions: Vec<ClassExpression>) -> Self {
        if expressions.is_empty() {
            Self::nothing() // Union of nothing is Nothing
        } else if expressions.len() == 1 {
            expressions.into_iter().next().unwrap() // Single expression
        } else {
            ClassExpression::ObjectUnionOf(expressions)
        }
    }

    /// Create a complement of a class expression
    pub fn complement_of(expression: ClassExpression) -> Self {
        ClassExpression::ObjectComplementOf(Box::new(expression))
    }

    /// Create an existential restriction (some values from)
    pub fn some_values_from(
        property: crate::ontologies::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> Self {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Create a universal restriction (all values from)
    pub fn all_values_from(
        property: crate::ontologies::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> Self {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Check if this class is a named class
    pub fn is_named_class(&self) -> bool {
        matches!(self, ClassExpression::Class(_))
    }

    /// Check if this class ia a complex class expression
    pub fn is_complex_class_expression(&self) -> bool {
        !self.is_named_class()
    }

    /// Get the named class IRI if this is a named class
    pub fn as_class(&self) -> Option<&Class> {
        if let ClassExpression::Class(class) = self {
            Some(class)
        } else {
            None
        }
    }

    /// Get all named classes referenced in this class expression
    pub fn signature(&self) -> HashSet<Class> {
        let mut signature = HashSet::new();
        self.collect_classes(&mut signature);
        signature
    }

    fn collect_classes(&self, signature: &mut HashSet<Class>) {
        match self {
            ClassExpression::Class(class) => {
                signature.insert(class.clone());
            }
            ClassExpression::ObjectIntersectionOf(expressions) |
            ClassExpression::ObjectUnionOf(expressions) => {
                for expr in expressions {
                    expr.collect_classes(signature);
                }
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } |
            ClassExpression::ObjectAllValuesFrom { filler, .. } |
            ClassExpression::ObjectHasValue { value: filler, .. } |
            ClassExpression::ObjectMinCardinality { filler, .. } |
            ClassExpression::ObjectMaxCardinality { filler, .. } |
            ClassExpression::ObjectExactCardinality { filler, .. } => {
                filler.collect_classes(signature);
            }
            ClassExpression::DataSomeValuesFrom { filler, .. } |
            ClassExpression::DataAllValuesFrom { filler, .. } |
            ClassExpression::DataMinCardinality { filler, .. } |
            ClassExpression::DataMaxCardinality { filler, .. } |
            ClassExpression::DataExactCardinality { filler, .. } => {
                // Data ranges do not contain named classes
            }
            ClassExpression::ObjectComplementOf(expr) => {
                expr.collect_classes(signature);
            }
            _ => {} // Other expressions do not contain named classes
        }
    }

    /// Compute the negation normal form (NNF) of this class expression
    pub fn to_nnf(&self) -> ClassExpression {
        self.to_nnf_helper(false)
    }

    fn to_nnf_helper(&self, negated: bool) -> ClassExpression {
        match self {
            ClassExpression::ObjectComplementOf(expr) => {
                // Negate the inner expression
                expr.to_nnf_helper(!negated)
            }

            ClassExpression::ObjectIntersectionOf(expressions) => {
                let nnf_expressions: Vec<_> = expressions
                    .iter()
                    .map(|e| e.to_nnf_helper(negated))
                    .collect();
                if negated {
                    ClassExpression::ObjectUnionOf(nnf_expressions)
                } else {
                    ClassExpression::ObjectIntersectionOf(nnf_expressions)
                }
            }

            ClassExpression::ObjectUnionOf(expressions) => {
                let nnf_expressions: Vec<_> = expressions
                    .iter()
                    .map(|e| e.to_nnf_helper(negated))
                    .collect();
                if negated {
                    ClassExpression::ObjectIntersectionOf(nnf_expressions)
                } else {
                    ClassExpression::ObjectUnionOf(nnf_expressions)
                }
            }

            ClassExpression::ObjectComplementOf(expr)) if negated => {
                // Double negation elimination
                expr.to_nnf_helper(false)
            }

            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                if negated {
                    // Negation of some values from becomes all values from
                    ClassExpression::ObjectAllValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(true)),
                    }
                } else {
                    ClassExpression::ObjectSomeValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(false)),
                    }
                }
            }

            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                if negated {
                    // Negation of all values from becomes some values from
                    ClassExpression::ObjectSomeValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(true)),
                    }
                } else {
                    ClassExpression::ObjectAllValuesFrom {
                        property: property.clone(),
                        filler: Box::new(filler.to_nnf_helper(false)),
                    }
                }
            }

            // For other expressions, wrap in complement if negated
            (expr, true) => ClassExpression::ObjectComplementOf(Box::new(expr.clone())),
            (expr, false) => expr.clone(),
            _ => self.clone(), // Other expressions remain unchanged
        }
    }

    /// Check if this class expression is in negation normal form (NNF)
    pub fn is_nnf(&self) -> bool {
        match self {
            ClassExpression::Class(_) => true,
            ClassExpression::ObjectIntersectionOf(expressions) | ClassExpression::ObjectUnionOf(expressions) => {
                expressions.iter().all(|e| e.is_nnf())
            }
            ClassExpression::ObjectComplementOf(expr) => {
                // NNF does not allow negation of complex expressions
                expr.is_named_class()
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } |
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                filler.is_nnf()
            }
            ClassExpression::ObjectMinCardinality { filler, .. } |
            ClassExpression::ObjectMaxCardinality { filler, .. } |
            ClassExpression::ObjectExactCardinality { filler, .. } => {
                // Cardinality restrictions are in NNF if filler is in NNF
                filler.is_nnf()
            }
            ClassExpression::DataSomeValuesFrom { filler, .. } |
            ClassExpression::DataAllValuesFrom { filler, .. } |
            ClassExpression::DataMinCardinality { filler, .. } |
            ClassExpression::DataMaxCardinality { filler, .. } |
            ClassExpression::DataExactCardinality { filler, .. } => {
                // Data ranges do not contain negations
                true
            }
            _ => {
                // Other expressions do not contain negations
                true
            }
        }
    }

    /// Simplify the class expression by applying logic rules
    pub fn simplify(&self) -> Result<ClassExpression> {
        match self {
            ClassExpression::ObjectIntersectionOf(expressions) => {
                let simplified: Vec<_> = expressions
                    .iter()
                    .map(|e| e.simplify())
                    .collect::<Result<Vec<_>>>()?;
                    
                // Remove duplicates and empty expressions
                let mut unique_exprs = Vec::new();
                let mut has_nothing = false;

                for expr in simplfied {
                    if let ClassExpression::Class(class) = &expr {
                        if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                            has_nothing = true;
                            break; // Nothing dominates intersection
                        } else if class.iri.as_str() != "http://www.w3.org/2002/07/owl#Thing" {
                            continue; // Ignore Thing in intersection
                        }
                    }

                    if !unique_exprs.contains(&expr) {
                        unique_exprs.push(expr);
                    }
                }

                if has_nothing {
                    Self::nothing()
                } else if unique_exprs.is_empty() {
                    Self::thing() // Empty intersection is Thing
                } else if unique_exprs.len() == 1 {
                    unique_exprs.into_iter().next().unwrap() // Single expression
                } else {
                    ClassExpression::ObjectIntersectionOf(unique_exprs)
                }
            }

            ClassExpression::ObjectUnionOf(expressions) => {
                let simplified : Vec<_> = expressions
                    .iter()
                    .map(|e| e.simplify())
                    .collect::<Result<Vec<_>>>()?;

                // Remove duplicates and empty expressions
                let mut unique_exprs = Vec::new();
                let mut has_thing = false;

                for expr in simplified {
                    if let ClassExpression::Class(class) = &expr {
                        if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                            has_thing = true;
                            break; // Thing dominates union
                        } else if class.iri.as_str() != "http://www.w3.org/2002/07/owl#Nothing" {
                            continue; // Ignore Nothing in union
                        }
                    }

                    if !unique_exprs.contains(&expr) {
                        unique_exprs.push(expr);
                    }
                }

                if has_thing {
                    Self::thing()
                } else if unique_exprs.is_empty() {
                    Self::nothing() // Empty union is Nothing
                } else if unique_exprs.len() == 1 {
                    unique_exprs.into_iter().next().unwrap() // Single expression
                } else {
                    ClassExpression::ObjectUnionOf(unique_exprs)
                }
            }

            ClassExpression::ObjectComplementOf(expr) => {
                let simplified = expr.simplify()?;

                if let ClassExpression::ObjectComplementOf(inner) = simplified {
                    // Double negation elimination
                    inner.as_ref().clone()
                } else if let ClassExpression::Class(class) = &simplified {
                    if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Thing" {
                        // Complement of Thing is Nothing
                        ClassExpression::nothing()
                    } else if class.iri.as_str() == "http://www.w3.org/2002/07/owl#Nothing" {
                        // Complement of Nothing is Thing
                        ClassExpression::thing()
                    } else {
                        // Complement of a named class remains unchanged
                        ClassExpression::ObjectComplementOf(Box::new(simplified))
                    }
                } else {
                    // Other expressions remain unchanged
                    ClassExpression::ObjectComplementOf(Box::new(simplified))
                }
            }

            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let simplified_filler = filler.simplify()?;
                ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(simplified_filler),
                }
            }

            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let simplified_filler = filler.simplify()?;
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(simplified_filler),
                }
            }

            ClassExpression::ObjectHasValue { property, value } => {
                // Has value restrictions do not simplify further
                ClassExpression::ObjectHasValue {
                    property: property.clone(),
                    value: value.clone(),
                }
            }

            ClassExpression::ObjectMinCardinality { property, cardinality, filler } => {
                ClassExpression::ObjectMinCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.as_ref().map(|f| Box::new(f.simplify())),
                }
            }

            ClassExpression::ObjectMaxCardinality { property, cardinality, filler } => {
                ClassExpression::ObjectMaxCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.as_ref().map(|f| Box::new(f.simplify())),
                }
            }

            ClassExpression::ObjectExactCardinality { property, cardinality, filler } => {
                ClassExpression::ObjectExactCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.as_ref().map(|f| Box::new(f.simplify())),
                }
            }

            ClassExpression::DataSomeValuesFrom { property, filler } => {
                // Data ranges do not simplify further
                ClassExpression::DataSomeValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataAllValuesFrom { property, filler } => {
                // Data ranges do not simplify further
                ClassExpression::DataAllValuesFrom {
                    property: property.clone(),
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataHasValue { property, value } => {
                // Has value restrictions do not simplify further
                ClassExpression::DataHasValue {
                    property: property.clone(),
                    value: value.clone(),
                }
            }

            ClassExpression::DataMinCardinality { property, cardinality, filler } => {
                ClassExpression::DataMinCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataMaxCardinality { property, cardinality, filler } => {
                ClassExpression::DataMaxCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.clone(),
                }
            }

            ClassExpression::DataExactCardinality { property, cardinality, filler } => {
                ClassExpression::DataExactCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.clone(),
                }
            }

            ClassExpression::AnnotationAssertion { property, subject, value } => {
                // Annotation assertions do not simplify further
                ClassExpression::AnnotationAssertion {
                    property: property.clone(),
                    subject: subject.clone(),
                    value: value.clone(),
                }
            }

            ClassExpression::SubAnnotationPropertyOf { sub_property, super_property } => {
                // Sub-annotation property assertions do not simplify further
                ClassExpression::SubAnnotationPropertyOf {
                    sub_property: sub_property.clone(),
                    super_property: super_property.clone(),
                }
            }

            ClassExpression::AnnotationPropertyDomain { property, domain } => {
                // Annotation property domains do not simplify further
                ClassExpression::AnnotationPropertyDomain {
                    property: property.clone(),
                    domain: domain.clone(),
                }
            }

            ClassExpression::AnnotationPropertyRange { property, range } => {
                // Annotation property ranges do not simplify further
                ClassExpression::AnnotationPropertyRange {
                    property: property.clone(),
                    range: range.clone(),
                }
            }

            _ => {
                // Other expressions remain unchanged
                self.clone()
            }
        }
    }
}

/// Concept store for managing named classes and class expressions
#[derive(Debug, Clone)]
pub struct ConceptStore {
    classes; HashMap<crate::ontology::IRI, Class>,
    expressions: HashMap<ConceptId, ClassExpression>,
    next_id: ConceptId,
}

impl ConceptStore {
    pub fn new() -> Self {
        let mut store = Self {
            classes: HashMap::new(),
            expressions: HashMap::new(),
            next_id: 0,
        }

        // Built-in classes
        store.add_class(Class::thing());
        store.add_class(Class::nothing());

        store
    }

    pub fn add_class(&mut self, class: Class) -> &Class {
        let iri = class.iri.clone();
        self.classes.entry(iri).or_insert(class)
    }

    pub fn get_class(&self, iri: &crate::ontology::IRI) -> Option<&Class> {
        self.classes.get(iri)
    }

    pub fn get_or_create_class(&mut self, iri: crate::ontology::IRI) -> &Class {
        if !self.classes.contains_key(&iri) {
            let class = Class::new(iri.clone());
            self.classes.insert(iri.clone(), class);
        }
        &self.classes[&iri]
    }

    pub fn add_expression(&mut self, expression: ClassExpression) -> ConceptId {
        let id = self.next_id;
        self.expressions.insert(id, expression);
        self.next_id += 1;
        id
    }

    pub fn get_expression(&self, id: ConceptId) -> Option<&ClassExpression> {
        self.expressions.get(&id)
    }

    pub fn all_classes(&self) -> impl Iterator<Item = &Class> {
        self.classes.values()
    }

    pub fn all_expressions(&self) -> impl Iterator<Item = &ClassExpression)> {
        self.expressions.values()
    }

    pub fn len(&self) -> usize {
        self.classes.len() + self.expressions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.expressions.is_empty()
    }
}

impl Default for ConceptStore {
    fn default() -> Self {
        Self::new()
    }
}
