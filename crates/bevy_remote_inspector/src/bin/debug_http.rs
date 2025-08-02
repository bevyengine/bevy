//! Debug HTTP client for testing bevy_remote connection

use bevy_remote_inspector::http_client::{HttpRemoteClient, HttpRemoteConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing HTTP connection to bevy_remote...");
    
    let config = HttpRemoteConfig::default();
    let mut client = HttpRemoteClient::new(&config);
    
    println!("📡 Attempting to connect to {}:{}", config.host, config.port);
    
    match client.connect().await {
        Ok(_) => {
            println!("✅ Connected successfully!");
            
            println!("📋 Attempting to list entities...");
            match client.list_entities().await {
                Ok(entity_ids) => {
                    println!("📊 Found {} entities: {:?}", entity_ids.len(), entity_ids);
                    
                    if !entity_ids.is_empty() {
                        println!("📦 Getting component data for entities...");
                        match client.get_entities(&entity_ids).await {
                            Ok(entities) => {
                                println!("✅ Retrieved {} entities with component data:", entities.len());
                                for entity in &entities {
                                    println!("  - Entity {}: {} (components: {})", 
                                        entity.id, 
                                        entity.name.as_deref().unwrap_or("Unnamed"),
                                        entity.components.len()
                                    );
                                    for (comp_name, _comp_data) in &entity.components {
                                        println!("    * {}", comp_name);
                                    }
                                }
                            }
                            Err(e) => println!("❌ Failed to get entity data: {}", e),
                        }
                    }
                }
                Err(e) => println!("❌ Failed to list entities: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Connection failed: {}", e);
            println!("💡 Make sure the target app is running:");
            println!("   cargo run --example target_app");
        }
    }
    
    Ok(())
}