//! Config trait for types that can be configured from sources

use crate::{Result, Source};
use std::str::FromStr;

/// Trait for types that can be configured by visiting sources
/// 
/// Implementations should define how to extract and parse the specific
/// fields they need from the provided source.
pub trait Config: Sized {
    /// Configure this type by visiting a single source
    /// 
    /// The implementation should query the source for the fields it needs
    /// and parse them appropriately (typically using FromStr for environment
    /// variables or serde for structured data).
    fn configure_from_source(source: &dyn Source) -> Result<Self>;
}

/// Helper function for parsing field values using FromStr
pub fn parse_field_value<T>(value: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    T::from_str(value).map_err(|e| crate::Error::Parse(format!("Failed to parse '{}': {}", value, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Simple test struct for manual Config implementation
    #[derive(Debug, Clone, PartialEq)]
    struct SimpleConfig {
        name: String,
        port: u16,
        enabled: bool,
    }

    impl Default for SimpleConfig {
        fn default() -> Self {
            Self {
                name: "default".to_string(),
                port: 3000,
                enabled: false,
            }
        }
    }

    impl Config for SimpleConfig {
        fn configure_from_source(source: &dyn Source) -> Result<Self> {
            let mut config = Self::default();
            
            // Try to get name field
            if let Some(name_str) = source.get_field("name")? {
                config.name = name_str;
            }
            
            // Try to get port field and parse it
            if let Some(port_str) = source.get_field("port")? {
                config.port = parse_field_value(&port_str)?;
            }
            
            // Try to get enabled field and parse it
            if let Some(enabled_str) = source.get_field("enabled")? {
                config.enabled = parse_field_value(&enabled_str)?;
            }
            
            Ok(config)
        }
    }

    struct MockSource {
        data: HashMap<String, String>,
    }

    impl MockSource {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
        
        fn with_field(mut self, field: &str, value: &str) -> Self {
            self.data.insert(field.to_string(), value.to_string());
            self
        }
    }

    impl Source for MockSource {
        fn get_field(&self, field: &str) -> Result<Option<String>> {
            Ok(self.data.get(field).cloned())
        }
    }

    #[test]
    fn test_config_from_source() {
        let source = MockSource::new()
            .with_field("name", "test")
            .with_field("port", "8080")
            .with_field("enabled", "true");
        
        let config = SimpleConfig::configure_from_source(&source).unwrap();
        
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_config_partial_override() {
        let source = MockSource::new()
            .with_field("port", "9000");
        
        let config = SimpleConfig::configure_from_source(&source).unwrap();
        
        assert_eq!(config.name, "default");  // default value
        assert_eq!(config.port, 9000);       // from source
        assert_eq!(config.enabled, false);   // default value
    }

    #[test]
    fn test_parse_field_value() {
        let result: i32 = parse_field_value("42").unwrap();
        assert_eq!(result, 42);
        
        let result: bool = parse_field_value("true").unwrap();
        assert_eq!(result, true);
        
        // Test error case
        let result: std::result::Result<i32, _> = parse_field_value("not_a_number");
        assert!(result.is_err());
    }
}