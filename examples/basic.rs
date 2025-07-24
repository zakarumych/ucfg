use ucfg::{Builder, EnvSource, Config, parse_field_value};
use std::env;

#[derive(Debug, Clone)]
struct AppConfig {
    name: String,
    port: u16,
    enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "DefaultApp".to_string(),
            port: 3000,
            enabled: false,
        }
    }
}

impl Config for AppConfig {
    fn configure_from_source(source: &dyn ucfg::Source) -> ucfg::Result<Self> {
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
        
        Ok(config)
    }
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