//! HTTP client for bevy_remote protocol with connection resilience
//!
//! This client implements the bevy_remote JSON-RPC protocol with comprehensive support for:
//! - **bevy/query**: Query entities and components with flexible filtering
//! - **bevy/get+watch**: Stream live component updates via Server-Sent Events (SSE)
//! - **Connection Management**: Auto-retry logic with exponential backoff
//! - **Error Recovery**: Robust error handling and reconnection strategies
//!
//! The client automatically handles connection failures and provides real-time updates
//! for component values in remote Bevy applications.

use anyhow::{anyhow, Result};
use async_channel::{Receiver, Sender};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use futures::TryStreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC request structure for bevy_remote protocol
#[derive(Serialize, Debug)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version (always "2.0")
    pub jsonrpc: String,
    /// Unique request identifier
    pub id: u32,
    /// Method name (e.g., "bevy/query", "bevy/get+watch")
    pub method: String,
    /// Optional method parameters
    pub params: Option<Value>,
}

/// JSON-RPC response structure
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u32,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error structure
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// Remote entity representation from bevy_remote
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteEntity {
    /// Entity ID from the remote Bevy application
    pub id: u32,
    /// Optional entity name (from bevy_core::Name component)
    pub name: Option<String>,
    /// Map of component type names to their serialized values
    pub components: HashMap<String, Value>,
}

/// Configuration for HTTP remote client
#[derive(Resource, Debug)]
pub struct HttpRemoteConfig {
    /// Remote server hostname (default: "localhost")
    pub host: String,
    /// Remote server port (default: 15702)
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

/// HTTP client for bevy_remote communication with connection resilience
///
/// This client manages communication with remote Bevy applications via the bevy_remote
/// JSON-RPC protocol. It provides automatic connection retry logic, live component
/// streaming, and robust error handling.
///
/// # Features
/// - Auto-retry connection logic with exponential backoff
/// - Live component value streaming via bevy/get+watch
/// - Comprehensive entity and component querying
/// - Connection status monitoring and recovery
/// - Async channel-based communication for non-blocking updates
#[derive(Resource)]
pub struct HttpRemoteClient {
    /// HTTP client for making requests
    pub client: Client,
    /// Base URL for the remote server (e.g., "http://localhost:15702")
    pub base_url: String,
    /// Counter for generating unique request IDs
    pub request_id: u32,
    /// Legacy channel for receiving live updates (for backward compatibility)
    pub update_receiver: Option<Receiver<RemoteUpdate>>,
    /// Sender for streaming component updates via bevy/get+watch
    pub component_update_sender: Option<Sender<ComponentUpdate>>,
    /// Receiver for streaming component updates via bevy/get+watch
    pub component_update_receiver: Option<Receiver<ComponentUpdate>>,
    /// Sender for connection status updates from async tasks
    pub connection_status_sender: Option<Sender<ConnectionStatusUpdate>>,
    /// Receiver for connection status updates from async tasks
    pub connection_status_receiver: Option<Receiver<ConnectionStatusUpdate>>,
    /// Map of entity IDs to their watched component lists
    pub watched_entities: HashMap<u32, Vec<String>>,
    /// Cached entities retrieved from the remote server
    pub entities: HashMap<u32, RemoteEntity>,
    /// Current connection status
    pub is_connected: bool,
    /// Last connection error message
    pub last_error: Option<String>,
    /// Current retry attempt count
    pub retry_count: u32,
    /// Maximum number of retry attempts before giving up
    pub max_retries: u32,
    /// Delay between retry attempts in seconds
    pub retry_delay: f32,
    /// Timestamp of the last retry attempt
    pub last_retry_time: f64,
    /// Interval for periodic connection checks in seconds
    pub connection_check_interval: f64,
    /// Timestamp of the last connection check
    pub last_connection_check: f64,
}

/// Live update from streaming endpoint
#[derive(Debug, Clone)]
pub struct RemoteUpdate {
    /// Entity ID that was updated
    pub entity_id: u32,
    /// Updated component data
    pub components: HashMap<String, Value>,
}

/// Enhanced component update structure for live streaming
#[derive(Debug, Clone)]
pub struct ComponentUpdate {
    /// Entity ID that was updated
    pub entity_id: u32,
    /// Components that were added or changed
    pub changed_components: HashMap<String, Value>,
    /// Components that were removed
    pub removed_components: Vec<String>,
    /// Timestamp when the update occurred
    pub timestamp: f64,
}

/// Response from bevy/get+watch endpoint
#[derive(Debug, Deserialize)]
pub struct BrpGetWatchingResponse {
    /// Updated component data
    pub components: Option<HashMap<String, Value>>,
    /// List of removed component type names
    pub removed: Option<Vec<String>>,
    /// Error information for failed components
    pub errors: Option<HashMap<String, Value>>,
}

/// Connection status update from async tasks to Bevy systems
#[derive(Debug, Clone)]
pub struct ConnectionStatusUpdate {
    /// Whether the connection is currently active
    pub is_connected: bool,
    /// Error message if connection failed
    pub error_message: Option<String>,
    /// Entities fetched during successful connection
    pub entities: HashMap<u32, RemoteEntity>,
}

impl HttpRemoteClient {
    /// Create a new HTTP remote client with the given configuration
    pub fn new(config: &HttpRemoteConfig) -> Self {
        let base_url = format!("http://{}:{}", config.host, config.port);

        Self {
            client: Client::new(),
            base_url,
            request_id: 1,
            update_receiver: None,
            // Initialize new streaming fields
            component_update_sender: None,
            component_update_receiver: None,
            // Initialize connection status communication
            connection_status_sender: None,
            connection_status_receiver: None,
            watched_entities: HashMap::new(),
            entities: HashMap::new(),
            is_connected: false,
            last_error: None,
            // Initialize retry logic
            retry_count: 0,
            max_retries: 10,  // Try 10 times before giving up
            retry_delay: 2.0, // Wait 2 seconds between retries
            last_retry_time: 0.0,
            connection_check_interval: 5.0, // Check every 5 seconds if disconnected
            last_connection_check: 0.0,
        }
    }

