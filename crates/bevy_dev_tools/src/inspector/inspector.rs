//! Main inspector plugin that coordinates all functionality

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::Color;
use bevy_camera::Camera2d;
use bevy_text::{TextFont, TextColor};
use tokio::runtime::Handle;
use std::collections::HashMap;
use async_channel;
use serde_json::Value;
use super::http_client::*;
use super::ui::*;
use super::ui::component_viewer::{LiveComponentCache, process_live_component_updates, cleanup_expired_change_indicators, auto_start_component_watching, update_live_component_display, handle_text_selection};
use crate::widgets::selectable_text::TextSelectionState;
use super::ui::virtual_scrolling::{handle_infinite_scroll_input, update_infinite_scrolling_display, update_scroll_momentum, update_scrollbar_indicator, setup_virtual_scrolling, VirtualScrollState, CustomScrollPosition};
use super::ui::entity_list::{EntityListVirtualState, SelectionDebounce};

/// Tokio runtime handle resource for async operations
#[derive(Resource)]
pub struct TokioRuntimeHandle(pub Handle);

/// Main plugin for the remote inspector
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        // Initialize Tokio runtime for HTTP operations
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let handle = rt.handle().clone();
        
        // Keep runtime alive by spawning it in a background thread
        std::thread::spawn(move || {
            rt.block_on(async {
                // Keep runtime alive indefinitely
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            })
        });
        
        app
            // Resources
            .insert_resource(TokioRuntimeHandle(handle))
            .init_resource::<HttpRemoteConfig>()
            .init_resource::<SelectedEntity>()
            .init_resource::<SelectionDebounce>()
            .init_resource::<EntityCache>()
            .init_resource::<ComponentCache>()
            .init_resource::<LiveComponentCache>()
            .init_resource::<TextSelectionState>()
            .init_resource::<VirtualScrollState>()
            .init_resource::<CustomScrollPosition>()
            .init_resource::<EntityListVirtualState>()
            
            // Startup systems
            .add_systems(Startup, (
                setup_http_client,
                setup_virtual_scrolling,
                setup_ui,
            ).chain())
            
            // First update system to populate UI immediately
            .add_systems(PostStartup, initial_ui_population)
            
            
            // Update systems
            .add_systems(Update, (
                // HTTP client systems
                update_entity_list_from_http,
                handle_http_updates,
                
                // Infinite scrolling systems
                handle_infinite_scroll_input,
                update_infinite_scrolling_display,
                update_scroll_momentum,
                update_scrollbar_indicator,
                
                // UI interaction systems
                handle_entity_selection,
                handle_collapsible_interactions,
                cleanup_old_component_content.before(update_component_viewer),
                update_component_viewer,
                update_connection_status,
                
                // New live update systems
                process_live_component_updates,
                cleanup_expired_change_indicators,
                update_live_component_display.after(process_live_component_updates),
                
                // Text selection and copying
                handle_text_selection,
                
                // Auto-start watching for selected entity
                auto_start_component_watching.after(handle_entity_selection),
            ));
    }
}

/// Set up the main UI layout
fn setup_ui(mut commands: Commands) {
    // Spawn UI camera first
    commands.spawn(Camera2d);
    
    // Add a test element to see if UI is working at all
    commands.spawn((
        Text::new("Inspector Loading..."),
        TextFont {
            font_size: 20.0,
            ..Default::default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..Default::default()
        },
    ));
    
    // Root UI container with absolute positioning
    let root = commands.spawn((
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
    )).id();

    // Left panel: Entity list
    let entity_list = spawn_entity_list(&mut commands, root);
    
    // Right panel: Component viewer
    let component_viewer = spawn_component_viewer(&mut commands, root);
    
    // Connection status indicator
    spawn_connection_status(&mut commands, root);
    
    println!("Inspector UI initialized");
    println!("Entity list: {:?}", entity_list);
    println!("Component viewer: {:?}", component_viewer);
}

/// Force initial population attempts HTTP connection
fn initial_ui_population(
    _commands: Commands,
    http_client: Res<HttpRemoteClient>,
    _entity_cache: ResMut<EntityCache>,
    _selected_entity: ResMut<SelectedEntity>,
    container_query: Query<Entity, With<EntityListContainer>>,
) {
    println!("Starting initial UI population - attempting HTTP connection");
    
    let Ok(_container_entity) = container_query.single() else {
        println!("Could not find entity list container");
        return;
    };
    
    // Show connection status - entities will be populated when connection succeeds
    if !http_client.is_connected {
        println!("Waiting for HTTP connection to populate entities...");
        // Entity list will be populated by update_entity_list_from_http once connected
    }
}

