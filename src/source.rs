//! Source trait and implementations for loading configuration data

use crate::Result;
use std::env;

/// Trait for configuration sources that can provide values for specific fields
/// 
/// Sources should be efficient and handle arbitrarily large datasets by providing
/// values only for requested fields rather than loading entire datasets.
/// 
/// Sources can return string values or provide deserializer implementations
/// depending on the nature of the data source.
pub trait Source {
    /// Get a string value for a specific field
    /// 
    /// Returns None if the field is not available in this source.
    /// The field name should be a simple identifier without complex paths.
    fn get_field(&self, field: &str) -> Result<Option<String>>;
}

/// Source implementation for environment variables
pub struct EnvSource {
    prefix: Option<String>,
}

impl EnvSource {
    /// Create a new EnvSource that reads all environment variables
    pub fn new() -> Self {
        Self { prefix: None }
    }

    /// Create a new EnvSource with a prefix filter
    pub fn with_prefix<S: Into<String>>(prefix: S) -> Self {
        Self {
            prefix: Some(prefix.into()),
        }
    }

    /// Convert a field name to an environment variable key
    fn field_to_env_key(&self, field: &str) -> String {
        if let Some(ref prefix) = self.prefix {
            format!("{}_{}", prefix, field)
        } else {
            field.to_string()
        }
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for EnvSource {
    fn get_field(&self, field: &str) -> Result<Option<String>> {
        let env_key = self.field_to_env_key(field);
        Ok(env::var(&env_key).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_source_basic() {
        unsafe {
            env::set_var("TEST_KEY", "test_value");
        }
        
        let source = EnvSource::with_prefix("TEST");
        let result = source.get_field("KEY").unwrap();
        
        assert_eq!(result, Some("test_value".to_string()));
        
        // Test non-existent field
        assert_eq!(source.get_field("NONEXISTENT").unwrap(), None);
        
        unsafe {
            env::remove_var("TEST_KEY");
        }
    }
    
    #[test]
    fn test_env_source_no_prefix() {
        unsafe {
            env::set_var("SIMPLE_KEY", "simple_value");
        }
        
        let source = EnvSource::new();
        
        assert_eq!(source.get_field("SIMPLE_KEY").unwrap(), Some("simple_value".to_string()));
        
        unsafe {
            env::remove_var("SIMPLE_KEY");
        }
    }
    
    #[test]
    fn test_env_source_returns_strings() {
        unsafe {
            env::set_var("TEST_number", "42");
            env::set_var("TEST_flag", "true");
            env::set_var("TEST_float", "3.14");
        }
        
        let source = EnvSource::with_prefix("TEST");
        
        // Environment source returns raw strings - parsing is done by Config
        assert_eq!(source.get_field("number").unwrap(), Some("42".to_string()));
        assert_eq!(source.get_field("flag").unwrap(), Some("true".to_string()));
        assert_eq!(source.get_field("float").unwrap(), Some("3.14".to_string()));
        
        unsafe {
            env::remove_var("TEST_number");
            env::remove_var("TEST_flag");
            env::remove_var("TEST_float");
        }
    }
}