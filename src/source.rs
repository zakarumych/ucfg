//! Source trait and implementations for loading configuration data

use crate::{Error, Result};
use serde::de::Deserializer;
use serde_json::Value;

use std::env;

/// Trait for loading configuration data from various sources
pub trait Source {
    /// Load configuration data as a JSON Value
    fn load(&self) -> Result<Value>;
}

/// Source implementation for any serde Deserializer
pub struct DeserializerSource<D> {
    deserializer: D,
}

impl<D> DeserializerSource<D> {
    /// Create a new DeserializerSource
    pub fn new(deserializer: D) -> Self {
        Self { deserializer }
    }
}

impl<D> Source for DeserializerSource<D>
where
    D: Deserializer<'static>,
{
    fn load(&self) -> Result<Value> {
        // This is tricky because we can't deserialize into Value directly from any Deserializer
        // without consuming it. For a real implementation, we'd need to handle this differently.
        // For now, this is a placeholder that would need to be redesigned.
        Err(Error::Source(
            "DeserializerSource requires rework to handle arbitrary deserializers".into(),
        ))
    }
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

    fn env_key_to_nested_key(&self, env_key: &str) -> String {
        let key = if let Some(ref prefix) = self.prefix {
            env_key.strip_prefix(&format!("{}_", prefix)).unwrap_or(env_key)
        } else {
            env_key
        };
        
        key.replace(&self.separator, ".")
    }

    fn build_nested_value(&self, key: &str, value: String) -> Value {
        let parts: Vec<&str> = key.split('.').collect();
        
        // Always build a nested structure, even for single keys
        let mut result = serde_json::Map::new();
        let mut current_map = &mut result;
        
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                current_map.insert(part.to_string(), Value::String(value.clone()));
            } else {
                current_map.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                // Get a mutable reference to the newly inserted map
                match current_map.get_mut(*part).unwrap() {
                    Value::Object(map) => current_map = map,
                    _ => unreachable!(),
                }
            }
        }
        
        Value::Object(result)
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for EnvSource {
    fn load(&self) -> Result<Value> {
        let mut result = serde_json::Map::new();
        
        for (key, value) in env::vars() {
            let should_include = if let Some(ref prefix) = self.prefix {
                key.starts_with(&format!("{}_", prefix))
            } else {
                true
            };
            
            if should_include {
                let nested_key = self.env_key_to_nested_key(&key);
                let nested_value = self.build_nested_value(&nested_key, value);
                
                // Merge this value into the result
                let mut result_value = Value::Object(result.clone());
                merge_values(&mut result_value, nested_value)?;
                if let Value::Object(map) = result_value {
                    result = map;
                }
            }
        }
        
        Ok(Value::Object(result))
    }
}

/// Helper function to merge JSON values
fn merge_values(target: &mut Value, source: Value) -> Result<()> {
    match (&mut *target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, value) in source_map {
                if let Some(existing) = target_map.get_mut(&key) {
                    merge_values(existing, value)?;
                } else {
                    target_map.insert(key, value);
                }
            }
        }
        (target_ref, source) => {
            *target_ref = source;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_source_basic() {
        unsafe {
            env::set_var("TEST_KEY", "test_value");
        }
        
        let source = EnvSource::with_prefix("TEST");
        let result = source.load().unwrap();
        
        assert_eq!(result["KEY"], Value::String("test_value".to_string()));
        
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
        let result = source.load().unwrap();
        
        assert_eq!(result["DB"]["HOST"], Value::String("localhost".to_string()));
        assert_eq!(result["DB"]["PORT"], Value::String("5432".to_string()));
        
        unsafe {
            env::remove_var("APP_DB__HOST");
            env::remove_var("APP_DB__PORT");
        }
    }
}