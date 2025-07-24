//! Example of implementing a custom Source for database-backed configuration
//! 
//! This demonstrates how to create truly scalable sources that can handle
//! arbitrarily large datasets without loading everything into memory.

use ucfg::{Source, Result, Builder};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Example database source that queries configuration from a simulated database
/// 
/// In a real implementation, this would use an actual database connection
/// and execute SQL queries to retrieve specific configuration values.
pub struct DatabaseSource {
    // In real code, this would be a database connection
    table_name: String,
    simulated_db: HashMap<String, String>,
}

impl DatabaseSource {
    pub fn new(table_name: &str) -> Self {
        // Simulate a database with some configuration data
        let mut db = HashMap::new();
        db.insert("app.name".to_string(), "Database App".to_string());
        db.insert("app.version".to_string(), "2.0.0".to_string());
        db.insert("server.host".to_string(), "db.example.com".to_string());
        db.insert("server.port".to_string(), "8080".to_string());
        db.insert("database.host".to_string(), "db.internal.com".to_string());
        db.insert("database.port".to_string(), "5432".to_string());
        db.insert("database.name".to_string(), "production".to_string());
        
        Self {
            table_name: table_name.to_string(),
            simulated_db: db,
        }
    }
}

impl Source for DatabaseSource {
    fn get(&self, key: &str) -> Result<Option<String>> {
        // In a real implementation, this would execute:
        // SELECT value FROM {table_name} WHERE config_key = ?
        
        println!("DatabaseSource: Querying key '{}' from table '{}'", key, self.table_name);
        
        // Simulate database query latency
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        Ok(self.simulated_db.get(key).cloned())
    }
    
    fn keys(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        // In a real implementation, this would execute:
        // SELECT config_key FROM {table_name}
        
        println!("DatabaseSource: Loading all keys from table '{}'", self.table_name);
        
        let keys: Vec<String> = self.simulated_db.keys().cloned().collect();
        Ok(Box::new(keys.into_iter()))
    }
}

/// Example API source that queries configuration from a REST API
pub struct ApiSource {
    base_url: String,
}

impl ApiSource {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

impl Source for ApiSource {
    fn get(&self, key: &str) -> Result<Option<String>> {
        // In a real implementation, this would make an HTTP request:
        // GET {base_url}/config/{key}
        
        println!("ApiSource: Fetching key '{}' from {}/config/{}", key, self.base_url, key);
        
        // Simulate some API responses
        match key {
            "feature.logging" => Ok(Some("true".to_string())),
            "feature.metrics" => Ok(Some("false".to_string())),
            "api.timeout" => Ok(Some("30".to_string())),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    app: AppInfo,
    server: ServerConfig,
    database: DatabaseConfig,
    feature: FeatureConfig,
    api: ApiConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct DatabaseConfig {
    host: String,
    port: u16,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct FeatureConfig {
    logging: bool,
    metrics: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ApiConfig {
    timeout: u32,
}

fn main() -> Result<()> {
    println!("=== Database Source Example ===");
    println!("This demonstrates how to implement truly scalable configuration sources");
    println!("that can handle arbitrarily large datasets without memory limitations.\n");
    
    // Create sources that represent large external systems
    let db_source = DatabaseSource::new("app_config");
    let api_source = ApiSource::new("https://config-api.example.com");
    
    // Build configuration from multiple large sources
    println!("Building configuration from database and API sources...\n");
    
    let config: AppConfig = Builder::new()
        .add_source(db_source)
        .add_source(api_source)
        .build()?;
    
    println!("Final configuration:");
    println!("{:#?}", config);
    
    println!("\n=== Key Benefits ===");
    println!("✓ Only queries needed configuration values");
    println!("✓ No memory limitations from source size");
    println!("✓ Sources can optimize query patterns");
    println!("✓ Supports lazy evaluation and caching");
    println!("✓ Can handle database, API, filesystem sources");
    
    Ok(())
}