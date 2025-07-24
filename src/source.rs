//! Source trait and implementations for loading configuration data

use crate::Result;
use std::env;

/// Trait for configuration sources that can provide values for specific fields
/// 
/// Sources should be efficient and handle arbitrarily large datasets by providing
/// values only for requested fields rather than loading entire datasets.
pub trait Source {
    /// Get a value for a specific field
    /// 
    /// Returns None if the field is not available in this source.
    /// The field name should be a simple identifier without complex paths.
    fn get_field(&self, field: &str) -> Result<Option<serde_json::Value>>;
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
    fn get_field(&self, field: &str) -> Result<Option<serde_json::Value>> {
        let env_key = self.field_to_env_key(field);
        if let Ok(value) = env::var(&env_key) {
            // Try to parse as different types for better type compatibility
            if let Ok(bool_val) = value.parse::<bool>() {
                Ok(Some(serde_json::Value::Bool(bool_val)))
            } else if let Ok(int_val) = value.parse::<i64>() {
                Ok(Some(serde_json::Value::Number(serde_json::Number::from(int_val))))
            } else if let Ok(float_val) = value.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(float_val) {
                    Ok(Some(serde_json::Value::Number(num)))
                } else {
                    Ok(Some(serde_json::Value::String(value)))
                }
            } else {
                Ok(Some(serde_json::Value::String(value)))
            }
        } else {
            Ok(None)
        }
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
        
        assert_eq!(result, Some(serde_json::Value::String("test_value".to_string())));
        
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
        
        assert_eq!(source.get_field("SIMPLE_KEY").unwrap(), Some(serde_json::Value::String("simple_value".to_string())));
        
        unsafe {
            env::remove_var("SIMPLE_KEY");
        }
    }
    
    #[test]
    fn test_env_source_type_conversion() {
        unsafe {
            env::set_var("TEST_number", "42");
            env::set_var("TEST_flag", "true");
            env::set_var("TEST_float", "3.14");
        }
        
        let source = EnvSource::with_prefix("TEST");
        
        // Test number conversion
        assert_eq!(source.get_field("number").unwrap(), 
                   Some(serde_json::Value::Number(serde_json::Number::from(42))));
        
        // Test boolean conversion
        assert_eq!(source.get_field("flag").unwrap(), 
                   Some(serde_json::Value::Bool(true)));
        
        // Test float conversion
        if let Some(serde_json::Value::Number(n)) = source.get_field("float").unwrap() {
            assert!((n.as_f64().unwrap() - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected a number");
        }
        
        unsafe {
            env::remove_var("TEST_number");
            env::remove_var("TEST_flag");
            env::remove_var("TEST_float");
        }
    }
}