//! Config trait for types that can be configured from sources

use crate::{Result, Source, source::Visitor};
use std::str::FromStr;

/// Trait for types that can be configured by visiting sources
/// 
/// Implementations should define how to extract and parse the specific
/// fields they need from the provided source and how to update themselves
/// when multiple sources are provided.
pub trait Config: Sized {
    /// Create a new instance configured from a single source
    fn configure_from_source(source: &dyn Source) -> Result<Self>;
    
    /// Update this instance with values from another source
    /// 
    /// This method allows Config implementations to decide how to handle
    /// multiple sources - whether to override, merge, or apply custom logic.
    fn update_from_source(&mut self, source: &dyn Source) -> Result<()>;
}

/// Helper function for parsing field values using FromStr
pub fn parse_field_value<T>(value: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    T::from_str(value).map_err(|e| crate::Error::Parse(format!("Failed to parse '{}': {}", value, e)))
}

/// A visitor that collects field values for parsing
pub struct FieldVisitor {
    pub string_value: Option<String>,
    pub json_value: Option<serde_json::Value>,
}

impl FieldVisitor {
    pub fn new() -> Self {
        Self {
            string_value: None,
            json_value: None,
        }
    }
    
    /// Parse the collected value using FromStr (for string values) or serde (for JSON values)
    pub fn parse_value<T>(&self) -> Result<Option<T>>
    where
        T: FromStr + serde::de::DeserializeOwned,
        T::Err: std::fmt::Display,
    {
        if let Some(ref s) = self.string_value {
            let parsed = parse_field_value(s)?;
            Ok(Some(parsed))
        } else if let Some(ref json) = self.json_value {
            // Try to deserialize from JSON
            let parsed = serde_json::from_value(json.clone())
                .map_err(|e| crate::Error::Parse(format!("Failed to parse JSON: {}", e)))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }
}

impl Visitor for FieldVisitor {
    fn visit_string(&mut self, value: String) -> Result<()> {
        self.string_value = Some(value);
        Ok(())
    }
    
    fn visit_json(&mut self, value: serde_json::Value) -> Result<()> {
        self.json_value = Some(value);
        Ok(())
    }
}

/// Helper function to get a field value from a source with parsing
pub fn get_field_value<T>(source: &dyn Source, field: &str) -> Result<Option<T>>
where
    T: FromStr + serde::de::DeserializeOwned,
    T::Err: std::fmt::Display,
{
    let mut visitor = FieldVisitor::new();
    let found = source.visit_field(field, &mut visitor)?;
    
    if found {
        visitor.parse_value()
    } else {
        Ok(None)
    }
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
            config.update_from_source(source)?;
            Ok(config)
        }
        
        fn update_from_source(&mut self, source: &dyn Source) -> Result<()> {
            // Try to get name field
            if let Some(name) = get_field_value::<String>(source, "name")? {
                self.name = name;
            }
            
            // Try to get port field and parse it
            if let Some(port) = get_field_value::<u16>(source, "port")? {
                self.port = port;
            }
            
            // Try to get enabled field and parse it
            if let Some(enabled) = get_field_value::<bool>(source, "enabled")? {
                self.enabled = enabled;
            }
            
            Ok(())
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
        fn visit_field(&self, field: &str, visitor: &mut dyn crate::source::Visitor) -> Result<bool> {
            if let Some(value) = self.data.get(field) {
                visitor.visit_string(value.clone())?;
                Ok(true)
            } else {
                Ok(false)
            }
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