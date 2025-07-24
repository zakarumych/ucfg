use ucfg::{Builder, DeserializerSource, EnvSource};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    name: String,
    port: u16,
    database: DatabaseConfig,
    features: FeaturesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DatabaseConfig {
    host: String,
    port: u16,
    username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FeaturesConfig {
    logging: bool,
    metrics: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set some example environment variables
    unsafe {
        env::set_var("MYAPP_name", "Production App");
        env::set_var("MYAPP_database__host", "db.example.com");
        env::set_var("MYAPP_database__username", "produser");
    }

    // Base configuration (could be loaded from a file)
    let base_config = serde_json::json!({
        "name": "Development App",
        "port": 8080,
        "database": {
            "host": "localhost",
            "port": 5432,
            "username": "devuser"
        },
        "features": {
            "logging": true,
            "metrics": false
        }
    });

    // Production overrides
    let prod_overrides = serde_json::json!({
        "port": 443,
        "features": {
            "metrics": true
        }
    });

    // Build layered configuration
    let config: AppConfig = Builder::new()
        .add_source(DeserializerSource::new(base_config))    // Base config
        .add_source(EnvSource::with_prefix("MYAPP"))         // Environment overrides
        .add_source(DeserializerSource::new(prod_overrides)) // Final overrides
        .build()?;

    println!("Final configuration:");
    println!("  Name: {}", config.name);                    // "Production App" (from env)
    println!("  Port: {}", config.port);                    // 443 (from prod_overrides)
    println!("  Database Host: {}", config.database.host);  // "db.example.com" (from env)
    println!("  Database Port: {}", config.database.port);  // 5432 (from base)
    println!("  Database User: {}", config.database.username); // "produser" (from env)
    println!("  Logging: {}", config.features.logging);     // true (from base)
    println!("  Metrics: {}", config.features.metrics);     // true (from prod_overrides)

    // Cleanup
    unsafe {
        env::remove_var("MYAPP_name");
        env::remove_var("MYAPP_database__host");
        env::remove_var("MYAPP_database__username");
    }

    Ok(())
}