//! Source trait and implementations for loading configuration data

use crate::{Error, Result};
use std::env;

/// Trait for querying configuration data from various sources
/// 
/// Sources can be arbitrarily large (e.g., databases, file systems, remote APIs),
/// so they support querying specific keys rather than loading all data at once.
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
/// This converts the data to JSON internally and supports querying specific keys
pub struct DeserializerSource<T> {
    data: T,
    // Cache the JSON representation for efficient key lookups
    json_cache: std::sync::Mutex<Option<serde_json::Value>>,
}

impl<T> DeserializerSource<T> {
    /// Create a new DeserializerSource
    pub fn new(data: T) -> Self {
        Self { 
            data,
            json_cache: std::sync::Mutex::new(None),
        }
    }
    
    /// Get the JSON representation, caching it on first access
    fn get_json(&self) -> Result<serde_json::Value>
    where
        T: serde::Serialize,
    {
        let mut cache = self.json_cache.lock().unwrap();
        if let Some(ref cached) = *cache {
            Ok(cached.clone())
        } else {
            let json = serde_json::to_value(&self.data)
                .map_err(|e| Error::Source(Box::new(e)))?;
            *cache = Some(json.clone());
            Ok(json)
        }
    }
}

impl<T> Source for DeserializerSource<T>
where
    T: serde::Serialize + Clone,
{
    fn get(&self, key: &str) -> Result<Option<String>> {
        let json = self.get_json()?;
        
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
        
        // Convert the final value to a string
        let result = match current {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            // For objects and arrays, serialize them as JSON strings
            other => serde_json::to_string(other)
                .map_err(|e| Error::Source(Box::new(e)))?,
        };
        
        Ok(Some(result))
    }
    
    fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        let json = self.get_json()?;
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