//! Source trait and implementations for loading configuration data

use crate::Result;
use std::env;

/// Visitor trait for handling different types of values from sources
pub trait Visitor {
    /// Handle a string value
    fn visit_string(&mut self, value: String) -> Result<()>;
    
    /// Handle a JSON value (for structured data)
    fn visit_json(&mut self, value: serde_json::Value) -> Result<()>;
}

/// Trait for configuration sources that can provide values for specific fields
/// 
/// Sources should be efficient and handle arbitrarily large datasets by providing
/// values only for requested fields rather than loading entire datasets.
/// 
/// Sources may contain string values or structured data with deserializers
/// depending on the nature of the data source.
pub trait Source {
    /// Visit a field with the provided visitor
    /// 
    /// Returns false if the field is not available in this source.
    /// The field name should be a simple identifier without complex paths.
    fn visit_field(&self, field: &str, visitor: &mut dyn Visitor) -> Result<bool>;
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
    fn visit_field(&self, field: &str, visitor: &mut dyn Visitor) -> Result<bool> {
        let env_key = self.field_to_env_key(field);
        if let Ok(value) = env::var(&env_key) {
            visitor.visit_string(value)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Test visitor that collects values
    struct TestVisitor {
        string_value: Option<String>,
        json_value: Option<serde_json::Value>,
    }

    impl TestVisitor {
        fn new() -> Self {
            Self {
                string_value: None,
                json_value: None,
            }
        }

        fn get_string_value(&self) -> Option<&String> {
            self.string_value.as_ref()
        }
    }

    impl Visitor for TestVisitor {
        fn visit_string(&mut self, value: String) -> Result<()> {
            self.string_value = Some(value);
            Ok(())
        }

        fn visit_json(&mut self, value: serde_json::Value) -> Result<()> {
            self.json_value = Some(value);
            Ok(())
        }
    }

    #[test]
    fn test_env_source_basic() {
        unsafe {
            env::set_var("TEST_KEY", "test_value");
        }
        
        let source = EnvSource::with_prefix("TEST");
        let mut visitor = TestVisitor::new();
        let found = source.visit_field("KEY", &mut visitor).unwrap();
        
        assert!(found);
        assert_eq!(visitor.get_string_value(), Some(&"test_value".to_string()));
        
        // Test non-existent field
        let mut visitor2 = TestVisitor::new();
        let found = source.visit_field("NONEXISTENT", &mut visitor2).unwrap();
        assert!(!found);
        
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
        let mut visitor = TestVisitor::new();
        let found = source.visit_field("SIMPLE_KEY", &mut visitor).unwrap();
        
        assert!(found);
        assert_eq!(visitor.get_string_value(), Some(&"simple_value".to_string()));
        
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
        let mut visitor1 = TestVisitor::new();
        source.visit_field("number", &mut visitor1).unwrap();
        assert_eq!(visitor1.get_string_value(), Some(&"42".to_string()));
        
        let mut visitor2 = TestVisitor::new();
        source.visit_field("flag", &mut visitor2).unwrap();
        assert_eq!(visitor2.get_string_value(), Some(&"true".to_string()));
        
        let mut visitor3 = TestVisitor::new();
        source.visit_field("float", &mut visitor3).unwrap();
        assert_eq!(visitor3.get_string_value(), Some(&"3.14".to_string()));
        
        unsafe {
            env::remove_var("TEST_number");
            env::remove_var("TEST_flag");
            env::remove_var("TEST_float");
        }
    }
}