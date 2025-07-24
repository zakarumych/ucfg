//! Config trait for types that can be configured from sources

use crate::{Error, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::str::FromStr;

/// Trait for types that can be configured from sources
pub trait Config: Sized {
    /// Create a new instance from a JSON Value
    fn from_value(value: Value) -> Result<Self>;
    
    /// Merge a JSON Value into this instance
    fn merge_value(&mut self, value: Value) -> Result<()>;
}

/// Blanket implementation for types that implement serde::Deserialize
impl<T> Config for T
where
    T: DeserializeOwned + Clone,
{
    fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value)
            .map_err(|e| Error::Deserialize(e.to_string()))
    }
    
    fn merge_value(&mut self, value: Value) -> Result<()> {
        // For serde types, we override rather than merge
        *self = Self::from_value(value)?;
        Ok(())
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

/// Convert a Value to a type that implements FromStr
pub fn value_to_from_str<T>(value: Value) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match value {
        Value::String(s) => {
            T::from_str(&s).map_err(|e| Error::Source(Box::new(e)))
        }
        _ => Err(Error::Deserialize(
            "Expected string value for FromStr conversion".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestConfig {
        name: String,
        port: u16,
    }

    #[test]
    fn test_config_from_value() {
        let value = serde_json::json!({
            "name": "test",
            "port": 8080
        });
        
        let config = TestConfig::from_value(value).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_config_merge_value() {
        let mut config = TestConfig {
            name: "old".to_string(),
            port: 3000,
        };
        
        let value = serde_json::json!({
            "name": "new",
            "port": 8080
        });
        
        config.merge_value(value).unwrap();
        assert_eq!(config.name, "new");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_value_to_from_str() {
        let value = Value::String("42".to_string());
        let num: i32 = value_to_from_str(value).unwrap();
        assert_eq!(num, 42);
    }
}