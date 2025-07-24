//! Config trait for types that can be configured from sources

use crate::{Error, Result, Source};
use serde::de::DeserializeOwned;
use std::str::FromStr;

/// Trait for types that can be configured from sources using a visitor pattern
///
/// Instead of loading entire sources into memory, configs can request specific
/// values from sources by key, enabling efficient configuration from large sources.
pub trait Config: Sized {
    /// Configure this type by visiting sources to collect required values
    /// 
    /// The implementation should call `visitor.visit(key)` for each configuration
    /// value it needs, where the visitor will query the sources in order.
    fn configure_with_visitor<V: ConfigVisitor>(visitor: V) -> Result<Self>;
}

/// Visitor trait for collecting configuration values from sources
pub trait ConfigVisitor {
    /// Visit a configuration key and get its value from the sources
    /// 
    /// Sources are checked in order, and the first source that contains
    /// the key provides the value.
    fn visit(&self, key: &str) -> Result<Option<String>>;
    
    /// Visit a configuration key with a default value
    fn visit_with_default(&self, key: &str, default: &str) -> Result<String> {
        self.visit(key).map(|opt| opt.unwrap_or_else(|| default.to_string()))
    }
}

/// Implementation of ConfigVisitor that queries multiple sources in order
pub struct SourceVisitor<'a> {
    sources: &'a [Box<dyn Source>],
}

impl<'a> SourceVisitor<'a> {
    pub fn new(sources: &'a [Box<dyn Source>]) -> Self {
        Self { sources }
    }
}

impl<'a> ConfigVisitor for SourceVisitor<'a> {
    fn visit(&self, key: &str) -> Result<Option<String>> {
        // Check sources in reverse order so later sources override earlier ones
        for source in self.sources.iter().rev() {
            if let Some(value) = source.get(key)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
}

/// Blanket implementation for types that implement serde::Deserialize
/// 
/// This implementation uses serde's derive macros to automatically discover
/// the field structure and query sources for the required values.
impl<T> Config for T
where
    T: DeserializeOwned + serde::Serialize + Default,
{
    fn configure_with_visitor<V: ConfigVisitor>(visitor: V) -> Result<Self> {
        // For serde types, we need to build a JSON value from the visitor
        // and then deserialize it. This requires discovering the structure.
        
        // Start with the default value to get the structure
        let default_instance = T::default();
        let default_json = serde_json::to_value(&default_instance)
            .map_err(|e| Error::Deserialize(e.to_string()))?;
            
        // Recursively visit all keys in the default structure
        let configured_json = visit_json_structure(&visitor, &default_json, "")?;
        
        // Deserialize the final JSON
        serde_json::from_value(configured_json)
            .map_err(|e| Error::Deserialize(e.to_string()))
    }
}

/// Helper function to recursively visit JSON structure and replace values from sources
fn visit_json_structure<V: ConfigVisitor>(
    visitor: &V,
    json: &serde_json::Value,
    key_prefix: &str,
) -> Result<serde_json::Value> {
    match json {
        serde_json::Value::Object(map) => {
            let mut result = serde_json::Map::new();
            
            for (key, value) in map {
                let full_key = if key_prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", key_prefix, key)
                };
                
                let new_value = visit_json_structure(visitor, value, &full_key)?;
                result.insert(key.clone(), new_value);
            }
            
            Ok(serde_json::Value::Object(result))
        }
        _ => {
            // For leaf values, try to get from sources
            if let Some(source_value) = visitor.visit(key_prefix)? {
                // Try to parse the source value as the same type as the default
                parse_source_value_as_json(&source_value, json)
            } else {
                // Use the default value
                Ok(json.clone())
            }
        }
    }
}

/// Helper function to parse a source value string as a JSON value of the expected type
fn parse_source_value_as_json(source_value: &str, expected_type: &serde_json::Value) -> Result<serde_json::Value> {
    match expected_type {
        serde_json::Value::String(_) => Ok(serde_json::Value::String(source_value.to_string())),
        serde_json::Value::Number(_) => {
            // Try to parse as number
            if let Ok(int_val) = source_value.parse::<i64>() {
                Ok(serde_json::Value::Number(serde_json::Number::from(int_val)))
            } else if let Ok(float_val) = source_value.parse::<f64>() {
                Ok(serde_json::Value::Number(
                    serde_json::Number::from_f64(float_val)
                        .ok_or_else(|| Error::Deserialize("Invalid float value".to_string()))?
                ))
            } else {
                Err(Error::Deserialize(format!("Cannot parse '{}' as number", source_value)))
            }
        }
        serde_json::Value::Bool(_) => {
            match source_value.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(serde_json::Value::Bool(true)),
                "false" | "0" | "no" | "off" => Ok(serde_json::Value::Bool(false)),
                _ => Err(Error::Deserialize(format!("Cannot parse '{}' as boolean", source_value))),
            }
        }
        serde_json::Value::Null => Ok(serde_json::Value::Null),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Try to parse as JSON
            serde_json::from_str(source_value)
                .map_err(|e| Error::Deserialize(format!("Cannot parse '{}' as JSON: {}", source_value, e)))
        }
    }
}

/// Helper trait for types that can be configured from strings
pub trait FromStrConfig: Sized {
    type Err;
    fn from_str_config(s: &str) -> std::result::Result<Self, Self::Err>;
}

/// Blanket implementation for types that implement FromStr
impl<T> FromStrConfig for T
where
    T: FromStr,
{
    type Err = T::Err;
    
    fn from_str_config(s: &str) -> std::result::Result<Self, Self::Err> {
        T::from_str(s)
    }
}

/// Convert a source value to a type that implements FromStr
pub fn value_from_str<T>(value: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    T::from_str(value).map_err(|e| Error::Source(Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct TestConfig {
        name: String,
        port: u16,
        enabled: bool,
    }

    struct MockVisitor {
        values: std::collections::HashMap<String, String>,
    }

    impl MockVisitor {
        fn new() -> Self {
            Self {
                values: std::collections::HashMap::new(),
            }
        }
        
        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.values.insert(key.to_string(), value.to_string());
            self
        }
    }

    impl ConfigVisitor for MockVisitor {
        fn visit(&self, key: &str) -> Result<Option<String>> {
            Ok(self.values.get(key).cloned())
        }
    }

    #[test]
    fn test_config_with_visitor() {
        let visitor = MockVisitor::new()
            .with_value("name", "test")
            .with_value("port", "8080")
            .with_value("enabled", "true");
        
        let config = TestConfig::configure_with_visitor(visitor).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_config_with_partial_visitor() {
        let visitor = MockVisitor::new()
            .with_value("name", "partial");
        
        let config = TestConfig::configure_with_visitor(visitor).unwrap();
        assert_eq!(config.name, "partial");
        assert_eq!(config.port, 0);  // default value
        assert_eq!(config.enabled, false);  // default value
    }

    #[test]
    fn test_value_from_str() {
        let result: i32 = value_from_str("42").unwrap();
        assert_eq!(result, 42);
        
        let result: bool = value_from_str("true").unwrap();
        assert_eq!(result, true);
    }
}