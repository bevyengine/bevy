//! Main inspector plugin that coordinates all functionality

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::Color;
use bevy_camera::Camera2d;
use bevy_text::{TextFont, TextColor};
use super::http_client::*;
use super::ui::*;
use super::ui::component_viewer::{LiveComponentCache, process_live_component_updates, cleanup_expired_change_indicators, auto_start_component_watching, update_live_component_display, handle_text_selection, TextSelectionState};
use super::ui::virtual_scrolling::{handle_infinite_scroll_input, update_infinite_scrolling_display, update_scroll_momentum, update_scrollbar_indicator, setup_virtual_scrolling, VirtualScrollState, CustomScrollPosition};
use super::ui::entity_list::{EntityListVirtualState, SelectionDebounce};

/// Main plugin for the remote inspector
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
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
    
    // Try to populate with initial data immediately
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(async {
        http_client.connect().await?;
        http_client.get_entities(&[]).await
    }) {
        Ok(entities) => {
            println!("HTTP client connected and loaded {} entities", entities.len());
        }
        Err(e) => {
            println!("HTTP client connection failed: {}", e);
            println!("Make sure the target app is running with bevy_remote enabled");
        }
    }
    
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
) {
    let current_time = time.elapsed_secs_f64();
    
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
            http_client.last_retry_time = current_time;
            http_client.last_connection_check = current_time;
            
            // Try to reconnect using tokio runtime
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    println!("Failed to create tokio runtime for reconnection: {}", e);
                    return;
                }
            };
            
            match rt.block_on(async {
                http_client.connect().await?;
                http_client.get_entities(&[]).await
            }) {
                Ok(entities) => {
                    println!("🔄 Reconnected! Loaded {} entities", entities.len());
                }
                Err(_) => {
                    // Error logging handled in connect() method
                }
            }
        }
    }
    
    // Process any pending updates from HTTP client
    let updates = http_client.check_updates();
    if !updates.is_empty() {
        println!("Received {} updates from HTTP client", updates.len());
    }
}