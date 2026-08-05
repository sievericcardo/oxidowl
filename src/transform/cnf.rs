use crate::ontology::*;

/// Converts class expressions to Clausal Normal Form (CNF).
/// CNF is a conjunction of disjunctions of literals.
/// Builds on top of NNF (Negation Normal Form).
pub struct ClausalNormalFormConverter;

impl ClausalNormalFormConverter {
    /// Convert a class expression to CNF.
    /// First converts to NNF, then distributes disjunctions over conjunctions.
    #[must_use]
    pub fn to_cnf(expr: &ClassExpression) -> ClassExpression {
        let nnf = expr.to_nnf();
        Self::distribute_union_over_intersection(&nnf)
    }

    /// Convert a class expression to Disjunctive Normal Form (DNF).
    /// DNF is a disjunction of conjunctions of literals.
    #[must_use]
    pub fn to_dnf(expr: &ClassExpression) -> ClassExpression {
        let nnf = expr.to_nnf();
        Self::distribute_intersection_over_union(&nnf)
    }

    /// Check if a class expression is in CNF.
    #[must_use]
    pub fn is_cnf(expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                conjuncts.iter().all(Self::is_clause)
            }
            _ => Self::is_clause(expr),
        }
    }

    /// Check if a class expression is in DNF.
    #[must_use]
    pub fn is_dnf(expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::ObjectUnionOf(disjuncts) => disjuncts.iter().all(Self::is_cube),
            _ => Self::is_cube(expr),
        }
    }

    fn is_clause(expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::ObjectUnionOf(operands) => operands.iter().all(Self::is_literal),
            _ => Self::is_literal(expr),
        }
    }

    fn is_cube(expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::ObjectIntersectionOf(operands) => {
                operands.iter().all(Self::is_literal)
            }
            _ => Self::is_literal(expr),
        }
    }

    fn is_literal(expr: &ClassExpression) -> bool {
        matches!(
            expr,
            ClassExpression::Class(_) | ClassExpression::ObjectComplementOf(_)
        )
    }

    fn distribute_union_over_intersection(expr: &ClassExpression) -> ClassExpression {
        match expr {
            ClassExpression::ObjectUnionOf(disjuncts) => {
                for (i, d) in disjuncts.iter().enumerate() {
                    if let ClassExpression::ObjectIntersectionOf(conjuncts) = d {
                        let mut rest: Vec<ClassExpression> = disjuncts.clone();
                        rest.remove(i);
                        let rest_union = if rest.len() == 1 {
                            rest[0].clone()
                        } else {
                            ClassExpression::ObjectUnionOf(rest)
                        };

                        let distributed: Vec<ClassExpression> = conjuncts
                            .iter()
                            .map(|c| {
                                ClassExpression::ObjectUnionOf(vec![c.clone(), rest_union.clone()])
                            })
                            .collect();

                        let simplified: Vec<ClassExpression> = distributed
                            .iter()
                            .map(Self::distribute_union_over_intersection)
                            .collect();

                        return ClassExpression::ObjectIntersectionOf(simplified);
                    }
                }
                expr.clone()
            }
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                let processed: Vec<ClassExpression> = conjuncts
                    .iter()
                    .map(Self::distribute_union_over_intersection)
                    .collect();
                ClassExpression::ObjectIntersectionOf(processed)
            }
            _ => expr.clone(),
        }
    }

    fn distribute_intersection_over_union(expr: &ClassExpression) -> ClassExpression {
        match expr {
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                for (i, c) in conjuncts.iter().enumerate() {
                    if let ClassExpression::ObjectUnionOf(disjuncts) = c {
                        let mut rest: Vec<ClassExpression> = conjuncts.clone();
                        rest.remove(i);
                        let rest_conj = if rest.len() == 1 {
                            rest[0].clone()
                        } else {
                            ClassExpression::ObjectIntersectionOf(rest)
                        };

                        let distributed: Vec<ClassExpression> = disjuncts
                            .iter()
                            .map(|d| {
                                ClassExpression::ObjectIntersectionOf(vec![
                                    d.clone(),
                                    rest_conj.clone(),
                                ])
                            })
                            .collect();

                        let simplified: Vec<ClassExpression> = distributed
                            .iter()
                            .map(Self::distribute_intersection_over_union)
                            .collect();

                        return ClassExpression::ObjectUnionOf(simplified);
                    }
                }
                expr.clone()
            }
            ClassExpression::ObjectUnionOf(disjuncts) => {
                let processed: Vec<ClassExpression> = disjuncts
                    .iter()
                    .map(Self::distribute_intersection_over_union)
                    .collect();
                ClassExpression::ObjectUnionOf(processed)
            }
            _ => expr.clone(),
        }
    }
}
