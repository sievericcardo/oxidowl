//! SWRL Extended Built-ins Registration
//!
//! This module provides consolidated registration of all extended SWRL built-ins.

use crate::swrl::{
    boolean_builtins,
    list_builtins,
    math_builtins,
    string_builtins,
    uri_builtins,
};

/// Register all extended SWRL built-ins to a registry
///
/// This function consolidates the registration of all built-ins that extend
/// the core SWRL specification, organized by category for better maintainability.
pub fn register_extended_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    // Register boolean built-ins
    boolean_builtins::register_boolean_builtins(registry);
    
    // Register mathematical built-ins
    math_builtins::register_math_builtins(registry);
    
    // Register string manipulation built-ins
    string_builtins::register_string_builtins(registry);
    
    // Register URI handling built-ins
    uri_builtins::register_uri_builtins(registry);
    
    // Register list manipulation built-ins
    list_builtins::register_list_builtins(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLBuiltInRegistry;
    use crate::ontology::IRI;

    #[test]
    fn test_extended_builtins_registration() {
        let mut registry = SWRLBuiltInRegistry::new();
        register_extended_builtins(&mut registry);
        
        // Test that some built-ins from each category are registered
        assert!(registry.get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#booleanNot")).is_some());
        assert!(registry.get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#ceiling")).is_some());
        assert!(registry.get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase")).is_some());
        assert!(registry.get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#resolveURI")).is_some());
        assert!(registry.get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#member")).is_some());
        
        // Check that we have a reasonable number of built-ins registered
        let builtin_count = registry.get_builtin_iris().len();
        assert!(builtin_count >= 20, "Expected at least 20 extended built-ins, got {}", builtin_count);
    }
}
