//! Builder for combining multiple configuration sources

use crate::{Config, Result, Source};

/// Builder that combines layers of sources to build configuration values
/// 
/// Sources are applied in order, with later sources overriding earlier ones.
/// The builder passes all sources to the Config implementation which visits
/// them to gather the values it needs.
pub struct Builder {
    sources: Vec<Box<dyn Source>>,
}

impl Builder {
    /// Create a new empty Builder
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }
    
    /// Add a source to the builder
    /// Sources are applied in the order they are added, with later sources
    /// overriding values from earlier sources.
    pub fn add_source<S: Source + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }
    
    /// Build a configuration value by having the Config visit all sources
    /// 
    /// The target configuration type specifies which values it needs,
    /// and visits sources to gather those specific values.
    pub fn build<T: Config>(&self) -> Result<T> {
        T::configure_from_sources(&self.sources)
    }
    
    /// Get the number of sources in the builder
    pub fn len(&self) -> usize {
        self.sources.len()
    }
    
    /// Check if the builder has no sources
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::EnvSource;
    use serde::{Deserialize, Serialize};
    use std::env;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct TestConfig {
        name: String,
        port: u16,
    }

    /// Mock source for testing
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
    fn test_builder_empty() {
        let builder = Builder::new();
        let config: TestConfig = builder.build().unwrap();
        assert_eq!(config, TestConfig::default());
    }

    #[test]
    fn test_builder_single_source() {
        let source = MockSource::new()
            .with_field("name", serde_json::Value::String("test".to_string()))
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(8080)));

        let config: TestConfig = Builder::new()
            .add_source(source)
            .build()
            .unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_builder_multiple_sources() {
        let source1 = MockSource::new()
            .with_field("name", serde_json::Value::String("base".to_string()))
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(3000)));

        let source2 = MockSource::new()
            .with_field("port", serde_json::Value::Number(serde_json::Number::from(8080)));

        let config: TestConfig = Builder::new()
            .add_source(source1)
            .add_source(source2)
            .build()
            .unwrap();

        assert_eq!(config.name, "base");  // from source1
        assert_eq!(config.port, 8080);   // overridden by source2
    }

    #[test]
    fn test_builder_with_env_source() {
        unsafe {
            env::set_var("TEST_name", "env_test");
            env::set_var("TEST_port", "9000");
        }

        let config: TestConfig = Builder::new()
            .add_source(EnvSource::with_prefix("TEST"))
            .build()
            .unwrap();
            
        assert_eq!(config.name, "env_test");
        assert_eq!(config.port, 9000);

        unsafe {
            env::remove_var("TEST_name");
            env::remove_var("TEST_port"); 
        }
    }
}