/// Set up the HTTP client
fn setup_http_client(
    mut commands: Commands,
    config: Res<HttpRemoteConfig>,
) {
    let mut http_client = HttpRemoteClient::new(&config);
    
    // Initialize connection status communication channel
    let (status_tx, status_rx) = async_channel::unbounded();
    http_client.connection_status_sender = Some(status_tx);
    http_client.connection_status_receiver = Some(status_rx);
    
    // Note: Initial connection will be handled by the retry system in handle_http_updates
    // This avoids blocking the startup and allows proper resource management
    println!("HTTP client initialized - connection will be established automatically");
    
    commands.insert_resource(http_client);
}


/// Update entity list from HTTP client - now just updates cache, virtual scrolling handles rendering
fn update_entity_list_from_http(
    http_client: Res<HttpRemoteClient>,
    mut entity_cache: ResMut<EntityCache>,
    mut selected_entity: ResMut<SelectedEntity>,
) {
    if !http_client.is_connected {
        return;
    }
    
    // Check if we have new entity data
    if http_client.entities.is_empty() {
        return;
    }
    
    // Check if entities changed by comparing counts only (more stable)
    let current_count = http_client.entities.len();
    let cached_count = entity_cache.entities.len();
    
    // Only update if count changed - avoid constant updates from ID order changes
    if current_count != cached_count {
        // Update cache - virtual scrolling will handle the UI updates
        entity_cache.entities.clear();
        for (id, remote_entity) in &http_client.entities {
            // Convert HttpRemoteEntity to the UI format we need
            let ui_entity = RemoteEntity {
                id: *id,
                name: remote_entity.name.clone(),
                components: remote_entity.components.clone(),
            };
            entity_cache.entities.insert(*id, ui_entity);
        }
        
        // Select first entity if none selected
        if selected_entity.entity_id.is_none() {
            if let Some(first_entity) = entity_cache.entities.values().next() {
                selected_entity.entity_id = Some(first_entity.id);
                println!("Auto-selected first entity: {}", first_entity.id);
            }
        }
        
        println!("Updated entity cache with {} entities from HTTP", entity_cache.entities.len());
    }
}

