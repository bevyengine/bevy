//! HTTP client for bevy_remote protocol with streaming support
//!
//! This client implements the bevy_remote JSON-RPC protocol with support for:
//! - bevy/list: List all entities  
//! - bevy/get: Get component data for entities
//! - bevy/get+watch: Stream live component updates

use anyhow::{anyhow, Result};
use bevy_ecs::prelude::*;
use bevy_tasks::Task;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use async_channel::{Receiver, Sender};
use tokio::sync::mpsc;
use futures::{StreamExt, TryStreamExt};

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
    // Legacy channel for receiving live updates (for backward compatibility)
    pub update_receiver: Option<mpsc::UnboundedReceiver<RemoteUpdate>>,
    // New streaming fields for bevy/get+watch
    pub watching_tasks: HashMap<u32, Task<()>>,
    pub component_update_sender: Option<Sender<ComponentUpdate>>,
    pub component_update_receiver: Option<Receiver<ComponentUpdate>>,
    pub watched_entities: HashMap<u32, Vec<String>>, // entity -> components being watched
    // Current cached data
    pub entities: HashMap<u32, RemoteEntity>,
    pub is_connected: bool,
    pub last_error: Option<String>,
    // Connection retry logic
    pub retry_count: u32,
    pub max_retries: u32,
    pub retry_delay: f32, // seconds
    pub last_retry_time: f64,
    pub connection_check_interval: f64, // seconds
    pub last_connection_check: f64,
}

/// Live update from streaming endpoint
#[derive(Debug, Clone)]
pub struct RemoteUpdate {
    pub entity_id: u32,
    pub components: HashMap<String, Value>,
}

/// Enhanced component update structure for live streaming
#[derive(Debug, Clone)]
pub struct ComponentUpdate {
    pub entity_id: u32,
    pub changed_components: HashMap<String, Value>,
    pub removed_components: Vec<String>,
    pub timestamp: f64,
}

/// Response from bevy/get+watch endpoint
#[derive(Debug, Deserialize)]
pub struct BrpGetWatchingResponse {
    pub components: Option<HashMap<String, Value>>,
    pub removed: Option<Vec<String>>,
    pub errors: Option<HashMap<String, Value>>,
}

impl HttpRemoteClient {
    pub fn new(config: &HttpRemoteConfig) -> Self {
        let base_url = format!("http://{}:{}", config.host, config.port);
        
        Self {
            client: Client::new(),
            base_url,
            request_id: 1,
            update_receiver: None,
            // Initialize new streaming fields
            watching_tasks: HashMap::new(),
            component_update_sender: None,
            component_update_receiver: None,
            watched_entities: HashMap::new(),
            entities: HashMap::new(),
            is_connected: false,
            last_error: None,
            // Initialize retry logic
            retry_count: 0,
            max_retries: 10, // Try 10 times before giving up
            retry_delay: 2.0, // Wait 2 seconds between retries
            last_retry_time: 0.0,
            connection_check_interval: 5.0, // Check every 5 seconds if disconnected
            last_connection_check: 0.0,
        }
    }

