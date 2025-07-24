//! Builder for combining multiple configuration sources

use crate::{Config, Result, Source};

/// Builder that combines layers of sources to build configuration values
/// 
/// The builder coordinates between Config implementations and Sources,
/// allowing Config to visit sources in the order they were added.
/// Later sources override earlier ones when they provide values for the same fields.
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
    
    /// Build a configuration value by coordinating Config and Sources
    /// 
    /// The builder creates a combined source that checks sources in reverse order
    /// (latest first) so that later sources override earlier ones.
    pub fn build<T: Config>(&self) -> Result<T> {
        let combined_source = CombinedSource::new(&self.sources);
        T::configure_from_source(&combined_source)
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

/// Internal combined source that checks sources in override order
struct CombinedSource<'a> {
    sources: &'a [Box<dyn Source>],
}

impl<'a> CombinedSource<'a> {
    fn new(sources: &'a [Box<dyn Source>]) -> Self {
        Self { sources }
    }
}

impl<'a> Source for CombinedSource<'a> {
    fn get_field(&self, field: &str) -> Result<Option<String>> {
        // Check sources in reverse order so later sources override earlier ones
        for source in self.sources.iter().rev() {
            if let Some(value) = source.get_field(field)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::EnvSource;
    use std::collections::HashMap;
    use std::env;

    #[derive(Debug, Clone, PartialEq)]
    struct TestConfig {
        name: String,
        port: u16,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                name: "default".to_string(),
                port: 3000,
            }
        }
    }

    impl Config for TestConfig {
        fn configure_from_source(source: &dyn Source) -> Result<Self> {
            let mut config = Self::default();
            
            if let Some(name) = source.get_field("name")? {
                config.name = name;
            }
            
            if let Some(port_str) = source.get_field("port")? {
                config.port = crate::config::parse_field_value(&port_str)?;
            }
            
            Ok(config)
        }
    }

    /// Mock source for testing
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
    fn test_builder_empty() {
        let builder = Builder::new();
        let config: TestConfig = builder.build().unwrap();
        assert_eq!(config, TestConfig::default());
    }

    #[test]
    fn test_builder_single_source() {
        let source = MockSource::new()
            .with_field("name", "test")
            .with_field("port", "8080");

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
            .with_field("name", "base")
            .with_field("port", "3000");

        let source2 = MockSource::new()
            .with_field("port", "8080");

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