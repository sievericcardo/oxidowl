// Display implementation for ClassExpression to support formatting in HyperTableau
impl std::fmt::Display for ClassExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassExpression::Class(class) => write!(f, "{}", class.iri),
            ClassExpression::ObjectIntersectionOf(operands) => {
                write!(f, "(")?;
                for (i, operand) in operands.iter().enumerate() {
                    if i > 0 { write!(f, " ⊓ ")?; }
                    write!(f, "{}", operand)?;
                }
                write!(f, ")")
            },
            ClassExpression::ObjectUnionOf(operands) => {
                write!(f, "(")?;
                for (i, operand) in operands.iter().enumerate() {
                    if i > 0 { write!(f, " ⊔ ")?; }
                    write!(f, "{}", operand)?;
                }
                write!(f, ")")
            },
            ClassExpression::ObjectComplementOf(operand) => write!(f, "¬{}", operand),
            ClassExpression::ObjectOneOf(individuals) => {
                write!(f, "{{")?;
                for (i, individual) in individuals.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", individual.iri)?;
                }
                write!(f, "}}")
            },
            ClassExpression::ObjectSomeValuesFrom { property, filler } => 
                write!(f, "∃{}.{}", property, filler),
            ClassExpression::ObjectAllValuesFrom { property, filler } => 
                write!(f, "∀{}.{}", property, filler),
            ClassExpression::ObjectHasValue { property, individual } => 
                write!(f, "∃{}.{{{}}}", property, individual.iri),
            ClassExpression::ObjectHasSelf { property } => 
                write!(f, "∃{}.Self", property),
            ClassExpression::ObjectMinCardinality { cardinality, property, filler } => {
                if let Some(filler) = filler {
                    write!(f, "≥{} {}.{}", cardinality, property, filler)
                } else {
                    write!(f, "≥{} {}", cardinality, property)
                }
            },
            ClassExpression::ObjectMaxCardinality { cardinality, property, filler } => {
                if let Some(filler) = filler {
                    write!(f, "≤{} {}.{}", cardinality, property, filler)
                } else {
                    write!(f, "≤{} {}", cardinality, property)
                }
            },
            ClassExpression::ObjectExactCardinality { cardinality, property, filler } => {
                if let Some(filler) = filler {
                    write!(f, "={} {}.{}", cardinality, property, filler)
                } else {
                    write!(f, "={} {}", cardinality, property)
                }
            },
            // For data property expressions, just use a simplified representation
            _ => write!(f, "ComplexExpression"),
        }
    }
}

impl std::fmt::Display for ObjectPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectPropertyExpression::ObjectProperty(prop) => write!(f, "{}", prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => write!(f, "{}⁻", prop.iri),
            ObjectPropertyExpression::PropertyChain(chain) => {
                write!(f, "PropertyChain(")?;
                for (i, prop) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ∘ ")?;
                    }
                    write!(f, "{}", prop)?;
                }
                write!(f, ")")
            }
        }
    }
}