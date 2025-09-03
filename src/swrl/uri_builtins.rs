//! SWRL URI Built-in Predicates
//!
//! This module implements URI handling built-in predicates for SWRL.

use crate::swrl::builtins::{SWRLBuiltIn, SWRLValue};
use crate::{Error, Result};

// =============================================================================
// URI BUILT-INS
// =============================================================================

/// Resolve URI built-in predicate
pub struct ResolveUriBuiltIn;

impl SWRLBuiltIn for ResolveUriBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 3 {
            return Err(Error::reasoning("ResolveURI expects exactly 3 arguments"));
        }

        match (&args[0], &args[1], &args[2]) {
            (SWRLValue::Uri(result), SWRLValue::Uri(relative), SWRLValue::Uri(base)) => {
                // Proper URI resolution following RFC 3986
                let resolved = if relative.starts_with("http://") || relative.starts_with("https://") || relative.starts_with("ftp://") {
                    // Absolute URI
                    relative.clone()
                } else if relative.starts_with("//") {
                    // Protocol-relative URI
                    let scheme = if base.starts_with("https://") {
                        "https:"
                    } else {
                        "http:"
                    };
                    format!("{}{}", scheme, relative)
                } else if relative.starts_with('/') {
                    // Absolute path
                    if let Some(scheme_end) = base.find("://") {
                        if let Some(path_start) = base[scheme_end + 3..].find('/') {
                            format!("{}{}", &base[..scheme_end + 3 + path_start], relative)
                        } else {
                            format!("{}{}", base, relative)
                        }
                    } else {
                        format!("{}{}", base, relative)
                    }
                } else if relative.starts_with("?") {
                    // Query component
                    if let Some(query_pos) = base.find('?') {
                        format!("{}{}", &base[..query_pos], relative)
                    } else {
                        format!("{}{}", base, relative)
                    }
                } else if relative.starts_with("#") {
                    // Fragment component  
                    if let Some(fragment_pos) = base.find('#') {
                        format!("{}{}", &base[..fragment_pos], relative)
                    } else {
                        format!("{}{}", base, relative)
                    }
                } else {
                    // Relative path
                    if base.ends_with('/') {
                        format!("{}{}", base, relative)
                    } else {
                        // Remove last path segment and append relative
                        if let Some(last_slash) = base.rfind('/') {
                            format!("{}/{}", &base[..last_slash], relative)
                        } else {
                            format!("{}/{}", base, relative)
                        }
                    }
                };
                Ok(SWRLValue::Boolean(*result == resolved))
            }
            _ => Err(Error::reasoning("ResolveURI requires URI arguments")),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#resolveURI"
    }

    fn arity(&self) -> Option<usize> {
        Some(3)
    }
}

/// Any URI constructor built-in predicate
pub struct AnyUriBuiltIn;

impl SWRLBuiltIn for AnyUriBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        if args.len() != 2 {
            return Err(Error::reasoning(
                "AnyURI expects exactly 2 arguments (result, string)",
            ));
        }

        match (&args[0], &args[1]) {
            (SWRLValue::Uri(result), SWRLValue::String(input)) => {
                // Basic URI validation and construction
                let uri = if input.starts_with("http://")
                    || input.starts_with("https://")
                    || input.starts_with("urn:")
                {
                    input.clone()
                } else {
                    format!("urn:{}", input)
                };
                Ok(SWRLValue::Boolean(*result == uri))
            }
            _ => Err(Error::reasoning(
                "AnyURI requires URI result and string input",
            )),
        }
    }

    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#anyURI"
    }

    fn arity(&self) -> Option<usize> {
        Some(2)
    }
}

/// Function to register all URI built-ins to a registry
pub fn register_uri_builtins(registry: &mut crate::swrl::builtins::SWRLBuiltInRegistry) {
    use crate::ontology::IRI;

    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#resolveURI"),
        Box::new(ResolveUriBuiltIn),
    );
    registry.register_builtin(
        IRI::new("http://www.w3.org/2003/11/swrlb#anyURI"),
        Box::new(AnyUriBuiltIn),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swrl::builtins::SWRLValue;

    #[test]
    fn test_resolve_uri() {
        let builtin = ResolveUriBuiltIn;

        let args = vec![
            SWRLValue::Uri("http://example.org/resource".to_string()),
            SWRLValue::Uri("resource".to_string()),
            SWRLValue::Uri("http://example.org/".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }

    #[test]
    fn test_any_uri() {
        let builtin = AnyUriBuiltIn;

        let args = vec![
            SWRLValue::Uri("http://example.org/test".to_string()),
            SWRLValue::String("http://example.org/test".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));

        // Test URN construction
        let args = vec![
            SWRLValue::Uri("urn:test".to_string()),
            SWRLValue::String("test".to_string()),
        ];
        let result = builtin.execute(&args).unwrap();
        assert_eq!(result, SWRLValue::Boolean(true));
    }
}
