//! Builder for combining multiple configuration sources

use crate::{Config, Result, Source};

/// Builder that provides sources to Config implementations
/// 
/// The builder coordinates by providing each source to the Config in order.
/// Config implementations decide how to handle multiple sources - whether
/// to override, merge, or apply custom logic.
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
    /// Sources are provided to Config in the order they are added.
    /// Config implementations decide how to handle multiple sources.
    pub fn add_source<S: Source + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }
    
    /// Build a configuration value by providing each source to Config
    /// 
    /// The builder provides sources to Config in order, allowing Config
    /// to decide how to handle multiple sources.
    pub fn build<T: Config>(&self) -> Result<T> {
        if self.sources.is_empty() {
            // If no sources, try to create a default config from an empty source
            return T::configure_from_source(&EmptySource);
        }
        
        // Start with the first source
        let mut config = T::configure_from_source(&*self.sources[0])?;
        
        // Update with remaining sources
        for source in &self.sources[1..] {
            config.update_from_source(&**source)?;
        }
        
        Ok(config)
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

/// Empty source for default configurations
struct EmptySource;

impl Source for EmptySource {
    fn visit_field(&self, _field: &str, _visitor: &mut dyn crate::source::Visitor) -> Result<bool> {
        Ok(false)
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
            config.update_from_source(source)?;
            Ok(config)
        }
        
        fn update_from_source(&mut self, source: &dyn Source) -> Result<()> {
            if let Some(name) = crate::config::get_field_value::<String>(source, "name")? {
                self.name = name;
            }
            
            if let Some(port) = crate::config::get_field_value::<u16>(source, "port")? {
                self.port = port;
            }
            
            Ok(())
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