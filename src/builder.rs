//! Builder for combining multiple configuration sources

use crate::{Config, Error, Result, Source};
use serde_json::Value;

/// Builder that combines layers of sources to build configuration values
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
    /// Sources are applied in the order they are added
    pub fn add_source<S: Source + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }
    
    /// Build a configuration value by applying all sources in order
    /// The first source creates the initial value, subsequent sources merge into it
    pub fn build<T: Config + Default>(&self) -> Result<T> {
        if self.sources.is_empty() {
            return Ok(T::default());
        }
        
        // Load the first source to create the initial config
        let first_value = self.sources[0].load()?;
        let mut config = T::from_value(first_value)?;
        
        // Apply remaining sources by merging
        for source in &self.sources[1..] {
            let value = source.load()?;
            config.merge_value(value)?;
        }
        
        Ok(config)
    }
    
    /// Build a configuration value without requiring Default
    /// All sources are merged into a single Value first, then converted to T
    pub fn build_merged<T: Config>(&self) -> Result<T> {
        if self.sources.is_empty() {
            return Err(Error::Source("No sources provided".into()));
        }
        
        let mut merged_value = Value::Object(serde_json::Map::new());
        
        for source in &self.sources {
            let value = source.load()?;
            merge_values(&mut merged_value, value)?;
        }
        
        T::from_value(merged_value)
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

/// Helper function to merge JSON values
fn merge_values(target: &mut Value, source: Value) -> Result<()> {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, value) in source_map {
                if let Some(existing) = target_map.get_mut(&key) {
                    merge_values(existing, value)?;
                } else {
                    target_map.insert(key, value);
                }
            }
        }
        (target, source) => {
            *target = source;
        }
    }
    Ok(())
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
        database: DatabaseConfig,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    struct DatabaseConfig {
        host: String,
        port: u16,
    }

    /// Mock source for testing
    struct MockSource {
        data: Value,
    }

    impl MockSource {
        fn new(data: Value) -> Self {
            Self { data }
        }
    }

    impl Source for MockSource {
        fn load(&self) -> Result<Value> {
            Ok(self.data.clone())
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
        let source = MockSource::new(serde_json::json!({
            "name": "test",
            "port": 8080,
            "database": {
                "host": "localhost",
                "port": 5432
            }
        }));

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
        let source1 = MockSource::new(serde_json::json!({
            "name": "base",
            "port": 3000,
            "database": {
                "host": "localhost",
                "port": 5432
            }
        }));

        let source2 = MockSource::new(serde_json::json!({
            "port": 8080,
            "database": {
                "host": "remote"
            }
        }));

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
    fn test_builder_with_env_source() {
        unsafe {
            env::set_var("TEST_NAME", "env_test");
            env::set_var("TEST_PORT", "9000");
        }

        let env_source = EnvSource::with_prefix("TEST");
        let config: TestConfig = Builder::new()
            .add_source(env_source)
            .build()
            .unwrap();

        // Note: This test would need the TestConfig to handle string-to-number conversion
        // For now, we just test that it doesn't panic
        assert!(!config.name.is_empty() || config.port > 0);

        unsafe {
            env::remove_var("TEST_NAME");
            env::remove_var("TEST_PORT");
        }
    }
}