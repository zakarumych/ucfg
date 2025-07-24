//! Builder for combining multiple configuration sources

use crate::{Config, Result, Source};
use crate::config::SourceVisitor;

/// Builder that combines layers of sources to build configuration values
/// 
/// Sources are applied in order, with later sources overriding earlier ones.
/// The builder uses a visitor pattern to efficiently query only the values
/// needed by the target configuration type.
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
    
    /// Build a configuration value by using the visitor pattern
    /// 
    /// The target configuration type specifies which values it needs,
    /// and sources are queried only for those specific values.
    pub fn build<T: Config>(&self) -> Result<T> {
        let visitor = SourceVisitor::new(&self.sources);
        T::configure_with_visitor(visitor)
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
    use crate::source::{DeserializerSource, EnvSource};
    use serde::{Deserialize, Serialize};
    use std::env;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct TestConfig {
        name: String,
        port: u16,
        database: DatabaseConfig,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct DatabaseConfig {
        host: String,
        port: u16,
    }

    /// Mock source for testing
    struct MockSource {
        data: std::collections::HashMap<String, String>,
    }

    impl MockSource {
        fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }
        
        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.data.insert(key.to_string(), value.to_string());
            self
        }
    }

    impl Source for MockSource {
        fn get(&self, key: &str) -> Result<Option<String>> {
            Ok(self.data.get(key).cloned())
        }
        
        fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
            let keys: Vec<String> = self.data.keys().cloned().collect();
            Ok(Box::new(keys.into_iter()))
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
            .with_value("name", "test")
            .with_value("port", "8080")
            .with_value("database.host", "localhost")
            .with_value("database.port", "5432");

        let config: TestConfig = Builder::new()
            .add_source(source)
            .build()
            .unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
    }

    #[test]
    fn test_builder_multiple_sources() {
        let source1 = MockSource::new()
            .with_value("name", "base")
            .with_value("port", "3000")
            .with_value("database.host", "localhost")
            .with_value("database.port", "5432");

        let source2 = MockSource::new()
            .with_value("port", "8080")
            .with_value("database.host", "remote");

        let config: TestConfig = Builder::new()
            .add_source(source1)
            .add_source(source2)
            .build()
            .unwrap();

        assert_eq!(config.name, "base");  // from source1
        assert_eq!(config.port, 8080);   // overridden by source2
        assert_eq!(config.database.host, "remote");  // overridden by source2
        assert_eq!(config.database.port, 5432);      // from source1
    }

    #[test]
    fn test_builder_with_deserializer_source() {
        let data = serde_json::json!({
            "name": "from_json",
            "port": 9000,
            "database": {
                "host": "json_host",
                "port": 6000
            }
        });

        let config: TestConfig = Builder::new()
            .add_source(DeserializerSource::new(data))
            .build()
            .unwrap();

        assert_eq!(config.name, "from_json");
        assert_eq!(config.port, 9000);
        assert_eq!(config.database.host, "json_host");
        assert_eq!(config.database.port, 6000);
    }

    #[test]
    fn test_builder_with_env_source() {
        unsafe {
            env::set_var("TEST_name", "env_test");
            env::set_var("TEST_port", "9000");
            env::set_var("TEST_database__host", "localhost");
            env::set_var("TEST_database__port", "5432");
        }

        let config: TestConfig = Builder::new()
            .add_source(EnvSource::with_prefix("TEST"))
            .build()
            .unwrap();
            
        assert_eq!(config.name, "env_test");
        assert_eq!(config.port, 9000);
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);

        unsafe {
            env::remove_var("TEST_name");
            env::remove_var("TEST_port"); 
            env::remove_var("TEST_database__host");
            env::remove_var("TEST_database__port");
        }
    }
}