/// Handle HTTP updates from remote client and auto-retry connection
fn handle_http_updates(
    mut http_client: ResMut<HttpRemoteClient>,
    time: Res<bevy_time::Time>,
    tokio_handle: Res<TokioRuntimeHandle>,
) {
    let current_time = time.elapsed_secs_f64();
    
    // Process connection status updates from async tasks
    let mut status_updates = Vec::new();
    if let Some(receiver) = &http_client.connection_status_receiver {
        while let Ok(status_update) = receiver.try_recv() {
            status_updates.push(status_update);
        }
    }
    
    // Process all collected status updates
    for status_update in status_updates {
        http_client.is_connected = status_update.is_connected;
        http_client.last_error = status_update.error_message;
        
        if status_update.is_connected {
            // Update entity cache with fetched entities
            http_client.entities = status_update.entities;
            println!("✅ Connection established and {} entities loaded", http_client.entities.len());
        } else {
            println!("❌ Connection failed: {}", 
                http_client.last_error.as_deref().unwrap_or("Unknown error"));
        }
    }
    
    // Auto-retry connection if not connected and enough time has passed
    if !http_client.is_connected {
        let should_retry = if http_client.retry_count == 0 {
            // First retry attempt
            true
        } else if http_client.retry_count < http_client.max_retries {
            // Subsequent retries with delay
            current_time - http_client.last_retry_time >= http_client.retry_delay as f64
        } else {
            // Periodic checks after max retries (less frequent)
            current_time - http_client.last_connection_check >= http_client.connection_check_interval
        };
        
        if should_retry {
            // Increment retry count first
            http_client.retry_count += 1;
            http_client.last_retry_time = current_time;
            http_client.last_connection_check = current_time;
            
            println!("🔄 Attempting reconnection (attempt {}/{})", 
                http_client.retry_count, http_client.max_retries);
            
            // Create a connection test
            let base_url = http_client.base_url.clone();
            let client = http_client.client.clone();
            let retry_count = http_client.retry_count;
            let max_retries = http_client.max_retries;
            let status_sender = http_client.connection_status_sender.clone();
            
            // Spawn async connection attempt using tokio runtime handle
            tokio_handle.0.spawn(async move {
                // Test basic connectivity first
                let health_url = format!("{}/health", base_url);
                let health_result = client.get(&health_url).send().await;
                
                match health_result {
                    Ok(response) if response.status().is_success() => {
                        println!("✅ Health check successful to {} (status: {})", base_url, response.status());
                    }
                    Ok(response) => {
                        println!("⚠️  Server responding but health check returned: {}", response.status());
                    }
                    Err(e) => {
                        println!("❌ Health check failed (attempt {}/{}): {}", retry_count, max_retries, e);
                    }
                }
                
                // Now test the actual JSON-RPC endpoint
                let jsonrpc_url = format!("{}/jsonrpc", base_url);
                let test_request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "bevy/query",
                    "params": {
                        "data": {
                            "components": [],
                            "option": [
                                "bevy_transform::components::transform::Transform",
                                "server::Cube",
                                "bevy_core_pipeline::core_3d::camera_3d::Camera3d",
                                "bevy_render::light::point_light::PointLight"
                            ],
                            "has": []
                        },
                        "filter": {
                            "with": [],
                            "without": []
                        },
                        "strict": false
                    }
                });
                
                let jsonrpc_result = client.post(&jsonrpc_url)
                    .json(&test_request)
                    .timeout(std::time::Duration::from_secs(10))
                    .send().await;
                
                match jsonrpc_result {
                    Ok(response) if response.status().is_success() => {
                        println!("✅ JSON-RPC connection successful to {} (attempt {}/{})", 
                            base_url, retry_count, max_retries);
                        
                        // Fetch entities on successful connection
                        let entities_result = response.json::<Value>().await;
                        match entities_result {
                            Ok(json_response) => {
                                // Debug: Print the actual JSON response structure
                                println!("📋 JSON-RPC Response: {}", serde_json::to_string_pretty(&json_response).unwrap_or_else(|_| "Failed to serialize".to_string()));
                                
                                // Parse the entities from the response
                                let mut entities = HashMap::new();
                                if let Some(result) = json_response.get("result") {
                                    if let Some(entities_array) = result.as_array() {
                                        for entity_data in entities_array {
                                            if let Some(entity_obj) = entity_data.as_object() {
                                                // Parse Bevy entity ID (numeric format)
                                                if let Some(entity_id_num) = entity_obj.get("entity").and_then(|id| id.as_u64()) {
                                                    let entity_id = entity_id_num as u32;
                                                    
                                                    // Extract components from the "components" object
                                                    let components: HashMap<String, Value> = entity_obj
                                                        .get("components")
                                                        .and_then(|c| c.as_object())
                                                        .map(|comp_obj| comp_obj.iter()
                                                            .map(|(k, v)| (k.clone(), v.clone()))
                                                            .collect())
                                                        .unwrap_or_default();
                                                    
                                                    let entity = RemoteEntity {
                                                        id: entity_id,
                                                        name: components.get("Name")
                                                            .and_then(|v| v.as_str())
                                                            .map(|s| s.to_string()),
                                                        components,
                                                    };
                                                    entities.insert(entity_id, entity);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Send successful connection status with entities
                                if let Some(sender) = &status_sender {
                                    let status_update = ConnectionStatusUpdate {
                                        is_connected: true,
                                        error_message: None,
                                        entities,
                                    };
                                    let _ = sender.send(status_update).await;
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to parse entities response: {}", e);
                                // Send failed connection status
                                if let Some(sender) = &status_sender {
                                    let status_update = ConnectionStatusUpdate {
                                        is_connected: false,
                                        error_message: Some(format!("Failed to parse entities: {}", e)),
                                        entities: HashMap::new(),
                                    };
                                    let _ = sender.send(status_update).await;
                                }
                            }
                        }
                    }
                    Ok(response) => {
                        println!("❌ JSON-RPC endpoint returned error {} (attempt {}/{})", 
                            response.status(), retry_count, max_retries);
                        // Send failed connection status
                        if let Some(sender) = &status_sender {
                            let status_update = ConnectionStatusUpdate {
                                is_connected: false,
                                error_message: Some(format!("JSON-RPC error: {}", response.status())),
                                entities: HashMap::new(),
                            };
                            let _ = sender.send(status_update).await;
                        }
                    }
                    Err(e) => {
                        println!("❌ JSON-RPC connection failed (attempt {}/{}): {}", 
                            retry_count, max_retries, e);
                        // Send failed connection status
                        if let Some(sender) = &status_sender {
                            let status_update = ConnectionStatusUpdate {
                                is_connected: false,
                                error_message: Some(format!("Connection failed: {}", e)),
                                entities: HashMap::new(),
                            };
                            let _ = sender.send(status_update).await;
                        }
                        if retry_count >= max_retries {
                            println!("   Max retries reached. Make sure the target app is running with bevy_remote enabled.");
                            println!("   Expected endpoints: {}/health and {}/jsonrpc", base_url, base_url);
                        }
                    }
                }
            });
        }
    }
    
    // Process any pending updates from HTTP client
    let updates = http_client.check_updates();
    if !updates.is_empty() {
        println!("Received {} updates from HTTP client", updates.len());
    }
}