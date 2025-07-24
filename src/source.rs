//! Source trait and implementations for loading configuration data

use crate::{Error, Result};
use std::env;

/// Trait for querying configuration data from various sources
/// 
/// Sources can be arbitrarily large (e.g., databases, file systems, remote APIs),
/// so they support querying specific keys rather than loading all data at once.
/// 
/// # Scalability Considerations
/// 
/// This trait is designed to support truly large sources by:
/// - Querying individual keys on-demand rather than loading entire datasets
/// - Supporting lazy evaluation and streaming where appropriate
/// - Allowing sources to optimize access patterns internally
/// 
/// For small to medium in-memory data, use `DeserializerSource`.
/// For large external sources (databases, APIs, file systems), implement this trait directly.
/// 
/// # Example: Database Source
/// 
/// ```rust,ignore
/// struct DatabaseSource {
///     connection: DatabaseConnection,
///     table: String,
/// }
/// 
/// impl Source for DatabaseSource {
///     fn get(&self, key: &str) -> Result<Option<String>> {
///         // Query only the specific key from the database
///         let query = format!("SELECT value FROM {} WHERE config_key = ?", self.table);
///         self.connection.query_row(&query, &[key])
///     }
/// }
/// ```
pub trait Source {
    /// Query a specific configuration value by key path
    /// 
    /// The key path uses dot notation for nested values (e.g., "database.host")
    /// Returns None if the key is not found in this source
    fn get(&self, key: &str) -> Result<Option<String>>;
    
    /// Check if this source contains a specific key
    fn contains_key(&self, key: &str) -> Result<bool> {
        self.get(key).map(|opt| opt.is_some())
    }
    
    /// Get all available keys from this source
    /// 
    /// For large sources, this may be expensive or impossible.
    /// Default implementation returns an empty iterator.
    fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        Ok(Box::new(std::iter::empty()))
    }
}

/// Source implementation for any serde-serializable data
/// 
/// This source is designed for small to medium-sized in-memory data structures
/// that can be efficiently serialized. For truly large sources (databases, APIs, etc.),
/// implement the `Source` trait directly for optimal performance.
/// 
/// # Scalability Note
/// 
/// While this implementation avoids caching the entire JSON representation,
/// it still serializes the data structure for each query. For very large data
/// or frequently queried configurations, consider implementing `Source` directly.
pub struct DeserializerSource<T> {
    data: T,
}

impl<T> DeserializerSource<T> {
    /// Create a new DeserializerSource
    pub fn new(data: T) -> Self {
        Self { data }
    }
    
    /// Query a specific key by serializing the data and navigating the structure
    /// This avoids caching the entire JSON representation, making it suitable for large sources
    fn query_key(&self, key: &str) -> Result<Option<serde_json::Value>>
    where
        T: serde::Serialize,
    {
        // Convert to JSON for this specific query
        let json = serde_json::to_value(&self.data)
            .map_err(|e| Error::Source(Box::new(e)))?;
        
        // Navigate the JSON using the dot notation key
        let mut current = &json;
        for part in key.split('.') {
            match current {
                serde_json::Value::Object(map) => {
                    if let Some(value) = map.get(part) {
                        current = value;
                    } else {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            }
        }
        
        Ok(Some(current.clone()))
    }
}

impl<T> Source for DeserializerSource<T>
where
    T: serde::Serialize + Clone,
{
    fn get(&self, key: &str) -> Result<Option<String>> {
        let value = self.query_key(key)?;
        
        if let Some(json_value) = value {
            // Convert the JSON value to a string
            let result = match json_value {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                // For objects and arrays, serialize them as JSON strings
                other => serde_json::to_string(&other)
                    .map_err(|e| Error::Source(Box::new(e)))?,
            };
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
    
    fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        // For keys(), we still need to serialize once, but this is only called
        // when specifically requesting all keys, not for normal operation
        let json = serde_json::to_value(&self.data)
            .map_err(|e| Error::Source(Box::new(e)))?;
        let keys = collect_json_keys(&json, "".to_string());
        Ok(Box::new(keys.into_iter()))
    }
}

/// Helper function to collect all keys from a JSON value using dot notation
fn collect_json_keys(value: &serde_json::Value, prefix: String) -> Vec<String> {
    let mut keys = Vec::new();
    
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                
                // Add the key itself
                keys.push(full_key.clone());
                
                // Recursively collect nested keys
                keys.extend(collect_json_keys(val, full_key));
            }
        }
        _ => {
            // For non-object values, just add the prefix if it's not empty
            if !prefix.is_empty() {
                keys.push(prefix);
            }
        }
    }
    
    keys
}

/// Source implementation for environment variables
pub struct EnvSource {
    prefix: Option<String>,
    separator: String,
}

impl EnvSource {
    /// Create a new EnvSource that reads all environment variables
    pub fn new() -> Self {
        Self {
            prefix: None,
            separator: "__".to_string(),
        }
    }

    /// Create a new EnvSource with a prefix filter
    pub fn with_prefix<S: Into<String>>(prefix: S) -> Self {
        Self {
            prefix: Some(prefix.into()),
            separator: "__".to_string(),
        }
    }

    /// Set the separator used for nested keys (default: "__")
    pub fn separator<S: Into<String>>(mut self, separator: S) -> Self {
        self.separator = separator.into();
        self
    }

