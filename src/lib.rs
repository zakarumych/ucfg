//! Ultimate Configuration Library
//!
//! This library provides a comprehensive solution for configuring applications from various sources,
//! layering configurations, supporting overloads, extensions, and more.
//!
//! # Features
//!
//! - **Source trait**: Provide configuration values efficiently from various sources
//! - **Direct source visiting**: Config types visit sources directly for needed values
//! - **Environment variables**: Support for simple field-based environment variable mapping
//! - **Serde integration**: Works with any serde-compatible types
//! - **Layered configuration**: Merge multiple sources with override support
//! - **Builder pattern**: Flexible configuration building
//!
//! # Architecture
//!
//! The library uses a direct visiting pattern where Config implementations visit sources
//! to gather the values they need. Sources provide values for specific fields rather than
//! complex key paths, keeping the architecture simple and efficient.
//!
//! This design allows the library to work with:
//! - Large databases (implement `Source` with SQL queries for specific fields)
//! - Remote APIs (implement `Source` with HTTP requests for individual fields)  
//! - File systems (implement `Source` with field-based file lookups)
//! - Environment variables (use `EnvSource` for field-based env var mapping)
//! - Any source that can provide field-value lookups
//!
//! ## Source Implementations
//!
//! ### Custom `Source` Implementations  
//! - Best for: Large external sources (databases, APIs, file systems)
//! - Advantage: True streaming and on-demand querying
//! - Use case: Production applications with large configuration datasets
//!
//! ### `EnvSource`
//! - Queries environment variables directly using simple field names
//! - Naturally scalable (no memory limitations)
//! - Maps field names to environment variables with optional prefixes
//!
//! # Quick Start
//!
//! ```rust
//! use ucfg::{Builder, EnvSource};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, Default)]
//! struct Config {
//!     name: String,
//!     port: u16,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Build configuration from environment variables
//! let config: Config = Builder::new()
//!     .add_source(EnvSource::with_prefix("APP"))
//!     .build()?;
//!
//! println!("Config: {:?}", config);
//! # Ok(())
//! # }
//! ```
//!
//! # Environment Variables
//!
//! Environment variables are mapped to configuration fields using simple names:
//!
//! - `APP_name` → `name` field
//! - `APP_port` → `port` field
//! - `APP_enabled` → `enabled` field
//!
//! # Scalability
//!
//! The direct visiting pattern ensures that only required configuration values are loaded,
//! making it efficient even for very large configuration sources. Sources can be
//! arbitrarily large (databases, APIs, file systems) without memory concerns.

use std::error::Error as StdError;
use std::fmt;

pub mod source;
pub mod config;
pub mod builder;

pub use source::{Source, EnvSource};
pub use config::{Config, FromStrConfig, value_from_str};
pub use builder::Builder;

/// Error type for configuration operations
#[derive(Debug)]
pub enum Error {
    /// Error from a source
    Source(Box<dyn StdError + Send + Sync>),
    /// Error during deserialization
    Deserialize(String),
    /// Error during merging
    Merge(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Source(e) => write!(f, "Source error: {}", e),
            Error::Deserialize(msg) => write!(f, "Deserialization error: {}", msg),
            Error::Merge(msg) => write!(f, "Merge error: {}", msg),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Source(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::env;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct AppConfig {
        name: String,
        port: u16,
        enabled: bool,
    }

    #[test]
    fn test_configuration_example() {
        // Setup environment variables
        unsafe {
            env::set_var("APP_name", "ProductionApp");
            env::set_var("APP_port", "8080");
            env::set_var("APP_enabled", "true");
        }

        // Build configuration from environment
        let final_config: AppConfig = Builder::new()
            .add_source(EnvSource::with_prefix("APP"))
            .build()
            .unwrap();

        println!("Final config: {:#?}", final_config);

        // Verify the configuration
        assert_eq!(final_config.name, "ProductionApp");
        assert_eq!(final_config.port, 8080);
        assert_eq!(final_config.enabled, true);

        // Cleanup
        unsafe {
            env::remove_var("APP_name");
            env::remove_var("APP_port");
            env::remove_var("APP_enabled");
        }
    }
}
