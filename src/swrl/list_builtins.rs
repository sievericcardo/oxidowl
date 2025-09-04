//! SWRL List Built-in Predicates
//!
//! This module implements list manipulation built-in predicates for SWRL.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

// =============================================================================
// LIST BUILT-INS
// =============================================================================

/// List concatenation built-in predicate
pub struct ListConcatBuiltIn;

impl SWRLBuiltIn for ListConcatBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() < 2 {
            return Err(Error::reasoning("ListConcat expects at least 2 arguments"));
        }

        // Parse input lists and concatenate them
        let mut concatenated_items = Vec::new();

        for arg in &args[1..] {
            let items = self.parse_list_value(arg)?;
            concatenated_items.extend(items);
        }

        // Create result list representation
        let result_list = self.create_list_value(concatenated_items)?;

        // Check if first argument matches the result
        Ok(SWRLValue::Boolean(
            self.lists_equal(&args[0], &result_list)?,
        ))
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#listConcat"
    }

    fn arity(&self) -> Option<usize> {
        None // Variable arity
    }
}

/// List member built-in predicate
pub struct MemberBuiltIn;

impl SWRLBuiltIn for MemberBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("Member expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (element, SWRLValue::String(list_str)) => {
                // Simple list representation as comma-separated values
                let members: Vec<&str> = list_str.split(',').map(|s| s.trim()).collect();
                let element_str = match element {
                    SWRLValue::String(s) => s.as_str(),
                    SWRLValue::Integer(i) => {
                        return Ok(SWRLValue::Boolean(
                            members
                                .iter()
                                .any(|m| m.parse::<i64>().map_or(false, |n| n == *i)),
                        ));
                    }
                    SWRLValue::Float(f) => {
                        return Ok(SWRLValue::Boolean(members.iter().any(|m| {
                            m.parse::<f64>()
                                .map_or(false, |n| (n - f).abs() < f64::EPSILON)
                        })));
                    }
                    _ => {
                        return Err(Error::reasoning(
                            "Unsupported element type for list membership",
                        ));
                    }
                };
                Ok(SWRLValue::Boolean(members.contains(&element_str)))
            }
            _ => Err(Error::reasoning(
                "Member requires element and list arguments",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#member"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// List length built-in predicate
pub struct ListLengthBuiltIn;

impl SWRLBuiltIn for ListLengthBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning("ListLength expects exactly 2 arguments"));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Integer(result), SWRLValue::String(list_str)) => {
                let length = if list_str.is_empty() {
                    0
                } else {
                    list_str.split(',').count() as i64
                };
                Ok(SWRLValue::Boolean(*result == length))
            }
            _ => Err(Error::reasoning(
                "ListLength requires integer result and list arguments",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#length"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// List intersection built-in predicate
pub struct ListIntersectionBuiltIn;

impl SWRLBuiltIn for ListIntersectionBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "ListIntersection expects exactly 3 arguments (result, list1, list2)",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(result), SWRLValue::String(list1), SWRLValue::String(list2)) => {
                let items1: Vec<&str> = if list1.is_empty() {
                    Vec::new()
                } else {
                    list1.split(',').map(|s| s.trim()).collect()
                };
                let items2: Vec<&str> = if list2.is_empty() {
                    Vec::new()
                } else {
                    list2.split(',').map(|s| s.trim()).collect()
                };

                let intersection: Vec<&str> = items1
                    .into_iter()
                    .filter(|item| items2.contains(item))
                    .collect();

                let intersection_str = intersection.join(",");
                Ok(SWRLValue::Boolean(*result == intersection_str))
            }
            _ => Err(Error::reasoning(
                "ListIntersection requires string arguments",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#listIntersection"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// List subtraction built-in predicate
pub struct ListSubtractionBuiltIn;

impl SWRLBuiltIn for ListSubtractionBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning(
                "ListSubtraction expects exactly 3 arguments (result, list1, list2)",
            ));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::String(result), SWRLValue::String(list1), SWRLValue::String(list2)) => {
                let items1: Vec<&str> = if list1.is_empty() {
                    Vec::new()
                } else {
                    list1.split(',').map(|s| s.trim()).collect()
                };
                let items2: Vec<&str> = if list2.is_empty() {
                    Vec::new()
                } else {
                    list2.split(',').map(|s| s.trim()).collect()
                };

                let difference: Vec<&str> = items1
                    .into_iter()
                    .filter(|item| !items2.contains(item))
                    .collect();

                let difference_str = difference.join(",");
                Ok(SWRLValue::Boolean(*result == difference_str))
            }
            _ => Err(Error::reasoning(
                "ListSubtraction requires string arguments",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#listSubtraction"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// First element of list built-in predicate
pub struct FirstBuiltIn;

impl SWRLBuiltIn for FirstBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "First expects exactly 2 arguments (result, list)",
            ));
        }

        match (&args[0], &args[1]) {
            (result, SWRLValue::String(list)) => {
                if list.is_empty() {
                    return Err(Error::reasoning("Cannot get first element of empty list"));
                }

                let first_item = list.split(',').next().unwrap().trim();

                // Try to match the type of the result
                match result {
                    SWRLValue::String(expected) => Ok(SWRLValue::Boolean(*expected == first_item)),
                    SWRLValue::Integer(expected) => {
                        if let Ok(parsed) = first_item.parse::<i64>() {
                            Ok(SWRLValue::Boolean(*expected == parsed))
                        } else {
                            Ok(SWRLValue::Boolean(false))
                        }
                    }
                    SWRLValue::Float(expected) => {
                        if let Ok(parsed) = first_item.parse::<f64>() {
                            Ok(SWRLValue::Boolean((expected - parsed).abs() < f64::EPSILON))
                        } else {
                            Ok(SWRLValue::Boolean(false))
                        }
                    }
                    _ => Ok(SWRLValue::String(first_item.to_string())),
                }
            }
            _ => Err(Error::reasoning("First requires list argument")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#first"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Rest of list (all but first) built-in predicate
pub struct RestBuiltIn;

impl SWRLBuiltIn for RestBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "Rest expects exactly 2 arguments (result, list)",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::String(result), SWRLValue::String(list)) => {
                if list.is_empty() {
                    return Ok(SWRLValue::Boolean(result.is_empty()));
                }

                let items: Vec<&str> = list.split(',').map(|s| s.trim()).collect();
                let rest_items = if items.len() > 1 {
                    items[1..].join(",")
                } else {
                    String::new()
                };

                Ok(SWRLValue::Boolean(*result == rest_items))
            }
            _ => Err(Error::reasoning("Rest requires string arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#rest"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Sublist built-in predicate
pub struct SublistBuiltIn;

impl SWRLBuiltIn for SublistBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 4 {
            return Err(Error::reasoning(
                "Sublist expects exactly 4 arguments (result, list, start, length)",
            ));
        }

        match (&args[0], &args[1], &args[2], &args[3]) {
            (
                SWRLValue::String(result),
                SWRLValue::String(list),
                SWRLValue::Integer(start),
                SWRLValue::Integer(length),
            ) => {
                if list.is_empty() {
                    return Ok(SWRLValue::Boolean(result.is_empty()));
                }

                let items: Vec<&str> = list.split(',').map(|s| s.trim()).collect();
                let start_idx = (*start as usize).saturating_sub(1); // 1-based indexing
                let end_idx = start_idx + (*length as usize);

                let sublist = if start_idx < items.len() {
                    let actual_end = end_idx.min(items.len());
                    items[start_idx..actual_end].join(",")
                } else {
                    String::new()
                };

                Ok(SWRLValue::Boolean(*result == sublist))
            }
            _ => Err(Error::reasoning(
                "Sublist requires string list and integer indices",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#sublist"
    }

    fn arity(&self) -> Option<usize> {
        Some(4)
    }
}

/// Empty list check built-in predicate
pub struct EmptyBuiltIn;

impl SWRLBuiltIn for EmptyBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 1 {
            return Err(Error::reasoning("Empty expects exactly 1 argument (list)"));
        }

        match &args[0] {
            SWRLValue::String(list) => Ok(SWRLValue::Boolean(
                list.is_empty() || list.trim().is_empty(),
            )),
            _ => Err(Error::reasoning("Empty requires string list argument")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#empty"
    }

    fn arity(&self) -> Option<usize> {
        Some(1)
    }
}

/// Function to register all list built-ins to a registry
pub fn register_list_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#listConcat"),
        Box::new(ListConcatBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#member"),
        Box::new(MemberBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#length"),
        Box::new(ListLengthBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#listIntersection"),
        Box::new(ListIntersectionBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#listSubtraction"),
        Box::new(ListSubtractionBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#first"),
        Box::new(FirstBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#rest"),
        Box::new(RestBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#sublist"),
        Box::new(SublistBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#empty"),
        Box::new(EmptyBuiltIn),
    );
}

// Helper methods for list operations
impl ListConcatBuiltIn {
    /// Parse a SWRL value as a list of items
    fn parse_list_value(&self, value: &SWRLValue) -> Result<Vec<String>> {
        match value {
            SWRLValue::String(s) => {
                if s.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(s.split(',').map(|s| s.trim().to_string()).collect())
                }
            }
            SWRLValue::Literal(lit) => {
                let s = &lit.value;
                if s.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(s.split(',').map(|s| s.trim().to_string()).collect())
                }
            }
            _ => Err(Error::reasoning("Cannot parse non-string value as list")),
        }
    }

    /// Create a SWRL value representing a list
    fn create_list_value(&self, items: Vec<String>) -> Result<SWRLValue> {
        Ok(SWRLValue::String(items.join(",")))
    }

    /// Check if two list values are equal
    fn lists_equal(&self, list1: &SWRLValue, list2: &SWRLValue) -> Result<bool> {
        let items1 = self.parse_list_value(list1)?;
        let items2 = self.parse_list_value(list2)?;
        Ok(items1 == items2)
    }
}

// Similar helper implementations for other list built-ins
impl MemberBuiltIn {
    fn parse_list_value(&self, value: &SWRLValue) -> Result<Vec<String>> {
        match value {
            SWRLValue::String(s) => {
                if s.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(s.split(',').map(|s| s.trim().to_string()).collect())
                }
            }
            _ => Err(Error::reasoning("Cannot parse non-string value as list")),
        }
    }
}

impl ListLengthBuiltIn {
    fn parse_list_value(&self, value: &SWRLValue) -> Result<Vec<String>> {
        match value {
            SWRLValue::String(s) => {
                if s.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(s.split(',').map(|s| s.trim().to_string()).collect())
                }
            }
            _ => Err(Error::reasoning("Cannot parse non-string value as list")),
        }
    }
}

impl ListIntersectionBuiltIn {
    fn parse_list_value(&self, value: &SWRLValue) -> Result<Vec<String>> {
        match value {
            SWRLValue::String(s) => {
                if s.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(s.split(',').map(|s| s.trim().to_string()).collect())
                }
            }
            _ => Err(Error::reasoning("Cannot parse non-string value as list")),
        }
    }

    fn create_list_value(&self, items: Vec<String>) -> Result<SWRLValue> {
        Ok(SWRLValue::String(items.join(",")))
    }

    fn lists_equal(&self, list1: &SWRLValue, list2: &SWRLValue) -> Result<bool> {
        let items1 = self.parse_list_value(list1)?;
        let items2 = self.parse_list_value(list2)?;
        Ok(items1 == items2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLValue;

    #[test]
    fn test_list_member() {
        let builtin = MemberBuiltIn;

        let args = vec![
            SWRLValue::String("apple".to_string()),
            SWRLValue::String("apple,banana,cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        let args = vec![
            SWRLValue::String("grape".to_string()),
            SWRLValue::String("apple,banana,cherry".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(false));
    }

    #[test]
    fn test_list_length() {
        let builtin = ListLengthBuiltIn;

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
    fn test_list_intersection() {
        let builtin = ListIntersectionBuiltIn;

        let args = vec![
            SWRLValue::String("apple,banana".to_string()),
            SWRLValue::String("apple,banana,cherry".to_string()),
            SWRLValue::String("apple,banana,grape".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }
}
