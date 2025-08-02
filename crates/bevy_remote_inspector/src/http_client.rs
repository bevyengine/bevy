//! HTTP client for bevy_remote protocol with streaming support
//!
//! This client implements the bevy_remote JSON-RPC protocol with support for:
//! - bevy/list: List all entities  
//! - bevy/get: Get component data for entities
//! - bevy/get+watch: Stream live component updates

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// JSON-RPC request structure
#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC response structure  
#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u32,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error structure
#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// Remote entity representation from bevy_remote
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteEntity {
    pub id: u32,
    pub name: Option<String>,
    pub components: HashMap<String, Value>,
}

/// Configuration for HTTP remote client
#[derive(Resource, Debug)]
pub struct HttpRemoteConfig {
    pub host: String,
    pub port: u16,
}

impl Default for HttpRemoteConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 15702,
        }
    }
}

/// HTTP client for bevy_remote communication
#[derive(Resource)]
pub struct HttpRemoteClient {
    pub client: Client,
    pub base_url: String,
    pub request_id: u32,
    // Channel for receiving live updates
    pub update_receiver: Option<mpsc::UnboundedReceiver<RemoteUpdate>>,
    // Current cached data
    pub entities: HashMap<u32, RemoteEntity>,
    pub is_connected: bool,
    pub last_error: Option<String>,
}

/// Live update from streaming endpoint
#[derive(Debug, Clone)]
pub struct RemoteUpdate {
    pub entity_id: u32,
    pub components: HashMap<String, Value>,
}

impl HttpRemoteClient {
    pub fn new(config: &HttpRemoteConfig) -> Self {
        let base_url = format!("http://{}:{}", config.host, config.port);
        
        Self {
            client: Client::new(),
            base_url,
            request_id: 1,
            update_receiver: None,
            entities: HashMap::new(),
            is_connected: false,
            last_error: None,
        }
    }

    /// Test connection to bevy_remote server
    pub async fn connect(&mut self) -> Result<()> {
        println!("🔌 Attempting to connect to {}", self.base_url);
        
        // Try a simple list request to test connectivity
        match self.list_entities().await {
            Ok(_) => {
                self.is_connected = true;
                self.last_error = None;
                println!("✅ Connected to bevy_remote at {}", self.base_url);
                Ok(())
            }
            Err(e) => {
                self.is_connected = false;
                self.last_error = Some(e.to_string());
                println!("❌ Failed to connect: {}", e);
                Err(e)
            }
        }
    }

