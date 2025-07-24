//! Ultimate Configuration Library
//!
//! This library provides a comprehensive solution for configuring applications from various sources,
//! with a focus on efficiency and scalability.
//!
//! # Key Principles
//!
//! - **No JSON bias**: Sources return strings, Config implementations choose parsing strategy
//! - **FromStr first**: Environment variables use FromStr parsing by default for better type safety
//! - **Individual source visiting**: Config visits one source at a time, Builder coordinates
//! - **Manual implementations**: No blanket Config implementations - explicit control over parsing
//! - **Scalable sources**: Sources can be arbitrarily large (databases, APIs) without memory concerns
//!
//! # Architecture
//!
//! The library uses a coordinated approach where:
//! 1. `Builder` coordinates multiple sources with override semantics  
//! 2. `Config` implementations define how to extract and parse specific fields
//! 3. `Source` implementations provide string values for requested fields
//! 4. Field parsing uses `FromStr` for environment variables, custom logic for other sources
//!
//! This design allows the library to work with:
//! - Large databases (implement `Source` with SQL queries for specific fields)
//! - Remote APIs (implement `Source` with HTTP requests for individual fields)  
//! - File systems (implement `Source` with field-based file lookups)
//! - Environment variables (use `EnvSource` for field-based env var mapping)
//! - Any source that can provide field-value lookups
//!
//! # Quick Start
//!
//! ```rust
//! use ucfg::{Builder, EnvSource, Config, get_field_value};
//! use std::str::FromStr;
//!
//! #[derive(Debug, Clone, PartialEq)]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     enabled: bool,
//! }
//!
//! impl Default for AppConfig {
//!     fn default() -> Self {
//!         Self {
//!             name: "MyApp".to_string(),
//!             port: 3000,
//!             enabled: false,
//!         }
//!     }
//! }
//!
//! impl Config for AppConfig {
//!     fn configure_from_source(source: &dyn ucfg::Source) -> ucfg::Result<Self> {
//!         let mut config = Self::default();
//!         config.update_from_source(source)?;
//!         Ok(config)
//!     }
//!     
//!     fn update_from_source(&mut self, source: &dyn ucfg::Source) -> ucfg::Result<()> {
//!         if let Some(name) = get_field_value::<String>(source, "name")? {
//!             self.name = name;
//!         }
//!         
//!         if let Some(port) = get_field_value::<u16>(source, "port")? {
//!             self.port = port;
//!         }
//!         
//!         if let Some(enabled) = get_field_value::<bool>(source, "enabled")? {
//!             self.enabled = enabled;
//!         }
//!         
//!         Ok(())
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Build configuration from environment variables
//! let config: AppConfig = Builder::new()
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
//! - `APP_name` → `name` field (used as string)
//! - `APP_port` → `port` field (parsed using `FromStr`)
//! - `APP_enabled` → `enabled` field (parsed using `FromStr`)
//!
//! # Scalability
//!
//! The field-based visiting pattern ensures that only required configuration values are loaded,
//! making it efficient even for very large configuration sources. Sources can be
//! arbitrarily large (databases, APIs, file systems) without memory concerns.

use std::error::Error as StdError;
use std::fmt;

pub mod source;
pub mod config;
pub mod builder;

pub use source::{Source, EnvSource, Visitor};
pub use config::{Config, parse_field_value, get_field_value};
pub use builder::Builder;

/// Error type for configuration operations
#[derive(Debug)]
pub enum Error {
    /// Error from a source
    Source(Box<dyn StdError + Send + Sync>),
    /// Error during parsing field values
    Parse(String),
    /// Error during deserialization
    Deserialize(String),
    /// Error during merging
    Merge(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Source(e) => write!(f, "Source error: {}", e),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
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
    use std::env;

    #[derive(Debug, Clone, PartialEq)]
    struct AppConfig {
        name: String,
        port: u16,
        enabled: bool,
    }

    impl Default for AppConfig {
        fn default() -> Self {
            Self {
                name: "DefaultApp".to_string(),
                port: 3000,
                enabled: false,
            }
        }
    }

    impl Config for AppConfig {
        fn configure_from_source(source: &dyn Source) -> Result<Self> {
            let mut config = Self::default();
            config.update_from_source(source)?;
            Ok(config)
        }
        
        fn update_from_source(&mut self, source: &dyn Source) -> Result<()> {
            if let Some(name) = config::get_field_value::<String>(source, "name")? {
                self.name = name;
            }
            
            if let Some(port) = config::get_field_value::<u16>(source, "port")? {
                self.port = port;
            }
            
            if let Some(enabled) = config::get_field_value::<bool>(source, "enabled")? {
                self.enabled = enabled;
            }
            
            Ok(())
        }
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
