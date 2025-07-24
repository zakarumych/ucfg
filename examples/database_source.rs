//! Example of implementing a custom Source for database-backed configuration
//! 
//! This demonstrates how to create truly scalable sources that can handle
//! arbitrarily large datasets without loading everything into memory.

use ucfg::{Source, Result, Builder, Config, parse_field_value};
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
        db.insert("name".to_string(), "Database App".to_string());
        db.insert("port".to_string(), "8080".to_string());
        db.insert("enabled".to_string(), "true".to_string());
        
        Self {
            table_name: table_name.to_string(),
            simulated_db: db,
        }
    }
}

impl Source for DatabaseSource {
    fn get_field(&self, field: &str) -> Result<Option<String>> {
        // In a real implementation, this would execute:
        // SELECT value FROM {table_name} WHERE field_name = ?
        
        println!("DatabaseSource: Querying field '{}' from table '{}'", field, self.table_name);
        
        // Simulate database query latency
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        Ok(self.simulated_db.get(field).cloned())
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
    fn get_field(&self, field: &str) -> Result<Option<String>> {
        // In a real implementation, this would make an HTTP request:
        // GET {base_url}/config/{field}
        
        println!("ApiSource: Fetching field '{}' from {}/config/{}", field, self.base_url, field);
        
        // Simulate some API responses
        match field {
            "logging" => Ok(Some("true".to_string())),
            "timeout" => Ok(Some("30".to_string())),
            _ => Ok(None),
        }
    }
}

#[derive(Debug)]
struct AppConfig {
    name: String,
    port: u16,
    enabled: bool,
    logging: bool,
    timeout: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "DefaultApp".to_string(),
            port: 3000,
            enabled: false,
            logging: false,
            timeout: 10,
        }
    }
}

impl Config for AppConfig {
    fn configure_from_source(source: &dyn Source) -> Result<Self> {
        let mut config = Self::default();
        
        if let Some(name) = source.get_field("name")? {
            config.name = name;
        }
        
        if let Some(port_str) = source.get_field("port")? {
            config.port = parse_field_value(&port_str)?;
        }
        
        if let Some(enabled_str) = source.get_field("enabled")? {
            config.enabled = parse_field_value(&enabled_str)?;
        }
        
        if let Some(logging_str) = source.get_field("logging")? {
            config.logging = parse_field_value(&logging_str)?;
        }
        
        if let Some(timeout_str) = source.get_field("timeout")? {
            config.timeout = parse_field_value(&timeout_str)?;
        }
        
        Ok(config)
    }
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
    println!("✓ Only queries needed configuration fields");
    println!("✓ No memory limitations from source size");
    println!("✓ Sources can optimize query patterns");
    println!("✓ Supports lazy evaluation and caching");
    println!("✓ Can handle database, API, filesystem sources");
    
    Ok(())
}