    /// Convert a nested configuration key to an environment variable key
    fn nested_key_to_env_key(&self, nested_key: &str) -> String {
        let env_key = nested_key.replace('.', &self.separator);
        if let Some(ref prefix) = self.prefix {
            format!("{}_{}", prefix, env_key)
        } else {
            env_key
        }
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for EnvSource {
    fn get(&self, key: &str) -> Result<Option<String>> {
        let env_key = self.nested_key_to_env_key(key);
        Ok(env::var(&env_key).ok())
    }
    
    fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        let prefix_filter = self.prefix.clone();
        let separator = self.separator.clone();
        
        let keys: Vec<String> = env::vars()
            .filter_map(move |(key, _)| {
                let should_include = if let Some(ref prefix) = prefix_filter {
                    key.starts_with(&format!("{}_", prefix))
                } else {
                    true
                };
                
                if should_include {
                    // Convert env key to nested key using the instance method logic
                    let nested_key = if let Some(ref prefix) = prefix_filter {
                        key.strip_prefix(&format!("{}_", prefix)).unwrap_or(&key)
                    } else {
                        &key
                    };
                    
                    let result = nested_key.replace(&separator, ".");
                    Some(result)
                } else {
                    None
                }
            })
            .collect();
            
        Ok(Box::new(keys.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::collections::HashMap;

    // Example of a custom source for large/external data
    // This demonstrates how to implement Source for truly scalable scenarios
    struct MockLargeSource {
        // Simulate a large external source (e.g., database, API)
        data: HashMap<String, String>,
    }

    impl MockLargeSource {
        fn new() -> Self {
            let mut data = HashMap::new();
            data.insert("app.name".to_string(), "Large App".to_string());
            data.insert("app.version".to_string(), "1.0.0".to_string());
            data.insert("database.host".to_string(), "large.db.com".to_string());
            data.insert("database.port".to_string(), "5432".to_string());
            Self { data }
        }
    }

    impl Source for MockLargeSource {
        fn get(&self, key: &str) -> Result<Option<String>> {
            // In a real implementation, this would query an external source
            // without loading all data into memory
            Ok(self.data.get(key).cloned())
        }

        fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
            // In a real implementation, this might stream keys from the source
            let keys: Vec<String> = self.data.keys().cloned().collect();
            Ok(Box::new(keys.into_iter()))
        }
    }

    #[test]
    fn test_custom_large_source() {
        let source = MockLargeSource::new();
        
        assert_eq!(source.get("app.name").unwrap(), Some("Large App".to_string()));
        assert_eq!(source.get("database.host").unwrap(), Some("large.db.com".to_string()));
        assert_eq!(source.get("nonexistent").unwrap(), None);
        
        let keys: Vec<String> = source.keys().unwrap().collect();
        assert!(keys.contains(&"app.name".to_string()));
        assert!(keys.contains(&"database.host".to_string()));
    }

    #[test]
    fn test_deserializer_source() {
        use serde::{Deserialize, Serialize};
        
        #[derive(Serialize, Deserialize, Clone)]
        struct TestData {
            name: String,
            value: i32,
            nested: NestedData,
        }
        
        #[derive(Serialize, Deserialize, Clone)]
        struct NestedData {
            flag: bool,
        }
        
        let data = TestData {
            name: "test".to_string(),
            value: 42,
            nested: NestedData { flag: true },
        };
        
        let source = DeserializerSource::new(data);
        
        // Test simple key access
        assert_eq!(source.get("name").unwrap(), Some("test".to_string()));
        assert_eq!(source.get("value").unwrap(), Some("42".to_string()));
        
        // Test nested key access
        assert_eq!(source.get("nested.flag").unwrap(), Some("true".to_string()));
        
        // Test non-existent key
        assert_eq!(source.get("nonexistent").unwrap(), None);
        assert_eq!(source.get("nested.nonexistent").unwrap(), None);
        
        // Test key listing
        let keys: Vec<String> = source.keys().unwrap().collect();
        assert!(keys.contains(&"name".to_string()));
        assert!(keys.contains(&"value".to_string()));
        assert!(keys.contains(&"nested".to_string()));
        assert!(keys.contains(&"nested.flag".to_string()));
    }

    #[test]
    fn test_env_source_basic() {
        unsafe {
            env::set_var("TEST_KEY", "test_value");
        }
        
        let source = EnvSource::with_prefix("TEST");
        let result = source.get("KEY").unwrap();
        
        assert_eq!(result, Some("test_value".to_string()));
        
        // Test non-existent key
        assert_eq!(source.get("NONEXISTENT").unwrap(), None);
        
        unsafe {
            env::remove_var("TEST_KEY");
        }
    }

    #[test]
    fn test_env_source_nested() {
        unsafe {
            env::set_var("APP_DB__HOST", "localhost");
            env::set_var("APP_DB__PORT", "5432");
        }
        
        let source = EnvSource::with_prefix("APP");
        
        assert_eq!(source.get("DB.HOST").unwrap(), Some("localhost".to_string()));
        assert_eq!(source.get("DB.PORT").unwrap(), Some("5432".to_string()));
        
        // Test key listing
        let keys: Vec<String> = source.keys().unwrap().collect();
        assert!(keys.contains(&"DB.HOST".to_string()));
        assert!(keys.contains(&"DB.PORT".to_string()));
        
        unsafe {
            env::remove_var("APP_DB__HOST");
            env::remove_var("APP_DB__PORT");
        }
    }
    
    #[test]
    fn test_env_source_no_prefix() {
        unsafe {
            env::set_var("SIMPLE_KEY", "simple_value");
        }
        
        let source = EnvSource::new();
        
        assert_eq!(source.get("SIMPLE_KEY").unwrap(), Some("simple_value".to_string()));
        
        unsafe {
            env::remove_var("SIMPLE_KEY");
        }
    }
}