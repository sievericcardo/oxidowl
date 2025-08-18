//! SWRL Collection Built-in Predicates
//!
//! This module implements collection handling built-in predicates for SWRL,
//! including set operations and collection queries.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};
use std::collections::HashSet;

// =============================================================================
// COLLECTION BUILT-INS
// =============================================================================

/// Count elements in a collection
pub struct CountBuiltIn;

impl SWRLBuiltIn for CountBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "Count expects exactly 2 arguments (result, collection)",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(expected_count), SWRLValue::String(collection_str)) => {
                let items: Vec<&str> = if collection_str.is_empty() {
                    Vec::new()
                } else {
                    collection_str.split(',').map(|s| s.trim()).collect()
                };

                let actual_count = items.len() as i64;
                Ok(SWRLValue::Boolean(*expected_count == actual_count))
            }
            _ => Err(Error::reasoning(
                "Count requires integer and collection arguments",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#count"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Check if collection is empty
pub struct IsEmptyBuiltIn;

impl SWRLBuiltIn for IsEmptyBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("IsEmpty expects exactly 1 argument"));
        }

        match &args[0] {
            SWRLValue::String(collection_str) => {
                Ok(SWRLValue::Boolean(collection_str.trim().is_empty()))
            }
            _ => Err(Error::reasoning("IsEmpty requires collection argument")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#isEmpty"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

/// Create union of two collections
pub struct UnionBuiltIn;

impl SWRLBuiltIn for UnionBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "Union expects exactly 3 arguments (result, collection1, collection2)",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(result), SWRLValue::String(coll1), SWRLValue::String(coll2)) => {
                let items1: HashSet<&str> = if coll1.is_empty() {
                    HashSet::new()
                } else {
                    coll1.split(',').map(|s| s.trim()).collect()
                };

                let items2: HashSet<&str> = if coll2.is_empty() {
                    HashSet::new()
                } else {
                    coll2.split(',').map(|s| s.trim()).collect()
                };

                let union: HashSet<&str> = items1.union(&items2).cloned().collect();
                let mut union_vec: Vec<&str> = union.into_iter().collect();
                union_vec.sort();

                let union_str = union_vec.join(",");
                Ok(SWRLValue::Boolean(*result == union_str))
            }
            _ => Err(Error::reasoning("Union requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#union"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Check if one collection is subset of another
pub struct SubsetBuiltIn;

impl SWRLBuiltIn for SubsetBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "Subset expects exactly 2 arguments (subset, superset)",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(subset_str), SWRLValue::String(superset_str)) => {
                let subset_items: HashSet<&str> = if subset_str.is_empty() {
                    HashSet::new()
                } else {
                    subset_str.split(',').map(|s| s.trim()).collect()
                };

                let superset_items: HashSet<&str> = if superset_str.is_empty() {
                    HashSet::new()
                } else {
                    superset_str.split(',').map(|s| s.trim()).collect()
                };

                let is_subset = subset_items.is_subset(&superset_items);
                Ok(SWRLValue::Boolean(is_subset))
            }
            _ => Err(Error::reasoning("Subset requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#subset"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Function to register all collection built-ins to a registry
pub fn register_collection_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#count"),
        Box::new(CountBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#isEmpty"),
        Box::new(IsEmptyBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#union"),
        Box::new(UnionBuiltIn),
    );

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#subset"),
        Box::new(SubsetBuiltIn),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_builtin() {
        let builtin = CountBuiltIn;

        let args = vec![
            SWRLValue::Integer(3),
            SWRLValue::String("apple,banana,cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![SWRLValue::Integer(0), SWRLValue::String("".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_is_empty_builtin() {
        let builtin = IsEmptyBuiltIn;

        let args = vec![SWRLValue::String("".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![SWRLValue::String("apple".to_string())];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_union_builtin() {
        let builtin = UnionBuiltIn;

        let args = vec![
            SWRLValue::String("apple,banana,cherry".to_string()),
            SWRLValue::String("apple,banana".to_string()),
            SWRLValue::String("cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_subset_builtin() {
        let builtin = SubsetBuiltIn;

        let args = vec![
            SWRLValue::String("apple,banana".to_string()),
            SWRLValue::String("apple,banana,cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![
            SWRLValue::String("apple,grape".to_string()),
            SWRLValue::String("apple,banana,cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }
}
