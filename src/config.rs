//! Config trait for types that can be configured from sources

use crate::{Error, Result, Source};
use serde::de::DeserializeOwned;
use std::str::FromStr;

/// Trait for types that can be configured by visiting sources
/// 
/// The Config implementation visits sources to gather the values it needs
/// and assembles them into the final configuration.
pub trait Config: Sized {
    /// Configure this type by visiting the provided sources
    /// 
    /// Sources are checked in order, with later sources overriding earlier ones.
    /// The implementation should query only the fields it needs.
    fn configure_from_sources(sources: &[Box<dyn Source>]) -> Result<Self>;
}

/// Blanket implementation for types that implement serde::Deserialize and Default
impl<T> Config for T
where
    T: DeserializeOwned + serde::Serialize + Default,
{
    fn configure_from_sources(sources: &[Box<dyn Source>]) -> Result<Self> {
        // Start with the default instance to discover the structure
        let default_instance = T::default();
        let default_json = serde_json::to_value(&default_instance)
            .map_err(|e| Error::Deserialize(e.to_string()))?;
            
        // Visit sources to build the final configuration
        let configured_json = visit_sources_for_json(sources, &default_json)?;
        
        // Deserialize the final JSON
        serde_json::from_value(configured_json)
            .map_err(|e| Error::Deserialize(e.to_string()))
    }
}

/// Helper function to visit sources and build JSON configuration
fn visit_sources_for_json(
    sources: &[Box<dyn Source>],
    default_json: &serde_json::Value,
) -> Result<serde_json::Value> {
    match default_json {
        serde_json::Value::Object(map) => {
            let mut result = serde_json::Map::new();
            
            for (field, default_value) in map {
                // Visit sources in reverse order so later sources override earlier ones
                let mut final_value = default_value.clone();
                
                for source in sources.iter().rev() {
                    if let Some(source_value) = source.get_field(field)? {
                        final_value = source_value;
                        break; // Use the first source that has this field
                    }
                }
                
                result.insert(field.clone(), final_value);
            }
            
            Ok(serde_json::Value::Object(result))
        }
        _ => {
            // For non-object values, just return the default
            Ok(default_json.clone())
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

    struct MockSource {
        data: std::collections::HashMap<String, serde_json::Value>,
    }

    impl MockSource {
        fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }
        
        fn with_field(mut self, field: &str, value: serde_json::Value) -> Self {
            self.data.insert(field.to_string(), value);
            self
        }
    }

    impl Source for MockSource {
        fn get_field(&self, field: &str) -> Result<Option<serde_json::Value>> {
            Ok(self.data.get(field).cloned())
        }
    }

    #[test]
    fn test_config_from_sources() {
        let source1 = MockSource::new()
            .with_field("name", serde_json::Value::String("test".to_string()))
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(8080)));
        
        let source2 = MockSource::new()
            .with_field("enabled", serde_json::Value::Bool(true));
        
        let sources: Vec<Box<dyn Source>> = vec![Box::new(source1), Box::new(source2)];
        let config = TestConfig::configure_from_sources(&sources).unwrap();
        
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_config_with_overrides() {
        let source1 = MockSource::new()
            .with_field("name", serde_json::Value::String("base".to_string()))
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(3000)));
        
        let source2 = MockSource::new()
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(8080)));
        
        let sources: Vec<Box<dyn Source>> = vec![Box::new(source1), Box::new(source2)];
        let config = TestConfig::configure_from_sources(&sources).unwrap();
        
        assert_eq!(config.name, "base");     // from source1
        assert_eq!(config.port, 8080);      // overridden by source2
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