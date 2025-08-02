//! Main inspector plugin that coordinates all functionality

use bevy::prelude::*;
use crate::http_client::*;
use crate::ui::*;
use crate::ui::virtual_scrolling::{handle_infinite_scroll_input, update_infinite_scrolling_display, update_scroll_momentum};

/// Main plugin for the remote inspector
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<HttpRemoteConfig>()
            .init_resource::<SelectedEntity>()
            .init_resource::<EntityCache>()
            .init_resource::<ComponentCache>()
            .init_resource::<crate::ui::virtual_scrolling::VirtualScrollState>()
            .init_resource::<crate::ui::virtual_scrolling::CustomScrollPosition>()
            
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
                
                // UI interaction systems
                handle_entity_selection,
                handle_collapsible_interactions,
                cleanup_old_component_content.before(update_component_viewer),
                update_component_viewer,
                update_connection_status,
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
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
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
            ..default()
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
            println!("✅ HTTP client connected and loaded {} entities", entities.len());
        }
        Err(e) => {
            println!("❌ HTTP client connection failed: {}", e);
            println!("💡 Make sure the target app is running with bevy_remote enabled");
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

/// Handle HTTP updates from remote client
fn handle_http_updates(
    mut http_client: ResMut<HttpRemoteClient>,
) {
    // Process any pending updates from HTTP client
    let updates = http_client.check_updates();
    if !updates.is_empty() {
        println!("📡 Received {} updates from HTTP client", updates.len());
    }
}