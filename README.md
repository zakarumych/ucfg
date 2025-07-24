# Ultimate Configuration Library

This library provides a comprehensive solution for configuring applications from various sources, layering configurations, supporting overloads, extensions, and more.

## Goal

To be one-stop for all configuration needs, regardless of the nature of the application or the complexity of the configuration.

## Features

- Using serde compatible types for leaf configurations.
- Supports sourcing configuration from:
  - Environment variables
  - Files (JSON, YAML, TOML) via serde deserializers
  - Custom sources via `Source` trait
- Supports multiple ways of updating configuration value from layered sources:
  - Recursive update (default)
  - Overriding (the only option for serde-deserializable types)
  - Extending (available for container types)

## Usage

```rust
use ucfg::{Builder, DeserializerSource, EnvSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    name: String,
    port: u16,
    database: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct DatabaseConfig {
    host: String,
    port: u16,
    username: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Base configuration
    let base_config = serde_json::json!({
        "name": "MyApp",
        "port": 8080,
        "database": {
            "host": "localhost",
            "port": 5432,
            "username": "dev"
        }
    });

    // Build configuration from multiple sources
    let config: AppConfig = Builder::new()
        .add_source(DeserializerSource::new(base_config))
        .add_source(EnvSource::with_prefix("APP"))  // APP_name, APP_database__host, etc.
        .build()?;

    println!("Final config: {:#?}", config);
    Ok(())
}
```

## Architecture

### Core Traits

- **`Source`**: Trait for loading configuration data from various sources
- **`Config`**: Trait for types that can be configured from sources (auto-implemented for serde types)

### Source Implementations

- **`DeserializerSource`**: Wraps any serde-serializable data
- **`EnvSource`**: Reads from environment variables at runtime with prefix filtering and nested key support

### Builder

- **`Builder`**: Manages layers of sources and applies them in order to build config values

Environment variables use `__` (double underscore) as the separator for nested keys by default:
- `APP_database__host` becomes `database.host`
- `APP_server__port` becomes `server.port`
