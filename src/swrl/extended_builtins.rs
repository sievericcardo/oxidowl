//! SWRL Extended Built-ins Registration
//!
//! This module provides consolidated registration of all extended SWRL built-ins.

use crate::swrl::{
    additional_builtins::register_additional_comparison_builtins,
    boolean_builtins::register_boolean_builtins, builtins::SWRLBuiltInRegistry,
    collection_builtins::register_collection_builtins, list_builtins::register_list_builtins,
    math_builtins::register_math_builtins, string_builtins::register_string_builtins,
    uri_builtins::register_uri_builtins,
};

/// Register all extended built-ins with the registry
pub fn register_extended_builtins(registry: &mut SWRLBuiltInRegistry) {
    register_math_builtins(registry);
    register_string_builtins(registry);
    register_uri_builtins(registry);
    register_boolean_builtins(registry);
    register_list_builtins(registry);
    register_additional_comparison_builtins(registry);
    register_collection_builtins(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::IRI;
    use crate::swrl::builtins::SWRLBuiltInRegistry;

    #[test]
    fn test_extended_builtins_registration() {
        let mut registry = SWRLBuiltInRegistry::new();
        register_extended_builtins(&mut registry);

        // Test that some built-ins from each category are registered
        assert!(
            registry
                .get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#booleanNot"))
                .is_some()
        );
        assert!(
            registry
                .get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#ceiling"))
                .is_some()
        );
        assert!(
            registry
                .get_builtin(&IRI::new(
                    "http://www.w3.org/2003/11/swrlb#stringEqualIgnoreCase"
                ))
                .is_some()
        );
        assert!(
            registry
                .get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#resolveURI"))
                .is_some()
        );
        assert!(
            registry
                .get_builtin(&IRI::new("http://www.w3.org/2003/11/swrlb#member"))
                .is_some()
        );

        // Check that we have a reasonable number of built-ins registered
        let builtin_count = registry.get_builtin_iris().len();
        assert!(
            builtin_count >= 20,
            "Expected at least 20 extended built-ins, got {}",
            builtin_count
        );
    }
}
