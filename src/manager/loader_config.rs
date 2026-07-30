//! OntologyLoaderConfiguration — consolidated loader settings.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingImportStrategy {
    ThrowException,
    Silent,
}

#[derive(Debug, Clone)]
pub struct LoaderConfig {
    pub connection_timeout: Duration,
    pub read_timeout: Duration,
    pub retry_count: u32,
    pub retry_backoff: Duration,
    pub strict_parsing: bool,
    pub follow_redirects: bool,
    pub accept_compression: bool,
    pub missing_import_strategy: MissingImportStrategy,
    pub max_import_depth: usize,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            retry_count: 3,
            retry_backoff: Duration::from_secs(1),
            strict_parsing: false,
            follow_redirects: true,
            accept_compression: true,
            missing_import_strategy: MissingImportStrategy::Silent,
            max_import_depth: 20,
        }
    }
}
