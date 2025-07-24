//! Ultimate Configuration Library
//!
//! This library provides a comprehensive solution for configuring applications from various sources,
//! layering configurations, supporting overloads, extensions, and more.

use std::error::Error as StdError;
use std::fmt;

pub mod source;
pub mod config;
pub mod builder;

pub use source::{Source, DeserializerSource, EnvSource};
pub use config::Config;
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
