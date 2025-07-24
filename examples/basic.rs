use ucfg::{Builder, EnvSource};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    name: String,
    port: u16,
    enabled: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set some example environment variables
    unsafe {
        env::set_var("MYAPP_name", "Production App");
        env::set_var("MYAPP_port", "8080");
        env::set_var("MYAPP_enabled", "true");
    }

    // Build configuration from environment variables
    let config: AppConfig = Builder::new()
        .add_source(EnvSource::with_prefix("MYAPP"))
        .build()?;

    println!("Configuration loaded from environment:");
    println!("  Name: {}", config.name);       // "Production App"
    println!("  Port: {}", config.port);       // 8080
    println!("  Enabled: {}", config.enabled); // true

    // Cleanup
    unsafe {
        env::remove_var("MYAPP_name");
        env::remove_var("MYAPP_port");
        env::remove_var("MYAPP_enabled");
    }

    Ok(())
}