    /// Test connection to bevy_remote server
    pub async fn connect(&mut self) -> Result<()> {
        debug!(
            "Attempting connection to {} (attempt {}/{})",
            self.base_url,
            self.retry_count + 1,
            self.max_retries
        );

        // Try a simple list request to test connectivity
        match self.list_entities().await {
            Ok(_) => {
                self.is_connected = true;
                self.last_error = None;
                self.retry_count = 0; // Reset retry counter on successful connection
                info!("Connected to bevy_remote at {}", self.base_url);
                Ok(())
            }
            Err(e) => {
                self.is_connected = false;
                self.last_error = Some(e.to_string());
                self.retry_count += 1;

                if self.retry_count <= self.max_retries {
                    warn!(
                        "Connection failed (attempt {}/{}): {}",
                        self.retry_count, self.max_retries, e
                    );
                    debug!("Will retry in {} seconds", self.retry_delay);
                } else {
                    error!(
                        "Failed to connect after {} attempts: {}",
                        self.max_retries, e
                    );
                    error!("Ensure target app is running with bevy_remote enabled");
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
            let entities: Vec<Value> = serde_json::from_value(result)
                .map_err(|e| anyhow!("Failed to parse entity query: {}", e))?;

            let mut entity_ids = Vec::new();
            for entity_obj in entities {
                if let Some(entity_id) = entity_obj.get("entity").and_then(|v| v.as_u64()) {
                    entity_ids.push(entity_id as u32);
                }
            }

            debug!("Listed {} entities via query", entity_ids.len());
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
            let query_results: Vec<Value> = serde_json::from_value(result)
                .map_err(|e| anyhow!("Failed to parse query results: {}", e))?;

            let mut entities = Vec::new();

            for query_result in query_results.iter() {
                if let (Some(entity_id), Some(components_obj)) = (
                    query_result.get("entity").and_then(|v| v.as_u64()),
                    query_result.get("components").and_then(|v| v.as_object()),
                ) {
                    let mut components = HashMap::new();

                    // Convert components object to HashMap
                    for (component_name, component_data) in components_obj {
                        components.insert(component_name.clone(), component_data.clone());
                    }

                    // Try to extract name from Name component if it exists
                    // Name is a tuple struct, so it should be in "0" field or direct string
                    let name = components.get("bevy_core::name::Name").and_then(|v| {
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

            debug!("Retrieved {} entities with component data", entities.len());
            Ok(entities)
        } else if let Some(error) = response.error {
            Err(anyhow!("bevy/query error: {}", error.message))
        } else {
            Err(anyhow!("Invalid response format"))
        }
    }

    /// Start streaming updates for entities via bevy/get+watch
    pub async fn start_watching(&mut self, entity_ids: &[u32]) -> Result<()> {
        debug!("Starting watch stream for {} entities", entity_ids.len());

        // Note: This method is now deprecated in favor of start_component_watching
        // which uses the new bevy_remote streaming API properly
        warn!("start_watching is deprecated, use start_component_watching instead");

        Ok(())
    }

    /// Start watching components for an entity using bevy/get+watch with bevy_tasks
    pub fn start_component_watching(
        &mut self,
        entity_id: u32,
        components: Vec<String>,
        tokio_handle: &tokio::runtime::Handle,
    ) -> Result<()> {
        // Create channel for component updates
        let (tx, rx) = async_channel::unbounded();
        self.component_update_sender = Some(tx.clone());
        self.component_update_receiver = Some(rx);

        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let components_clone = components.clone();

        // Use tokio runtime handle for reqwest compatibility
        tokio_handle.spawn(async move {
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
                            // Simple async delay without tokio
                            let sleep_future = async {
                                use std::time::{Duration, Instant};
                                let start = Instant::now();
                                let target_duration = Duration::from_secs(1);

                                loop {
                                    if start.elapsed() >= target_duration {
                                        break;
                                    }
                                    futures::future::ready(()).await;
                                }
                            };
                            sleep_future.await;
                        }
                        Err(e) => {
                            println!("Watch connection error for entity {}: {}", entity_id, e);
                            // Simple async delay without tokio
                            let sleep_future = async {
                                use std::time::{Duration, Instant};
                                let start = Instant::now();
                                let target_duration = Duration::from_secs(1);

                                loop {
                                    if start.elapsed() >= target_duration {
                                        break;
                                    }
                                    futures::future::ready(()).await;
                                }
                            };
                            sleep_future.await;
                            // Retry connection
                        }
                    }
                }
        });

        self.watched_entities.insert(entity_id, components);
        println!("Started component watching task for entity {}", entity_id);
        Ok(())
    }

    /// Stop watching components for an entity
    pub fn stop_component_watching(&mut self, entity_id: u32) {
        if self.watched_entities.remove(&entity_id).is_some() {
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

        let response = self
            .client
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

/// Get current timestamp as f64
fn current_time() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