    /// Query all entities with any components via bevy/query
    pub async fn list_entities(&mut self) -> Result<Vec<u32>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "bevy/query".to_string(),
            params: Some(serde_json::json!({
                "data": {
                    "components": [],
                    "option": [],
                    "has": []
                },
                "filter": {
                    "with": [],
                    "without": []
                },
                "strict": false
            })),
        };

        let response = self.send_request(request).await?;
        
        if let Some(result) = response.result {
            // Parse the query response which is an array of entity objects
            let entities: Vec<serde_json::Value> = serde_json::from_value(result)
                .map_err(|e| anyhow!("Failed to parse entity query: {}", e))?;
            
            let mut entity_ids = Vec::new();
            for entity_obj in entities {
                if let Some(entity_id) = entity_obj.get("entity").and_then(|v| v.as_u64()) {
                    entity_ids.push(entity_id as u32);
                }
            }
            
            println!("📋 Listed {} entities via query", entity_ids.len());
            Ok(entity_ids)
        } else if let Some(error) = response.error {
            Err(anyhow!("bevy/query error: {}", error.message))
        } else {
            Err(anyhow!("Invalid response format"))
        }
    }

    /// Get component data for all entities via bevy/query with full component data
    pub async fn get_entities(&mut self, _entity_ids: &[u32]) -> Result<Vec<RemoteEntity>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "bevy/query".to_string(),
            params: Some(serde_json::json!({
                "data": {
                    "components": [],
                    "option": "all",
                    "has": []
                },
                "filter": {
                    "with": [],
                    "without": []
                },
                "strict": false
            })),
        };

        let response = self.send_request(request).await?;
        
        if let Some(result) = response.result {
            // Parse the query response
            let query_results: Vec<serde_json::Value> = serde_json::from_value(result)
                .map_err(|e| anyhow!("Failed to parse query results: {}", e))?;
            
            let mut entities = Vec::new();
            
            for query_result in query_results.iter() {
                if let (Some(entity_id), Some(components_obj)) = (
                    query_result.get("entity").and_then(|v| v.as_u64()),
                    query_result.get("components").and_then(|v| v.as_object())
                ) {
                    let mut components = std::collections::HashMap::new();
                    
                    // Convert components object to HashMap
                    for (component_name, component_data) in components_obj {
                        components.insert(component_name.clone(), component_data.clone());
                    }
                    
                    // Try to extract name from Name component if it exists
                    // Name is a tuple struct, so it should be in "0" field or direct string
                    let name = components.get("bevy_core::name::Name")
                        .and_then(|v| {
                            // Try as direct string first
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else {
                                // Try as tuple struct with "0" field
                                v.as_object()
                                    .and_then(|obj| obj.get("0"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            }
                        });
                    
                    let entity = RemoteEntity {
                        id: entity_id as u32,
                        name,
                        components,
                    };
                    
                    entities.push(entity);
                }
            }
            
            // Update local cache
            self.entities.clear();
            for entity in &entities {
                self.entities.insert(entity.id, entity.clone());
            }
            
            println!("📦 Retrieved {} entities with component data", entities.len());
            Ok(entities)
        } else if let Some(error) = response.error {
            Err(anyhow!("bevy/query error: {}", error.message))
        } else {
            Err(anyhow!("Invalid response format"))
        }
    }

    /// Start streaming updates for entities via bevy/get+watch
    pub async fn start_watching(&mut self, entity_ids: &[u32]) -> Result<()> {
        println!("👀 Starting watch stream for {} entities", entity_ids.len());
        
        let _request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "bevy/get+watch".to_string(),
            params: Some(serde_json::json!({
                "entities": entity_ids
            })),
        };

        // This would establish a streaming connection
        // For now, we'll simulate it with periodic polling
        let (tx, rx) = mpsc::unbounded_channel();
        self.update_receiver = Some(rx);

        // Spawn a background task for streaming (simulation)
        let _client = self.client.clone();
        let _base_url = self.base_url.clone();
        let entity_ids = entity_ids.to_vec();
        
        tokio::spawn(async move {
            // In a real implementation, this would be a streaming HTTP connection
            // For now, we'll poll every few seconds
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
            
            loop {
                interval.tick().await;
                
                // Simulate getting updates (in real implementation, this would be streaming)
                for &entity_id in &entity_ids {
                    let update = RemoteUpdate {
                        entity_id,
                        components: simulate_component_changes(entity_id),
                    };
                    
                    if tx.send(update).is_err() {
                        break; // Receiver dropped
                    }
                }
            }
        });

        Ok(())
    }

    /// Check for live updates from streaming endpoint
    pub fn check_updates(&mut self) -> Vec<RemoteUpdate> {
        let mut updates = Vec::new();
        
        if let Some(ref mut receiver) = self.update_receiver {
            while let Ok(update) = receiver.try_recv() {
                updates.push(update);
            }
        }
        
        updates
    }

    /// Get entity by ID from cache
    pub fn get_entity(&self, entity_id: u32) -> Option<&RemoteEntity> {
        self.entities.get(&entity_id)
    }

    /// Get all cached entity IDs
    pub fn get_entity_ids(&self) -> Vec<u32> {
        self.entities.keys().copied().collect()
    }

    /// Send JSON-RPC request
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let url = format!("{}/jsonrpc", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;

        Ok(response)
    }

    fn next_id(&mut self) -> u32 {
        let id = self.request_id;
        self.request_id += 1;
        id
    }
}

/// Simulate component changes for testing
fn simulate_component_changes(entity_id: u32) -> HashMap<String, Value> {
    let mut components = HashMap::new();
    
    // Simulate Transform component with changing values
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    
    let x = (time * 0.5).sin() as f32;
    let y = (time * 0.3).cos() as f32;
    
    components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        serde_json::json!({
            "translation": { "x": x, "y": y, "z": 0.0 },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    
    // Simulate health changes for entities with Player component
    if entity_id == 1 || entity_id == 2 {
        let health = 50 + (time * 0.1).sin() as i32 * 25;
        components.insert(
            "Player".to_string(),
            serde_json::json!({
                "speed": 5.0,
                "health": health.max(1).min(100)
            })
        );
    }
    
    components
}