    /// Test connection to bevy_remote server
    pub async fn connect(&mut self) -> Result<()> {
        println!("Attempting to connect to {} (attempt {}/{})", 
            self.base_url, self.retry_count + 1, self.max_retries);
        
        // Try a simple list request to test connectivity
        match self.list_entities().await {
            Ok(_) => {
                self.is_connected = true;
                self.last_error = None;
                self.retry_count = 0; // Reset retry counter on successful connection
                println!("✅ Connected to bevy_remote at {}", self.base_url);
                Ok(())
            }
            Err(e) => {
                self.is_connected = false;
                self.last_error = Some(e.to_string());
                self.retry_count += 1;
                
                if self.retry_count <= self.max_retries {
                    println!("⚠️  Connection failed (attempt {}/{}): {}", 
                        self.retry_count, self.max_retries, e);
                    println!("   Will retry in {} seconds...", self.retry_delay);
                } else {
                    println!("❌ Failed to connect after {} attempts: {}", self.max_retries, e);
                    println!("   Make sure the target app is running with bevy_remote enabled");
                }
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
            
            println!("Listed {} entities via query", entity_ids.len());
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
            
            println!("Retrieved {} entities with component data", entities.len());
            Ok(entities)
        } else if let Some(error) = response.error {
            Err(anyhow!("bevy/query error: {}", error.message))
        } else {
            Err(anyhow!("Invalid response format"))
        }
    }

    /// Start streaming updates for entities via bevy/get+watch
    pub async fn start_watching(&mut self, entity_ids: &[u32]) -> Result<()> {
        println!("Starting watch stream for {} entities", entity_ids.len());
        
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

    /// Start watching components for an entity using bevy/get+watch with tokio runtime
    pub fn start_component_watching(&mut self, entity_id: u32, components: Vec<String>) -> Result<()> {
        // Create channel for component updates
        let (tx, rx) = async_channel::unbounded();
        self.component_update_sender = Some(tx.clone());
        self.component_update_receiver = Some(rx);
        
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let components_clone = components.clone();
        
        // Spawn task using tokio::spawn since reqwest needs tokio runtime
        let task = bevy_tasks::AsyncComputeTaskPool::get().spawn(async move {
            // Create a tokio runtime for this task since reqwest needs it
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    println!("Failed to create tokio runtime: {}", e);
                    return;
                }
            };
            
            rt.block_on(async {
                // Use bevy/get+watch with continuous polling
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    id: 1, // We'll use a fixed ID for watching requests
                    method: "bevy/get+watch".to_string(),
                    params: Some(serde_json::json!({
                        "entity": entity_id,
                        "components": components_clone,
                        "strict": false
                    })),
                };
                
                println!("Starting component watching for entity {} with {} components", 
                    entity_id, components_clone.len());
                
                // Use streaming SSE connection for real-time updates
                let url = format!("{}/jsonrpc", base_url);
                
                loop {
                    match client.post(&url).json(&request).send().await {
                        Ok(response) => {
                            // Process the streaming response
                            let mut stream = response.bytes_stream();
                            let mut buffer = String::new();
                            
                            while let Ok(Some(chunk)) = stream.try_next().await {
                                if let Ok(text) = std::str::from_utf8(&chunk) {
                                    buffer.push_str(text);
                                    
                                    // Process complete lines
                                    while let Some(newline_pos) = buffer.find('\n') {
                                        let line = buffer[..newline_pos].trim().to_string();
                                        buffer.drain(..newline_pos + 1);
                                        
                                        // Process SSE data lines
                                        if let Some(json_str) = line.strip_prefix("data: ") {
                                            match serde_json::from_str::<JsonRpcResponse>(json_str) {
                                                Ok(json_response) => {
                                                    if let Some(result) = json_response.result {
                                                        if let Ok(watch_response) = serde_json::from_value::<BrpGetWatchingResponse>(result) {
                                                            let update = ComponentUpdate {
                                                                entity_id,
                                                                changed_components: watch_response.components.unwrap_or_default(),
                                                                removed_components: watch_response.removed.unwrap_or_default(),
                                                                timestamp: current_time(),
                                                            };
                                                            
                                                            if !update.changed_components.is_empty() || !update.removed_components.is_empty() {
                                                                println!("Live update for entity {}: {} changed, {} removed", 
                                                                    entity_id, 
                                                                    update.changed_components.len(),
                                                                    update.removed_components.len()
                                                                );
                                                                
                                                                if tx.send(update).await.is_err() {
                                                                    println!("Component update receiver dropped for entity {}", entity_id);
                                                                    return; // Exit the task
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("Failed to parse SSE JSON for entity {}: {} ({})", entity_id, json_str, e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            println!("SSE stream ended for entity {}, reconnecting...", entity_id);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                        Err(e) => {
                            println!("Watch connection error for entity {}: {}", entity_id, e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            // Retry connection
                        }
                    }
                }
            });
        });
        
        self.watching_tasks.insert(entity_id, task);
        self.watched_entities.insert(entity_id, components);
        println!("Started component watching task for entity {}", entity_id);
        Ok(())
    }

    /// Stop watching components for an entity
    pub fn stop_component_watching(&mut self, entity_id: u32) {
        if self.watching_tasks.remove(&entity_id).is_some() {
            self.watched_entities.remove(&entity_id);
            println!("Stopped component watching for entity {}", entity_id);
        }
    }

    /// Check for live component updates from streaming endpoint
    pub fn check_component_updates(&mut self) -> Vec<ComponentUpdate> {
        let mut updates = Vec::new();
        
        if let Some(ref receiver) = self.component_update_receiver {
            while let Ok(update) = receiver.try_recv() {
                updates.push(update);
            }
        }
        
        updates
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

/// Async function to send a watch request to bevy_remote (handles SSE streaming)
async fn send_watch_request(
    client: &reqwest::Client,
    base_url: &str,
    request: &JsonRpcRequest,
) -> Result<Option<BrpGetWatchingResponse>> {
    let url = format!("{}/jsonrpc", base_url);
    let response = client
        .post(&url)
        .json(request)
        .send()
        .await
        .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

    // Check if this is a streaming response (SSE)
    if let Some(content_type) = response.headers().get("content-type") {
        if content_type.to_str().unwrap_or("").contains("text/plain") {
            // This is likely a streaming SSE response
            let response_text = response
                .text()
                .await
                .map_err(|e| anyhow!("Failed to read streaming response: {}", e))?;
            
            // Parse SSE format: look for "data: " lines
            for line in response_text.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    // Try to parse the JSON data
                    match serde_json::from_str::<JsonRpcResponse>(json_str) {
                        Ok(json_response) => {
                            if let Some(result) = json_response.result {
                                // Parse the result as our expected format
                                if let Ok(watch_response) = serde_json::from_value::<BrpGetWatchingResponse>(result) {
                                    return Ok(Some(watch_response));
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to parse SSE JSON line: {} ({})", json_str, e);
                        }
                    }
                }
            }
            return Ok(None); // No valid data found
        }
    }
    
    // Fallback to regular JSON parsing for non-streaming responses
    let json_response: JsonRpcResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;
    
    match json_response.result {
        Some(value) => {
            if value.is_null() {
                Ok(None) // No changes detected
            } else {
                // Try to parse as BrpGetWatchingResponse
                match serde_json::from_value::<BrpGetWatchingResponse>(value.clone()) {
                    Ok(watch_response) => Ok(Some(watch_response)),
                    Err(_) => {
                        // Fallback: try to parse as simple component map
                        if let Some(components_obj) = value.as_object() {
                            let mut components = HashMap::new();
                            for (k, v) in components_obj {
                                components.insert(k.clone(), v.clone());
                            }
                            Ok(Some(BrpGetWatchingResponse {
                                components: Some(components),
                                removed: None,
                                errors: None,
                            }))
                        } else {
                            Ok(None)
                        }
                    }
                }
            }
        },
        None => {
            if let Some(error) = json_response.error {
                Err(anyhow!("Watch request error: {}", error.message))
            } else {
                Ok(None)
            }
        }
    }
}

/// Get current timestamp as f64
fn current_time() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}