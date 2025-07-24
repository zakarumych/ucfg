//! Ultimate Configuration Library
//!
//! This library provides a comprehensive solution for configuring applications from various sources,
//! layering configurations, supporting overloads, extensions, and more.
//!
//! # Features
//!
//! - **Source trait**: Query configuration data from various sources efficiently
//! - **Visitor pattern**: Load only required values, supporting arbitrarily large sources
//! - **Environment variables**: Support for nested keys with prefix filtering
//! - **Serde integration**: Works with any serde-compatible types
//! - **Layered configuration**: Merge multiple sources with override support
//! - **Builder pattern**: Flexible configuration building
//!
//! # Architecture
//!
//! The library uses a visitor pattern to efficiently handle large configuration sources.
//! Instead of loading entire sources into memory, configuration types specify which
//! values they need, and sources are queried only for those specific keys.
//!
//! This design allows the library to work with:
//! - Large databases (implement `Source` with SQL queries)
//! - Remote APIs (implement `Source` with HTTP requests)  
//! - File systems (implement `Source` with file I/O)
//! - In-memory data (use `DeserializerSource` for convenience)
//! - Any source that can provide key-value lookups
//!
//! ## Source Implementations
//!
//! ### `DeserializerSource<T>`
//! - Best for: Small to medium in-memory data structures
//! - Limitation: Serializes data for each query (not suitable for very large data)
//! - Use case: Configuration structs, JSON files loaded into memory
//!
//! ### Custom `Source` Implementations  
//! - Best for: Large external sources (databases, APIs, file systems)
//! - Advantage: True streaming and on-demand querying
//! - Use case: Production applications with large configuration datasets
//!
//! ### `EnvSource`
//! - Queries environment variables directly
//! - Naturally scalable (no memory limitations)
//! - Supports nested keys with configurable separators
//!
//! # Quick Start
//!
//! ```rust
//! use ucfg::{Builder, DeserializerSource, EnvSource};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, Default)]
//! struct Config {
//!     name: String,
//!     port: u16,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create base configuration
//! let base = serde_json::json!({
//!     "name": "MyApp",
//!     "port": 8080
//! });
//!
//! // Build layered configuration
//! let config: Config = Builder::new()
//!     .add_source(DeserializerSource::new(base))
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
//! Environment variables are mapped to nested configuration using `__` as separator:
//!
//! - `APP_name` → `name`
//! - `APP_database__host` → `database.host`
//! - `APP_server__port` → `server.port`
//!
//! # Scalability
//!
//! The visitor pattern ensures that only required configuration values are loaded,
//! making it efficient even for very large configuration sources. Sources can be
//! arbitrarily large (databases, APIs, file systems) without memory concerns.

use std::error::Error as StdError;
use std::fmt;

pub mod source;
pub mod config;
pub mod builder;

pub use source::{Source, DeserializerSource, EnvSource};
pub use config::{Config, ConfigVisitor, SourceVisitor, FromStrConfig, value_from_str};
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
        database: DatabaseConfig,
        features: FeaturesConfig,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct DatabaseConfig {
        host: String,
        port: u16,
        username: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct FeaturesConfig {
        logging: bool,
        metrics: bool,
    }

    #[test]
    fn test_full_configuration_example() {
        // Setup environment variables
        unsafe {
            env::set_var("APP_name", "ProductionApp");
            env::set_var("APP_database__username", "produser");
        }

        // Base configuration from code
        let base_config = serde_json::json!({
            "name": "MyApp",
            "port": 8080,
            "database": {
                "host": "localhost",
                "port": 5432,
                "username": "dev"
            },
            "features": {
                "logging": true,
                "metrics": false
            }
        });

        // Override configuration
        let override_config = serde_json::json!({
            "port": 9000,
            "features": {
                "logging": false
            }
        });

        // Build configuration with multiple sources
        let final_config: AppConfig = Builder::new()
            .add_source(DeserializerSource::new(base_config))
            .add_source(EnvSource::with_prefix("APP"))
            .add_source(DeserializerSource::new(override_config))
            .build()
            .unwrap();

        println!("Final config: {:#?}", final_config);

        // Verify the layered configuration
        assert_eq!(final_config.name, "ProductionApp");           // from env
        assert_eq!(final_config.port, 9000);                     // from override
        assert_eq!(final_config.database.host, "localhost");     // from base
        assert_eq!(final_config.database.username, "produser");  // from env
        assert_eq!(final_config.features.logging, false);        // from override
        assert_eq!(final_config.features.metrics, false);        // from base

        // Cleanup
        unsafe {
            env::remove_var("APP_name");
            env::remove_var("APP_database__username");
        }
    }
}
